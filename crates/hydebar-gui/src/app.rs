use std::{
    collections::HashMap,
    f32::consts::PI,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use flexi_logger::LoggerHandle;
use hydebar_core::{
    HEIGHT,
    config::{self, ConfigApplied, ConfigDegradation, ConfigEvent, ConfigImpact, ConfigManager},
    event_bus::{BusEvent, EventReceiver, ModuleEvent},
    menu::{MenuSize, MenuType, menu_wrapper},
    modules::{
        self,
        app_launcher::AppLauncher,
        battery::Battery,
        clipboard::Clipboard,
        clock::Clock,
        custom_module::Custom,
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer,
        privacy::Privacy,
        settings::{Settings, brightness::BrightnessMessage},
        system_info::SystemInfo,
        tray::{TrayMessage, TrayModule},
        updates::Updates,
        window_title::WindowTitle,
        workspaces::Workspaces,
    },
    outputs::{HasOutput, Outputs},
    position_button::ButtonUIRef,
    services::{Service, ServiceEvent, brightness::BrightnessCommand, tray::TrayEvent},
    style::{backdrop_color, darken_color, hydebar_theme},
    utils,
};
use hydebar_proto::config::{AppearanceStyle, Config, ModuleName, Position};
use hydebar_proto::ports::hyprland::HyprlandPort;
use iced::{
    Alignment, Color, Element, Gradient, Length, Radians, Subscription, Task, Theme,
    daemon::Appearance,
    event::{
        listen_with,
        wayland::{Event as WaylandEvent, OutputEvent},
    },
    gradient::Linear,
    keyboard, time,
    widget::{Row, container},
    window::Id,
};
use log::{debug, error, info, warn};
use wayland_client::protocol::wl_output::WlOutput;

use crate::{centerbox, get_log_spec};

pub struct App {
    config_path: PathBuf,
    logger: LoggerHandle,
    hyprland: Arc<dyn HyprlandPort>,
    config_manager: Arc<ConfigManager>,
    bus_receiver: Arc<Mutex<EventReceiver>>,
    micro_ticker: MicroTicker,
    pub config: Config,
    pub outputs: Outputs,
    pub app_launcher: AppLauncher,
    pub custom: HashMap<String, Custom>,
    pub updates: Updates,
    pub clipboard: Clipboard,
    pub workspaces: Workspaces,
    pub window_title: WindowTitle,
    pub system_info: SystemInfo,
    pub keyboard_layout: KeyboardLayout,
    pub keyboard_submap: KeyboardSubmap,
    pub tray: TrayModule,
    pub clock: Clock,
    pub battery: Battery,
    pub privacy: Privacy,
    pub settings: Settings,
    pub media_player: MediaPlayer,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    MicroTick,
    BusFlushed(BusFlushOutcome),
    ConfigChanged(ConfigApplied),
    ConfigDegraded(ConfigDegradation),
    ToggleMenu(MenuType, Id, ButtonUIRef),
    CloseMenu(Id),
    CloseAllMenus,
    OpenLauncher,
    OpenClipboard,
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::TrayMessage),
    Clock(modules::clock::Message),
    Battery(modules::battery::Message),
    Privacy(modules::privacy::PrivacyMessage),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    OutputEvent((OutputEvent, WlOutput)),
    LaunchCommand(String),
    CustomUpdate(String, modules::custom_module::Message),
}

#[derive(Debug, Clone)]
struct BusFlushOutcome {
    events: Vec<BusEvent>,
    had_error: bool,
}

impl BusFlushOutcome {
    fn empty() -> Self {
        Self {
            events: Vec::new(),
            had_error: false,
        }
    }

    fn with_events(events: Vec<BusEvent>, had_error: bool) -> Self {
        Self { events, had_error }
    }

    fn had_error(&self) -> bool {
        self.had_error
    }

    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn into_events(self) -> Vec<BusEvent> {
        self.events
    }
}

#[derive(Debug, Clone)]
struct MicroTicker {
    fast_interval: Duration,
    slow_interval: Duration,
    idle_threshold: u8,
    idle_ticks: u8,
    current_interval: Duration,
}

impl MicroTicker {
    fn new(fast_interval: Duration, slow_interval: Duration, idle_threshold: u8) -> Self {
        Self {
            fast_interval,
            slow_interval,
            idle_threshold,
            idle_ticks: 0,
            current_interval: fast_interval,
        }
    }

    fn interval(&self) -> Duration {
        self.current_interval
    }

    fn record_activity(&mut self) {
        self.idle_ticks = 0;
        self.current_interval = self.fast_interval;
    }

    fn record_idle(&mut self) {
        if self.idle_ticks < self.idle_threshold {
            self.idle_ticks += 1;
        }

        if self.idle_ticks >= self.idle_threshold {
            self.current_interval = self.slow_interval;
        }
    }
}

