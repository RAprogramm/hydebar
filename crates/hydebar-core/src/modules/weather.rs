use std::time::Duration;

use log::error;
use serde::Deserialize;
use tokio::{task::JoinHandle, time::interval};

use crate::{ModuleContext, ModuleEventSender, event_bus::ModuleEvent};

/// Weather condition data from wttr.in API
#[derive(Debug, Clone, Deserialize,)]
pub struct CurrentCondition
{
    pub temp_C:         String,
    pub temp_F:         String,
    #[serde(rename = "weatherDesc")]
    pub weather_desc:   Vec<WeatherDesc,>,
    pub humidity:       String,
    #[serde(rename = "windspeedKmph")]
    pub windspeed_kmph: String,
}

#[derive(Debug, Clone, Deserialize,)]
pub struct WeatherDesc
{
    pub value: String,
}

#[derive(Debug, Clone, Deserialize,)]
pub struct WeatherResponse
{
    pub current_condition: Vec<CurrentCondition,>,
}

/// Weather data for rendering
#[derive(Debug, Clone,)]
pub struct WeatherData
{
    pub temperature:  String,
    pub description:  String,
    pub humidity:     String,
    pub wind_speed:   String,
    pub location:     String,
    pub use_celsius:  bool,
    pub last_updated: chrono::DateTime<chrono::Local,>,
}

impl WeatherData
{
    pub fn new(location: String, use_celsius: bool,) -> Self
    {
        Self {
            temperature: String::from("--",),
            description: String::from("Loading...",),
            humidity: String::from("--",),
            wind_speed: String::from("--",),
            location,
            use_celsius,
            last_updated: chrono::Local::now(),
        }
    }

    pub fn from_response(response: WeatherResponse, location: String, use_celsius: bool,) -> Self
    {
        if let Some(condition,) = response.current_condition.first() {
            let temperature = if use_celsius {
                format!("{}°C", condition.temp_C)
            } else {
                format!("{}°F", condition.temp_F)
            };

            Self {
                temperature,
                description: condition
                    .weather_desc
                    .first()
                    .map(|d| d.value.clone(),)
                    .unwrap_or_default(),
                humidity: format!("{}%", condition.humidity),
                wind_speed: format!("{} km/h", condition.windspeed_kmph),
                location,
                use_celsius,
                last_updated: chrono::Local::now(),
            }
        } else {
            Self::new(location, use_celsius,)
        }
    }

    pub fn display_temp(&self,) -> &str
    {
        &self.temperature
    }

    pub fn display_description(&self,) -> &str
    {
        &self.description
    }
}

/// Events emitted by the weather module
#[derive(Debug, Clone,)]
pub enum WeatherEvent
{
    Updated(WeatherData,),
    Error(String,),
}

/// Message type for GUI communication
#[derive(Debug, Clone,)]
pub enum Message
{
    Update(WeatherData,),
    Error(String,),
    Refresh,
}

/// Weather module - business logic only, no GUI!
#[derive(Debug,)]
pub struct Weather
{
    data:            WeatherData,
    update_interval: Duration,
    sender:          Option<ModuleEventSender<WeatherEvent,>,>,
    task:            Option<JoinHandle<(),>,>,
}

impl Weather
{
    pub fn new(location: String, use_celsius: bool, update_interval_minutes: u64,) -> Self
    {
        Self {
            data:            WeatherData::new(location, use_celsius,),
            update_interval: Duration::from_secs(update_interval_minutes * 60,),
            sender:          None,
            task:            None,
        }
    }

    /// Get current weather data for rendering
    pub fn data(&self,) -> &WeatherData
    {
        &self.data
    }

    /// Initialize with module context
    pub fn register(&mut self, ctx: &ModuleContext,)
    {
        self.sender = Some(ctx.module_sender(|event: WeatherEvent| match event {
            WeatherEvent::Updated(data,) => ModuleEvent::Weather(Message::Update(data,),),
            WeatherEvent::Error(err,) => ModuleEvent::Weather(Message::Error(err,),),
        },),);

        if let Some(task,) = self.task.take() {
            task.abort();
        }

        if let Some(sender,) = self.sender.clone() {
            let interval_duration = self.update_interval;
            let location = self.data.location.clone();
            let use_celsius = self.data.use_celsius;

            self.task = Some(ctx.runtime_handle().spawn(async move {
                let mut ticker = interval(interval_duration,);

                loop {
                    ticker.tick().await;

                    match Self::fetch_weather(&location,).await {
                        Ok(response,) => {
                            let data = WeatherData::from_response(
                                response,
                                location.clone(),
                                use_celsius,
                            );
                            if let Err(err,) = sender.try_send(WeatherEvent::Updated(data,),) {
                                error!("Failed to publish weather update: {err}");
                            }
                        }
                        Err(err,) => {
                            error!("Failed to fetch weather: {err}");
                            if let Err(e,) =
                                sender.try_send(WeatherEvent::Error(err.to_string(),),)
                            {
                                error!("Failed to publish weather error: {e}");
                            }
                        }
                    }
                }
            },),);
        }

        // Trigger immediate fetch
        if let Some(sender,) = &self.sender {
            let location = self.data.location.clone();
            let use_celsius = self.data.use_celsius;
            let update_sender = sender.clone();

            ctx.runtime_handle().spawn(async move {
                match Self::fetch_weather(&location,).await {
                    Ok(response,) => {
                        let data = WeatherData::from_response(response, location, use_celsius,);
                        let _ = update_sender.try_send(WeatherEvent::Updated(data,),);
                    }
                    Err(err,) => {
                        let _ = update_sender.try_send(WeatherEvent::Error(err.to_string(),),);
                    }
                }
            },);
        }
    }

    /// Update weather state from GUI message
    pub fn update(&mut self, message: Message,)
    {
        match message {
            Message::Update(data,) => {
                self.data = data;
            }
            Message::Error(err,) => {
                error!("Weather module error: {err}");
                self.data.description = format!("Error: {err}");
            }
            Message::Refresh => {
                // Trigger manual refresh
                if let Some(sender,) = &self.sender {
                    let location = self.data.location.clone();
                    let use_celsius = self.data.use_celsius;
                    let update_sender = sender.clone();

                    tokio::spawn(async move {
                        match Self::fetch_weather(&location,).await {
                            Ok(response,) => {
                                let data =
                                    WeatherData::from_response(response, location, use_celsius,);
                                let _ = update_sender.try_send(WeatherEvent::Updated(data,),);
                            }
                            Err(err,) => {
                                let _ =
                                    update_sender.try_send(WeatherEvent::Error(err.to_string(),),);
                            }
                        }
                    },);
                }
            }
        }
    }

    /// Fetch weather data from wttr.in API
    async fn fetch_weather(location: &str,) -> anyhow::Result<WeatherResponse,>
    {
        let url = format!("https://wttr.in/{}?format=j1", location);
        let response = reqwest::get(&url,).await?.json::<WeatherResponse>().await?;

        Ok(response,)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn weather_data_new()
    {
        let data = WeatherData::new(String::from("London",), true,);
        assert_eq!(data.location, "London");
        assert_eq!(data.temperature, "--");
        assert!(data.use_celsius);
    }

    #[test]
    fn weather_data_display()
    {
        let data = WeatherData::new(String::from("London",), true,);
        assert_eq!(data.display_temp(), "--");
        assert_eq!(data.display_description(), "Loading...");
    }
}
