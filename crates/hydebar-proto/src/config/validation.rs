use std::collections::HashSet;

use super::{Config, ModuleDef, ModuleName};

/// Errors returned when validating a [`Config`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationError {
    /// Duplicate custom module definitions were found.
    DuplicateCustomModule { name: String },

    /// A module references a custom module definition that does not exist.
    MissingCustomModule { name: String }
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateCustomModule {
                name
            } => {
                write!(f, "duplicate custom module definition for '{}'", name)
            }
            Self::MissingCustomModule {
                name
            } => {
                write!(
                    f,
                    "custom module '{}' referenced in layout but not defined",
                    name
                )
            }
        }
    }
}

impl std::error::Error for ConfigValidationError {}

impl Config {
    /// Validates the configuration, ensuring module definitions are consistent.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigValidationError`] if duplicate custom modules are
    /// defined or if the module layout references undefined custom modules.
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
                    name: module.name.clone()
                });
            }
        }

        let ensure_custom_module_exists = |name: &str| {
            if !seen_custom_modules.contains(name) {
                return Err(ConfigValidationError::MissingCustomModule {
                    name: name.to_owned()
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
    use super::{super::CustomModuleDef, *};
    use crate::config::Modules;

    fn custom_module(name: &str) -> CustomModuleDef {
        CustomModuleDef {
            name:       name.to_owned(),
            command:    String::from("true"),
            icon:       None,
            listen_cmd: None,
            icons:      None,
            alert:      None
        }
    }

    #[test]
    fn validate_accepts_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_duplicate_custom_modules() {
        let config = Config {
            custom_modules: vec![custom_module("foo"), custom_module("foo")],
            ..Default::default()
        };

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
        let config = Config {
            custom_modules: vec![custom_module("foo")],
            modules:        Modules {
                left: vec![ModuleDef::Single(ModuleName::Custom("bar".to_owned()))],
                ..Default::default()
            },
            ..Default::default()
        };

        let error = config
            .validate()
            .expect_err("expected missing module error");
        assert!(matches!(
            error,
            ConfigValidationError::MissingCustomModule { ref name } if name == "bar"
        ));
    }
}
