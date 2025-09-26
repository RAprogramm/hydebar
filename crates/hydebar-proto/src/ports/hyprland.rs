use std::{error::Error, fmt, pin::Pin, time::Duration};

use tokio_stream::Stream;

/// Stream type alias used for Hyprland event subscriptions.
pub type HyprlandEventStream<E> =
    Pin<Box<dyn Stream<Item = Result<E, HyprlandError>> + Send + 'static>>;

/// Error type returned by [`HyprlandPort`] operations.
///
/// Each error variant stores the logical operation name to aid diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum HyprlandError {
    /// The requested operation timed out before it could complete.
    #[error("operation `{operation}` timed out after {timeout:?}")]
    Timeout {
        /// Logical operation identifier.
        operation: &'static str,
        /// Maximum allotted time before aborting the operation.
        timeout: Duration,
    },
    /// The backend failed to execute the requested operation.
    #[error("operation `{operation}` failed: {source}")]
    Backend {
        /// Logical operation identifier.
        operation: &'static str,
        /// Source error reported by the backend implementation.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
    /// The async runtime required to perform the operation was unavailable.
    #[error("operation `{operation}` unavailable because no async runtime is active")]
    RuntimeUnavailable {
        /// Logical operation identifier.
        operation: &'static str,
    },
    /// The requested operation is not supported by the underlying backend.
    #[error("operation `{operation}` not supported by this Hyprland backend")]
    Unsupported {
        /// Logical operation identifier.
        operation: &'static str,
    },
    /// The operation failed with an explanatory message.
    #[error("operation `{operation}` failed: {message}")]
    Message {
        /// Logical operation identifier.
        operation: &'static str,
        /// Human readable error description.
        message: String,
    },
}

impl HyprlandError {
    /// Helper for constructing [`HyprlandError::Unsupported`].
    pub const fn unsupported(operation: &'static str) -> Self {
        Self::Unsupported { operation }
    }

    /// Helper for constructing [`HyprlandError::RuntimeUnavailable`].
    pub const fn runtime_unavailable(operation: &'static str) -> Self {
        Self::RuntimeUnavailable { operation }
    }

    /// Helper for constructing [`HyprlandError::Message`].
    pub fn message(operation: &'static str, message: impl Into<String>) -> Self {
        Self::Message {
            operation,
            message: message.into(),
        }
    }
}

/// Immutable snapshot describing monitors and workspaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandWorkspaceSnapshot {
    /// Known monitors reported by Hyprland.
    pub monitors: Vec<HyprlandMonitorInfo>,
    /// Known workspaces reported by Hyprland.
    pub workspaces: Vec<HyprlandWorkspaceInfo>,
    /// Identifier of the currently active workspace, if available.
    pub active_workspace_id: Option<i32>,
}

/// Metadata describing a Hyprland monitor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandMonitorInfo {
    /// Monitor identifier as reported by Hyprland.
    pub id: i32,
    /// Human readable monitor name.
    pub name: String,
    /// ID of the special workspace focused on this monitor, if any.
    pub special_workspace_id: Option<i32>,
}

/// Metadata describing a Hyprland workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandWorkspaceInfo {
    /// Workspace identifier.
    pub id: i32,
    /// Workspace name.
    pub name: String,
    /// Index of the monitor the workspace is assigned to, if any.
    pub monitor_id: Option<usize>,
    /// Name of the monitor the workspace is assigned to.
    pub monitor_name: String,
    /// Number of windows currently present in the workspace.
    pub window_count: u16,
}

/// Metadata describing the focused Hyprland window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandWindowInfo {
    /// Window title provided by the client.
    pub title: String,
    /// Window class name.
    pub class: String,
}

/// Snapshot of the keyboard state known to Hyprland.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandKeyboardState {
    /// Currently active XKB layout.
    pub active_layout: String,
    /// Whether multiple layouts are configured.
    pub has_multiple_layouts: bool,
    /// Name of the currently active submap, if any.
    pub active_submap: Option<String>,
}

/// Identifies a monitor for Hyprland dispatch calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandMonitorSelector {
    /// Select monitor by its numeric identifier.
    Id(usize),
    /// Select monitor by its name.
    Name(String),
}

impl fmt::Display for HyprlandMonitorSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "monitor-id:{id}"),
            Self::Name(name) => write!(f, "monitor-name:{name}"),
        }
    }
}

/// Identifies a workspace for Hyprland dispatch calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandWorkspaceSelector {
    /// Select workspace by numeric identifier.
    Id(i32),
    /// Select workspace by name.
    Name(String),
}

impl fmt::Display for HyprlandWorkspaceSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "workspace-id:{id}"),
            Self::Name(name) => write!(f, "workspace-name:{name}"),
        }
    }
}

/// Events related to Hyprland windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandWindowEvent {
    /// The active window changed.
    ActiveWindowChanged,
    /// A workspace focus change occurred.
    WorkspaceFocusChanged,
    /// A window was closed.
    WindowClosed,
}

