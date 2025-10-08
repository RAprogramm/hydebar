use super::{ReadOnlyService, ServiceEvent};
pub mod error;
pub mod inotify;
pub mod pipewire;
pub mod publisher;

pub use error::PrivacyError;
pub use publisher::PrivacyEventPublisher;

use self::{
    inotify::{WebcamEventSource, WebcamWatcher},
    pipewire::{PipewireEventSource, PipewireListener},
};
use iced::{
    Subscription,
    futures::{FutureExt, Stream, StreamExt, select, stream::pending},
    stream::channel,
};
use log::{debug, error, info, warn};
use std::{any::TypeId, fs, ops::Deref, path::Path, pin::Pin};
use tokio::sync::mpsc::UnboundedReceiver;

const WEBCAM_DEVICE_PATH: &str = "/dev/video0";

pub(crate) type PrivacyStream = Pin<Box<dyn Stream<Item = PrivacyEvent> + Send>>;

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
        self.nodes.iter().any(|node| node.media == Media::Audio)
    }

    /// Returns `true` while the webcam device is reported as in use.
    pub fn webcam_access(&self) -> bool {
        self.webcam_access > 0
    }

    /// Returns `true` when a video capture node (typically screen sharing) is active.
    pub fn screenshare_access(&self) -> bool {
        self.nodes.iter().any(|node| node.media == Media::Video)
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
        P: PrivacyEventPublisher + Send,
    {
        let pipewire = PipewireListener::default();
        let webcam = WebcamWatcher::new(Path::new(WEBCAM_DEVICE_PATH));
        Self::start_listening_with_sources(state, publisher, &pipewire, &webcam).await
    }

    async fn start_listening_with_sources<P, Pipewire, Webcam>(
        state: State,
        publisher: &mut P,
        pipewire_source: &Pipewire,
        webcam_source: &Webcam,
    ) -> Result<State, PrivacyError>
    where
        P: PrivacyEventPublisher,
        Pipewire: PipewireEventSource,
        Webcam: WebcamEventSource,
    {
        match state {
            State::Init => {
                let pipewire = pipewire_source.subscribe().await?;
                let webcam = match webcam_source.subscribe().await {
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
                self.data.nodes.retain(|node| node.id != id);
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

            if !pid_path.join("fd").exists() {
                continue;
            }

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
    use super::{
        ApplicationNode, Media, PrivacyEvent, PrivacyService, ServiceEvent, State,
        error::PrivacyError,
    };
    use crate::services::privacy::{inotify::WebcamEventSource, pipewire::PipewireEventSource};
    use iced::futures::{StreamExt, channel::mpsc, future, stream};
    use std::{
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
    };
    use tokio::sync::mpsc::unbounded_channel;

    #[derive(Default)]
    struct TestPipewireSource {
        receiver:
            Mutex<Option<Result<tokio::sync::mpsc::UnboundedReceiver<PrivacyEvent>, PrivacyError>>>,
    }

    impl TestPipewireSource {
        fn new(receiver: tokio::sync::mpsc::UnboundedReceiver<PrivacyEvent>) -> Self {
            Self {
                receiver: Mutex::new(Some(Ok(receiver))),
            }
        }

        fn failing(error: PrivacyError) -> Self {
            Self {
                receiver: Mutex::new(Some(Err(error))),
            }
        }
    }

    impl PipewireEventSource for TestPipewireSource {
        type Future<'a>
            = Pin<
            Box<
                dyn Future<
                        Output = Result<
                            tokio::sync::mpsc::UnboundedReceiver<PrivacyEvent>,
                            PrivacyError,
                        >,
                    > + Send
                    + 'a,
            >,
        >
        where
            Self: 'a;

        fn subscribe(&self) -> Self::Future<'_> {
            let result = self
                .receiver
                .lock()
                .expect("pipewire receiver mutex poisoned")
                .take()
                .unwrap_or_else(|| Err(PrivacyError::channel("pipewire factory reused")));
            Box::pin(async move { result })
        }
    }

    #[derive(Default, Clone)]
    struct TestWebcamSource {
        stream: Arc<Mutex<Option<Result<super::PrivacyStream, PrivacyError>>>>,
    }

    impl TestWebcamSource {
        fn new(stream: super::PrivacyStream) -> Self {
            Self {
                stream: Arc::new(Mutex::new(Some(Ok(stream)))),
            }
        }

        fn failing(error: PrivacyError) -> Self {
            Self {
                stream: Arc::new(Mutex::new(Some(Err(error)))),
            }
        }
    }

    impl WebcamEventSource for TestWebcamSource {
        type Future<'a>
            = Pin<Box<dyn Future<Output = Result<super::PrivacyStream, PrivacyError>> + Send + 'a>>
        where
            Self: 'a;

        fn subscribe(&self) -> Self::Future<'_> {
            let result = self
                .stream
                .lock()
                .expect("webcam stream mutex poisoned")
                .take()
                .unwrap_or_else(|| Err(PrivacyError::channel("webcam factory reused")));
            Box::pin(async move { result })
        }
    }

    #[tokio::test]
    async fn init_succeeds_with_all_listeners() {
        let (pipewire_tx, pipewire_rx) = unbounded_channel();
        drop(pipewire_tx);
        let pipewire_source = TestPipewireSource::new(pipewire_rx);

        let webcam_stream = stream::pending::<PrivacyEvent>().boxed();
        let webcam_source = TestWebcamSource::new(webcam_stream);

        let (mut output_tx, mut output_rx) = mpsc::channel(10);
        let state = State::Init;
        let state = PrivacyService::start_listening_with_sources(
            state,
            &mut output_tx,
            &pipewire_source,
            &webcam_source,
        )
        .await
        .expect("initialisation should succeed");

        assert!(matches!(state, State::Active { .. }));
        let event = output_rx.next().await;
        assert!(matches!(event, Some(ServiceEvent::Init(_))));
    }

    #[tokio::test]
    async fn init_reports_pipewire_failure() {
        let pipewire_source = TestPipewireSource::failing(PrivacyError::pipewire_mainloop("boom"));
        let webcam_source = TestWebcamSource::new(stream::pending::<PrivacyEvent>().boxed());
        let (mut output_tx, _output_rx) = mpsc::channel(1);

        let result = PrivacyService::start_listening_with_sources(
            State::Init,
            &mut output_tx,
            &pipewire_source,
            &webcam_source,
        )
        .await;
        assert!(matches!(result, Err(PrivacyError::PipewireMainloop { .. })));
    }

    #[tokio::test]
    async fn init_falls_back_when_webcam_missing() {
        let (pipewire_tx, pipewire_rx) = unbounded_channel();
        drop(pipewire_tx);
        let pipewire_source = TestPipewireSource::new(pipewire_rx);

        let webcam_source = TestWebcamSource::failing(PrivacyError::WebcamUnavailable);
        let (mut output_tx, mut output_rx) = mpsc::channel(2);
        let state = PrivacyService::start_listening_with_sources(
            State::Init,
            &mut output_tx,
            &pipewire_source,
            &webcam_source,
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
        let pipewire_source = TestPipewireSource::new(pipewire_rx);

        let webcam_source = TestWebcamSource::new(stream::pending::<PrivacyEvent>().boxed());
        let (mut output_tx, output_rx) = mpsc::channel::<ServiceEvent<PrivacyService>>(1);
        drop(output_rx);

        let result = PrivacyService::start_listening_with_sources(
            State::Init,
            &mut output_tx,
            &pipewire_source,
            &webcam_source,
        )
        .await;
        assert!(matches!(result, Err(PrivacyError::Channel { .. })));
    }

    #[tokio::test]
    async fn pipewire_updates_are_forwarded() {
        let (pipewire_tx, pipewire_rx) = unbounded_channel();
        let pipewire_source = TestPipewireSource::new(pipewire_rx);
        let webcam_source = TestWebcamSource::new(stream::pending::<PrivacyEvent>().boxed());
        let (mut output_tx, mut output_rx) = mpsc::channel(4);

        let state = PrivacyService::start_listening_with_sources(
            State::Init,
            &mut output_tx,
            &pipewire_source,
            &webcam_source,
        )
        .await
        .expect("initialisation should succeed");

        let mut state = match state {
            State::Active { pipewire, webcam } => State::Active { pipewire, webcam },
            State::Init => panic!("expected active state"),
        };

        pipewire_tx
            .send(PrivacyEvent::AddNode(ApplicationNode {
                id: 1,
                media: Media::Audio,
            }))
            .expect("send to pipewire receiver");

        state = PrivacyService::start_listening_with_sources(
            state,
            &mut output_tx,
            &pipewire_source,
            &webcam_source,
        )
        .await
        .expect("processing should succeed");

        // Skip the initial init event.
        let _ = output_rx.next().await;
        let update = output_rx.next().await;
        assert!(matches!(
            update,
            Some(ServiceEvent::Update(PrivacyEvent::AddNode(_)))
        ));
    }

    #[tokio::test]
    async fn webcam_updates_are_forwarded() {
        let (pipewire_tx, pipewire_rx) = unbounded_channel();
        drop(pipewire_tx);
        let pipewire_source = TestPipewireSource::new(pipewire_rx);

        let webcam_stream = stream::once(future::ready(PrivacyEvent::WebcamOpen))
            .chain(stream::pending())
            .boxed();
        let webcam_source = TestWebcamSource::new(webcam_stream);
        let (mut output_tx, mut output_rx) = mpsc::channel(4);

        let state = PrivacyService::start_listening_with_sources(
            State::Init,
            &mut output_tx,
            &pipewire_source,
            &webcam_source,
        )
        .await
        .expect("initialisation should succeed");

        let mut state = match state {
            State::Active { pipewire, webcam } => State::Active { pipewire, webcam },
            State::Init => panic!("expected active state"),
        };

        state = PrivacyService::start_listening_with_sources(
            state,
            &mut output_tx,
            &pipewire_source,
            &webcam_source,
        )
        .await
        .expect("processing should succeed");

        // Skip the initial init event.
        let _ = output_rx.next().await;
        let update = output_rx.next().await;
        assert!(matches!(
            update,
            Some(ServiceEvent::Update(PrivacyEvent::WebcamOpen))
        ));
    }
}
