use std::{collections::HashMap, ops::Deref};

use iced::futures::{
    Stream, StreamExt,
    stream::{BoxStream, select_all}
};
use itertools::Itertools;
use log::{debug, warn};
use masterror::{AppError, AppResult};
use tokio::process::Command;
use zbus::{
    Result, proxy,
    zvariant::{self, ObjectPath, OwnedObjectPath, OwnedValue, Value}
};

use super::DeviceType;
use crate::services::{
    bluetooth::BluetoothService,
    network::{
        AccessPoint, ActiveConnectionInfo, ConnectivityState, DeviceState, KnownConnection,
        NetworkBackend, NetworkData, NetworkEvent, Vpn
    }
};

#[derive(Clone)]
pub struct NetworkDbus<'a>(NetworkManagerProxy<'a>);

impl NetworkBackend for NetworkDbus<'_> {
    async fn initialize_data(&self) -> AppResult<NetworkData> {
        let nm = self;

        // airplane mode
        let bluetooth_soft_blocked = BluetoothService::check_rfkill_soft_block()
            .await
            .unwrap_or_default();

        let wifi_present = nm.wifi_device_present().await?;

        let wifi_enabled = nm.wireless_enabled().await.unwrap_or_default();
        debug!("Wifi enabled: {wifi_enabled}");

        let airplane_mode = bluetooth_soft_blocked && !wifi_enabled;
        debug!("Airplane mode: {airplane_mode}");

        let active_connections = nm.active_connections_info().await?;
        debug!("Active connections: {active_connections:?}");

        let wireless_access_points = nm.wireless_access_points().await?;
        debug!("Wireless access points: {wireless_access_points:?}");

        let known_connections = nm
            .known_connections_internal(&wireless_access_points)
            .await?;
        debug!("Known connections: {known_connections:?}");

        Ok(NetworkData {
            wifi_present,
            active_connections,
            wifi_enabled,
            airplane_mode,
            connectivity: nm.connectivity().await?,
            wireless_access_points,
            known_connections,
            scanning_nearby_wifi: false,
            last_error: None
        })
    }

    async fn set_airplane_mode(&self, enable: bool) -> AppResult<()> {
        let rfkill_res = Command::new("/usr/sbin/rfkill")
            .arg(if enable { "block" } else { "unblock" })
            .arg("bluetooth")
            .output()
            .await;

        if let Err(e) = rfkill_res {
            debug!("Failed to set bluetooth rfkill: {e}");
        } else {
            debug!("Bluetooth rfkill set successfully");
        }

        let nm = NetworkDbus::new(self.0.inner().connection()).await?;
        nm.set_wireless_enabled(!enable)
            .await
            .map_err(|e| AppError::internal(format!("Failed to set wireless enabled: {}", e)))?;

        Ok(())
    }

    async fn scan_nearby_wifi(&self) -> AppResult<()> {
        for device_path in self
            .wireless_access_points()
            .await?
            .iter()
            .map(|ap| ap.path.clone())
        {
            let device = WirelessDeviceProxy::builder(self.0.inner().connection())
                .path(device_path)
                .map_err(|e| {
                    AppError::internal(format!("Failed to set WirelessDeviceProxy path: {}", e))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build WirelessDeviceProxy: {}", e))
                })?;

            device
                .request_scan(HashMap::new())
                .await
                .map_err(|e| AppError::internal(format!("Failed to request WiFi scan: {}", e)))?;
        }

        Ok(())
    }

    async fn set_wifi_enabled(&self, enable: bool) -> AppResult<()> {
        self.set_wireless_enabled(enable)
            .await
            .map_err(|e| AppError::internal(format!("Failed to set WiFi enabled state: {}", e)))?;
        Ok(())
    }

    async fn select_access_point(
        &mut self,
        access_point: &AccessPoint,
        password: Option<String>
    ) -> AppResult<()> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;
        let connection = settings.find_connection(&access_point.ssid).await?;

        if let Some(connection) = connection.as_ref() {
            if let Some(password) = password {
                let connection = ConnectionSettingsProxy::builder(self.0.inner().connection())
                    .path(connection)
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to set ConnectionSettingsProxy path: {}",
                            e
                        ))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to build ConnectionSettingsProxy: {}",
                            e
                        ))
                    })?;

                let mut s = connection.get_settings().await.map_err(|e| {
                    AppError::internal(format!("Failed to get connection settings: {}", e))
                })?;
                if let Some(wifi_settings) = s.get_mut("802-11-wireless-security") {
                    let new_password = zvariant::Value::from(password.clone())
                        .try_to_owned()
                        .map_err(|e| {
                            AppError::internal(format!("Failed to convert password value: {}", e))
                        })?;
                    wifi_settings.insert("psk".to_string(), new_password);
                }

                connection.update(s).await.map_err(|e| {
                    AppError::internal(format!("Failed to update connection settings: {}", e))
                })?;
            }

            self.activate_connection(
                connection.clone(),
                access_point.device_path.to_owned(),
                OwnedObjectPath::try_from("/").map_err(|e| {
                    AppError::internal(format!("Failed to create object path: {}", e))
                })?
            )
            .await
            .map_err(|e| AppError::internal(format!("Failed to activate connection: {}", e)))?;
        } else {
            let name = access_point.ssid.clone();
            debug!("Create new wifi connection: {name}");

            let mut conn_settings: HashMap<&str, HashMap<&str, zvariant::Value>> =
                HashMap::from([
                    (
                        "802-11-wireless",
                        HashMap::from([("ssid", Value::Array(name.as_bytes().into()))])
                    ),
                    (
                        "connection",
                        HashMap::from([
                            ("id", Value::Str(name.into())),
                            ("type", Value::Str("802-11-wireless".into()))
                        ])
                    )
                ]);

            if let Some(pass) = password {
                conn_settings.insert(
                    "802-11-wireless-security",
                    HashMap::from([
                        ("psk", Value::Str(pass.into())),
                        ("key-mgmt", Value::Str("wpa-psk".into()))
                    ])
                );
            }

            self.add_and_activate_connection(
                conn_settings,
                &access_point.device_path,
                &access_point.path
            )
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to add and activate connection: {}", e))
            })?;
        }

        Ok(())
    }

    async fn set_vpn(
        &self,
        connection: OwnedObjectPath,
        enable: bool
    ) -> AppResult<Vec<KnownConnection>> {
        if enable {
            debug!("Activating VPN: {connection:?}");
            self.activate_connection(
                connection,
                OwnedObjectPath::try_from("/").unwrap(),
                OwnedObjectPath::try_from("/").unwrap()
            )
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to activate VPN connection: {}", e))
            })?;
        } else {
            debug!("Deactivating VPN: {connection:?}");
            self.deactivate_connection(connection).await.map_err(|e| {
                AppError::internal(format!("Failed to deactivate VPN connection: {}", e))
            })?;
        }

        let known_connections = self.known_connections().await?;
        Ok(known_connections)
    }

    async fn known_connections(&self) -> AppResult<Vec<KnownConnection>> {
        let wireless_access_points = self.wireless_access_points().await?;
        self.known_connections_internal(&wireless_access_points)
            .await
    }
}

