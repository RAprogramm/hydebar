use super::{Module, OnModulePress};
use crate::{
    app,
    config::{AppearanceColor, WorkspaceVisibilityMode, WorkspacesModuleConfig},
    outputs::Outputs,
    style::workspace_button_style,
};

use hyprland::{
    data::{Monitors as HlMonitors, Workspace as HlWorkspace, Workspaces as HlWorkspaces},
    dispatch::{Dispatch, DispatchType, MonitorIdentifier, WorkspaceIdentifierWithSpecial},
    event_listener::AsyncEventListener,
    shared::{HyprData, HyprDataActive, HyprDataVec},
};

use iced::{
    Element, Length, Subscription, alignment,
    stream::channel,
    widget::{Row, button, container, text},
    window::Id,
};

use itertools::Itertools;
use log::{debug, error};
use std::{
    any::TypeId,
    convert::TryFrom,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub id: i32,
    pub name: String,
    pub monitor_id: Option<usize>, // index for color lookup; may be None
    pub monitor: String,           // monitor name for fallback
    pub active: bool,
    pub windows: u16,
}

fn get_workspaces(config: &WorkspacesModuleConfig) -> Vec<Workspace> {
    let active = HlWorkspace::get_active().ok();
    let monitors = HlMonitors::get().map(|m| m.to_vec()).unwrap_or_default();
    let workspaces = HlWorkspaces::get().map(|w| w.to_vec()).unwrap_or_default();

    // Deduplicate by ID to avoid duplicates from Hyprland.
    let workspaces: Vec<_> = workspaces.into_iter().unique_by(|w| w.id).collect();

    // Preallocate result vector.
    let mut result: Vec<Workspace> = Vec::with_capacity(workspaces.len());

    let (special, normal): (Vec<_>, Vec<_>) = workspaces.into_iter().partition(|w| w.id < 0);

    // Map special workspaces.
    for w in special.iter() {
        result.push(Workspace {
            id: w.id,
            name: w
                .name
                .split(':')
                .last()
                .map_or_else(|| String::new(), ToOwned::to_owned),
            // Option<i128> -> Option<usize> with bounds check.
            monitor_id: w.monitor_id.and_then(|mid| usize::try_from(mid).ok()),
            monitor: w.monitor.clone(),
            active: monitors.iter().any(|m| m.special_workspace.id == w.id),
            windows: w.windows,
        });
    }

    // Map normal workspaces.
    for w in normal.iter() {
        result.push(Workspace {
            id: w.id,
            name: w.name.clone(),
            monitor_id: w.monitor_id.and_then(|mid| usize::try_from(mid).ok()),
            monitor: w.monitor.clone(),
            active: Some(w.id) == active.as_ref().map(|a| a.id),
            windows: w.windows,
        });
    }

    if !config.enable_workspace_filling || normal.is_empty() {
        result.sort_by_key(|w| w.id);
        return result;
    }

    // Synthesize "missing" workspaces [1..=max_id] for filling UI.
    let existing_ids = normal.iter().map(|w| w.id).collect::<Vec<_>>();
    let mut max_id = *existing_ids.iter().max().unwrap_or(&0);
    if let Some(max_workspaces) = config.max_workspaces {
        if max_workspaces > max_id as u32 {
            max_id = max_workspaces as i32;
        }
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
            windows: 0,
        });
    }

    result.sort_by_key(|w| w.id);
    result
}

pub struct Workspaces {
    workspaces: Vec<Workspace>,
}

