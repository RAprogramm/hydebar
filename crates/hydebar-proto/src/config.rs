use hex_color::HexColor;
use iced::{Color, theme::palette};
use regex::Regex;
use serde::{Deserialize, Deserializer, de::Visitor};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use thiserror::Error;

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

#[derive(Deserialize, Clone, Debug)]
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

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum AppearanceColor {
    Simple(HexColor),
    Complete {
        base: HexColor,
        strong: Option<HexColor>,
        weak: Option<HexColor>,
        text: Option<HexColor>,
    },
}

impl AppearanceColor {
    pub fn get_base(&self) -> Color {
        match self {
            AppearanceColor::Simple(color) => Color::from_rgb8(color.r, color.g, color.b),
            AppearanceColor::Complete { base, .. } => Color::from_rgb8(base.r, base.g, base.b),
        }
    }

    pub fn get_text(&self) -> Option<Color> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { text, .. } => {
                text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
            }
        }
    }

    pub fn get_weak_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { weak, text, .. } => weak.map(|color| {
                palette::Pair::new(
                    Color::from_rgb8(color.r, color.g, color.b),
                    text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
                        .unwrap_or(text_fallback),
                )
            }),
        }
    }

    pub fn get_strong_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete { strong, text, .. } => strong.map(|color| {
                palette::Pair::new(
                    Color::from_rgb8(color.r, color.g, color.b),
                    text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
                        .unwrap_or(text_fallback),
                )
            }),
        }
    }
}

#[derive(Deserialize, Default, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AppearanceStyle {
    #[default]
    Islands,
    Solid,
    Gradient,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct MenuAppearance {
    #[serde(deserialize_with = "opacity_deserializer", default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub backdrop: f32,
}

impl Default for MenuAppearance {
    fn default() -> Self {
        Self {
            opacity: default_opacity(),
            backdrop: f32::default(),
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Appearance {
    #[serde(default)]
    pub font_name: Option<String>,
    #[serde(
        deserialize_with = "scale_factor_deserializer",
        default = "default_scale_factor"
    )]
    pub scale_factor: f64,
    #[serde(default)]
    pub style: AppearanceStyle,
    #[serde(deserialize_with = "opacity_deserializer", default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub menu: MenuAppearance,
    #[serde(default = "default_background_color")]
    pub background_color: AppearanceColor,
    #[serde(default = "default_primary_color")]
    pub primary_color: AppearanceColor,
    #[serde(default = "default_secondary_color")]
    pub secondary_color: AppearanceColor,
    #[serde(default = "default_success_color")]
    pub success_color: AppearanceColor,
    #[serde(default = "default_danger_color")]
    pub danger_color: AppearanceColor,
    #[serde(default = "default_text_color")]
    pub text_color: AppearanceColor,
    #[serde(default = "default_workspace_colors")]
    pub workspace_colors: Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
}

static PRIMARY: HexColor = HexColor::rgb(250, 179, 135);

fn scale_factor_deserializer<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = f64::deserialize(deserializer)?;

    if v <= 0.0 {
        return Err(serde::de::Error::custom(
            "Scale factor must be greater than 0.0",
        ));
    }

    if v > 2.0 {
        return Err(serde::de::Error::custom(
            "Scale factor cannot be greater than 2.0",
        ));
    }

    Ok(v)
}

fn default_scale_factor() -> f64 {
    1.0
}

fn opacity_deserializer<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = f32::deserialize(deserializer)?;

    if v < 0.0 {
        return Err(serde::de::Error::custom("Opacity cannot be negative"));
    }

    if v > 1.0 {
        return Err(serde::de::Error::custom(
            "Opacity cannot be greater than 1.0",
        ));
    }

    Ok(v)
}

fn default_opacity() -> f32 {
    1.0
}

fn default_background_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base: HexColor::rgb(30, 30, 46),
        strong: Some(HexColor::rgb(69, 71, 90)),
        weak: Some(HexColor::rgb(49, 50, 68)),
        text: None,
    }
}

fn default_primary_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base: PRIMARY,
        strong: None,
        weak: None,
        text: Some(HexColor::rgb(30, 30, 46)),
    }
}

fn default_secondary_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base: HexColor::rgb(17, 17, 27),
        strong: Some(HexColor::rgb(24, 24, 37)),
        weak: None,
        text: None,
    }
}

fn default_success_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(166, 227, 161))
}

fn default_danger_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base: HexColor::rgb(243, 139, 168),
        weak: Some(HexColor::rgb(249, 226, 175)),
        strong: None,
        text: None,
    }
}

fn default_text_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(205, 214, 244))
}

