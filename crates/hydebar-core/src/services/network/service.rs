use std::{any::TypeId, ops::Deref, time::Duration};

use iced::{
    Subscription, Task,
    futures::{SinkExt, Stream, StreamExt, TryFutureExt},
    stream::channel,
};
use log::{debug, error, info};
use tokio::time::sleep;
use zbus::zvariant::OwnedObjectPath;

use super::backend::{NetworkBackend, iwd::IwdDbus, network_manager::NetworkDbus};
pub use super::data::{
    AccessPoint, ActiveConnectionInfo, ConnectivityState, DeviceState, KnownConnection,
    NetworkCommand, NetworkData, NetworkEvent, NetworkServiceError, Vpn,
};
use crate::services::{ReadOnlyService, Service, ServiceEvent, ServiceEventPublisher};

#[derive(Debug, Clone,)]
/// Reactive service responsible for keeping track of the system network state.
///
/// # Examples
/// ```no_run
/// # async fn demo(service: &mut hydebar_core::services::network::NetworkService) {
/// use hydebar_core::services::network::NetworkServiceError;
/// service.apply_error(NetworkServiceError::new("temporary failure",),);
/// # }
/// ```
pub struct NetworkService
{
    data:           NetworkData,
    conn:           zbus::Connection,
    backend_choice: BackendChoice,
}

impl Deref for NetworkService
{
    type Target = NetworkData;

    fn deref(&self,) -> &Self::Target
    {
        &self.data
    }
}

enum State
{
    Init,
    Active(zbus::Connection, BackendChoice,),
    Error,
}

impl ReadOnlyService for NetworkService
{
    type UpdateEvent = NetworkEvent;
    type Error = NetworkServiceError;

    fn update(&mut self, event: Self::UpdateEvent,)
    {
        self.data.last_error = None;
        match event {
            NetworkEvent::AirplaneMode(airplane_mode,) => {
                self.data.airplane_mode = airplane_mode;
            }
            NetworkEvent::WiFiEnabled(wifi_enabled,) => {
                debug!("WiFi enabled: {wifi_enabled}");
                self.data.wifi_enabled = wifi_enabled;
            }
            NetworkEvent::ScanningNearbyWifi => {
                self.data.scanning_nearby_wifi = true;
            }
            NetworkEvent::WirelessDevice {
                wifi_present,
                wireless_access_points,
            } => {
                self.data.wifi_present = wifi_present;
                self.data.scanning_nearby_wifi = false;
                self.data.wireless_access_points = wireless_access_points;
            }
            NetworkEvent::ActiveConnections(active_connections,) => {
                self.data.active_connections = active_connections;
            }
            NetworkEvent::KnownConnections(known_connections,) => {
                self.data.known_connections = known_connections;
            }
            NetworkEvent::Strength((ssid, new_strength,),) => {
                if let Some(ap,) =
                    self.data.wireless_access_points.iter_mut().find(|ap| ap.ssid == ssid,)
                {
                    ap.strength = new_strength;

                    if let Some(ActiveConnectionInfo::WiFi {
                        strength, ..
                    },) =
                        self.data.active_connections.iter_mut().find(|ac| ac.name() == ap.ssid,)
                    {
                        *strength = new_strength;
                    }
                }
            }
            NetworkEvent::Connectivity(connectivity,) => {
                self.data.connectivity = connectivity;
            }
            NetworkEvent::WirelessAccessPoint(wireless_access_points,) => {
                self.data.wireless_access_points = wireless_access_points;
            }
            NetworkEvent::RequestPasswordForSSID(_,) => {}
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self,>,>
    {
        let id = TypeId::of::<Self,>();

        Subscription::run_with_id(
            id,
            channel(50, async |mut output| {
                NetworkService::listen(&mut output,).await;
            },),
        )
    }
}

#[derive(Debug, Copy, Clone,)]
enum BackendChoice
{
    NetworkManager,
    Iwd,
}

impl BackendChoice
{
    fn with_connection(self, conn: zbus::Connection,) -> BackendChoiceWithConnection
    {
        BackendChoiceWithConnection {
            choice: self,
            conn,
        }
    }
}

struct BackendChoiceWithConnection
{
    choice: BackendChoice,
    conn:   zbus::Connection,
}

impl NetworkBackend for BackendChoiceWithConnection
{
    async fn initialize_data(&self,) -> anyhow::Result<NetworkData,>
    {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn,).await?.initialize_data().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn,).await?.initialize_data().await,
        }
    }

    async fn set_airplane_mode(&self, enable: bool,) -> anyhow::Result<(),>
    {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn,).await?.set_airplane_mode(enable,).await
            }
            BackendChoice::Iwd => {
                IwdDbus::new(&self.conn,).await?.set_airplane_mode(enable,).await
            }
        }
    }

    async fn scan_nearby_wifi(&self,) -> anyhow::Result<(),>
    {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn,).await?.scan_nearby_wifi().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn,).await?.scan_nearby_wifi().await,
        }
    }

    async fn set_wifi_enabled(&self, enable: bool,) -> anyhow::Result<(),>
    {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn,).await?.set_wifi_enabled(enable,).await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn,).await?.set_wifi_enabled(enable,).await,
        }
    }

    async fn select_access_point(
        &mut self,
        ap: &AccessPoint,
        password: Option<String,>,
    ) -> anyhow::Result<(),>
    {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn,).await?.select_access_point(ap, password,).await
            }
            BackendChoice::Iwd => {
                IwdDbus::new(&self.conn,).await?.select_access_point(ap, password,).await
            }
        }
    }

    async fn set_vpn(
        &self,
        connection_path: OwnedObjectPath,
        enable: bool,
    ) -> anyhow::Result<Vec<KnownConnection,>,>
    {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn,).await?.set_vpn(connection_path, enable,).await
            }
            // IWD does not handle VPNs directly
            BackendChoice::Iwd => Err(anyhow::anyhow!("IWD does not support VPN management"),),
        }
    }

    async fn known_connections(&self,) -> anyhow::Result<Vec<KnownConnection,>,>
    {
        match self.choice {
            BackendChoice::NetworkManager => {
                NetworkDbus::new(&self.conn,).await?.known_connections().await
            }
            BackendChoice::Iwd => IwdDbus::new(&self.conn,).await?.known_connections().await,
        }
    }
}

