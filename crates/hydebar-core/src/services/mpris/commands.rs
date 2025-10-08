use std::{fmt::Display, future::Future, pin::Pin};

use zbus::Connection;

use super::{data::MprisPlayerData, dbus::MprisPlayerProxy, ipc};
use crate::modules::ModuleError;

/// Helper that converts lower-level errors into [`ModuleError`] values.
pub(crate) fn module_error(context: &str, err: impl Display,) -> ModuleError
{
    ModuleError::registration(format!("{context}: {err}"),)
}

/// Command issued against an MPRIS-compatible media player service.
///
/// # Examples
///
/// ```
/// use crate::services::mpris::{MprisPlayerCommand, PlayerCommand};
///
/// let command =
///     MprisPlayerCommand::new("org.mpris.MediaPlayer2.Player".into(), PlayerCommand::Next,);
/// assert_eq!(command.service_name, "org.mpris.MediaPlayer2.Player");
/// ```
#[derive(Debug,)]
pub struct MprisPlayerCommand
{
    /// The fully qualified service name of the target player.
    pub service_name: String,
    /// The action the service should perform.
    pub command:      PlayerCommand,
}

impl MprisPlayerCommand
{
    /// Creates a new [`MprisPlayerCommand`] targeting `service_name`.
    pub fn new(service_name: String, command: PlayerCommand,) -> Self
    {
        Self {
            service_name,
            command,
        }
    }
}

/// Supported MPRIS player commands.
#[derive(Debug,)]
pub enum PlayerCommand
{
    /// Jump to the previous item in the playlist.
    Prev,
    /// Toggle playback between play and pause states.
    PlayPause,
    /// Jump to the next item in the playlist.
    Next,
    /// Adjust the playback volume to a percentage in the range `[0, 100]`.
    Volume(f64,),
}

/// Trait describing how player actions are executed for a proxy implementation.
pub(crate) trait PlayerCommandExecutor
{
    /// Executes a [`PlayerCommand`] against the underlying proxy.
    fn execute_command<'a,>(
        &'a self,
        command: &'a PlayerCommand,
    ) -> Pin<Box<dyn Future<Output = Result<(), ModuleError,>,> + Send + 'a,>,>;
}

impl PlayerCommandExecutor for MprisPlayerProxy<'static,>
{
    fn execute_command<'a,>(
        &'a self,
        command: &'a PlayerCommand,
    ) -> Pin<Box<dyn Future<Output = Result<(), ModuleError,>,> + Send + 'a,>,>
    {
        Box::pin(async move {
            match command {
                PlayerCommand::Prev => self
                    .previous()
                    .await
                    .map_err(|err| module_error("failed to execute previous command", err,),),
                PlayerCommand::PlayPause => self
                    .play_pause()
                    .await
                    .map_err(|err| module_error("failed to execute play/pause command", err,),),
                PlayerCommand::Next => self
                    .next()
                    .await
                    .map_err(|err| module_error("failed to execute next command", err,),),
                PlayerCommand::Volume(volume,) => self
                    .set_volume(volume / 100.0,)
                    .await
                    .map_err(|err| module_error("failed to execute volume command", err,),),
            }
        },)
    }
}

/// Executes `command` against the provided player `data`, refreshing the cached
/// view of available players on success.
pub(crate) async fn execute_player_command(
    conn: &Connection,
    data: &[MprisPlayerData],
    command: MprisPlayerCommand,
) -> Result<Vec<MprisPlayerData,>, ModuleError,>
{
    let target = data.iter().find(|entry| entry.service == command.service_name,);
    let player = target.ok_or_else(|| {
        ModuleError::registration(format!("unknown MPRIS service '{}'", command.service_name),)
    },)?;

    player.proxy.execute_command(&command.command,).await?;

    let names: Vec<String,> = data.iter().map(|entry| entry.service.clone(),).collect();
    Ok(ipc::fetch_players(conn, &names,).await,)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn command_builder_preserves_inputs()
    {
        let command = MprisPlayerCommand::new("svc".into(), PlayerCommand::Prev,);
        assert_eq!(command.service_name, "svc");
        match command.command {
            PlayerCommand::Prev => {}
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn module_error_formats_context()
    {
        let error = module_error("context", "failure",);
        assert!(matches!(
            error,
            ModuleError::Registration {
                reason: ref value
            } if value == "context: failure"
        ));
        assert_eq!(format!("{error}"), "Module registration failed: context: failure");
    }
}
