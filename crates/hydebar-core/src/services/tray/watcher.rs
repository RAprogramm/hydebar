use std::{future::Future, pin::Pin};

use anyhow::Error;
use iced::futures::{Stream, StreamExt, stream::select_all, stream_select};
use log::{debug, error, info};
use masterror::AppError;
use futures::future::pending;

use crate::services::ServiceEvent;

use super::{
    StatusNotifierItem, TrayData, TrayEvent, TrayService,
    dbus::{StatusNotifierWatcher, StatusNotifierWatcherProxy},
    icon,
};

pub(crate) type TrayEventStream = Pin<Box<dyn Stream<Item = TrayEvent> + Send + 'static>>;

#[derive(Debug)]
pub enum TrayWatcherError {
    Connection(Error),
    Initialization(Error),
    EventStream(Error),
}

impl std::fmt::Display for TrayWatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(err) => write!(f, "failed to connect to system bus: {}", err),
            Self::Initialization(err) => write!(f, "failed to initialise tray service: {}", err),
            Self::EventStream(err) => write!(f, "failed to listen for tray events: {}", err),
        }
    }
}

impl std::error::Error for TrayWatcherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connection(err) | Self::Initialization(err) | Self::EventStream(err) => {
                err.source()
            }
        }
    }
}

pub(crate) async fn initialize_data(conn: &zbus::Connection) -> Result<TrayData, TrayWatcherError> {
    debug!("initializing tray data");
    let proxy = StatusNotifierWatcherProxy::new(conn)
        .await
        .map_err(|err| TrayWatcherError::Initialization(err.into()))?;

    let items = proxy
        .registered_status_notifier_items()
        .await
        .map_err(|err| TrayWatcherError::Initialization(err.into()))?;

    let mut status_items = Vec::with_capacity(items.len());
    for item in items {
        let item = StatusNotifierItem::new(conn, item)
            .await
            .map_err(TrayWatcherError::Initialization)?;
        status_items.push(item);
    }

    debug!("created items: {status_items:?}");

    Ok(TrayData(status_items))
}

pub(crate) async fn events(conn: &zbus::Connection) -> Result<TrayEventStream, TrayWatcherError> {
    let watcher = StatusNotifierWatcherProxy::new(conn)
        .await
        .map_err(|err| TrayWatcherError::EventStream(err.into()))?;

    let registered = watcher
        .receive_status_notifier_item_registered()
        .await
        .map_err(|err| TrayWatcherError::EventStream(err.into()))?
        .filter_map({
            let conn = conn.clone();
            move |event| {
                let conn = conn.clone();
                async move {
                    debug!("registered {event:?}");
                    match event.args() {
                        Ok(args) => {
                            let item =
                                StatusNotifierItem::new(&conn, args.service.to_string()).await;
                            item.map(TrayEvent::Registered).ok()
                        }
                        _ => None,
                    }
                }
            }
        })
        .boxed();

    let unregistered = watcher
        .receive_status_notifier_item_unregistered()
        .await
        .map_err(|err| TrayWatcherError::EventStream(err.into()))?
        .filter_map(|event| async move {
            debug!("unregistered {event:?}");
            match event.args() {
                Ok(args) => Some(TrayEvent::Unregistered(args.service.to_string())),
                _ => None,
            }
        })
        .boxed();

    let items = watcher
        .registered_status_notifier_items()
        .await
        .map_err(|err| TrayWatcherError::EventStream(err.into()))?;

    let mut icon_pixel_change = Vec::with_capacity(items.len());
    let mut icon_name_change = Vec::with_capacity(items.len());
    let mut menu_layout_change = Vec::with_capacity(items.len());

    for name in items {
        let item = StatusNotifierItem::new(conn, name.to_string())
            .await
            .map_err(TrayWatcherError::EventStream)?;

        let stream = item.item_proxy.receive_icon_pixmap_changed().await;
        icon_pixel_change.push(
            stream
                .filter_map({
                    let name = name.clone();
                    move |icon| {
                        let name = name.clone();
                        async move {
                            icon.get()
                                .await
                                .ok()
                                .and_then(icon::icon_from_pixmaps)
                                .map(|icon| TrayEvent::IconChanged(name.to_owned(), icon))
                        }
                    }
                })
                .boxed(),
        );

        let stream = item.item_proxy.receive_icon_name_changed().await;
        icon_name_change.push(
            stream
                .filter_map({
                    let name = name.clone();
                    move |icon_name| {
                        let name = name.clone();
                        async move {
                            icon_name
                                .get()
                                .await
                                .ok()
                                .as_deref()
                                .and_then(icon::icon_from_name)
                                .map(|icon| TrayEvent::IconChanged(name.to_owned(), icon))
                        }
                    }
                })
                .boxed(),
        );

        if let Ok(layout_updated) = item.menu_proxy.receive_layout_updated().await {
            menu_layout_change.push(
                layout_updated
                    .filter_map({
                        let name = name.clone();
                        let menu_proxy = item.menu_proxy.clone();
                        move |_| {
                            debug!("layout update event name {name}");
                            let name = name.clone();
                            let menu_proxy = menu_proxy.clone();
                            async move {
                                menu_proxy
                                    .get_layout(0, -1, &[])
                                    .await
                                    .ok()
                                    .map(|(_, layout)| {
                                        TrayEvent::MenuLayoutChanged(name.to_owned(), layout)
                                    })
                            }
                        }
                    })
                    .boxed(),
            );
        }
    }

    Ok(stream_select!(
        registered,
        unregistered,
        select_all(icon_pixel_change),
        select_all(icon_name_change),
        select_all(menu_layout_change)
    )
    .boxed())
}

