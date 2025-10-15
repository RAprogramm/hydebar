use std::{
    any::TypeId,
    fs,
    ops::Deref,
    path::{Path, PathBuf}
};

use iced::{
    Subscription, Task,
    futures::{StreamExt, stream::pending},
    stream::channel
};
use log::{debug, error, info, warn};
use tokio::io::{Interest, unix::AsyncFd};
use zbus::proxy;

use super::{ReadOnlyService, Service, ServiceEvent, ServiceEventPublisher};

#[path = "brightness/error.rs"]
mod error;

pub use error::BrightnessError;

#[derive(Debug, Clone, Default)]
pub struct BrightnessData {
    pub current: u32,
    pub max:     u32
}

#[derive(Debug, Clone)]
pub struct BrightnessService {
    data:        BrightnessData,
    device_path: PathBuf,
    conn:        zbus::Connection
}

impl Deref for BrightnessService {
    type Target = BrightnessData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl BrightnessService {
    async fn get_max_brightness(device_path: &Path) -> Result<u32, BrightnessError> {
        let path = device_path.join("max_brightness");
        let contents = fs::read_to_string(&path)
            .map_err(|err| BrightnessError::filesystem(format!("{}: {err}", path.display())))?;
        let value = contents
            .trim()
            .parse::<u32>()
            .map_err(|err| BrightnessError::parse(format!("{}: {err}", path.display())))?;

        Ok(value)
    }

    async fn get_actual_brightness(device_path: &Path) -> Result<u32, BrightnessError> {
        let path = device_path.join("actual_brightness");
        let contents = fs::read_to_string(&path)
            .map_err(|err| BrightnessError::filesystem(format!("{}: {err}", path.display())))?;
        let value = contents
            .trim()
            .parse::<u32>()
            .map_err(|err| BrightnessError::parse(format!("{}: {err}", path.display())))?;

        Ok(value)
    }

    async fn initialize_data(device_path: &Path) -> Result<BrightnessData, BrightnessError> {
        let max_brightness = Self::get_max_brightness(device_path).await?;
        let actual_brightness = Self::get_actual_brightness(device_path).await?;

        debug!("Max brightness: {max_brightness}, current brightness: {actual_brightness}");

        Ok(BrightnessData {
            current: actual_brightness,
            max:     max_brightness
        })
    }

    fn resolve_device_path(device_path: Option<PathBuf>) -> Result<PathBuf, BrightnessError> {
        device_path.ok_or(BrightnessError::MissingDevice)
    }

    async fn init_service() -> Result<(zbus::Connection, PathBuf), BrightnessError> {
        let backlight_devices = Self::backlight_enumerate()?;
        let candidate = backlight_devices
            .iter()
            .find(|device| device.subsystem().and_then(|s| s.to_str()) == Some("backlight"));
        let device_path =
            match Self::resolve_device_path(candidate.map(|d| d.syspath().to_path_buf())) {
                Ok(path) => path,
                Err(err @ BrightnessError::MissingDevice) => {
                    warn!("No backlight devices found");
                    return Err(err);
                }
                Err(err) => return Err(err)
            };

        let conn = zbus::Connection::system()
            .await
            .map_err(BrightnessError::from)?;

        Ok((conn, device_path))
    }

    pub async fn backlight_monitor_listener()
    -> Result<AsyncFd<udev::MonitorSocket>, BrightnessError> {
        let builder = udev::MonitorBuilder::new().map_err(BrightnessError::from)?;
        let builder = builder
            .match_subsystem("backlight")
            .map_err(BrightnessError::from)?;
        let socket = builder.listen().map_err(BrightnessError::from)?;

        AsyncFd::with_interest(socket, Interest::READABLE | Interest::WRITABLE)
            .map_err(BrightnessError::from)
    }

    fn backlight_enumerate() -> Result<Vec<udev::Device>, BrightnessError> {
        let mut enumerator = udev::Enumerator::new().map_err(BrightnessError::from)?;
        enumerator
            .match_subsystem("backlight")
            .map_err(BrightnessError::from)?;

        Ok(enumerator
            .scan_devices()
            .map_err(BrightnessError::from)?
            .collect())
    }

    async fn start_listening<P>(state: State, publisher: &mut P) -> Result<State, BrightnessError>
    where
        P: ServiceEventPublisher<Self> + Send
    {
        match state {
            State::Init => {
                let (conn, device_path) = Self::init_service().await?;
                let data = Self::initialize_data(&device_path).await?;
                let service = BrightnessService {
                    data,
                    device_path: device_path.clone(),
                    conn
                };
                let _ = publisher.send(ServiceEvent::Init(service)).await;

                Ok(State::Active(device_path))
            }
            State::Active(device_path) => {
                info!("Listening for brightness events");
                let mut current_value = Self::get_actual_brightness(&device_path).await?;
                let mut socket = Self::backlight_monitor_listener().await?;

                loop {
                    let mut guard = socket.writable_mut().await.map_err(BrightnessError::from)?;

                    for evt in guard.get_inner().iter() {
                        debug!("{:?}: {:?}", evt.event_type(), evt.device());

                        if evt.device().subsystem().and_then(|s| s.to_str()) != Some("backlight") {
                            continue;
                        }

                        match evt.event_type() {
                            udev::EventType::Change => {
                                debug!("Changed backlight device: {:?}", evt.syspath());
                                let new_value = Self::get_actual_brightness(&device_path).await?;

                                if new_value != current_value {
                                    current_value = new_value;
                                    let _ = publisher
                                        .send(ServiceEvent::Update(BrightnessEvent(new_value)))
                                        .await;
                                }
                            }
                            other => {
                                debug!("Unhandled event type: {other:?}");
                            }
                        }
                    }

                    guard.clear_ready();
                }

                #[allow(unreachable_code)]
                Ok(State::Active(device_path))
            }
            State::Error => {
                error!("Brightness service error");
                let _ = pending::<u8>().next().await;
                Ok(State::Error)
            }
        }
    }

    async fn set_brightness(
        conn: &zbus::Connection,
        device_path: &Path,
        value: u32
    ) -> Result<(), BrightnessError> {
        let brightness_ctrl = BrightnessCtrlProxy::new(conn)
            .await
            .map_err(BrightnessError::from)?;
        let device_name = device_path
            .file_name()
            .and_then(|d| d.to_str())
            .ok_or_else(|| {
                BrightnessError::filesystem(format!(
                    "invalid device path: {}",
                    device_path.display()
                ))
            })?;

        brightness_ctrl
            .set_brightness("backlight", device_name, value)
            .await
            .map_err(BrightnessError::from)?;

        Ok(())
    }
}

impl BrightnessService {
    pub async fn listen<P>(publisher: &mut P)
    where
        P: ServiceEventPublisher<Self> + Send
    {
        let mut state = State::Init;

        loop {
            match Self::start_listening(state, publisher).await {
                Ok(next_state) => {
                    state = next_state;
                }
                Err(err) => {
                    error!("Brightness service failure: {err:?}");
                    let _ = publisher.send(ServiceEvent::Error(err.clone())).await;
                    state = State::Error;
                }
            }
        }
    }

