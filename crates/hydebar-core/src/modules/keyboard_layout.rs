use hydebar_proto::ports::hyprland::{HyprlandKeyboardEvent, HyprlandKeyboardState, HyprlandPort};
use iced::Element;
use iced::widget::text;
use log::error;
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time::sleep};
use tokio_stream::StreamExt;

use crate::{
    ModuleContext, ModuleEventSender, config::KeyboardLayoutModuleConfig,
    event_bus::ModuleEvent,
};

use super::{Module, ModuleError, OnModulePress};

const KEYBOARD_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub struct KeyboardLayout {
    hyprland: Arc<dyn HyprlandPort>,
    multiple_layout: bool,
    active: String,
    sender: Option<ModuleEventSender<Message>>,
    task: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LayoutConfigChanged(bool),
    ActiveLayoutChanged(String),
    ChangeLayout,
}

impl KeyboardLayout {
    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        let HyprlandKeyboardState {
            active_layout,
            has_multiple_layouts,
            ..
        } = hyprland.keyboard_state().unwrap_or(HyprlandKeyboardState {
            active_layout: "unknown".to_string(),
            has_multiple_layouts: false,
            active_submap: None,
        });

        Self {
            hyprland,
            multiple_layout: has_multiple_layouts,
            active: active_layout,
            sender: None,
            task: None,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ActiveLayoutChanged(layout) => {
                self.active = layout;
            }
            Message::LayoutConfigChanged(layout_flag) => self.multiple_layout = layout_flag,
            Message::ChangeLayout => {
                if let Err(err) = self.hyprland.switch_keyboard_layout() {
                    error!("failed to switch keyboard layout: {err}");
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn active_layout(&self) -> &str {
        &self.active
    }

    #[cfg(test)]
    pub(crate) fn has_multiple_layouts(&self) -> bool {
        self.multiple_layout
    }
}

impl<M> Module<M> for KeyboardLayout {
    type ViewData<'a> = &'a KeyboardLayoutModuleConfig;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::KeyboardLayout));

        if let Some(handle) = self.task.take() {
            handle.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let hyprland = Arc::clone(&self.hyprland);
            self.task = Some(ctx.runtime_handle().spawn(async move {
                loop {
                    match hyprland.keyboard_events() {
                        Ok(mut stream) => {
                            while let Some(event) = stream.next().await {
                                match event {
                                    Ok(HyprlandKeyboardEvent::LayoutChanged(layout)) => {
                                        if let Err(err) = sender
                                            .try_send(Message::ActiveLayoutChanged(layout))
                                        {
                                            error!("failed to publish active layout update: {err}");
                                        }
                                    }
                                    Ok(HyprlandKeyboardEvent::LayoutConfigurationChanged(flag)) => {
                                        if let Err(err) = sender
                                            .try_send(Message::LayoutConfigChanged(flag))
                                        {
                                            error!("failed to publish layout configuration update: {err}");
                                        }
                                    }
                                    Ok(HyprlandKeyboardEvent::SubmapChanged(_)) => {}
                                    Err(err) => {
                                        error!("keyboard event stream error: {err}");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("failed to start keyboard event stream: {err}");
                        }
                    }

                    sleep(KEYBOARD_EVENT_RETRY_DELAY).await;
                }
            }));
        }

        Ok(())
    }

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if !self.multiple_layout {
            None
        } else {
            let active = match config.labels.get(&self.active) {
                Some(value) => value.to_string(),
                None => self.active.clone(),
            };
            Some((
                text(active).into(),
                None, // Action handled in GUI layer
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_from_keyboard_state() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();

        let module = KeyboardLayout::new(port_trait);

        assert_eq!(module.active_layout(), "us");
        assert!(module.has_multiple_layouts());
    }

    #[test]
    fn change_layout_invokes_port_command() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let mut module = KeyboardLayout::new(port_trait);

        module.update(Message::ChangeLayout);

        assert_eq!(port.switch_layout_calls(), 1);
    }
}
