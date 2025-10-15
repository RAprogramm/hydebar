#![allow(async_fn_in_trait)]

pub mod iwd;
pub mod network_manager;

mod common;
pub(crate) use common::*;
use masterror::AppResult;
use zbus::zvariant::OwnedObjectPath;

use super::data::{AccessPoint, KnownConnection, NetworkData};

/// Trait defining the interface for a network backend implementation.
pub trait NetworkBackend: Send + Sync {
    /// Initializes the backend and fetches the initial network data snapshot.
    async fn initialize_data(&self) -> AppResult<NetworkData>;

    /// Toggles airplane mode for the backend.
    async fn set_airplane_mode(&self, enable: bool) -> AppResult<()>;

    /// Requests a scan for nearby Wi-Fi networks.
    async fn scan_nearby_wifi(&self) -> AppResult<()>;

    /// Enables or disables Wi-Fi functionality on the backend.
    async fn set_wifi_enabled(&self, enable: bool) -> AppResult<()>;

    /// Connects to a specific access point, optionally using a password.
    async fn select_access_point(
        &mut self,
        ap: &AccessPoint,
        password: Option<String>
    ) -> AppResult<()>;

    /// Retrieves the known connections from the backend.
    async fn known_connections(&self) -> AppResult<Vec<KnownConnection>>;

    /// Enables or disables a VPN connection.
    async fn set_vpn(
        &self,
        connection_path: OwnedObjectPath,
        enable: bool
    ) -> AppResult<Vec<KnownConnection>>;
}
