use std::{sync::Arc, time::Duration};

use hydebar_proto::ports::hyprland::{
    HyprlandMonitorSelector, HyprlandPort, HyprlandWorkspaceEvent, HyprlandWorkspaceSelector,
    HyprlandWorkspaceSnapshot
};
use iced::{
    Element, Length, alignment,
    widget::{Row, button, container, text},
    window::Id
};
use itertools::Itertools;
use log::{debug, error};
use tokio::{task::JoinHandle, time::sleep};
use tokio_stream::StreamExt;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, ModuleEventSender,
    config::{AppearanceColor, WorkspaceVisibilityMode, WorkspacesModuleConfig},
    event_bus::ModuleEvent,
    outputs::Outputs,
    style::workspace_button_style
};

const WORKSPACE_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id:         i32,
    pub name:       String,
    pub monitor_id: Option<usize>, // index for color lookup; may be None
    pub monitor:    String,        // monitor name for fallback
    pub active:     bool,
    pub windows:    u16
}

fn get_workspaces(port: &dyn HyprlandPort, config: &WorkspacesModuleConfig) -> Vec<Workspace> {
    let snapshot = match port.workspace_snapshot() {
        Ok(snapshot) => snapshot,
        Err(err) => {
            error!("failed to retrieve workspace snapshot: {err}");
            return Vec::new();
        }
    };

    map_snapshot_to_workspaces(&snapshot, config)
}

fn map_snapshot_to_workspaces(
    snapshot: &HyprlandWorkspaceSnapshot,
    config: &WorkspacesModuleConfig
) -> Vec<Workspace> {
    let active = snapshot.active_workspace_id;
    let monitors = &snapshot.monitors;

    // Deduplicate by ID to avoid duplicates from Hyprland.
    let workspaces: Vec<_> = snapshot.workspaces.iter().unique_by(|w| w.id).collect();

    // Preallocate result vector.
    let mut result: Vec<Workspace> = Vec::with_capacity(workspaces.len());

    let (special, normal): (Vec<_>, Vec<_>) = workspaces.into_iter().partition(|w| w.id < 0);

    // Map special workspaces.
    for w in special.iter() {
        result.push(Workspace {
            id:         w.id,
            name:       w
                .name
                .as_str()
                .split(':')
                .next_back()
                .map_or_else(String::new, ToOwned::to_owned),
            // Option<i128> -> Option<usize> with bounds check.
            monitor_id: w.monitor_id,
            monitor:    w.monitor_name.clone(),
            active:     monitors
                .iter()
                .any(|m| m.special_workspace_id == Some(w.id)),
            windows:    w.window_count
        });
    }

    // Map normal workspaces.
    for w in normal.iter() {
        result.push(Workspace {
            id:         w.id,
            name:       w.name.clone(),
            monitor_id: w.monitor_id,
            monitor:    w.monitor_name.clone(),
            active:     Some(w.id) == active,
            windows:    w.window_count
        });
    }

    if !config.enable_workspace_filling || normal.is_empty() {
        result.sort_by_key(|w| w.id);
        return result;
    }

    // Synthesize "missing" workspaces [1..=max_id] for filling UI.
    let existing_ids = normal.iter().map(|w| w.id).collect::<Vec<_>>();
    let mut max_id = *existing_ids.iter().max().unwrap_or(&0);
    if let Some(max_workspaces) = config.max_workspaces
        && max_workspaces > max_id as u32
    {
        max_id = max_workspaces as i32;
    }

    let missing_ids: Vec<i32> = (1..=max_id)
        .filter(|id| !existing_ids.contains(id))
        .collect();

    result.reserve(missing_ids.len());

    for id in missing_ids {
        result.push(Workspace {
            id,
            name: id.to_string(),
            monitor_id: None,
            monitor: String::new(),
            active: false,
            windows: 0
        });
    }

    result.sort_by_key(|w| w.id);
    result
}

pub struct Workspaces {
    hyprland:   Arc<dyn HyprlandPort>,
    workspaces: Vec<Workspace>,
    sender:     Option<ModuleEventSender<Message>>,
    task:       Option<JoinHandle<()>>
}

impl Workspaces {
    pub fn new(hyprland: Arc<dyn HyprlandPort>, config: &WorkspacesModuleConfig) -> Self {
        let workspaces = get_workspaces(hyprland.as_ref(), config);

        Self {
            hyprland,
            workspaces,
            sender: None,
            task: None
        }
    }

