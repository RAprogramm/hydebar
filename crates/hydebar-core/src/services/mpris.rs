use super::{ReadOnlyService, Service, ServiceEvent};
use crate::modules::ModuleError;
use dbus::MprisPlayerProxy;
use futures::{Future, Stream, StreamExt, future::join_all, stream::SelectAll};
use iced::{Subscription, Task};
use log::{debug, error, info};
use std::{collections::HashMap, fmt::Display, ops::Deref, pin::Pin, sync::Arc};
use tokio::task::yield_now;
use zbus::{fdo::DBusProxy, zvariant::OwnedValue};

mod dbus;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    #[default]
    Playing,
    Paused,
    Stopped,
}
impl From<String> for PlaybackStatus {
    fn from(playback_status: String) -> PlaybackStatus {
        match playback_status.as_str() {
            "Playing" => PlaybackStatus::Playing,
            "Paused" => PlaybackStatus::Paused,
            "Stopped" => PlaybackStatus::Stopped,
            _ => PlaybackStatus::Playing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MprisPlayerData {
    pub service: String,
    pub metadata: Option<MprisPlayerMetadata>,
    pub volume: Option<f64>,
    pub state: PlaybackStatus,
    proxy: MprisPlayerProxy<'static>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MprisPlayerMetadata {
    pub artists: Option<Vec<String>>,
    pub title: Option<String>,
}

impl Display for MprisPlayerMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match (self.artists.clone(), self.title.clone()) {
            (None, None) => String::new(),
            (None, Some(t)) => t,
            (Some(a), None) => a.join(", "),
            (Some(a), Some(t)) => format!("{} - {}", a.join(", "), t),
        };
        write!(f, "{t}")
    }
}

impl From<HashMap<String, OwnedValue>> for MprisPlayerMetadata {
    fn from(value: HashMap<String, OwnedValue>) -> Self {
        let artists = match value.get("xesam:artist") {
            Some(v) => v.clone().try_into().ok(),
            None => None,
        };
        let title = match value.get("xesam:title") {
            Some(v) => v.clone().try_into().ok(),
            None => None,
        };

        Self { artists, title }
    }
}

#[derive(Debug, Clone)]
pub struct MprisPlayerService {
    data: Vec<MprisPlayerData>,
    conn: zbus::Connection,
}

impl Deref for MprisPlayerService {
    type Target = Vec<MprisPlayerData>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub enum MprisPlayerEvent {
    Refresh(Vec<MprisPlayerData>),
    Metadata(String, Option<MprisPlayerMetadata>),
    Volume(String, Option<f64>),
    State(String, PlaybackStatus),
}

pub(crate) trait MprisEventPublisher {
    fn send(
        &mut self,
        event: ServiceEvent<MprisPlayerService>,
    ) -> Pin<Box<dyn Future<Output = Result<(), ModuleError>> + Send + '_>>;
}

#[derive(Debug, Clone)]
pub(crate) enum ListenerState {
    Init,
    Active(zbus::Connection),
}

impl ReadOnlyService for MprisPlayerService {
    type UpdateEvent = MprisPlayerEvent;
    type Error = ModuleError;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            MprisPlayerEvent::Refresh(data) => self.data = data,
            MprisPlayerEvent::Metadata(service, metadata) => {
                let s = self.data.iter_mut().find(|d| d.service == service);
                if let Some(s) = s {
                    s.metadata = metadata;
                }
            }
            MprisPlayerEvent::Volume(service, volume) => {
                let s = self.data.iter_mut().find(|d| d.service == service);
                if let Some(s) = s {
                    s.volume = volume;
                }
            }
            MprisPlayerEvent::State(service, state) => {
                let s = self.data.iter_mut().find(|d| d.service == service);
                if let Some(s) = s {
                    s.state = state;
                }
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::none()
    }
}

const MPRIS_PLAYER_SERVICE_PREFIX: &str = "org.mpris.MediaPlayer2.";

#[derive(Debug)]
enum Event {
    NameOwner,
    Metadata(String, Option<MprisPlayerMetadata>),
    Volume(String, Option<f64>),
    State(String, PlaybackStatus),
}

fn module_error(context: &str, err: impl Display) -> ModuleError {
    ModuleError::registration(format!("{context}: {err}"))
}

impl MprisPlayerService {
    async fn initialize_data(conn: &zbus::Connection) -> anyhow::Result<Vec<MprisPlayerData>> {
        let dbus = DBusProxy::new(conn).await?;
        let names: Vec<String> = dbus
            .list_names()
            .await?
            .iter()
            .filter_map(|a| {
                if a.starts_with(MPRIS_PLAYER_SERVICE_PREFIX) {
                    Some(a.to_string())
                } else {
                    None
                }
            })
            .collect();

        debug!("Found MPRIS player services: {names:?}");

        Ok(Self::get_mpris_player_data(conn, &names).await)
    }

