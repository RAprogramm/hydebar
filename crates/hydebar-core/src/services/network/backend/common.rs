use crate::services::network::{ConnectivityState, DeviceState};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Ethernet,
    Wifi,
    Bluetooth,
    TunTap,
    WireGuard,
    Generic,
    Other,
    #[default]
    Unknown,
}

impl From<u32> for DeviceType {
    fn from(device_type: u32) -> DeviceType {
        match device_type {
            1 => DeviceType::Ethernet,
            2 => DeviceType::Wifi,
            5 => DeviceType::Bluetooth,
            14 => DeviceType::Generic,
            16 => DeviceType::TunTap,
            29 => DeviceType::WireGuard,
            3..=32 => DeviceType::Other,
            _ => DeviceType::Unknown,
        }
    }
}

impl From<u32> for ConnectivityState {
    fn from(state: u32) -> ConnectivityState {
        match state {
            1 => ConnectivityState::None,
            2 => ConnectivityState::Portal,
            3 => ConnectivityState::Loss,
            4 => ConnectivityState::Full,
            _ => ConnectivityState::Unknown,
        }
    }
}

impl From<String> for ConnectivityState {
    fn from(state: String) -> ConnectivityState {
        match state.as_str() {
            "inactive" | "disconnected" => ConnectivityState::None,
            "portal" => ConnectivityState::Portal,
            "failed" => ConnectivityState::Loss,
            "connected" => ConnectivityState::Full,
            _ => ConnectivityState::Unknown,
        }
    }
}

impl From<Vec<ConnectivityState>> for ConnectivityState {
    fn from(states: Vec<ConnectivityState>) -> ConnectivityState {
        if states.is_empty() {
            return ConnectivityState::Unknown;
        }

        let mut state = states[0];
        for s in states.iter().skip(1) {
            if Into::<u32>::into(*s) >= state.into() {
                state = *s;
            }
        }

        state
    }
}

impl From<ConnectivityState> for u32 {
    fn from(val: ConnectivityState) -> Self {
        match val {
            ConnectivityState::None => 1,
            ConnectivityState::Portal => 2,
            ConnectivityState::Loss => 3,
            ConnectivityState::Full => 4,
            ConnectivityState::Unknown => 0,
        }
    }
}

impl From<u32> for DeviceState {
    fn from(device_state: u32) -> Self {
        match device_state {
            10 => DeviceState::Unmanaged,
            20 => DeviceState::Unavailable,
            30 => DeviceState::Disconnected,
            40 => DeviceState::Prepare,
            50 => DeviceState::Config,
            60 => DeviceState::NeedAuth,
            70 => DeviceState::IpConfig,
            80 => DeviceState::IpCheck,
            90 => DeviceState::Secondaries,
            100 => DeviceState::Activated,
            110 => DeviceState::Deactivating,
            120 => DeviceState::Failed,
            _ => DeviceState::Unknown,
        }
    }
}
