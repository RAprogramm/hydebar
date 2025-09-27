use super::{ReadOnlyService, ServiceEvent};
pub mod error;

use self::error::PrivacyError;
use iced::{
    Subscription,
    futures::{FutureExt, Stream, StreamExt, channel::mpsc::Sender, select, stream::pending},
    stream::channel,
};
use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, info, warn};
use pipewire::{context::ContextRc, core::CoreRc, main_loop::MainLoopRc};
use std::{any::TypeId, fs, future::Future, ops::Deref, path::Path, pin::Pin, thread};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    oneshot,
};

const WEBCAM_DEVICE_PATH: &str = "/dev/video0";

type PrivacyStream = Pin<Box<dyn Stream<Item = PrivacyEvent> + Send>>;

/// Sink used to publish privacy service events to interested consumers.
pub trait PrivacyEventPublisher {
    type SendFuture<'a>: Future<Output = Result<(), PrivacyError>> + Send + 'a
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<PrivacyService>) -> Self::SendFuture<'_>;
}

impl PrivacyEventPublisher for Sender<ServiceEvent<PrivacyService>> {
    type SendFuture<'a> = Pin<Box<dyn Future<Output = Result<(), PrivacyError>> + Send + 'a>>;

    fn send(&mut self, event: ServiceEvent<PrivacyService>) -> Self::SendFuture<'_> {
        Box::pin(async move {
            self.send(event)
                .await
                .map_err(|err| PrivacyError::channel(err.to_string()))
        })
    }
}

/// Media class reported by PipeWire for an application node.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Media {
    /// The node represents a video stream, typically screen sharing.
    Video,
    /// The node represents an audio stream, typically microphone usage.
    Audio,
}

/// Metadata describing an application node that is accessing privacy-sensitive resources.
#[derive(Debug, Clone)]
pub struct ApplicationNode {
    /// Identifier assigned by PipeWire.
    pub id: u32,
    /// Media classification of the node.
    pub media: Media,
}

/// Aggregated privacy information exposed to UI consumers.
#[derive(Debug, Clone)]
pub struct PrivacyData {
    nodes: Vec<ApplicationNode>,
    webcam_access: i32,
}

impl PrivacyData {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            webcam_access: is_device_in_use(WEBCAM_DEVICE_PATH),
        }
    }

    /// Returns `true` when no privacy-sensitive resources are currently in use.
    pub fn no_access(&self) -> bool {
        self.nodes.is_empty() && self.webcam_access == 0
    }

    /// Returns `true` when an audio input node is active.
    pub fn microphone_access(&self) -> bool {
        self.nodes.iter().any(|n| n.media == Media::Audio)
    }

    /// Returns `true` while the webcam device is reported as in use.
    pub fn webcam_access(&self) -> bool {
        self.webcam_access > 0
    }

    /// Returns `true` when a video capture node (typically screen sharing) is active.
    pub fn screenshare_access(&self) -> bool {
        self.nodes.iter().any(|n| n.media == Media::Video)
    }
}

/// Service exposing read-only privacy state to interested modules.
#[derive(Debug, Clone)]
pub struct PrivacyService {
    data: PrivacyData,
}