    async fn get_mpris_player_data(
        conn: &zbus::Connection,
        names: &[String],
    ) -> Vec<MprisPlayerData> {
        join_all(names.iter().map(|s| async {
            match MprisPlayerProxy::new(conn, s.to_string()).await {
                Ok(proxy) => {
                    let metadata = proxy
                        .metadata()
                        .await
                        .map_or(None, |m| Some(MprisPlayerMetadata::from(m)));

                    let volume = proxy.volume().await.map(|v| v * 100.0).ok();
                    let state = proxy
                        .playback_status()
                        .await
                        .map(PlaybackStatus::from)
                        .unwrap_or_default();

                    Some(MprisPlayerData {
                        service: s.to_string(),
                        metadata,
                        volume,
                        state,
                        proxy,
                    })
                }
                Err(_) => None,
            }
        }))
        .await
        .into_iter()
        .flatten()
        .collect()
    }

    async fn events(conn: &zbus::Connection) -> anyhow::Result<impl Stream<Item = Event> + Send> {
        let dbus = DBusProxy::new(conn).await?;
        let data = Self::initialize_data(conn).await?;

        let mut combined = SelectAll::new();

        combined.push(
            dbus.receive_name_owner_changed()
                .await?
                .filter_map(|s| async move {
                    match s.args() {
                        Ok(a) => a
                            .name
                            .starts_with(MPRIS_PLAYER_SERVICE_PREFIX)
                            .then_some(Event::NameOwner),
                        Err(_) => None,
                    }
                })
                .boxed(),
        );

        for s in data.iter() {
            let cache = Arc::new(s.metadata.clone());

            combined.push(
                s.proxy
                    .receive_metadata_changed()
                    .await
                    .filter_map({
                        let cache = cache.clone();
                        let service = s.service.clone();

                        move |m| {
                            let cache = cache.clone();
                            let service = service.clone();

                            async move {
                                let new_metadata =
                                    m.get().await.map(MprisPlayerMetadata::from).ok();
                                if &new_metadata == cache.as_ref() {
                                    None
                                } else {
                                    debug!("Metadata changed: {new_metadata:?}");

                                    Some(Event::Metadata(service, new_metadata))
                                }
                            }
                        }
                    })
                    .boxed(),
            );
        }

        for s in data.iter() {
            let volume = s.volume;

            combined.push(
                s.proxy
                    .receive_volume_changed()
                    .await
                    .filter_map({
                        let service = s.service.clone();
                        move |v| {
                            let service = service.clone();
                            async move {
                                let new_volume = v.get().await.ok();
                                if volume == new_volume {
                                    None
                                } else {
                                    debug!("Volume changed: {new_volume:?}");

                                    Some(Event::Volume(service, new_volume))
                                }
                            }
                        }
                    })
                    .boxed(),
            );
        }

        for s in data.iter() {
            let state = s.state;

            combined.push(
                s.proxy
                    .receive_playback_status_changed()
                    .await
                    .filter_map({
                        let service = s.service.clone();
                        move |v| {
                            let service = service.clone();
                            async move {
                                let new_state =
                                    v.get().await.map(PlaybackStatus::from).unwrap_or_default();
                                if state == new_state {
                                    None
                                } else {
                                    debug!("PlaybackStatus changed: {new_state:?}");

                                    Some(Event::State(service, new_state))
                                }
                            }
                        }
                    })
                    .boxed(),
            );
        }

        Ok(combined)
    }

