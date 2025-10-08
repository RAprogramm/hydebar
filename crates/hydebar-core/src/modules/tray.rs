use std::{future::Future, pin::Pin, sync::Arc};

use iced::{
    Element, Length,
    widget::{Column, Row, button, horizontal_rule, row, text, toggler},
    window::Id,
};
use log::{debug, error, warn};
use tokio::{runtime::Handle, task::JoinHandle};

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, ModuleEventSender,
    components::icons::{Icons, icon},
    event_bus::ModuleEvent,
    services::{
        ReadOnlyService, ServiceEvent,
        tray::{
            TrayCommand, TrayService,
            dbus::{Layout, LayoutProps},
        },
    },
    style::ghost_button_style,
};

#[derive(Debug, Clone,)]
pub enum TrayMessage
{
    Event(Box<ServiceEvent<TrayService,>,>,),
    ToggleSubmenu(i32,),
    MenuSelected(String, i32,),
}

type ListenerSpawner =
    Arc<dyn Fn(ModuleEventSender<TrayMessage,>, Handle,) -> JoinHandle<(),> + Send + Sync,>;
type CommandFactory =
    Arc<dyn Fn(Option<&TrayService,>, TrayCommand,) -> Option<TrayCommandFuture,> + Send + Sync,>;
type TrayCommandFuture =
    Pin<Box<dyn Future<Output = ServiceEvent<TrayService,>,> + Send + 'static,>,>;

pub struct TrayModule
{
    pub service:      Option<TrayService,>,
    pub submenus:     Vec<i32,>,
    sender:           Option<ModuleEventSender<TrayMessage,>,>,
    runtime:          Option<Handle,>,
    listener_handles: Vec<JoinHandle<(),>,>,
    listener_spawner: ListenerSpawner,
    command_factory:  CommandFactory,
}

impl std::fmt::Debug for TrayModule
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result
    {
        f.debug_struct("TrayModule",)
            .field("service", &self.service,)
            .field("submenus", &self.submenus,)
            .field("sender", &self.sender,)
            .field("runtime", &self.runtime,)
            .field("listener_handles", &format!("<{} handles>", self.listener_handles.len()),)
            .field("listener_spawner", &"<function>",)
            .field("command_factory", &"<function>",)
            .finish()
    }
}

impl TrayModule
{
    fn abort_listener_handles(&mut self,)
    {
        for handle in self.listener_handles.drain(..,) {
            handle.abort();
        }
    }

    fn spawn_listener(&mut self,)
    {
        let Some(sender,) = self.sender.clone() else {
            warn!("tray module missing event sender; skipping listener spawn");
            return;
        };
        let Some(runtime,) = self.runtime.clone() else {
            warn!("tray module missing runtime handle; skipping listener spawn");
            return;
        };

        let spawner = Arc::clone(&self.listener_spawner,);
        self.listener_handles.push(spawner(sender, runtime,),);
    }

    fn dispatch_command(&self, command_future: TrayCommandFuture,)
    {
        let Some(runtime,) = self.runtime.clone() else {
            warn!("tray module missing runtime handle; skipping command dispatch");
            return;
        };
        let Some(sender,) = self.sender.clone() else {
            warn!("tray module missing event sender; skipping command dispatch");
            return;
        };

        runtime.spawn(async move {
            let event = command_future.await;
            if let Err(err,) = sender.try_send(TrayMessage::Event(Box::new(event,),),) {
                error!("failed to publish tray command result: {err}");
            }
        },);
    }

    pub fn update(&mut self, message: TrayMessage,)
    {
        match message {
            TrayMessage::Event(event,) => match *event {
                ServiceEvent::Init(service,) => {
                    self.service = Some(service,);
                }
                ServiceEvent::Update(data,) => {
                    if let Some(service,) = self.service.as_mut() {
                        service.update(data,);
                    }
                }
                ServiceEvent::Error(_,) => {
                    error!("Tray service error occurred");
                }
            },
            TrayMessage::ToggleSubmenu(index,) => {
                if self.submenus.contains(&index,) {
                    self.submenus.retain(|i| i != &index,);
                } else {
                    self.submenus.push(index,);
                }
            }
            TrayMessage::MenuSelected(name, id,) => {
                debug!("Tray menu click: {id}");

                if let Some(command,) = (self.command_factory)(
                    self.service.as_ref(),
                    TrayCommand::MenuSelected(name, id,),
                ) {
                    self.dispatch_command(command,);
                }
            }
        }
    }

