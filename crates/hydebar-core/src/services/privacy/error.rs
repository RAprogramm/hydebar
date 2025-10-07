use std::sync::Arc;

use masterror::Error;

/// Error type emitted by the privacy service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyError {
    /// Failed to initialise the PipeWire main loop.
    PipewireMainloop { context: Arc<str> },

    /// Failed to create the PipeWire context that owns the registry connection.
    PipewireContext { context: Arc<str> },

    /// Failed to connect to the PipeWire core service.
    PipewireCore { context: Arc<str> },

    /// Failed to access the PipeWire registry.
    PipewireRegistry { context: Arc<str> },

    /// Failed to initialise the inotify subsystem for webcam monitoring.
    InotifyInit { context: Arc<str> },

    /// Failed to register the webcam device with inotify.
    InotifyWatch { context: Arc<str> },

    /// Failed to communicate with the internal service channels.
    Channel { context: Arc<str> },

    /// The webcam device is not present on the system.
    WebcamUnavailable,
}

impl std::fmt::Display for PrivacyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PipewireMainloop { context } => {
                write!(f, "failed to initialise PipeWire main loop: {}", context)
            }
            Self::PipewireContext { context } => {
                write!(f, "failed to create PipeWire context: {}", context)
            }
            Self::PipewireCore { context } => {
                write!(f, "failed to connect to PipeWire core: {}", context)
            }
            Self::PipewireRegistry { context } => {
                write!(f, "failed to access PipeWire registry: {}", context)
            }
            Self::InotifyInit { context } => {
                write!(f, "failed to initialise inotify: {}", context)
            }
            Self::InotifyWatch { context } => {
                write!(f, "failed to watch webcam device: {}", context)
            }
            Self::Channel { context } => {
                write!(f, "privacy service channel error: {}", context)
            }
            Self::WebcamUnavailable => {
                write!(f, "webcam device is unavailable")
            }
        }
    }
}

impl std::error::Error for PrivacyError {}

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
