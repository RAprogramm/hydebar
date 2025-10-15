use std::time::Duration;

use chrono::{DateTime, Local};
use log::error;
use tokio::{task::JoinHandle, time::interval};

use crate::{
    ModuleContext, ModuleEventSender, event_bus::ModuleEvent, modules::weather::WeatherData
};

/// Clock data for rendering
#[derive(Debug, Clone)]
pub struct ClockData {
    pub current_time: DateTime<Local>,
    pub weather:      Option<WeatherData>
}

impl ClockData {
    pub fn new() -> Self {
        Self {
            current_time: Local::now(),
            weather:      None
        }
    }

    pub fn update(&mut self) {
        self.current_time = Local::now();
    }

    pub fn update_weather(&mut self, weather: WeatherData) {
        self.weather = Some(weather);
    }

    /// Format the time according to chrono format string
    pub fn format(&self, format: &str) -> String {
        self.current_time.format(format).to_string()
    }
}

impl Default for ClockData {
    fn default() -> Self {
        Self::new()
    }
}

/// Events emitted by the clock module
#[derive(Debug, Clone)]
pub enum ClockEvent {
    Tick(DateTime<Local>)
}

/// Message type for GUI communication
#[derive(Debug, Clone)]
pub enum Message {
    Update,
    UpdateWeather(WeatherData)
}

/// Clock module - business logic only, no GUI!
#[derive(Debug)]
pub struct Clock {
    data:          ClockData,
    tick_interval: Duration,
    sender:        Option<ModuleEventSender<ClockEvent>>,
    task:          Option<JoinHandle<()>>
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            data:          ClockData::new(),
            tick_interval: Duration::from_secs(5),
            sender:        None,
            task:          None
        }
    }
}

impl Clock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get current clock data for rendering
    pub fn data(&self) -> &ClockData {
        &self.data
    }

    /// Initialize with module context and time format
    pub fn register(&mut self, ctx: &ModuleContext, format: &str) {
        self.tick_interval = Self::determine_interval(format);
        self.data.update();
        self.sender =
            Some(ctx.module_sender(|_event: ClockEvent| ModuleEvent::Clock(Message::Update)));

        if let Some(task) = self.task.take() {
            task.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let interval_duration = self.tick_interval;
            let update_sender = sender.clone();

            self.task = Some(ctx.runtime_handle().spawn(async move {
                let mut ticker = interval(interval_duration);

                loop {
                    ticker.tick().await;
                    let now = Local::now();

                    if let Err(err) = update_sender.try_send(ClockEvent::Tick(now)) {
                        error!("Failed to publish clock tick: {err}");
                    }
                }
            }));
        }
    }

    /// Update clock state from GUI message
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.data.update();

                if let Some(sender) = &self.sender
                    && let Err(e) = sender.try_send(ClockEvent::Tick(self.data.current_time))
                {
                    error!("Failed to emit clock event: {}", e);
                }
            }
            Message::UpdateWeather(weather) => {
                self.data.update_weather(weather);
            }
        }
    }

    /// Determine tick interval based on format string
    fn determine_interval(format: &str) -> Duration {
        const SECOND_SPECIFIERS: [&str; 6] = ["%S", "%T", "%X", "%r", "%:z", "%s"];

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_data_format() {
        let data = ClockData::new();
        let formatted = data.format("%H:%M");
        assert!(formatted.contains(':'));
        assert_eq!(formatted.len(), 5);
    }

    #[test]
    fn determine_interval_with_seconds() {
        let interval = Clock::determine_interval("%H:%M:%S");
        assert_eq!(interval, Duration::from_secs(1));
    }

    #[test]
    fn determine_interval_without_seconds() {
        let interval = Clock::determine_interval("%H:%M");
        assert_eq!(interval, Duration::from_secs(5));
    }
}
