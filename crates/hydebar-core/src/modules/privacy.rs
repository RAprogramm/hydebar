use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, ModuleEventSender,
    components::icons::{Icons, icon},
    event_bus::ModuleEvent,
    services::{
        ReadOnlyService, ServiceEvent,
        privacy::{PrivacyEventPublisher, PrivacyService, State, error::PrivacyError},
    },
};
use iced::{
    Alignment, Element,
    widget::{Row, container},
};
use log::{error, warn};
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
};
use tokio::task::JoinHandle;

#[cfg(test)]
use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

#[cfg(test)]
use crate::event_bus::{BusEvent, EventBus, ModuleEvent as BusModuleEvent};

#[cfg(test)]
use iced::futures::future::pending;

#[cfg(test)]
use tokio::time::timeout;

/// Message emitted by the privacy module subscription.
#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    Event(ServiceEvent<PrivacyService>),
}

/// UI module exposing privacy information icons.
#[derive(Debug, Default)]
pub struct Privacy {
    pub service: Option<PrivacyService>,
    sender: Option<ModuleEventSender<PrivacyMessage>>,
    tasks: Vec<JoinHandle<()>>,
}

impl Privacy {
    /// Update the module state based on new privacy events.
    pub fn update(&mut self, message: PrivacyMessage) {
        if let PrivacyMessage::Event(event) = message {
            match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                }
                ServiceEvent::Update(data) => {
                    if let Some(privacy) = self.service.as_mut() {
                        privacy.update(data);
                    }
                }
                ServiceEvent::Error(error) => match error {
                    PrivacyError::WebcamUnavailable => {
                        warn!(
                            "Webcam device unavailable; continuing with PipeWire-only privacy data"
                        );
                    }
                    _ => error!("Privacy service error: {error}"),
                },
            }
        }
    }
}

impl Module for Privacy {
    type ViewData<'a> = ();
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        for task in self.tasks.drain(..) {
            task.abort();
        }

        let sender = ctx.module_sender(ModuleEvent::Privacy);
        let mut publisher = ModulePublisher::new(sender.clone());
        let mut error_sender = sender.clone();

        let task = ctx.runtime_handle().spawn(async move {
            let mut state = State::Init;

            loop {
                match run_start_listening(state, &mut publisher).await {
                    Ok(next_state) => {
                        state = next_state;
                    }
                    Err(error) => {
                        if let Err(err) = error_sender
                            .try_send(PrivacyMessage::Event(ServiceEvent::Error(error.clone())))
                        {
                            warn!("failed to publish privacy service error: {err}");
                            break;
                        }

                        state = State::Init;
                    }
                }
            }
        });

        self.sender = Some(sender);
        self.tasks.push(task);

        Ok(())
    }

    /// Render the privacy indicator when data is available.
    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if let Some(service) = self.service.as_ref() {
            if !service.no_access() {
                Some((
                    container(
                        Row::new()
                            .push_maybe(
                                service
                                    .screenshare_access()
                                    .then(|| icon(Icons::ScreenShare)),
                            )
                            .push_maybe(service.webcam_access().then(|| icon(Icons::Webcam)))
                            .push_maybe(service.microphone_access().then(|| icon(Icons::Mic1)))
                            .align_y(Alignment::Center)
                            .spacing(8),
                    )
                    .style(|theme| container::Style {
                        text_color: Some(theme.extended_palette().danger.weak.color),
                        ..Default::default()
                    })
                    .into(),
                    None,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}

struct ModulePublisher {
    sender: ModuleEventSender<PrivacyMessage>,
}

impl ModulePublisher {
    fn new(sender: ModuleEventSender<PrivacyMessage>) -> Self {
        Self { sender }
    }
}

impl PrivacyEventPublisher for ModulePublisher {
    type SendFuture<'a>
        = Ready<Result<(), PrivacyError>>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<PrivacyService>) -> Self::SendFuture<'_> {
        ready(
            self.sender
                .try_send(PrivacyMessage::Event(event))
                .map_err(|err| {
                    PrivacyError::channel(format!("failed to publish privacy event: {err}"))
                }),
        )
    }
}

type StartListeningFuture<'a> =
    Pin<Box<dyn Future<Output = Result<State, PrivacyError>> + Send + 'a>>;

fn run_start_listening<'a>(
    state: State,
    publisher: &'a mut dyn PrivacyEventPublisher,
) -> StartListeningFuture<'a> {
    #[cfg(test)]
    {
        if let Some(callback) = start_listening_override()
            .lock()
            .expect("start listening override mutex poisoned")
            .clone()
        {
            return callback(state, publisher);
        }
    }

    Box::pin(PrivacyService::start_listening(state, publisher))
}