impl NetworkService
{
    /// Records a recoverable error on the network service state.
    ///
    /// # Examples
    /// ```
    /// use std::ops::Deref;
    ///
    /// use hydebar_core::services::network::{NetworkService, NetworkServiceError};
    ///
    /// fn inspect(service: &NetworkService,) -> Option<&NetworkServiceError,>
    /// {
    ///     service.deref().last_error.as_ref()
    /// }
    ///
    /// # fn exercise(mut service: NetworkService) {
    /// service.apply_error(NetworkServiceError::new("unreachable",),);
    /// assert!(inspect(&service).is_some());
    /// # }
    /// ```
    pub fn apply_error(&mut self, error: NetworkServiceError,)
    {
        self.data.last_error = Some(error,);
    }

    async fn consume_network_events<S, P,>(
        mut events: S, publisher: &mut P,
    ) -> anyhow::Result<(),>
    where
        S: Stream<Item = anyhow::Result<NetworkEvent,>,> + Unpin,
        P: ServiceEventPublisher<Self,> + Send,
    {
        while let Some(event,) = events.next().await {
            let event = event?;
            let mut exit_loop = false;
            if let NetworkEvent::WirelessDevice {
                ..
            } = event
            {
                exit_loop = true;
            }
            let _ = publisher.send(ServiceEvent::Update(event,),).await;

            if exit_loop {
                break;
            }
        }

        Ok((),)
    }

