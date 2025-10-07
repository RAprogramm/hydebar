use crate::{
    ModuleContext, ModuleEventSender,
    components::icons::Icons,
    event_bus::ModuleEvent,
    services::{ServiceEvent, upower::{BatteryData as UPowerBatteryData, UPowerEvent, UPowerService}},
};
use log::warn;
use std::time::Duration;

/// Battery icon type based on capacity and charging state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryIcon {
    Charging(u8),
    Discharging(u8),
    Full,
    Unknown,
}

impl From<BatteryIcon> for Icons {
    fn from(icon: BatteryIcon) -> Self {
        match icon {
            BatteryIcon::Charging(capacity) => match capacity {
                0..=20 => Icons::BatteryCharging20,
                21..=30 => Icons::BatteryCharging30,
                31..=50 => Icons::BatteryCharging50,
                51..=60 => Icons::BatteryCharging60,
                61..=80 => Icons::BatteryCharging80,
                81..=90 => Icons::BatteryCharging90,
                _ => Icons::BatteryCharging100,
            },
            BatteryIcon::Discharging(capacity) => match capacity {
                0..=10 => Icons::Battery0,
                11..=20 => Icons::Battery20,
                21..=30 => Icons::Battery30,
                31..=50 => Icons::Battery50,
                51..=60 => Icons::Battery60,
                61..=80 => Icons::Battery80,
                81..=90 => Icons::Battery90,
                _ => Icons::Battery100,
            },
            BatteryIcon::Full => Icons::Battery100,
            BatteryIcon::Unknown => Icons::BatteryUnknown,
        }
    }
}

/// Power management profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerProfile {
    #[default]
    Balanced,
    Performance,
    PowerSaver,
    Unknown,
}

impl From<crate::services::upower::PowerProfile> for PowerProfile {
    fn from(profile: crate::services::upower::PowerProfile) -> Self {
        match profile {
            crate::services::upower::PowerProfile::PowerSaver => PowerProfile::PowerSaver,
            crate::services::upower::PowerProfile::Balanced => PowerProfile::Balanced,
            crate::services::upower::PowerProfile::Performance => PowerProfile::Performance,
            crate::services::upower::PowerProfile::Unknown => PowerProfile::Unknown,
        }
    }
}

impl From<PowerProfile> for Icons {
    fn from(profile: PowerProfile) -> Self {
        match profile {
            PowerProfile::Performance => Icons::PowerProfilePerformance,
            PowerProfile::Balanced => Icons::PowerProfileBalanced,
            PowerProfile::PowerSaver => Icons::PowerProfilePowerSaver,
            PowerProfile::Unknown => Icons::PowerProfileBalanced,
        }
    }
}

/// Visual indicator state for battery status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorState {
    Normal,
    Warning,
    Danger,
    Success,
}

/// Complete battery state information for rendering
#[derive(Debug, Clone)]
pub struct BatteryData {
    pub capacity: u8,
    pub charging: bool,
    pub icon: BatteryIcon,
    pub time_remaining: Option<Duration>,
    pub power_profile: PowerProfile,
    pub indicator_state: IndicatorState,
}

impl BatteryData {
    pub fn new(
        capacity: u8,
        charging: bool,
        time_remaining: Option<Duration>,
        power_profile: PowerProfile,
    ) -> Self {
        let icon = if charging {
            if capacity >= 100 {
                BatteryIcon::Full
            } else {
                BatteryIcon::Charging(capacity)
            }
        } else {
            BatteryIcon::Discharging(capacity)
        };

        let indicator_state = if charging || capacity >= 100 {
            IndicatorState::Success
        } else if capacity <= 10 {
            IndicatorState::Danger
        } else if capacity <= 20 {
            IndicatorState::Warning
        } else {
            IndicatorState::Normal
        };

        Self {
            capacity,
            charging,
            icon,
            time_remaining,
            power_profile,
            indicator_state,
        }
    }
}

