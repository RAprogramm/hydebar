use std::future::{Ready, ready};

use log::warn;

use super::{
    audio::AudioMessage, bluetooth::BluetoothMessage, brightness::BrightnessMessage,
    network::NetworkMessage, state::Message, upower::UPowerMessage
};
use crate::{
    ModuleEventSender,
    services::{
        ServiceEvent, ServiceEventPublisher, audio::AudioService, bluetooth::BluetoothService,
        brightness::BrightnessService, network::NetworkService, upower::UPowerService
    }
};

pub(super) struct AudioEventForwarder {
    sender: ModuleEventSender<Message>
}

impl AudioEventForwarder {
    pub fn new(sender: ModuleEventSender<Message>) -> Self {
        Self {
            sender
        }
    }
}

impl ServiceEventPublisher<AudioService> for AudioEventForwarder {
    type SendFuture<'a>
        = Ready<()>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<AudioService>) -> Self::SendFuture<'_> {
        if let Err(err) = self
            .sender
            .try_send(Message::Audio(AudioMessage::Event(event)))
        {
            warn!("failed to publish audio event: {err}");
        }

        ready(())
    }
}

pub(super) struct BrightnessEventForwarder {
    sender: ModuleEventSender<Message>
}

impl BrightnessEventForwarder {
    pub fn new(sender: ModuleEventSender<Message>) -> Self {
        Self {
            sender
        }
    }
}

impl ServiceEventPublisher<BrightnessService> for BrightnessEventForwarder {
    type SendFuture<'a>
        = Ready<()>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<BrightnessService>) -> Self::SendFuture<'_> {
        if let Err(err) = self
            .sender
            .try_send(Message::Brightness(BrightnessMessage::Event(event)))
        {
            warn!("failed to publish brightness event: {err}");
        }

        ready(())
    }
}

pub(super) struct NetworkEventForwarder {
    sender: ModuleEventSender<Message>
}

impl NetworkEventForwarder {
    pub fn new(sender: ModuleEventSender<Message>) -> Self {
        Self {
            sender
        }
    }
}

impl ServiceEventPublisher<NetworkService> for NetworkEventForwarder {
    type SendFuture<'a>
        = Ready<()>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<NetworkService>) -> Self::SendFuture<'_> {
        if let Err(err) = self
            .sender
            .try_send(Message::Network(NetworkMessage::Event(event)))
        {
            warn!("failed to publish network event: {err}");
        }

        ready(())
    }
}

pub(super) struct BluetoothEventForwarder {
    sender: ModuleEventSender<Message>
}

impl BluetoothEventForwarder {
    pub fn new(sender: ModuleEventSender<Message>) -> Self {
        Self {
            sender
        }
    }
}

impl ServiceEventPublisher<BluetoothService> for BluetoothEventForwarder {
    type SendFuture<'a>
        = Ready<()>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<BluetoothService>) -> Self::SendFuture<'_> {
        if let Err(err) = self
            .sender
            .try_send(Message::Bluetooth(BluetoothMessage::Event(event)))
        {
            warn!("failed to publish bluetooth event: {err}");
        }

        ready(())
    }
}

pub(super) struct UPowerEventForwarder {
    sender: ModuleEventSender<Message>
}

impl UPowerEventForwarder {
    pub fn new(sender: ModuleEventSender<Message>) -> Self {
        Self {
            sender
        }
    }
}

impl ServiceEventPublisher<UPowerService> for UPowerEventForwarder {
    type SendFuture<'a>
        = Ready<()>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<UPowerService>) -> Self::SendFuture<'_> {
        if let Err(err) = self
            .sender
            .try_send(Message::UPower(UPowerMessage::Event(event)))
        {
            warn!("failed to publish upower event: {err}");
        }

        ready(())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use tokio::runtime::Runtime;

    use super::*;
    use crate::{
        ModuleContext, ModuleEventSender,
        event_bus::{BusEvent, EventBus, EventReceiver, ModuleEvent},
        modules::settings::Message
    };

    fn setup_forwarder() -> (Runtime, EventReceiver, ModuleEventSender<Message>) {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let sender = bus.sender();
        let receiver = bus.receiver();
        let ctx = ModuleContext::new(sender, runtime.handle().clone());
        let module_sender = ctx.module_sender(ModuleEvent::Settings);
        (runtime, receiver, module_sender)
    }

    #[test]
    fn audio_forwarder_enqueues_events() {
        let (runtime, mut receiver, sender) = setup_forwarder();
        let mut forwarder = AudioEventForwarder::new(sender);

        let _ = forwarder.send(ServiceEvent::Error(()));

        let event = receiver.try_recv().expect("event queued");
        match event {
            Some(BusEvent::Module(ModuleEvent::Settings(Message::Audio(
                AudioMessage::Event(ServiceEvent::Error(()))
            )))) => {}
            other => panic!("unexpected event: {other:?}")
        }

        drop(runtime);
    }

    #[test]
    fn network_forwarder_enqueues_events() {
        let (runtime, mut receiver, sender) = setup_forwarder();
        let mut forwarder = NetworkEventForwarder::new(sender);

        let error = crate::services::network::NetworkServiceError::new("failure");
        let _ = forwarder.send(ServiceEvent::Error(error.clone()));

        let event = receiver.try_recv().expect("event queued");
        match event {
            Some(BusEvent::Module(ModuleEvent::Settings(Message::Network(
                NetworkMessage::Event(ServiceEvent::Error(received))
            )))) => {
                assert_eq!(received.message(), error.message());
            }
            other => panic!("unexpected event: {other:?}")
        }

        drop(runtime);
    }
}