    pub(crate) async fn start_listening<P>(
        state: ListenerState,
        publisher: &mut P,
    ) -> Result<ListenerState, ModuleError>
    where
        P: MprisEventPublisher,
    {
        #[cfg(test)]
        if let Some(callback) = test_support::current_start_listening_override() {
            let mut publisher = publisher as &mut dyn MprisEventPublisher;
            return (callback)(state, &mut publisher).await;
        }

        Self::start_listening_internal(state, publisher).await
    }

    async fn start_listening_internal<P>(
        state: ListenerState,
        publisher: &mut P,
    ) -> Result<ListenerState, ModuleError>
    where
        P: MprisEventPublisher,
    {
        match state {
            ListenerState::Init => {
                let conn = zbus::Connection::session()
                    .await
                    .map_err(|err| module_error("failed to connect to session bus", err))?;

                match Self::initialize_data(&conn).await {
                    Ok(data) => {
                        info!("MPRIS player service initialized");

                        publisher
                            .send(ServiceEvent::Init(MprisPlayerService {
                                data,
                                conn: conn.clone(),
                            }))
                            .await?;

                        Ok(ListenerState::Active(conn))
                    }
                    Err(err) => {
                        error!("Failed to initialize MPRIS player service: {err}");
                        Err(module_error(
                            "failed to initialize MPRIS player service",
                            err,
                        ))
                    }
                }
            }
            ListenerState::Active(conn) => match Self::events(&conn).await {
                Ok(events) => {
                    let mut chunks = events.ready_chunks(10);

                    while let Some(chunk) = chunks.next().await {
                        debug!("MPRIS player service receive events: {chunk:?}");

                        let mut need_refresh = false;

                        for event in chunk {
                            match event {
                                Event::NameOwner => {
                                    debug!("MPRIS player service name owner changed");
                                    need_refresh = true;
                                }
                                Event::Metadata(service, metadata) => {
                                    debug!(
                                        "MPRIS player service {service} metadata changed: {metadata:?}"
                                    );
                                    publisher
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Metadata(
                                            service, metadata,
                                        )))
                                        .await?;
                                }
                                Event::Volume(service, volume) => {
                                    debug!(
                                        "MPRIS player service {service} volume changed: {volume:?}"
                                    );
                                    publisher
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Volume(
                                            service, volume,
                                        )))
                                        .await?;
                                }
                                Event::State(service, state) => {
                                    debug!(
                                        "MPRIS player service {service} playback status changed: {state:?}"
                                    );
                                    publisher
                                        .send(ServiceEvent::Update(MprisPlayerEvent::State(
                                            service, state,
                                        )))
                                        .await?;
                                }
                            }
                        }

                        if need_refresh {
                            match Self::initialize_data(&conn).await {
                                Ok(data) => {
                                    debug!("Refreshing MPRIS player data");
                                    publisher
                                        .send(ServiceEvent::Update(MprisPlayerEvent::Refresh(data)))
                                        .await?;
                                }
                                Err(err) => {
                                    error!("Failed to fetch MPRIS player data: {err}");
                                    return Err(module_error(
                                        "failed to refresh MPRIS player data",
                                        err,
                                    ));
                                }
                            }

                            break;
                        }
                    }

                    Ok(ListenerState::Active(conn))
                }
                Err(err) => {
                    error!("Failed to listen for MPRIS player events: {err}");
                    Err(module_error(
                        "failed to listen for MPRIS player events",
                        err,
                    ))
                }
            },
        }
    }

    pub(crate) async fn execute_command(
        service: Option<MprisPlayerService>,
        command: MprisPlayerCommand,
    ) -> Result<Vec<MprisPlayerData>, ModuleError> {
        #[cfg(test)]
        if let Some(callback) = test_support::current_execute_command_override() {
            return (callback)(service, command).await;
        }

        let service = service
            .ok_or_else(|| ModuleError::registration("MPRIS player service is not initialised"))?;

        let names: Vec<String> = service.data.iter().map(|d| d.service.clone()).collect();
        let player = service
            .data
            .iter()
            .find(|d| d.service == command.service_name)
            .ok_or_else(|| {
                ModuleError::registration(format!(
                    "unknown MPRIS service '{}'",
                    command.service_name
                ))
            })?;

        let proxy = player.proxy.clone();
        match command.command {
            PlayerCommand::Prev => {
                proxy
                    .previous()
                    .await
                    .map_err(|err| module_error("failed to execute previous command", err))?;
            }
            PlayerCommand::PlayPause => {
                proxy
                    .play_pause()
                    .await
                    .map_err(|err| module_error("failed to execute play/pause command", err))?;
            }
            PlayerCommand::Next => {
                proxy
                    .next()
                    .await
                    .map_err(|err| module_error("failed to execute next command", err))?;
            }
            PlayerCommand::Volume(v) => {
                proxy
                    .set_volume(v / 100.0)
                    .await
                    .map_err(|err| module_error("failed to execute volume command", err))?;
            }
        }

        Ok(Self::get_mpris_player_data(&service.conn, &names).await)
    }
}

