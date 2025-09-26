use std::sync::Arc;

use masterror::Error;
use zbus::Error as ZbusError;

/// Error type emitted by the brightness service.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BrightnessError {
    /// Filesystem interaction failed while reading or writing brightness data.
    #[error("failed to access backlight filesystem: {context}")]
    Filesystem { context: Arc<str> },

    /// Parsing the brightness level from sysfs failed.
    #[error("failed to parse brightness value: {context}")]
    Parse { context: Arc<str> },

    /// DBus call to the system brightness controller failed.
    #[error("failed to interact with system bus: {context}")]
    DBus { context: Arc<str> },

    /// No usable backlight device was detected on the system.
    #[error("no backlight devices found")]
    MissingDevice,
}

impl BrightnessError {
    fn arc_from(value: impl Into<String>) -> Arc<str> {
        Arc::<str>::from(value.into())
    }

    /// Create a filesystem error with contextual information.
    pub fn filesystem(context: impl Into<String>) -> Self {
        Self::Filesystem {
            context: Self::arc_from(context),
        }
    }

    /// Create a parse error with contextual information.
    pub fn parse(context: impl Into<String>) -> Self {
        Self::Parse {
            context: Self::arc_from(context),
        }
    }

    /// Create a DBus error with contextual information.
    pub fn dbus(context: impl Into<String>) -> Self {
        Self::DBus {
            context: Self::arc_from(context),
        }
    }
}

impl From<std::io::Error> for BrightnessError {
    fn from(value: std::io::Error) -> Self {
        BrightnessError::filesystem(value.to_string())
    }
}

impl From<std::num::ParseIntError> for BrightnessError {
    fn from(value: std::num::ParseIntError) -> Self {
        BrightnessError::parse(value.to_string())
    }
}

impl From<ZbusError> for BrightnessError {
    fn from(value: ZbusError) -> Self {
        BrightnessError::dbus(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::BrightnessError;

    #[test]
    fn converts_io_errors() {
        let err = BrightnessError::from(std::io::Error::new(std::io::ErrorKind::Other, "boom"));
        assert!(matches!(
            err,
            BrightnessError::Filesystem { ref context } if context.as_ref() == "boom"
        ));
    }

    #[test]
    fn converts_parse_errors() {
        let err = "foo".parse::<u32>().unwrap_err();
        let err = BrightnessError::from(err);
        assert!(matches!(err, BrightnessError::Parse { .. }));
    }

    #[test]
    fn converts_zbus_errors() {
        let err = zbus::Error::Failure("failure".into());
        let err = BrightnessError::from(err);
        assert!(matches!(err, BrightnessError::DBus { .. }));
    }
}
