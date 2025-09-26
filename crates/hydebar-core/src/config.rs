use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::{
    any::TypeId,
    ffi::{OsStr, OsString},
    fmt::Display,
    future::Future,
    io::Read,
    pin::Pin,
};

pub use hydebar_proto::config::*;

use hydebar_proto::config::{Config, DEFAULT_CONFIG_FILE_PATH};
use iced::futures::channel::mpsc::{SendError, Sender};
use iced::futures::{SinkExt, Stream, StreamExt, pin_mut};
use iced::{Subscription, stream::channel};
use inotify::{EventMask, Inotify, WatchMask};
use log::{debug, error, info, warn};
use shellexpand::full;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum ConfigEvent {
    Updated(Box<Config>),
}

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("failed to expand config path '{input}': {source}")]
    Expand {
        input: String,
        #[source]
        source: shellexpand::LookupError<std::env::VarError>,
    },
    #[error("config file does not exist: {path}")]
    Missing { path: PathBuf },
    #[error("config path '{path}' has no parent directory")]
    MissingParent { path: PathBuf },
    #[error("failed to create config directory '{path}': {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
enum ConfigReadError {
    #[error("failed to read config file '{path}': {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file '{path}': {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

pub fn get_config(path: Option<PathBuf>) -> Result<(Config, PathBuf), ConfigLoadError> {
    match path {
        Some(path) => {
            info!("Config path provided {path:?}");
            let expanded = expand_path(path)?;

            if !expanded.exists() {
                return Err(ConfigLoadError::Missing { path: expanded });
            }

            let config = load_config_or_default(&expanded);

            Ok((config, expanded))
        }
        None => {
            let expanded = expand_path(PathBuf::from(DEFAULT_CONFIG_FILE_PATH))?;
            ensure_parent_exists(&expanded)?;

            let config = load_config_or_default(&expanded);

            Ok((config, expanded))
        }
    }
}

fn expand_path(path: PathBuf) -> Result<PathBuf, ConfigLoadError> {
    let input = path.to_string_lossy().into_owned();
    match full(&input) {
        Ok(expanded) => Ok(PathBuf::from(expanded.to_string())),
        Err(source) => Err(ConfigLoadError::Expand { input, source }),
    }
}

fn ensure_parent_exists(path: &Path) -> Result<(), ConfigLoadError> {
    let parent = path
        .parent()
        .ok_or_else(|| ConfigLoadError::MissingParent {
            path: path.to_path_buf(),
        })?;

    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|source| ConfigLoadError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    Ok(())
}

fn read_config(path: &Path) -> Result<Config, ConfigReadError> {
    let mut content = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut content))
        .map_err(|source| ConfigReadError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    toml::from_str(&content).map_err(|source| ConfigReadError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

fn load_config_or_default(path: &Path) -> Config {
    info!("Decoding config file {path:?}");

    match read_config(path) {
        Ok(config) => {
            info!("Config file loaded successfully");
            config
        }
        Err(err) => {
            warn!("{err}");
            warn!("Falling back to default configuration");
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn get_config_returns_default_on_parse_error() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "invalid = [").expect("failed to write invalid config");

        let (config, returned_path) =
            get_config(Some(config_path.clone())).expect("get_config should succeed");

        assert_eq!(returned_path, config_path);
        let default = Config::default();
        assert_eq!(config.log_level, default.log_level);
        assert_eq!(config.menu_keyboard_focus, default.menu_keyboard_focus);
        assert_eq!(config.position, default.position);
    }

    #[test]
    fn get_config_errors_when_file_missing() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("missing.toml");

        let error = get_config(Some(config_path.clone())).expect_err("expected error");

        match error {
            ConfigLoadError::Missing { path } => assert_eq!(path, config_path),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Changed,
    Removed,
}

trait WatchedEvent {
    fn file_name(&self) -> Option<&OsStr>;

    fn mask(&self) -> EventMask;
}

impl WatchedEvent for inotify::Event<OsString> {
    fn file_name(&self) -> Option<&OsStr> {
        self.name.as_deref()
    }

    fn mask(&self) -> EventMask {
        self.mask
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchLoopOutcome {
    StreamEnded,
    HandlerClosed,
}

fn interpret_event<E: WatchedEvent>(event: &E, target_name: &OsStr) -> Option<Event> {
    let name = event.file_name()?;

    if name != target_name {
        return None;
    }

    let mask = event.mask();

    if mask == EventMask::DELETE | EventMask::MOVED_FROM {
        debug!("File deleted or moved");
        Some(Event::Removed)
    } else if mask == EventMask::CREATE | EventMask::MODIFY | EventMask::MOVED_TO {
        debug!("File created or moved");
        Some(Event::Changed)
    } else {
        None
    }
}

async fn process_event_batches<S, E, Err, F, Fut, HandlerErr>(
    mut stream: Pin<&mut S>,
    target_name: &OsStr,
    mut handler: F,
) -> WatchLoopOutcome
where
    S: Stream<Item = Vec<Result<E, Err>>>,
    E: WatchedEvent + std::fmt::Debug,
    Err: Display,
    F: FnMut(Event) -> Fut,
    Fut: Future<Output = Result<(), HandlerErr>>,
    HandlerErr: Display,
{
    while let Some(batch) = stream.as_mut().next().await {
        let mut file_event = None;

        for event in batch {
            match event {
                Ok(event) => {
                    debug!("Event: {event:?}");

                    match interpret_event(&event, target_name) {
                        Some(kind) => {
                            file_event = Some(kind);
                        }
                        None => {
                            debug!("Ignoring event");
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to read watch event: {err}");
                }
            }
        }

        if let Some(kind) = file_event {
            if let Err(err) = handler(kind).await {
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
    output: &mut Sender<ConfigEvent>,
    path: &Path,
    event: Event,
) -> Result<(), SendError> {
    match event {
        Event::Changed => {
            info!("Reload config file");

            let new_config = load_config_or_default(path);

            output
                .send(ConfigEvent::Updated(Box::new(new_config)))
                .await
        }
        Event::Removed => {
            info!("Config file removed");

            output.send(ConfigEvent::Updated(Box::default())).await
        }
    }
}

pub fn subscription(path: &Path) -> Subscription<ConfigEvent> {
    let id = TypeId::of::<Config>();
    let path = path.to_path_buf();

    Subscription::run_with_id(
        id,
        channel(100, move |mut output| async move {
            let Some(folder) = path.parent().map(Path::to_path_buf) else {
                error!(
                    "Config file path does not have a parent directory, cannot watch for changes"
                );
                return;
            };

            let Some(file_name) = path.file_name().map(OsStr::to_os_string) else {
                error!("Config file path does not have a file name, cannot watch for changes");
                return;
            };

            loop {
                let inotify = match Inotify::init() {
                    Ok(inotify) => inotify,
                    Err(e) => {
                        error!("Failed to initialize inotify: {e}");
                        break;
                    }
                };

                debug!("Watching config file at {path:?}");

                let watch_result = inotify.watches().add(
                    &folder,
                    WatchMask::CREATE | WatchMask::DELETE | WatchMask::MOVE | WatchMask::MODIFY,
                );

                if let Err(e) = watch_result {
                    error!("Failed to add watch for {folder:?}: {e}");
                    break;
                }

                let buffer = [0; 1024];
                let stream = match inotify.into_event_stream(buffer) {
                    Ok(stream) => stream,
                    Err(e) => {
                        error!("Failed to create inotify event stream: {e}");
                        break;
                    }
                };

                let event_stream = stream.ready_chunks(10);
                pin_mut!(event_stream);

                let sender_template = output.clone();
                let path_clone = path.clone();

                match process_event_batches(
                    event_stream.as_mut(),
                    file_name.as_os_str(),
                    move |event| {
                        let mut sender = sender_template.clone();
                        let path = path_clone.clone();

                        async move { handle_watch_event(&mut sender, &path, event).await }
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
        }),
    )
}
