use iced::{
    Alignment, Element,
    widget::{Column, Row, button, container, scrollable, text},
};
use log::error;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext, ModuleEventSender,
    components::icons::{Icons, icon},
    event_bus::ModuleEvent,
    menu::MenuType,
    services::{
        ReadOnlyService, ServiceEvent,
        notifications::{Notification, NotificationsService},
    },
};

/// Message emitted by the notifications module.
#[derive(Debug, Clone,)]
pub enum NotificationsMessage
{
    Event(ServiceEvent<NotificationsService,>,),
    Dismiss(u32,),
    ClearAll,
    ToggleDND,
}

/// UI module displaying notification center with bell icon.
#[derive(Debug, Default,)]
pub struct Notifications
{
    pub service: Option<NotificationsService,>,
    sender:      Option<ModuleEventSender<NotificationsMessage,>,>,
}

impl Notifications
{
    /// Update the module state based on notification events.
    pub fn update(&mut self, message: NotificationsMessage,)
    {
        match message {
            NotificationsMessage::Event(event,) => match event {
                ServiceEvent::Init(service,) => {
                    self.service = Some(service,);
                }
                ServiceEvent::Update(data,) => {
                    if let Some(notifications,) = self.service.as_mut() {
                        notifications.update(data,);
                    }
                }
                ServiceEvent::Error(error,) => {
                    error!("Notifications service error: {error}");
                }
            },
            NotificationsMessage::Dismiss(id,) => {
                if let Some(service,) = self.service.as_mut() {
                    service.dismiss(id,);
                }
            }
            NotificationsMessage::ClearAll => {
                if let Some(service,) = self.service.as_mut() {
                    service.clear_all();
                }
            }
            NotificationsMessage::ToggleDND => {
                if let Some(service,) = self.service.as_mut() {
                    service.toggle_dnd();
                }
            }
        }
    }

    /// Render notification center menu popup.
    pub fn menu_view(&self, _opacity: f32,) -> Element<'_, NotificationsMessage,>
    {
        let Some(service,) = self.service.as_ref() else {
            return text("Loading notifications...",).into();
        };

        let notifications = service.get_notifications();
        let is_dnd = service.is_dnd();

        let mut content = Column::new().spacing(8,).padding(12,);

        // Header with DND toggle
        let header = Row::new()
            .push(text("Notifications",).size(16,),)
            .push(
                button(text(if is_dnd { "DND: ON" } else { "DND: OFF" },),)
                    .on_press(NotificationsMessage::ToggleDND,),
            )
            .push(button(text("Clear All",),).on_press(NotificationsMessage::ClearAll,),)
            .spacing(8,)
            .align_y(Alignment::Center,);

        content = content.push(header,);

        // Notification list
        if notifications.is_empty() {
            content = content.push(text("No notifications",).size(14,),);
        } else {
            let mut list = Column::new().spacing(4,);

            for notification in notifications {
                list = list.push(notification_item(notification,),);
            }

            content = content.push(scrollable(list,).height(300,),);
        }

        container(content,)
            .style(move |theme| container::Style {
                background: Some(theme.palette().background.into(),),
                border: iced::Border {
                    color:  theme.palette().primary,
                    width:  1.0,
                    radius: 8.0.into(),
                },
                text_color: Some(theme.palette().text,),
                ..Default::default()
            },)
            .into()
    }
}

impl<M,> Module<M,> for Notifications
where
    M: 'static + Clone + From<NotificationsMessage,>,
{
    type ViewData<'a,> = ();
    type RegistrationData<'a,> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_,>,
    ) -> Result<(), ModuleError,>
    {
        let sender = ctx.module_sender(ModuleEvent::Notifications,);
        self.sender = Some(sender,);

        Ok((),)
    }

    fn subscription(&self,) -> Option<iced::Subscription<M,>,>
    {
        use crate::services::ReadOnlyService;

        Some(
            crate::services::notifications::NotificationsService::subscribe()
                .map(NotificationsMessage::Event,)
                .map(M::from,),
        )
    }

    /// Render notification icon with unread count.
    fn view(
        &self,
        _: Self::ViewData<'_,>,
    ) -> Option<(Element<'static, M,>, Option<OnModulePress<M,>,>,),>
    {
        let unread_count = self.service.as_ref().map(|s| s.unread_count(),).unwrap_or(0,);

        let content = if unread_count > 0 {
            Row::new()
                .push(text(format!("🔔 {}", unread_count,),),)
                .spacing(4,)
                .align_y(Alignment::Center,)
        } else {
            Row::new().push(text("🔔",),)
        };

        Some((
            container(content,).into(),
            Some(OnModulePress::ToggleMenu(MenuType::Notifications,),),
        ),)
    }
}

/// Render a single notification item.
fn notification_item<M,>(notification: Notification,) -> Element<'static, M,>
where
    M: 'static + Clone + From<NotificationsMessage,>,
{
    let summary = text(notification.summary.clone(),).size(14,);
    let body = text(notification.body.clone(),).size(12,);

    let content = Column::new()
        .push(
            Row::new()
                .push(summary,)
                .push(
                    button(icon(Icons::Close,),)
                        .on_press(NotificationsMessage::Dismiss(notification.id,).into(),),
                )
                .spacing(8,)
                .align_y(Alignment::Center,),
        )
        .push(body,)
        .spacing(4,);

    container(content,)
        .padding(8,)
        .style(|theme| container::Style {
            background: Some(theme.extended_palette().background.weak.color.into(),),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        },)
        .into()
}
