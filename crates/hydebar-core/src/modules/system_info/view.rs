use crate::{
    app,
    components::icons::{Icons, icon},
    config::{SystemIndicator, SystemModuleConfig},
    menu::MenuType,
    modules::OnModulePress,
};
use iced::{
    Alignment, Element, Length, Theme,
    widget::{Column, Row, column, container, horizontal_rule, row, text},
};

use super::{Message, data::SystemInfoData};

fn info_element<'a>(info_icon: Icons, label: &str, value: String) -> Element<'a, Message> {
    row!(
        container(icon(info_icon).size(22)).center_x(Length::Fixed(32.)),
        text(label).width(Length::Fill),
        text(value)
    )
    .align_y(Alignment::Center)
    .spacing(8)
    .into()
}

fn indicator_info_element<'a, V>(
    info_icon: Icons,
    value: V,
    unit: &str,
    threshold: Option<(V, V)>,
    prefix: Option<&str>,
) -> Element<'a, app::Message>
where
    V: std::fmt::Display + PartialOrd + Copy + 'a,
{
    let content = container(
        row!(
            icon(info_icon),
            if let Some(prefix) = prefix {
                text(format!("{prefix} {value}{unit}"))
            } else {
                text(format!("{value}{unit}"))
            }
        )
        .spacing(4),
    );

    if let Some((warn_threshold, alert_threshold)) = threshold {
        content
            .style(move |theme: &Theme| container::Style {
                text_color: if value > warn_threshold && value < alert_threshold {
                    Some(theme.extended_palette().danger.weak.color)
                } else if value >= alert_threshold {
                    Some(theme.palette().danger)
                } else {
                    None
                },
                ..Default::default()
            })
            .into()
    } else {
        content.into()
    }
}

fn format_speed(speed: u32) -> (u32, &'static str) {
    if speed > 1000 {
        (speed / 1000, "MB/s")
    } else {
        (speed, "KB/s")
    }
}

/// Render the module menu displaying detailed system metrics.
pub fn build_menu_view(data: &SystemInfoData) -> Element<Message> {
    column![
        text("System Info").size(20),
        horizontal_rule(1),
        Column::new()
            .push(info_element(
                Icons::Cpu,
                "CPU Usage",
                format!("{}%", data.cpu_usage)
            ))
            .push(info_element(
                Icons::Mem,
                "Memory Usage",
                format!("{}%", data.memory_usage)
            ))
            .push(info_element(
                Icons::Mem,
                "Swap memory Usage",
                format!("{}%", data.memory_swap_usage),
            ))
            .push_maybe(
                data.temperature
                    .map(|temp| { info_element(Icons::Temp, "Temperature", format!("{temp}°C")) })
            )
            .push(
                Column::with_children(
                    data.disks
                        .iter()
                        .map(|(mount_point, usage)| {
                            info_element(
                                Icons::Drive,
                                &format!("Disk Usage {mount_point}"),
                                format!("{usage}%"),
                            )
                        })
                        .collect::<Vec<Element<_>>>(),
                )
                .spacing(4),
            )
            .push_maybe(data.network.as_ref().map(|network| {
                let (download_value, download_unit) = format_speed(network.download_speed);
                let (upload_value, upload_unit) = format_speed(network.upload_speed);

                Column::with_children(vec![
                    info_element(Icons::IpAddress, "IP Address", network.ip.clone()),
                    info_element(
                        Icons::DownloadSpeed,
                        "Download Speed",
                        format!("{download_value} {download_unit}"),
                    ),
                    info_element(
                        Icons::UploadSpeed,
                        "Upload Speed",
                        format!("{upload_value} {upload_unit}"),
                    ),
                ])
            }))
            .spacing(4)
            .padding([0, 8])
    ]
    .spacing(8)
    .into()
}