impl<'a> Deref for NetworkDbus<'a> {
    type Target = NetworkManagerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> NetworkDbus<'a> {
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let nm = NetworkManagerProxy::new(conn).await.map_err(|e| {
            AppError::internal(format!("Failed to create NetworkManagerProxy: {}", e))
        })?;

        Ok(Self(nm))
    }
}

impl<'a> NetworkDbus<'a> {
    pub async fn subscribe_events(
        &'a self
    ) -> AppResult<impl Stream<Item = AppResult<NetworkEvent>> + 'a> {
        type EventStream<'s> = BoxStream<'s, AppResult<NetworkEvent>>;

        let conn = self.0.inner().connection();
        let settings = NetworkSettingsDbus::new(conn).await?;
        let mut streams: Vec<EventStream<'a>> = Vec::new();

        let wireless_enabled = self
            .clone()
            .receive_wireless_enabled_changed()
            .await
            .then(|signal| async move {
                let value = signal.get().await.map_err(|e| {
                    AppError::internal(format!("Failed to get wireless enabled state: {}", e))
                })?;

                debug!("WiFi enabled changed: {value}");
                Ok(NetworkEvent::WiFiEnabled(value))
            })
            .boxed();
        streams.push(wireless_enabled);

        let connectivity_changed = self
            .clone()
            .receive_connectivity_changed()
            .await
            .then(|signal| async move {
                let value = ConnectivityState::from(signal.get().await.map_err(|e| {
                    AppError::internal(format!("Failed to get connectivity state: {}", e))
                })?);

                debug!("Connectivity changed: {value:?}");
                Ok(NetworkEvent::Connectivity(value))
            })
            .boxed();
        streams.push(connectivity_changed);