fn default_workspace_colors() -> Vec<AppearanceColor> {
    vec![
        AppearanceColor::Simple(PRIMARY),
        AppearanceColor::Simple(HexColor::rgb(180, 190, 254)),
        AppearanceColor::Simple(HexColor::rgb(203, 166, 247)),
    ]
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font_name: None,
            scale_factor: 1.0,
            style: AppearanceStyle::default(),
            opacity: default_opacity(),
            menu: MenuAppearance::default(),
            background_color: default_background_color(),
            primary_color: default_primary_color(),
            secondary_color: default_secondary_color(),
            success_color: default_success_color(),
            danger_color: default_danger_color(),
            text_color: default_text_color(),
            workspace_colors: default_workspace_colors(),
            special_workspace_colors: None,
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModuleName {
    AppLauncher,
    Updates,
    Clipboard,
    Workspaces,
    WindowTitle,
    SystemInfo,
    KeyboardLayout,
    KeyboardSubmap,
    Tray,
    Clock,
    Battery,
    Privacy,
    Settings,
    MediaPlayer,
    Custom(String),
}

impl<'de> Deserialize<'de> for ModuleName {
    fn deserialize<D>(deserializer: D) -> Result<ModuleName, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModuleNameVisitor;
        impl Visitor<'_> for ModuleNameVisitor {
            type Value = ModuleName;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string representing a ModuleName")
            }
            fn visit_str<E>(self, value: &str) -> Result<ModuleName, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "AppLauncher" => ModuleName::AppLauncher,
                    "Updates" => ModuleName::Updates,
                    "Clipboard" => ModuleName::Clipboard,
                    "Workspaces" => ModuleName::Workspaces,
                    "WindowTitle" => ModuleName::WindowTitle,
                    "SystemInfo" => ModuleName::SystemInfo,
                    "KeyboardLayout" => ModuleName::KeyboardLayout,
                    "KeyboardSubmap" => ModuleName::KeyboardSubmap,
                    "Tray" => ModuleName::Tray,
                    "Clock" => ModuleName::Clock,
                    "Battery" => ModuleName::Battery,
                    "Privacy" => ModuleName::Privacy,
                    "Settings" => ModuleName::Settings,
                    "MediaPlayer" => ModuleName::MediaPlayer,
                    other => ModuleName::Custom(other.to_string()),
                })
            }
        }
        deserializer.deserialize_str(ModuleNameVisitor)
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum ModuleDef {
    Single(ModuleName),
    Group(Vec<ModuleName>),
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Modules {
    #[serde(default)]
    pub left: Vec<ModuleDef>,
    #[serde(default)]
    pub center: Vec<ModuleDef>,
    #[serde(default)]
    pub right: Vec<ModuleDef>,
}

impl Default for Modules {
    fn default() -> Self {
        Self {
            left: vec![ModuleDef::Single(ModuleName::Workspaces)],
            center: vec![ModuleDef::Single(ModuleName::WindowTitle)],
            right: vec![ModuleDef::Group(vec![
                ModuleName::Clock,
                ModuleName::Privacy,
                ModuleName::Battery,
                ModuleName::Settings,
            ])],
        }
    }
}

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub enum Outputs {
    #[default]
    All,
    Active,
    #[serde(deserialize_with = "non_empty")]
    Targets(Vec<String>),
}

fn non_empty<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let vec = <Vec<T>>::deserialize(d)?;
    if vec.is_empty() {
        use serde::de::Error;

        Err(D::Error::custom("need non-empty"))
    } else {
        Ok(vec)
    }
}

/// Newtype wrapper around `Regex`to be deserializable and usable as a hashmap key
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct RegexCfg(#[serde_as(as = "DisplayFromStr")] pub Regex);

impl PartialEq for RegexCfg {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}
impl Eq for RegexCfg {}

impl std::hash::Hash for RegexCfg {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // hash the raw pattern string
        self.0.as_str().hash(state);
    }
}

impl Deref for RegexCfg {
    type Target = Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
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

/// Errors returned when validating a [`Config`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigValidationError {
    /// Duplicate custom module definitions were found.
    #[error("duplicate custom module definition for '{name}'")]
    DuplicateCustomModule { name: String },

    /// A module references a custom module definition that does not exist.
    #[error("custom module '{name}' referenced in layout but not defined")]
    MissingCustomModule { name: String },
}

impl Config {
    /// Validates the configuration, ensuring module definitions are consistent.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigValidationError`] if duplicate custom modules are defined or if
    /// the module layout references undefined custom modules.
    ///
    /// # Examples
    ///
    /// ```
    /// use hydebar_proto::config::Config;
    ///
    /// let config = Config::default();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        let mut seen_custom_modules = HashSet::new();

        for module in &self.custom_modules {
            if !seen_custom_modules.insert(module.name.clone()) {
                return Err(ConfigValidationError::DuplicateCustomModule {
                    name: module.name.clone(),
                });
            }
        }

        let mut ensure_custom_module_exists = |name: &str| {
            if !seen_custom_modules.contains(name) {
                return Err(ConfigValidationError::MissingCustomModule {
                    name: name.to_owned(),
                });
            }

            Ok(())
        };

        for module_def in self
            .modules
            .left
            .iter()
            .chain(self.modules.center.iter())
            .chain(self.modules.right.iter())
        {
            match module_def {
                ModuleDef::Single(ModuleName::Custom(name)) => {
                    ensure_custom_module_exists(name)?;
                }
                ModuleDef::Group(group) => {
                    for module in group {
                        if let ModuleName::Custom(name) = module {
                            ensure_custom_module_exists(name)?;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom_module(name: &str) -> CustomModuleDef {
        CustomModuleDef {
            name: name.to_owned(),
            command: String::from("true"),
            icon: None,
            listen_cmd: None,
            icons: None,
            alert: None,
        }
    }

    #[test]
    fn validate_accepts_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_duplicate_custom_modules() {
        let mut config = Config::default();
        config.custom_modules = vec![custom_module("foo"), custom_module("foo")];

        let error = config
            .validate()
            .expect_err("expected duplicate module error");
        assert!(matches!(
            error,
            ConfigValidationError::DuplicateCustomModule { ref name } if name == "foo"
        ));
    }

    #[test]
    fn validate_rejects_missing_custom_module_reference() {
        let mut config = Config::default();
        config.custom_modules = vec![custom_module("foo")];
        config.modules.left = vec![ModuleDef::Single(ModuleName::Custom("bar".to_owned()))];

        let error = config
            .validate()
            .expect_err("expected missing module error");
        assert!(matches!(
            error,
            ConfigValidationError::MissingCustomModule { ref name } if name == "bar"
        ));
    }
}
