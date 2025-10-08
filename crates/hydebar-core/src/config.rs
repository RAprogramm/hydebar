use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

pub use hydebar_proto::config::*;

pub mod manager;
pub mod watch;

use log::{info, warn};
pub use manager::{
    ConfigApplied, ConfigDegradation, ConfigImpact, ConfigManager, ConfigUpdateError,
};
use shellexpand::full;
pub use watch::{ConfigEvent, subscription};

#[derive(Debug,)]
pub enum ConfigLoadError
{
    Expand
    {
        input: String, source: shellexpand::LookupError<std::env::VarError,>,
    },
    Missing
    {
        path: PathBuf,
    },
    MissingParent
    {
        path: PathBuf,
    },
    CreateDir
    {
        path: PathBuf, source: std::io::Error,
    },
}

impl std::fmt::Display for ConfigLoadError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result
    {
        match self {
            Self::Expand {
                input,
                source,
            } => {
                write!(f, "failed to expand config path '{}': {}", input, source)
            }
            Self::Missing {
                path,
            } => {
                write!(f, "config file does not exist: {}", path.display())
            }
            Self::MissingParent {
                path,
            } => {
                write!(f, "config path '{}' has no parent directory", path.display())
            }
            Self::CreateDir {
                path,
                source,
            } => {
                write!(f, "failed to create config directory '{}': {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for ConfigLoadError
{
    fn source(&self,) -> Option<&(dyn std::error::Error + 'static),>
    {
        match self {
            Self::Expand {
                source, ..
            } => Some(source,),
            Self::CreateDir {
                source, ..
            } => Some(source,),
            _ => None,
        }
    }
}

#[derive(Debug,)]
pub(crate) enum ConfigReadError
{
    Read
    {
        path: PathBuf, source: std::io::Error,
    },
    Parse
    {
        path: PathBuf, source: toml::de::Error,
    },
}

impl std::fmt::Display for ConfigReadError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result
    {
        match self {
            Self::Read {
                path,
                source,
            } => {
                write!(f, "failed to read config file '{}': {}", path.display(), source)
            }
            Self::Parse {
                path,
                source,
            } => {
                write!(f, "failed to parse config file '{}': {}", path.display(), source)
            }
        }
    }
}

impl std::error::Error for ConfigReadError
{
    fn source(&self,) -> Option<&(dyn std::error::Error + 'static),>
    {
        match self {
            Self::Read {
                source, ..
            } => Some(source,),
            Self::Parse {
                source, ..
            } => Some(source,),
        }
    }
}

pub fn get_config(path: Option<PathBuf,>,) -> Result<(Config, PathBuf,), ConfigLoadError,>
{
    match path {
        Some(path,) => {
            info!("Config path provided {path:?}");
            let expanded = expand_path(path,)?;

            if !expanded.exists() {
                return Err(ConfigLoadError::Missing {
                    path: expanded,
                },);
            }

            let config = load_config_or_default(&expanded,);

            Ok((config, expanded,),)
        }
        None => {
            let expanded = expand_path(PathBuf::from(DEFAULT_CONFIG_FILE_PATH,),)?;
            ensure_parent_exists(&expanded,)?;

            let config = load_config_or_default(&expanded,);

            Ok((config, expanded,),)
        }
    }
}

fn expand_path(path: PathBuf,) -> Result<PathBuf, ConfigLoadError,>
{
    let input = path.to_string_lossy().into_owned();
    match full(&input,) {
        Ok(expanded,) => Ok(PathBuf::from(expanded.to_string(),),),
        Err(source,) => Err(ConfigLoadError::Expand {
            input,
            source,
        },),
    }
}

fn ensure_parent_exists(path: &Path,) -> Result<(), ConfigLoadError,>
{
    let parent = path.parent().ok_or_else(|| ConfigLoadError::MissingParent {
        path: path.to_path_buf(),
    },)?;

    if !parent.exists() {
        fs::create_dir_all(parent,).map_err(|source| ConfigLoadError::CreateDir {
            path: parent.to_path_buf(),
            source,
        },)?;
    }

    Ok((),)
}

pub(crate) fn read_config(path: &Path,) -> Result<Config, ConfigReadError,>
{
    let mut content = String::new();
    File::open(path,).and_then(|mut file| file.read_to_string(&mut content,),).map_err(
        |source| ConfigReadError::Read {
            path: path.to_path_buf(),
            source,
        },
    )?;

    toml::from_str(&content,).map_err(|source| ConfigReadError::Parse {
        path: path.to_path_buf(),
        source,
    },)
}

fn load_config_or_default(path: &Path,) -> Config
{
    info!("Decoding config file {path:?}");

    match read_config(path,) {
        Ok(config,) => match config.validate() {
            Ok((),) => {
                info!("Config file loaded successfully");
                config
            }
            Err(err,) => {
                warn!("{err}");
                warn!("Falling back to default configuration");
                Config::default()
            }
        },
        Err(err,) => {
            warn!("{err}");
            warn!("Falling back to default configuration");
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn get_config_returns_default_on_parse_error()
    {
        let temp_dir = TempDir::new().expect("failed to create temp dir",);
        let config_path = temp_dir.path().join("config.toml",);
        fs::write(&config_path, "invalid = [",).expect("failed to write invalid config",);

        let (config, returned_path,) =
            get_config(Some(config_path.clone(),),).expect("get_config should succeed",);

        assert_eq!(returned_path, config_path);
        let default = Config::default();
        assert_eq!(config.log_level, default.log_level);
        assert_eq!(config.menu_keyboard_focus, default.menu_keyboard_focus);
        assert_eq!(config.position, default.position);
    }

    #[test]
    fn get_config_errors_when_file_missing()
    {
        let temp_dir = TempDir::new().expect("failed to create temp dir",);
        let config_path = temp_dir.path().join("missing.toml",);

        let error = get_config(Some(config_path.clone(),),).expect_err("expected error",);

        match error {
            ConfigLoadError::Missing {
                path,
            } => assert_eq!(path, config_path),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