        let active_connections_changes = self
            .clone()
            .receive_active_connections_changed()
            .await
            .then({
                let backend = self.clone();
                move |_| {
                    let backend = backend.clone();
                    async move {
                        let value = backend.active_connections_info().await?;

                        debug!("Active connections changed: {value:?}");
                        Ok(NetworkEvent::ActiveConnections(value))
                    }
                }
            })
            .boxed();
        streams.push(active_connections_changes);

        let devices = self.wireless_devices().await?;

        let wireless_devices_changed = self
            .clone()
            .receive_devices_changed()
            .await
            .then({
                let backend = self.clone();
                let devices = devices.clone();
                move |_| {
                    let backend = backend.clone();
                    let devices = devices.clone();
                    async move {
                        let current_devices = backend.wireless_devices().await?;
                        if current_devices != devices {
                            let wifi_present = backend.wifi_device_present().await?;
                            let wireless_access_points =
                                backend.wireless_access_points().await?;

                            debug!(
                                "Wireless device changed: wifi present {wifi_present:?}, wireless_access_points {wireless_access_points:?}",
                            );
                            Ok(Some(NetworkEvent::WirelessDevice {
                                wifi_present,
                                wireless_access_points,
                            }))
                        } else {
                            Ok(None)
                        }
                    }
                }
            })
            .filter_map(|result| async move { result.transpose() })
            .boxed();
        streams.push(wireless_devices_changed);

        let wireless_access_points = self.wireless_access_points().await?;

