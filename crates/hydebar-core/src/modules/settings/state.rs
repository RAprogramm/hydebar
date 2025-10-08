use super::{
    audio::AudioMessage,
    bluetooth::BluetoothMessage,
    brightness::BrightnessMessage,
    commands::SettingsCommandExt,
    event_forwarders::{
        AudioEventForwarder, BluetoothEventForwarder, BrightnessEventForwarder,
        NetworkEventForwarder, UPowerEventForwarder,
    },
    network::NetworkMessage,
    power::PowerMessage,
    upower::UPowerMessage,
    view::SettingsViewExt,
};
use crate::{
    ModuleContext, ModuleEventSender,
    config::SettingsModuleConfig,
    event_bus::ModuleEvent,
    menu::MenuType,
    modules::{Module, ModuleError, OnModulePress},
    outputs::Outputs,
    password_dialog,
    services::{
        ReadOnlyService,
        ServiceEvent,
        audio::{AudioCommand, AudioService},
        bluetooth::{BluetoothCommand, BluetoothService},
        brightness::{BrightnessCommand, BrightnessService},
        idle_inhibitor::IdleInhibitorManager,
        network::{NetworkCommand, NetworkEvent, NetworkService},
        upower::{PowerProfileCommand, UPowerService},
    },
};
use iced::Task;
use log::info;
use tokio::{runtime::Handle, task::JoinHandle};

pub struct Settings {
    pub(super) audio: Option<AudioService>,
    pub brightness: Option<BrightnessService>,
    pub(super) network: Option<NetworkService>,
    pub(super) bluetooth: Option<BluetoothService>,
    pub(super) idle_inhibitor: Option<IdleInhibitorManager>,
    pub sub_menu: Option<SubMenu>,
    pub(super) upower: Option<UPowerService>,
    pub(super) password_dialog: Option<(String, String)>,
    pub(super) sender: Option<ModuleEventSender<Message>>,
    pub(super) runtime: Option<Handle>,
    pub(super) tasks: Vec<JoinHandle<()>>,
}

impl Default for Settings {
    fn default() -> Self {
        let idle_inhibitor = match IdleInhibitorManager::new() {
            Ok(manager) => Some(manager),
            Err(err) => {
                log::warn!("Failed to initialize idle inhibitor: {err}");
                None
            }
        };

        Self {
            audio: None,
            brightness: None,
            network: None,
            bluetooth: None,
            idle_inhibitor,
            sub_menu: None,
            upower: None,
            password_dialog: None,
            sender: None,
            runtime: None,
            tasks: Vec::new(),
        }
    }
}

impl Settings {
    pub(super) fn runtime(&self) -> Option<Handle> {
        self.runtime.as_ref().cloned()
    }

    pub(super) fn sender(&self) -> Option<ModuleEventSender<Message>> {
        self.sender.as_ref().cloned()
    }

