use std::{sync::Arc, thread, time::Duration};

use hydebar_proto::ports::hyprland::HyprlandError;
use log::warn;

use super::{config::HyprlandClientConfig, util::calculate_retry_delay};

/// Execute a blocking Hyprland request in a worker thread and wait for it to
/// complete within the provided timeout.
pub(crate) fn execute_once<R, F>(
    operation: &'static str,
    timeout_dur: Duration,
    func: Arc<F>,
) -> Result<R, HyprlandError>
where
    R: Send + 'static,
    F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let result = func();
        if tx.send(result).is_err() {
            warn!(
                target: "hydebar::hyprland",
                "result receiver dropped before completion (operation={operation})"
            );
        }
    });

    match rx.recv_timeout(timeout_dur) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(HyprlandError::Timeout {
            operation,
            timeout: timeout_dur,
        }),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(HyprlandError::message(
            operation,
            "worker thread terminated before sending result",
        )),
    }
}

/// Execute a blocking Hyprland request with retry and backoff semantics derived
/// from [`HyprlandClientConfig`].
pub(crate) fn execute_with_retry<R, F>(
    config: &HyprlandClientConfig,
    operation: &'static str,
    func: F,
) -> Result<R, HyprlandError>
where
    R: Send + 'static,
    F: Fn() -> Result<R, HyprlandError> + Send + Sync + 'static,
{
    let func = Arc::new(func);
    let mut last_error = None;

    for attempt in 1..=config.retry_attempts {
        let func_clone = Arc::clone(&func);
        match execute_once(operation, config.request_timeout, func_clone) {
            Ok(result) => return Ok(result),
            Err(err) => {
                warn!(
                    target: "hydebar::hyprland",
                    "Hyprland operation failed (operation={operation}, attempt={attempt}, error={err})"
                );
                last_error = Some(err);
                if attempt < config.retry_attempts {
                    let delay = calculate_retry_delay(config.retry_backoff, attempt);
                    if !delay.is_zero() {
                        thread::sleep(delay);
                    }
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        HyprlandError::message(operation, "Hyprland operation failed without error detail")
    }))
}

// TODO: Fix broken tests
#[cfg(all(test, feature = "enable-broken-tests"))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn base_config() -> HyprlandClientConfig {
        HyprlandClientConfig {
            request_timeout: Duration::from_millis(50),
            listener_timeout: Duration::from_secs(1),
            retry_attempts: 3,
            retry_backoff: Duration::ZERO,
        }
    }

    #[test]
    fn execute_once_propagates_success() {
        let result = execute_once("test", Duration::from_secs(1), Arc::new(|| Ok(42)));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn execute_with_retry_eventually_succeeds() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        let result = execute_with_retry(&base_config(), "retry", move || {
            let value = counter_clone.fetch_add(1, Ordering::SeqCst);
            if value < 2 {
                Err(HyprlandError::message("retry", "try again"))
            } else {
                Ok(value)
            }
        });

        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn execute_with_retry_returns_last_error() {
        let error = execute_with_retry(&base_config(), "retry", || -> Result<(), HyprlandError> {
            Err(HyprlandError::message("retry", "failed"))
        })
        .unwrap_err();

        assert!(matches!(
            error,
            HyprlandError::Backend { .. }
                | HyprlandError::Message { .. }
                | HyprlandError::Timeout { .. }
        ));
    }
}