impl Deref for PrivacyService {
    type Target = PrivacyData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl PrivacyService {
    async fn create_pipewire_listener() -> Result<UnboundedReceiver<PrivacyEvent>, PrivacyError> {
        let (tx, rx) = unbounded_channel::<PrivacyEvent>();
        let (init_tx, init_rx) = oneshot::channel::<Result<(), PrivacyError>>();

        let builder = thread::Builder::new().name("privacy-pipewire".into());
        builder
            .spawn(move || {
                struct PipewireRuntime {
                    mainloop: MainLoopRc,
                    _context: ContextRc,
                    _core: CoreRc,
                    _listener: pipewire::registry::Listener,
                }

                impl PipewireRuntime {
                    fn new(tx: UnboundedSender<PrivacyEvent>) -> Result<Self, PrivacyError> {
                        let mainloop = MainLoopRc::new(None)
                            .map_err(|err| PrivacyError::pipewire_mainloop(err.to_string()))?;
                        let context = ContextRc::new(&mainloop, None)
                            .map_err(|err| PrivacyError::pipewire_context(err.to_string()))?;
                        let core = context
                            .connect_rc(None)
                            .map_err(|err| PrivacyError::pipewire_core(err.to_string()))?;
                        let registry = core
                            .get_registry_rc()
                            .map_err(|err| PrivacyError::pipewire_registry(err.to_string()))?;
                        let remove_tx = tx.clone();
                        let listener = registry
                            .add_listener_local()
                            .global({
                                let tx = tx.clone();
                                move |global| {
                                    if let Some(props) = global.props {
                                        if let Some(media) = props.get("media.class").filter(|v| {
                                            *v == "Stream/Input/Video" || *v == "Stream/Input/Audio"
                                        }) {
                                            debug!("New global: {global:?}");
                                            let event = PrivacyEvent::AddNode(ApplicationNode {
                                                id: global.id,
                                                media: if media == "Stream/Input/Video" {
                                                    Media::Video
                                                } else {
                                                    Media::Audio
                                                },
                                            });
                                            if let Err(err) = tx.send(event) {
                                                warn!(
                                                    "Failed to forward PipeWire add event: {err}"
                                                );
                                            }
                                        }
                                    }
                                }
                            })
                            .global_remove(move |id| {
                                debug!("Remove global: {id}");
                                if let Err(err) = remove_tx.send(PrivacyEvent::RemoveNode(id)) {
                                    warn!("Failed to forward PipeWire remove event: {err}");
                                }
                            })
                            .register();

                        Ok(Self {
                            mainloop,
                            _context: context,
                            _core: core,
                            _listener: listener,
                        })
                    }

                    fn run(self) {
                        self.mainloop.run();
                    }
                }

                match PipewireRuntime::new(tx) {
                    Ok(runtime) => {
                        if init_tx.send(Ok(())).is_err() {
                            warn!("PipeWire initialisation receiver dropped before completion");
                            return;
                        }
                        runtime.run();
                        warn!("PipeWire mainloop exited");
                    }
                    Err(err) => {
                        error!("Failed to initialise PipeWire: {err}");
                        if init_tx.send(Err(err.clone())).is_err() {
                            warn!("Unable to report PipeWire initialisation failure: {err}");
                        }
                    }
                }
            })
            .map_err(|err| {
                PrivacyError::channel(format!("failed to spawn PipeWire listener thread: {err}"))
            })?;

        match init_rx.await {
            Ok(Ok(())) => Ok(rx),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(PrivacyError::channel(
                "failed to receive PipeWire initialisation result",
            )),
        }
    }

    async fn webcam_listener() -> Result<PrivacyStream, PrivacyError> {
        let inotify = Inotify::init().map_err(|err| PrivacyError::inotify_init(err.to_string()))?;
        match inotify.watches().add(
            WEBCAM_DEVICE_PATH,
            WatchMask::CLOSE_WRITE
                | WatchMask::CLOSE_NOWRITE
                | WatchMask::DELETE_SELF
                | WatchMask::OPEN
                | WatchMask::ATTRIB,
        ) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(PrivacyError::WebcamUnavailable);
            }
            Err(err) => {
                return Err(PrivacyError::inotify_watch(err.to_string()));
            }
        }

        let buffer = [0; 512];
        let stream = inotify
            .into_event_stream(buffer)
            .map_err(|err| PrivacyError::inotify_init(err.to_string()))?
            .filter_map(|event| async move {
                match event {
                    Ok(event) => {
                        debug!("Webcam event: {event:?}");
                        match event.mask {
                            EventMask::OPEN => Some(PrivacyEvent::WebcamOpen),
                            EventMask::CLOSE_WRITE | EventMask::CLOSE_NOWRITE => {
                                Some(PrivacyEvent::WebcamClose)
                            }
                            _ => None,
                        }
                    }
                    Err(err) => {
                        warn!("Failed to read webcam event: {err}");
                        None
                    }
                }
            })
            .boxed();