    async fn start_listening<P,>(state: State, publisher: &mut P,) -> State
    where
        P: ServiceEventPublisher<Self,> + Send,
    {
        match state {
            State::Init => match zbus::Connection::system().await {
                Ok(conn,) => {
                    info!("Connecting to backend");
                    let maybe_backend: Result<(NetworkData, BackendChoice,), _,> =
                        match NetworkDbus::new(&conn,)
                            .and_then(|nm| async move { nm.initialize_data().await },)
                            .await
                        {
                            Ok(data,) => {
                                info!("NetworkManager service initialized");
                                Ok((data, BackendChoice::NetworkManager,),)
                            }
                            Err(err,) => {
                                info!(
                                    "Failed to initialize NetworkManager. Falling back to iwd. Error: {err}"
                                );
                                match IwdDbus::new(&conn,)
                                    .and_then(|iwd| async move { iwd.initialize_data().await },)
                                    .await
                                {
                                    Ok(data,) => {
                                        info!("IWD service initialized");
                                        Ok((data, BackendChoice::Iwd,),)
                                    }
                                    Err(err,) => {
                                        error!("Failed to initialize network service: {err}");
                                        Err(err,)
                                    }
                                }
                            }
                        };
                    info!("Connected");

                    match maybe_backend {
                        Ok((data, choice,),) => {
                            info!("Network service initialized");
                            let _ = publisher
                                .send(ServiceEvent::Init(NetworkService {
                                    data,
                                    conn: conn.clone(),
                                    backend_choice: choice,
                                },),)
                                .await;
                            State::Active(conn, choice,)
                        }
                        Err(err,) => {
                            if err.is::<zbus::Error>() {
                                error!("Failed to connect to system bus: {err}");
                            } else {
                                error!("Failed to initialize network service: {err}");
                            }
                            let error = NetworkServiceError::from(err,);
                            let _ = publisher.send(ServiceEvent::Error(error,),).await;
                            State::Error
                        }
                    }
                }
                Err(err,) => {
                    error!("Failed to connect to system bus: {err}");
                    let error = NetworkServiceError::new(format!(
                        "Failed to connect to system bus: {err}"
                    ),);
                    let _ = publisher.send(ServiceEvent::Error(error,),).await;

                    State::Error
                }
            },
            State::Active(conn, choice,) => {
                info!("Listening for network events");

                match choice {
                    BackendChoice::NetworkManager => {
                        let nm = match NetworkDbus::new(&conn,).await {
                            Ok(nm,) => nm,
                            Err(e,) => {
                                error!("Failed to create NetworkDbus: {e}");
                                let error = NetworkServiceError::from(e,);
                                let _ = publisher.send(ServiceEvent::Error(error,),).await;
                                return State::Error;
                            }
                        };

                        match nm.subscribe_events().await {
                            Ok(events,) => {
                                match Self::consume_network_events(events, publisher,).await {
                                    Ok((),) => {
                                        debug!("Network service exit events stream");
                                        State::Active(conn, choice,)
                                    }
                                    Err(err,) => {
                                        error!("Network event stream error: {err}");
                                        let error = NetworkServiceError::from(err,);
                                        let _ = publisher.send(ServiceEvent::Error(error,),).await;
                                        State::Error
                                    }
                                }
                            }
                            Err(err,) => {
                                error!("Failed to listen for network events: {err}");
                                let error = NetworkServiceError::from(err,);
                                let _ = publisher.send(ServiceEvent::Error(error,),).await;

                                State::Error
                            }
                        }
                    }
                    BackendChoice::Iwd => {
                        let iwd = match IwdDbus::new(&conn,).await {
                            Ok(iwd,) => iwd,
                            Err(err,) => {
                                error!("Failed to create IwdDbus: {err}");
                                let error = NetworkServiceError::from(err,);
                                let _ = publisher.send(ServiceEvent::Error(error,),).await;
                                return State::Error;
                            }
                        };
                        match iwd.subscribe_events().await {
                            Ok(mut event_s,) => {
                                while let Some(events,) = event_s.next().await {
                                    for event in events {
                                        let _ =
                                            publisher.send(ServiceEvent::Update(event,),).await;
                                    }
                                }

                                debug!("Network service exit events stream");

                                State::Active(conn, choice,)
                            }
                            Err(err,) => {
                                error!("Failed to listen for network events: {err}");
                                let error = NetworkServiceError::from(err,);
                                let _ = publisher.send(ServiceEvent::Error(error,),).await;

                                State::Error
                            }
                        }
                    }
                }
            }
            State::Error => {
                error!("Network service error");

                sleep(Duration::from_secs(1,),).await;

                State::Init
            }
        }
    }

