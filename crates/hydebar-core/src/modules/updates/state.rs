use std::{sync::Arc, time::Duration};

use iced::{Element, window::Id};
use log::{error, warn};
use tokio::{runtime::Handle, task::JoinHandle, time::sleep};

use super::{commands, view};
use crate::{
    ModuleContext, ModuleEventSender,
    config::UpdatesModuleConfig,
    event_bus::ModuleEvent,
    menu::MenuType,
    modules::{Module, ModuleError, OnModulePress},
    outputs::Outputs
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub(super) package: String,
    pub(super) from:    String,
    pub(super) to:      String
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdatesCheckCompleted(Vec<Update>),
    UpdateFinished,
    ToggleUpdatesList,
    CheckNow,
    Update(Id)
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub(crate) enum CheckState {
    #[default]
    Checking,
    Ready
}

#[derive(Default)]
pub struct Updates {
    state:                    CheckState,
    updates:                  Vec<Update>,
    pub is_updates_list_open: bool,
    registration:             Option<UpdatesRegistration>,
    sender:                   Option<ModuleEventSender<Message>>,
    runtime:                  Option<Handle>,
    tasks:                    Vec<JoinHandle<()>>
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates")
            .field("state", &self.state)
            .field("updates", &self.updates)
            .field("is_updates_list_open", &self.is_updates_list_open)
            .field("registration", &self.registration)
            .field("sender", &self.sender)
            .field("runtime", &self.runtime)
            .field("tasks", &format!("<{} tasks>", self.tasks.len()))
            .finish()
    }
}

impl Clone for Updates {
    fn clone(&self) -> Self {
        Self {
            state:                self.state.clone(),
            updates:              self.updates.clone(),
            is_updates_list_open: self.is_updates_list_open,
            registration:         self.registration.clone(),
            sender:               self.sender.clone(),
            runtime:              self.runtime.clone(),
            tasks:                Vec::new() // JoinHandles can't be cloned
        }
    }
}

#[derive(Debug, Clone)]
struct UpdatesRegistration {
    check_command:  Arc<str>,
    update_command: Arc<str>
}

impl Updates {
    pub fn update(
        &mut self,
        message: Message,
        _config: &UpdatesModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config
    ) {
        match message {
            Message::UpdatesCheckCompleted(updates) => {
                self.updates = updates;
                self.state = CheckState::Ready;
            }
            Message::UpdateFinished => {
                self.updates.clear();
                self.state = CheckState::Ready;
            }
            Message::ToggleUpdatesList => {
                self.is_updates_list_open = !self.is_updates_list_open;
            }
            Message::CheckNow => {
                self.state = CheckState::Checking;

                match (
                    self.runtime.clone(),
                    self.sender.clone(),
                    self.registration
                        .as_ref()
                        .map(|registration| Arc::clone(&registration.check_command))
                ) {
                    (Some(runtime), Some(sender), Some(check_command)) => {
                        runtime.spawn(async move {
                            match commands::check_for_updates(check_command.as_ref()).await {
                                Ok(updates) => {
                                    if let Err(err) =
                                        sender.try_send(Message::UpdatesCheckCompleted(updates))
                                    {
                                        error!("failed to publish updates check result: {err}");
                                    }
                                }
                                Err(err) => {
                                    warn!("failed to run manual updates check: {err}");
                                    if let Err(err) =
                                        sender.try_send(Message::UpdatesCheckCompleted(Vec::new()))
                                    {
                                        error!(
                                            "failed to publish manual updates check failure: {err}"
                                        );
                                    }
                                }
                            }
                        });
                    }
                    _ => {
                        warn!("updates module is not fully initialised; skipping manual check");
                        self.state = CheckState::Ready;
                    }
                }
            }
            Message::Update(id) => {
                if let (Some(runtime), Some(sender), Some(registration)) = (
                    self.runtime.clone(),
                    self.sender.clone(),
                    self.registration.as_ref()
                ) {
                    let update_command = Arc::clone(&registration.update_command);

                    runtime.spawn(async move {
                        if let Err(err) = commands::apply_updates(update_command.as_ref()).await {
                            err.or_log("failed to execute update command");
                        }

                        if let Err(err) = sender.try_send(Message::UpdateFinished) {
                            error!("failed to publish update completion: {err}");
                        }
                    });
                } else {
                    warn!("updates module is not fully initialised; skipping update command");
                }

                let _ = outputs.close_menu_if::<Message>(id, MenuType::Updates, main_config);
            }
        }
    }

