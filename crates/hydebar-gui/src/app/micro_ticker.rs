use std::time::Duration;

#[derive(Debug, Clone)]
pub(super) struct MicroTicker {
    fast_interval: Duration,
    slow_interval: Duration,
    idle_threshold: u8,
    idle_ticks: u8,
    current_interval: Duration,
}

impl MicroTicker {
    pub(super) fn new(
        fast_interval: Duration,
        slow_interval: Duration,
        idle_threshold: u8,
    ) -> Self {
        Self {
            fast_interval,
            slow_interval,
            idle_threshold,
            idle_ticks: 0,
            current_interval: fast_interval,
        }
    }

    pub(super) fn interval(&self) -> Duration {
        self.current_interval
    }

    pub(super) fn record_activity(&mut self) {
        self.idle_ticks = 0;
        self.current_interval = self.fast_interval;
    }

    pub(super) fn record_idle(&mut self) {
        if self.idle_ticks < self.idle_threshold {
            self.idle_ticks += 1;
        }

        if self.idle_ticks >= self.idle_threshold {
            self.current_interval = self.slow_interval;
        }
    }
}

impl Default for MicroTicker {
    fn default() -> Self {
        Self::new(Duration::from_millis(16), Duration::from_millis(33), 3)
    }
}
