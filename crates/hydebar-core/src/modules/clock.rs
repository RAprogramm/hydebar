use crate::{ModuleContext, app};

use super::{Module, ModuleError, OnModulePress};
use chrono::{DateTime, Local};
use iced::{Element, Subscription, time::every, widget::text};
use std::time::Duration;

pub struct Clock {
    date: DateTime<Local>,
    tick_interval: Duration,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            date: Local::now(),
            tick_interval: Duration::from_secs(5),
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
        _: &ModuleContext,
        format: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        self.tick_interval = Self::determine_interval(format);
        Ok(())
    }
    fn view(
        &self,
        format: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((text(self.date.format(format).to_string()).into(), None))
    }

    fn subscription(&self) -> Option<Subscription<app::Message>> {
        Some(
            every(self.tick_interval)
                .map(|_| Message::Update)
                .map(app::Message::Clock),
        )
    }
}
