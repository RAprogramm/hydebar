mod config;
mod listeners;
mod sync_ops;
mod util;

use std::sync::Arc;

use hydebar_proto::ports::hyprland::{
    HyprlandError, HyprlandEventStream, HyprlandKeyboardEvent, HyprlandKeyboardState,
    HyprlandMonitorInfo, HyprlandMonitorSelector, HyprlandPort, HyprlandWindowEvent,
    HyprlandWindowInfo, HyprlandWorkspaceEvent, HyprlandWorkspaceInfo, HyprlandWorkspaceSelector,
    HyprlandWorkspaceSnapshot
};
use hyprland::{
    ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes,
    data::{Client, Devices, Monitors, Workspace, Workspaces},
    dispatch::{Dispatch, DispatchType, MonitorIdentifier, WorkspaceIdentifierWithSpecial},
    keyword::Keyword,
    shared::{HyprData, HyprDataActive, HyprDataActiveOptional}
};

pub use self::config::HyprlandClientConfig;
use self::{
    listeners::{spawn_keyboard_listener, spawn_window_listener, spawn_workspace_listener},
    sync_ops::execute_with_retry
};

const WORKSPACE_SNAPSHOT_OP: &str = "workspace_snapshot";
const ACTIVE_WINDOW_OP: &str = "active_window";
const CHANGE_WORKSPACE_OP: &str = "change_workspace";
const TOGGLE_SPECIAL_OP: &str = "toggle_special_workspace";
const KEYBOARD_STATE_OP: &str = "keyboard_state";
const SWITCH_LAYOUT_OP: &str = "switch_keyboard_layout";

/// [`HyprlandPort`] implementation backed by the `hyprland-rs` crate.
#[derive(Clone, Debug)]
pub struct HyprlandClient {
    config: Arc<HyprlandClientConfig>
}

impl Default for HyprlandClient {
    fn default() -> Self {
        Self {
            config: Arc::new(HyprlandClientConfig::default())
        }
    }
}

impl HyprlandClient {
    /// Construct a new [`HyprlandClient`] using
    /// [`HyprlandClientConfig::default`].
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a [`HyprlandClient`] with the provided configuration.
    pub fn with_config(config: HyprlandClientConfig) -> Self {
        Self {
            config: Arc::new(config)
        }
    }

    pub(crate) fn backend_error<E>(operation: &'static str, err: E) -> HyprlandError
    where
        E: std::error::Error + Send + Sync + 'static
    {
        HyprlandError::Backend {
            operation,
            source: Box::new(err)
        }
    }

    fn execute_with_retry<R, F>(
        &self,
        operation: &'static str,
        func: F
    ) -> Result<R, HyprlandError>
    where
        R: Send + 'static,
        F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static
    {
        execute_with_retry(&self.config, operation, func)
    }

    fn spawn_window_listener(
        &self
    ) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
        spawn_window_listener(self.config.clone())
    }

    fn spawn_workspace_listener(
        &self
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
        spawn_workspace_listener(self.config.clone())
    }

    fn spawn_keyboard_listener(
        &self
    ) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
        spawn_keyboard_listener(self.clone(), self.config.clone())
    }
}

