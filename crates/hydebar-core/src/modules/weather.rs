use std::time::Duration;

use log::error;
use masterror::{AppError, AppResult};
use serde::Deserialize;
use tokio::{task::JoinHandle, time::interval};

use crate::{ModuleContext, ModuleEventSender, event_bus::ModuleEvent};

/// OpenWeatherMap API response structures
#[derive(Debug, Clone, Deserialize)]
pub struct WeatherResponse {
    pub main:    MainWeather,
    pub weather: Vec<WeatherCondition>,
    pub wind:    Wind
}

#[derive(Debug, Clone, Deserialize)]
pub struct MainWeather {
    pub temp:     f64,
    pub humidity: u32
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeatherCondition {
    pub description: String,
    pub icon:        String
}

#[derive(Debug, Clone, Deserialize)]
pub struct Wind {
    pub speed: f64
}

/// Weather data for rendering
#[derive(Debug, Clone)]
pub struct WeatherData {
    pub temperature:  String,
    pub description:  String,
    pub humidity:     String,
    pub wind_speed:   String,
    pub location:     String,
    pub use_celsius:  bool,
    pub last_updated: chrono::DateTime<chrono::Local>
}

impl WeatherData {
    pub fn new(location: String, use_celsius: bool) -> Self {
        Self {
            temperature: String::from("--"),
            description: String::from("Loading..."),
            humidity: String::from("--"),
            wind_speed: String::from("--"),
            location,
            use_celsius,
            last_updated: chrono::Local::now()
        }
    }

    pub fn from_response(response: WeatherResponse, location: String, use_celsius: bool) -> Self {
        // OpenWeatherMap returns temperature in Kelvin by default
        let temp_kelvin = response.main.temp;
        let temperature = if use_celsius {
            format!("{:.0}°C", temp_kelvin - 273.15)
        } else {
            format!("{:.0}°F", (temp_kelvin - 273.15) * 9.0 / 5.0 + 32.0)
        };

        let description = response
            .weather
            .first()
            .map(|w| w.description.clone())
            .unwrap_or_else(|| String::from("Unknown"));

        Self {
            temperature,
            description,
            humidity: format!("{}%", response.main.humidity),
            wind_speed: format!("{:.1} m/s", response.wind.speed),
            location,
            use_celsius,
            last_updated: chrono::Local::now()
        }
    }

    pub fn display_temp(&self) -> &str {
        &self.temperature
    }

    pub fn display_description(&self) -> &str {
        &self.description
    }
}

/// Events emitted by the weather module
#[derive(Debug, Clone)]
pub enum WeatherEvent {
    Updated(WeatherData),
    Error(String)
}

/// Message type for GUI communication
#[derive(Debug, Clone)]
pub enum Message {
    Update(WeatherData),
    Error(String),
    Refresh
}

/// Weather module - business logic only, no GUI!
#[derive(Debug)]
pub struct Weather {
    data:            WeatherData,
    api_key:         Option<String>,
    update_interval: Duration,
    sender:          Option<ModuleEventSender<WeatherEvent>>,
    task:            Option<JoinHandle<()>>
}

impl Weather {
    pub fn new(
        location: String,
        api_key: Option<String>,
        use_celsius: bool,
        update_interval_minutes: u64
    ) -> Self {
        Self {
            data: WeatherData::new(location, use_celsius),
            api_key,
            update_interval: Duration::from_secs(update_interval_minutes * 60),
            sender: None,
            task: None
        }
    }

    /// Get current weather data for rendering
    pub fn data(&self) -> &WeatherData {
        &self.data
    }

