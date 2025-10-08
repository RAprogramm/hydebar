use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

use iced::futures::StreamExt;
use inotify::{EventMask, Inotify, WatchMask};

use crate::services::privacy::{PrivacyError, PrivacyEvent, PrivacyStream};

/// Provides webcam state updates sourced from inotify events.
pub(crate) trait WebcamEventSource
{
    /// Future returned when subscribing to webcam state notifications.
    type Future<'a,>: Future<Output = Result<PrivacyStream, PrivacyError,>,> + Send + 'a
    where
        Self: 'a;

    /// Subscribe to webcam state notifications.
    fn subscribe(&self,) -> Self::Future<'_,>;
}

/// Watches a webcam device path using the inotify subsystem.
#[derive(Debug, Clone,)]
pub(crate) struct WebcamWatcher
{
    device_path: PathBuf,
}

impl WebcamWatcher
{
    /// Create a new watcher for the provided webcam device path.
    pub(crate) fn new(path: &Path,) -> Self
    {
        Self {
            device_path: path.into(),
        }
    }

    async fn create_stream(&self,) -> Result<PrivacyStream, PrivacyError,>
    {
        let inotify =
            Inotify::init().map_err(|err| PrivacyError::inotify_init(err.to_string(),),)?;
        match inotify.watches().add(
            &self.device_path,
            WatchMask::CLOSE_WRITE
                | WatchMask::CLOSE_NOWRITE
                | WatchMask::DELETE_SELF
                | WatchMask::OPEN
                | WatchMask::ATTRIB,
        ) {
            Ok(_,) => {}
            Err(err,) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(PrivacyError::WebcamUnavailable,);
            }
            Err(err,) => {
                return Err(PrivacyError::inotify_watch(err.to_string(),),);
            }
        }

        let buffer = [0; 512];
        let stream = inotify
            .into_event_stream(buffer,)
            .map_err(|err| PrivacyError::inotify_init(err.to_string(),),)?
            .filter_map(|event| async move {
                match event {
                    Ok(event,) => match event.mask {
                        EventMask::OPEN => Some(PrivacyEvent::WebcamOpen,),
                        EventMask::CLOSE_WRITE | EventMask::CLOSE_NOWRITE => {
                            Some(PrivacyEvent::WebcamClose,)
                        }
                        _ => None,
                    },
                    Err(error,) => {
                        log::warn!("Failed to read webcam event: {error}");
                        None
                    }
                }
            },)
            .boxed();

        Ok(stream,)
    }
}

impl WebcamEventSource for WebcamWatcher
{
    type Future<'a,>
        = Pin<Box<dyn Future<Output = Result<PrivacyStream, PrivacyError,>,> + Send + 'a,>,>
    where
        Self: 'a;

    fn subscribe(&self,) -> Self::Future<'_,>
    {
        Box::pin(self.create_stream(),)
    }
}
