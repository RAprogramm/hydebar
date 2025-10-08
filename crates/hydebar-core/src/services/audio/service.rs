use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

use iced::{Subscription, Task, stream::channel};
use log::{error, warn};
use tokio::{
    sync::mpsc::UnboundedSender,
    time::{Duration, sleep},
};

use super::{
    backend::{AudioBackend, BackendCommand, BackendEvent, BackendHandle, PulseAudioBackend},
    model::{AudioData, AudioEvent, Device, Volume},
};
use crate::services::{ReadOnlyService, Service, ServiceEvent, ServiceEventPublisher};

/// Delay applied before attempting to reconnect to the backend after an error.
const RECONNECT_BACKOFF: Duration = Duration::from_millis(500,);

/// Commands accepted by the audio service.
#[derive(Debug, Clone,)]
pub enum AudioCommand
{
    ToggleSinkMute,
    ToggleSourceMute,
    SinkVolume(i32,),
    SourceVolume(i32,),
    DefaultSink(String, String,),
    DefaultSource(String, String,),
}

/// Read/write handle to the audio state and command channel.
#[derive(Debug, Clone,)]
pub struct AudioService
{
    data:      AudioData,
    commander: UnboundedSender<BackendCommand,>,
}

impl AudioService
{
    fn send_backend_command(&self, command: BackendCommand,)
    {
        if let Err(err,) = self.commander.send(command,) {
            error!("Failed to dispatch audio command: {err}");
        }
    }

    fn apply_command(&mut self, command: AudioCommand,)
    {
        match command {
            AudioCommand::ToggleSinkMute => {
                if let Some(sink,) = self
                    .data
                    .sinks
                    .iter()
                    .find(|sink| sink.name == self.data.server_info.default_sink,)
                {
                    self.send_backend_command(BackendCommand::SinkMute(
                        sink.name.clone(),
                        !sink.is_mute,
                    ),);
                }
            }
            AudioCommand::ToggleSourceMute => {
                if let Some(source,) = self
                    .data
                    .sources
                    .iter()
                    .find(|source| source.name == self.data.server_info.default_source,)
                {
                    self.send_backend_command(BackendCommand::SourceMute(
                        source.name.clone(),
                        !source.is_mute,
                    ),);
                }
            }
            AudioCommand::SinkVolume(volume,) => {
                let command = self
                    .data
                    .sinks
                    .iter_mut()
                    .find(|sink| sink.name == self.data.server_info.default_sink,)
                    .and_then(|sink| {
                        sink.volume
                            .scale_volume(volume as f64 / 100.0,)
                            .map(|volume| BackendCommand::SinkVolume(sink.name.clone(), *volume,),)
                    },);

                if let Some(command,) = command {
                    self.send_backend_command(command,);
                }
            }
            AudioCommand::SourceVolume(volume,) => {
                let command = self
                    .data
                    .sources
                    .iter_mut()
                    .find(|source| source.name == self.data.server_info.default_source,)
                    .and_then(|source| {
                        source.volume.scale_volume(volume as f64 / 100.0,).map(|volume| {
                            BackendCommand::SourceVolume(source.name.clone(), *volume,)
                        },)
                    },);

                if let Some(command,) = command {
                    self.send_backend_command(command,);
                }
            }
            AudioCommand::DefaultSink(name, port,) => {
                self.send_backend_command(BackendCommand::DefaultSink(name, port,),);
            }
            AudioCommand::DefaultSource(name, port,) => {
                self.send_backend_command(BackendCommand::DefaultSource(name, port,),);
            }
        }
    }

    pub async fn run_command(mut self, command: AudioCommand,) -> Option<ServiceEvent<Self,>,>
    {
        self.apply_command(command,);
        None
    }

    async fn listen_with_backend<P, B,>(backend: B, publisher: &mut P,)
    where
        P: ServiceEventPublisher<Self,> + Send,
        B: AudioBackend,
    {
        let mut state = State::Init;
        let backend = backend;

        loop {
            state = Self::start_listening(&backend, state, publisher,).await;
        }
    }