impl HyprlandPort for HyprlandClient {
    fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
        self.spawn_window_listener()
    }

    fn workspace_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
        self.spawn_workspace_listener()
    }

    fn keyboard_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
        self.spawn_keyboard_listener()
    }

    fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError> {
        self.execute_with_retry(ACTIVE_WINDOW_OP, || {
            Client::get_active()
                .map_err(|err| HyprlandClient::backend_error(ACTIVE_WINDOW_OP, err))
                .map(|maybe_client| {
                    maybe_client.map(|client| HyprlandWindowInfo {
                        title: client.title,
                        class: client.class
                    })
                })
        })
    }

    fn workspace_snapshot(&self) -> Result<HyprlandWorkspaceSnapshot, HyprlandError> {
        self.execute_with_retry(WORKSPACE_SNAPSHOT_OP, || {
            let monitors = Monitors::get()
                .map_err(|err| HyprlandClient::backend_error(WORKSPACE_SNAPSHOT_OP, err))?;
            let workspaces = Workspaces::get()
                .map_err(|err| HyprlandClient::backend_error(WORKSPACE_SNAPSHOT_OP, err))?;
            let active = Workspace::get_active()
                .map_err(|err| HyprlandClient::backend_error(WORKSPACE_SNAPSHOT_OP, err))?;

            let monitors = monitors
                .into_iter()
                .map(|monitor| HyprlandMonitorInfo {
                    id:                   i32::try_from(monitor.id).unwrap_or(i32::MAX),
                    name:                 monitor.name,
                    special_workspace_id: Some(monitor.special_workspace.id)
                })
                .collect();

            let workspaces = workspaces
                .into_iter()
                .map(|workspace| HyprlandWorkspaceInfo {
                    id:           workspace.id,
                    name:         workspace.name,
                    monitor_id:   workspace.monitor_id.and_then(|id| usize::try_from(id).ok()),
                    monitor_name: workspace.monitor,
                    window_count: workspace.windows
                })
                .collect();

            Ok(HyprlandWorkspaceSnapshot {
                monitors,
                workspaces,
                active_workspace_id: Some(active.id)
            })
        })
    }

    fn change_workspace(&self, workspace: HyprlandWorkspaceSelector) -> Result<(), HyprlandError> {
        self.execute_with_retry(CHANGE_WORKSPACE_OP, move || {
            let identifier = match &workspace {
                HyprlandWorkspaceSelector::Id(id) => WorkspaceIdentifierWithSpecial::Id(*id),
                HyprlandWorkspaceSelector::Name(name) => {
                    WorkspaceIdentifierWithSpecial::Name(name.as_str())
                }
            };
            Dispatch::call(DispatchType::Workspace(identifier))
                .map_err(|err| HyprlandClient::backend_error(CHANGE_WORKSPACE_OP, err))
        })
    }

    fn focus_and_toggle_special_workspace(
        &self,
        monitor: HyprlandMonitorSelector,
        workspace_name: &str
    ) -> Result<(), HyprlandError> {
        let workspace_name = workspace_name.to_string();
        self.execute_with_retry(TOGGLE_SPECIAL_OP, move || {
            let monitor_identifier = match &monitor {
                HyprlandMonitorSelector::Id(id) => {
                    MonitorIdentifier::Id((*id).try_into().unwrap_or(i128::MAX))
                }
                HyprlandMonitorSelector::Name(name) => MonitorIdentifier::Name(name.as_str())
            };
            Dispatch::call(DispatchType::FocusMonitor(monitor_identifier))
                .and_then(|_| {
                    Dispatch::call(DispatchType::ToggleSpecialWorkspace(Some(
                        workspace_name.clone()
                    )))
                })
                .map_err(|err| HyprlandClient::backend_error(TOGGLE_SPECIAL_OP, err))
        })
    }

    fn keyboard_state(&self) -> Result<HyprlandKeyboardState, HyprlandError> {
        self.execute_with_retry(KEYBOARD_STATE_OP, || {
            let keyword = Keyword::get("input:kb_layout")
                .map_err(|err| HyprlandClient::backend_error(KEYBOARD_STATE_OP, err))?;
            let has_multiple_layouts = keyword
                .value
                .to_string()
                .split(',')
                .filter(|value| !value.trim().is_empty())
                .count()
                > 1;

            let devices = Devices::get()
                .map_err(|err| HyprlandClient::backend_error(KEYBOARD_STATE_OP, err))?;
            let active_layout = devices
                .keyboards
                .iter()
                .find(|keyboard| keyboard.main)
                .map(|keyboard| keyboard.active_keymap.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            Ok(HyprlandKeyboardState {
                active_layout,
                has_multiple_layouts,
                active_submap: None
            })
        })
    }

    fn switch_keyboard_layout(&self) -> Result<(), HyprlandError> {
        self.execute_with_retry(SWITCH_LAYOUT_OP, || {
            hyprland::ctl::switch_xkb_layout::call("all", SwitchXKBLayoutCmdTypes::Next)
                .map_err(|err| HyprlandClient::backend_error(SWITCH_LAYOUT_OP, err))
        })
    }
}