/// Events related to Hyprland workspaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandWorkspaceEvent {
    /// A new workspace was added.
    Added,
    /// Workspace metadata changed.
    Changed,
    /// A workspace was removed.
    Removed,
    /// A workspace was moved to another monitor.
    Moved,
    /// The active special workspace changed.
    SpecialChanged,
    /// A special workspace was removed.
    SpecialRemoved,
    /// A window opened within a workspace.
    WindowOpened,
    /// A window closed within a workspace.
    WindowClosed,
    /// A window was moved between workspaces.
    WindowMoved,
    /// The active monitor changed.
    ActiveMonitorChanged,
}

/// Keyboard related Hyprland events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HyprlandKeyboardEvent {
    /// The active keyboard layout changed.
    LayoutChanged(String),
    /// Keyboard layout configuration changed (e.g. config reload).
    LayoutConfigurationChanged(bool),
    /// The active keyboard submap changed.
    SubmapChanged(Option<String>),
}

/// Abstraction over Hyprland-specific functionality required by Hydebar modules.
///
/// Backends are expected to provide retry/timeout behaviour and surface errors
/// using [`HyprlandError`]. All methods must be thread-safe.
///
/// # Examples
/// ```ignore
/// use std::sync::Arc;
/// use hydebar_proto::ports::hyprland::{
///     HyprlandEventStream, HyprlandKeyboardEvent, HyprlandKeyboardState, HyprlandMonitorSelector,
///     HyprlandPort, HyprlandWorkspaceEvent, HyprlandWorkspaceSelector, HyprlandWindowEvent,
/// };
///
/// struct DummyPort;
///
/// impl HyprlandPort for DummyPort {
///     fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
///         Err(HyprlandError::unsupported("window_events"))
///     }
///
///     fn workspace_events(
///         &self,
///     ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
///         Err(HyprlandError::unsupported("workspace_events"))
///     }
///
///     fn keyboard_events(
///         &self,
///     ) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
///         Err(HyprlandError::unsupported("keyboard_events"))
///     }
///
///     fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError> {
///         Err(HyprlandError::unsupported("active_window"))
///     }
///
///     fn workspace_snapshot(&self) -> Result<HyprlandWorkspaceSnapshot, HyprlandError> {
///         Err(HyprlandError::unsupported("workspace_snapshot"))
///     }
///
///     fn change_workspace(
///         &self,
///         _: HyprlandWorkspaceSelector,
///     ) -> Result<(), HyprlandError> {
///         Err(HyprlandError::unsupported("change_workspace"))
///     }
///
///     fn focus_and_toggle_special_workspace(
///         &self,
///         _: HyprlandMonitorSelector,
///         _: &str,
///     ) -> Result<(), HyprlandError> {
///         Err(HyprlandError::unsupported("focus_and_toggle_special_workspace"))
///     }
///
///     fn keyboard_state(&self) -> Result<HyprlandKeyboardState, HyprlandError> {
///         Err(HyprlandError::unsupported("keyboard_state"))
///     }
///
///     fn switch_keyboard_layout(&self) -> Result<(), HyprlandError> {
///         Err(HyprlandError::unsupported("switch_keyboard_layout"))
///     }
/// }
///
/// let port: Arc<dyn HyprlandPort> = Arc::new(DummyPort);
/// assert!(port.active_window().is_err());
/// ```
pub trait HyprlandPort: Send + Sync {
    /// Subscribe to window related events.
    fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError>;

    /// Subscribe to workspace related events.
    fn workspace_events(
        &self,
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError>;

    /// Subscribe to keyboard related events.
    fn keyboard_events(&self) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError>;

    /// Retrieve the currently active window, if any.
    fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError>;

    /// Obtain the latest snapshot of monitors and workspaces.
    fn workspace_snapshot(&self) -> Result<HyprlandWorkspaceSnapshot, HyprlandError>;

    /// Request Hyprland to focus the provided workspace.
    fn change_workspace(&self, workspace: HyprlandWorkspaceSelector) -> Result<(), HyprlandError>;

    /// Focus the provided monitor and toggle a special workspace by name.
    fn focus_and_toggle_special_workspace(
        &self,
        monitor: HyprlandMonitorSelector,
        workspace_name: &str,
    ) -> Result<(), HyprlandError>;

    /// Retrieve the current keyboard state, including layout metadata.
    fn keyboard_state(&self) -> Result<HyprlandKeyboardState, HyprlandError>;

    /// Request Hyprland to switch to the next keyboard layout.
    fn switch_keyboard_layout(&self) -> Result<(), HyprlandError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_selector_display() {
        assert_eq!(HyprlandMonitorSelector::Id(3).to_string(), "monitor-id:3");
        assert_eq!(
            HyprlandMonitorSelector::Name("DP-1".into()).to_string(),
            "monitor-name:DP-1"
        );
    }

    #[test]
    fn workspace_selector_display() {
        assert_eq!(
            HyprlandWorkspaceSelector::Id(2).to_string(),
            "workspace-id:2"
        );
        assert_eq!(
            HyprlandWorkspaceSelector::Name("code".into()).to_string(),
            "workspace-name:code"
        );
    }

    #[test]
    fn keyboard_state_equality() {
        let state_a = HyprlandKeyboardState {
            active_layout: "us".into(),
            has_multiple_layouts: true,
            active_submap: Some("resize".into()),
        };
        let state_b = state_a.clone();
        assert_eq!(state_a, state_b);
    }
}
