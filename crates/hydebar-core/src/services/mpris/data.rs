use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult}
};

use zbus::zvariant::OwnedValue;

use super::dbus::MprisPlayerProxy;

/// Playback state reported by an MPRIS-compatible media player.
///
/// # Examples
///
/// ```
/// use crate::services::mpris::PlaybackStatus;
///
/// assert_eq!(
///     PlaybackStatus::from(String::from("Playing")),
///     PlaybackStatus::Playing
/// );
/// assert_eq!(
///     PlaybackStatus::from(String::from("unknown")),
///     PlaybackStatus::Playing
/// );
/// ```
///
/// Unknown variants default to [`PlaybackStatus::Playing`].
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    /// The player is actively playing media.
    #[default]
    Playing,
    /// The player is paused.
    Paused,
    /// The player is stopped.
    Stopped
}

impl From<String> for PlaybackStatus {
    fn from(playback_status: String) -> PlaybackStatus {
        match playback_status.as_str() {
            "Playing" => PlaybackStatus::Playing,
            "Paused" => PlaybackStatus::Paused,
            "Stopped" => PlaybackStatus::Stopped,
            _ => PlaybackStatus::Playing
        }
    }
}

/// Song metadata exposed by an MPRIS-compatible player.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
///
/// use zbus::zvariant::OwnedValue;
///
/// use crate::services::mpris::MprisPlayerMetadata;
///
/// let mut values = HashMap::new();
/// values.insert("xesam:title".to_string(), OwnedValue::from("Example"));
///
/// let metadata = MprisPlayerMetadata::from(values);
/// assert_eq!(metadata.title.as_deref(), Some("Example"));
/// ```
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MprisPlayerMetadata {
    /// List of artists contributing to the current track.
    pub artists: Option<Vec<String>>,
    /// Title of the currently playing track.
    pub title:   Option<String>
}

impl Display for MprisPlayerMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let title = match (self.artists.as_ref(), self.title.as_ref()) {
            (None, None) => String::new(),
            (None, Some(track_title)) => track_title.clone(),
            (Some(artists), None) => artists.join(", "),
            (Some(artists), Some(track_title)) => {
                format!("{} - {}", artists.join(", "), track_title)
            }
        };

        write!(f, "{title}")
    }
}

impl From<HashMap<String, OwnedValue>> for MprisPlayerMetadata {
    fn from(value: HashMap<String, OwnedValue>) -> Self {
        let artists = match value.get("xesam:artist") {
            Some(entry) => entry.clone().try_into().ok(),
            None => None
        };
        let title = match value.get("xesam:title") {
            Some(entry) => entry.clone().try_into().ok(),
            None => None
        };

        Self {
            artists,
            title
        }
    }
}

/// Representation of a single MPRIS player instance known to the service.
#[derive(Debug, Clone)]
pub struct MprisPlayerData {
    /// Service name on the D-Bus session bus.
    pub service:      String,
    /// Cached metadata returned by the player.
    pub metadata:     Option<MprisPlayerMetadata>,
    /// Cached volume level expressed as a percentage [0, 100].
    pub volume:       Option<f64>,
    /// Current playback status as reported by the player.
    pub state:        PlaybackStatus,
    pub(crate) proxy: MprisPlayerProxy<'static>
}

/// Events produced by the MPRIS service.
#[derive(Debug, Clone)]
pub enum MprisPlayerEvent {
    /// Signals that the known players list should be refreshed entirely.
    Refresh(Vec<MprisPlayerData>),
    /// Metadata for a specific service changed.
    Metadata(String, Option<MprisPlayerMetadata>),
    /// Volume for a specific service changed.
    Volume(String, Option<f64>),
    /// Playback state for a specific service changed.
    State(String, PlaybackStatus)
}