#[cfg(test)]
type StartListeningCallback = Arc<
    dyn for<'a> Fn(State, &'a mut dyn PrivacyEventPublisher) -> StartListeningFuture<'a>
        + Send
        + Sync,
>;

#[cfg(test)]
fn start_listening_override() -> &'static Mutex<Option<StartListeningCallback>> {
    static OVERRIDE: OnceLock<Mutex<Option<StartListeningCallback>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_start_listening_override(callback: Option<StartListeningCallback>) {
    let mut slot = start_listening_override()
        .lock()
        .expect("start listening override mutex poisoned");
    *slot = callback;
}

#[cfg(test)]
struct OverrideGuard;

#[cfg(test)]
impl OverrideGuard {
    fn install(callback: Option<StartListeningCallback>) -> Self {
        set_start_listening_override(callback);
        Self
    }
}

#[cfg(test)]
impl Drop for OverrideGuard {
    fn drop(&mut self) {
        set_start_listening_override(None);
    }
}

#[cfg(test)]
async fn recv_event(receiver: &mut crate::event_bus::EventReceiver) -> BusEvent {
    loop {
        if let Some(event) = receiver
            .try_recv()
            .expect("event bus receiver should not be poisoned")
        {
            return event;
        }

        tokio::task::yield_now().await;
    }
}

#[cfg(test)]
struct CancellationProbe {
    flag: Arc<AtomicBool>,
}

#[cfg(test)]
impl Drop for CancellationProbe {
    fn drop(&mut self) {
        self.flag.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reports_listener_errors_via_event_bus() {
        let error = PrivacyError::channel("boom");
        let error_clone = error.clone();

        let callback: StartListeningCallback = Arc::new(move |state, _publisher| {
            let err = error_clone.clone();
            Box::pin(async move {
                let _ = state;
                Err(err)
            })
        });
        let _guard = OverrideGuard::install(Some(callback));

        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let mut receiver = bus.receiver();
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let mut privacy = Privacy::default();
        assert!(privacy.register(&context, ()).is_ok());

        let event = timeout(Duration::from_secs(1), recv_event(&mut receiver))
            .await
            .expect("privacy event should be emitted");

        match event {
            BusEvent::Module(BusModuleEvent::Privacy(PrivacyMessage::Event(
                ServiceEvent::Error(err),
            ))) => {
                assert_eq!(err, error);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        for task in privacy.tasks.drain(..) {
            task.abort();
        }
    }

    #[tokio::test]
    async fn aborts_previous_listener_tasks_on_re_registration() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let call_count = Arc::new(AtomicUsize::new(0));

        let callback: StartListeningCallback = Arc::new({
            let cancelled = Arc::clone(&cancelled);
            let call_count = Arc::clone(&call_count);
            move |state, _publisher| {
                call_count.fetch_add(1, Ordering::SeqCst);
                let next_state = state;
                let flag = Arc::clone(&cancelled);
                Box::pin(async move {
                    let _probe = CancellationProbe { flag };
                    pending::<()>().await;
                    Ok(next_state)
                })
            }
        });
        let _guard = OverrideGuard::install(Some(callback));

        let bus = EventBus::new(NonZeroUsize::new(4).expect("non-zero capacity"));
        let context = ModuleContext::new(bus.sender(), tokio::runtime::Handle::current());

        let mut privacy = Privacy::default();
        assert!(privacy.register(&context, ()).is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        assert!(privacy.register(&context, ()).is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        timeout(Duration::from_secs(1), async {
            loop {
                if cancelled.load(Ordering::SeqCst) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("previous listener should be cancelled");

        for task in privacy.tasks.drain(..) {
            task.abort();
        }
    }
}