        Ok(stream)
    }

    async fn emit_event<P>(publisher: &mut P, event: ServiceEvent<Self>) -> Result<(), PrivacyError>
    where
        P: PrivacyEventPublisher,
    {
        publisher.send(event).await
    }

    pub(crate) async fn start_listening<P>(
        state: State,
        publisher: &mut P,
    ) -> Result<State, PrivacyError>
    where
        P: PrivacyEventPublisher,
    {
        Self::start_listening_with_factories(
            state,
            publisher,
            Self::create_pipewire_listener,
            Self::webcam_listener,
        )
        .await
    }

    async fn start_listening_with_factories<PF, PFut, WF, WFut, Pub>(
        state: State,
        publisher: &mut Pub,
        mut pipewire_factory: PF,
        mut webcam_factory: WF,
    ) -> Result<State, PrivacyError>
    where
        PF: FnMut() -> PFut,
        PFut: Future<Output = Result<UnboundedReceiver<PrivacyEvent>, PrivacyError>> + Send,
        WF: FnMut() -> WFut,
        WFut: Future<Output = Result<PrivacyStream, PrivacyError>> + Send,
        Pub: PrivacyEventPublisher,
    {
        match state {
            State::Init => {
                let pipewire = pipewire_factory().await?;
                let webcam = match webcam_factory().await {
                    Ok(stream) => stream,
                    Err(err @ PrivacyError::WebcamUnavailable) => {
                        warn!("{err}");
                        pending::<PrivacyEvent>().boxed()
                    }
                    Err(err) => return Err(err),
                };

                let data = PrivacyData::new();
                Self::emit_event(publisher, ServiceEvent::Init(PrivacyService { data })).await?;

                Ok(State::Active { pipewire, webcam })
            }
            State::Active {
                mut pipewire,
                mut webcam,
            } => {
                info!("Listening for privacy events");

                let mut webcam_pin = webcam.as_mut();
                let mut webcam_future = webcam_pin.next().fuse();

                select! {
                    value = pipewire.recv().fuse() => {
                        match value {
                            Some(event) => {
                                Self::emit_event(publisher, ServiceEvent::Update(event)).await?;
                            }
                            None => {
                                error!("PipeWire listener exited unexpectedly");
                                return Err(PrivacyError::channel(
                                    "pipewire listener closed unexpectedly",
                                ));
                            }
                        }
                    }
                    value = webcam_future => {
                        match value {
                            Some(event) => {
                                Self::emit_event(publisher, ServiceEvent::Update(event)).await?;
                            }
                            None => {
                                error!("Webcam listener exited unexpectedly");
                                return Err(PrivacyError::channel(
                                    "webcam listener closed unexpectedly",
                                ));
                            }
                        }
                    }
                };

                Ok(State::Active { pipewire, webcam })
            }
        }
    }

    #[cfg(test)]
    async fn start_listening_with<Pub, PF, PFut, WF, WFut>(
        state: State,
        publisher: &mut Pub,
        pipewire_factory: PF,
        webcam_factory: WF,
    ) -> Result<State, PrivacyError>
    where
        Pub: PrivacyEventPublisher,
        PF: FnMut() -> PFut,
        PFut: Future<Output = Result<UnboundedReceiver<PrivacyEvent>, PrivacyError>> + Send,
        WF: FnMut() -> WFut,
        WFut: Future<Output = Result<PrivacyStream, PrivacyError>> + Send,
    {
        Self::start_listening_with_factories(state, publisher, pipewire_factory, webcam_factory)
            .await
    }
}

pub(crate) enum State {
    Init,
    Active {
        pipewire: UnboundedReceiver<PrivacyEvent>,
        webcam: PrivacyStream,
    },
}

/// Event emitted by the privacy service listeners.
#[derive(Debug, Clone)]
pub enum PrivacyEvent {
    /// A new PipeWire node has been announced.
    AddNode(ApplicationNode),
    /// A PipeWire node has been removed.
    RemoveNode(u32),
    /// The webcam device has been opened by an application.
    WebcamOpen,
    /// The webcam device has been closed by an application.
    WebcamClose,
}

impl ReadOnlyService for PrivacyService {
    type UpdateEvent = PrivacyEvent;
    type Error = PrivacyError;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            PrivacyEvent::AddNode(node) => {
                self.data.nodes.push(node);
            }
            PrivacyEvent::RemoveNode(id) => {
                self.data.nodes.retain(|n| n.id != id);
            }
            PrivacyEvent::WebcamOpen => {
                self.data.webcam_access += 1;
                debug!("Webcam opened {}", self.data.webcam_access);
            }
            PrivacyEvent::WebcamClose => {
                self.data.webcam_access = i32::max(self.data.webcam_access - 1, 0);
                debug!("Webcam closed {}", self.data.webcam_access);
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    match PrivacyService::start_listening(state, &mut output).await {
                        Ok(next_state) => {
                            state = next_state;
                        }
                        Err(error) => {
                            if let Err(send_error) = PrivacyService::emit_event(
                                &mut output,
                                ServiceEvent::Error(error.clone()),
                            )
                            .await
                            {
                                warn!("Failed to emit privacy service error event: {send_error}");
                                break;
                            }

                            state = State::Init;
                        }
                    }
                }
            }),
        )
    }
}

