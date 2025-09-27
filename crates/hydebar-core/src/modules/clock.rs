use crate::{ModuleContext, ModuleEventSender, app, event_bus::ModuleEvent};

use super::{Module, ModuleError, OnModulePress};
use chrono::{DateTime, Local};
use iced::{Element, widget::text};
use log::error;
use std::time::Duration;
use tokio::{task::JoinHandle, time::interval};

pub struct Clock {
    date: DateTime<Local>,
    tick_interval: Duration,
    sender: Option<ModuleEventSender<Message>>,
    task: Option<JoinHandle<()>>,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            date: Local::now(),
            tick_interval: Duration::from_secs(5),
            sender: None,
            task: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Update,
}

impl Clock {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.date = Local::now();
            }
        }
    }

    fn determine_interval(format: &str) -> Duration {
        const SECOND_SPECIFIERS: [&str; 6] = [
            "%S",  // Seconds (00-60)
            "%T",  // Hour:Minute:Second
            "%X",  // Locale time representation with seconds
            "%r",  // 12-hour clock time with seconds
            "%:z", // UTC offset with seconds
            "%s",  // Unix timestamp (seconds since epoch)
        ];

        if SECOND_SPECIFIERS
            .iter()
            .any(|specifier| format.contains(specifier))
        {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        }
    }
}

impl Module for Clock {
    type ViewData<'a> = &'a str;
    type RegistrationData<'a> = &'a str;

    fn register(
        &mut self,
        ctx: &ModuleContext,
        format: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        self.tick_interval = Self::determine_interval(format);
        self.date = Local::now();
        self.sender = Some(ctx.module_sender(ModuleEvent::Clock));

        if let Some(task) = self.task.take() {
            task.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let interval_duration = self.tick_interval;
            self.task = Some(ctx.runtime_handle().spawn(async move {
                let mut ticker = interval(interval_duration);

                loop {
                    ticker.tick().await;

                    if let Err(err) = sender.try_send(Message::Update) {
                        error!("failed to publish clock tick: {err}");
                    }
                }
            }));
        }

        Ok(())
    }
    fn view(
        &self,
        format: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((text(self.date.format(format).to_string()).into(), None))
    }
}