#[derive(Debug)]
pub struct MprisPlayerCommand {
    pub service_name: String,
    pub command: PlayerCommand,
}

#[derive(Debug)]
pub enum PlayerCommand {
    Prev,
    PlayPause,
    Next,
    Volume(f64),
}

impl Service for MprisPlayerService {
    type Command = MprisPlayerCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let service = Some(self.clone());

        Task::perform(
            async move {
                match MprisPlayerService::execute_command(service, command).await {
                    Ok(data) => ServiceEvent::Update(MprisPlayerEvent::Refresh(data)),
                    Err(error) => ServiceEvent::Error(error),
                }
            },
            |event| event,
        )
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::{
        sync::{Arc, Mutex, OnceLock},
        time::Duration,
    };

    pub type StartListeningFuture =
        Pin<Box<dyn Future<Output = Result<ListenerState, ModuleError>> + Send>>;
    pub type StartListeningCallback = Arc<
        dyn Fn(ListenerState, &mut dyn MprisEventPublisher) -> StartListeningFuture + Send + Sync,
    >;

    pub type ExecuteCommandFuture =
        Pin<Box<dyn Future<Output = Result<Vec<MprisPlayerData>, ModuleError>> + Send>>;
    pub type ExecuteCommandCallback = Arc<
        dyn Fn(Option<MprisPlayerService>, MprisPlayerCommand) -> ExecuteCommandFuture
            + Send
            + Sync,
    >;

    static START_LISTENING_OVERRIDE: OnceLock<Mutex<Option<StartListeningCallback>>> =
        OnceLock::new();
    static EXECUTE_COMMAND_OVERRIDE: OnceLock<Mutex<Option<ExecuteCommandCallback>>> =
        OnceLock::new();

    fn start_listening_override() -> &'static Mutex<Option<StartListeningCallback>> {
        START_LISTENING_OVERRIDE.get_or_init(|| Mutex::new(None))
    }

    fn execute_command_override() -> &'static Mutex<Option<ExecuteCommandCallback>> {
        EXECUTE_COMMAND_OVERRIDE.get_or_init(|| Mutex::new(None))
    }

    pub fn install_start_listening_override(callback: StartListeningCallback) -> OverrideGuard {
        *start_listening_override()
            .lock()
            .expect("start listening override mutex poisoned") = Some(callback);
        OverrideGuard {
            target: OverrideTarget::StartListening,
        }
    }

    pub fn install_execute_command_override(callback: ExecuteCommandCallback) -> OverrideGuard {
        *execute_command_override()
            .lock()
            .expect("execute command override mutex poisoned") = Some(callback);
        OverrideGuard {
            target: OverrideTarget::ExecuteCommand,
        }
    }

    pub(crate) fn current_start_listening_override() -> Option<StartListeningCallback> {
        start_listening_override()
            .lock()
            .expect("start listening override mutex poisoned")
            .clone()
    }

    pub(crate) fn current_execute_command_override() -> Option<ExecuteCommandCallback> {
        execute_command_override()
            .lock()
            .expect("execute command override mutex poisoned")
            .clone()
    }

    pub struct OverrideGuard {
        target: OverrideTarget,
    }

    enum OverrideTarget {
        StartListening,
        ExecuteCommand,
    }

    impl Drop for OverrideGuard {
        fn drop(&mut self) {
            match self.target {
                OverrideTarget::StartListening => {
                    *start_listening_override()
                        .lock()
                        .expect("start listening override mutex poisoned") = None;
                }
                OverrideTarget::ExecuteCommand => {
                    *execute_command_override()
                        .lock()
                        .expect("execute command override mutex poisoned") = None;
                }
            }
        }
    }

    pub async fn yield_once() {
        yield_now().await;
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
