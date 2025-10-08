use std::time::Duration;

use log::warn;

use crate::{
    ModuleContext,
    components::icons::Icons,
    services::{
        ServiceEvent,
        upower::{BatteryData as UPowerBatteryData, UPowerEvent, UPowerService},
    },
};

/// Battery icon type based on capacity and charging state
#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
pub enum BatteryIcon
{
    Charging(u8,),
    Discharging(u8,),
    Full,
    Unknown,
}

impl From<BatteryIcon,> for Icons
{
    fn from(icon: BatteryIcon,) -> Self
    {
        match icon {
            BatteryIcon::Charging(_,) => Icons::BatteryCharging,
            BatteryIcon::Discharging(capacity,) => match capacity {
                0..=20 => Icons::Battery0,
                21..=40 => Icons::Battery1,
                41..=60 => Icons::Battery2,
                61..=80 => Icons::Battery3,
                _ => Icons::Battery4,
            },
            BatteryIcon::Full => Icons::Battery4,
            BatteryIcon::Unknown => Icons::Battery0,
        }
    }
}

/// Power management profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default,)]
pub enum PowerProfile
{
    #[default]
    Balanced,
    Performance,
    PowerSaver,
    Unknown,
}

impl From<crate::services::upower::PowerProfile,> for PowerProfile
{
    fn from(profile: crate::services::upower::PowerProfile,) -> Self
    {
        match profile {
            crate::services::upower::PowerProfile::PowerSaver => PowerProfile::PowerSaver,
            crate::services::upower::PowerProfile::Balanced => PowerProfile::Balanced,
            crate::services::upower::PowerProfile::Performance => PowerProfile::Performance,
            crate::services::upower::PowerProfile::Unknown => PowerProfile::Unknown,
        }
    }
}

impl From<PowerProfile,> for Icons
{
    fn from(profile: PowerProfile,) -> Self
    {
        match profile {
            PowerProfile::Performance => Icons::Performance,
            PowerProfile::Balanced => Icons::Balanced,
            PowerProfile::PowerSaver => Icons::PowerSaver,
            PowerProfile::Unknown => Icons::Balanced,
        }
    }
}

/// Visual indicator state for battery status
#[derive(Debug, Clone, Copy, PartialEq, Eq,)]
pub enum IndicatorState
{
    Normal,
    Warning,
    Danger,
    Success,
}

/// Complete battery state information for rendering
#[derive(Debug, Clone,)]
pub struct BatteryData
{
    pub capacity:        u8,
    pub charging:        bool,
    pub icon:            BatteryIcon,
    pub time_remaining:  Option<Duration,>,
    pub power_profile:   PowerProfile,
    pub indicator_state: IndicatorState,
}

impl BatteryData
{
    pub fn new(
        capacity: u8,
        charging: bool,
        time_remaining: Option<Duration,>,
        power_profile: PowerProfile,
    ) -> Self
    {
        let icon = if charging {
            if capacity >= 100 { BatteryIcon::Full } else { BatteryIcon::Charging(capacity,) }
        } else {
            BatteryIcon::Discharging(capacity,)
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
#[derive(Debug, Clone,)]
pub enum BatteryEvent
{
    StatusChanged(BatteryData,),
    ProfileChanged(PowerProfile,),
    LowBattery(u8,),
    CriticalBattery(u8,),
}

/// Message type for GUI communication
#[derive(Debug, Clone,)]
pub enum Message
{
    Event(ServiceEvent<UPowerService,>,),
}

/// Battery monitoring module
#[derive(Debug, Default,)]
pub struct Battery
{
    data: Option<BatteryData,>,
    // sender: Option<ModuleEventSender<BatteryEvent>>, // Unused - battery events not sent to UI
}

impl Battery
{
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Returns current battery data if available
    pub fn data(&self,) -> Option<&BatteryData,>
    {
        self.data.as_ref()
    }

    /// Registers module with event system
    pub fn register(&mut self, _ctx: &ModuleContext,)
    {
        // BatteryEvent is not used for UI updates, Battery module only
        // subscribes to service events
    }

    /// Processes incoming messages from GUI layer
    pub fn update(&mut self, message: Message,)
    {
        match message {
            Message::Event(event,) => self.handle_service_event(event,),
        }
    }

    fn handle_service_event(&mut self, event: ServiceEvent<UPowerService,>,)
    {
        match event {
            ServiceEvent::Init(service,) => {
                if let Some(battery,) = service.battery {
                    self.update_battery_data(battery, service.power_profile.into(),);
                }
            }
            ServiceEvent::Update(update,) => match update {
                UPowerEvent::UpdateBattery(battery,) => {
                    let profile = self.data.as_ref().map(|d| d.power_profile,).unwrap_or_default();
                    self.update_battery_data(battery, profile,);
                }
                UPowerEvent::NoBattery => {
                    self.data = None;
                }
                UPowerEvent::UpdatePowerProfile(profile,) => {
                    if let Some(data,) = &mut self.data {
                        data.power_profile = profile.into();
                    }
                }
            },
            ServiceEvent::Error(_,) => {
                warn!("Failed to receive battery updates from UPower");
            }
        }
    }

    fn update_battery_data(&mut self, upower_data: UPowerBatteryData, power_profile: PowerProfile,)
    {
        let capacity = upower_data.capacity.clamp(0, 100,) as u8;
        let charging =
            matches!(upower_data.status, crate::services::upower::BatteryStatus::Charging(_));

        let data = BatteryData::new(capacity, charging, None, power_profile,);

        // Battery events are not currently sent to the UI
        // Notification logic could be added here in the future
        // if !charging {
        //     if capacity <= 5 {
        //         // Critical battery notification
        //     } else if capacity <= 15 {
        //         // Low battery notification
        //     }
        // }

        self.data = Some(data,);
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn battery_data_critical_state()
    {
        let data = BatteryData::new(5, false, None, PowerProfile::default(),);
        assert_eq!(data.indicator_state, IndicatorState::Danger);
    }

    #[test]
    fn battery_data_warning_state()
    {
        let data = BatteryData::new(15, false, None, PowerProfile::default(),);
        assert_eq!(data.indicator_state, IndicatorState::Warning);
    }

    #[test]
    fn battery_data_charging_success()
    {
        let data = BatteryData::new(50, true, None, PowerProfile::default(),);
        assert_eq!(data.indicator_state, IndicatorState::Success);
    }

    #[test]
    fn battery_icon_charging()
    {
        let data = BatteryData::new(50, true, None, PowerProfile::default(),);
        assert!(matches!(data.icon, BatteryIcon::Charging(50)));
    }

    #[test]
    fn battery_icon_discharging()
    {
        let data = BatteryData::new(75, false, None, PowerProfile::default(),);
        assert!(matches!(data.icon, BatteryIcon::Discharging(75)));
    }
}
