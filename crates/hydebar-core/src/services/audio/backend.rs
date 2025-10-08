use std::{
    any::TypeId,
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    thread::{self, JoinHandle},
};

use anyhow::Context as _;
use iced::futures::executor::block_on;
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        self, Context, FlagSet,
        introspect::{Introspector, SinkInfo, SourceInfo},
        subscribe::InterestMaskSet,
    },
    def::{DevicePortType, PortAvailable, SinkState, SourceState},
    mainloop::standard::{IterateResult, Mainloop},
    operation::{self, Operation},
    proplist::{Proplist, properties::APPLICATION_NAME},
    volume::ChannelVolumes,
};
use log::{debug, error, trace};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::services::audio::model::{AudioEvent, Device, DeviceType, Port, ServerInfo};

/// Commands accepted by backend implementations.
#[derive(Debug, Clone,)]
pub enum BackendCommand
{
    SinkMute(String, bool,),
    SourceMute(String, bool,),
    SinkVolume(String, ChannelVolumes,),
    SourceVolume(String, ChannelVolumes,),
    DefaultSink(String, String,),
    DefaultSource(String, String,),
}

/// Events emitted by backend implementations.
#[derive(Debug, Clone,)]
pub enum BackendEvent
{
    Error(String,),
    Update(AudioEvent,),
}

/// Future returned by backend spawners.
pub type BackendFuture = Pin<Box<dyn Future<Output = anyhow::Result<BackendHandle,>,> + Send,>,>;

/// Abstraction over backend implementations to allow testing without
/// PulseAudio.
pub trait AudioBackend: Send + Sync + Clone + 'static
{
    fn spawn(&self,) -> BackendFuture;
}

/// Default PulseAudio backend implementation.
#[derive(Clone, Default,)]
pub struct PulseAudioBackend;

impl AudioBackend for PulseAudioBackend
{
    fn spawn(&self,) -> BackendFuture
    {
        Box::pin(async { PulseAudioServer::start().await },)
    }
}

/// Handle returned by [`AudioBackend::spawn`].
///
/// Keeps the listener and commander thread handles alive for the lifetime
/// of the backend. When dropped, the threads will be aborted.
#[derive(Debug,)]
pub struct BackendHandle
{
    pub(crate) receiver: UnboundedReceiver<BackendEvent,>,
    pub(crate) sender:   UnboundedSender<BackendCommand,>,
    _listener:           Option<JoinHandle<(),>,>,
    _commander:          Option<JoinHandle<(),>,>,
}

