use crate::{
    app,
    components::icons::{Icons, icon},
    config::BatteryModuleConfig,
    menu::MenuType,
    modules::{Module, OnModulePress},
    services::{
        ServiceEvent,
        upower::{BatteryData, PowerProfile, UPowerEvent, UPowerService},
    },
    utils::IndicatorState,
};
use iced::{
    Alignment, Element, Subscription, Theme,
    widget::{Row, container, row, text},
};
use log::warn;
use std::any::TypeId;

/// Maintains state required to render the battery module.
#[derive(Default)]
pub struct Battery {
    battery: Option<BatteryData>,
    power_profile: PowerProfile,
}

/// Messages emitted by the battery module runtime.
#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<UPowerService>),
}

struct SubscriptionMarker;

impl Battery {
    /// Updates the module state in response to a new message.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Event(ServiceEvent::Init(service)) => {
                self.battery = service.battery;
                self.power_profile = service.power_profile;
            }
            Message::Event(ServiceEvent::Update(update)) => match update {
                UPowerEvent::UpdateBattery(data) => {
                    self.battery = Some(data);
                }
                UPowerEvent::NoBattery => {
                    self.battery = None;
                }
                UPowerEvent::UpdatePowerProfile(profile) => {
                    self.power_profile = profile;
                }
            },
            Message::Event(ServiceEvent::Error(_)) => {
                warn!("Failed to receive battery updates from UPower");
            }
        }
    }

    fn battery_indicator(&self, config: &BatteryModuleConfig) -> Option<Element<'_, app::Message>> {
        let battery = self.battery?;
        let state = battery.get_indicator_state();
        let mut content = row!(icon(battery.get_icon()))
            .align_y(Alignment::Center)
            .spacing(4);

        if config.show_percentage {
            content = content.push(text(format!("{}%", battery.capacity)));
        }

        Some(
            container(content)
                .style(move |theme: &Theme| container::Style {
                    text_color: Some(match state {
                        IndicatorState::Success => theme.palette().success,
                        IndicatorState::Warning => theme.extended_palette().danger.weak.color,
                        IndicatorState::Danger => theme.palette().danger,
                        IndicatorState::Normal => theme.palette().text,
                    }),
                    ..Default::default()
                })
                .into(),
        )
    }

    fn power_profile_indicator(&self) -> Option<Element<'_, app::Message>> {
        let profile = self.power_profile;

        if matches!(profile, PowerProfile::Unknown) {
            return None;
        }

        let icon_type: Icons = profile.into();

        Some(
            container(icon(icon_type))
                .style(move |theme: &Theme| container::Style {
                    text_color: Some(match profile {
                        PowerProfile::Performance => theme.palette().danger,
                        PowerProfile::PowerSaver => theme.palette().success,
                        PowerProfile::Balanced | PowerProfile::Unknown => theme.palette().text,
                    }),
                    ..Default::default()
                })
                .into(),
        )
    }
}

impl Module for Battery {
    type ViewData<'a> = &'a BatteryModuleConfig;
    type RegistrationData<'a> = ();

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        let mut segments: Vec<Element<app::Message>> = Vec::new();

        if config.show_power_profile {
            if let Some(profile) = self.power_profile_indicator() {
                segments.push(profile);
            }
        }

        if let Some(battery) = self.battery_indicator(config) {
            segments.push(battery);
        }

        if segments.is_empty() {
            return if config.show_when_unavailable {
                Some((
                    container(text("Battery")).into(),
                    config
                        .open_settings_on_click
                        .then_some(OnModulePress::ToggleMenu(MenuType::Settings)),
                ))
            } else {
                None
            };
        }

        let content = Row::with_children(segments)
            .align_y(Alignment::Center)
            .spacing(8);

        let action = config
            .open_settings_on_click
            .then_some(OnModulePress::ToggleMenu(MenuType::Settings));

        Some((content.into(), action))
    }

    fn subscription(&self) -> Option<Subscription<app::Message>> {
        Some(
            UPowerService::subscription_with_id(TypeId::of::<SubscriptionMarker>())
                .map(Message::Event)
                .map(app::Message::Battery),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::upower::{BatteryStatus, UPowerEvent};
    use std::time::Duration;

    fn config() -> BatteryModuleConfig {
        BatteryModuleConfig::default()
    }

    #[test]
    fn hides_view_without_battery() {
        let battery = Battery::default();
        assert!(battery.view(&config()).is_none());
    }

    #[test]
    fn shows_view_with_battery() {
        let mut battery = Battery::default();
        battery.update(Message::Event(ServiceEvent::Update(
            UPowerEvent::UpdateBattery(BatteryData {
                capacity: 42,
                status: BatteryStatus::Discharging(Duration::from_secs(10)),
            }),
        )));

        let view = battery.view(&config());
        assert!(view.is_some());
    }

    #[test]
    fn displays_placeholder_when_configured() {
        let mut config = config();
        config.show_when_unavailable = true;

        let battery = Battery::default();
        let view = battery.view(&config);

        assert!(view.is_some());
    }
}
