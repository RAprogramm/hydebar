use std::time::Instant;

use itertools::Itertools;
use sysinfo::{Components, Disks, Networks, System};

/// Snapshot of network utilisation metrics captured during sampling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkData {
    pub ip:             String,
    pub download_speed: u32,
    pub upload_speed:   u32,
    last_check:         Instant
}

impl NetworkData {
    /// Create a new network metric snapshot with the provided parameters.
    pub fn new(ip: String, download_speed: u32, upload_speed: u32, last_check: Instant) -> Self {
        Self {
            ip,
            download_speed,
            upload_speed,
            last_check
        }
    }

    /// Instant when the underlying network totals were observed.
    pub fn last_check(&self) -> Instant {
        self.last_check
    }
}

/// Aggregated system information consumed by the UI layer.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemInfoData {
    pub cpu_usage:         u32,
    pub memory_usage:      u32,
    pub memory_swap_usage: u32,
    pub temperature:       Option<i32>,
    pub disks:             Vec<(String, u32)>,
    pub network:           Option<NetworkData>
}

#[derive(Debug, Clone)]
struct NetworkSnapshot {
    ip:                Option<String>,
    total_received:    u64,
    total_transmitted: u64,
    timestamp:         Instant
}

impl NetworkSnapshot {
    fn capture(networks: &Networks, now: Instant) -> Option<Self> {
        let (ip, total_received, total_transmitted) = networks.iter().fold(
            (None, 0_u64, 0_u64),
            |(first_ip, received, transmitted), (_, data)| {
                let next_ip = first_ip.or_else(|| {
                    data.ip_networks()
                        .iter()
                        .sorted_by(|a, b| a.addr.cmp(&b.addr))
                        .next()
                        .map(|ip| ip.addr.to_string())
                });

                (
                    next_ip,
                    received + data.received(),
                    transmitted + data.transmitted()
                )
            }
        );

        let ip = ip?;

        Some(Self {
            ip: Some(ip),
            total_received,
            total_transmitted,
            timestamp: now
        })
    }

    fn to_data(&self, previous: Option<&NetworkSnapshot>) -> NetworkData {
        let elapsed = previous
            .map(|snapshot| self.timestamp.saturating_duration_since(snapshot.timestamp))
            .unwrap_or_default();
        let seconds = elapsed.as_secs();

        let compute_speed = |current: u64, previous_total: u64| -> u32 {
            if seconds == 0 {
                return 0;
            }

            let delta = current.saturating_sub(previous_total);
            ((delta / 1000) as u32) / (seconds as u32)
        };

        NetworkData {
            ip:             self.ip.clone().unwrap_or_else(|| "Unknown".to_string()),
            download_speed: compute_speed(
                self.total_received,
                previous.map_or(0, |snapshot| snapshot.total_received)
            ),
            upload_speed:   compute_speed(
                self.total_transmitted,
                previous.map_or(0, |snapshot| snapshot.total_transmitted)
            ),
            last_check:     self.timestamp
        }
    }
}

/// Samples system metrics using the [`sysinfo`] crate.
#[derive(Debug)]
pub struct SystemInfoSampler {
    system:       System,
    components:   Components,
    disks:        Disks,
    networks:     Networks,
    last_network: Option<NetworkSnapshot>
}

impl Default for SystemInfoSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemInfoSampler {
    /// Instantiate a sampler with refreshed sysinfo collections.
    pub fn new() -> Self {
        Self {
            system:       System::new(),
            components:   Components::new_with_refreshed_list(),
            disks:        Disks::new_with_refreshed_list(),
            networks:     Networks::new_with_refreshed_list(),
            last_network: None
        }
    }

    /// Capture the latest system metrics, updating internal state for
    /// subsequent samples.
    pub fn sample(&mut self) -> SystemInfoData {
        self.system
            .refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
        self.system.refresh_memory();
        self.components.refresh(true);
        self.disks.refresh(true);
        self.networks.refresh(true);

        let now = Instant::now();
        let observation = NetworkSnapshot::capture(&self.networks, now);
        let network = observation
            .as_ref()
            .map(|snapshot| snapshot.to_data(self.last_network.as_ref()));
        self.last_network = observation;

        let cpu_usage = self.system.global_cpu_usage().floor() as u32;
        let memory_usage = percentage(
            self.system
                .total_memory()
                .saturating_sub(self.system.available_memory()),
            self.system.total_memory()
        );
        let memory_swap_usage = percentage(
            self.system
                .total_swap()
                .saturating_sub(self.system.free_swap()),
            self.system.total_swap()
        );

        let temperature = self
            .components
            .iter()
            .find(|component| component.label() == "acpitz temp1")
            .and_then(|component| component.temperature().map(|value| value as i32));

        let disks = self
            .disks
            .iter()
            .filter(|disk| !disk.is_removable() && disk.total_space() != 0)
            .map(|disk| {
                let mount_point = disk.mount_point().to_string_lossy().to_string();
                let usage = percentage(
                    disk.total_space().saturating_sub(disk.available_space()),
                    disk.total_space()
                );

                (mount_point, usage)
            })
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .collect();

        SystemInfoData {
            cpu_usage,
            memory_usage,
            memory_swap_usage,
            temperature,
            disks,
            network
        }
    }
}

fn percentage(used: u64, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }

    ((used as f32 / total as f32) * 100.) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_speed_zero_when_no_elapsed() {
        let timestamp = Instant::now();
        let previous = NetworkSnapshot {
            ip: Some("127.0.0.1".to_string()),
            total_received: 1024,
            total_transmitted: 2048,
            timestamp
        };
        let snapshot = NetworkSnapshot {
            ip: Some("127.0.0.1".to_string()),
            total_received: 2048,
            total_transmitted: 4096,
            timestamp
        };

        let data = snapshot.to_data(Some(&previous));

        assert_eq!(data.download_speed, 0);
        assert_eq!(data.upload_speed, 0);
    }

    #[test]
    fn percentage_handles_zero_total() {
        assert_eq!(percentage(5, 0), 0);
    }

    #[test]
    fn sampler_produces_data() {
        let mut sampler = SystemInfoSampler::new();
        let data = sampler.sample();

        assert!(data.cpu_usage <= 100);
        assert!(data.memory_usage <= 100);
        assert!(data.memory_swap_usage <= 100);
    }
}
