use std::process::{ExitStatus, Stdio};

use tokio::process;

use super::state::Update;

/// Errors that can occur while executing an update-related shell command.
#[derive(Debug, thiserror::Error)]
pub(super) enum CommandError {
    /// Failed to spawn the command.
    #[error("failed to execute command")]
    Io(#[from] std::io::Error),
    /// The command exited with a non-zero status.
    #[error("command exited with failure status: {0}")]
    Status(ExitStatus),
    /// The command produced output that was not valid UTF-8.
    #[error("command output was not valid UTF-8")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

impl CommandError {
    pub(super) fn or_log(self, context: &str) {
        log::warn!("{context}: {self}");
    }
}

pub(super) async fn check_for_updates(command: &str) -> Result<Vec<Update>, CommandError> {
    let output = process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        return Err(CommandError::Status(output.status));
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(parse_updates(stdout.trim_end_matches('\n')))
}

pub(super) async fn apply_updates(command: &str) -> Result<(), CommandError> {
    let output = process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !output.success() {
        return Err(CommandError::Status(output));
    }

    Ok(())
}

fn parse_updates(output: &str) -> Vec<Update> {
    output.lines().filter_map(parse_update_line).collect()
}

fn parse_update_line(line: &str) -> Option<Update> {
    let mut tokens = line.split_whitespace();
    let package = tokens.next()?;
    let from = tokens.next()?;
    let separator = tokens.next()?;
    let to = tokens.next()?;

    if separator != "->" {
        return None;
    }

    Some(Update {
        package: package.to_owned(),
        from: from.to_owned(),
        to: to.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_updates_skips_malformed_lines() {
        let output = "pkg1 1 -> 2\ninvalid line\npkg2 3 -> 4";

        let updates = parse_updates(output);

        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].package, "pkg1");
        assert_eq!(updates[1].package, "pkg2");
    }

    #[test]
    fn parse_updates_handles_empty_input() {
        let updates = parse_updates("");

        assert!(updates.is_empty());
    }
}
