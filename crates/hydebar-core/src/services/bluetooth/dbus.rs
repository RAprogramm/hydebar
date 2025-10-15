use std::collections::HashMap;

use masterror::{AppError, AppResult};
use zbus::{
    proxy,
    zvariant::{OwnedObjectPath, OwnedValue}
};

use super::{BluetoothDevice, BluetoothState};

type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

pub struct BluetoothDbus<'a> {
    pub bluez:   BluezObjectManagerProxy<'a>,
    pub adapter: Option<AdapterProxy<'a>>
}

impl BluetoothDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let bluez = BluezObjectManagerProxy::new(conn).await.map_err(|e| {
            AppError::internal(format!("Failed to create BluezObjectManagerProxy: {}", e))
        })?;
        let adapter = bluez
            .get_managed_objects()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get managed objects: {}", e)))?
            .into_iter()
            .filter_map(|(key, item)| {
                if item.contains_key("org.bluez.Adapter1") {
                    Some(key)
                } else {
                    None
                }
            })
            .next();

        let adapter = if let Some(adapter) = adapter {
            Some(
                AdapterProxy::builder(conn)
                    .path(adapter)
                    .map_err(|e| AppError::internal(format!("Failed to set adapter path: {}", e)))?
                    .build()
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to build AdapterProxy: {}", e))
                    })?
            )
        } else {
            None
        };

        Ok(Self {
            bluez,
            adapter
        })
    }

    pub async fn set_powered(&self, value: bool) -> AppResult<()> {
        if let Some(adapter) = &self.adapter {
            adapter.set_powered(value).await.map_err(|e| {
                AppError::internal(format!("Failed to set adapter powered state: {}", e))
            })?;
        }

        Ok(())
    }

    pub async fn state(&self) -> AppResult<BluetoothState> {
        match &self.adapter {
            Some(adapter) => {
                if adapter.powered().await.map_err(|e| {
                    AppError::internal(format!("Failed to get adapter powered state: {}", e))
                })? {
                    Ok(BluetoothState::Active)
                } else {
                    Ok(BluetoothState::Inactive)
                }
            }
            _ => Ok(BluetoothState::Unavailable)
        }
    }

    pub async fn devices(&self) -> AppResult<Vec<BluetoothDevice>> {
        let devices_proxy = self
            .bluez
            .get_managed_objects()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to get managed objects for devices: {}", e))
            })?
            .into_iter()
            .filter_map(|(key, item)| {
                if item.contains_key("org.bluez.Device1") {
                    Some((key.clone(), item.contains_key("org.bluez.Battery1")))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut devices = Vec::new();
        for (device_path, has_battery) in devices_proxy {
            let device = DeviceProxy::builder(self.bluez.inner().connection())
                .path(device_path.clone())
                .map_err(|e| AppError::internal(format!("Failed to set device path: {}", e)))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {}", e)))?;

            let name = device
                .alias()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get device alias: {}", e)))?;
            let connected = device.connected().await.map_err(|e| {
                AppError::internal(format!("Failed to get device connected state: {}", e))
            })?;
            let paired = device.paired().await.unwrap_or(false);

            if paired {
                let battery = if connected && has_battery {
                    let battery_proxy = BatteryProxy::builder(self.bluez.inner().connection())
                        .path(&device_path)
                        .map_err(|e| {
                            AppError::internal(format!("Failed to set battery path: {}", e))
                        })?
                        .build()
                        .await
                        .map_err(|e| {
                            AppError::internal(format!("Failed to build BatteryProxy: {}", e))
                        })?;

                    Some(battery_proxy.percentage().await.map_err(|e| {
                        AppError::internal(format!("Failed to get battery percentage: {}", e))
                    })?)
                } else {
                    None
                };

                devices.push(BluetoothDevice {
                    name,
                    battery,
                    path: device_path,
                    connected
                });
            }
        }

        Ok(devices)
    }

    pub async fn connect_device(&self, device_path: &OwnedObjectPath) -> AppResult<()> {
        let device = DeviceProxy::builder(self.bluez.inner().connection())
            .path(device_path)
            .map_err(|e| {
                AppError::internal(format!("Failed to set device path for connect: {}", e))
            })?
            .build()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to build DeviceProxy for connect: {}", e))
            })?;

        device
            .connect()
            .await
            .map_err(|e| AppError::internal(format!("Failed to connect device: {}", e)))?;
        Ok(())
    }

    pub async fn disconnect_device(&self, device_path: &OwnedObjectPath) -> AppResult<()> {
        let device = DeviceProxy::builder(self.bluez.inner().connection())
            .path(device_path)
            .map_err(|e| {
                AppError::internal(format!("Failed to set device path for disconnect: {}", e))
            })?
            .build()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to build DeviceProxy for disconnect: {}", e))
            })?;

        device
            .disconnect()
            .await
            .map_err(|e| AppError::internal(format!("Failed to disconnect device: {}", e)))?;
        Ok(())
    }
}

#[proxy(
    default_service = "org.bluez",
    default_path = "/",
    interface = "org.freedesktop.DBus.ObjectManager"
)]
pub trait BluezObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;

    #[zbus(signal)]
    fn interfaces_added(&self) -> Result<()>;

    #[zbus(signal)]
    fn interfaces_removed(&self) -> Result<()>;
}

#[proxy(
    default_service = "org.bluez",
    default_path = "/org/bluez/hci0",
    interface = "org.bluez.Adapter1"
)]
pub trait Adapter {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;
}

#[proxy(default_service = "org.bluez", interface = "org.bluez.Device1")]
trait Device {
    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn connected(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;

    fn connect(&self) -> zbus::Result<()>;

    fn disconnect(&self) -> zbus::Result<()>;
}

#[proxy(default_service = "org.bluez", interface = "org.bluez.Battery1")]
pub trait Battery {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<u8>;
}
