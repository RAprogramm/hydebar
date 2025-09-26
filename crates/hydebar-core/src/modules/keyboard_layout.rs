use hydebar_proto::ports::hyprland::{HyprlandKeyboardEvent, HyprlandKeyboardState, HyprlandPort};
use iced::{Element, Subscription, stream::channel, widget::text};
use log::error;
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::time::sleep;
use tokio_stream::StreamExt;

use crate::{app, config::KeyboardLayoutModuleConfig};

use super::{Module, OnModulePress};

const KEYBOARD_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub struct KeyboardLayout {
    hyprland: Arc<dyn HyprlandPort>,
    multiple_layout: bool,
    active: String,
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

impl Module for KeyboardLayout {
    type ViewData<'a> = &'a KeyboardLayoutModuleConfig;
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if !self.multiple_layout {
            None
        } else {
            let active = match config.labels.get(&self.active) {
                Some(value) => value.to_string(),
                None => self.active.clone(),
            };
            Some((
                text(active).into(),
                Some(OnModulePress::Action(Box::new(
                    app::Message::KeyboardLayout(Message::ChangeLayout),
                ))),
            ))
        }
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        let id = TypeId::of::<Self>();

        let hyprland = Arc::clone(&self.hyprland);

        Some(
            Subscription::run_with_id(
                id,
                channel(10, move |output| {
                    let hyprland = Arc::clone(&hyprland);
                    let output = Arc::new(RwLock::new(output));

                    async move {
                        loop {
                            match hyprland.keyboard_events() {
                                Ok(mut stream) => {
                                    while let Some(event) = stream.next().await {
                                        match event {
                                            Ok(HyprlandKeyboardEvent::LayoutChanged(layout)) => {
                                                match output.write() {
                                                    Ok(mut guard) => {
                                                        if let Err(err) = guard
                                                            .try_send(Message::ActiveLayoutChanged(
                                                                layout,
                                                            ))
                                                        {
                                                            error!(
                                                                "failed to enqueue active layout update: {err}"
                                                            );
                                                        }
                                                    }
                                                    Err(_) => {
                                                        error!(
                                                            "failed to acquire lock for active layout update"
                                                        );
                                                    }
                                                }
                                            }
                                            Ok(HyprlandKeyboardEvent::LayoutConfigurationChanged(
                                                multiple,
                                            )) => {
                                                match output.write() {
                                                    Ok(mut guard) => {
                                                        if let Err(err) = guard
                                                            .try_send(Message::LayoutConfigChanged(
                                                                multiple,
                                                            ))
                                                        {
                                                            error!(
                                                                "failed to enqueue keyboard layout flag update: {err}"
                                                            );
                                                        }
                                                    }
                                                    Err(_) => {
                                                        error!(
                                                            "failed to acquire lock for keyboard layout flag update"
                                                        );
                                                    }
                                                }
                                            }
                                            Ok(HyprlandKeyboardEvent::SubmapChanged(_)) => {
                                                // Submap events are handled by the KeyboardSubmap module.
                                            }
                                            Err(err) => {
                                                error!(
                                                    "keyboard event stream error, restarting listener: {err}"
                                                );
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!(
                                        "failed to start keyboard event stream, retrying: {err}"
                                    );
                                }
                            }

                            sleep(KEYBOARD_EVENT_RETRY_DELAY).await;
                        }
                    }
                }),
            )
            .map(app::Message::KeyboardLayout),
        )
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