/// Build the indicator widgets representing the configured subset of metrics.
pub fn indicator_elements(
    data: &SystemInfoData,
    config: &SystemModuleConfig,
) -> Vec<Element<app::Message>> {
    config
        .indicators
        .iter()
        .filter_map(|indicator| match indicator {
            SystemIndicator::Cpu => Some(indicator_info_element(
                Icons::Cpu,
                data.cpu_usage,
                "%",
                Some((config.cpu.warn_threshold, config.cpu.alert_threshold)),
                None,
            )),
            SystemIndicator::Memory => Some(indicator_info_element(
                Icons::Mem,
                data.memory_usage,
                "%",
                Some((config.memory.warn_threshold, config.memory.alert_threshold)),
                None,
            )),
            SystemIndicator::MemorySwap => Some(indicator_info_element(
                Icons::Mem,
                data.memory_swap_usage,
                "%",
                Some((config.memory.warn_threshold, config.memory.alert_threshold)),
                Some("swap"),
            )),
            SystemIndicator::Temperature => data.temperature.map(|temperature| {
                indicator_info_element(
                    Icons::Temp,
                    temperature,
                    "°C",
                    Some((
                        config.temperature.warn_threshold,
                        config.temperature.alert_threshold,
                    )),
                    None,
                )
            }),
            SystemIndicator::Disk(mount) => data.disks.iter().find_map(|(disk_mount, disk)| {
                if disk_mount == mount {
                    Some(indicator_info_element(
                        Icons::Drive,
                        *disk,
                        "%",
                        Some((config.disk.warn_threshold, config.disk.alert_threshold)),
                        Some(disk_mount.as_str()),
                    ))
                } else {
                    None
                }
            }),
            SystemIndicator::IpAddress => data.network.as_ref().map(|network| {
                indicator_info_element(Icons::IpAddress, network.ip.as_str(), "", None, None)
            }),
            SystemIndicator::DownloadSpeed => data.network.as_ref().map(|network| {
                let (value, unit) = format_speed(network.download_speed);
                indicator_info_element(Icons::DownloadSpeed, value, unit, None, None)
            }),
            SystemIndicator::UploadSpeed => data.network.as_ref().map(|network| {
                let (value, unit) = format_speed(network.upload_speed);
                indicator_info_element(Icons::UploadSpeed, value, unit, None, None)
            }),
        })
        .collect()
}

/// Construct the condensed indicator row shown in the module section.
pub fn build_indicator_view(
    data: &SystemInfoData,
    config: &SystemModuleConfig,
) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
    let indicators = indicator_elements(data, config);

    Some((
        Row::with_children(indicators)
            .align_y(Alignment::Center)
            .spacing(4)
            .into(),
        Some(OnModulePress::ToggleMenu(MenuType::SystemInfo)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DiskIndicatorConfig, MemoryIndicatorConfig, TemperatureIndicatorConfig};

    fn data_fixture() -> SystemInfoData {
        SystemInfoData {
            cpu_usage: 25,
            memory_usage: 50,
            memory_swap_usage: 10,
            temperature: Some(42),
            disks: vec![("/".to_string(), 60)],
            network: None,
        }
    }

    #[test]
    fn indicator_row_contains_configured_entries() {
        let data = data_fixture();
        let config = SystemModuleConfig {
            indicators: vec![SystemIndicator::Cpu, SystemIndicator::Memory],
            cpu: Default::default(),
            memory: MemoryIndicatorConfig {
                warn_threshold: 70,
                alert_threshold: 90,
            },
            temperature: TemperatureIndicatorConfig {
                warn_threshold: 70,
                alert_threshold: 90,
            },
            disk: Default::default(),
        };

        let indicators = indicator_elements(&data, &config);
        assert_eq!(indicators.len(), 2);
    }

    #[test]
    fn indicator_elements_include_network_entries_when_available() {
        let mut data = data_fixture();
        data.network = Some(crate::modules::system_info::NetworkData::new(
            "127.0.0.1".to_string(),
            2048,
            1024,
            std::time::Instant::now(),
        ));

        let config = SystemModuleConfig {
            indicators: vec![SystemIndicator::IpAddress, SystemIndicator::DownloadSpeed],
            ..SystemModuleConfig::default()
        };

        let indicators = indicator_elements(&data, &config);
        assert_eq!(indicators.len(), 2);
    }

    #[test]
    fn format_speed_converts_large_values_to_megabytes() {
        let (value, unit) = format_speed(2048);
        assert_eq!((value, unit), (2, "MB/s"));
    }
}