        let mut device_state_changes = Vec::with_capacity(wireless_access_points.len());
        for access_point in wireless_access_points.iter() {
            let device_proxy = DeviceProxy::builder(conn)
                .path(access_point.device_path.clone())
                .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {}", e)))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {}", e)))?;

            let ssid = access_point.ssid.clone();
            device_state_changes.push(
                device_proxy
                    .receive_state_changed()
                    .await
                    .then({
                        let ssid = ssid.clone();
                        move |state| {
                            let ssid = ssid.clone();
                            async move {
                                let value =
                                    state.get().await.map(DeviceState::from).map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to get device state: {}",
                                            e
                                        ))
                                    })?;
                                if value == DeviceState::NeedAuth {
                                    debug!("Request password for ssid {ssid}");
                                    Ok(Some(NetworkEvent::RequestPasswordForSSID(ssid)))
                                } else {
                                    Ok(None)
                                }
                            }
                        }
                    })
                    .filter_map(|result| async move { result.transpose() })
                    .boxed()
            );
        }

        if !device_state_changes.is_empty() {
            let device_states = select_all(device_state_changes).boxed();
            streams.push(device_states);
        }

        let mut access_point_changes = Vec::with_capacity(wireless_access_points.len());
        for access_point in wireless_access_points.iter() {
            let proxy = WirelessDeviceProxy::builder(conn)
                .path(access_point.device_path.clone())
                .map_err(|e| {
                    AppError::internal(format!("Failed to set WirelessDeviceProxy path: {}", e))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build WirelessDeviceProxy: {}", e))
                })?;

            access_point_changes.push(
                proxy
                    .receive_access_points_changed()
                    .await
                    .then({
                        let backend = self.clone();
                        move |_| {
                            let backend = backend.clone();
                            async move {
                                let wireless_access_points =
                                    backend.wireless_access_points().await?;
                                debug!("access_points_changed {wireless_access_points:?}");

                                Ok(NetworkEvent::WirelessAccessPoint(wireless_access_points))
                            }
                        }
                    })
                    .boxed()
            );
        }

        let mut strength_changes_streams = Vec::with_capacity(wireless_access_points.len());
        for access_point in wireless_access_points {
            let ssid = access_point.ssid.clone();
            let proxy = AccessPointProxy::builder(conn)
                .path(access_point.path.clone())
                .map_err(|e| {
                    AppError::internal(format!("Failed to set AccessPointProxy path: {}", e))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build AccessPointProxy: {}", e))
                })?;

            strength_changes_streams.push(
                proxy
                    .receive_strength_changed()
                    .await
                    .then({
                        let ssid = ssid.clone();
                        move |signal| {
                            let ssid = ssid.clone();
                            async move {
                                let value = signal.get().await.map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to get signal strength: {}",
                                        e
                                    ))
                                })?;
                                debug!("Strength changed value: {ssid}, {value}");
                                Ok(NetworkEvent::Strength((ssid, value)))
                            }
                        }
                    })
                    .boxed()
            );
        }

        let strength_changes = select_all(strength_changes_streams).boxed();
        streams.push(strength_changes);

        let access_points = select_all(access_point_changes).boxed();
        streams.push(access_points);

        let known_connections = settings
            .clone()
            .receive_connections_changed()
            .await
            .then({
                let backend = self.clone();
                move |_| {
                    let backend = backend.clone();
                    async move {
                        let known_connections = backend.known_connections().await?;

                        debug!("Known connections changed");
                        Ok(NetworkEvent::KnownConnections(known_connections))
                    }
                }
            })
            .boxed();
        streams.push(known_connections);

        let events = select_all(streams);

        Ok(events)
    }

    pub async fn connectivity(&self) -> AppResult<ConnectivityState> {
        self.0
            .connectivity()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get connectivity state: {}", e)))
            .map(ConnectivityState::from)
    }

    pub async fn wifi_device_present(&self) -> AppResult<bool> {
        let devices = self
            .devices()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get devices: {}", e)))?;
        for d in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .path(d)
                .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {}", e)))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {}", e)))?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn active_connections(&self) -> AppResult<Vec<OwnedObjectPath>> {
        let connections =
            self.0.active_connections().await.map_err(|e| {
                AppError::internal(format!("Failed to get active connections: {}", e))
            })?;

        Ok(connections)
    }

    pub async fn active_connections_info(&self) -> AppResult<Vec<ActiveConnectionInfo>> {
        let active_connections = self.active_connections().await?;
        let mut ac_proxies: Vec<ActiveConnectionProxy> =
            Vec::with_capacity(active_connections.len());
        for active_connection in &active_connections {
            let active_connection = ActiveConnectionProxy::builder(self.0.inner().connection())
                .path(active_connection)
                .map_err(|e| {
                    AppError::internal(format!("Failed to set ActiveConnectionProxy path: {}", e))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build ActiveConnectionProxy: {}", e))
                })?;
            ac_proxies.push(active_connection);
        }

        let mut info = Vec::<ActiveConnectionInfo>::with_capacity(active_connections.len());
        for connection in ac_proxies {
            if connection.vpn().await.unwrap_or_default() {
                info.push(ActiveConnectionInfo::Vpn {
                    name:        connection.id().await.map_err(|e| {
                        AppError::internal(format!("Failed to get VPN connection ID: {}", e))
                    })?,
                    object_path: connection.inner().path().to_owned().into()
                });
                continue;
            }
            for device in connection.devices().await.unwrap_or_default() {
                let device = DeviceProxy::builder(self.0.inner().connection())
                    .path(device)
                    .map_err(|e| {
                        AppError::internal(format!("Failed to set DeviceProxy path: {}", e))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to build DeviceProxy for active connection: {}",
                            e
                        ))
                    })?;

                match device.device_type().await.map(DeviceType::from).ok() {
                    Some(DeviceType::Ethernet) => {
                        let wired_device = WiredDeviceProxy::builder(self.0.inner().connection())
                            .path(device.0.path())
                            .map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to set WiredDeviceProxy path: {}",
                                    e
                                ))
                            })?
                            .build()
                            .await
                            .map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to build WiredDeviceProxy: {}",
                                    e
                                ))
                            })?;

                        info.push(ActiveConnectionInfo::Wired {
                            name:  connection.id().await.map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to get wired connection ID: {}",
                                    e
                                ))
                            })?,
                            speed: wired_device.speed().await.map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to get wired device speed: {}",
                                    e
                                ))
                            })?
                        });
                    }
                    Some(DeviceType::Wifi) => {
                        let wireless_device =
                            WirelessDeviceProxy::builder(self.0.inner().connection())
                                .path(device.0.path())
                                .map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to set WirelessDeviceProxy path: {}",
                                        e
                                    ))
                                })?
                                .build()
                                .await
                                .map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to build WirelessDeviceProxy: {}",
                                        e
                                    ))
                                })?;

                        if let Ok(access_point) = wireless_device.active_access_point().await {
                            let access_point =
                                AccessPointProxy::builder(self.0.inner().connection())
                                    .path(access_point)
                                    .map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to set AccessPointProxy path: {}",
                                            e
                                        ))
                                    })?
                                    .build()
                                    .await
                                    .map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to build AccessPointProxy: {}",
                                            e
                                        ))
                                    })?;

                            info.push(ActiveConnectionInfo::WiFi {
                                id:       connection.id().await.map_err(|e| {
                                    AppError::internal(format!(
                                        "Failed to get WiFi connection ID: {}",
                                        e
                                    ))
                                })?,
                                name:     String::from_utf8_lossy(
                                    &access_point.ssid().await.map_err(|e| {
                                        AppError::internal(format!(
                                            "Failed to get access point SSID: {}",
                                            e
                                        ))
                                    })?
                                )
                                .into_owned(),
                                strength: access_point.strength().await.unwrap_or_default()
                            });
                        }
                    }
                    Some(DeviceType::WireGuard) => {
                        info.push(ActiveConnectionInfo::Vpn {
                            name:        connection.id().await.map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to get WireGuard connection ID: {}",
                                    e
                                ))
                            })?,
                            object_path: connection.inner().path().to_owned().into()
                        });
                    }
                    _ => {}
                }
            }
        }

        info.sort_by(|a, b| {
            let helper = |conn: &ActiveConnectionInfo| match conn {
                ActiveConnectionInfo::Vpn {
                    name, ..
                } => format!("0{name}"),
                ActiveConnectionInfo::Wired {
                    name, ..
                } => format!("1{name}"),
                ActiveConnectionInfo::WiFi {
                    name, ..
                } => format!("2{name}")
            };
            helper(a).cmp(&helper(b))
        });

        Ok(info)
    }

    pub async fn known_connections_internal(
        &self,
        wireless_access_points: &[AccessPoint]
    ) -> AppResult<Vec<KnownConnection>> {
        let settings = NetworkSettingsDbus::new(self.0.inner().connection()).await?;

        let known_connections = settings.know_connections().await?;

        let mut known_ssid = Vec::with_capacity(known_connections.len());
        let mut known_vpn = Vec::new();
        for c in known_connections {
            let cs = ConnectionSettingsProxy::builder(self.0.inner().connection())
                .path(c.clone())
                .map_err(|e| {
                    AppError::internal(format!(
                        "Failed to set ConnectionSettingsProxy path: {}",
                        e
                    ))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build ConnectionSettingsProxy: {}", e))
                })?;
            let Ok(s) = cs.get_settings().await else {
                warn!("Failed to get settings for connection {c}");
                continue;
            };

            let wifi = s.get("802-11-wireless");

            if wifi.is_some() {
                let ssid =
                    s.get("connection")
                        .and_then(|c| c.get("id"))
                        .map(|s| match s.deref() {
                            Value::Str(v) => v.to_string(),
                            _ => "".to_string()
                        });

                if let Some(cur_ssid) = ssid {
                    known_ssid.push(cur_ssid);
                }
            } else if s.contains_key("vpn") {
                let id = s
                    .get("connection")
                    .and_then(|c| c.get("id"))
                    .map(|v| match v.deref() {
                        Value::Str(v) => v.to_string(),
                        _ => "".to_string()
                    });

                if let Some(id) = id {
                    known_vpn.push(Vpn {
                        name: id, path: c
                    });
                }
            }
        }
        let known_connections: Vec<_> = wireless_access_points
            .iter()
            .filter_map(|a| {
                if known_ssid.contains(&a.ssid) {
                    Some(KnownConnection::AccessPoint(a.clone()))
                } else {
                    None
                }
            })
            .chain(known_vpn.into_iter().map(KnownConnection::Vpn))
            .collect();

        Ok(known_connections)
    }

    pub async fn wireless_devices(&self) -> AppResult<Vec<OwnedObjectPath>> {
        let devices = self
            .devices()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get devices: {}", e)))?;
        let mut wireless_devices = Vec::new();
        for d in devices {
            let device = DeviceProxy::builder(self.0.inner().connection())
                .path(&d)
                .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {}", e)))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {}", e)))?;

            if matches!(
                device.device_type().await.map(DeviceType::from),
                Ok(DeviceType::Wifi)
            ) {
                wireless_devices.push(d);
            }
        }

        Ok(wireless_devices)
    }

    pub async fn wireless_access_points(&self) -> AppResult<Vec<AccessPoint>> {
        let wireless_devices = self.wireless_devices().await?;
        let wireless_access_point_futures: Vec<_> = wireless_devices
            .into_iter()
            .map(|path| async move {
                let device = DeviceProxy::builder(self.0.inner().connection())
                    .path(&path)
                    .map_err(|e| {
                        AppError::internal(format!("Failed to set DeviceProxy path: {}", e))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to build DeviceProxy: {}", e))
                    })?;
                let wireless_device = WirelessDeviceProxy::builder(self.0.inner().connection())
                    .path(&path)
                    .map_err(|e| {
                        AppError::internal(format!(
                            "Failed to set WirelessDeviceProxy path: {}",
                            e
                        ))
                    })?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to build WirelessDeviceProxy: {}", e))
                    })?;
                wireless_device
                    .request_scan(HashMap::new())
                    .await
                    .map_err(|e| AppError::internal(format!("Failed to request scan: {}", e)))?;
                let mut scan_changed = wireless_device.receive_last_scan_changed().await;
                if let Some(t) = scan_changed.next().await
                    && let Ok(-1) = t.get().await
                {
                    return Ok(Default::default());
                }
                let access_points = wireless_device.get_access_points().await.map_err(|e| {
                    AppError::internal(format!("Failed to get access points: {}", e))
                })?;
                let state: DeviceState = device
                    .cached_state()
                    .unwrap_or_default()
                    .map(DeviceState::from)
                    .unwrap_or_else(|| DeviceState::Unknown);

                // Sort by strength and remove duplicates
                let mut aps = HashMap::<String, AccessPoint>::new();
                for ap in access_points {
                    let ap = AccessPointProxy::builder(self.0.inner().connection())
                        .path(ap)
                        .map_err(|e| {
                            AppError::internal(format!(
                                "Failed to set AccessPointProxy path: {}",
                                e
                            ))
                        })?
                        .build()
                        .await
                        .map_err(|e| {
                            AppError::internal(format!("Failed to build AccessPointProxy: {}", e))
                        })?;

                    let ssid = String::from_utf8_lossy(
                        &ap.ssid()
                            .await
                            .map_err(|e| {
                                AppError::internal(format!(
                                    "Failed to get access point SSID: {}",
                                    e
                                ))
                            })?
                            .clone()
                    )
                    .into_owned();
                    let public = ap.flags().await.unwrap_or_default() == 0;
                    let strength = ap.strength().await.map_err(|e| {
                        AppError::internal(format!("Failed to get access point strength: {}", e))
                    })?;
                    if let Some(access_point) = aps.get(&ssid)
                        && access_point.strength > strength
                    {
                        continue;
                    }

                    aps.insert(
                        ssid.clone(),
                        AccessPoint {
                            ssid,
                            strength,
                            state,
                            public,
                            working: false,
                            path: ap.inner().path().clone().into(),
                            device_path: device.0.path().clone().into()
                        }
                    );
                }

                let aps = aps
                    .into_values()
                    .sorted_by(|a, b| b.strength.cmp(&a.strength))
                    .collect();

                Ok(aps)
            })
            .collect();

        let mut wireless_access_points = Vec::with_capacity(wireless_access_point_futures.len());
        for f in wireless_access_point_futures {
            let mut access_points: AppResult<Vec<AccessPoint>> = f.await;
            if let Ok(access_points) = &mut access_points {
                wireless_access_points.append(access_points);
            }
        }

        wireless_access_points.sort_by(|a, b| b.strength.cmp(&a.strength));

        Ok(wireless_access_points)
    }
}

