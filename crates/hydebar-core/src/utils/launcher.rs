use std::{
    process::{ExitStatus, Output},
    sync::Arc,
};

use log::error;
use tokio::process::Command;

/// Error type emitted when launching shell commands fails.
///
/// The error keeps a shared reference to the original command string so callers
/// can differentiate failures per command without cloning large buffers.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
///
/// use hydebar::utils::launcher::{LauncherError, run_shell_command_with_output};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let runtime = tokio::runtime::Runtime::new()?;
/// let command = Arc::from("true",);
/// runtime.block_on(async move {
///     let output = run_shell_command_with_output(&command,).await?;
///     assert!(output.status.success());
///     Ok::<(), LauncherError,>((),)
/// },)?;
/// Ok((),)
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq,)]
pub enum LauncherError
{
    /// The command could not be spawned by the operating system.
    Spawn
    {
        /// The attempted command string.
        command: Arc<str,>,
        /// Additional context provided by the OS error.
        context: Arc<str,>,
    },
    /// The command executed but returned a non-zero exit status.
    NonZeroExit
    {
        /// The attempted command string.
        command: Arc<str,>,
        /// The exit status returned by the process.
        status:  ExitStatus,
    },
}

impl std::fmt::Display for LauncherError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result
    {
        match self {
            Self::Spawn {
                command,
                context,
            } => {
                write!(f, "failed to spawn `{}`: {}", command, context)
            }
            Self::NonZeroExit {
                command,
                status,
            } => {
                write!(f, "command `{}` exited with status {}", command, status)
            }
        }
    }
}

impl std::error::Error for LauncherError {}

impl LauncherError
{
    fn spawn_error(command: Arc<str,>, error: std::io::Error,) -> Self
    {
        Self::Spawn {
            command,
            context: Arc::from(error.to_string(),),
        }
    }

    fn exit_error(command: Arc<str,>, status: ExitStatus,) -> Self
    {
        Self::NonZeroExit {
            command,
            status,
        }
    }
}

/// Execute the given command and return its stdout/stderr output.
///
/// # Errors
///
/// Returns [`LauncherError::Spawn`] if the process cannot be created or
/// [`LauncherError::NonZeroExit`] when the command finishes unsuccessfully.
pub async fn run_shell_command_with_output(command: &Arc<str,>,)
-> Result<Output, LauncherError,>
{
    let mut process = Command::new("bash",);
    process.arg("-c",).arg(command.as_ref(),);

    let output = process
        .output()
        .await
        .map_err(|error| LauncherError::spawn_error(command.clone(), error,),)?;

    if output.status.success() {
        Ok(output,)
    } else {
        Err(LauncherError::exit_error(command.clone(), output.status,),)
    }
}

fn spawn_and_log(command: String, context: &'static str,)
{
    tokio::spawn(async move {
        let command_arc: Arc<str,> = Arc::from(command,);
        match run_shell_command_with_output(&command_arc,).await {
            Ok(output,) => {
                if !output.stderr.is_empty() {
                    error!(
                        "{context} command produced stderr: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            Err(error,) => {
                error!("{context} command failed: {error}");
            }
        }
    },);
}

/// Execute an arbitrary shell command without awaiting its completion.
///
/// The command is executed in a background Tokio task to preserve the
/// fire-and-forget semantics used throughout the UI.
///
/// # Examples
///
/// ```no_run
/// use hydebar::utils::launcher;
///
/// launcher::execute_command("notify-send hydebar 'Hello'".to_owned(),);
/// ```
pub fn execute_command(command: String,)
{
    spawn_and_log(command, "launcher",);
}

/// Execute the configured suspend command in the background.
pub fn suspend(command: String,)
{
    spawn_and_log(command, "suspend",);
}

/// Execute the configured shutdown command in the background.
pub fn shutdown(command: String,)
{
    spawn_and_log(command, "shutdown",);
}

/// Execute the configured reboot command in the background.
pub fn reboot(command: String,)
{
    spawn_and_log(command, "reboot",);
}

/// Execute the configured logout command in the background.
pub fn logout(command: String,)
{
    spawn_and_log(command, "logout",);
}

#[cfg(test)]
mod tests
{
    use std::{sync::Arc, time::Duration};

    use super::{LauncherError, run_shell_command_with_output};

    #[tokio::test]
    async fn reports_successful_status() -> Result<(), Box<dyn std::error::Error,>,>
    {
        let command = Arc::from("true",);

        let output = tokio::time::timeout(
            Duration::from_secs(5,),
            run_shell_command_with_output(&command,),
        )
        .await??;

        assert!(output.status.success());

        Ok((),)
    }

    #[tokio::test]
    async fn reports_non_zero_exit_as_error() -> Result<(), Box<dyn std::error::Error,>,>
    {
        let command = Arc::from("exit 42",);

        let outcome = tokio::time::timeout(
            Duration::from_secs(5,),
            run_shell_command_with_output(&command,),
        )
        .await?;

        match outcome {
            Err(LauncherError::NonZeroExit {
                status, ..
            },) => {
                assert_eq!(status.code(), Some(42));
            }
            other => {
                return Err(format!("unexpected outcome: {other:?}").into(),);
            }
        }

        Ok((),)
    }

    #[tokio::test]
    async fn captures_command_output() -> Result<(), Box<dyn std::error::Error,>,>
    {
        let command = Arc::from("printf foo",);

        let output = tokio::time::timeout(
            Duration::from_secs(5,),
            run_shell_command_with_output(&command,),
        )
        .await??;

        assert_eq!(output.stdout, b"foo");
        assert!(output.stderr.is_empty());
        assert!(output.status.success());

        Ok((),)
    }
}