    pub fn menu_view(&self, name: &'_ str, opacity: f32,) -> Element<'_, TrayMessage,>
    {
        match self
            .service
            .as_ref()
            .and_then(|service| service.data.iter().find(|item| item.name == name,),)
        {
            Some(item,) => Column::with_children(
                item.menu.2.iter().map(|menu| self.menu_voice(name, menu, opacity,),),
            )
            .spacing(8,)
            .into(),
            _ => Row::new().into(),
        }
    }

    fn menu_voice(&self, name: &str, layout: &Layout, opacity: f32,) -> Element<'_, TrayMessage,>
    {
        match &layout.1 {
            LayoutProps {
                label: Some(label,),
                toggle_type: Some(toggle_type,),
                toggle_state: Some(state,),
                ..
            } if toggle_type == "checkmark" => toggler(*state > 0,)
                .label(label.replace("_", "",).to_owned(),)
                .on_toggle({
                    let name = name.to_owned();
                    let id = layout.0;

                    move |_| TrayMessage::MenuSelected(name.to_owned(), id,)
                },)
                .width(Length::Fill,)
                .into(),
            LayoutProps {
                children_display: Some(display,),
                label: Some(label,),
                ..
            } if display == "submenu" => {
                let is_open = self.submenus.contains(&layout.0,);
                Column::new()
                    .push(
                        button(row!(
                            text(label.replace("_", "").to_owned()).width(Length::Fill),
                            icon(if is_open { Icons::MenuOpen } else { Icons::MenuClosed })
                        ),)
                        .style(ghost_button_style(opacity,),)
                        .padding([8, 8,],)
                        .on_press(TrayMessage::ToggleSubmenu(layout.0,),)
                        .width(Length::Fill,),
                    )
                    .push_maybe(if is_open {
                        Some(
                            Column::with_children(
                                layout
                                    .2
                                    .iter()
                                    .map(|menu| self.menu_voice(name, menu, opacity,),)
                                    .collect::<Vec<_,>>(),
                            )
                            .padding([0, 0, 0, 16,],)
                            .spacing(4,),
                        )
                    } else {
                        None
                    },)
                    .into()
            }
            LayoutProps {
                label: Some(label,), ..
            } => button(text(label.replace("_", "",),),)
                .style(ghost_button_style(opacity,),)
                .on_press(TrayMessage::MenuSelected(name.to_owned(), layout.0,),)
                .width(Length::Fill,)
                .padding([8, 8,],)
                .into(),
            LayoutProps {
                type_: Some(t,), ..
            } if t == "separator" => horizontal_rule(1,).into(),
            _ => Row::new().into(),
        }
    }
}

impl<M,> Module<M,> for TrayModule
where
    M: 'static + Clone,
{
    type ViewData<'a,> = (Id, f32,);
    type RegistrationData<'a,> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_,>,
    ) -> Result<(), ModuleError,>
    {
        self.abort_listener_handles();
        self.sender = Some(ctx.module_sender(ModuleEvent::Tray,),);
        self.runtime = Some(ctx.runtime_handle().clone(),);
        self.spawn_listener();

        Ok((),)
    }

