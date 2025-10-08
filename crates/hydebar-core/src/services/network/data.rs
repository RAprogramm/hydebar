use zbus::zvariant::OwnedObjectPath;

/// Describes network-related events emitted by the [`NetworkService`].
///
/// # Examples
/// ```
/// use hydebar_core::services::network::NetworkEvent;
/// let event = NetworkEvent::ScanningNearbyWifi;
/// assert!(matches!(event, NetworkEvent::ScanningNearbyWifi));
/// ```
#[derive(Debug, Clone,)]
pub enum NetworkEvent
{
    /// Indicates that Wi-Fi has been enabled or disabled.
    WiFiEnabled(bool,),
    /// Indicates that airplane mode has been enabled or disabled.
    AirplaneMode(bool,),
    /// Provides the current connectivity state.
    Connectivity(ConnectivityState,),
    /// Carries information about wireless devices and access points.
    WirelessDevice
    {
        /// Whether a Wi-Fi adapter is present on the system.
        wifi_present:           bool,
        /// Visible access points for the adapter.
        wireless_access_points: Vec<AccessPoint,>,
    },
    /// Lists currently active connections.
    ActiveConnections(Vec<ActiveConnectionInfo,>,),
    /// Lists connections remembered by the backend.
    KnownConnections(Vec<KnownConnection,>,),
    /// Provides an updated snapshot of visible access points.
    WirelessAccessPoint(Vec<AccessPoint,>,),
    /// Contains a signal strength update for an SSID.
    Strength((String, u8,),),
    /// Requests a password for the given SSID.
    RequestPasswordForSSID(String,),
    /// Indicates that the backend is scanning for Wi-Fi networks.
    ScanningNearbyWifi,
}

/// Commands accepted by the [`NetworkService`].
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::{AccessPoint, NetworkCommand};
/// use zbus::zvariant::OwnedObjectPath;
///
/// let command = NetworkCommand::ScanNearByWiFi;
/// assert!(matches!(command, NetworkCommand::ScanNearByWiFi));
///
/// let ap = AccessPoint {
///     ssid:        "test".into(),
///     strength:    0,
///     state:       DeviceState::Unknown,
///     public:      true,
///     working:     false,
///     path:        OwnedObjectPath::try_from("/",).unwrap(),
///     device_path: OwnedObjectPath::try_from("/",).unwrap(),
/// };
/// let _ = NetworkCommand::SelectAccessPoint((ap, None,),);
/// ```
#[derive(Debug, Clone,)]
pub enum NetworkCommand
{
    /// Request a Wi-Fi scan.
    ScanNearByWiFi,
    /// Toggle Wi-Fi enablement.
    ToggleWiFi,
    /// Toggle airplane mode.
    ToggleAirplaneMode,
    /// Request connection to an access point.
    SelectAccessPoint((AccessPoint, Option<String,>,),),
    /// Toggle a VPN connection.
    ToggleVpn(Vpn,),
}

/// Collection of data maintained by the [`NetworkService`].
///
/// # Examples
/// ```
/// use hydebar_core::services::network::{ConnectivityState, NetworkData};
///
/// let data = NetworkData::default();
/// assert!(matches!(data.connectivity, ConnectivityState::Unknown));
/// ```
#[derive(Debug, Default, Clone,)]
pub struct NetworkData
{
    /// Whether a Wi-Fi adapter is present.
    pub wifi_present:           bool,
    /// Discovered wireless access points.
    pub wireless_access_points: Vec<AccessPoint,>,
    /// Active network connections reported by the backend.
    pub active_connections:     Vec<ActiveConnectionInfo,>,
    /// Connections remembered by the backend.
    pub known_connections:      Vec<KnownConnection,>,
    /// Whether Wi-Fi is enabled.
    pub wifi_enabled:           bool,
    /// Whether airplane mode is active.
    pub airplane_mode:          bool,
    /// Connectivity status reported by the backend.
    pub connectivity:           ConnectivityState,
    /// Whether the backend is scanning for Wi-Fi.
    pub scanning_nearby_wifi:   bool,
    /// The last error encountered by the service, if any.
    pub last_error:             Option<NetworkServiceError,>,
}

/// Describes a Wi-Fi access point.
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::{AccessPoint, DeviceState};
/// use zbus::zvariant::OwnedObjectPath;
///
/// let ap = AccessPoint {
///     ssid:        "example".into(),
///     strength:    42,
///     state:       DeviceState::Activated,
///     public:      true,
///     working:     true,
///     path:        OwnedObjectPath::try_from("/",).unwrap(),
///     device_path: OwnedObjectPath::try_from("/",).unwrap(),
/// };
/// assert_eq!(ap.ssid, "example");
/// ```
#[derive(Debug, PartialEq, Eq, Clone,)]
pub struct AccessPoint
{
    pub ssid:        String,
    pub strength:    u8,
    pub state:       DeviceState,
    pub public:      bool,
    pub working:     bool,
    pub path:        OwnedObjectPath,
    pub device_path: OwnedObjectPath,
}

