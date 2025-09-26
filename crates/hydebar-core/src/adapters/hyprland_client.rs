use std::{sync::Arc, thread, time::Duration};

use hydebar_proto::ports::hyprland::{
    HyprlandError, HyprlandEventStream, HyprlandKeyboardEvent, HyprlandKeyboardState,
    HyprlandMonitorInfo, HyprlandMonitorSelector, HyprlandPort, HyprlandWindowEvent,
    HyprlandWindowInfo, HyprlandWorkspaceEvent, HyprlandWorkspaceInfo, HyprlandWorkspaceSelector,
    HyprlandWorkspaceSnapshot,
};
use hyprland::{
    ctl::switch_xkb_layout::SwitchXKBLayoutCmdTypes,
    data::{Client, Devices, Monitors, Workspace, Workspaces},
    dispatch::{Dispatch, DispatchType, MonitorIdentifier, WorkspaceIdentifierWithSpecial},
    event_listener::AsyncEventListener,
    keyword::Keyword,
};
use log::warn;
use tokio::{
    runtime::Handle,
    sync::mpsc,
    time::{sleep, timeout},
};
use tokio_stream::wrappers::ReceiverStream;

const CHANNEL_CAPACITY: usize = 64;
const WINDOW_EVENTS_OP: &str = "window_events";
const WORKSPACE_EVENTS_OP: &str = "workspace_events";
const KEYBOARD_EVENTS_OP: &str = "keyboard_events";
const WORKSPACE_SNAPSHOT_OP: &str = "workspace_snapshot";
const ACTIVE_WINDOW_OP: &str = "active_window";
const CHANGE_WORKSPACE_OP: &str = "change_workspace";
const TOGGLE_SPECIAL_OP: &str = "toggle_special_workspace";
const KEYBOARD_STATE_OP: &str = "keyboard_state";
const SWITCH_LAYOUT_OP: &str = "switch_keyboard_layout";

/// Configuration options for [`HyprlandClient`].
#[derive(Clone, Debug)]
pub struct HyprlandClientConfig {
    /// Maximum duration to wait for a synchronous Hyprland request to complete.
    pub request_timeout: Duration,
    /// Maximum time to wait for the Hyprland event listener to yield before treating it as hung.
    pub listener_timeout: Duration,
    /// Total number of retry attempts for synchronous Hyprland requests.
    pub retry_attempts: u8,
    /// Base delay between retry attempts for synchronous Hyprland requests.
    pub retry_backoff: Duration,
}

impl Default for HyprlandClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(2),
            listener_timeout: Duration::from_secs(60),
            retry_attempts: 3,
            retry_backoff: Duration::from_millis(250),
        }
    }
}

/// [`HyprlandPort`] implementation backed by the `hyprland-rs` crate.
#[derive(Clone, Debug)]
pub struct HyprlandClient {
    config: Arc<HyprlandClientConfig>,
}

impl Default for HyprlandClient {
    fn default() -> Self {
        Self {
            config: Arc::new(HyprlandClientConfig::default()),
        }
    }
}

impl HyprlandClient {
    /// Construct a new [`HyprlandClient`] using [`HyprlandClientConfig::default`].
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a [`HyprlandClient`] with the provided configuration.
    pub fn with_config(config: HyprlandClientConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    fn backend_error<E>(operation: &'static str, err: E) -> HyprlandError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HyprlandError::Backend {
            operation,
            source: Box::new(err),
        }
    }