    pub fn update(
        &mut self,
        message: Message,
        config: &SettingsModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config,
    )  {
        match message {
            Message::ToggleMenu(id, button_ui_ref) => {
                self.sub_menu = None;
                self.password_dialog = None;
                outputs.toggle_menu::<Message>(id, MenuType::Settings, button_ui_ref, main_config);
            }
            Message::Audio(msg) => match msg {
                AudioMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.audio = Some(service);
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(audio) = self.audio.as_mut() {
                            audio.update(data);

                            if self.sub_menu == Some(SubMenu::Sinks) && audio.sinks.len() < 2 {
                                self.sub_menu = None;
                            }

                            if self.sub_menu == Some(SubMenu::Sources) && audio.sources.len() < 2 {
                                self.sub_menu = None;
                            }
                        }
                    }
                    ServiceEvent::Error(err) => {
                        log::error!("Audio service error: {err:?}");
                    }
                },
                AudioMessage::ToggleSinkMute => {
                    let _spawned = self.spawn_audio_command(AudioCommand::ToggleSinkMute);
                }
                AudioMessage::SinkVolumeChanged(value) => {
                    let _spawned = self.spawn_audio_command(AudioCommand::SinkVolume(value));
                }
                AudioMessage::DefaultSinkChanged(name, port) => {
                    let _spawned = self.spawn_audio_command(AudioCommand::DefaultSink(name, port));
                }
                AudioMessage::ToggleSourceMute => {
                    let _spawned = self.spawn_audio_command(AudioCommand::ToggleSourceMute);
                }
                AudioMessage::SourceVolumeChanged(value) => {
                    let _spawned = self.spawn_audio_command(AudioCommand::SourceVolume(value));
                }
                AudioMessage::DefaultSourceChanged(name, port) => {
                    let _spawned =
                        self.spawn_audio_command(AudioCommand::DefaultSource(name, port));
                }
                AudioMessage::SinksMore(id) => {
                    if let Some(cmd) = &config.audio_sinks_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu::<Message>(id, main_config);
                    } else {
                    }
                }
                AudioMessage::SourcesMore(id) => {
                    if let Some(cmd) = &config.audio_sources_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu::<Message>(id, main_config);
                    } else {
                    }
                }
            },
            Message::UPower(msg) => match msg {
                UPowerMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.upower = Some(service);
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(upower) = self.upower.as_mut() {
                            upower.update(data);
                        }
                    }
                    ServiceEvent::Error(err) => {
                        log::error!("UPower service error: {err:?}");
                    }
                },
                UPowerMessage::TogglePowerProfile => {
                    let _spawned = self.spawn_upower_command(PowerProfileCommand::Toggle);
                }
            },
            Message::Network(msg) => match msg {
                NetworkMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.network = Some(service);
                    }
                    ServiceEvent::Update(NetworkEvent::RequestPasswordForSSID(ssid)) => {
                        self.password_dialog = Some((ssid, String::new()));
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(network) = self.network.as_mut() {
                            network.update(data);
                        }
                    }
                    ServiceEvent::Error(err) => {
                        log::error!("Network service error: {err:?}");
                    }
                },
                NetworkMessage::ToggleAirplaneMode => {
                    if self.sub_menu == Some(SubMenu::Wifi) {
                        self.sub_menu = None;
                    }

                    let _spawned = self.spawn_network_command(NetworkCommand::ToggleAirplaneMode);
                }
                NetworkMessage::ToggleWiFi => {
                    if self.sub_menu == Some(SubMenu::Wifi) {
                        self.sub_menu = None;
                    }

                    let _spawned = self.spawn_network_command(NetworkCommand::ToggleWiFi);
                }
                NetworkMessage::SelectAccessPoint(ac) => {
                    let _spawned =
                        self.spawn_network_command(NetworkCommand::SelectAccessPoint((ac, None)));
                }
                NetworkMessage::RequestWiFiPassword(id, ssid) => {
                    info!("Requesting password for {ssid}");
                    self.password_dialog = Some((ssid, String::new()));
                    outputs.request_keyboard::<Message>(id, main_config.menu_keyboard_focus);
                }
                NetworkMessage::ScanNearByWiFi => {
                    let _spawned = self.spawn_network_command(NetworkCommand::ScanNearByWiFi);
                }
                NetworkMessage::WiFiMore(id) => {
                    if let Some(cmd) = &config.wifi_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu::<Message>(id, main_config);
                    } else {
                    }
                }
                NetworkMessage::VpnMore(id) => {
                    if let Some(cmd) = &config.vpn_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu::<Message>(id, main_config);
                    } else {
                    }
                }
                NetworkMessage::ToggleVpn(vpn) => {
                    let _spawned = self.spawn_network_command(NetworkCommand::ToggleVpn(vpn));
                }
            },
            Message::Bluetooth(msg) => match msg {
                BluetoothMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.bluetooth = Some(service);
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(bluetooth) = self.bluetooth.as_mut() {
                            bluetooth.update(data);
                        }
                    }
                    ServiceEvent::Error(err) => {
                        log::error!("Bluetooth service error: {err:?}");
                    }
                },
                BluetoothMessage::Toggle => match self.bluetooth.as_mut() {
                    Some(_) => {
                        if self.sub_menu == Some(SubMenu::Bluetooth) {
                            self.sub_menu = None;
                        }

                        let _spawned = self.spawn_bluetooth_command(BluetoothCommand::Toggle);
                    }
                    None => {
                        log::warn!("Bluetooth service not initialized");
                    }
                },
                BluetoothMessage::More(id) => {
                    if let Some(cmd) = &config.bluetooth_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu::<Message>(id, main_config);
                    } else {
                    }
                }
            },
            Message::Brightness(msg) => match msg {
                BrightnessMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.brightness = Some(service);
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(brightness) = self.brightness.as_mut() {
                            brightness.update(data);
                        }
                    }
                    ServiceEvent::Error(err) => {
                        log::error!("Brightness service error: {err:?}");
                    }
                },
                BrightnessMessage::Change(value) => {
                    let _spawned = self.spawn_brightness_command(BrightnessCommand::Set(value));
                }
            },
            Message::ToggleSubMenu(menu_type) => {
                if self.sub_menu == Some(menu_type) {
                    self.sub_menu.take();
                } else {
                    self.sub_menu.replace(menu_type);

                    if menu_type == SubMenu::Wifi {
                        let _spawned = self.spawn_network_command(NetworkCommand::ScanNearByWiFi);
                    }
                }

            }
            Message::ToggleInhibitIdle => {
                if let Some(idle_inhibitor) = &mut self.idle_inhibitor {
                    idle_inhibitor.toggle();
                }
            }
            Message::Lock => {
                if let Some(lock_cmd) = &config.lock_cmd {
                    crate::utils::launcher::execute_command(lock_cmd.to_string());
                }
            }
            Message::Power(msg) => {
                msg.update();
            }
            Message::PasswordDialog(msg) => match msg {
                password_dialog::Message::PasswordChanged(password) => {
                    if let Some((_, current_password)) = &mut self.password_dialog {
                        *current_password = password;
                    }

                }
                password_dialog::Message::DialogConfirmed(id) => {
                    if let Some((ssid, password)) = self.password_dialog.take() {
                        if let Some(network) = self.network.as_ref() {
                            if let Some(access_point) = network
                                .wireless_access_points
                                .iter()
                                .find(|ap| ap.ssid == ssid)
                                .cloned()
                            {
                                self.spawn_network_command(NetworkCommand::SelectAccessPoint((
                                    // We intentionally clone the password to avoid holding a
                                    // mutable reference across the async boundary.
                                    access_point,
                                    Some(password.clone()),
                                )));
                            }
                        }

                        outputs.release_keyboard::<Message>(id, main_config.menu_keyboard_focus);
                    } else {
                        outputs.release_keyboard::<Message>(id, main_config.menu_keyboard_focus);
                    }
                }
                password_dialog::Message::DialogCancelled(id) => {
                    self.password_dialog = None;

                    outputs.release_keyboard::<Message>(id, main_config.menu_keyboard_focus);
                }
            },
        }
    }
}

