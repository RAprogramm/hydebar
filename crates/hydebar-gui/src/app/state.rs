use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use flexi_logger::LoggerHandle;
use hydebar_core::{
    ModuleContext,
    config::{ConfigApplied, ConfigDegradation, ConfigManager},
    event_bus::{EventReceiver, EventSender},
    menu::MenuType,
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
    outputs::Outputs,
    position_button::ButtonUIRef,
};
use hydebar_proto::{config::Config, ports::hyprland::HyprlandPort};
use iced::{Task, event::wayland::OutputEvent, window::Id};
use tokio::runtime::Handle;
use wayland_client::protocol::wl_output::WlOutput;

use super::{bus::BusFlushOutcome, micro_ticker::MicroTicker};

pub struct App {
    pub(super) config_path: PathBuf,
    pub(super) logger: LoggerHandle,
    pub(super) hyprland: Arc<dyn HyprlandPort>,
    pub(super) config_manager: Arc<ConfigManager>,
    pub(super) bus_receiver: Arc<Mutex<EventReceiver>>,
    pub(super) micro_ticker: MicroTicker,
    pub(super) module_context: ModuleContext,
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
    Tray(TrayMessage),
    Clock(modules::clock::Message),
    Battery(modules::battery::Message),
    Privacy(modules::privacy::PrivacyMessage),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    OutputEvent((OutputEvent, WlOutput)),
    LaunchCommand(String),
    CustomUpdate(String, modules::custom_module::Message),
}

impl App {
    pub fn new(
        (
            logger,
            config,
            config_manager,
            config_path,
            hyprland,
            event_sender,
            runtime_handle,
            bus_receiver,
        ): (
            LoggerHandle,
            Config,
            Arc<ConfigManager>,
            PathBuf,
            Arc<dyn HyprlandPort>,
            EventSender,
            Handle,
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
            let module_context = ModuleContext::new(event_sender, runtime_handle);
            let hyprland_clone = Arc::clone(&hyprland);
            let mut app = App {
                config_path,
                logger,
                hyprland,
                config_manager,
                bus_receiver: Arc::new(Mutex::new(bus_receiver)),
                micro_ticker: MicroTicker::default(),
                module_context,
                outputs,
                app_launcher: AppLauncher,
                custom,
                updates: Updates::default(),
                clipboard: Clipboard,
                workspaces: Workspaces::new(Arc::clone(&hyprland_clone), &config.workspaces),
                window_title: WindowTitle::new(Arc::clone(&hyprland_clone), &config.window_title),
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
            };

            app.register_modules();

            (app, task)
        }
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
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let event_sender = bus.sender();
        let runtime_handle = runtime.handle().clone();
        let bus_receiver = bus.receiver();

        let (app, _) = App::new((
            logger,
            config,
            Arc::clone(&config_manager),
            path,
            Arc::clone(&mock_port),
            event_sender,
            runtime_handle,
            bus_receiver,
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
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let event_sender = bus.sender();
        let runtime_handle = runtime.handle().clone();
        let bus_receiver = bus.receiver();

        let (mut app, _) = App::new((
            logger,
            config,
            Arc::clone(&config_manager),
            path,
            mock_port,
            event_sender,
            runtime_handle,
            bus_receiver,
        ))();

        let _ = app.update(Message::KeyboardLayout(
            hydebar_core::modules::keyboard_layout::Message::ChangeLayout,
        ));

        assert_eq!(mock.switch_layout_calls(), 1);
    }
}
