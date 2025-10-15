use std::time::Duration;

use tokio::time::sleep;

/// Compute the delay to wait before retrying an operation using linear backoff.
///
/// The returned duration is `base_backoff * attempt` with saturating
/// multiplication.
///
/// # Examples
///
/// ```ignore
/// use std::time::Duration;
/// use hydebar_core::adapters::hyprland_client::util::calculate_retry_delay;
///
/// let delay = calculate_retry_delay(Duration::from_millis(100), 3);
/// assert_eq!(delay, Duration::from_millis(300));
/// ```
pub(crate) fn calculate_retry_delay(base_backoff: Duration, attempt: u8) -> Duration {
    if attempt == 0 {
        return Duration::ZERO;
    }

    base_backoff.saturating_mul(u32::from(attempt))
}

/// Sleep for the provided backoff duration if it is non-zero.
///
/// This helper keeps listener retry loops concise and avoids duplicating the
/// zero-duration guard at each call site.
pub(crate) async fn sleep_with_backoff(backoff: Duration) {
    if backoff.is_zero() {
        return;
    }

    sleep(backoff).await;
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use super::{calculate_retry_delay, sleep_with_backoff};

    #[test]
    fn retry_delay_uses_linear_backoff() {
        assert_eq!(
            calculate_retry_delay(Duration::from_millis(50), 0),
            Duration::ZERO
        );
        assert_eq!(
            calculate_retry_delay(Duration::from_millis(50), 1),
            Duration::from_millis(50)
        );
        assert_eq!(
            calculate_retry_delay(Duration::from_millis(50), 2),
            Duration::from_millis(100)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn sleep_with_zero_backoff_returns_immediately() {
        sleep_with_backoff(Duration::ZERO).await;
    }

    #[tokio::test(start_paused = true)]
    async fn sleep_with_positive_backoff_awaits_duration() {
        let duration = Duration::from_millis(250);
        let start = tokio::time::Instant::now();
        sleep_with_backoff(duration).await;
        assert_eq!(tokio::time::Instant::now() - start, duration);
    }
}