impl<M> Module<M> for Settings
where
    M: 'static + Clone + From<Message>,
{
    type ViewData<'a> = <Self as SettingsViewExt>::ViewData<'a>;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        for task in self.tasks.drain(..) {
            task.abort();
        }

        let sender = ctx.module_sender(ModuleEvent::Settings);

        let mut tasks = Vec::new();

        let mut audio_publisher = AudioEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            AudioService::listen(&mut audio_publisher).await;
        }));

        let mut brightness_publisher = BrightnessEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            BrightnessService::listen(&mut brightness_publisher).await;
        }));

        let mut network_publisher = NetworkEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            NetworkService::listen(&mut network_publisher).await;
        }));

        let mut bluetooth_publisher = BluetoothEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            BluetoothService::listen(&mut bluetooth_publisher).await;
        }));

        let mut upower_publisher = UPowerEventForwarder::new(sender.clone());
        tasks.push(ctx.runtime_handle().spawn(async move {
            UPowerService::listen(&mut upower_publisher).await;
        }));

        self.sender = Some(sender);
        self.runtime = Some(ctx.runtime_handle().clone());
        self.tasks = tasks;

        Ok(())
    }

    fn view(
        &self,
        data: Self::ViewData<'_>,
    ) -> Option<(iced::Element<'static, M>, Option<OnModulePress<M>>)> {
        self.settings_view(data)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu(iced::window::Id, crate::position_button::ButtonUIRef),
    UPower(UPowerMessage),
    Network(NetworkMessage),
    Bluetooth(BluetoothMessage),
    Audio(AudioMessage),
    Brightness(BrightnessMessage),
    ToggleInhibitIdle,
    Lock,
    Power(PowerMessage),
    ToggleSubMenu(SubMenu),
    PasswordDialog(password_dialog::Message),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubMenu {
    Power,
    Sinks,
    Sources,
    Wifi,
    Vpn,
    Bluetooth,
}

// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use super::*;
    use crate::{event_bus::EventBus, modules::Module};
    use futures::future;
    use std::{
        num::NonZeroUsize,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    use tokio::runtime::Runtime;

    #[test]
    fn register_spawns_event_forwarders() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut settings = Settings::default();

        settings
            .register(&ctx, ())
            .expect("register should succeed");

        assert!(settings.sender.is_some());
        assert!(settings.runtime.is_some());
        assert_eq!(settings.tasks.len(), 5);

        for task in settings.tasks.drain(..) {
            task.abort();
        }
    }

    #[test]
    fn register_aborts_existing_tasks() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut settings = Settings::default();

        let cancelled = Arc::new(AtomicBool::new(false));
        let guard_flag = Arc::clone(&cancelled);

        settings.tasks.push(runtime.spawn(async move {
            struct CancelGuard(Arc<AtomicBool>);

            impl Drop for CancelGuard {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::SeqCst);
                }
            }

            let _guard = CancelGuard(guard_flag);

            future::pending::<()>().await;
        }));

        settings
            .register(&ctx, ())
            .expect("register should succeed");

        assert!(cancelled.load(Ordering::SeqCst));

        for task in settings.tasks.drain(..) {
            task.abort();
        }
    }
}