    pub async fn listen<P,>(publisher: &mut P,)
    where
        P: ServiceEventPublisher<Self,> + Send,
    {
        let mut state = State::Init;

        loop {
            state = Self::start_listening(state, publisher,).await;
        }
    }

    pub async fn run_command(self, command: NetworkCommand,) -> ServiceEvent<Self,>
    {
        let mut bc = self.backend_choice.with_connection(self.conn.clone(),);

        match command {
            NetworkCommand::ToggleAirplaneMode => {
                let airplane_mode = self.airplane_mode;
                debug!("Toggling airplane mode to: {}", !airplane_mode);
                let result = bc.set_airplane_mode(!airplane_mode,).await;
                let new_state = if result.is_ok() { !airplane_mode } else { airplane_mode };

                ServiceEvent::Update(NetworkEvent::AirplaneMode(new_state,),)
            }
            NetworkCommand::ScanNearByWiFi => {
                let _ = bc.scan_nearby_wifi().await;
                ServiceEvent::Update(NetworkEvent::ScanningNearbyWifi,)
            }
            NetworkCommand::ToggleWiFi => {
                let wifi_enabled = self.wifi_enabled;
                debug!("Toggling wifi to: {}", !wifi_enabled);
                let result = bc.set_wifi_enabled(!wifi_enabled,).await;
                let new_state = if result.is_ok() { !wifi_enabled } else { wifi_enabled };

                ServiceEvent::Update(NetworkEvent::WiFiEnabled(new_state,),)
            }
            NetworkCommand::SelectAccessPoint((access_point, password,),) => {
                bc.select_access_point(&access_point, password,).await.unwrap_or_default();
                let known_connections = bc.known_connections().await.unwrap_or_default();

                ServiceEvent::Update(NetworkEvent::KnownConnections(known_connections,),)
            }
            NetworkCommand::ToggleVpn(vpn,) => {
                let mut active_vpn = self.active_connections.iter().find_map(|kc| match kc {
                    ActiveConnectionInfo::Vpn {
                        name,
                        object_path,
                    } if name == &vpn.name => Some(object_path.clone(),),
                    _ => None,
                },);

                let (object_path, new_state,) = if let Some(active_vpn,) = active_vpn.take() {
                    (active_vpn, false,)
                } else {
                    (vpn.path, true,)
                };

                bc.set_vpn(object_path, new_state,).await.unwrap_or_default();
                let known_connections = bc.known_connections().await.unwrap_or_default();

                ServiceEvent::Update(NetworkEvent::KnownConnections(known_connections,),)
            }
        }
    }
}

impl Service for NetworkService
{
    type Command = NetworkCommand;

    fn command(&mut self, command: Self::Command,) -> Task<ServiceEvent<Self,>,>
    {
        debug!("Command: {command:?}");
        let service = self.clone();

        Task::perform(
            async move { NetworkService::run_command(service, command,).await },
            |event| event,
        )
    }
}

#[cfg(test)]
mod tests
{
    use anyhow::anyhow;
    use iced::futures::{StreamExt, channel::mpsc, stream};
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    async fn consume_network_events_stops_on_error()
    {
        let (mut sender, mut receiver,) = mpsc::channel(4,);

        let events = stream::iter(vec![
            Ok(NetworkEvent::WiFiEnabled(true,),),
            Err(anyhow!("boom"),),
            Ok(NetworkEvent::WiFiEnabled(false,),),
        ],);

        let result = NetworkService::consume_network_events(events, &mut sender,).await;
        assert!(result.is_err(), "expected error from stream consumption");

        let first_event = receiver.next().await;
        assert!(
            matches!(first_event, Some(ServiceEvent::Update(NetworkEvent::WiFiEnabled(true)))),
            "unexpected event: {first_event:?}"
        );

        drop(sender,);
        assert!(receiver.next().await.is_none(), "no further events expected");
    }

    #[tokio::test]
    async fn state_error_transitions_to_init_after_delay()
    {
        let (mut sender, _receiver,) = mpsc::channel(1,);

        let state = timeout(
            Duration::from_secs(2,),
            NetworkService::start_listening(State::Error, &mut sender,),
        )
        .await
        .expect("network listener should complete after delay",);
        assert!(matches!(state, State::Init));
    }
}
