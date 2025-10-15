use std::time::Duration;

/// Configuration options for [`HyprlandClient`](super::HyprlandClient).
///
/// # Examples
///
/// ```no_run
/// use hydebar_core::adapters::hyprland_client::{HyprlandClient, HyprlandClientConfig};
///
/// let client = HyprlandClient::with_config(HyprlandClientConfig::default());
/// assert!(client.active_window().is_ok());
/// ```
#[derive(Clone, Debug)]
pub struct HyprlandClientConfig {
    /// Maximum duration to wait for a synchronous Hyprland request to complete.
    pub request_timeout:  Duration,
    /// Maximum time to wait for the Hyprland event listener to yield before
    /// treating it as hung.
    pub listener_timeout: Duration,
    /// Total number of retry attempts for synchronous Hyprland requests.
    pub retry_attempts:   u8,
    /// Base delay between retry attempts for synchronous Hyprland requests.
    pub retry_backoff:    Duration
}

impl Default for HyprlandClientConfig {
    fn default() -> Self {
        Self {
            request_timeout:  Duration::from_secs(2),
            listener_timeout: Duration::from_secs(60),
            retry_attempts:   3,
            retry_backoff:    Duration::from_millis(250)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::HyprlandClientConfig;

    #[test]
    fn default_values_are_sensible() {
        let config = HyprlandClientConfig::default();

        assert_eq!(config.request_timeout, Duration::from_secs(2));
        assert_eq!(config.listener_timeout, Duration::from_secs(60));
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.retry_backoff, Duration::from_millis(250));
    }
}
