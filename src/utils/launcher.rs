use std::{
    process::{ExitStatus, Output},
    sync::Arc,
};

use log::error;
use masterror::Error;
use tokio::process::Command;

/// Error type emitted when launching shell commands fails.
///
/// The error keeps a shared reference to the original command string so callers can
/// differentiate failures per command without cloning large buffers.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
///
/// use hydebar::utils::launcher::{
///     run_shell_command,
///     CommandCapture,
///     CommandOutcome,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let runtime = tokio::runtime::Runtime::new()?;
/// let command = Arc::from("true");
/// runtime.block_on(async move {
///     match run_shell_command(&command, CommandCapture::Status).await? {
///         CommandOutcome::Status(status) => {
///             assert!(status.success());
///         }
///         CommandOutcome::Output(_) => unreachable!(),
///     }
///     Ok::<(), hydebar::utils::launcher::LauncherError>(())
/// })?;
/// Ok(())
/// # }
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LauncherError {
    /// The command could not be spawned by the operating system.
    #[error("failed to spawn `{command}`: {context}")]
    Spawn {
        /// The attempted command string.
        command: Arc<str>,
        /// Additional context provided by the OS error.
        context: Arc<str>,
    },
    /// The command executed but returned a non-zero exit status.
    #[error("command `{command}` exited with status {status}")]
    NonZeroExit {
        /// The attempted command string.
        command: Arc<str>,
        /// The exit status returned by the process.
        status: ExitStatus,
    },
}

impl LauncherError {
    fn spawn_error(command: Arc<str>, error: std::io::Error) -> Self {
        Self::Spawn {
            command,
            context: Arc::from(error.to_string()),
        }
    }

    fn exit_error(command: Arc<str>, status: ExitStatus) -> Self {
        Self::NonZeroExit { command, status }
    }
}

/// Determines whether the launcher should observe only the exit status or capture full
/// output of the executed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCapture {
    /// Await only the process exit status.
    Status,
    /// Capture stdout and stderr alongside the exit status.
    Output,
}

/// Outcome of a launched command depending on the capture mode requested.
#[derive(Debug)]
pub enum CommandOutcome {
    /// Process exit status when [`CommandCapture::Status`] was requested.
    Status(ExitStatus),
    /// Full process output when [`CommandCapture::Output`] was requested.
    Output(Output),
}

/// Launch `bash -c <command>` asynchronously using Tokio and return the observed result.
///
/// The function reuses a shared command string across errors to avoid unnecessary
/// allocations and reports non-zero exit codes as [`LauncherError::NonZeroExit`].
///
/// # Errors
///
/// Returns [`LauncherError::Spawn`] if the process cannot be created or
/// [`LauncherError::NonZeroExit`] when the command finishes unsuccessfully.
pub async fn run_shell_command(
    command: &Arc<str>,
    capture: CommandCapture,
) -> Result<CommandOutcome, LauncherError> {
    let mut process = Command::new("bash");
    process.arg("-c").arg(command.as_ref());

    match capture {
        CommandCapture::Status => {
            let status = process
                .status()
                .await
                .map_err(|error| LauncherError::spawn_error(command.clone(), error))?;

            if status.success() {
                Ok(CommandOutcome::Status(status))
            } else {
                Err(LauncherError::exit_error(command.clone(), status))
            }
        }
        CommandCapture::Output => {
            let output = process
                .output()
                .await
                .map_err(|error| LauncherError::spawn_error(command.clone(), error))?;

            if output.status.success() {
                Ok(CommandOutcome::Output(output))
            } else {
                Err(LauncherError::exit_error(command.clone(), output.status))
            }
        }
    }
}

fn spawn_and_log(command: String, context: &'static str) {
    tokio::spawn(async move {
        let command_arc: Arc<str> = Arc::from(command);
        if let Err(error) = run_shell_command(&command_arc, CommandCapture::Status).await {
            error!("{context} command failed: {error}");
        }
    });
}

/// Execute an arbitrary shell command without awaiting its completion.
///
/// The command is executed in a background Tokio task to preserve the fire-and-forget
/// semantics used throughout the UI.
///
/// # Examples
///
/// ```no_run
/// use hydebar::utils::launcher;
///
/// launcher::execute_command("notify-send hydebar 'Hello'".to_owned());
/// ```
pub fn execute_command(command: String) {
    spawn_and_log(command, "launcher");
}

/// Execute the configured suspend command in the background.
pub fn suspend(command: String) {
    spawn_and_log(command, "suspend");
}

/// Execute the configured shutdown command in the background.
pub fn shutdown(command: String) {
    spawn_and_log(command, "shutdown");
}

/// Execute the configured reboot command in the background.
pub fn reboot(command: String) {
    spawn_and_log(command, "reboot");
}

/// Execute the configured logout command in the background.
pub fn logout(command: String) {
    spawn_and_log(command, "logout");
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{CommandCapture, CommandOutcome, LauncherError, run_shell_command};

    #[tokio::test]
    async fn reports_successful_status() -> Result<(), Box<dyn std::error::Error>> {
        let command = Arc::from("true");

        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            run_shell_command(&command, CommandCapture::Status),
        )
        .await?;

        let status = match outcome? {
            CommandOutcome::Status(status) => status,
            CommandOutcome::Output(_) => {
                return Err("status capture expected".into());
            }
        };

        assert!(status.success());

        Ok(())
    }

    #[tokio::test]
    async fn reports_non_zero_exit_as_error() -> Result<(), Box<dyn std::error::Error>> {
        let command = Arc::from("exit 42");

        let outcome = tokio::time::timeout(
            Duration::from_secs(5),
            run_shell_command(&command, CommandCapture::Status),
        )
        .await?;

        match outcome {
            Err(LauncherError::NonZeroExit { status, .. }) => {
                assert_eq!(status.code(), Some(42));
            }
            other => {
                return Err(format!("unexpected outcome: {other:?}").into());
            }
        }

        Ok(())
    }
}
