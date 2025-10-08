use std::{
    any::TypeId,
    ffi::{OsStr, OsString},
    fmt::Display,
    future::Future,
    path::Path,
    pin::Pin,
    sync::Arc,
};

use iced::{
    Subscription,
    futures::{
        SinkExt, Stream, StreamExt,
        channel::mpsc::{SendError, Sender},
        pin_mut,
    },
    stream::channel,
};
use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, info, warn};

use super::{ConfigReadError, read_config};
use crate::config::manager::{ConfigApplied, ConfigDegradation, ConfigManager, ConfigUpdateError};

/// Events produced by the configuration watcher subscription.
#[derive(Debug, Clone,)]
pub enum ConfigEvent
{
    /// A new, validated configuration was applied.
    Applied(ConfigApplied,),
    /// The configuration could not be refreshed and the previous state is
    /// retained.
    Degraded(ConfigDegradation,),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
enum Event
{
    Changed,
    Removed,
}

trait WatchedEvent
{
    fn file_name(&self,) -> Option<&OsStr,>;

    fn mask(&self,) -> EventMask;
}

impl WatchedEvent for inotify::Event<OsString,>
{
    fn file_name(&self,) -> Option<&OsStr,>
    {
        self.name.as_deref()
    }

    fn mask(&self,) -> EventMask
    {
        self.mask
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
enum WatchLoopOutcome
{
    StreamEnded,
    HandlerClosed,
}

fn interpret_event<E: WatchedEvent,>(event: &E, target_name: &OsStr,) -> Option<Event,>
{
    let name = event.file_name()?;

    if name != target_name {
        return None;
    }

    let mask = event.mask();

    let is_removed = mask.contains(EventMask::DELETE,) || mask.contains(EventMask::MOVED_FROM,);

    if is_removed && !mask.intersects(EventMask::CREATE | EventMask::MODIFY | EventMask::MOVED_TO,)
    {
        debug!("File deleted or moved");
        return Some(Event::Removed,);
    }

    let is_changed = mask.intersects(
        EventMask::CREATE | EventMask::MODIFY | EventMask::MOVED_TO | EventMask::CLOSE_WRITE,
    );

    if is_changed {
        debug!("File changed");
        Some(Event::Changed,)
    } else {
        None
    }
}

async fn process_event_batches<S, E, Err, F, Fut,>(
    mut stream: Pin<&mut S,>,
    target_name: &OsStr,
    mut handler: F,
) -> WatchLoopOutcome
where
    S: Stream<Item = Vec<Result<E, Err,>,>,>,
    E: WatchedEvent + std::fmt::Debug,
    Err: Display,
    F: FnMut(Event,) -> Fut,
    Fut: Future<Output = Result<(), SendError,>,>,
{
    while let Some(batch,) = stream.as_mut().next().await {
        let mut file_event = None;

        for event in batch {
            match event {
                Ok(event,) => {
                    debug!("Event: {event:?}");

                    match interpret_event(&event, target_name,) {
                        Some(kind,) => {
                            file_event = Some(kind,);
                        }
                        None => {
                            debug!("Ignoring event");
                        }
                    }
                }
                Err(err,) => {
                    error!("Failed to read watch event: {err}");
                }
            }
        }

        if let Some(kind,) = file_event {
            if let Err(err,) = handler(kind,).await {
                warn!("Stopping config watch because handler returned an error: {err}");
                return WatchLoopOutcome::HandlerClosed;
            }
        } else {
            debug!("No relevant file event detected.");
        }
    }

    WatchLoopOutcome::StreamEnded
}

async fn handle_watch_event(
    output: &mut Sender<ConfigEvent,>,
    path: &Path,
    event: Event,
    manager: Arc<ConfigManager,>,
) -> Result<(), SendError,>
{
    match event {
        Event::Changed => {
            info!("Reload config file");

            match load_candidate(path, &manager,) {
                Ok(applied,) => output.send(ConfigEvent::Applied(applied,),).await,
                Err(reason,) => {
                    warn!("Configuration update failed: {reason}");
                    send_degradation(output, manager, reason,).await
                }
            }
        }
        Event::Removed => {
            info!("Config file removed");

            send_degradation(output, manager, ConfigUpdateError::Removed,).await
        }
    }
}

fn load_candidate(
    path: &Path,
    manager: &ConfigManager,
) -> Result<ConfigApplied, ConfigUpdateError,>
{
    let config = read_config(path,).map_err(convert_read_error,)?;

    config.validate()?;

    manager.apply(config,).map_err(|err| ConfigUpdateError::state(err.to_string(),),)
}

fn convert_read_error(err: ConfigReadError,) -> ConfigUpdateError
{
    match err {
        ConfigReadError::Read {
            path,
            source,
        } => ConfigUpdateError::read(path, &source,),
        ConfigReadError::Parse {
            path,
            source,
        } => ConfigUpdateError::parse(path, &source,),
    }
}

async fn send_degradation(
    output: &mut Sender<ConfigEvent,>,
    manager: Arc<ConfigManager,>,
    reason: ConfigUpdateError,
) -> Result<(), SendError,>
{
    match manager.degraded(reason,) {
        Ok(degradation,) => output.send(ConfigEvent::Degraded(degradation,),).await,
        Err(err,) => {
            error!("Failed to report configuration degradation: {err}");
            Ok((),)
        }
    }
}

pub fn subscription(path: &Path, manager: Arc<ConfigManager,>,) -> Subscription<ConfigEvent,>
{
    let id = TypeId::of::<ConfigEvent,>();
    let path = path.to_path_buf();

    Subscription::run_with_id(
        id,
        channel(100, move |output| {
            let manager = Arc::clone(&manager,);

            async move {
                let Some(folder,) = path.parent().map(Path::to_path_buf,) else {
                    error!(
                        "Config file path does not have a parent directory, cannot watch for changes"
                    );
                    return;
                };

                let Some(file_name,) = path.file_name().map(OsStr::to_os_string,) else {
                    error!("Config file path does not have a file name, cannot watch for changes");
                    return;
                };

                loop {
                    let inotify = match Inotify::init() {
                        Ok(inotify,) => inotify,
                        Err(e,) => {
                            error!("Failed to initialize inotify: {e}");
                            break;
                        }
                    };

                    debug!("Watching config file at {path:?}");

                    let watch_result = inotify.watches().add(
                        &folder,
                        WatchMask::CREATE
                            | WatchMask::DELETE
                            | WatchMask::MOVE
                            | WatchMask::MODIFY,
                    );

                    if let Err(e,) = watch_result {
                        error!("Failed to add watch for {folder:?}: {e}");
                        break;
                    }

                    let buffer = [0; 1024];
                    let stream = match inotify.into_event_stream(buffer,) {
                        Ok(stream,) => stream,
                        Err(e,) => {
                            error!("Failed to create inotify event stream: {e}");
                            break;
                        }
                    };

                    let event_stream = stream.ready_chunks(10,);
                    pin_mut!(event_stream);

                    let sender_template = output.clone();
                    let path_clone = path.clone();
                    let manager_clone = Arc::clone(&manager,);

                    match process_event_batches(
                        event_stream.as_mut(),
                        file_name.as_os_str(),
                        move |event| {
                            let mut sender = sender_template.clone();
                            let path = path_clone.clone();
                            let manager = Arc::clone(&manager_clone);

                            async move { handle_watch_event(&mut sender, &path, event, manager).await }
                        },
                    )
                    .await
                    {
                        WatchLoopOutcome::StreamEnded => {
                            info!(
                                "Config watch stream closed; attempting to restart the inotify watcher"
                            );
                            continue;
                        }
                        WatchLoopOutcome::HandlerClosed => {
                            info!("Config watch handler closed; stopping watcher loop");
                            break;
                        }
                    }
                }

                info!("Config watcher terminated");
            }
        },),
    )
}

#[cfg(test)]
mod tests
{
    use std::ffi::{OsStr, OsString};

    use hydebar_proto::config::Config;
    use iced::futures::channel::mpsc;
    use tempfile::TempDir;

    use super::*;
    use crate::config::manager::ConfigManager;

    #[derive(Debug,)]
    struct FakeEvent
    {
        name: Option<OsString,>,
        mask: EventMask,
    }

    impl WatchedEvent for FakeEvent
    {
        fn file_name(&self,) -> Option<&OsStr,>
        {
            self.name.as_deref()
        }

        fn mask(&self,) -> EventMask
        {
            self.mask
        }
    }

    #[test]
    fn interpret_event_detects_removed_events()
    {
        let target = OsStr::new("config.toml",);

        let delete_event = FakeEvent {
            name: Some(OsString::from("config.toml",),),
            mask: EventMask::DELETE,
        };
        assert_eq!(interpret_event(&delete_event, target), Some(Event::Removed));

        let moved_from_event = FakeEvent {
            name: Some(OsString::from("config.toml",),),
            mask: EventMask::MOVED_FROM,
        };
        assert_eq!(interpret_event(&moved_from_event, target), Some(Event::Removed));

        let unrelated_name = FakeEvent {
            name: Some(OsString::from("other.toml",),),
            mask: EventMask::DELETE,
        };
        assert_eq!(interpret_event(&unrelated_name, target), None);
    }

    #[test]
    fn interpret_event_detects_changed_events()
    {
        let target = OsStr::new("config.toml",);

        for mask in
            [EventMask::CREATE, EventMask::MODIFY, EventMask::MOVED_TO, EventMask::CLOSE_WRITE,]
        {
            let event = FakeEvent {
                name: Some(OsString::from("config.toml",),),
                mask,
            };
            assert_eq!(interpret_event(&event, target), Some(Event::Changed));
        }

        let ignored_event = FakeEvent {
            name: Some(OsString::from("config.toml",),),
            mask: EventMask::ACCESS,
        };
        assert_eq!(interpret_event(&ignored_event, target), None);
    }

    #[tokio::test]
    async fn emits_applied_event_for_valid_update()
    {
        let temp_dir = TempDir::new().expect("failed to create temp dir",);
        let config_path = temp_dir.path().join("config.toml",);
        std::fs::write(&config_path, "",).expect("failed to write config",);

        let manager = Arc::new(ConfigManager::new(Config::default(),),);
        let (mut sender, mut receiver,) = mpsc::channel(10,);

        handle_watch_event(&mut sender, &config_path, Event::Changed, Arc::clone(&manager,),)
            .await
            .expect("sending event should succeed",);

        match receiver.next().await {
            Some(ConfigEvent::Applied(_,),) => {}
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn emits_degraded_event_for_invalid_toml()
    {
        let temp_dir = TempDir::new().expect("failed to create temp dir",);
        let config_path = temp_dir.path().join("config.toml",);
        std::fs::write(&config_path, "invalid = [",).expect("failed to write invalid config",);

        let manager = Arc::new(ConfigManager::new(Config::default(),),);
        let (mut sender, mut receiver,) = mpsc::channel(10,);

        handle_watch_event(&mut sender, &config_path, Event::Changed, Arc::clone(&manager,),)
            .await
            .expect("sending event should succeed",);

        match receiver.next().await {
            Some(ConfigEvent::Degraded(event,),) => {
                assert!(matches!(event.reason, ConfigUpdateError::Parse { .. }));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn emits_degraded_event_when_file_removed()
    {
        let temp_dir = TempDir::new().expect("failed to create temp dir",);
        let config_path = temp_dir.path().join("config.toml",);
        std::fs::write(&config_path, "",).expect("failed to write config",);

        let manager = Arc::new(ConfigManager::new(Config::default(),),);
        let (mut sender, mut receiver,) = mpsc::channel(10,);

        handle_watch_event(&mut sender, &config_path, Event::Removed, manager,)
            .await
            .expect("sending event should succeed",);

        match receiver.next().await {
            Some(ConfigEvent::Degraded(event,),) => {
                assert!(matches!(event.reason, ConfigUpdateError::Removed));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
