use std::path::{Path, PathBuf};
use std::{
    any::TypeId,
    error::Error,
    ffi::{OsStr, OsString},
    fmt::Display,
    fs::File,
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

#[derive(Debug, Clone)]
pub enum ConfigEvent {
    Updated(Box<Config>),
}

pub fn get_config(path: Option<PathBuf>) -> Result<(Config, PathBuf), Box<dyn Error + Send>> {
    match path {
        Some(path) => {
            info!("Config path provided {path:?}");
            expand_path(path).and_then(|expanded| {
                if !expanded.exists() {
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Config file does not exist: {}", expanded.display()),
                    )))
                } else {
                    Ok((read_config(&expanded).unwrap_or_default(), expanded))
                }
            })
        }
        None => expand_path(PathBuf::from(DEFAULT_CONFIG_FILE_PATH)).map(|expanded| {
            let parent = expanded
                .parent()
                .expect("Failed to get default config parent directory");

            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .expect("Failed to create default config parent directory");
            }

            (read_config(&expanded).unwrap_or_default(), expanded)
        }),
    }
}

fn expand_path(path: PathBuf) -> Result<PathBuf, Box<dyn Error + Send>> {
    let str_path = path.to_string_lossy();
    let expanded = full(&str_path).map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;

    Ok(PathBuf::from(expanded.to_string()))
}

fn read_config(path: &Path) -> Result<Config, Box<dyn Error + Send>> {
    let mut content = String::new();
    let read_result = File::open(path).and_then(|mut file| file.read_to_string(&mut content));

    match read_result {
        Ok(_) => {
            info!("Decoding config file {path:?}");

            let res = toml::from_str(&content);

            match res {
                Ok(config) => {
                    info!("Config file loaded successfully");
                    Ok(config)
                }
                Err(e) => {
                    warn!("Failed to parse config file: {e}");
                    Err(Box::new(e))
                }
            }
        }
        Err(e) => {
            warn!("Failed to read config file: {e}");

            Err(Box::new(e))
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

            let new_config = read_config(path).unwrap_or_default();

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
