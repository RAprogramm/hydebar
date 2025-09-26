use crate::{
    app,
    config::{WindowTitleConfig, WindowTitleMode},
    utils::truncate_text,
};
use hydebar_proto::ports::hyprland::{HyprlandPort, HyprlandWindowEvent};
use iced::{Element, Subscription, stream::channel, widget::text};
use log::error;
use std::{
    any::TypeId,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::time::sleep;
use tokio_stream::StreamExt;

const WINDOW_EVENT_RETRY_DELAY: Duration = Duration::from_millis(500);

use super::{Module, OnModulePress};

fn get_window(port: &dyn HyprlandPort, config: &WindowTitleConfig) -> Option<String> {
    match port.active_window() {
        Ok(Some(window)) => Some(match config.mode {
            WindowTitleMode::Title => window.title,
            WindowTitleMode::Class => window.class,
        }),
        Ok(None) => None,
        Err(err) => {
            error!("failed to retrieve active window: {err}");
            None
        }
    }
}

pub struct WindowTitle {
    hyprland: Arc<dyn HyprlandPort>,
    value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TitleChanged,
}

impl WindowTitle {
    pub fn new(hyprland: Arc<dyn HyprlandPort>, config: &WindowTitleConfig) -> Self {
        let init = get_window(hyprland.as_ref(), config);

        Self {
            hyprland,
            value: init,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockHyprlandPort;
    use hydebar_proto::config::{WindowTitleConfig, WindowTitleMode};

    #[test]
    fn initializes_title_from_port() {
        let port = Arc::new(MockHyprlandPort::with_active_window("Demo", "Class"));
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let config = WindowTitleConfig {
            mode: WindowTitleMode::Title,
            ..Default::default()
        };

        let module = WindowTitle::new(port_trait, &config);

        assert_eq!(module.current_value(), Some("Demo"));
    }

    #[test]
    fn update_handles_absent_window() {
        let port = Arc::new(MockHyprlandPort::default());
        *port
            .active_window
            .lock()
            .expect("active window lock poisoned") = None;
        let port_trait: Arc<dyn HyprlandPort> = port.clone();
        let config = WindowTitleConfig::default();

        let mut module = WindowTitle::new(port_trait, &config);
        module.update(Message::TitleChanged, &config);

        assert_eq!(module.current_value(), None);
    }
}

impl WindowTitle {
    pub fn update(&mut self, message: Message, config: &WindowTitleConfig) {
        match message {
            Message::TitleChanged => {
                if let Some(value) = get_window(self.hyprland.as_ref(), config) {
                    self.value = Some(truncate_text(&value, config.truncate_title_after_length));
                } else {
                    self.value = None;
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn current_value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

impl Module for WindowTitle {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        self.value.as_ref().map(|value| {
            (
                text(value)
                    .size(12)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .into(),
                None,
            )
        })
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
                            match hyprland.window_events() {
                                Ok(mut stream) => {
                                    while let Some(event) = stream.next().await {
                                        match event {
                                            Ok(
                                                HyprlandWindowEvent::ActiveWindowChanged
                                                | HyprlandWindowEvent::WindowClosed
                                                | HyprlandWindowEvent::WorkspaceFocusChanged,
                                            ) => {
                                                if let Ok(mut guard) = output.write() {
                                                    if let Err(err) = guard.try_send(Message::TitleChanged) {
                                                        error!(
                                                            "failed to enqueue title change notification: {err}"
                                                        );
                                                    }
                                                } else {
                                                    error!(
                                                        "failed to acquire lock for title change notification"
                                                    );
                                                }
                                            }
                                            Err(err) => {
                                                error!(
                                                    "window event stream error, restarting listener: {err}"
                                                );
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!(
                                        "failed to start window event stream, retrying: {err}"
                                    );
                                }
                            }

                            sleep(WINDOW_EVENT_RETRY_DELAY).await;
                        }
                    }
                }),
            )
            .map(app::Message::WindowTitle),
        )
    }
}