impl BackendHandle
{
    fn new(
        receiver: UnboundedReceiver<BackendEvent,>,
        sender: UnboundedSender<BackendCommand,>,
        listener: JoinHandle<(),>,
        commander: JoinHandle<(),>,
    ) -> Self
    {
        Self {
            receiver,
            sender,
            _listener: Some(listener,),
            _commander: Some(commander,),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_parts(
        receiver: UnboundedReceiver<BackendEvent,>,
        sender: UnboundedSender<BackendCommand,>,
    ) -> Self
    {
        Self {
            receiver,
            sender,
            _listener: None,
            _commander: None,
        }
    }

    pub(crate) fn commander(&self,) -> UnboundedSender<BackendCommand,>
    {
        self.sender.clone()
    }

    pub(crate) async fn recv(&mut self,) -> Option<BackendEvent,>
    {
        self.receiver.recv().await
    }
}

struct PulseAudioServer
{
    mainloop:     Mainloop,
    context:      Context,
    introspector: Introspector,
}

impl PulseAudioServer
{
    fn new() -> anyhow::Result<Self,>
    {
        let name = format!("{:?}", TypeId::of::<Self,>());
        let mut proplist = Proplist::new().context("create PulseAudio properties",)?;
        proplist
            .set_str(APPLICATION_NAME, name.as_str(),)
            .map_err(|_| anyhow::anyhow!("failed to set application name"),)?;

        let mut mainloop = Mainloop::new().context("create PulseAudio mainloop",)?;

        let mut context = Context::new_with_proplist(&mainloop, name.as_str(), &proplist,)
            .context("create PulseAudio context",)?;

        context.connect(None, FlagSet::NOFLAGS, None,).context("connect PulseAudio context",)?;

        loop {
            match mainloop.iterate(true,) {
                IterateResult::Quit(_,) | IterateResult::Err(_,) => {
                    return Err(anyhow::anyhow!("PulseAudio mainloop failed during init"),);
                }
                IterateResult::Success(_,) => {
                    if context.get_state() == context::State::Ready {
                        break;
                    }
                }
            }
        }

        let introspector = context.introspect();

        Ok(Self {
            mainloop,
            context,
            introspector,
        },)
    }

    async fn start() -> anyhow::Result<BackendHandle,>
    {
        let (from_server_tx, from_server_rx,) = tokio::sync::mpsc::unbounded_channel();
        let (to_server_tx, to_server_rx,) = tokio::sync::mpsc::unbounded_channel();

        let listener = Self::start_listener(from_server_tx.clone(),).await?;
        let commander = Self::start_commander(from_server_tx.clone(), to_server_rx,).await?;

        Ok(BackendHandle::new(from_server_rx, to_server_tx, listener, commander,),)
    }

    async fn start_listener(
        from_server_tx: UnboundedSender<BackendEvent,>,
    ) -> anyhow::Result<JoinHandle<(),>,>
    {
        let (ready_tx, mut ready_rx,) = tokio::sync::mpsc::unbounded_channel();

        let handle = thread::spawn({
            let from_server_tx = from_server_tx.clone();
            move || match Self::new() {
                Ok(mut server,) => {
                    let _ = ready_tx.send(true,);

                    server.context.subscribe(
                        InterestMaskSet::SERVER
                            .union(InterestMaskSet::SINK,)
                            .union(InterestMaskSet::SOURCE,),
                        |result| {
                            if !result {
                                error!("Audio subscription failed");
                            }
                        },
                    );

                    if let Err(err,) =
                        server.wait_for_response(server.introspector.get_server_info({
                            let tx = from_server_tx.clone();
                            move |info| {
                                Self::send_server_info(info, &tx,);
                            }
                        },),)
                    {
                        error!("Failed to get server info: {err}");
                        let _ = from_server_tx.send(BackendEvent::Error(err.to_string(),),);
                    }

                    let sinks = Rc::new(RefCell::new(Vec::new(),),);
                    if let Err(err,) =
                        server.wait_for_response(server.introspector.get_sink_info_list({
                            let tx = from_server_tx.clone();
                            let sinks = sinks.clone();
                            move |info| {
                                Self::populate_and_send_sinks(info, &tx, &mut sinks.borrow_mut(),);
                            }
                        },),)
                    {
                        error!("Failed to get sink info: {err}");
                        let _ = from_server_tx.send(BackendEvent::Error(err.to_string(),),);
                    }

                    let sources = Rc::new(RefCell::new(Vec::new(),),);
                    if let Err(err,) =
                        server.wait_for_response(server.introspector.get_source_info_list({
                            let tx = from_server_tx.clone();
                            let sources = sources.clone();
                            move |info| {
                                Self::populate_and_send_sources(
                                    info,
                                    &tx,
                                    &mut sources.borrow_mut(),
                                );
                            }
                        },),)
                    {
                        error!("Failed to get source info: {err}");
                        let _ = from_server_tx.send(BackendEvent::Error(err.to_string(),),);
                    }

                    let introspector = server.context.introspect();
                    let from_server_tx_clone = from_server_tx.clone();
                    server.context.set_subscribe_callback(Some(Box::new(
                        move |_facility, _operation, _idx| {
                            server.introspector.get_server_info({
                                let tx = from_server_tx_clone.clone();

                                move |info| {
                                    Self::send_server_info(info, &tx,);
                                }
                            },);
                            introspector.get_sink_info_list({
                                let tx = from_server_tx_clone.clone();
                                let sinks = sinks.clone();

                                move |info| {
                                    Self::populate_and_send_sinks(
                                        info,
                                        &tx,
                                        &mut sinks.borrow_mut(),
                                    );
                                }
                            },);
                            introspector.get_source_info_list({
                                let tx = from_server_tx_clone.clone();
                                let sources = sources.clone();

                                move |info| {
                                    Self::populate_and_send_sources(
                                        info,
                                        &tx,
                                        &mut sources.borrow_mut(),
                                    );
                                }
                            },);
                        },
                    ),),);

                    loop {
                        let data = server.mainloop.iterate(true,);
                        if let IterateResult::Quit(_,) | IterateResult::Err(_,) = data {
                            error!("PulseAudio mainloop error");
                            let _ = from_server_tx
                                .send(BackendEvent::Error("PulseAudio mainloop error".into(),),);
                            break;
                        }
                    }
                }
                Err(err,) => {
                    error!("Failed to start PulseAudio listener thread: {err}");
                    let _ = ready_tx.send(false,);
                }
            }
        },);

        match ready_rx.recv().await {
            Some(true,) => Ok(handle,),
            _ => Err(anyhow::anyhow!("Failed to start PulseAudio listener thread"),),
        }
    }

    async fn start_commander(
        from_server_tx: UnboundedSender<BackendEvent,>,
        mut to_server_rx: UnboundedReceiver<BackendCommand,>,
    ) -> anyhow::Result<JoinHandle<(),>,>
    {
        let (ready_tx, mut ready_rx,) = tokio::sync::mpsc::unbounded_channel();

        let handle = thread::spawn(move || {
            block_on(async move {
                match Self::new() {
                    Ok(mut server,) => {
                        let _ = ready_tx.send(true,);
                        while let Some(command,) = to_server_rx.recv().await {
                            if let Err(err,) = match command {
                                BackendCommand::SinkMute(name, mute,) => {
                                    server.set_sink_mute(&name, mute,)
                                }
                                BackendCommand::SourceMute(name, mute,) => {
                                    server.set_source_mute(&name, mute,)
                                }
                                BackendCommand::SinkVolume(name, volume,) => {
                                    server.set_sink_volume(&name, &volume,)
                                }
                                BackendCommand::SourceVolume(name, volume,) => {
                                    server.set_source_volume(&name, &volume,)
                                }
                                BackendCommand::DefaultSink(name, port,) => {
                                    server.set_default_sink(&name, &port,)
                                }
                                BackendCommand::DefaultSource(name, port,) => {
                                    server.set_default_source(&name, &port,)
                                }
                            } {
                                error!("PulseAudio command failed: {err}");
                            }
                        }
                    }
                    Err(err,) => {
                        error!("Failed to start PulseAudio commander: {err}");
                        let _ = from_server_tx.send(BackendEvent::Error(err.to_string(),),);
                    }
                }
            },)
        },);

        match ready_rx.recv().await {
            Some(true,) => Ok(handle,),
            _ => Err(anyhow::anyhow!("Failed to start PulseAudio commander thread"),),
        }
    }

    fn wait_for_response<T: ?Sized,>(&mut self, operation: Operation<T,>,) -> anyhow::Result<(),>
    {
        loop {
            match self.mainloop.iterate(true,) {
                IterateResult::Quit(_,) | IterateResult::Err(_,) => {
                    error!("PulseAudio iterate failure");
                    return Err(anyhow::anyhow!("PulseAudio iterate failure"),);
                }
                IterateResult::Success(_,) => {
                    if operation.get_state() == operation::State::Done {
                        break;
                    }
                }
            }
        }

        Ok((),)
    }

    fn send_server_info(
        info: &libpulse_binding::context::introspect::ServerInfo<'_,>,
        tx: &UnboundedSender<BackendEvent,>,
    )
    {
        let _ = tx.send(BackendEvent::Update(AudioEvent::ServerInfo(info.into(),),),);
    }

    fn populate_and_send_sinks(
        info: ListResult<&SinkInfo<'_,>,>,
        tx: &UnboundedSender<BackendEvent,>,
        sinks: &mut Vec<Device,>,
    )
    {
        match info {
            ListResult::Item(data,) => {
                if data.ports.iter().any(|port| port.available != PortAvailable::No,) {
                    debug!("Adding sink data: {data:?}");
                    sinks.push(data.into(),);
                }
            }
            ListResult::End => {
                debug!("New sink list {sinks:?}");
                let _ = tx.send(BackendEvent::Update(AudioEvent::Sinks(sinks.clone(),),),);
                sinks.clear();
            }
            ListResult::Error => error!("Error during sink list population"),
        }
    }

    fn populate_and_send_sources(
        info: ListResult<&SourceInfo<'_,>,>,
        tx: &UnboundedSender<BackendEvent,>,
        sources: &mut Vec<Device,>,
    )
    {
        match info {
            ListResult::Item(data,) => {
                trace!("Received source data: {data:?}");

                if data.name.as_ref().map(|name| !name.contains("monitor",),).unwrap_or_default() {
                    debug!("Adding source data: {data:?}");
                    sources.push(data.into(),);
                }
            }
            ListResult::End => {
                debug!("New sources list {sources:?}");
                let _ = tx.send(BackendEvent::Update(AudioEvent::Sources(sources.clone(),),),);
                sources.clear();
            }
            ListResult::Error => error!("Error during sources list population"),
        }
    }

    fn set_sink_mute(&mut self, name: &str, mute: bool,) -> anyhow::Result<(),>
    {
        let op = self.introspector.set_sink_mute_by_name(name, mute, None,);
        self.wait_for_response(op,)
    }

    fn set_source_mute(&mut self, name: &str, mute: bool,) -> anyhow::Result<(),>
    {
        let op = self.introspector.set_source_mute_by_name(name, mute, None,);
        self.wait_for_response(op,)
    }

    fn set_sink_volume(&mut self, name: &str, volume: &ChannelVolumes,) -> anyhow::Result<(),>
    {
        let op = self.introspector.set_sink_volume_by_name(name, volume, None,);
        self.wait_for_response(op,)
    }

    fn set_source_volume(&mut self, name: &str, volume: &ChannelVolumes,) -> anyhow::Result<(),>
    {
        let op = self.introspector.set_source_volume_by_name(name, volume, None,);
        self.wait_for_response(op,)
    }

    fn set_default_sink(&mut self, name: &str, port: &str,) -> anyhow::Result<(),>
    {
        let op = self.context.set_default_sink(name, |_| {},);
        self.wait_for_response(op,)?;

        let op = self.introspector.set_sink_port_by_name(name, port, None,);
        self.wait_for_response(op,)
    }

    fn set_default_source(&mut self, name: &str, port: &str,) -> anyhow::Result<(),>
    {
        let op = self.context.set_default_source(name, |_| {},);
        self.wait_for_response(op,)?;

        let op = self.introspector.set_source_port_by_name(name, port, None,);
        self.wait_for_response(op,)
    }
}

impl From<&libpulse_binding::context::introspect::ServerInfo<'_,>,> for ServerInfo
{
    fn from(value: &libpulse_binding::context::introspect::ServerInfo<'_,>,) -> Self
    {
        Self {
            default_sink:   value
                .default_sink_name
                .as_ref()
                .map_or_else(String::default, ToString::to_string,),
            default_source: value
                .default_source_name
                .as_ref()
                .map_or_else(String::default, ToString::to_string,),
        }
    }
}

impl From<&SinkInfo<'_,>,> for Device
{
    fn from(value: &SinkInfo<'_,>,) -> Self
    {
        Self {
            name:        value.name.as_ref().map_or(String::default(), ToString::to_string,),
            description: value.proplist.get_str("device.description",).unwrap_or_default(),
            volume:      value.volume,
            is_mute:     value.mute,
            in_use:      value.state == SinkState::Running,
            ports:       value
                .ports
                .iter()
                .filter_map(|port| {
                    if port.available != PortAvailable::No {
                        Some(Port {
                            name:        port
                                .name
                                .as_ref()
                                .map_or(String::default(), ToString::to_string,),
                            description: port
                                .description
                                .as_ref()
                                .map_or(String::default(), ToString::to_string,),
                            device_type: match port.r#type {
                                DevicePortType::Headphones => DeviceType::Headphones,
                                DevicePortType::Speaker => DeviceType::Speaker,
                                DevicePortType::Headset => DeviceType::Headset,
                                DevicePortType::HDMI => DeviceType::Hdmi,
                                _ => DeviceType::Speaker,
                            },
                            active:      value.active_port.as_ref().and_then(|p| p.name.as_ref(),)
                                == port.name.as_ref(),
                        },)
                    } else {
                        None
                    }
                },)
                .collect::<Vec<_,>>(),
        }
    }
}

impl From<&SourceInfo<'_,>,> for Device
{
    fn from(value: &SourceInfo<'_,>,) -> Self
    {
        Self {
            name:        value.name.as_ref().map_or(String::default(), ToString::to_string,),
            description: value.proplist.get_str("device.description",).unwrap_or_default(),
            volume:      value.volume,
            is_mute:     value.mute,
            in_use:      value.state == SourceState::Running,
            ports:       value
                .ports
                .iter()
                .filter_map(|port| {
                    if port.available != PortAvailable::No {
                        Some(Port {
                            name:        port
                                .name
                                .as_ref()
                                .map_or(String::default(), ToString::to_string,),
                            description: port
                                .description
                                .as_ref()
                                .map_or(String::default(), ToString::to_string,),
                            device_type: match port.r#type {
                                DevicePortType::Headphones => DeviceType::Headphones,
                                DevicePortType::Speaker => DeviceType::Speaker,
                                DevicePortType::Headset => DeviceType::Headset,
                                DevicePortType::HDMI => DeviceType::Hdmi,
                                _ => DeviceType::Speaker,
                            },
                            active:      value.active_port.as_ref().and_then(|p| p.name.as_ref(),)
                                == port.name.as_ref(),
                        },)
                    } else {
                        None
                    }
                },)
                .collect::<Vec<_,>>(),
        }
    }
}