    pub async fn run_command(self, command: BrightnessCommand) -> ServiceEvent<Self> {
        match command {
            BrightnessCommand::Set(value) => {
                match Self::set_brightness(&self.conn, &self.device_path, value).await {
                    Ok(()) => ServiceEvent::Update(BrightnessEvent(value)),
                    Err(err) => ServiceEvent::Error(err)
                }
            }
            BrightnessCommand::Refresh => {
                match Self::get_actual_brightness(&self.device_path).await {
                    Ok(value) => ServiceEvent::Update(BrightnessEvent(value)),
                    Err(err) => ServiceEvent::Error(err)
                }
            }
        }
    }
}

enum State {
    Init,
    Active(PathBuf),
    Error
}

#[derive(Debug, Clone)]
pub struct BrightnessEvent(u32);

impl ReadOnlyService for BrightnessService {
    type UpdateEvent = BrightnessEvent;
    type Error = BrightnessError;

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data.current = event.0;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async |mut output| {
                BrightnessService::listen(&mut output).await;
            })
        )
    }
}

#[derive(Debug, Clone)]
pub enum BrightnessCommand {
    Set(u32),
    Refresh
}

impl Service for BrightnessService {
    type Command = BrightnessCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let service = self.clone();

        Task::perform(
            async move { BrightnessService::run_command(service, command).await },
            |event| event
        )
    }
}

#[proxy(
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto",
    interface = "org.freedesktop.login1.Session"
)]
trait BrightnessCtrl {
    fn set_brightness(&self, subsystem: &str, name: &str, value: u32) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::{BrightnessError, BrightnessService};

    #[test]
    fn resolve_device_path_without_device_fails() {
        let result = BrightnessService::resolve_device_path(None);
        assert!(matches!(result, Err(BrightnessError::MissingDevice)));
    }
}
