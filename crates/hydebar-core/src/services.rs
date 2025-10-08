use iced::{
    Subscription, Task,
    futures::{SinkExt, channel::mpsc::Sender},
};
use std::{future::Future, pin::Pin};

pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod idle_inhibitor;
pub mod mpris;
pub mod network;
pub mod privacy;
pub mod tray;
pub mod upower;

#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),
    Update(S::UpdateEvent),
    Error(S::Error),
}

pub trait Service: ReadOnlyService {
    type Command;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>>;
}

pub trait ReadOnlyService: Sized {
    type UpdateEvent;
    type Error: Clone;

    fn update(&mut self, event: Self::UpdateEvent);

    fn subscribe() -> Subscription<ServiceEvent<Self>>;
}

pub trait ServiceEventPublisher<S: ReadOnlyService> {
    type SendFuture<'a>: Future<Output = ()> + Send + 'a
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<S>) -> Self::SendFuture<'_>;
}

impl<S> ServiceEventPublisher<S> for Sender<ServiceEvent<S>>
where
    S: ReadOnlyService + 'static + Send,
    S::UpdateEvent: Send,
    S::Error: Send,
{
    type SendFuture<'a>
        = Pin<Box<dyn Future<Output = ()> + Send + 'a>>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<S>) -> Self::SendFuture<'_> {
        Box::pin(async move {
            let _ = SinkExt::send(self, event).await;
        })
    }
}
