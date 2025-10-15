use std::ops::Deref;

use masterror::{AppError, AppResult};
use zbus::{
    Result, proxy,
    zvariant::{ObjectPath, OwnedObjectPath}
};

pub struct UPowerDbus<'a>(UPowerProxy<'a>);

impl<'a> Deref for UPowerDbus<'a> {
    type Target = UPowerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Battery(Vec<DeviceProxy<'static>>);

impl Battery {
    pub async fn state(&self) -> i32 {
        let mut charging = false;
        let mut discharging = false;

        for device in &self.0 {
            if let Ok(state) = device.state().await {
                match state {
                    1 => {
                        charging = true;
                    }
                    2 => {
                        discharging = true;
                    }
                    _ => {}
                }
            }
        }

        if charging {
            1
        } else if discharging {
            2
        } else {
            4
        }
    }

    pub async fn percentage(&self) -> f64 {
        let mut percentage = 0.0;
        let mut count = 0;

        for device in &self.0 {
            if let Ok(p) = device.percentage().await {
                percentage += p;
                count += 1;
            }
        }

        percentage / count as f64
    }

    pub async fn time_to_empty(&self) -> i64 {
        let mut time = 0;

        for device in &self.0 {
            if let Ok(t) = device.time_to_empty().await {
                time += t;
            }
        }

        time
    }

    pub async fn time_to_full(&self) -> i64 {
        let mut time = 0;

        for device in &self.0 {
            if let Ok(t) = device.time_to_full().await {
                time += t;
            }
        }

        time
    }

    pub fn get_devices_path(self) -> Vec<ObjectPath<'static>> {
        self.0
            .into_iter()
            .map(|device| device.inner().path().to_owned())
            .collect()
    }
}

impl UPowerDbus<'_> {
    pub async fn new(conn: &zbus::Connection) -> AppResult<Self> {
        let nm = UPowerProxy::new(conn)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create UPowerProxy: {}", e)))?;

        Ok(Self(nm))
    }

    pub async fn get_battery_devices(&self) -> AppResult<Option<Battery>> {
        let devices = self.enumerate_devices().await.map_err(|e| {
            AppError::internal(format!("Failed to enumerate UPower devices: {}", e))
        })?;

        let mut res = Vec::new();

        for device in devices {
            let device = DeviceProxy::builder(self.inner().connection())
                .path(device)
                .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {}", e)))?
                .build()
                .await
                .map_err(|e| AppError::internal(format!("Failed to build DeviceProxy: {}", e)))?;

            let device_type = device
                .device_type()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get device type: {}", e)))?;
            let power_supply = device
                .power_supply()
                .await
                .map_err(|e| AppError::internal(format!("Failed to get power supply: {}", e)))?;

            if device_type == 2 && power_supply {
                res.push(device);
            }
        }

        if !res.is_empty() {
            Ok(Some(Battery(res)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_device(&self, path: &ObjectPath<'static>) -> AppResult<DeviceProxy<'static>> {
        let device = DeviceProxy::builder(self.inner().connection())
            .path(path)
            .map_err(|e| AppError::internal(format!("Failed to set DeviceProxy path: {}", e)))?
            .build()
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to build DeviceProxy for path: {}", e))
            })?;

        Ok(device)
    }
}

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
pub trait UPower {
    fn enumerate_devices(&self) -> Result<Vec<OwnedObjectPath>>;

    #[zbus(signal)]
    fn device_added(&self) -> Result<OwnedObjectPath>;
}

#[proxy(
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/Device",
    interface = "org.freedesktop.UPower.Device"
)]
pub trait Device {
    #[zbus(property, name = "Type")]
    fn device_type(&self) -> Result<u32>;

    #[zbus(property)]
    fn power_supply(&self) -> Result<bool>;

    #[zbus(property)]
    fn time_to_empty(&self) -> Result<i64>;

    #[zbus(property)]
    fn time_to_full(&self) -> Result<i64>;

    #[zbus(property)]
    fn percentage(&self) -> Result<f64>;

    #[zbus(property)]
    fn state(&self) -> Result<u32>;
}

#[proxy(
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles",
    interface = "org.freedesktop.UPower.PowerProfiles"
)]
pub trait PowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> Result<()>;
}
