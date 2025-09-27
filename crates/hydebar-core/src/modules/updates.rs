use crate::{
    ModuleContext, ModuleEventSender,
    app::{self},
    components::icons::{Icons, icon},
    config::UpdatesModuleConfig,
    event_bus::ModuleEvent,
    menu::MenuType,
    outputs::Outputs,
    style::ghost_button_style,
};
use iced::{
    Alignment, Element, Length, Padding, Task,
    alignment::Horizontal,
    widget::{Column, button, column, container, horizontal_rule, row, scrollable, text},
    window::Id,
};
use log::{error, warn};
use serde::Deserialize;
use std::{convert, process::Stdio, sync::Arc, time::Duration};
use tokio::{process, runtime::Handle, task::JoinHandle, time::sleep};

use super::{Module, ModuleError, OnModulePress};

#[derive(Deserialize, Debug, Clone)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
}

async fn check_update_now(check_cmd: &str) -> Vec<Update> {
    let check_update_cmd = process::Command::new("bash")
        .arg("-c")
        .arg(check_cmd)
        .stdout(Stdio::piped())
        .output()
        .await;

    match check_update_cmd {
        Ok(check_update_cmd) => {
            let cmd_output = String::from_utf8_lossy(&check_update_cmd.stdout);
            let mut new_updates: Vec<Update> = Vec::new();
            for update in cmd_output.split('\n') {
                if update.is_empty() {
                    continue;
                }

                let data = update.split(' ').collect::<Vec<&str>>();
                if data.len() < 4 {
                    continue;
                }
                new_updates.push(Update {
                    package: data[0].to_string(),
                    from: data[1].to_string(),
                    to: data[3].to_string(),
                });
            }

            new_updates
        }
        Err(e) => {
            error!("Error: {e:?}");
            vec![]
        }
    }
}