fn is_device_in_use(target: &str) -> i32 {
    let mut used_by = 0;
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let pid_path = entry.path();

            // Skip non-numeric directories (not process folders)
            if !pid_path.join("fd").exists() {
                continue;
            }

            // Check file descriptors in each process folder
            if let Ok(fd_entries) = fs::read_dir(pid_path.join("fd")) {
                for fd_entry in fd_entries.flatten() {
                    if let Ok(link_path) = fs::read_link(fd_entry.path()) {
                        if link_path == Path::new(target) {
                            used_by += 1;
                        }
                    }
                }
            }
        }
    }

    used_by
}

#[cfg(test)]
mod tests {
    use super::{PrivacyEvent, PrivacyService, ServiceEvent, State, error::PrivacyError};
    use iced::futures::{StreamExt, channel::mpsc, stream::pending};
    use tokio::sync::mpsc::unbounded_channel;

    #[tokio::test]
    async fn init_succeeds_with_all_listeners() {
        let (pipewire_tx, pipewire_rx) = unbounded_channel();
        drop(pipewire_tx);
        let mut pipewire_rx = Some(pipewire_rx);
        let pipewire_factory = move || {
            let receiver = pipewire_rx
                .take()
                .expect("pipewire factory should only be called once");
            async move { Ok(receiver) }
        };

        let mut webcam_stream = Some(pending::<PrivacyEvent>().boxed());
        let webcam_factory = move || {
            let stream = webcam_stream
                .take()
                .expect("webcam factory should only be called once");
            async move { Ok(stream) }
        };

        let (mut output_tx, mut output_rx) = mpsc::channel(10);
        let state = State::Init;
        let state = PrivacyService::start_listening_with(
            state,
            &mut output_tx,
            pipewire_factory,
            webcam_factory,
        )
        .await
        .expect("initialisation should succeed");

        assert!(matches!(state, State::Active { .. }));
        let event = output_rx.next().await;
        assert!(matches!(event, Some(ServiceEvent::Init(_))));
    }

    #[tokio::test]
    async fn init_reports_pipewire_failure() {
        let pipewire_factory = || async { Err(PrivacyError::pipewire_mainloop("boom")) };
        let webcam_factory = || async { Ok(pending::<PrivacyEvent>().boxed()) };
        let (mut output_tx, _output_rx) = mpsc::channel(1);

        let result = PrivacyService::start_listening_with(
            State::Init,
            &mut output_tx,
            pipewire_factory,
            webcam_factory,
        )
        .await;
        assert!(matches!(result, Err(PrivacyError::PipewireMainloop { .. })));
    }

    #[tokio::test]
    async fn init_falls_back_when_webcam_missing() {
        let (pipewire_tx, pipewire_rx) = unbounded_channel();
        drop(pipewire_tx);
        let mut pipewire_rx = Some(pipewire_rx);
        let pipewire_factory = move || {
            let receiver = pipewire_rx
                .take()
                .expect("pipewire factory should only be called once");
            async move { Ok(receiver) }
        };

        let webcam_factory = || async { Err(PrivacyError::WebcamUnavailable) };
        let (mut output_tx, mut output_rx) = mpsc::channel(2);
        let state = PrivacyService::start_listening_with(
            State::Init,
            &mut output_tx,
            pipewire_factory,
            webcam_factory,
        )
        .await
        .expect("initialisation should succeed with webcam fallback");

        assert!(matches!(state, State::Active { .. }));
        let event = output_rx.next().await;
        assert!(matches!(event, Some(ServiceEvent::Init(_))));
    }

    #[tokio::test]
    async fn init_fails_when_output_channel_closed() {
        let (pipewire_tx, pipewire_rx) = unbounded_channel();
        drop(pipewire_tx);
        let mut pipewire_rx = Some(pipewire_rx);
        let pipewire_factory = move || {
            let receiver = pipewire_rx
                .take()
                .expect("pipewire factory should only be called once");
            async move { Ok(receiver) }
        };

        let webcam_factory = || async { Ok(pending::<PrivacyEvent>().boxed()) };
        let (mut output_tx, output_rx) = mpsc::channel::<ServiceEvent<PrivacyService>>(1);
        drop(output_rx);

        let result = PrivacyService::start_listening_with(
            State::Init,
            &mut output_tx,
            pipewire_factory,
            webcam_factory,
        )
        .await;
        assert!(matches!(result, Err(PrivacyError::Channel { .. })));
    }
}
