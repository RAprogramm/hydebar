#![allow(async_fn_in_trait)]

pub mod iwd;
pub mod network_manager;

mod common;
pub(crate) use common::*;
use zbus::zvariant::OwnedObjectPath;

use super::data::{AccessPoint, KnownConnection, NetworkData};

/// Trait defining the interface for a network backend implementation.
pub trait NetworkBackend: Send + Sync
{
    /// Initializes the backend and fetches the initial network data snapshot.
    async fn initialize_data(&self,) -> anyhow::Result<NetworkData,>;

    /// Toggles airplane mode for the backend.
    async fn set_airplane_mode(&self, enable: bool,) -> anyhow::Result<(),>;

    /// Requests a scan for nearby Wi-Fi networks.
    async fn scan_nearby_wifi(&self,) -> anyhow::Result<(),>;

    /// Enables or disables Wi-Fi functionality on the backend.
    async fn set_wifi_enabled(&self, enable: bool,) -> anyhow::Result<(),>;

    /// Connects to a specific access point, optionally using a password.
    async fn select_access_point(
        &mut self,
        ap: &AccessPoint,
        password: Option<String,>,
    ) -> anyhow::Result<(),>;

    /// Retrieves the known connections from the backend.
    async fn known_connections(&self,) -> anyhow::Result<Vec<KnownConnection,>,>;

    /// Enables or disables a VPN connection.
    async fn set_vpn(
        &self,
        connection_path: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<Vec<KnownConnection,>,>;
}