async fn update(update_cmd: &str) {
    let _ = process::Command::new("bash")
        .arg("-c")
        .arg(update_cmd)
        .output()
        .await;
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdatesCheckCompleted(Vec<Update>),
    UpdateFinished,
    ToggleUpdatesList,
    CheckNow,
    Update(Id),
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
enum State {
    #[default]
    Checking,
    Ready,
}

#[derive(Debug, Default, Clone)]
pub struct Updates {
    state: State,
    pub updates: Vec<Update>,
    pub is_updates_list_open: bool,
    registration: Option<UpdatesRegistration>,
    sender: Option<ModuleEventSender<Message>>,
    runtime: Option<Handle>,
    tasks: Vec<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
struct UpdatesRegistration {
    check_command: Arc<str>,
    update_command: Arc<str>,
}

impl Updates {
    pub fn update(
        &mut self,
        message: Message,
        _config: &UpdatesModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config,
    ) -> Task<crate::app::Message> {
        match message {
            Message::UpdatesCheckCompleted(updates) => {
                self.updates = updates;
                self.state = State::Ready;

                Task::none()
            }
            Message::UpdateFinished => {
                self.updates.clear();
                self.state = State::Ready;

                Task::none()
            }
            Message::ToggleUpdatesList => {
                self.is_updates_list_open = !self.is_updates_list_open;

                Task::none()
            }
            Message::CheckNow => {
                self.state = State::Checking;

                match (
                    self.runtime.clone(),
                    self.sender.clone(),
                    self.registration
                        .as_ref()
                        .map(|registration| Arc::clone(&registration.check_command)),
                ) {
                    (Some(runtime), Some(sender), Some(check_command)) => {
                        runtime.spawn(async move {
                            let updates = check_update_now(check_command.as_ref()).await;

                            if let Err(err) =
                                sender.try_send(Message::UpdatesCheckCompleted(updates))
                            {
                                error!("failed to publish updates check result: {err}");
                            }
                        });
                    }
                    _ => {
                        warn!("updates module is not fully initialised; skipping manual check");
                        self.state = State::Ready;
                    }
                }

                Task::none()
            }
            Message::Update(id) => {
                if let (Some(runtime), Some(sender), Some(registration)) = (
                    self.runtime.clone(),
                    self.sender.clone(),
                    self.registration.as_ref(),
                ) {
                    let update_command = Arc::clone(&registration.update_command);

                    runtime.spawn(async move {
                        update(update_command.as_ref()).await;

                        if let Err(err) = sender.try_send(Message::UpdateFinished) {
                            error!("failed to publish update completion: {err}");
                        }
                    });
                } else {
                    warn!("updates module is not fully initialised; skipping update command");
                }

                outputs.close_menu_if(id, MenuType::Updates, main_config)
            }
        }
    }

    pub fn menu_view(&self, id: Id, opacity: f32) -> Element<Message> {
        column!(
            if self.updates.is_empty() {
                convert::Into::<Element<'_, _, _>>::into(
                    container(text("Up to date ;)")).padding([8, 8]),
                )
            } else {
                let mut elements = column!(
                    button(row!(
                        text(format!("{} Updates available", self.updates.len()))
                            .width(Length::Fill),
                        icon(if self.is_updates_list_open {
                            Icons::MenuClosed
                        } else {
                            Icons::MenuOpen
                        })
                    ))
                    .style(ghost_button_style(opacity))
                    .padding([8, 8])
                    .on_press(Message::ToggleUpdatesList)
                    .width(Length::Fill),
                );

                if self.is_updates_list_open {
                    elements = elements.push(
                        container(scrollable(
                            Column::with_children(
                                self.updates
                                    .iter()
                                    .map(|update| {
                                        column!(
                                            text(update.package.clone())
                                                .size(10)
                                                .width(Length::Fill),
                                            text(format!(
                                                "{} -> {}",
                                                {
                                                    let mut res = update.from.clone();
                                                    res.truncate(18);

                                                    res
                                                },
                                                {
                                                    let mut res = update.to.clone();
                                                    res.truncate(18);

                                                    res
                                                },
                                            ))
                                            .width(Length::Fill)
                                            .align_x(Horizontal::Right)
                                            .size(10)
                                        )
                                        .into()
                                    })
                                    .collect::<Vec<Element<'_, _, _>>>(),
                            )
                            .padding(Padding::ZERO.right(16))
                            .spacing(4),
                        ))
                        .padding([8, 0])
                        .max_height(300),
                    );
                }
                elements.into()
            },
            horizontal_rule(1),
            button("Update")
                .style(ghost_button_style(opacity))
                .padding([8, 8])
                .on_press(Message::Update(id))
                .width(Length::Fill),
            button({
                let mut content = row!(text("Check now").width(Length::Fill),);

                if self.state == State::Checking {
                    content = content.push(icon(Icons::Refresh));
                }

                content
            })
            .style(ghost_button_style(opacity))
            .padding([8, 8])
            .on_press(Message::CheckNow)
            .width(Length::Fill),
        )
        .spacing(4)
        .into()
    }
}

impl Module for Updates {
    type ViewData<'a> = &'a Option<UpdatesModuleConfig>;
    type RegistrationData<'a> = Option<&'a UpdatesModuleConfig>;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::Updates));
        self.runtime = Some(ctx.runtime_handle().clone());

        for task in self.tasks.drain(..) {
            task.abort();
        }

        self.registration = config.map(|definition| UpdatesRegistration {
            check_command: Arc::from(definition.check_cmd.as_str()),
            update_command: Arc::from(definition.update_cmd.as_str()),
        });

        if let (Some(registration), Some(sender)) =
            (self.registration.as_ref(), self.sender.clone())
        {
            let check_command = Arc::clone(&registration.check_command);

            let task = ctx.runtime_handle().spawn(async move {
                loop {
                    let updates = check_update_now(check_command.as_ref()).await;

                    if let Err(err) = sender.try_send(Message::UpdatesCheckCompleted(updates)) {
                        error!("failed to publish scheduled updates check: {err}");
                    }

                    sleep(Duration::from_secs(3600)).await;
                }
            });

            self.tasks.push(task);
        }

        Ok(())
    }

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if config.is_some() {
            let mut content = row!(container(icon(match self.state {
                State::Checking => Icons::Refresh,
                State::Ready if self.updates.is_empty() => Icons::NoUpdatesAvailable,
                _ => Icons::UpdatesAvailable,
            })))
            .align_y(Alignment::Center)
            .spacing(4);

            if !self.updates.is_empty() {
                content = content.push(text(self.updates.len()));
            }

            Some((
                content.into(),
                Some(OnModulePress::ToggleMenu(MenuType::Updates)),
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        num::NonZeroUsize,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use futures::future;
    use tokio::runtime::Runtime;

    use crate::{
        config::Config,
        event_bus::{BusEvent, EventBus, ModuleEvent},
        outputs::Outputs,
    };

    use super::*;

    #[test]
    fn register_spawns_hourly_task() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut updates = Updates::default();
        let config = UpdatesModuleConfig {
            check_cmd: ":".into(),
            update_cmd: ":".into(),
        };

        updates
            .register(&ctx, Some(&config))
            .expect("register should succeed");

        assert!(updates.sender.is_some());
        assert_eq!(updates.tasks.len(), 1);

        for task in updates.tasks.drain(..) {
            task.abort();
        }
    }

    #[test]
    fn register_aborts_existing_tasks() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut updates = Updates::default();

        let cancelled = Arc::new(AtomicBool::new(false));
        let guard_flag = Arc::clone(&cancelled);

        updates.tasks.push(runtime.spawn(async move {
            struct CancelGuard(Arc<AtomicBool>);

            impl Drop for CancelGuard {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::SeqCst);
                }
            }

            let _guard = CancelGuard(guard_flag);

            future::pending::<()>().await;
        }));

        let config = UpdatesModuleConfig {
            check_cmd: ":".into(),
            update_cmd: ":".into(),
        };

        updates
            .register(&ctx, Some(&config))
            .expect("register should succeed");

        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(1), async {
                loop {
                    if cancelled.load(Ordering::SeqCst) {
                        break;
                    }

                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("task should be aborted promptly");
        });

        for task in updates.tasks.drain(..) {
            task.abort();
        }
    }

    #[test]
    fn check_now_enqueues_result_on_event_bus() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let mut receiver = bus.receiver();
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut updates = Updates::default();
        let config = UpdatesModuleConfig {
            check_cmd: "printf 'pkg 1 -> 2\\n'".into(),
            update_cmd: ":".into(),
        };

        updates
            .register(&ctx, Some(&config))
            .expect("register should succeed");

        // Drain the initial scheduled check, if it has already emitted a result.
        while matches!(
            receiver.try_recv().expect("drain"),
            Some(BusEvent::Module(ModuleEvent::Updates(
                Message::UpdatesCheckCompleted(_)
            )))
        ) {}

        let mut outputs = dummy_outputs();
        let main_config = Config::default();

        updates.update(Message::CheckNow, &config, &mut outputs, &main_config);

        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    if let Some(BusEvent::Module(ModuleEvent::Updates(message))) =
                        receiver.try_recv().expect("recv")
                    {
                        if let Message::UpdatesCheckCompleted(updates) = message {
                            assert_eq!(updates.len(), 1);
                            break;
                        }
                    }

                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("check-now result should arrive");
        });

        for task in updates.tasks.drain(..) {
            task.abort();
        }
    }

    fn dummy_outputs() -> Outputs {
        let config = Config::default();
        Outputs::new::<crate::app::Message>(config.appearance.style, config.position, &config).0
    }
}
