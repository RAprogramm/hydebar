use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::{Arc, RwLock}
};

use hydebar_proto::config::{Config, ConfigValidationError, CustomModuleDef, ModuleName};

/// Represents the effect a configuration update has on the running system.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigImpact {
    /// Modules whose configuration changed and may require additional handling.
    pub affected_modules:       BTreeSet<ModuleName>,
    /// Whether the module layout changed.
    pub layout_changed:         bool,
    /// Whether appearance settings changed.
    pub appearance_changed:     bool,
    /// Whether output targeting changed.
    pub outputs_changed:        bool,
    /// Whether the bar position changed.
    pub position_changed:       bool,
    /// Whether the log level changed.
    pub log_level_changed:      bool,
    /// Whether menu keyboard focus changed.
    pub menu_focus_changed:     bool,
    /// Whether custom module definitions changed.
    pub custom_modules_changed: bool
}

impl ConfigImpact {
    /// Returns `true` if the given module is listed as affected by the update.
    pub fn affects_module(&self, module: &ModuleName) -> bool {
        self.affected_modules.contains(module)
    }
}

/// Applied configuration along with its computed impact.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigApplied {
    /// The fully validated configuration that was applied.
    pub config: Arc<Config>,
    /// The impact of applying the configuration.
    pub impact: ConfigImpact
}

/// Describes failures that occurred while attempting to refresh the
/// configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigUpdateError {
    /// Reading the configuration file from disk failed.
    Read { path: PathBuf, context: String },
    /// Parsing TOML content failed.
    Parse { path: PathBuf, context: String },
    /// Validation detected a logical inconsistency.
    Validation(ConfigValidationError),
    /// The configuration file was removed.
    Removed,
    /// Updating the configuration state failed for an internal reason.
    State { context: String }
}

impl std::fmt::Display for ConfigUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read {
                path,
                context
            } => {
                write!(f, "failed to read config at {:?}: {}", path, context)
            }
            Self::Parse {
                path,
                context
            } => {
                write!(f, "failed to parse config at {:?}: {}", path, context)
            }
            Self::Validation(err) => write!(f, "{}", err),
            Self::Removed => write!(f, "configuration file removed"),
            Self::State {
                context
            } => {
                write!(f, "failed to update configuration state: {}", context)
            }
        }
    }
}

impl std::error::Error for ConfigUpdateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(err) => Some(err),
            _ => None
        }
    }
}

impl From<ConfigValidationError> for ConfigUpdateError {
    fn from(err: ConfigValidationError) -> Self {
        Self::Validation(err)
    }
}

impl ConfigUpdateError {
    /// Construct a read error with contextual information.
    pub fn read(path: PathBuf, err: &std::io::Error) -> Self {
        Self::Read {
            path,
            context: err.to_string()
        }
    }

    /// Construct a parse error with contextual information.
    pub fn parse(path: PathBuf, err: &toml::de::Error) -> Self {
        Self::Parse {
            path,
            context: err.to_string()
        }
    }

    /// Construct a state management error.
    pub fn state(context: impl Into<String>) -> Self {
        Self::State {
            context: context.into()
        }
    }
}

/// Information about configuration degradation events.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigDegradation {
    /// The reason the configuration could not be refreshed.
    pub reason:     ConfigUpdateError,
    /// The last known valid configuration.
    pub last_valid: Box<Config>
}

/// Errors produced by [`ConfigManager`].
#[derive(Debug)]
pub enum ConfigManagerError {
    /// The internal configuration state lock was poisoned.
    Poisoned
}

impl std::fmt::Display for ConfigManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => write!(f, "config state lock poisoned")
        }
    }
}

impl std::error::Error for ConfigManagerError {}

/// Tracks and manages the last known valid configuration.
#[derive(Debug)]
pub struct ConfigManager {
    state: RwLock<Config>
}

impl ConfigManager {
    /// Creates a new manager seeded with the initial configuration.
    pub fn new(initial: Config) -> Self {
        Self {
            state: RwLock::new(initial)
        }
    }

    fn with_state<F, T>(&self, f: F) -> Result<T, ConfigManagerError>
    where
        F: FnOnce(&Config) -> T
    {
        self.state
            .read()
            .map_err(|_| ConfigManagerError::Poisoned)
            .map(|guard| f(&guard))
    }

