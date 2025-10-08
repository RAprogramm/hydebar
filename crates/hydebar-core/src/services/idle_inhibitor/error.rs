use std::sync::Arc;

use wayland_client::{ConnectError, DispatchError};

/// Error type emitted by the idle inhibitor service.
///
/// The error captures failures to connect to the Wayland compositor, missing
/// globals announced by the compositor, and dispatch roundtrip errors.
///
/// # Examples
/// ```ignore
/// use hydebar::services::idle_inhibitor::IdleInhibitorError;
///
/// let err = IdleInhibitorError::missing_idle_inhibit_manager();
/// assert!(matches!(err, IdleInhibitorError::MissingGlobal { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq,)]
pub enum IdleInhibitorError
{
    /// Establishing a Wayland connection failed.
    Connection
    {
        context: Arc<str,>,
    },

    /// A required Wayland global was not advertised by the compositor.
    MissingGlobal
    {
        global: MissingGlobal,
    },

    /// Dispatching Wayland events failed during a roundtrip.
    Dispatch
    {
        context: Arc<str,>,
    },
}

impl std::fmt::Display for IdleInhibitorError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result
    {
        match self {
            Self::Connection {
                context,
            } => {
                write!(f, "failed to connect to wayland compositor: {}", context)
            }
            Self::MissingGlobal {
                global,
            } => {
                write!(f, "missing wayland global: {}", global)
            }
            Self::Dispatch {
                context,
            } => {
                write!(f, "failed to dispatch wayland events: {}", context)
            }
        }
    }
}

impl std::error::Error for IdleInhibitorError {}

impl IdleInhibitorError
{
    fn arc_from(value: impl Into<String,>,) -> Arc<str,>
    {
        Arc::<str,>::from(value.into(),)
    }

    /// Create a connection error with contextual information.
    pub fn connection(context: impl Into<String,>,) -> Self
    {
        Self::Connection {
            context: Self::arc_from(context,),
        }
    }

    /// Create a dispatch error with contextual information.
    pub fn dispatch(context: impl Into<String,>,) -> Self
    {
        Self::Dispatch {
            context: Self::arc_from(context,),
        }
    }

    /// Create an error describing a missing compositor global.
    pub fn missing_compositor() -> Self
    {
        Self::MissingGlobal {
            global: MissingGlobal::Compositor,
        }
    }

    /// Create an error describing a missing idle inhibit manager global.
    pub fn missing_idle_inhibit_manager() -> Self
    {
        Self::MissingGlobal {
            global: MissingGlobal::IdleInhibitManager,
        }
    }

    /// Create an error describing a missing compositor surface global.
    pub fn missing_surface() -> Self
    {
        Self::MissingGlobal {
            global: MissingGlobal::Surface,
        }
    }
}

impl From<ConnectError,> for IdleInhibitorError
{
    fn from(value: ConnectError,) -> Self
    {
        IdleInhibitorError::connection(value.to_string(),)
    }
}

impl From<DispatchError,> for IdleInhibitorError
{
    fn from(value: DispatchError,) -> Self
    {
        IdleInhibitorError::dispatch(value.to_string(),)
    }
}

/// Enumeration of required Wayland globals for idle inhibition.
#[derive(Debug, Clone, PartialEq, Eq,)]
pub enum MissingGlobal
{
    /// The `wl_compositor` interface.
    Compositor,
    /// The surface derived from `wl_compositor`.
    Surface,
    /// The `zwp_idle_inhibit_manager_v1` interface.
    IdleInhibitManager,
}

impl core::fmt::Display for MissingGlobal
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_,>,) -> core::fmt::Result
    {
        match self {
            MissingGlobal::Compositor => f.write_str("wl_compositor",),
            MissingGlobal::Surface => f.write_str("wl_surface",),
            MissingGlobal::IdleInhibitManager => f.write_str("zwp_idle_inhibit_manager_v1",),
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::{IdleInhibitorError, MissingGlobal};

    #[test]
    fn connection_error_converts()
    {
        let err = IdleInhibitorError::from(wayland_client::ConnectError::NoCompositor,);
        assert!(matches!(err, IdleInhibitorError::Connection { .. }));
    }

    #[test]
    fn dispatch_error_converts()
    {
        let err = IdleInhibitorError::from(wayland_client::DispatchError::Backend(
            wayland_client::backend::WaylandError::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "dispatch",
            ),),
        ),);
        assert!(matches!(err, IdleInhibitorError::Dispatch { .. }));
    }

    #[test]
    fn missing_global_display_matches_variant()
    {
        let err = IdleInhibitorError::missing_idle_inhibit_manager();
        assert_eq!(format!("{err}"), "missing wayland global: zwp_idle_inhibit_manager_v1");
    }

    #[test]
    fn missing_global_variants_are_distinct()
    {
        assert_ne!(MissingGlobal::Compositor, MissingGlobal::Surface);
    }
}