impl Default for MicroTicker {
    fn default() -> Self {
        Self::new(Duration::from_millis(16), Duration::from_millis(33), 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flexi_logger::LoggerHandle;
    use hydebar_core::{config::ConfigManager, event_bus::EventBus, test_utils::MockHyprlandPort};
    use hydebar_proto::ports::hyprland::HyprlandPort;
    use std::{num::NonZeroUsize, sync::OnceLock};

    fn test_logger() -> LoggerHandle {
        static LOGGER: OnceLock<LoggerHandle> = OnceLock::new();
        LOGGER
            .get_or_init(|| {
                flexi_logger::Logger::try_with_env_or_str("off")
                    .expect("failed to configure test logger")
                    .start()
                    .expect("failed to start test logger")
            })
            .clone()
    }

    #[test]
    fn app_stores_injected_hyprland_port() {
        let logger = test_logger();
        let config = Config::default();
        let path = PathBuf::new();
        let mock = Arc::new(MockHyprlandPort::default());
        let mock_port: Arc<dyn HyprlandPort> = mock.clone();

        let config_manager = Arc::new(ConfigManager::new(config.clone()));
        let capacity = NonZeroUsize::new(16).expect("non-zero");
        let bus = EventBus::new(capacity);

        let (app, _) = App::new((
            logger,
            config,
            Arc::clone(&config_manager),
            path,
            Arc::clone(&mock_port),
            bus.receiver(),
        ))();

        assert!(Arc::ptr_eq(&app.hyprland, &mock_port));
    }

    #[test]
    fn keyboard_layout_change_triggers_port_call() {
        let logger = test_logger();
        let config = Config::default();
        let path = PathBuf::new();
        let mock = Arc::new(MockHyprlandPort::default());
        let mock_port: Arc<dyn HyprlandPort> = mock.clone();

        let config_manager = Arc::new(ConfigManager::new(config.clone()));
        let capacity = NonZeroUsize::new(16).expect("non-zero");
        let bus = EventBus::new(capacity);

        let (mut app, _) = App::new((
            logger,
            config,
            Arc::clone(&config_manager),
            path,
            mock_port,
            bus.receiver(),
        ))();

        let _ = app.update(Message::KeyboardLayout(
            hydebar_core::modules::keyboard_layout::Message::ChangeLayout,
        ));

        assert_eq!(mock.switch_layout_calls(), 1);
    }
}

impl App {
    pub fn new(
        (logger, config, config_manager, config_path, hyprland, bus_receiver): (
            LoggerHandle,
            Config,
            Arc<ConfigManager>,
            PathBuf,
            Arc<dyn HyprlandPort>,
            EventReceiver,
        ),
    ) -> impl FnOnce() -> (Self, Task<Message>) {
        move || {
            let (outputs, task) = Outputs::new(config.appearance.style, config.position, &config);

            let custom = config
                .custom_modules
                .iter()
                .map(|o| (o.name.clone(), Custom::default()))
                .collect();
            let hyprland_clone = Arc::clone(&hyprland);
            (
                App {
                    config_path,
                    logger,
                    hyprland,
                    config_manager,
                    bus_receiver: Arc::new(Mutex::new(bus_receiver)),
                    micro_ticker: MicroTicker::default(),
                    outputs,
                    app_launcher: AppLauncher,
                    custom,
                    updates: Updates::default(),
                    clipboard: Clipboard,
                    workspaces: Workspaces::new(Arc::clone(&hyprland_clone), &config.workspaces),
                    window_title: WindowTitle::new(
                        Arc::clone(&hyprland_clone),
                        &config.window_title,
                    ),
                    system_info: SystemInfo::default(),
                    keyboard_layout: KeyboardLayout::new(Arc::clone(&hyprland_clone)),
                    keyboard_submap: KeyboardSubmap::new(hyprland_clone),
                    tray: TrayModule::default(),
                    clock: Clock::default(),
                    battery: Battery::default(),
                    privacy: Privacy::default(),
                    settings: Settings::default(),
                    media_player: MediaPlayer::default(),
                    config,
                },
                task,
            )
        }
    }

    pub fn title(&self, _id: Id) -> String {
        String::from("hydebar")
    }

    pub fn theme(&self, _id: Id) -> Theme {
        hydebar_theme(&self.config.appearance)
    }

    pub fn style(&self, theme: &Theme) -> Appearance {
        Appearance {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
            icon_color: theme.palette().text,
        }
    }

    pub fn scale_factor(&self, _id: Id) -> f64 {
        self.config.appearance.scale_factor
    }

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
                let ConfigApplied { config, impact } = update;
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
                                crate::app::Message::Settings(
                                    crate::modules::settings::Message::Brightness(
                                        BrightnessMessage::Event(event),
                                    ),
                                )
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
            Message::SystemInfo(message) => self.system_info.update(message),
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
            Message::Privacy(msg) => self.privacy.update(msg),
            Message::Settings(message) => self.settings.update(
                message,
                &self.config.settings,
                &mut self.outputs,
                &self.config,
            ),
            Message::OutputEvent((event, wl_output)) => match event {
                iced::event::wayland::OutputEvent::Created(info) => {
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
                iced::event::wayland::OutputEvent::Removed => {
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

    pub fn view(&self, id: Id) -> Element<Message> {
        match self.outputs.has(id) {
            Some(HasOutput::Main) => {
                let left = self.modules_section(
                    &self.config.modules.left,
                    id,
                    self.config.appearance.opacity,
                );
                let center = self.modules_section(
                    &self.config.modules.center,
                    id,
                    self.config.appearance.opacity,
                );
                let right = self.modules_section(
                    &self.config.modules.right,
                    id,
                    self.config.appearance.opacity,
                );

                let centerbox = centerbox::Centerbox::new([left, center, right])
                    .spacing(4)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .height(
                        if self.config.appearance.style == AppearanceStyle::Islands {
                            HEIGHT
                        } else {
                            HEIGHT - 8.
                        } as f32,
                    )
                    .padding(
                        if self.config.appearance.style == AppearanceStyle::Islands {
                            [4, 4]
                        } else {
                            [0, 0]
                        },
                    );

                container(centerbox)
                    .style(|t| container::Style {
                        background: match self.config.appearance.style {
                            AppearanceStyle::Gradient => Some({
                                let start_color = t
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity);

                                let start_color = if self.outputs.menu_is_open() {
                                    darken_color(start_color, self.config.appearance.menu.backdrop)
                                } else {
                                    start_color
                                };

                                let end_color = if self.outputs.menu_is_open() {
                                    backdrop_color(self.config.appearance.menu.backdrop)
                                } else {
                                    Color::TRANSPARENT
                                };

                                Gradient::Linear(
                                    Linear::new(Radians(PI))
                                        .add_stop(
                                            0.0,
                                            match self.config.position {
                                                Position::Top => start_color,
                                                Position::Bottom => end_color,
                                            },
                                        )
                                        .add_stop(
                                            1.0,
                                            match self.config.position {
                                                Position::Top => end_color,
                                                Position::Bottom => start_color,
                                            },
                                        ),
                                )
                                .into()
                            }),
                            AppearanceStyle::Solid => Some({
                                let bg = t
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity);
                                if self.outputs.menu_is_open() {
                                    darken_color(bg, self.config.appearance.menu.backdrop)
                                } else {
                                    bg
                                }
                                .into()
                            }),
                            AppearanceStyle::Islands => {
                                if self.outputs.menu_is_open() {
                                    Some(
                                        backdrop_color(self.config.appearance.menu.backdrop).into(),
                                    )
                                } else {
                                    None
                                }
                            }
                        },
                        ..Default::default()
                    })
                    .into()
            }
            Some(HasOutput::Menu(menu_info)) => match menu_info {
                Some((MenuType::Updates, button_ui_ref)) => menu_wrapper(
                    id,
                    self.updates
                        .menu_view(id, self.config.appearance.menu.opacity)
                        .map(Message::Updates),
                    MenuSize::Small,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.style,
                    self.config.appearance.menu.opacity,
                    self.config.appearance.menu.backdrop,
                ),
                Some((MenuType::Tray(name), button_ui_ref)) => menu_wrapper(
                    id,
                    self.tray
                        .menu_view(name, self.config.appearance.menu.opacity)
                        .map(Message::Tray),
                    MenuSize::Small,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.style,
                    self.config.appearance.menu.opacity,
                    self.config.appearance.menu.backdrop,
                ),
                Some((MenuType::Settings, button_ui_ref)) => menu_wrapper(
                    id,
                    self.settings
                        .menu_view(
                            id,
                            &self.config.settings,
                            self.config.appearance.menu.opacity,
                            self.config.position,
                        )
                        .map(Message::Settings),
                    MenuSize::Medium,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.style,
                    self.config.appearance.menu.opacity,
                    self.config.appearance.menu.backdrop,
                ),
                Some((MenuType::MediaPlayer, button_ui_ref)) => menu_wrapper(
                    id,
                    self.media_player
                        .menu_view(
                            &self.config.media_player,
                            self.config.appearance.menu.opacity,
                        )
                        .map(Message::MediaPlayer),
                    MenuSize::Large,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.style,
                    self.config.appearance.menu.opacity,
                    self.config.appearance.menu.backdrop,
                ),
                Some((MenuType::SystemInfo, button_ui_ref)) => menu_wrapper(
                    id,
                    self.system_info.menu_view().map(Message::SystemInfo),
                    MenuSize::Medium,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.style,
                    self.config.appearance.menu.opacity,
                    self.config.appearance.menu.backdrop,
                ),
                None => Row::new().into(),
            },
            None => Row::new().into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let timer = time::every(self.micro_ticker.interval()).map(|_| Message::MicroTick);

        Subscription::batch(vec![
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
        ])
    }
}

async fn drain_bus(receiver: Arc<Mutex<EventReceiver>>) -> BusFlushOutcome {
    let mut guard = match receiver.lock() {
        Ok(guard) => guard,
        Err(err) => {
            error!("event bus receiver poisoned: {err}");
            return BusFlushOutcome::with_events(Vec::new(), true);
        }
    };

    let mut events = Vec::new();
    let mut had_error = false;

    loop {
        match guard.try_recv() {
            Ok(Some(event)) => events.push(event),
            Ok(None) => break,
            Err(err) => {
                error!("failed to read event bus payload: {err}");
                had_error = true;
                break;
            }
        }
    }

    BusFlushOutcome::with_events(events, had_error)
}