/// Events emitted by battery module
#[derive(Debug, Clone)]
pub enum BatteryEvent {
    StatusChanged(BatteryData),
    ProfileChanged(PowerProfile),
    LowBattery(u8),
    CriticalBattery(u8),
}

/// Message type for GUI communication
#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<UPowerService>),
}

/// Battery monitoring module
#[derive(Debug)]
pub struct Battery {
    data: Option<BatteryData>,
    sender: Option<ModuleEventSender<BatteryEvent>>,
}

impl Default for Battery {
    fn default() -> Self {
        Self {
            data: None,
            sender: None,
        }
    }
}

impl Battery {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns current battery data if available
    pub fn data(&self) -> Option<&BatteryData> {
        self.data.as_ref()
    }

    /// Registers module with event system
    pub fn register(&mut self, ctx: &ModuleContext) {
        self.sender = Some(ctx.module_sender(ModuleEvent::Battery));
    }

    /// Processes incoming messages from GUI layer
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Event(event) => self.handle_service_event(event),
        }
    }

    fn handle_service_event(&mut self, event: ServiceEvent<UPowerService>) {
        match event {
            ServiceEvent::Init(service) => {
                if let Some(battery) = service.battery {
                    self.update_battery_data(battery, service.power_profile.into());
                }
            }
            ServiceEvent::Update(update) => match update {
                UPowerEvent::UpdateBattery(battery) => {
                    let profile = self.data
                        .as_ref()
                        .map(|d| d.power_profile)
                        .unwrap_or_default();
                    self.update_battery_data(battery, profile);
                }
                UPowerEvent::NoBattery => {
                    self.data = None;
                }
                UPowerEvent::UpdatePowerProfile(profile) => {
                    if let Some(data) = &mut self.data {
                        data.power_profile = profile.into();
                        self.emit_event(BatteryEvent::ProfileChanged(profile.into()));
                    }
                }
            },
            ServiceEvent::Error(_) => {
                warn!("Failed to receive battery updates from UPower");
            }
        }
    }

    fn update_battery_data(&mut self, upower_data: UPowerBatteryData, power_profile: PowerProfile) {
        let capacity = upower_data.capacity;
        let charging = matches!(upower_data.state, crate::services::upower::State::Charging);

        let data = BatteryData::new(
            capacity,
            charging,
            None,
            power_profile,
        );

        if !charging {
            if capacity <= 5 {
                self.emit_event(BatteryEvent::CriticalBattery(capacity));
            } else if capacity <= 15 {
                self.emit_event(BatteryEvent::LowBattery(capacity));
            }
        }

        self.emit_event(BatteryEvent::StatusChanged(data.clone()));
        self.data = Some(data);
    }

    fn emit_event(&self, event: BatteryEvent) {
        if let Some(sender) = &self.sender {
            if let Err(e) = sender.try_send(event) {
                warn!("Failed to emit battery event: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_data_critical_state() {
        let data = BatteryData::new(5, false, None, PowerProfile::default());
        assert_eq!(data.indicator_state, IndicatorState::Danger);
    }

    #[test]
    fn battery_data_warning_state() {
        let data = BatteryData::new(15, false, None, PowerProfile::default());
        assert_eq!(data.indicator_state, IndicatorState::Warning);
    }

    #[test]
    fn battery_data_charging_success() {
        let data = BatteryData::new(50, true, None, PowerProfile::default());
        assert_eq!(data.indicator_state, IndicatorState::Success);
    }

    #[test]
    fn battery_icon_charging() {
        let data = BatteryData::new(50, true, None, PowerProfile::default());
        assert!(matches!(data.icon, BatteryIcon::Charging(50)));
    }

    #[test]
    fn battery_icon_discharging() {
        let data = BatteryData::new(75, false, None, PowerProfile::default());
        assert!(matches!(data.icon, BatteryIcon::Discharging(75)));
    }
}
