use hydebar_proto::ports::hyprland::{HyprlandKeyboardEvent, HyprlandKeyboardState, HyprlandPort};
use iced::Element;
use iced::widget::text;
use log::error;
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time::sleep};
use tokio_stream::StreamExt;

use crate::{ event_bus::ModuleEvent};

use super::{Module, ModuleError, OnModulePress};

pub struct KeyboardSubmap {
    hyprland: Arc<dyn HyprlandPort>,
    submap: String,
    sender: Option<ModuleEventSender<Message>>,
    task: Option<JoinHandle<()>>,
}

const SUBMAP_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

impl KeyboardSubmap {
    pub fn new(hyprland: Arc<dyn HyprlandPort>) -> Self {
        let initial_submap = hyprland
            .keyboard_state()
            .unwrap_or(HyprlandKeyboardState {
                active_layout: String::new(),
                has_multiple_layouts: false,
                active_submap: None,
            })
            .active_submap
            .unwrap_or_default();

        Self {
            hyprland,
            submap: initial_submap,
            sender: None,
            task: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SubmapChanged(String),
}

impl KeyboardSubmap {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SubmapChanged(submap) => {
                self.submap = submap;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn submap(&self) -> &str {
        &self.submap
    }
}

impl<M> Module<M> for KeyboardSubmap {
    type ViewData<'a> = ();
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        self.sender = Some(ctx.module_sender(ModuleEvent::KeyboardSubmap));

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
                                    Ok(HyprlandKeyboardEvent::SubmapChanged(submap)) => {
                                        let payload = submap.unwrap_or_default();
                                        if let Err(err) =
                                            sender.try_send(Message::SubmapChanged(payload))
                                        {
                                            error!("failed to publish submap update: {err}");
                                        }
                                    }
                                    Ok(_) => {}
                                    Err(err) => {
                                        error!("keyboard submap stream error: {err}");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("failed to start keyboard submap stream: {err}");
                        }
                    }

                    sleep(SUBMAP_EVENT_RETRY_DELAY).await;
                }
            }));
        }

        Ok(())
    }

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if self.submap.is_empty() {
            None
        } else {
            Some((text(&self.submap).into(), None))
        }
    }

    // No iced subscription required; updates are dispatched via the module event sender.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockHyprlandPort;

    #[test]
    fn initializes_with_port_submap() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();

        let module = KeyboardSubmap::new(port_trait);

        assert_eq!(module.submap(), "resize");
    }

    #[test]
    fn update_replaces_submap_value() {
        let port = Arc::new(MockHyprlandPort::default());
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let mut module = KeyboardSubmap::new(port_trait);

        module.update(Message::SubmapChanged("launch".into()));

        assert_eq!(module.submap(), "launch");
    }
}