    /// Returns the last successfully applied configuration.
    pub fn last_valid(&self) -> Result<Config, ConfigManagerError> {
        self.with_state(Clone::clone)
    }

    /// Records a degradation event and returns contextual information for
    /// consumers.
    pub fn degraded(
        &self,
        reason: ConfigUpdateError
    ) -> Result<ConfigDegradation, ConfigManagerError> {
        self.with_state(|config| ConfigDegradation {
            reason,
            last_valid: Box::new(config.clone())
        })
    }

    /// Applies a freshly loaded configuration, computing the impact relative to
    /// the previous state.
    pub fn apply(&self, updated: Config) -> Result<ConfigApplied, ConfigManagerError> {
        let mut guard = self
            .state
            .write()
            .map_err(|_| ConfigManagerError::Poisoned)?;

        let impact = compute_impact(&guard, &updated);
        *guard = updated.clone();

        Ok(ConfigApplied {
            config: Arc::new(updated),
            impact
        })
    }
}

fn compute_impact(previous: &Config, next: &Config) -> ConfigImpact {
    let mut impact = ConfigImpact::default();

    if previous.modules != next.modules {
        impact.layout_changed = true;
    }

    if previous.appearance != next.appearance {
        impact.appearance_changed = true;
    }

    if previous.appearance.workspace_colors != next.appearance.workspace_colors
        || previous.appearance.special_workspace_colors != next.appearance.special_workspace_colors
    {
        impact.affected_modules.insert(ModuleName::Workspaces);
    }

    if previous.outputs != next.outputs {
        impact.outputs_changed = true;
    }

    if previous.position != next.position {
        impact.position_changed = true;
    }

    if previous.log_level != next.log_level {
        impact.log_level_changed = true;
    }

    if previous.menu_keyboard_focus != next.menu_keyboard_focus {
        impact.menu_focus_changed = true;
    }

    mark_if_changed(
        &mut impact,
        ModuleName::AppLauncher,
        &previous.app_launcher_cmd,
        &next.app_launcher_cmd
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Clipboard,
        &previous.clipboard_cmd,
        &next.clipboard_cmd
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Updates,
        &previous.updates,
        &next.updates
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Workspaces,
        &previous.workspaces,
        &next.workspaces
    );
    mark_if_changed(
        &mut impact,
        ModuleName::WindowTitle,
        &previous.window_title,
        &next.window_title
    );
    mark_if_changed(
        &mut impact,
        ModuleName::SystemInfo,
        &previous.system,
        &next.system
    );
    mark_if_changed(
        &mut impact,
        ModuleName::Battery,
        &previous.battery,
        &next.battery
    );
    mark_if_changed(&mut impact, ModuleName::Clock, &previous.clock, &next.clock);
    mark_if_changed(
        &mut impact,
        ModuleName::Settings,
        &previous.settings,
        &next.settings
    );
    mark_if_changed(
        &mut impact,
        ModuleName::MediaPlayer,
        &previous.media_player,
        &next.media_player
    );
    mark_if_changed(
        &mut impact,
        ModuleName::KeyboardLayout,
        &previous.keyboard_layout,
        &next.keyboard_layout
    );

    if previous.custom_modules != next.custom_modules {
        impact.custom_modules_changed = true;
        update_custom_module_impact(&mut impact, &previous.custom_modules, &next.custom_modules);
    }

    impact
}

fn mark_if_changed<T>(impact: &mut ConfigImpact, module: ModuleName, previous: &T, next: &T)
where
    T: PartialEq
{
    if previous != next {
        impact.affected_modules.insert(module);
    }
}

fn update_custom_module_impact(
    impact: &mut ConfigImpact,
    previous: &[CustomModuleDef],
    next: &[CustomModuleDef]
) {
    let previous_map: HashMap<&str, &CustomModuleDef> = previous
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect();
    let next_map: HashMap<&str, &CustomModuleDef> = next
        .iter()
        .map(|module| (module.name.as_str(), module))
        .collect();

    for (name, module) in &next_map {
        let needs_update = match previous_map.get(name) {
            Some(current) => *current != *module,
            None => true
        };

        if needs_update {
            impact
                .affected_modules
                .insert(ModuleName::Custom((*name).to_string()));
        }
    }

    for name in previous_map.keys() {
        if !next_map.contains_key(name) {
            impact
                .affected_modules
                .insert(ModuleName::Custom((*name).to_string()));
        }
    }
}
