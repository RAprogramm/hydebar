use std::sync::{Arc, Mutex};

use hydebar_core::event_bus::{BusEvent, EventReceiver};
use log::error;

#[derive(Debug, Clone)]
pub(super) struct BusFlushOutcome {
    events: Vec<BusEvent>,
    had_error: bool,
}

impl BusFlushOutcome {
    pub(super) fn empty() -> Self {
        Self {
            events: Vec::new(),
            had_error: false,
        }
    }

    pub(super) fn with_events(events: Vec<BusEvent>, had_error: bool) -> Self {
        Self { events, had_error }
    }

    pub(super) fn had_error(&self) -> bool {
        self.had_error
    }

    pub(super) fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(super) fn into_events(self) -> Vec<BusEvent> {
        self.events
    }
}

pub(super) async fn drain_bus(receiver: Arc<Mutex<EventReceiver>>) -> BusFlushOutcome {
    let mut guard = match receiver.lock() {
        Ok(guard) => guard,
        Err(err) => {
            error!("event bus receiver poisoned: {err}");
            return BusFlushOutcome::with_events(Vec::new(), true);
        }
    };

    let mut events = Vec::new();
    let mut had_error = false;

    loop {
        match guard.try_recv() {
            Ok(Some(event)) => events.push(event),
            Ok(None) => break,
            Err(err) => {
                error!("failed to read event bus payload: {err}");
                had_error = true;
                break;
            }
        }
    }

    BusFlushOutcome::with_events(events, had_error)
}
