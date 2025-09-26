use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{Icons, icon},
    services::{
        ReadOnlyService, ServiceEvent,
        privacy::{PrivacyService, error::PrivacyError},
    },
};
use iced::{
    Alignment, Element, Subscription, Task,
    widget::{Row, container},
};
use log::{error, warn};

/// Message emitted by the privacy module subscription.
#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    Event(ServiceEvent<PrivacyService>),
}

/// UI module exposing privacy information icons.
#[derive(Debug, Default, Clone)]
pub struct Privacy {
    pub service: Option<PrivacyService>,
}

impl Privacy {
    /// Update the module state based on new privacy events.
    pub fn update(&mut self, message: PrivacyMessage) -> Task<crate::app::Message> {
        match message {
            PrivacyMessage::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Task::none()
                }
                ServiceEvent::Update(data) => {
                    if let Some(privacy) = self.service.as_mut() {
                        privacy.update(data);
                    }
                    Task::none()
                }
                ServiceEvent::Error(error) => {
                    match error {
                        PrivacyError::WebcamUnavailable => {
                            warn!(
                                "Webcam device unavailable; continuing with PipeWire-only privacy data"
                            );
                        }
                        _ => error!("Privacy service error: {error}"),
                    }
                    Task::none()
                }
            },
        }
    }
}

impl Module for Privacy {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    /// Render the privacy indicator when data is available.
    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if let Some(service) = self.service.as_ref() {
            if !service.no_access() {
                Some((
                    container(
                        Row::new()
                            .push_maybe(
                                service
                                    .screenshare_access()
                                    .then(|| icon(Icons::ScreenShare)),
                            )
                            .push_maybe(service.webcam_access().then(|| icon(Icons::Webcam)))
                            .push_maybe(service.microphone_access().then(|| icon(Icons::Mic1)))
                            .align_y(Alignment::Center)
                            .spacing(8),
                    )
                    .style(|theme| container::Style {
                        text_color: Some(theme.extended_palette().danger.weak.color),
                        ..Default::default()
                    })
                    .into(),
                    None,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Subscribe to the privacy service updates.
    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(PrivacyService::subscribe().map(|e| app::Message::Privacy(PrivacyMessage::Event(e))))
    }
}