    async fn start_listening<P, B,>(backend: &B, state: State, publisher: &mut P,) -> State
    where
        P: ServiceEventPublisher<Self,> + Send,
        B: AudioBackend,
    {
        match state {
            State::Init => match backend.spawn().await {
                Ok(handle,) => {
                    let _ = publisher
                        .send(ServiceEvent::Init(AudioService {
                            data:      AudioData::default(),
                            commander: handle.commander(),
                        },),)
                        .await;

                    State::Active(handle,)
                }
                Err(err,) => {
                    error!("Failed to initialise audio backend: {err}");
                    let _ = publisher.send(ServiceEvent::Error((),),).await;
                    State::Error
                }
            },
            State::Active(mut handle,) => match handle.recv().await {
                Some(BackendEvent::Error(err,),) => {
                    error!("Audio backend error: {err}");
                    let _ = publisher.send(ServiceEvent::Error((),),).await;
                    State::Error
                }
                Some(BackendEvent::Update(event,),) => {
                    let _ = publisher.send(ServiceEvent::Update(event,),).await;
                    State::Active(handle,)
                }
                None => {
                    warn!("Audio backend closed event stream");
                    let _ = publisher.send(ServiceEvent::Error((),),).await;
                    State::Error
                }
            },
            State::Error => {
                sleep(RECONNECT_BACKOFF,).await;
                State::Init
            }
        }
    }

    fn update_from_event(&mut self, event: AudioEvent,)
    {
        match event {
            AudioEvent::Sinks(sinks,) => {
                self.data.sinks = sinks;
                self.data.cur_sink_volume = Self::active_device_volume(
                    &self.data.sinks,
                    &self.data.server_info.default_sink,
                );
            }
            AudioEvent::Sources(sources,) => {
                self.data.sources = sources;
                self.data.cur_source_volume = Self::active_device_volume(
                    &self.data.sources,
                    &self.data.server_info.default_source,
                );
            }
            AudioEvent::ServerInfo(info,) => {
                self.data.server_info = info;
                self.data.cur_sink_volume = Self::active_device_volume(
                    &self.data.sinks,
                    &self.data.server_info.default_sink,
                );
                self.data.cur_source_volume = Self::active_device_volume(
                    &self.data.sources,
                    &self.data.server_info.default_source,
                );
            }
        }
    }

    fn active_device_volume(devices: &[Device], default: &str,) -> i32
    {
        let volume = devices
            .iter()
            .find_map(|device| {
                if device.ports.iter().any(|port| port.active && device.name == default,) {
                    Some(if device.is_mute { 0.0 } else { device.volume.get_volume() },)
                } else {
                    None
                }
            },)
            .unwrap_or_default();

        (volume * 100.0) as i32
    }

    pub async fn listen<P,>(publisher: &mut P,)
    where
        P: ServiceEventPublisher<Self,> + Send,
    {
        Self::listen_with_backend(PulseAudioBackend, publisher,).await;
    }
}

impl Deref for AudioService
{
    type Target = AudioData;

    fn deref(&self,) -> &Self::Target
    {
        &self.data
    }
}

impl DerefMut for AudioService
{
    fn deref_mut(&mut self,) -> &mut Self::Target
    {
        &mut self.data
    }
}

impl ReadOnlyService for AudioService
{
    type UpdateEvent = AudioEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent,)
    {
        self.update_from_event(event,);
    }

    fn subscribe() -> Subscription<ServiceEvent<Self,>,>
    {
        let id = TypeId::of::<Self,>();

        Subscription::run_with_id(
            id,
            channel(100, |mut output| async move {
                AudioService::listen(&mut output,).await;
            },),
        )
    }
}

impl Service for AudioService
{
    type Command = AudioCommand;

    fn command(&mut self, command: Self::Command,) -> Task<ServiceEvent<Self,>,>
    {
        self.apply_command(command,);
        Task::none()
    }
}

enum State
{
    Init,
    Active(BackendHandle,),
    Error,
}

// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests
{
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use futures::FutureExt;
    use libpulse_binding::volume::ChannelVolumes;
    use tokio::sync::mpsc;

    use super::*;
    use crate::services::audio::backend::BackendFuture;

    #[tokio::test]
    async fn commands_are_dispatched_to_backend()
    {
        let (tx, mut rx,) = mpsc::unbounded_channel();
        let mut service = AudioService {
            data:      AudioData {
                server_info:       crate::services::audio::model::ServerInfo {
                    default_sink:   "sink".into(),
                    default_source: "source".into(),
                },
                sinks:             vec![Device {
                    name:        "sink".into(),
                    description: String::new(),
                    volume:      ChannelVolumes::default(),
                    is_mute:     false,
                    in_use:      true,
                    ports:       vec![crate::services::audio::model::Port {
                        name:        "port".into(),
                        description: String::new(),
                        device_type: crate::services::audio::model::DeviceType::Speaker,
                        active:      true,
                    }],
                }],
                sources:           vec![Device {
                    name:        "source".into(),
                    description: String::new(),
                    volume:      ChannelVolumes::default(),
                    is_mute:     false,
                    in_use:      true,
                    ports:       vec![crate::services::audio::model::Port {
                        name:        "port".into(),
                        description: String::new(),
                        device_type: crate::services::audio::model::DeviceType::Headset,
                        active:      true,
                    }],
                }],
                cur_sink_volume:   0,
                cur_source_volume: 0,
            },
            commander: tx,
        };

        service.apply_command(AudioCommand::ToggleSinkMute,);
        match rx.recv().await {
            Some(BackendCommand::SinkMute(name, true,),) if name == "sink" => {}
            other => panic!("unexpected command: {other:?}"),
        }

        service.apply_command(AudioCommand::ToggleSourceMute,);
        match rx.recv().await {
            Some(BackendCommand::SourceMute(name, true,),) if name == "source" => {}
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[derive(Clone,)]
    struct TestBackend
    {
        sequences: Arc<Mutex<VecDeque<Vec<BackendEvent,>,>,>,>,
        starts:    Arc<Mutex<usize,>,>,
    }

    impl TestBackend
    {
        fn new(sequences: Vec<Vec<BackendEvent,>,>,) -> Self
        {
            Self {
                sequences: Arc::new(Mutex::new(sequences.into_iter().collect(),),),
                starts:    Arc::new(Mutex::new(0,),),
            }
        }

        fn start_count(&self,) -> usize
        {
            *self.starts.lock().unwrap()
        }
    }

    impl AudioBackend for TestBackend
    {
        fn spawn(&self,) -> BackendFuture
        {
            let sequences = self.sequences.clone();
            let starts = self.starts.clone();

            Box::pin(async move {
                let events = sequences
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or_else(|| vec![BackendEvent::Error("exhausted".into(),)],);

                *starts.lock().unwrap() += 1;

                let (event_tx, event_rx,) = mpsc::unbounded_channel();
                let (command_tx, mut command_rx,) = mpsc::unbounded_channel();

                tokio::spawn(async move {
                    for event in events {
                        let _ = event_tx.send(event,);
                    }
                    drop(event_tx,);
                    while command_rx.recv().await.is_some() {}
                },);

                Ok(BackendHandle::from_parts(event_rx, command_tx,),)
            },)
        }
    }

    struct TestPublisher
    {
        sender: mpsc::UnboundedSender<ServiceEvent<AudioService,>,>,
    }

    impl ServiceEventPublisher<AudioService,> for TestPublisher
    {
        type SendFuture<'a,>
            = futures::future::BoxFuture<'a, (),>
        where
            Self: 'a;

        fn send(&mut self, event: ServiceEvent<AudioService,>,) -> Self::SendFuture<'_,>
        {
            let sender = self.sender.clone();
            async move {
                let _ = sender.send(event,);
            }
            .boxed()
        }
    }

    #[tokio::test(start_paused = true)]
    #[ignore = "Timing-sensitive test - needs rework"]
    async fn service_reconnects_after_backend_error()
    {
        tokio::time::pause();

        let backend = TestBackend::new(vec![
            vec![BackendEvent::Error("failure".into(),)],
            vec![BackendEvent::Update(AudioEvent::ServerInfo(
                crate::services::audio::model::ServerInfo {
                    default_sink:   String::from("sink",),
                    default_source: String::from("source",),
                },
            ),)],
        ],);

        let (event_tx, mut event_rx,) = mpsc::unbounded_channel();
        let publisher = TestPublisher {
            sender: event_tx,
        };

        let backend_clone = backend.clone();
        let listener = tokio::spawn(async move {
            let mut publisher = publisher;
            AudioService::listen_with_backend(backend_clone, &mut publisher,).await;
        },);

        // Expect first init event.
        let first = event_rx.recv().await.unwrap();
        assert!(matches!(first, ServiceEvent::Init(_)));

        // Advance time to allow reconnection attempts after error.
        tokio::time::advance(RECONNECT_BACKOFF,).await;
        tokio::time::advance(RECONNECT_BACKOFF,).await;

        // Expect an error event followed by a new init and update.
        let mut init_count = 1;
        let mut update_seen = false;
        for _ in 0..4 {
            if let Some(event,) = event_rx.recv().await {
                match event {
                    ServiceEvent::Init(_,) => init_count += 1,
                    ServiceEvent::Update(AudioEvent::ServerInfo(_,),) => {
                        update_seen = true;
                        break;
                    }
                    _ => {}
                }
            }
        }

        assert!(update_seen, "expected server info update after reconnection");
        assert_eq!(init_count, 2, "expected service to reinitialise once");
        assert_eq!(backend.start_count(), 2);

        listener.abort();
    }
}