    #[cfg(test)]
    pub(crate) fn items(&self) -> &[Workspace] {
        &self.workspaces
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesChanged,
    ChangeWorkspace(i32),
    ToggleSpecialWorkspace(i32)
}

impl Workspaces {
    pub fn update(&mut self, message: Message, config: &WorkspacesModuleConfig) {
        match message {
            Message::WorkspacesChanged => {
                self.workspaces = get_workspaces(self.hyprland.as_ref(), config);
            }
            Message::ChangeWorkspace(id) => {
                if id > 0 {
                    let already_active = self.workspaces.iter().any(|w| w.active && w.id == id);
                    if !already_active {
                        debug!("changing workspace to: {id}");
                        let res = self
                            .hyprland
                            .change_workspace(HyprlandWorkspaceSelector::Id(id));
                        if let Err(e) = res {
                            error!("failed to dispatch workspace change: {e}");
                        }
                    }
                }
            }
            Message::ToggleSpecialWorkspace(id) => {
                if let Some(special) = self.workspaces.iter().find(|w| w.id == id && w.id < 0) {
                    debug!("toggle special workspace: {id}");

                    // Prefer focusing by monitor index if present; otherwise, fall back to monitor
                    // name.
                    let monitor_ident = match special.monitor_id {
                        Some(idx) => HyprlandMonitorSelector::Id(idx),
                        None => HyprlandMonitorSelector::Name(special.monitor.clone())
                    };

                    let res = self
                        .hyprland
                        .focus_and_toggle_special_workspace(monitor_ident, &special.name);

                    if let Err(e) = res {
                        error!("failed to dispatch special workspace toggle: {e}");
                    }
                }
            }
        }
    }
}

impl<M> Module<M> for Workspaces
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = (
        &'a Outputs,
        Id,
        &'a WorkspacesModuleConfig,
        &'a [AppearanceColor],
        Option<&'a [AppearanceColor]>
    );
    type RegistrationData<'a> = &'a WorkspacesModuleConfig;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.workspaces = get_workspaces(self.hyprland.as_ref(), config);

        self.sender = Some(ctx.module_sender(ModuleEvent::Workspaces));

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(async move {
                loop {
                    match hyprland.workspace_events() {
                        Ok(mut stream) => {
                            while let Some(event) = stream.next().await {
                                match event {
                                    Ok(
                                        HyprlandWorkspaceEvent::Added
                                        | HyprlandWorkspaceEvent::Changed
                                        | HyprlandWorkspaceEvent::Removed
                                        | HyprlandWorkspaceEvent::Moved
                                        | HyprlandWorkspaceEvent::SpecialChanged
                                        | HyprlandWorkspaceEvent::SpecialRemoved
                                        | HyprlandWorkspaceEvent::WindowClosed
                                        | HyprlandWorkspaceEvent::WindowOpened
                                        | HyprlandWorkspaceEvent::WindowMoved
                                        | HyprlandWorkspaceEvent::ActiveMonitorChanged
                                    ) => {
                                        if let Err(err) =
                                            sender.try_send(Message::WorkspacesChanged)
                                        {
                                            error!("failed to publish workspace update: {err}");
                                        }
                                    }
                                    Err(err) => {
                                        error!("workspace event stream error: {err}");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("failed to start workspace event stream: {err}");
                        }
                    }

                    sleep(WORKSPACE_EVENT_RETRY_DELAY).await;
                }
            }));
        }

        if let Some(sender) = self.sender.clone()
            && let Err(err) = sender.try_send(Message::WorkspacesChanged)
        {
            error!("failed to enqueue initial workspace refresh: {err}");
        }

        Ok(())
    }

    fn view(
        &self,
        (outputs, id, config, workspace_colors, special_workspace_colors): Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        let monitor_name = outputs.get_monitor_name(id).map(|s| s.to_string());

        Some((
            Row::with_children(
                self.workspaces
                    .iter()
                    .filter_map(|w| {
                        if config.visibility_mode == WorkspaceVisibilityMode::All
                            || w.monitor == monitor_name.as_deref().unwrap_or(&w.monitor)
                            || !outputs.has_name(&w.monitor)
                        {
                            let empty = w.windows == 0;
                            let monitor = w.monitor_id;

                            // Safe color lookup by monitor index; None means "no color".
                            let color = monitor.map(|m| {
                                if w.id > 0 {
                                    workspace_colors.get(m).copied()
                                } else {
                                    special_workspace_colors
                                        .unwrap_or(workspace_colors)
                                        .get(m)
                                        .copied()
                                }
                            });

                            let w_id = w.id;
                            let w_name = w.name.clone();
                            let w_active = w.active;

                            Some(
                                button(
                                    container(
                                        if w_id < 0 { text(w_name) } else { text(w_id) }.size(10)
                                    )
                                    .align_x(alignment::Horizontal::Center)
                                    .align_y(alignment::Vertical::Center)
                                )
                                .style(workspace_button_style(empty, color))
                                .padding(if w_id < 0 {
                                    if w_active { [0, 16] } else { [0, 8] }
                                } else {
                                    [0, 0]
                                })
                                .on_press(if w_id > 0 {
                                    Message::ChangeWorkspace(w_id)
                                } else {
                                    Message::ToggleSpecialWorkspace(w_id)
                                })
                                .width(if w_id < 0 {
                                    Length::Shrink
                                } else if w_active {
                                    Length::Fixed(32.)
                                } else {
                                    Length::Fixed(16.)
                                })
                                .height(16)
                                .into()
                            )
                        } else {
                            None
                        }
                    })
                    .map(|elem: Element<'_, Message>| elem.map(M::from))
                    .collect::<Vec<Element<'_, M, _, _>>>()
            )
            .padding([2, 0])
            .spacing(4)
            .into(),
            None
        ))
    }

    // Background updates are delivered via the shared module event sender.
}

#[cfg(test)]
mod tests {
    use hydebar_proto::config::WorkspacesModuleConfig;

    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_from_port_snapshot() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let config = WorkspacesModuleConfig::default();

        let module = Workspaces::new(port_trait, &config);

        assert!(!module.items().is_empty());
    }

    #[test]
    fn change_workspace_dispatches_via_port() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let config = WorkspacesModuleConfig::default();

        let mut module = Workspaces::new(port_trait, &config);
        module.update(Message::ChangeWorkspace(2), &config);

        assert_eq!(port.workspace_calls(), 1);
    }
}
