mod appearance;
mod modules;
mod serde_helpers;
mod validation;

pub use appearance::{Appearance, AppearanceColor, AppearanceStyle, MenuAppearance};
pub use modules::{ModuleDef, ModuleName, Modules, Outputs, Position};
pub use serde_helpers::RegexCfg;
pub use validation::ConfigValidationError;

use serde::Deserialize;
use serde_with::serde_as;
use std::collections::HashMap;

pub const DEFAULT_CONFIG_FILE_PATH: &str = "~/.config/hydebar/config.toml";

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UpdatesModuleConfig {
    pub check_cmd: String,
    pub update_cmd: String,
}

#[derive(Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub enum WorkspaceVisibilityMode {
    #[default]
    All,
    MonitorSpecific,
}

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct WorkspacesModuleConfig {
    #[serde(default)]
    pub visibility_mode: WorkspaceVisibilityMode,
    #[serde(default)]
    pub enable_workspace_filling: bool,
    pub max_workspaces: Option<u32>,
}

#[derive(Deserialize, Clone, Default, PartialEq, Eq, Debug)]
pub enum WindowTitleMode {
    #[default]
    Title,
    Class,
}

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct WindowTitleConfig {
    #[serde(default)]
    pub mode: WindowTitleMode,
    #[serde(default = "default_truncate_title_after_length")]
    pub truncate_title_after_length: u32,
}

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct KeyboardLayoutModuleConfig {
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoCpu {
    #[serde(default = "default_cpu_warn_threshold")]
    pub warn_threshold: u32,
    #[serde(default = "default_cpu_alert_threshold")]
    pub alert_threshold: u32,
}

impl Default for SystemInfoCpu {
    fn default() -> Self {
        Self {
            warn_threshold: default_cpu_warn_threshold(),
            alert_threshold: default_cpu_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoMemory {
    #[serde(default = "default_mem_warn_threshold")]
    pub warn_threshold: u32,
    #[serde(default = "default_mem_alert_threshold")]
    pub alert_threshold: u32,
}

impl Default for SystemInfoMemory {
    fn default() -> Self {
        Self {
            warn_threshold: default_mem_warn_threshold(),
            alert_threshold: default_mem_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoTemperature {
    #[serde(default = "default_temp_warn_threshold")]
    pub warn_threshold: i32,
    #[serde(default = "default_temp_alert_threshold")]
    pub alert_threshold: i32,
}

impl Default for SystemInfoTemperature {
    fn default() -> Self {
        Self {
            warn_threshold: default_temp_warn_threshold(),
            alert_threshold: default_temp_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemInfoDisk {
    #[serde(default = "default_disk_warn_threshold")]
    pub warn_threshold: u32,
    #[serde(default = "default_disk_alert_threshold")]
    pub alert_threshold: u32,
}

impl Default for SystemInfoDisk {
    fn default() -> Self {
        Self {
            warn_threshold: default_disk_warn_threshold(),
            alert_threshold: default_disk_alert_threshold(),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SystemIndicator {
    Cpu,
    Memory,
    MemorySwap,
    Temperature,
    Disk(String),
    IpAddress,
    DownloadSpeed,
    UploadSpeed,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SystemModuleConfig {
    #[serde(default = "default_system_indicators")]
    pub indicators: Vec<SystemIndicator>,
    #[serde(default)]
    pub cpu: SystemInfoCpu,
    #[serde(default)]
    pub memory: SystemInfoMemory,
    #[serde(default)]
    pub temperature: SystemInfoTemperature,
    #[serde(default)]
    pub disk: SystemInfoDisk,
}

fn default_system_indicators() -> Vec<SystemIndicator> {
    vec![
        SystemIndicator::Cpu,
        SystemIndicator::Memory,
        SystemIndicator::Temperature,
    ]
}

fn default_cpu_warn_threshold() -> u32 {
    60
}

fn default_cpu_alert_threshold() -> u32 {
    80
}

fn default_mem_warn_threshold() -> u32 {
    70
}

fn default_mem_alert_threshold() -> u32 {
    85
}

fn default_temp_warn_threshold() -> i32 {
    60
}

fn default_temp_alert_threshold() -> i32 {
    80
}

fn default_disk_warn_threshold() -> u32 {
    80
}

fn default_disk_alert_threshold() -> u32 {
    90
}

impl Default for SystemModuleConfig {
    fn default() -> Self {
        Self {
            indicators: default_system_indicators(),
            cpu: SystemInfoCpu::default(),
            memory: SystemInfoMemory::default(),
            temperature: SystemInfoTemperature::default(),
            disk: SystemInfoDisk::default(),
        }
    }
}

/// Configuration for the battery module.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BatteryModuleConfig {
    #[serde(default = "default_show_percentage")]
    pub show_percentage: bool,
    #[serde(default = "default_show_power_profile")]
    pub show_power_profile: bool,
    #[serde(default = "default_open_settings_on_click")]
    pub open_settings_on_click: bool,
    #[serde(default)]
    pub show_when_unavailable: bool,
}

impl Default for BatteryModuleConfig {
    fn default() -> Self {
        Self {
            show_percentage: default_show_percentage(),
            show_power_profile: default_show_power_profile(),
            open_settings_on_click: default_open_settings_on_click(),
            show_when_unavailable: false,
        }
    }
}

fn default_show_percentage() -> bool {
    true
}

fn default_show_power_profile() -> bool {
    true
}

fn default_open_settings_on_click() -> bool {
    true
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClockModuleConfig {
    pub format: String,
}

impl Default for ClockModuleConfig {
    fn default() -> Self {
        Self {
            format: "%a %d %b %R".to_string(),
        }
    }
}

fn default_shutdown_cmd() -> String {
    "shutdown now".to_string()
}

fn default_suspend_cmd() -> String {
    "systemctl suspend".to_string()
}

fn default_reboot_cmd() -> String {
    "systemctl reboot".to_string()
}

fn default_logout_cmd() -> String {
    "loginctl kill-user $(whoami)".to_string()
}

#[derive(Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct SettingsModuleConfig {
    pub lock_cmd: Option<String>,
    #[serde(default = "default_shutdown_cmd")]
    pub shutdown_cmd: String,
    #[serde(default = "default_suspend_cmd")]
    pub suspend_cmd: String,
    #[serde(default = "default_reboot_cmd")]
    pub reboot_cmd: String,
    #[serde(default = "default_logout_cmd")]
    pub logout_cmd: String,
    pub audio_sinks_more_cmd: Option<String>,
    pub audio_sources_more_cmd: Option<String>,
    pub wifi_more_cmd: Option<String>,
    pub vpn_more_cmd: Option<String>,
    pub bluetooth_more_cmd: Option<String>,
    #[serde(default)]
    pub remove_airplane_btn: bool,
    #[serde(default)]
    pub remove_idle_btn: bool,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MediaPlayerModuleConfig {
    #[serde(default = "default_media_player_max_title_length")]
    pub max_title_length: u32,
}

impl Default for MediaPlayerModuleConfig {
    fn default() -> Self {
        MediaPlayerModuleConfig {
            max_title_length: default_media_player_max_title_length(),
        }
    }
}

fn default_media_player_max_title_length() -> u32 {
    100
}

#[serde_as]
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CustomModuleDef {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub icon: Option<String>,

    /// yields json lines containing text, alt, (pot tooltip)
    pub listen_cmd: Option<String>,
    /// map of regex -> icon
    pub icons: Option<HashMap<RegexCfg, String>>,
    /// regex to show alert
    pub alert: Option<RegexCfg>,
    // .. appearance etc
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub outputs: Outputs,
    #[serde(default)]
    pub modules: Modules,
    pub app_launcher_cmd: Option<String>,
    #[serde(rename = "CustomModule", default)]
    pub custom_modules: Vec<CustomModuleDef>,
    pub clipboard_cmd: Option<String>,
    #[serde(default)]
    pub updates: Option<UpdatesModuleConfig>,
    #[serde(default)]
    pub workspaces: WorkspacesModuleConfig,
    #[serde(default)]
    pub window_title: WindowTitleConfig,
    #[serde(default)]
    pub system: SystemModuleConfig,
    #[serde(default)]
    pub battery: BatteryModuleConfig,
    #[serde(default)]
    pub clock: ClockModuleConfig,
    #[serde(default)]
    pub settings: SettingsModuleConfig,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub media_player: MediaPlayerModuleConfig,
    #[serde(default)]
    pub keyboard_layout: KeyboardLayoutModuleConfig,
    #[serde(default)]
    pub menu_keyboard_focus: bool,
}

fn default_log_level() -> String {
    "warn".to_owned()
}

fn default_menu_keyboard_focus() -> bool {
    true
}

fn default_truncate_title_after_length() -> u32 {
    150
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            position: Position::Top,
            outputs: Outputs::default(),
            modules: Modules::default(),
            app_launcher_cmd: None,
            clipboard_cmd: None,
            updates: None,
            workspaces: WorkspacesModuleConfig::default(),
            window_title: WindowTitleConfig::default(),
            system: SystemModuleConfig::default(),
            battery: BatteryModuleConfig::default(),
            clock: ClockModuleConfig::default(),
            settings: SettingsModuleConfig::default(),
            appearance: Appearance::default(),
            media_player: MediaPlayerModuleConfig::default(),
            keyboard_layout: KeyboardLayoutModuleConfig::default(),
            custom_modules: vec![],
            menu_keyboard_focus: default_menu_keyboard_focus(),
        }
    }
}
