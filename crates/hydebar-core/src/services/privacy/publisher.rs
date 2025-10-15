use std::{future::Future, pin::Pin};

use iced::futures::{SinkExt, channel::mpsc::Sender};

use super::{PrivacyError, PrivacyService};
use crate::services::ServiceEvent;

/// Sink used to publish privacy service events to interested consumers.
pub trait PrivacyEventPublisher {
    /// Future type returned when emitting a [`ServiceEvent`].
    type SendFuture<'a>: Future<Output = Result<(), PrivacyError>> + Send + 'a
    where
        Self: 'a;

    /// Publish a privacy service event to subscribers.
    fn send(&mut self, event: ServiceEvent<PrivacyService>) -> Self::SendFuture<'_>;
}

impl PrivacyEventPublisher for Sender<ServiceEvent<PrivacyService>> {
    type SendFuture<'a>
        = Pin<Box<dyn Future<Output = Result<(), PrivacyError>> + Send + 'a>>
    where
        Self: 'a;

    fn send(&mut self, event: ServiceEvent<PrivacyService>) -> Self::SendFuture<'_> {
        Box::pin(async move {
            SinkExt::send(self, event)
                .await
                .map_err(|error| PrivacyError::channel(error.to_string()))
        })
    }
}