#[derive(Clone)]
pub struct NetworkSettingsDbus<'a>(SettingsProxy<'a>);

impl<'a> Deref for NetworkSettingsDbus<'a> {
    type Target = SettingsProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl NetworkSettingsDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let settings = SettingsProxy::new(conn)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create SettingsProxy: {}", e)))?;

        Ok(Self(settings))
    }

    pub async fn know_connections(&self) -> AppResult<Vec<OwnedObjectPath>> {
        self.list_connections()
            .await
            .map_err(|e| AppError::internal(format!("Failed to list connections: {}", e)))
    }

    pub async fn find_connection(&self, name: &str) -> AppResult<Option<OwnedObjectPath>> {
        let connections = self
            .list_connections()
            .await
            .map_err(|e| AppError::internal(format!("Failed to list connections: {}", e)))?;

        for connection in connections {
            let connection = ConnectionSettingsProxy::builder(self.inner().connection())
                .path(connection)
                .map_err(|e| {
                    AppError::internal(format!(
                        "Failed to set ConnectionSettingsProxy path: {}",
                        e
                    ))
                })?
                .build()
                .await
                .map_err(|e| {
                    AppError::internal(format!("Failed to build ConnectionSettingsProxy: {}", e))
                })?;

            let s = connection.get_settings().await.map_err(|e| {
                AppError::internal(format!("Failed to get connection settings: {}", e))
            })?;
            let id = s
                .get("connection")
                .unwrap()
                .get("id")
                .map(|v| match v.deref() {
                    Value::Str(v) => v.to_string(),
                    _ => "".to_string()
                })
                .unwrap();
            if id == name {
                return Ok(Some(connection.inner().path().to_owned().into()));
            }
        }

        Ok(None)
    }
}