    fn view(
        &self,
        (_id, _opacity,): Self::ViewData<'_,>,
    ) -> Option<(Element<'static, M,>, Option<OnModulePress<M,>,>,),>
    {
        // TODO: Tray view needs special handling for position_button messages
        // This requires GUI layer integration as buttons need to construct messages
        // with ButtonUIRef which can't be done generically in core.
        // For now, disabled to allow compilation.
        None
    }

    fn subscription(&self,) -> Option<iced::Subscription<M,>,>
    {
        None
    }
}

impl Default for TrayModule
{
    fn default() -> Self
    {
        Self {
            service:          None,
            submenus:         Vec::new(),
            sender:           None,
            runtime:          None,
            listener_handles: Vec::new(),
            listener_spawner: default_listener_spawner(),
            command_factory:  default_command_factory(),
        }
    }
}

impl Drop for TrayModule
{
    fn drop(&mut self,)
    {
        self.abort_listener_handles();
    }
}

fn default_listener_spawner() -> ListenerSpawner
{
    Arc::new(|sender, runtime| {
        runtime.spawn(async move {
            TrayService::start_listening(|event| {
                let sender = sender.clone();
                async move {
                    if let Err(err,) = sender.try_send(TrayMessage::Event(Box::new(event,),),) {
                        error!("failed to publish tray service event: {err}");
                    }
                }
            },)
            .await;
        },)
    },)
}

fn default_command_factory() -> CommandFactory
{
    Arc::new(|service, command| service.and_then(|svc| svc.prepare_command(command,),),)
}

#[cfg(test)]
impl TrayModule
{
    fn with_factories(listener_spawner: ListenerSpawner, command_factory: CommandFactory,)
    -> Self
    {
        Self {
            service: None,
            submenus: Vec::new(),
            sender: None,
            runtime: None,
            listener_handles: Vec::new(),
            listener_spawner,
            command_factory,
        }
    }
}

#[cfg(test)]
mod tests
{
    use std::{
        future::pending,
        num::NonZeroUsize,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use tokio::{runtime::Handle, task::yield_now, time::timeout};

    use super::{
        CommandFactory, ListenerSpawner, TrayMessage, TrayModule, default_command_factory,
        default_listener_spawner,
    };
    use crate::{
        ModuleContext,
        event_bus::{BusEvent, EventBus, ModuleEvent},
        modules::Module,
        services::{
            ServiceEvent,
            tray::{TrayCommand, TrayEvent},
        },
    };

    #[test]
    #[ignore = "Timing-sensitive test - needs rework"]
    fn aborts_existing_listener_on_reregistration()
    {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1,)
            .enable_all()
            .build()
            .expect("runtime",);
        let bus = EventBus::new(NonZeroUsize::new(4,).expect("capacity",),);
        let context = ModuleContext::new(bus.sender(), runtime.handle().clone(),);

        let (tx, mut rx,) = tokio::sync::oneshot::channel();
        let cancellation = Arc::new(Mutex::new(Some(tx,),),);
        let cancellation_spawner = Arc::clone(&cancellation,);

        let listener_spawner: ListenerSpawner = Arc::new(move |_, handle: Handle| {
            let cancellation = Arc::clone(&cancellation_spawner,);

            handle.spawn(async move {
                struct CancellationProbe
                {
                    signal: Arc<Mutex<Option<tokio::sync::oneshot::Sender<(),>,>,>,>,
                }

                impl Drop for CancellationProbe
                {
                    fn drop(&mut self,)
                    {
                        if let Some(sender,) =
                            self.signal.lock().expect("cancellation lock",).take()
                        {
                            let _ = sender.send((),);
                        }
                    }
                }

                let _probe = CancellationProbe {
                    signal: cancellation,
                };
                pending::<(),>().await;
            },)
        },);

        let mut module = TrayModule::with_factories(listener_spawner, default_command_factory(),);

        <TrayModule as Module<(),>>::register(&mut module, &context, (),)
            .expect("first registration",);
        <TrayModule as Module<(),>>::register(&mut module, &context, (),)
            .expect("second registration",);

        runtime
            .block_on(async {
                timeout(Duration::from_secs(2,), async {
                    loop {
                        if rx.try_recv().is_ok() {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                },)
                .await
            },)
            .expect("listener aborted",);
    }

    #[test]
    fn publishes_command_results_via_event_bus()
    {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1,)
            .enable_all()
            .build()
            .expect("runtime",);
        let bus = EventBus::new(NonZeroUsize::new(4,).expect("capacity",),);
        let sender = bus.sender();
        let mut receiver = bus.receiver();
        let context = ModuleContext::new(sender, runtime.handle().clone(),);

        let listener_spawner: ListenerSpawner =
            Arc::new(|_, handle: Handle| handle.spawn(async {},),);
        let command_factory: CommandFactory = Arc::new(|_, command| match command {
            TrayCommand::MenuSelected(name, _,) => {
                let layout = super::Layout(
                    1,
                    super::LayoutProps {
                        children_display: None,
                        label:            Some("Updated".into(),),
                        type_:            None,
                        toggle_type:      None,
                        toggle_state:     None,
                    },
                    Vec::new(),
                );

                Some(Box::pin(async move {
                    ServiceEvent::Update(TrayEvent::MenuLayoutChanged(name, layout,),)
                },),)
            }
        },);

        let mut module = TrayModule::with_factories(listener_spawner, command_factory,);
        <TrayModule as Module<(),>>::register(&mut module, &context, (),).expect("registration",);

        // update() returns (), just verify it doesn't panic
        module.update(TrayMessage::MenuSelected("tray".into(), 42,),);

        let event = runtime
            .block_on(async {
                timeout(Duration::from_millis(100,), async {
                    loop {
                        if let Some(event,) = receiver.try_recv().expect("bus read",) {
                            break event;
                        }
                        yield_now().await;
                    }
                },)
                .await
            },)
            .expect("event published",);

        match event {
            BusEvent::Module(ModuleEvent::Tray(TrayMessage::Event(event,),),) => match *event {
                ServiceEvent::Update(TrayEvent::MenuLayoutChanged(ref name, _,),) => {
                    assert_eq!(name, "tray");
                }
                other => panic!("unexpected tray event: {other:?}"),
            },
            other => panic!("unexpected bus event: {other:?}"),
        }
    }

    #[test]
    fn retains_default_listener_spawner()
    {
        let _module =
            TrayModule::with_factories(default_listener_spawner(), default_command_factory(),);
    }
}
