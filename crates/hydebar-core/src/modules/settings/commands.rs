use log::warn;
use tokio::runtime::Handle;

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

pub(super) trait SettingsCommandExt
{
    fn spawn_audio_command(&self, command: AudioCommand,) -> bool;
    fn spawn_brightness_command(&self, command: BrightnessCommand,) -> bool;
    fn spawn_network_command(&self, command: NetworkCommand,) -> bool;
    fn spawn_bluetooth_command(&self, command: BluetoothCommand,) -> bool;
    fn spawn_upower_command(&self, command: PowerProfileCommand,) -> bool;
}

impl SettingsCommandExt for Settings
{
    fn spawn_audio_command(&self, command: AudioCommand,) -> bool
    {
        spawn_optional_event_command(OptionalEventCommandParams {
            runtime: self.runtime(),
            sender: self.sender(),
            service: self.audio.clone(),
            command,
            runner: AudioService::run_command,
            message_ctor: Message::Audio,
            event_ctor: AudioMessage::Event,
            service_name: "audio",
        },)
    }

    fn spawn_brightness_command(&self, command: BrightnessCommand,) -> bool
    {
        spawn_event_command(EventCommandParams {
            runtime: self.runtime(),
            sender: self.sender(),
            service: self.brightness.clone(),
            command,
            runner: BrightnessService::run_command,
            message_ctor: Message::Brightness,
            event_ctor: BrightnessMessage::Event,
            service_name: "brightness",
        },)
    }

    fn spawn_network_command(&self, command: NetworkCommand,) -> bool
    {
        spawn_event_command(EventCommandParams {
            runtime: self.runtime(),
            sender: self.sender(),
            service: self.network.clone(),
            command,
            runner: NetworkService::run_command,
            message_ctor: Message::Network,
            event_ctor: NetworkMessage::Event,
            service_name: "network",
        },)
    }

    fn spawn_bluetooth_command(&self, command: BluetoothCommand,) -> bool
    {
        spawn_optional_event_command(OptionalEventCommandParams {
            runtime: self.runtime(),
            sender: self.sender(),
            service: self.bluetooth.clone(),
            command,
            runner: BluetoothService::run_command,
            message_ctor: Message::Bluetooth,
            event_ctor: BluetoothMessage::Event,
            service_name: "bluetooth",
        },)
    }

    fn spawn_upower_command(&self, command: PowerProfileCommand,) -> bool
    {
        spawn_event_command(EventCommandParams {
            runtime: self.runtime(),
            sender: self.sender(),
            service: self.upower.clone(),
            command,
            runner: UPowerService::run_command,
            message_ctor: Message::UPower,
            event_ctor: UPowerMessage::Event,
            service_name: "upower",
        },)
    }
}

struct EventCommandParams<S, Command, Fut, Msg,>
where
    S: Send + Clone + ReadOnlyService + 'static,
    Command: Send + 'static,
    Fut: std::future::Future<Output = ServiceEvent<S,>,> + Send + 'static,
    Msg: Send + 'static,
{
    runtime:      Option<Handle,>,
    sender:       Option<crate::ModuleEventSender<Message,>,>,
    service:      Option<S,>,
    command:      Command,
    runner:       fn(S, Command,) -> Fut,
    message_ctor: fn(Msg,) -> Message,
    event_ctor:   fn(ServiceEvent<S,>,) -> Msg,
    service_name: &'static str,
}

fn spawn_event_command<S, Command, Fut, Msg,>(
    params: EventCommandParams<S, Command, Fut, Msg,>,
) -> bool
where
    S: Send + Clone + ReadOnlyService + 'static,
    Command: Send + 'static,
    Fut: std::future::Future<Output = ServiceEvent<S,>,> + Send + 'static,
    Msg: Send + 'static,
{
    if let (Some(handle,), Some(sender,), Some(service,),) =
        (params.runtime, params.sender, params.service,)
    {
        let service_name = params.service_name.to_string();
        let runner = params.runner;
        let message_ctor = params.message_ctor;
        let event_ctor = params.event_ctor;
        let command = params.command;
        handle.spawn(async move {
            let event = runner(service, command,).await;
            if let Err(err,) = sender.try_send(message_ctor(event_ctor(event,),),) {
                warn!("failed to publish {service_name} command event: {err}");
            }
        },);
        true
    } else {
        warn!(
            "{} command ignored because runtime, sender, or service is unavailable",
            params.service_name
        );
        false
    }
}

struct OptionalEventCommandParams<S, Command, Fut, Msg,>
where
    S: Send + Clone + ReadOnlyService + 'static,
    Command: Send + 'static,
    Fut: std::future::Future<Output = Option<ServiceEvent<S,>,>,> + Send + 'static,
    Msg: Send + 'static,
{
    runtime:      Option<Handle,>,
    sender:       Option<crate::ModuleEventSender<Message,>,>,
    service:      Option<S,>,
    command:      Command,
    runner:       fn(S, Command,) -> Fut,
    message_ctor: fn(Msg,) -> Message,
    event_ctor:   fn(ServiceEvent<S,>,) -> Msg,
    service_name: &'static str,
}

fn spawn_optional_event_command<S, Command, Fut, Msg,>(
    params: OptionalEventCommandParams<S, Command, Fut, Msg,>,
) -> bool
where
    S: Send + Clone + ReadOnlyService + 'static,
    Command: Send + 'static,
    Fut: std::future::Future<Output = Option<ServiceEvent<S,>,>,> + Send + 'static,
    Msg: Send + 'static,
{
    if let (Some(handle,), Some(sender,), Some(service,),) =
        (params.runtime, params.sender, params.service,)
    {
        let service_name = params.service_name.to_string();
        let runner = params.runner;
        let message_ctor = params.message_ctor;
        let event_ctor = params.event_ctor;
        let command = params.command;
        handle.spawn(async move {
            if let Some(event,) = runner(service, command,).await
                && let Err(err,) = sender.try_send(message_ctor(event_ctor(event,),),)
            {
                warn!("failed to publish {service_name} command event: {err}");
            }
        },);
        true
    } else {
        warn!(
            "{} command ignored because runtime, sender, or service is unavailable",
            params.service_name
        );
        false
    }
}

// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests
{
    use super::*;

    #[test]
    fn commands_fail_gracefully_without_runtime()
    {
        let settings = Settings::default();

        assert!(!settings.spawn_audio_command(AudioCommand::ToggleSinkMute));
        assert!(!settings.spawn_bluetooth_command(BluetoothCommand::Toggle));
        assert!(!settings.spawn_brightness_command(BrightnessCommand::Set(50)));
        assert!(!settings.spawn_network_command(NetworkCommand::ToggleWiFi));
        assert!(!settings.spawn_upower_command(PowerProfileCommand::Toggle));
    }
}
