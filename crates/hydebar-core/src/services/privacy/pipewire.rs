use std::{future::Future, pin::Pin, thread};

use pipewire::{context::ContextRc, core::CoreRc, main_loop::MainLoopRc};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    oneshot,
};

use crate::services::privacy::{ApplicationNode, Media, PrivacyError, PrivacyEvent};

/// Provides access to privacy events published by PipeWire.
pub(crate) trait PipewireEventSource
{
    /// Future returned when subscribing to PipeWire notifications.
    type Future<'a,>: Future<Output = Result<UnboundedReceiver<PrivacyEvent,>, PrivacyError,>,>
        + Send
        + 'a
    where
        Self: 'a;

    /// Subscribe to PipeWire privacy notifications.
    fn subscribe(&self,) -> Self::Future<'_,>;
}

/// Factory creating PipeWire-backed privacy event receivers.
#[derive(Debug, Default, Clone, Copy,)]
pub(crate) struct PipewireListener;

impl PipewireListener
{
    async fn create_receiver(&self,) -> Result<UnboundedReceiver<PrivacyEvent,>, PrivacyError,>
    {
        let (tx, rx,) = unbounded_channel::<PrivacyEvent,>();
        let (init_tx, init_rx,) = oneshot::channel::<Result<(), PrivacyError,>,>();

        let builder = thread::Builder::new().name("privacy-pipewire".into(),);
        builder
            .spawn(move || {
                struct PipewireRuntime
                {
                    mainloop:  MainLoopRc,
                    _context:  ContextRc,
                    _core:     CoreRc,
                    _listener: pipewire::registry::Listener,
                }

                impl PipewireRuntime
                {
                    fn new(tx: UnboundedSender<PrivacyEvent,>,) -> Result<Self, PrivacyError,>
                    {
                        let mainloop = MainLoopRc::new(None,)
                            .map_err(|err| PrivacyError::pipewire_mainloop(err.to_string(),),)?;
                        let context = ContextRc::new(&mainloop, None,)
                            .map_err(|err| PrivacyError::pipewire_context(err.to_string(),),)?;
                        let core = context
                            .connect_rc(None,)
                            .map_err(|err| PrivacyError::pipewire_core(err.to_string(),),)?;
                        let registry = core
                            .get_registry_rc()
                            .map_err(|err| PrivacyError::pipewire_registry(err.to_string(),),)?;
                        let remove_tx = tx.clone();
                        let listener = registry
                            .add_listener_local()
                            .global({
                                let tx = tx.clone();
                                move |global| {
                                    if let Some(props,) = global.props
                                        && let Some(media,) =
                                            props.get("media.class",).filter(|value| {
                                                *value == "Stream/Input/Video"
                                                    || *value == "Stream/Input/Audio"
                                            },)
                                    {
                                        let event = PrivacyEvent::AddNode(ApplicationNode {
                                            id:    global.id,
                                            media: if media == "Stream/Input/Video" {
                                                Media::Video
                                            } else {
                                                Media::Audio
                                            },
                                        },);
                                        if let Err(error,) = tx.send(event,) {
                                            log::warn!(
                                                "Failed to forward PipeWire add event: {error}"
                                            );
                                        }
                                    }
                                }
                            },)
                            .global_remove(move |id| {
                                if let Err(error,) = remove_tx.send(PrivacyEvent::RemoveNode(id,),)
                                {
                                    log::warn!("Failed to forward PipeWire remove event: {error}");
                                }
                            },)
                            .register();

                        Ok(Self {
                            mainloop,
                            _context: context,
                            _core: core,
                            _listener: listener,
                        },)
                    }

                    fn run(self,)
                    {
                        self.mainloop.run();
                    }
                }

                match PipewireRuntime::new(tx,) {
                    Ok(runtime,) => {
                        if init_tx.send(Ok((),),).is_err() {
                            log::warn!(
                                "PipeWire initialisation receiver dropped before completion"
                            );
                            return;
                        }
                        runtime.run();
                        log::warn!("PipeWire mainloop exited");
                    }
                    Err(error,) => {
                        log::error!("Failed to initialise PipeWire: {error}");
                        if init_tx.send(Err(error.clone(),),).is_err() {
                            log::warn!(
                                "Unable to report PipeWire initialisation failure: {error}"
                            );
                        }
                    }
                }
            },)
            .map_err(|err| {
                PrivacyError::channel(format!("failed to spawn PipeWire listener thread: {err}"),)
            },)?;

        match init_rx.await {
            Ok(Ok((),),) => Ok(rx,),
            Ok(Err(err,),) => Err(err,),
            Err(_,) => {
                Err(PrivacyError::channel("failed to receive PipeWire initialisation result",),)
            }
        }
    }
}

impl PipewireEventSource for PipewireListener
{
    type Future<'a,>
        = Pin<
        Box<
            dyn Future<Output = Result<UnboundedReceiver<PrivacyEvent,>, PrivacyError,>,>
                + Send
                + 'a,
        >,
    >
    where
        Self: 'a;

    fn subscribe(&self,) -> Self::Future<'_,>
    {
        Box::pin(self.create_receiver(),)
    }
}
