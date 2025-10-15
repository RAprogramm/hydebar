// Module available for both internal tests and cross-crate testing via feature
// flag
#![cfg(any(test, feature = "test-utils"))]

use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering}
};

use hydebar_proto::ports::hyprland::{
    HyprlandError, HyprlandEventStream, HyprlandKeyboardEvent, HyprlandKeyboardState,
    HyprlandMonitorInfo, HyprlandMonitorSelector, HyprlandPort, HyprlandWindowEvent,
    HyprlandWindowInfo, HyprlandWorkspaceEvent, HyprlandWorkspaceInfo, HyprlandWorkspaceSelector,
    HyprlandWorkspaceSnapshot
};
use tokio_stream;

#[derive(Debug)]
pub struct MockHyprlandPort {
    pub active_window:          Mutex<Option<HyprlandWindowInfo>>,
    pub workspace_snapshot:     Mutex<HyprlandWorkspaceSnapshot>,
    pub keyboard_state:         Mutex<HyprlandKeyboardState>,
    pub change_workspace_calls: AtomicUsize,
    pub toggle_special_calls:   AtomicUsize,
    pub switch_layout_calls:    AtomicUsize
}

impl Default for MockHyprlandPort {
    fn default() -> Self {
        Self {
            active_window:          Mutex::new(Some(HyprlandWindowInfo {
                title: "Mock Window".into(),
                class: "MockClass".into()
            })),
            workspace_snapshot:     Mutex::new(HyprlandWorkspaceSnapshot {
                monitors:            vec![HyprlandMonitorInfo {
                    id:                   0,
                    name:                 "MockMonitor".into(),
                    special_workspace_id: None
                }],
                workspaces:          vec![HyprlandWorkspaceInfo {
                    id:           1,
                    name:         "1".into(),
                    monitor_id:   Some(0),
                    monitor_name: "MockMonitor".into(),
                    window_count: 0
                }],
                active_workspace_id: Some(1)
            }),
            keyboard_state:         Mutex::new(HyprlandKeyboardState {
                active_layout:        "us".into(),
                has_multiple_layouts: true,
                active_submap:        Some("resize".into())
            }),
            change_workspace_calls: AtomicUsize::new(0),
            toggle_special_calls:   AtomicUsize::new(0),
            switch_layout_calls:    AtomicUsize::new(0)
        }
    }
}

impl MockHyprlandPort {
    pub fn with_active_window(title: &str, class: &str) -> Self {
        let port = Self::default();
        *port
            .active_window
            .lock()
            .expect("poisoned active window lock") = Some(HyprlandWindowInfo {
            title: title.into(),
            class: class.into()
        });
        port
    }

    pub fn workspace_calls(&self) -> usize {
        self.change_workspace_calls.load(Ordering::SeqCst)
    }

    pub fn toggle_special_calls(&self) -> usize {
        self.toggle_special_calls.load(Ordering::SeqCst)
    }

    pub fn switch_layout_calls(&self) -> usize {
        self.switch_layout_calls.load(Ordering::SeqCst)
    }
}

impl HyprlandPort for MockHyprlandPort {
    fn window_events(&self) -> Result<HyprlandEventStream<HyprlandWindowEvent>, HyprlandError> {
        Ok(Box::pin(tokio_stream::pending()))
    }

    fn workspace_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandWorkspaceEvent>, HyprlandError> {
        Ok(Box::pin(tokio_stream::pending()))
    }

    fn keyboard_events(
        &self
    ) -> Result<HyprlandEventStream<HyprlandKeyboardEvent>, HyprlandError> {
        Ok(Box::pin(tokio_stream::pending()))
    }

    fn active_window(&self) -> Result<Option<HyprlandWindowInfo>, HyprlandError> {
        Ok(self
            .active_window
            .lock()
            .expect("poisoned active window lock")
            .clone())
    }

    fn workspace_snapshot(&self) -> Result<HyprlandWorkspaceSnapshot, HyprlandError> {
        Ok(self
            .workspace_snapshot
            .lock()
            .expect("poisoned workspace snapshot lock")
            .clone())
    }

    fn change_workspace(&self, _: HyprlandWorkspaceSelector) -> Result<(), HyprlandError> {
        self.change_workspace_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn focus_and_toggle_special_workspace(
        &self,
        _: HyprlandMonitorSelector,
        _: &str
    ) -> Result<(), HyprlandError> {
        self.toggle_special_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn keyboard_state(&self) -> Result<HyprlandKeyboardState, HyprlandError> {
        Ok(self
            .keyboard_state
            .lock()
            .expect("poisoned keyboard state lock")
            .clone())
    }

    fn switch_keyboard_layout(&self) -> Result<(), HyprlandError> {
        self.switch_layout_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
