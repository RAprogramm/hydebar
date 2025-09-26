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

use crate::app;

use super::{Module, OnModulePress};

pub struct KeyboardSubmap {
    hyprland: Arc<dyn HyprlandPort>,
    submap: String,
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

impl Module for KeyboardSubmap {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if self.submap.is_empty() {
            None
        } else {
            Some((text(&self.submap).into(), None))
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
                                            Ok(HyprlandKeyboardEvent::SubmapChanged(submap)) => {
                                                let payload = submap.unwrap_or_default();
                                                match output.write() {
                                                    Ok(mut guard) => {
                                                        if let Err(err) = guard
                                                            .try_send(Message::SubmapChanged(
                                                                payload,
                                                            ))
                                                        {
                                                            error!(
                                                                "failed to enqueue submap update: {err}"
                                                            );
                                                        }
                                                    }
                                                    Err(_) => {
                                                        error!(
                                                            "failed to acquire lock for submap update"
                                                        );
                                                    }
                                                }
                                            }
                                            Ok(_) => {}
                                            Err(err) => {
                                                error!(
                                                    "keyboard event stream error, restarting submap listener: {err}"
                                                );
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!(
                                        "failed to start keyboard submap stream, retrying: {err}"
                                    );
                                }
                            }

                            sleep(SUBMAP_EVENT_RETRY_DELAY).await;
                        }
                    }
                }),
            )
            .map(app::Message::KeyboardSubmap),
        )
    }
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
