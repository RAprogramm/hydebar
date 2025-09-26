use std::sync::Arc;

use masterror::Error;

/// Error type emitted by the privacy service.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PrivacyError {
    /// Failed to initialise the PipeWire main loop.
    #[error("failed to initialise PipeWire main loop: {context}")]
    PipewireMainloop { context: Arc<str> },

    /// Failed to create the PipeWire context that owns the registry connection.
    #[error("failed to create PipeWire context: {context}")]
    PipewireContext { context: Arc<str> },

    /// Failed to connect to the PipeWire core service.
    #[error("failed to connect to PipeWire core: {context}")]
    PipewireCore { context: Arc<str> },

    /// Failed to access the PipeWire registry.
    #[error("failed to access PipeWire registry: {context}")]
    PipewireRegistry { context: Arc<str> },

    /// Failed to initialise the inotify subsystem for webcam monitoring.
    #[error("failed to initialise inotify: {context}")]
    InotifyInit { context: Arc<str> },

    /// Failed to register the webcam device with inotify.
    #[error("failed to watch webcam device: {context}")]
    InotifyWatch { context: Arc<str> },

    /// Failed to communicate with the internal service channels.
    #[error("privacy service channel error: {context}")]
    Channel { context: Arc<str> },

    /// The webcam device is not present on the system.
    #[error("webcam device is unavailable")]
    WebcamUnavailable,
}

impl PrivacyError {
    fn arc_from(value: impl Into<String>) -> Arc<str> {
        Arc::<str>::from(value.into())
    }

    /// Create a new PipeWire main loop error with additional context.
    pub fn pipewire_mainloop(context: impl Into<String>) -> Self {
        Self::PipewireMainloop {
            context: Self::arc_from(context),
        }
    }

    /// Create a new PipeWire context error with additional context.
    pub fn pipewire_context(context: impl Into<String>) -> Self {
        Self::PipewireContext {
            context: Self::arc_from(context),
        }
    }

    /// Create a new PipeWire core connection error with additional context.
    pub fn pipewire_core(context: impl Into<String>) -> Self {
        Self::PipewireCore {
            context: Self::arc_from(context),
        }
    }

    /// Create a new PipeWire registry error with additional context.
    pub fn pipewire_registry(context: impl Into<String>) -> Self {
        Self::PipewireRegistry {
            context: Self::arc_from(context),
        }
    }

    /// Create a new inotify initialisation error with additional context.
    pub fn inotify_init(context: impl Into<String>) -> Self {
        Self::InotifyInit {
            context: Self::arc_from(context),
        }
    }

    /// Create a new inotify watch registration error with additional context.
    pub fn inotify_watch(context: impl Into<String>) -> Self {
        Self::InotifyWatch {
            context: Self::arc_from(context),
        }
    }

    /// Create a new channel error with contextual information.
    pub fn channel(context: impl Into<String>) -> Self {
        Self::Channel {
            context: Self::arc_from(context),
        }
    }
}

impl From<std::io::Error> for PrivacyError {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::NotFound => PrivacyError::WebcamUnavailable,
            _ => PrivacyError::inotify_init(value.to_string()),
        }
    }
}

impl From<pipewire::Error> for PrivacyError {
    fn from(value: pipewire::Error) -> Self {
        PrivacyError::pipewire_mainloop(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::PrivacyError;

    #[test]
    fn converts_not_found_to_webcam_unavailable() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        assert_eq!(PrivacyError::from(err), PrivacyError::WebcamUnavailable);
    }

    #[test]
    fn converts_pipewire_error() {
        let err = pipewire::Error::NoMemory;
        assert!(matches!(
            PrivacyError::from(err),
            PrivacyError::PipewireMainloop { .. }
        ));
    }
}
