use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

pub use hydebar_proto::config::*;

pub mod manager;
pub mod watch;

pub use manager::{
    ConfigApplied, ConfigDegradation, ConfigImpact, ConfigManager, ConfigUpdateError,
};
pub use watch::{ConfigEvent, subscription};

use hydebar_proto::config::{Config, DEFAULT_CONFIG_FILE_PATH};
use log::{info, warn};
use shellexpand::full;
use masterror::AppError;

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("failed to expand config path '{input}': {source}")]
    Expand {
        input: String,
        #[source]
        source: shellexpand::LookupError<std::env::VarError>,
    },
    #[error("config file does not exist: {path}")]
    Missing { path: PathBuf },
    #[error("config path '{path}' has no parent directory")]
    MissingParent { path: PathBuf },
    #[error("failed to create config directory '{path}': {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub(crate) enum ConfigReadError {
    #[error("failed to read config file '{path}': {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file '{path}': {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

pub fn get_config(path: Option<PathBuf>) -> Result<(Config, PathBuf), ConfigLoadError> {
    match path {
        Some(path) => {
            info!("Config path provided {path:?}");
            let expanded = expand_path(path)?;

            if !expanded.exists() {
                return Err(ConfigLoadError::Missing { path: expanded });
            }

            let config = load_config_or_default(&expanded);

            Ok((config, expanded))
        }
        None => {
            let expanded = expand_path(PathBuf::from(DEFAULT_CONFIG_FILE_PATH))?;
            ensure_parent_exists(&expanded)?;

            let config = load_config_or_default(&expanded);

            Ok((config, expanded))
        }
    }
}

fn expand_path(path: PathBuf) -> Result<PathBuf, ConfigLoadError> {
    let input = path.to_string_lossy().into_owned();
    match full(&input) {
        Ok(expanded) => Ok(PathBuf::from(expanded.to_string())),
        Err(source) => Err(ConfigLoadError::Expand { input, source }),
    }
}

fn ensure_parent_exists(path: &Path) -> Result<(), ConfigLoadError> {
    let parent = path
        .parent()
        .ok_or_else(|| ConfigLoadError::MissingParent {
            path: path.to_path_buf(),
        })?;

    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|source| ConfigLoadError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    Ok(())
}

pub(crate) fn read_config(path: &Path) -> Result<Config, ConfigReadError> {
    let mut content = String::new();
    File::open(path)
        .and_then(|mut file| file.read_to_string(&mut content))
        .map_err(|source| ConfigReadError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    toml::from_str(&content).map_err(|source| ConfigReadError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

fn load_config_or_default(path: &Path) -> Config {
    info!("Decoding config file {path:?}");

    match read_config(path) {
        Ok(config) => match config.validate() {
            Ok(()) => {
                info!("Config file loaded successfully");
                config
            }
            Err(err) => {
                warn!("{err}");
                warn!("Falling back to default configuration");
                Config::default()
            }
        },
        Err(err) => {
            warn!("{err}");
            warn!("Falling back to default configuration");
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn get_config_returns_default_on_parse_error() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "invalid = [").expect("failed to write invalid config");

        let (config, returned_path) =
            get_config(Some(config_path.clone())).expect("get_config should succeed");

        assert_eq!(returned_path, config_path);
        let default = Config::default();
        assert_eq!(config.log_level, default.log_level);
        assert_eq!(config.menu_keyboard_focus, default.menu_keyboard_focus);
        assert_eq!(config.position, default.position);
    }

    #[test]
    fn get_config_errors_when_file_missing() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("missing.toml");

        let error = get_config(Some(config_path.clone())).expect_err("expected error");

        match error {
            ConfigLoadError::Missing { path } => assert_eq!(path, config_path),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