    /// Initialize with module context
    pub fn register(&mut self, ctx: &ModuleContext) {
        self.sender = Some(ctx.module_sender(|event: WeatherEvent| match event {
            WeatherEvent::Updated(data) => ModuleEvent::Weather(Message::Update(data)),
            WeatherEvent::Error(err) => ModuleEvent::Weather(Message::Error(err))
        }));

        if let Some(task) = self.task.take() {
            task.abort();
        }

        if let Some(sender) = self.sender.clone() {
            let interval_duration = self.update_interval;
            let location = self.data.location.clone();
            let use_celsius = self.data.use_celsius;
            let api_key = self.api_key.clone();

            self.task = Some(ctx.runtime_handle().spawn(async move {
                let mut ticker = interval(interval_duration);

                loop {
                    ticker.tick().await;

                    match Self::fetch_weather(&location, &api_key).await {
                        Ok(response) => {
                            let data = WeatherData::from_response(
                                response,
                                location.clone(),
                                use_celsius
                            );
                            if let Err(err) = sender.try_send(WeatherEvent::Updated(data)) {
                                error!("Failed to publish weather update: {err}");
                            }
                        }
                        Err(err) => {
                            error!("Failed to fetch weather: {err}");
                            if let Err(e) = sender.try_send(WeatherEvent::Error(err.to_string())) {
                                error!("Failed to publish weather error: {e}");
                            }
                        }
                    }
                }
            }));
        }

        // Trigger immediate fetch
        if let Some(sender) = &self.sender {
            let location = self.data.location.clone();
            let use_celsius = self.data.use_celsius;
            let api_key = self.api_key.clone();
            let update_sender = sender.clone();

            ctx.runtime_handle().spawn(async move {
                match Self::fetch_weather(&location, &api_key).await {
                    Ok(response) => {
                        let data = WeatherData::from_response(response, location, use_celsius);
                        let _ = update_sender.try_send(WeatherEvent::Updated(data));
                    }
                    Err(err) => {
                        let _ = update_sender.try_send(WeatherEvent::Error(err.to_string()));
                    }
                }
            });
        }
    }

    /// Update weather state from GUI message
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update(data) => {
                self.data = data;
            }
            Message::Error(err) => {
                error!("Weather module error: {err}");
                self.data.description = format!("Error: {err}");
            }
            Message::Refresh => {
                // Trigger manual refresh
                if let Some(sender) = &self.sender {
                    let location = self.data.location.clone();
                    let use_celsius = self.data.use_celsius;
                    let api_key = self.api_key.clone();
                    let update_sender = sender.clone();

                    tokio::spawn(async move {
                        match Self::fetch_weather(&location, &api_key).await {
                            Ok(response) => {
                                let data =
                                    WeatherData::from_response(response, location, use_celsius);
                                let _ = update_sender.try_send(WeatherEvent::Updated(data));
                            }
                            Err(err) => {
                                let _ =
                                    update_sender.try_send(WeatherEvent::Error(err.to_string()));
                            }
                        }
                    });
                }
            }
        }
    }

    /// Fetch weather data from OpenWeatherMap API
    async fn fetch_weather(
        location: &str,
        api_key: &Option<String>
    ) -> AppResult<WeatherResponse> {
        let api_key = api_key
            .as_ref()
            .ok_or_else(|| AppError::internal("Weather API key not configured in config.toml"))?;

        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}",
            location, api_key
        );

        let response = reqwest::get(&url)
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AppError::internal(format!("Weather API timeout for location '{}'", location))
                } else if e.is_connect() {
                    AppError::internal("No internet connection - cannot fetch weather")
                } else {
                    AppError::internal(format!("Network error fetching weather: {}", e))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::internal(match status.as_u16() {
                401 => format!("Invalid weather API key ({})", status),
                404 => format!("Location '{}' not found in weather database", location),
                429 => "Weather API rate limit exceeded - try again later".to_string(),
                500..=599 => format!("Weather API server error ({})", status),
                _ => format!("Weather API returned error {} for location '{}'", status, location)
            }));
        }

        let weather = response
            .json::<WeatherResponse>()
            .await
            .map_err(|e| {
                AppError::internal(format!(
                    "Invalid weather data format from API: {}",
                    e
                ))
            })?;

        Ok(weather)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_data_new() {
        let data = WeatherData::new(String::from("London"), true);
        assert_eq!(data.location, "London");
        assert_eq!(data.temperature, "--");
        assert!(data.use_celsius);
    }

    #[test]
    fn weather_data_display() {
        let data = WeatherData::new(String::from("London"), true);
        assert_eq!(data.display_temp(), "--");
        assert_eq!(data.display_description(), "Loading...");
    }
}