/// Describes a VPN entry.
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::Vpn;
/// use zbus::zvariant::OwnedObjectPath;
///
/// let vpn = Vpn {
///     name: "work".into(), path: OwnedObjectPath::try_from("/",).unwrap(),
/// };
/// assert_eq!(vpn.name, "work");
/// ```
#[derive(Debug, Clone,)]
pub struct Vpn
{
    pub name: String,
    pub path: OwnedObjectPath,
}

/// Known connections stored by the backend.
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::{AccessPoint, DeviceState, KnownConnection};
/// use zbus::zvariant::OwnedObjectPath;
///
/// let ap = AccessPoint {
///     ssid:        "lab".into(),
///     strength:    0,
///     state:       DeviceState::Unknown,
///     public:      true,
///     working:     false,
///     path:        OwnedObjectPath::try_from("/",).unwrap(),
///     device_path: OwnedObjectPath::try_from("/",).unwrap(),
/// };
/// let connection = KnownConnection::AccessPoint(ap,);
/// assert!(matches!(connection, KnownConnection::AccessPoint(_)));
/// ```
#[derive(Debug, Clone,)]
pub enum KnownConnection
{
    AccessPoint(AccessPoint,),
    Vpn(Vpn,),
}

/// Active connection information summarised by the backend.
///
/// # Examples
/// ```
/// use std::convert::TryFrom;
///
/// use hydebar_core::services::network::ActiveConnectionInfo;
/// use zbus::zvariant::OwnedObjectPath;
///
/// let info = ActiveConnectionInfo::Vpn {
///     name:        "vpn".into(),
///     object_path: OwnedObjectPath::try_from("/",).unwrap(),
/// };
/// assert_eq!(info.name(), "vpn");
/// ```
#[derive(Debug, Clone,)]
pub enum ActiveConnectionInfo
{
    Wired
    {
        name: String, speed: u32,
    },
    WiFi
    {
        id: String, name: String, strength: u8,
    },
    Vpn
    {
        name: String, object_path: OwnedObjectPath,
    },
}

impl ActiveConnectionInfo
{
    /// Returns the human-friendly name of the connection.
    ///
    /// # Examples
    /// ```
    /// use hydebar_core::services::network::ActiveConnectionInfo;
    /// use zbus::zvariant::OwnedObjectPath;
    ///
    /// let info = ActiveConnectionInfo::Vpn {
    ///     name:        "vpn".into(),
    ///     object_path: OwnedObjectPath::try_from("/",).unwrap(),
    /// };
    /// assert_eq!(info.name(), "vpn");
    /// ```
    #[must_use]
    pub fn name(&self,) -> String
    {
        match self {
            Self::Wired {
                name, ..
            } => name.clone(),
            Self::WiFi {
                name, ..
            } => name.clone(),
            Self::Vpn {
                name, ..
            } => name.clone(),
        }
    }
}

/// Errors surfaced by the [`NetworkService`].
///
/// # Examples
/// ```
/// use hydebar_core::services::network::NetworkServiceError;
///
/// let error = NetworkServiceError::new("failure",);
/// assert_eq!(error.message(), "failure");
/// ```
#[derive(Debug, Clone, PartialEq, Eq,)]
pub struct NetworkServiceError
{
    message: String,
}

impl NetworkServiceError
{
    /// Creates a new error with the provided message.
    #[must_use]
    pub fn new(message: impl Into<String,>,) -> Self
    {
        Self {
            message: message.into(),
        }
    }

    /// Borrows the error message.
    #[must_use]
    pub fn message(&self,) -> &str
    {
        &self.message
    }
}

impl From<anyhow::Error,> for NetworkServiceError
{
    fn from(err: anyhow::Error,) -> Self
    {
        Self::new(format!("{err:#}"),)
    }
}

/// Describes the system connectivity status.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq,)]
pub enum ConnectivityState
{
    None,
    Portal,
    Loss,
    Full,
    #[default]
    Unknown,
}

/// Describes the state of a device as reported by the backend.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq,)]
pub enum DeviceState
{
    Unmanaged,
    Unavailable,
    Disconnected,
    Prepare,
    Config,
    NeedAuth,
    IpConfig,
    IpCheck,
    Secondaries,
    Activated,
    Deactivating,
    Failed,
    #[default]
    Unknown,
}