    fn execute_once<R, F>(
        operation: &'static str,
        timeout_dur: Duration,
        func: Arc<F>,
    ) -> Result<R, HyprlandError>
    where
        R: Send + 'static,
        F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let result = func();
            if tx.send(result).is_err() {
                warn!(
                    target: "hydebar::hyprland",
                    "result receiver dropped before completion (operation={operation})"
                );
            }
        });

        match rx.recv_timeout(timeout_dur) {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(HyprlandError::Timeout {
                operation,
                timeout: timeout_dur,
            }),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(HyprlandError::message(
                operation,
                "worker thread terminated before sending result",
            )),
        }
    }

    fn execute_with_retry<R, F>(&self, operation: &'static str, func: F) -> Result<R, HyprlandError>
    where
        R: Send + 'static,
        F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static,
    {
        let func = Arc::new(func);
        let mut last_error = None;
        for attempt in 1..=self.config.retry_attempts {
            let func_clone = Arc::clone(&func);
            match Self::execute_once(operation, self.config.request_timeout, func_clone) {
                Ok(result) => return Ok(result),
                Err(err) => {
                    warn!(
                        target: "hydebar::hyprland",
                        "Hyprland operation failed (operation={operation}, attempt={attempt}, error={err})"
                    );
                    last_error = Some(err);
                    if attempt < self.config.retry_attempts {
                        let delay = self.config.retry_backoff.saturating_mul(u32::from(attempt));
                        thread::sleep(delay);
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            HyprlandError::message(operation, "Hyprland operation failed without error detail")
        }))
    }

    fn spawn_window_listener(
        &self,
    ) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
        let handle = Handle::try_current()
            .map_err(|_| HyprlandError::runtime_unavailable(WINDOW_EVENTS_OP))?;
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let listener_timeout = self.config.listener_timeout;
        let retry_backoff = self.config.retry_backoff;

        handle.spawn(async move {
            let mut tx = tx;
            loop {
                let mut listener = AsyncEventListener::new();

                listener.add_active_window_changed_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) =
                                tx.send(Ok(HyprlandWindowEvent::ActiveWindowChanged)).await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "window event receiver dropped (operation={}, error={err})",
                                    WINDOW_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_window_closed_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) = tx.send(Ok(HyprlandWindowEvent::WindowClosed)).await {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "window event receiver dropped (operation={}, error={err})",
                                    WINDOW_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_workspace_changed_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) = tx
                                .send(Ok(HyprlandWindowEvent::WorkspaceFocusChanged))
                                .await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "window event receiver dropped (operation={}, error={err})",
                                    WINDOW_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                let result = timeout(listener_timeout, listener.start_listener_async()).await;
                match result {
                    Ok(Ok(())) => {
                        warn!(
                            target: "hydebar::hyprland",
                            "window listener stopped unexpectedly (operation={})",
                            WINDOW_EVENTS_OP
                        );
                    }
                    Ok(Err(err)) => {
                        let send_err = tx
                            .send(Err(HyprlandClient::backend_error(WINDOW_EVENTS_OP, err)))
                            .await;
                        if let Err(send_err) = send_err {
                            warn!(
                                target: "hydebar::hyprland",
                                "failed to publish window listener error (operation={}, error={send_err})",
                                WINDOW_EVENTS_OP
                            );
                            break;
                        }
                    }
                    Err(_) => {
                        let send_err = tx
                            .send(Err(HyprlandError::Timeout {
                                operation: WINDOW_EVENTS_OP,
                                timeout: listener_timeout,
                            }))
                            .await;
                        if let Err(send_err) = send_err {
                            warn!(
                                target: "hydebar::hyprland",
                                "failed to publish window listener timeout (operation={}, error={send_err})",
                                WINDOW_EVENTS_OP
                            );
                            break;
                        }
                    }
                }

                if tx.is_closed() {
                    break;
                }

                sleep(retry_backoff).await;
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn spawn_workspace_listener(
        &self,
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
        let handle = Handle::try_current()
            .map_err(|_| HyprlandError::runtime_unavailable(WORKSPACE_EVENTS_OP))?;
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let listener_timeout = self.config.listener_timeout;
        let retry_backoff = self.config.retry_backoff;

        handle.spawn(async move {
            let mut tx = tx;
            loop {
                let mut listener = AsyncEventListener::new();

                listener.add_workspace_added_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::Added)).await {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_workspace_changed_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::Changed)).await {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_workspace_deleted_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::Removed)).await {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_workspace_moved_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::Moved)).await {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_changed_special_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) =
                                tx.send(Ok(HyprlandWorkspaceEvent::SpecialChanged)).await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_special_removed_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) =
                                tx.send(Ok(HyprlandWorkspaceEvent::SpecialRemoved)).await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_window_closed_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) =
                                tx.send(Ok(HyprlandWorkspaceEvent::WindowClosed)).await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_window_opened_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) =
                                tx.send(Ok(HyprlandWorkspaceEvent::WindowOpened)).await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_window_moved_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) = tx.send(Ok(HyprlandWorkspaceEvent::WindowMoved)).await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                listener.add_active_monitor_changed_handler({
                    let tx = tx.clone();
                    move |_| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            if let Err(err) = tx
                                .send(Ok(HyprlandWorkspaceEvent::ActiveMonitorChanged))
                                .await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "workspace event receiver dropped (operation={}, error={err})",
                                    WORKSPACE_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                let result = timeout(listener_timeout, listener.start_listener_async()).await;
                match result {
                    Ok(Ok(())) => {
                        warn!(
                            target: "hydebar::hyprland",
                            "workspace listener stopped unexpectedly (operation={})",
                            WORKSPACE_EVENTS_OP
                        );
                    }
                    Ok(Err(err)) => {
                        let send_err = tx
                            .send(Err(HyprlandClient::backend_error(WORKSPACE_EVENTS_OP, err)))
                            .await;
                        if let Err(send_err) = send_err {
                            warn!(
                                target: "hydebar::hyprland",
                                "failed to publish workspace listener error (operation={}, error={send_err})",
                                WORKSPACE_EVENTS_OP
                            );
                            break;
                        }
                    }
                    Err(_) => {
                        let send_err = tx
                            .send(Err(HyprlandError::Timeout {
                                operation: WORKSPACE_EVENTS_OP,
                                timeout: listener_timeout,
                            }))
                            .await;
                        if let Err(send_err) = send_err {
                            warn!(
                                target: "hydebar::hyprland",
                                "failed to publish workspace listener timeout (operation={}, error={send_err})",
                                WORKSPACE_EVENTS_OP
                            );
                            break;
                        }
                    }
                }

                if tx.is_closed() {
                    break;
                }

                sleep(retry_backoff).await;
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn spawn_keyboard_listener(
        &self,
    ) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
        let handle = Handle::try_current()
            .map_err(|_| HyprlandError::runtime_unavailable(KEYBOARD_EVENTS_OP))?;
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let listener_timeout = self.config.listener_timeout;
        let retry_backoff = self.config.retry_backoff;
        let client = self.clone();

        handle.spawn(async move {
            let mut tx = tx;
            loop {
                let mut listener = AsyncEventListener::new();

                listener.add_layout_changed_handler({
                    let tx = tx.clone();
                    let client = client.clone();
                    move |_| {
                        let tx = tx.clone();
                        let client = client.clone();
                        Box::pin(async move {
                            match client.keyboard_state() {
                                Ok(state) => {
                                    if let Err(err) = tx
                                        .send(Ok(HyprlandKeyboardEvent::LayoutChanged(
                                            state.active_layout,
                                        )))
                                        .await
                                    {
                                        warn!(
                                            target: "hydebar::hyprland",
                                            "keyboard event receiver dropped (operation={}, error={err})",
                                            KEYBOARD_EVENTS_OP
                                        );
                                    }
                                }
                                Err(err) => {
                                    if let Err(send_err) = tx.send(Err(err)).await {
                                        warn!(
                                            target: "hydebar::hyprland",
                                            "failed to publish keyboard state error (operation={}, error={send_err})",
                                            KEYBOARD_EVENTS_OP
                                        );
                                    }
                                }
                            }
                        })
                    }
                });

                listener.add_config_reloaded_handler({
                    let tx = tx.clone();
                    let client = client.clone();
                    move || {
                        let tx = tx.clone();
                        let client = client.clone();
                        Box::pin(async move {
                            match client.keyboard_state() {
                                Ok(state) => {
                                    if let Err(err) = tx
                                        .send(Ok(
                                            HyprlandKeyboardEvent::LayoutConfigurationChanged(
                                                state.has_multiple_layouts,
                                            ),
                                        ))
                                        .await
                                    {
                                        warn!(
                                            target: "hydebar::hyprland",
                                            "keyboard event receiver dropped (operation={}, error={err})",
                                            KEYBOARD_EVENTS_OP
                                        );
                                    }
                                }
                                Err(err) => {
                                    if let Err(send_err) = tx.send(Err(err)).await {
                                        warn!(
                                            target: "hydebar::hyprland",
                                            "failed to publish keyboard config error (operation={}, error={send_err})",
                                            KEYBOARD_EVENTS_OP
                                        );
                                    }
                                }
                            }
                        })
                    }
                });

                listener.add_sub_map_changed_handler({
                    let tx = tx.clone();
                    move |submap| {
                        let tx = tx.clone();
                        Box::pin(async move {
                            let payload = if submap.trim().is_empty() {
                                None
                            } else {
                                Some(submap)
                            };
                            if let Err(err) = tx
                                .send(Ok(HyprlandKeyboardEvent::SubmapChanged(payload)))
                                .await
                            {
                                warn!(
                                    target: "hydebar::hyprland",
                                    "keyboard event receiver dropped (operation={}, error={err})",
                                    KEYBOARD_EVENTS_OP
                                );
                            }
                        })
                    }
                });

                let result = timeout(listener_timeout, listener.start_listener_async()).await;
                match result {
                    Ok(Ok(())) => {
                        warn!(
                            target: "hydebar::hyprland",
                            "keyboard listener stopped unexpectedly (operation={})",
                            KEYBOARD_EVENTS_OP
                        );
                    }
                    Ok(Err(err)) => {
                        let send_err = tx
                            .send(Err(HyprlandClient::backend_error(KEYBOARD_EVENTS_OP, err)))
                            .await;
                        if let Err(send_err) = send_err {
                            warn!(
                                target: "hydebar::hyprland",
                                "failed to publish keyboard listener error (operation={}, error={send_err})",
                                KEYBOARD_EVENTS_OP
                            );
                            break;
                        }
                    }
                    Err(_) => {
                        let send_err = tx
                            .send(Err(HyprlandError::Timeout {
                                operation: KEYBOARD_EVENTS_OP,
                                timeout: listener_timeout,
                            }))
                            .await;
                        if let Err(send_err) = send_err {
                            warn!(
                                target: "hydebar::hyprland",
                                "failed to publish keyboard listener timeout (operation={}, error={send_err})",
                                KEYBOARD_EVENTS_OP
                            );
                            break;
                        }
                    }
                }

                if tx.is_closed() {
                    break;
                }

                sleep(retry_backoff).await;
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }
}

impl HyprlandPort for HyprlandClient {
    fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
        self.spawn_window_listener()
    }

    fn workspace_events(
        &self,
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
        self.spawn_workspace_listener()
    }

    fn keyboard_events(&self) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
        self.spawn_keyboard_listener()
    }

    fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError> {
        self.execute_with_retry(ACTIVE_WINDOW_OP, || {
            Client::get_active()
                .map_err(|err| HyprlandClient::backend_error(ACTIVE_WINDOW_OP, err))
                .map(|maybe_client| {
                    maybe_client.map(|client| HyprlandWindowInfo {
                        title: client.title,
                        class: client.class,
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
                .to_vec()
                .into_iter()
                .map(|monitor| HyprlandMonitorInfo {
                    id: monitor.id,
                    name: monitor.name,
                    special_workspace_id: Some(monitor.special_workspace.id),
                })
                .collect();

            let workspaces = workspaces
                .to_vec()
                .into_iter()
                .map(|workspace| HyprlandWorkspaceInfo {
                    id: workspace.id,
                    name: workspace.name,
                    monitor_id: workspace.monitor_id.and_then(|id| usize::try_from(id).ok()),
                    monitor_name: workspace.monitor,
                    window_count: workspace.windows.try_into().unwrap_or(u16::MAX),
                })
                .collect();

            Ok(HyprlandWorkspaceSnapshot {
                monitors,
                workspaces,
                active_workspace_id: active.map(|w| w.id),
            })
        })
    }

    fn change_workspace(&self, workspace: HyprlandWorkspaceSelector) -> Result<(), HyprlandError> {
        self.execute_with_retry(CHANGE_WORKSPACE_OP, || {
            let identifier = match &workspace {
                HyprlandWorkspaceSelector::Id(id) => WorkspaceIdentifierWithSpecial::Id(*id),
                HyprlandWorkspaceSelector::Name(name) => {
                    WorkspaceIdentifierWithSpecial::Name(name.clone())
                }
            };
            Dispatch::call(DispatchType::Workspace(identifier))
                .map_err(|err| HyprlandClient::backend_error(CHANGE_WORKSPACE_OP, err))
        })
    }

    fn focus_and_toggle_special_workspace(
        &self,
        monitor: HyprlandMonitorSelector,
        workspace_name: &str,
    ) -> Result<(), HyprlandError> {
        let workspace_name = workspace_name.to_string();
        self.execute_with_retry(TOGGLE_SPECIAL_OP, || {
            let monitor_identifier = match &monitor {
                HyprlandMonitorSelector::Id(id) => MonitorIdentifier::Id((*id).into()),
                HyprlandMonitorSelector::Name(name) => MonitorIdentifier::Name(name.clone()),
            };
            Dispatch::call(DispatchType::FocusMonitor(monitor_identifier))
                .and_then(|_| {
                    Dispatch::call(DispatchType::ToggleSpecialWorkspace(Some(
                        workspace_name.clone(),
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
                active_submap: None,
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
