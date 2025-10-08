use super::{
    audio::AudioMessage,
    bluetooth::BluetoothMessage,
    brightness::BrightnessMessage,
    network::NetworkMessage,
    state::{Message, Settings},
    upower::UPowerMessage,
};
use crate::services::{
    ReadOnlyService, ServiceEvent,
    audio::{AudioCommand, AudioService},
    bluetooth::{BluetoothCommand, BluetoothService},
    brightness::{BrightnessCommand, BrightnessService},
    network::{NetworkCommand, NetworkService},
    upower::{PowerProfileCommand, UPowerService},
};
use log::warn;
use tokio::runtime::Handle;

pub(super) trait SettingsCommandExt {
    fn spawn_audio_command(&self, command: AudioCommand) -> bool;
    fn spawn_brightness_command(&self, command: BrightnessCommand) -> bool;
    fn spawn_network_command(&self, command: NetworkCommand) -> bool;
    fn spawn_bluetooth_command(&self, command: BluetoothCommand) -> bool;
    fn spawn_upower_command(&self, command: PowerProfileCommand) -> bool;
}

impl SettingsCommandExt for Settings {
    fn spawn_audio_command(&self, command: AudioCommand) -> bool {
        spawn_optional_event_command(
            self.runtime(),
            self.sender(),
            self.audio.clone(),
            command,
            AudioService::run_command,
            Message::Audio,
            AudioMessage::Event,
            "audio",
        )
    }

    fn spawn_brightness_command(&self, command: BrightnessCommand) -> bool {
        spawn_event_command(
            self.runtime(),
            self.sender(),
            self.brightness.clone(),
            command,
            BrightnessService::run_command,
            Message::Brightness,
            BrightnessMessage::Event,
            "brightness",
        )
    }

    fn spawn_network_command(&self, command: NetworkCommand) -> bool {
        spawn_event_command(
            self.runtime(),
            self.sender(),
            self.network.clone(),
            command,
            NetworkService::run_command,
            Message::Network,
            NetworkMessage::Event,
            "network",
        )
    }

    fn spawn_bluetooth_command(&self, command: BluetoothCommand) -> bool {
        spawn_optional_event_command(
            self.runtime(),
            self.sender(),
            self.bluetooth.clone(),
            command,
            BluetoothService::run_command,
            Message::Bluetooth,
            BluetoothMessage::Event,
            "bluetooth",
        )
    }

    fn spawn_upower_command(&self, command: PowerProfileCommand) -> bool {
        spawn_event_command(
            self.runtime(),
            self.sender(),
            self.upower.clone(),
            command,
            UPowerService::run_command,
            Message::UPower,
            UPowerMessage::Event,
            "upower",
        )
    }
}

fn spawn_event_command<S, Command, Fut, Msg>(
    runtime: Option<Handle>,
    sender: Option<crate::ModuleEventSender<Message>>,
    service: Option<S>,
    command: Command,
    runner: fn(S, Command) -> Fut,
    message_ctor: fn(Msg) -> Message,
    event_ctor: fn(ServiceEvent<S>) -> Msg,
    service_name: &str,
) -> bool
where
    S: Send + Clone + ReadOnlyService + 'static,
    Command: Send + 'static,
    Fut: std::future::Future<Output = ServiceEvent<S>> + Send + 'static,
    Msg: Send + 'static,
{
    if let (Some(handle), Some(sender), Some(service)) = (runtime, sender, service) {
        let service_name = service_name.to_string();
        handle.spawn(async move {
            let event = runner(service, command).await;
            if let Err(err) = sender.try_send(message_ctor(event_ctor(event))) {
                warn!("failed to publish {service_name} command event: {err}");
            }
        });
        true
    } else {
        warn!("{service_name} command ignored because runtime, sender, or service is unavailable");
        false
    }
}

fn spawn_optional_event_command<S, Command, Fut, Msg>(
    runtime: Option<Handle>,
    sender: Option<crate::ModuleEventSender<Message>>,
    service: Option<S>,
    command: Command,
    runner: fn(S, Command) -> Fut,
    message_ctor: fn(Msg) -> Message,
    event_ctor: fn(ServiceEvent<S>) -> Msg,
    service_name: &str,
) -> bool
where
    S: Send + Clone + ReadOnlyService + 'static,
    Command: Send + 'static,
    Fut: std::future::Future<Output = Option<ServiceEvent<S>>> + Send + 'static,
    Msg: Send + 'static,
{
    if let (Some(handle), Some(sender), Some(service)) = (runtime, sender, service) {
        let service_name = service_name.to_string();
        handle.spawn(async move {
            if let Some(event) = runner(service, command).await {
                if let Err(err) = sender.try_send(message_ctor(event_ctor(event))) {
                    warn!("failed to publish {service_name} command event: {err}");
                }
            }
        });
        true
    } else {
        warn!("{service_name} command ignored because runtime, sender, or service is unavailable");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_fail_gracefully_without_runtime() {
        let settings = Settings::default();

        assert!(!settings.spawn_audio_command(AudioCommand::ToggleSinkMute));
        assert!(!settings.spawn_bluetooth_command(BluetoothCommand::Toggle));
        assert!(!settings.spawn_brightness_command(BrightnessCommand::Set(0.5)));
        assert!(!settings.spawn_network_command(NetworkCommand::ToggleWiFi));
        assert!(!settings.spawn_upower_command(PowerProfileCommand::Toggle));
    }
}
