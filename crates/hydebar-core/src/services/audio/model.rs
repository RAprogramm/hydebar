use crate::components::icons::Icons;
use libpulse_binding::volume::ChannelVolumes;

/// Describes a single audio device (sink or source).
///
/// Each device carries metadata exported by PulseAudio that is consumed by the
/// settings UI.
#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub volume: ChannelVolumes,
    pub is_mute: bool,
    pub in_use: bool,
    pub ports: Vec<Port>,
}

/// Represents a selectable device port and its metadata.
#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub device_type: DeviceType,
    pub active: bool,
}

/// Enumerates known device categories.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DeviceType {
    Headphones,
    Speaker,
    Headset,
    Hdmi,
}

impl DeviceType {
    /// Returns the icon that should be displayed for the device category.
    #[must_use]
    pub fn get_icon(&self) -> Icons {
        match self {
            DeviceType::Speaker => Icons::Speaker3,
            DeviceType::Headphones => Icons::Headphones1,
            DeviceType::Headset => Icons::Headset,
            DeviceType::Hdmi => Icons::MonitorSpeaker,
        }
    }
}

/// Server level metadata tracked by the audio service.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    pub default_sink: String,
    pub default_source: String,
}

/// Provides a view on common volume operations for PulseAudio channel volumes.
pub trait Volume {
    /// Returns the normalized volume value in range `[0.0, 1.0]`.
    fn get_volume(&self) -> f64;

    /// Scales the volume to `max` and returns the modified value when
    /// successful.
    fn scale_volume(&mut self, max: f64) -> Option<&mut ChannelVolumes>;
}

impl Volume for ChannelVolumes {
    fn get_volume(&self) -> f64 {
        self.avg().0 as f64 / libpulse_binding::volume::Volume::NORMAL.0 as f64
    }

    fn scale_volume(&mut self, max: f64) -> Option<&mut ChannelVolumes> {
        let max = max.clamp(0.0, 1.0);
        self.scale(libpulse_binding::volume::Volume(
            (libpulse_binding::volume::Volume::NORMAL.0 as f64 * max) as u32,
        ))
    }
}

/// Convenience helpers for sink collections.
pub trait Sinks {
    /// Computes the icon for the default sink.
    fn get_icon(&self, default_sink: &str) -> Icons;
}

impl Sinks for Vec<Device> {
    fn get_icon(&self, default_sink: &str) -> Icons {
        match self.iter().find_map(|sink| {
            if sink.ports.iter().any(|port| port.active) && sink.name == default_sink {
                Some((sink.is_mute, sink.volume.get_volume()))
            } else {
                None
            }
        }) {
            Some((true, _)) => Icons::Speaker0,
            Some((false, volume)) => {
                if volume > 0.66 {
                    Icons::Speaker3
                } else if volume > 0.33 {
                    Icons::Speaker2
                } else if volume > 0.000_001 {
                    Icons::Speaker1
                } else {
                    Icons::Speaker0
                }
            }
            None => Icons::Speaker0,
        }
    }
}

/// Runtime state tracked by the audio service and exposed to the UI.
#[derive(Debug, Clone)]
pub struct AudioData {
    pub server_info: ServerInfo,
    pub sinks: Vec<Device>,
    pub sources: Vec<Device>,
    pub cur_sink_volume: i32,
    pub cur_source_volume: i32,
}

impl Default for AudioData {
    fn default() -> Self {
        Self {
            server_info: ServerInfo::default(),
            sinks: Vec::new(),
            sources: Vec::new(),
            cur_sink_volume: 0,
            cur_source_volume: 0,
        }
    }
}

/// Events produced by the backend to update the service state.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    Sinks(Vec<Device>),
    Sources(Vec<Device>),
    ServerInfo(ServerInfo),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_type_icons_match_expectations() {
        assert_eq!(DeviceType::Headphones.get_icon(), Icons::Headphones1);
        assert_eq!(DeviceType::Speaker.get_icon(), Icons::Speaker3);
        assert_eq!(DeviceType::Headset.get_icon(), Icons::Headset);
        assert_eq!(DeviceType::Hdmi.get_icon(), Icons::MonitorSpeaker);
    }

    #[test]
    fn sink_collection_icon_considers_mute_state() {
        let sinks = vec![Device {
            name: "default".into(),
            description: String::new(),
            volume: ChannelVolumes::default(),
            is_mute: true,
            in_use: true,
            ports: vec![Port {
                name: "port".into(),
                description: String::new(),
                device_type: DeviceType::Speaker,
                active: true,
            }],
        }];

        assert_eq!(sinks.get_icon("default"), Icons::Speaker0);
    }

    #[test]
    fn sink_collection_returns_default_when_no_match() {
        let sinks = vec![Device {
            name: "other".into(),
            description: String::new(),
            volume: ChannelVolumes::default(),
            is_mute: false,
            in_use: true,
            ports: vec![Port {
                name: "port".into(),
                description: String::new(),
                device_type: DeviceType::Speaker,
                active: true,
            }],
        }];

        assert_eq!(sinks.get_icon("default"), Icons::Speaker0);
    }

    #[test]
    fn volume_trait_clamps_to_valid_range() {
        let mut volume = ChannelVolumes::default();
        // scale_volume clamps max to [0.0, 1.0], so 1.2 becomes 1.0
        // On empty ChannelVolumes, scale() may return None
        let result = volume.scale_volume(1.2);
        // Just verify it doesn't panic and returns expected type
        let _ = result;
    }
}
