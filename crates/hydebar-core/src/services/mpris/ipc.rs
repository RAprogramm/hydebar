use std::{pin::Pin, sync::Arc};

use anyhow::Context;
use futures::{Stream, StreamExt, future::join_all, stream::SelectAll};
use zbus::{Connection, fdo::DBusProxy};

use super::{
    data::{MprisPlayerData, MprisPlayerMetadata, PlaybackStatus},
    dbus::MprisPlayerProxy,
};

/// Prefix applied to all MPRIS-compliant player service names on the session
/// bus.
pub(crate) const MPRIS_PLAYER_SERVICE_PREFIX: &str = "org.mpris.MediaPlayer2.";

/// Stream item emitted by [`build_event_stream`].
#[derive(Debug,)]
pub(crate) enum IpcEvent
{
    /// Indicates that the ownership of an MPRIS name changed.
    NameOwner,
    /// Metadata for `service` changed.
    Metadata(String, Option<MprisPlayerMetadata,>,),
    /// Volume for `service` changed.
    Volume(String, Option<f64,>,),
    /// Playback state for `service` changed.
    State(String, PlaybackStatus,),
}

/// Combined event stream type returned by [`build_event_stream`].
pub(crate) type EventStream = SelectAll<Pin<Box<dyn Stream<Item = IpcEvent,> + Send,>,>,>;

/// Returns `true` when `name` references an MPRIS player service.
pub(crate) fn is_mpris_service(name: &str,) -> bool
{
    name.starts_with(MPRIS_PLAYER_SERVICE_PREFIX,)
}

/// Fetches all available MPRIS players on the provided D-Bus `conn`.
pub(crate) async fn collect_players(conn: &Connection,) -> anyhow::Result<Vec<MprisPlayerData,>,>
{
    let names = list_mpris_service_names(conn,).await?;
    Ok(fetch_players(conn, &names,).await,)
}

async fn list_mpris_service_names(conn: &Connection,) -> anyhow::Result<Vec<String,>,>
{
    let dbus = DBusProxy::new(conn,).await?;
    let names = dbus
        .list_names()
        .await
        .context("failed to list D-Bus names",)?
        .iter()
        .filter(|name| is_mpris_service(name,),)
        .map(ToString::to_string,)
        .collect();

    Ok(names,)
}

/// Retrieves `MprisPlayerData` entries for each service in `names`.
pub(crate) async fn fetch_players(conn: &Connection, names: &[String],) -> Vec<MprisPlayerData,>
{
    join_all(names.iter().map(|service| async {
        match MprisPlayerProxy::new(conn, service.to_string(),).await {
            Ok(proxy,) => {
                let metadata = proxy.metadata().await.map(MprisPlayerMetadata::from,).ok();
                let volume = proxy.volume().await.map(|value| value * 100.0,).ok();
                let state =
                    proxy.playback_status().await.map(PlaybackStatus::from,).unwrap_or_default();

                Some(MprisPlayerData {
                    service: service.to_string(),
                    metadata,
                    volume,
                    state,
                    proxy,
                },)
            }
            Err(_,) => None,
        }
    },),)
    .await
    .into_iter()
    .flatten()
    .collect()
}

/// Builds a stream that emits [`IpcEvent`] values for all active players.
pub(crate) async fn build_event_stream(conn: &Connection,) -> anyhow::Result<EventStream,>
{
    let dbus = DBusProxy::new(conn,).await?;
    let data = collect_players(conn,).await?;
    let mut combined = SelectAll::new();

    combined.push(Box::pin(dbus.receive_name_owner_changed().await?.filter_map(
        |signal| async move {
            match signal.args() {
                Ok(args,) if is_mpris_service(&args.name,) => Some(IpcEvent::NameOwner,),
                _ => None,
            }
        },
    ),) as Pin<Box<dyn Stream<Item = IpcEvent,> + Send,>,>,);

    for entry in &data {
        let cache = Arc::new(entry.metadata.clone(),);
        let service = entry.service.clone();

        combined.push(Box::pin(entry.proxy.receive_metadata_changed().await.filter_map({
            let cache = Arc::clone(&cache,);
            let service = service.clone();

            move |metadata| {
                let cache = Arc::clone(&cache,);
                let service = service.clone();

                async move {
                    let new_metadata = metadata.get().await.map(MprisPlayerMetadata::from,).ok();

                    if new_metadata.as_ref() == cache.as_ref().as_ref() {
                        None
                    } else {
                        Some(IpcEvent::Metadata(service, new_metadata,),)
                    }
                }
            }
        },),) as Pin<Box<dyn Stream<Item = IpcEvent,> + Send,>,>,);
    }

    for entry in &data {
        let service = entry.service.clone();
        let volume = entry.volume;

        combined.push(Box::pin(entry.proxy.receive_volume_changed().await.filter_map(
            move |signal| {
                let service = service.clone();

                async move {
                    let new_volume = signal.get().await.ok();
                    if new_volume == volume {
                        None
                    } else {
                        Some(IpcEvent::Volume(service, new_volume,),)
                    }
                }
            },
        ),) as Pin<Box<dyn Stream<Item = IpcEvent,> + Send,>,>,);
    }

    for entry in &data {
        let service = entry.service.clone();
        let state = entry.state;

        combined.push(Box::pin(entry.proxy.receive_playback_status_changed().await.filter_map(
            move |signal| {
                let service = service.clone();

                async move {
                    let new_state =
                        signal.get().await.map(PlaybackStatus::from,).unwrap_or_default();

                    if new_state == state {
                        None
                    } else {
                        Some(IpcEvent::State(service, new_state,),)
                    }
                }
            },
        ),) as Pin<Box<dyn Stream<Item = IpcEvent,> + Send,>,>,);
    }

    Ok(combined,)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn detects_mpris_service_prefix()
    {
        assert!(is_mpris_service("org.mpris.MediaPlayer2.foo"));
        assert!(!is_mpris_service("org.freedesktop.DBus"));
    }
}