pub(crate) async fn start_listening<F, Fut>(mut publisher: F)
where
    F: FnMut(ServiceEvent<TrayService>) -> Fut + Send,
    Fut: Future<Output = ()> + Send,
{
    let mut state = State::Init;

    loop {
        state = drive_state(state, &mut publisher).await;
    }
}

enum State {
    Init,
    Active(zbus::Connection),
    Error,
}

async fn drive_state<F, Fut>(state: State, publisher: &mut F) -> State
where
    F: FnMut(ServiceEvent<TrayService>) -> Fut + Send,
    Fut: Future<Output = ()> + Send,
{
    match state {
        State::Init => match StatusNotifierWatcher::start_server().await {
            Ok(conn) => match initialize_data(&conn).await {
                Ok(data) => {
                    info!("Tray service initialized");

                    publisher(ServiceEvent::Init(TrayService {
                        data,
                        _conn: conn.clone(),
                    }))
                    .await;

                    State::Active(conn)
                }
                Err(err) => transition_to_error(err),
            },
            Err(err) => transition_to_error(TrayWatcherError::Connection(err.into())),
        },
        State::Active(conn) => {
            info!("Listening for tray events");

            match events(&conn).await {
                Ok(mut stream) => {
                    while let Some(event) = stream.next().await {
                        debug!("tray data {event:?}");

                        let reload_events = matches!(event, TrayEvent::Registered(_));

                        publisher(ServiceEvent::Update(event)).await;

                        if reload_events {
                            break;
                        }
                    }

                    State::Active(conn)
                }
                Err(err) => transition_to_error(err),
            }
        }
        State::Error => {
            error!("Tray service error");

            pending::<()>().await;
            State::Error
        }
    }
}

fn transition_to_error(error: TrayWatcherError) -> State {
    error!("{error}");
    State::Error
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::{State, TrayWatcherError, transition_to_error};

    #[test]
    fn transition_sets_error_state() {
        let state = transition_to_error(TrayWatcherError::Connection(anyhow!("boom")));
        assert!(matches!(state, State::Error));
    }

    #[test]
    fn error_variants_have_context() {
        let error = TrayWatcherError::EventStream(anyhow!("failure"));
        let message = format!("{error}");
        assert!(message.contains("failed to listen"));
    }
}