impl Workspaces {
    pub fn new(config: &WorkspacesModuleConfig) -> Self {
        Self {
            workspaces: get_workspaces(config),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    WorkspacesChanged,
    ChangeWorkspace(i32),
    ToggleSpecialWorkspace(i32),
}

impl Workspaces {
    pub fn update(&mut self, message: Message, config: &WorkspacesModuleConfig) {
        match message {
            Message::WorkspacesChanged => {
                self.workspaces = get_workspaces(config);
            }
            Message::ChangeWorkspace(id) => {
                if id > 0 {
                    let already_active = self.workspaces.iter().any(|w| w.active && w.id == id);
                    if !already_active {
                        debug!("changing workspace to: {id}");
                        let res = Dispatch::call(DispatchType::Workspace(
                            WorkspaceIdentifierWithSpecial::Id(id),
                        ));
                        if let Err(e) = res {
                            error!("failed to dispatch workspace change: {e:?}");
                        }
                    }
                }
            }
            Message::ToggleSpecialWorkspace(id) => {
                if let Some(special) = self.workspaces.iter().find(|w| w.id == id && w.id < 0) {
                    debug!("toggle special workspace: {id}");

                    // Prefer focusing by monitor index if present; otherwise, fall back to monitor name.
                    let monitor_ident = match special.monitor_id {
                        Some(idx) => MonitorIdentifier::Id(idx as i128),
                        None => MonitorIdentifier::Name(&special.monitor.clone()),
                    };

                    let res =
                        Dispatch::call(DispatchType::FocusMonitor(monitor_ident)).and_then(|_| {
                            Dispatch::call(DispatchType::ToggleSpecialWorkspace(Some(
                                special.name.clone(),
                            )))
                        });

                    if let Err(e) = res {
                        error!("failed to dispatch special workspace toggle: {e:?}");
                    }
                }
            }
        }
    }
}

impl Module for Workspaces {
    type ViewData<'a> = (
        &'a Outputs,
        Id,
        &'a WorkspacesModuleConfig,
        &'a [AppearanceColor],
        Option<&'a [AppearanceColor]>,
    );
    type SubscriptionData<'a> = &'a WorkspacesModuleConfig;

    fn view(
        &'_ self,
        (outputs, id, config, workspace_colors, special_workspace_colors): Self::ViewData<'_>,
    ) -> Option<(Element<'_, app::Message>, Option<OnModulePress>)> {
        let monitor_name = outputs.get_monitor_name(id);

        Some((
            Into::<Element<Message>>::into(
                Row::with_children(
                    self.workspaces
                        .iter()
                        .filter_map(|w| {
                            if config.visibility_mode == WorkspaceVisibilityMode::All
                                || w.monitor == monitor_name.unwrap_or_else(|| &w.monitor)
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

                                Some(
                                    button(
                                        container(
                                            if w.id < 0 {
                                                text(w.name.as_str())
                                            } else {
                                                text(w.id)
                                            }
                                            .size(10),
                                        )
                                        .align_x(alignment::Horizontal::Center)
                                        .align_y(alignment::Vertical::Center),
                                    )
                                    .style(workspace_button_style(empty, color))
                                    .padding(if w.id < 0 {
                                        if w.active { [0, 16] } else { [0, 8] }
                                    } else {
                                        [0, 0]
                                    })
                                    .on_press(if w.id > 0 {
                                        Message::ChangeWorkspace(w.id)
                                    } else {
                                        Message::ToggleSpecialWorkspace(w.id)
                                    })
                                    .width(if w.id < 0 {
                                        Length::Shrink
                                    } else if w.active {
                                        Length::Fixed(32.)
                                    } else {
                                        Length::Fixed(16.)
                                    })
                                    .height(16)
                                    .into(),
                                )
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<Element<'_, _, _>>>(),
                )
                .padding([2, 0])
                .spacing(4),
            )
            .map(app::Message::Workspaces),
            None,
        ))
    }

    fn subscription(
        &self,
        config: Self::SubscriptionData<'_>,
    ) -> Option<Subscription<app::Message>> {
        let id = TypeId::of::<Self>();
        let enable_workspace_filling = config.enable_workspace_filling;

        Some(
            Subscription::run_with_id(
                format!("{id:?}-{enable_workspace_filling}"),
                channel(10, async move |output| {
                    let output = Arc::new(RwLock::new(output));

                    // We keep the listener in a loop and restart on error.
                    loop {
                        let mut event_listener = AsyncEventListener::new();

                        // Helper to DRY send with logging.
                        let send = |output: &Arc<RwLock<_>>| {
                            let out: Arc<RwLock<iced::futures::channel::mpsc::Sender<Message>>> =
                                output.clone();
                            Box::pin(async move {
                                if let Ok(mut guard) = out.write() {
                                    if let Err(e) = guard.try_send(Message::WorkspacesChanged) {
                                        error!("failed to enqueue WorkspacesChanged: {e:?}");
                                    }
                                } else {
                                    error!("failed to acquire output lock for WorkspacesChanged");
                                }
                            })
                        };

                        event_listener.add_workspace_added_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("workspace added: {e:?}");
                                send(&output)
                            }
                        });

                        event_listener.add_workspace_changed_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("workspace changed: {e:?}");
                                send(&output)
                            }
                        });

                        event_listener.add_workspace_deleted_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("workspace deleted: {e:?}");
                                send(&output)
                            }
                        });

                        event_listener.add_workspace_moved_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("workspace moved: {e:?}");
                                send(&output)
                            }
                        });

                        event_listener.add_changed_special_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("special workspace changed: {e:?}");
                                send(&output)
                            }
                        });

                        event_listener.add_special_removed_handler({
                            let output = output.clone();
                            move |e| {
                                debug!("special workspace removed: {e:?}");
                                send(&output)
                            }
                        });

                        event_listener.add_window_closed_handler({
                            let output = output.clone();
                            move |_| send(&output)
                        });

                        event_listener.add_window_opened_handler({
                            let output = output.clone();
                            move |_| send(&output)
                        });

                        event_listener.add_window_moved_handler({
                            let output = output.clone();
                            move |_| send(&output)
                        });

                        event_listener.add_active_monitor_changed_handler({
                            let output = output.clone();
                            move |_| send(&output)
                        });

                        if let Err(e) = event_listener.start_listener_async().await {
                            error!("restarting workspaces listener due to error: {e:?}");
                        }
                    }
                }),
            )
            .map(app::Message::Workspaces),
        )
    }
}