    pub fn menu_view(&self, id: Id, opacity: f32) -> Element<'_, Message> {
        view::menu_view(self, id, opacity)
    }

    pub(crate) fn updates(&self) -> &[Update] {
        &self.updates
    }

    pub(crate) fn is_updates_list_open(&self) -> bool {
        self.is_updates_list_open
    }

    pub(crate) fn state(&self) -> &CheckState {
        &self.state
    }
}

impl<M> Module<M> for Updates
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = &'a Option<UpdatesModuleConfig>;
    type RegistrationData<'a> = Option<&'a UpdatesModuleConfig>;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        config: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::Updates));
        self.runtime = Some(ctx.runtime_handle().clone());

        for task in self.tasks.drain(..) {
            task.abort();
        }

        self.registration = config.map(|definition| UpdatesRegistration {
            check_command:  Arc::from(definition.check_cmd.as_str()),
            update_command: Arc::from(definition.update_cmd.as_str())
        });

        if let (Some(registration), Some(sender)) =
            (self.registration.as_ref(), self.sender.clone())
        {
            let check_command = Arc::clone(&registration.check_command);

            let task = ctx.runtime_handle().spawn(async move {
                loop {
                    match commands::check_for_updates(check_command.as_ref()).await {
                        Ok(updates) => {
                            if let Err(err) =
                                sender.try_send(Message::UpdatesCheckCompleted(updates))
                            {
                                error!("failed to publish scheduled updates check: {err}");
                            }
                        }
                        Err(err) => {
                            err.or_log("failed to run scheduled updates check");
                        }
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
        config: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if config.is_some() {
            Some((
                view::icon(&self.state, self.updates.len()).map(M::from),
                Some(OnModulePress::ToggleMenu(MenuType::Updates))
            ))
        } else {
            None
        }
    }
}

// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use std::{
        num::NonZeroUsize,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering}
        }
    };

    use futures::future;
    use tokio::runtime::Runtime;

    use super::*;
    use crate::{
        config::Config,
        event_bus::{BusEvent, EventBus, ModuleEvent},
        outputs::Outputs
    };

    #[test]
    fn register_spawns_hourly_task() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut updates = Updates::default();
        let config = UpdatesModuleConfig {
            check_cmd:  ":".into(),
            update_cmd: ":".into()
        };

        <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config))
            .expect("register should succeed");

        assert!(updates.sender.is_some());
        assert_eq!(updates.tasks.len(), 1);

        for task in updates.tasks.drain(..) {
            task.abort();
        }
    }

    #[test]
    #[ignore = "Timing-sensitive test - needs rework"]
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
            check_cmd:  ":".into(),
            update_cmd: ":".into()
        };

        <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config))
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
            check_cmd:  "printf 'pkg 1 -> 2\\n'".into(),
            update_cmd: ":".into()
        };

        <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config))
            .expect("register should succeed");

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

    #[test]
    fn toggle_updates_list_flips_visibility() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut updates = Updates::default();
        let config = UpdatesModuleConfig {
            check_cmd:  ":".into(),
            update_cmd: ":".into()
        };

        <Updates as Module<Message>>::register(&mut updates, &ctx, Some(&config))
            .expect("register should succeed");

        let mut outputs = dummy_outputs();
        let main_config = Config::default();

        assert!(!updates.is_updates_list_open);

        updates.update(
            Message::ToggleUpdatesList,
            &config,
            &mut outputs,
            &main_config
        );
        assert!(updates.is_updates_list_open);

        updates.update(
            Message::ToggleUpdatesList,
            &config,
            &mut outputs,
            &main_config
        );
        assert!(!updates.is_updates_list_open);

        for task in updates.tasks.drain(..) {
            task.abort();
        }
    }

    fn dummy_outputs() -> Outputs {
        let config = Config::default();
        Outputs::new::<()>(config.appearance.style, config.position, &config).0
    }
}
