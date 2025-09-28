use std::{collections::HashMap, sync::Arc};

use hydebar_core::{
    config::{self, ConfigEvent, ConfigImpact},
    event_bus::{BusEvent, ModuleEvent},
    menu::MenuType,
    modules::{
        self, custom_module::Custom, settings::brightness::BrightnessMessage, tray::TrayMessage,
    },
    services::{ServiceEvent, brightness::BrightnessCommand, tray::TrayEvent},
    utils,
};
use hydebar_proto::config::{Config, ModuleName};
use iced::{
    Subscription, Task,
    event::{
        listen_with,
        wayland::{Event as WaylandEvent, OutputEvent},
    },
    keyboard, time,
};
use log::{debug, error, info, warn};

use crate::get_log_spec;

use super::{
    bus::{BusFlushOutcome, drain_bus},
    state::{App, Message},
};

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MicroTick => Task::perform(
                drain_bus(Arc::clone(&self.bus_receiver)),
                Message::BusFlushed,
            ),
            Message::BusFlushed(outcome) => {
                if outcome.had_error() {
                    error!("failed to drain event bus, keeping fast cadence");
                    self.micro_ticker.record_activity();
                }

                if outcome.is_empty() {
                    if !outcome.had_error() {
                        self.micro_ticker.record_idle();
                    }
                    Task::none()
                } else {
                    if !outcome.had_error() {
                        self.micro_ticker.record_activity();
                    }

                    let tasks: Vec<_> = outcome
                        .into_events()
                        .into_iter()
                        .filter_map(App::message_from_bus_event)
                        .map(|msg| self.update(msg))
                        .collect();

                    Task::batch(tasks)
                }
            }
            Message::None => Task::none(),
            Message::ConfigChanged(update) => {
                let hydebar_core::config::ConfigApplied { config, impact } = update;
                let mut new_config = *config;

                info!("New config applied: {new_config:?}");
                debug!("Config impact: {impact:?}");

                let mut tasks = Vec::new();

                let outputs_need_sync = impact.outputs_changed
                    || impact.position_changed
                    || self.config.appearance.style != new_config.appearance.style
                    || self.config.appearance.scale_factor != new_config.appearance.scale_factor;

                if outputs_need_sync {
                    warn!("Outputs or layout changed, syncing");
                    tasks.push(self.outputs.sync(
                        new_config.appearance.style,
                        &new_config.outputs,
                        new_config.position,
                        &new_config,
                    ));
                }

                if impact.custom_modules_changed {
                    self.update_custom_modules(&new_config, &impact);
                }

                self.config = new_config;

                self.register_modules();

                if impact.log_level_changed {
                    if let Err(err) = self
                        .logger
                        .set_new_spec(get_log_spec(&self.config.log_level))
                    {
                        error!("failed to update log level: {err}");
                    }
                }

                Task::batch(tasks)
            }
            Message::ConfigDegraded(degradation) => {
                warn!("Configuration degradation reported: {}", degradation.reason);
                Task::none()
            }
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                let mut cmd = vec![];
                match &menu_type {
                    MenuType::Updates => {
                        self.updates.is_updates_list_open = false;
                    }
                    MenuType::Tray(name) => {
                        if let Some(_tray) = self
                            .tray
                            .service
                            .as_ref()
                            .and_then(|t| t.iter().find(|t| &t.name == name))
                        {
                            self.tray.submenus.clear();
                        }
                    }
                    MenuType::Settings => {
                        self.settings.sub_menu = None;

                        if let Some(brightness) = self.settings.brightness.as_mut() {
                            cmd.push(brightness.command(BrightnessCommand::Refresh).map(|event| {
                                Message::Settings(crate::modules::settings::Message::Brightness(
                                    BrightnessMessage::Event(event),
                                ))
                            }));
                        }
                    }
                    _ => {}
                };
                cmd.push(
                    self.outputs
                        .toggle_menu(id, menu_type, button_ui_ref, &self.config),
                );

                Task::batch(cmd)
            }
            Message::CloseMenu(id) => self.outputs.close_menu(id, &self.config),
            Message::CloseAllMenus => {
                if self.outputs.menu_is_open() {
                    self.outputs.close_all_menus(&self.config)
                } else {
                    Task::none()
                }
            }
            Message::Updates(message) => {
                if let Some(updates_config) = self.config.updates.as_ref() {
                    self.updates
                        .update(message, updates_config, &mut self.outputs, &self.config)
                } else {
                    Task::none()
                }
            }
            Message::OpenLauncher => {
                if let Some(app_launcher_cmd) = self.config.app_launcher_cmd.as_ref() {
                    utils::launcher::execute_command(app_launcher_cmd.to_string());
                }
                Task::none()
            }
            Message::LaunchCommand(command) => {
                utils::launcher::execute_command(command);
                Task::none()
            }
            Message::CustomUpdate(name, message) => {
                match self.custom.get_mut(&name) {
                    Some(c) => c.update(message),
                    None => error!("Custom module '{name}' not found"),
                };
                Task::none()
            }
            Message::OpenClipboard => {
                if let Some(clipboard_cmd) = self.config.clipboard_cmd.as_ref() {
                    utils::launcher::execute_command(clipboard_cmd.to_string());
                }
                Task::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg, &self.config.workspaces);

                Task::none()
            }
            Message::WindowTitle(message) => {
                self.window_title.update(message, &self.config.window_title);
                Task::none()
            }
            Message::SystemInfo(message) => {
                self.system_info.update(message);
                Task::none()
            }
            Message::KeyboardLayout(message) => {
                self.keyboard_layout.update(message);
                Task::none()
            }
            Message::KeyboardSubmap(message) => {
                self.keyboard_submap.update(message);
                Task::none()
            }
            Message::Tray(msg) => {
                let close_tray = match &msg {
                    TrayMessage::Event(event) => {
                        if let ServiceEvent::Update(TrayEvent::Unregistered(name)) = event.as_ref()
                        {
                            self.outputs
                                .close_all_menu_if(MenuType::Tray(name.clone()), &self.config)
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none(),
                };

                Task::batch(vec![self.tray.update(msg), close_tray])
            }
            Message::Clock(message) => {
                self.clock.update(message);
                Task::none()
            }
            Message::Battery(message) => {
                self.battery.update(message);
                Task::none()
            }
            Message::Privacy(msg) => {
                self.privacy.update(msg);
                Task::none()
            }
            Message::Settings(message) => self.settings.update(
                message,
                &self.config.settings,
                &mut self.outputs,
                &self.config,
            ),
            Message::OutputEvent((event, wl_output)) => match event {
                OutputEvent::Created(info) => {
                    info!("Output created: {info:?}");
                    let name = info
                        .as_ref()
                        .and_then(|info| info.name.as_deref())
                        .unwrap_or("");

                    self.outputs.add(
                        self.config.appearance.style,
                        &self.config.outputs,
                        self.config.position,
                        name,
                        wl_output,
                        &self.config,
                    )
                }
                OutputEvent::Removed => {
                    info!("Output destroyed");
                    self.outputs.remove(
                        self.config.appearance.style,
                        self.config.position,
                        wl_output,
                        &self.config,
                    )
                }
                _ => Task::none(),
            },
            Message::MediaPlayer(msg) => self.media_player.update(msg),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let timer = time::every(self.micro_ticker.interval()).map(|_| Message::MicroTick);

        let mut subscriptions = vec![
            timer,
            config::subscription(&self.config_path, Arc::clone(&self.config_manager)).map(
                |event| match event {
                    ConfigEvent::Applied(config) => Message::ConfigChanged(config),
                    ConfigEvent::Degraded(degradation) => Message::ConfigDegraded(degradation),
                },
            ),
            listen_with(move |evt, _, _| match evt {
                iced::Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
                    WaylandEvent::Output(event, wl_output),
                )) => {
                    debug!("Wayland event: {event:?}");
                    Some(Message::OutputEvent((event, wl_output)))
                }
                iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    debug!("Keyboard event received: {key:?}");
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
                        debug!("ESC key pressed, closing all menus");
                        Some(Message::CloseAllMenus)
                    } else {
                        None
                    }
                }
                _ => None,
            }),
        ];

        subscriptions.extend(self.modules_subscriptions(&self.config.modules.left));
        subscriptions.extend(self.modules_subscriptions(&self.config.modules.center));
        subscriptions.extend(self.modules_subscriptions(&self.config.modules.right));

        Subscription::batch(subscriptions)
    }

    fn register_modules(&mut self) {
        let ctx = &self.module_context;
        let mut register = |name: &str, result: Result<(), modules::ModuleError>| {
            if let Err(err) = result {
                error!("failed to register {name} module: {err}");
            }
        };

        register("app-launcher", self.app_launcher.register(ctx, ())); // uses optional config at view time
        register("clipboard", self.clipboard.register(ctx, ()));
        register("clock", self.clock.register(ctx, &self.config.clock.format));
        register(
            "updates",
            self.updates.register(ctx, self.config.updates.as_ref()),
        );
        register(
            "workspaces",
            self.workspaces.register(ctx, &self.config.workspaces),
        );
        register("window-title", self.window_title.register(ctx, ()));
        register("system-info", self.system_info.register(ctx, ()));
        register("keyboard-layout", self.keyboard_layout.register(ctx, ()));
        register("keyboard-submap", self.keyboard_submap.register(ctx, ()));
        register("tray", self.tray.register(ctx, ()));
        register("battery", self.battery.register(ctx, ()));
        register("privacy", self.privacy.register(ctx, ()));
        register("settings", self.settings.register(ctx, ()));
        register("media-player", self.media_player.register(ctx, ()));

        for definition in &self.config.custom_modules {
            match self.custom.get_mut(&definition.name) {
                Some(module) => {
                    if let Err(err) = module.register(ctx, Some(definition)) {
                        error!(
                            "failed to register custom module '{}': {err}",
                            definition.name
                        );
                    }
                }
                None => error!(
                    "custom module '{}' missing runtime state entry during registration",
                    definition.name
                ),
            }
        }

        for (name, module) in self.custom.iter_mut() {
            if !self
                .config
                .custom_modules
                .iter()
                .any(|definition| definition.name == *name)
            {
                if let Err(err) = module.register(ctx, None) {
                    error!("failed to clear registration for custom module '{name}': {err}");
                }
            }
        }
    }

    fn update_custom_modules(&mut self, config: &Config, impact: &ConfigImpact) {
        let mut state = HashMap::with_capacity(config.custom_modules.len());

        for module in &config.custom_modules {
            let module_name = module.name.clone();
            let module_key = ModuleName::Custom(module_name.clone());

            let entry = if impact.affects_module(&module_key) {
                Custom::default()
            } else if let Some(existing) = self.custom.remove(module_name.as_str()) {
                existing
            } else {
                Custom::default()
            };

            state.insert(module_name, entry);
        }

        self.custom = state;
    }

    fn message_from_bus_event(event: BusEvent) -> Option<Message> {
        match event {
            BusEvent::Redraw => Some(Message::None),
            BusEvent::PopupToggle => Some(Message::CloseAllMenus),
            BusEvent::Module(module) => App::message_from_module_event(module),
        }
    }

    fn message_from_module_event(event: ModuleEvent) -> Option<Message> {
        match event {
            ModuleEvent::Updates(message) => Some(Message::Updates(message)),
            ModuleEvent::Workspaces(message) => Some(Message::Workspaces(message)),
            ModuleEvent::WindowTitle(message) => Some(Message::WindowTitle(message)),
            ModuleEvent::SystemInfo(message) => Some(Message::SystemInfo(message)),
            ModuleEvent::KeyboardLayout(message) => Some(Message::KeyboardLayout(message)),
            ModuleEvent::KeyboardSubmap(message) => Some(Message::KeyboardSubmap(message)),
            ModuleEvent::Tray(message) => Some(Message::Tray(message)),
            ModuleEvent::Clock(message) => Some(Message::Clock(message)),
            ModuleEvent::Battery(message) => Some(Message::Battery(message)),
            ModuleEvent::Privacy(message) => Some(Message::Privacy(message)),
            ModuleEvent::Settings(message) => Some(Message::Settings(message)),
            ModuleEvent::MediaPlayer(message) => Some(Message::MediaPlayer(message)),
            ModuleEvent::Custom { name, message } => {
                Some(Message::CustomUpdate(name.as_ref().to_owned(), message))
            }
        }
    }
}