#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
pub trait NetworkManager {
    fn activate_connection(
        &self,
        connection: OwnedObjectPath,
        device: OwnedObjectPath,
        specific_object: OwnedObjectPath
    ) -> Result<OwnedObjectPath>;

    fn add_and_activate_connection(
        &self,
        connection: HashMap<&str, HashMap<&str, Value<'_>>>,
        device: &ObjectPath<'_>,
        specific_object: &ObjectPath<'_>
    ) -> Result<(OwnedObjectPath, OwnedObjectPath)>;

    fn deactivate_connection(&self, connection: OwnedObjectPath) -> Result<()>;

    #[zbus(property)]
    fn active_connections(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn devices(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn wireless_enabled(&self) -> Result<bool>;

    #[zbus(property)]
    fn set_wireless_enabled(&self, value: bool) -> Result<()>;

    #[zbus(property)]
    fn connectivity(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Connection/Active",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
trait ActiveConnection {
    #[zbus(property)]
    fn id(&self) -> Result<String>;

    #[zbus(property)]
    fn uuid(&self) -> Result<String>;

    #[zbus(property, name = "Type")]
    fn connection_type(&self) -> Result<String>;

    #[zbus(property)]
    fn state(&self) -> Result<u32>;

    #[zbus(property)]
    fn vpn(&self) -> Result<bool>;

    #[zbus(property)]
    fn devices(&self) -> Result<Vec<OwnedObjectPath>>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device",
    interface = "org.freedesktop.NetworkManager.Device"
)]
pub trait Device {
    #[zbus(property)]
    fn device_type(&self) -> Result<u32>;

    #[zbus(property)]
    fn available_connections(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn active_connection(&self) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn state(&self) -> Result<u32>;
}

#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wired",
    default_service = "org.freedesktop.NetworkManager"
)]
trait WiredDevice {
    /// Carrier property
    #[zbus(property)]
    fn carrier(&self) -> zbus::Result<bool>;

    /// HwAddress property
    #[zbus(property)]
    fn hw_address(&self) -> zbus::Result<String>;

    /// PermHwAddress property
    #[zbus(property)]
    fn perm_hw_address(&self) -> zbus::Result<String>;

    /// S390Subchannels property
    #[zbus(property)]
    fn s390subchannels(&self) -> zbus::Result<Vec<String>>;

    /// Speed property
    #[zbus(property)]
    fn speed(&self) -> zbus::Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Device/Wireless",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
pub trait WirelessDevice {
    /// GetAccessPoints method
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    #[zbus(property)]
    fn active_access_point(&self) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn access_points(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn last_scan(&self) -> zbus::Result<i64>;

    fn request_scan(&self, options: HashMap<String, OwnedValue>) -> Result<()>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/AccessPoint",
    interface = "org.freedesktop.NetworkManager.AccessPoint"
)]
pub trait AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> Result<Vec<u8>>;

    #[zbus(property)]
    fn strength(&self) -> Result<u8>;

    #[zbus(property)]
    fn flags(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings",
    interface = "org.freedesktop.NetworkManager.Settings"
)]
pub trait Settings {
    fn add_connection(
        &self,
        connection: HashMap<String, HashMap<String, OwnedValue>>
    ) -> Result<OwnedObjectPath>;

    #[zbus(property)]
    fn connections(&self) -> Result<Vec<OwnedObjectPath>>;

    fn load_connections(&self, filenames: &[&str]) -> Result<(bool, Vec<String>)>;

    fn list_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings/Connection",
    interface = "org.freedesktop.NetworkManager.Settings.Connection"
)]
trait ConnectionSettings {
    fn update(&self, settings: HashMap<String, HashMap<String, OwnedValue>>) -> Result<()>;

    fn get_settings(&self) -> Result<HashMap<String, HashMap<String, OwnedValue>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::network::ConnectivityState;

    #[test]
    fn device_type_from_u32_maps_known_values() {
        assert_eq!(DeviceType::from(2), DeviceType::Wifi);
        assert_eq!(DeviceType::from(29), DeviceType::WireGuard);
        assert_eq!(DeviceType::from(42), DeviceType::Unknown);
    }

    #[test]
    fn connectivity_state_from_vec_prefers_highest_state() {
        let states = vec![
            ConnectivityState::Portal,
            ConnectivityState::Loss,
            ConnectivityState::Full,
        ];

        assert_eq!(ConnectivityState::from(states), ConnectivityState::Full);
    }
}
