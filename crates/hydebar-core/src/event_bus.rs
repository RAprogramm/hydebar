use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use crate::modules;

use thiserror::Error;

/// High-level events emitted by the core to drive UI updates.
///
/// The enum is marked as `#[non_exhaustive]` to allow additional
/// variants without breaking downstream consumers.
///
/// # Examples
///
/// ```
/// # use hydebar_core::event_bus::BusEvent;
/// let event = BusEvent::Redraw;
/// assert!(matches!(event, BusEvent::Redraw));
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BusEvent {
    /// Request a redraw of the main surface.
    Redraw,
    /// Toggle the visibility of popup menus.
    PopupToggle,
    /// Module-level events that carry payloads for the GUI bridge.
    Module(ModuleEvent),
}

impl BusEvent {
    fn is_coalescable_with(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (BusEvent::Redraw, BusEvent::Redraw) | (BusEvent::PopupToggle, BusEvent::PopupToggle)
        )
    }
}

/// Events originating from individual modules.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ModuleEvent {
    /// Updates module state or view logic.
    Updates(modules::updates::Message),
    /// Workspace module activity.
    Workspaces(modules::workspaces::Message),
    /// Window title state changes.
    WindowTitle(modules::window_title::Message),
    /// System information updates.
    SystemInfo(modules::system_info::Message),
    /// Keyboard layout refreshes.
    KeyboardLayout(modules::keyboard_layout::Message),
    /// Keyboard submap notifications.
    KeyboardSubmap(modules::keyboard_submap::Message),
    /// Tray module interactions.
    Tray(modules::tray::TrayMessage),
    /// Clock module updates.
    Clock(modules::clock::Message),
    /// Battery module updates.
    Battery(modules::battery::Message),
    /// Privacy module updates.
    Privacy(modules::privacy::PrivacyMessage),
    /// Settings module events.
    Settings(modules::settings::Message),
    /// Media player module events.
    MediaPlayer(modules::media_player::Message),
    /// Custom module events keyed by module name.
    Custom {
        name: Arc<str>,
        message: modules::custom_module::Message,
    },
}

#[derive(Debug)]
struct EventBusInner {
    queue: Mutex<VecDeque<BusEvent>>,
    capacity: usize,
}

impl EventBusInner {
    fn new(capacity: NonZeroUsize) -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(capacity.get())),
            capacity: capacity.get(),
        }
    }
}

/// Error returned when interacting with the [`EventBus`].
///
/// # Examples
///
/// ```
/// # use hydebar_core::event_bus::EventBusError;
/// let error = EventBusError::QueueFull { capacity: 4 };
/// assert!(matches!(error, EventBusError::QueueFull { .. }));
/// ```
#[derive(Debug, Error)]
pub enum EventBusError {
    /// The queue reached its configured capacity.
    #[error("event queue is full (capacity: {capacity})")]
    QueueFull { capacity: usize },
    /// The queue mutex was poisoned by a panic in another thread.
    #[error("event queue state is poisoned")]
    Poisoned,
}

/// In-memory queue that coalesces redraw-heavy events between dispatch cycles.
///
/// # Preconditions
///
/// - `capacity` passed to [`EventBus::new`] must be greater than zero.
///
/// # Postconditions
///
/// - The queue starts empty with enough capacity for `capacity` events.
///
/// # Examples
///
/// ```
/// # use hydebar_core::event_bus::{BusEvent, EventBus};
/// # use std::num::NonZeroUsize;
/// let bus = EventBus::new(NonZeroUsize::new(8).expect("non-zero"));
/// let sender = bus.sender();
/// let mut receiver = bus.receiver();
/// sender.try_send(BusEvent::Redraw).expect("send");
/// assert_eq!(receiver.try_recv().expect("receive"), Some(BusEvent::Redraw));
/// ```
#[derive(Debug, Clone)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

impl EventBus {
    /// Construct a bus with the provided capacity.
    ///
    /// # Preconditions
    ///
    /// - `capacity` must be non-zero.
    ///
    /// # Postconditions
    ///
    /// - Returns a bus with an empty queue sized for `capacity` items.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::event_bus::{BusEvent, EventBus};
    /// # use std::num::NonZeroUsize;
    /// let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
    /// let sender = bus.sender();
    /// sender.try_send(BusEvent::Redraw).expect("send");
    /// ```
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            inner: Arc::new(EventBusInner::new(capacity)),
        }
    }

    /// Acquire a sender handle tied to the bus.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::event_bus::{BusEvent, EventBus};
    /// # use std::num::NonZeroUsize;
    /// let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
    /// let sender = bus.sender();
    /// sender.try_send(BusEvent::Redraw).expect("send");
    /// ```
    pub fn sender(&self) -> EventSender {
        EventSender {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Acquire a receiver handle tied to the bus.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::event_bus::{BusEvent, EventBus};
    /// # use std::num::NonZeroUsize;
    /// let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
    /// let mut receiver = bus.receiver();
    /// assert_eq!(receiver.try_recv().expect("empty"), None);
    /// ```
    pub fn receiver(&self) -> EventReceiver {
        EventReceiver {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Handle used to enqueue events.
///
/// # Examples
///
/// ```
/// # use hydebar_core::event_bus::{BusEvent, EventBus};
/// # use std::num::NonZeroUsize;
/// let bus = EventBus::new(NonZeroUsize::new(2).expect("capacity"));
/// let sender = bus.sender();
/// sender.try_send(BusEvent::Redraw).expect("send");
/// ```
#[derive(Debug, Clone)]
pub struct EventSender {
    inner: Arc<EventBusInner>,
}

impl EventSender {
    /// Attempt to enqueue a new event.
    ///
    /// # Preconditions
    ///
    /// - The underlying queue must not already be at capacity.
    ///
    /// # Postconditions
    ///
    /// - Enqueues `event` unless it is coalesced with an existing [`BusEvent::Redraw`]
    ///   or [`BusEvent::PopupToggle`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::event_bus::{BusEvent, EventBus};
    /// # use std::num::NonZeroUsize;
    /// let bus = EventBus::new(NonZeroUsize::new(2).expect("capacity"));
    /// let sender = bus.sender();
    /// sender.try_send(BusEvent::Redraw).expect("send");
    /// ```
    pub fn try_send(&self, event: BusEvent) -> Result<(), EventBusError> {
        let mut queue = self
            .inner
            .queue
            .lock()
            .map_err(|_| EventBusError::Poisoned)?;

        if let Some(last) = queue.back() {
            if last.is_coalescable_with(&event) {
                return Ok(());
            }
        }

        if queue.len() >= self.inner.capacity {
            return Err(EventBusError::QueueFull {
                capacity: self.inner.capacity,
            });
        }

        queue.push_back(event);
        Ok(())
    }
}

/// Handle used to drain events in FIFO order.
///
/// # Examples
///
/// ```
/// # use hydebar_core::event_bus::{BusEvent, EventBus};
/// # use std::num::NonZeroUsize;
/// let bus = EventBus::new(NonZeroUsize::new(2).expect("capacity"));
/// let sender = bus.sender();
/// let mut receiver = bus.receiver();
/// sender.try_send(BusEvent::Redraw).expect("send");
/// assert_eq!(receiver.try_recv().expect("receive"), Some(BusEvent::Redraw));
/// ```
#[derive(Debug)]
pub struct EventReceiver {
    inner: Arc<EventBusInner>,
}

impl EventReceiver {
    /// Attempt to fetch the next event, returning `None` if the queue is empty.
    ///
    /// # Postconditions
    ///
    /// - Removes and returns the front event if one exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::event_bus::{BusEvent, EventBus};
    /// # use std::num::NonZeroUsize;
    /// let bus = EventBus::new(NonZeroUsize::new(1).expect("capacity"));
    /// let sender = bus.sender();
    /// let mut receiver = bus.receiver();
    /// sender.try_send(BusEvent::Redraw).expect("send");
    /// assert_eq!(receiver.try_recv().expect("receive"), Some(BusEvent::Redraw));
    /// ```
    pub fn try_recv(&mut self) -> Result<Option<BusEvent>, EventBusError> {
        let mut queue = self
            .inner
            .queue
            .lock()
            .map_err(|_| EventBusError::Poisoned)?;
        Ok(queue.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_consecutive_redraw_events() {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let sender = bus.sender();
        let mut receiver = bus.receiver();

        sender.try_send(BusEvent::Redraw).expect("first send");
        sender.try_send(BusEvent::Redraw).expect("coalesced send");

        assert!(matches!(
            receiver.try_recv().expect("receive"),
            Some(BusEvent::Redraw)
        ));
        assert_eq!(receiver.try_recv().expect("empty"), None);
    }

    #[test]
    fn coalesces_consecutive_popup_toggle_events() {
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let sender = bus.sender();
        let mut receiver = bus.receiver();

        sender.try_send(BusEvent::PopupToggle).expect("first send");
        sender
            .try_send(BusEvent::PopupToggle)
            .expect("coalesced send");

        assert!(matches!(
            receiver.try_recv().expect("receive"),
            Some(BusEvent::PopupToggle)
        ));
        assert_eq!(receiver.try_recv().expect("empty"), None);
    }

    #[test]
    fn preserves_non_coalescable_ordering() {
        let bus = EventBus::new(NonZeroUsize::new(8).expect("capacity"));
        let sender = bus.sender();
        let mut receiver = bus.receiver();

        sender.try_send(BusEvent::Redraw).expect("send redraw");
        sender
            .try_send(BusEvent::Module(ModuleEvent::Updates(
                modules::updates::Message::CheckNow,
            )))
            .expect("send module");
        sender.try_send(BusEvent::Redraw).expect("send redraw");
        sender.try_send(BusEvent::PopupToggle).expect("send popup");
        sender
            .try_send(BusEvent::PopupToggle)
            .expect("coalesced popup");

        assert!(matches!(
            receiver.try_recv().expect("receive"),
            Some(BusEvent::Redraw)
        ));
        assert!(matches!(
            receiver.try_recv().expect("receive"),
            Some(BusEvent::Module(ModuleEvent::Updates(
                modules::updates::Message::CheckNow
            )))
        ));
        assert!(matches!(
            receiver.try_recv().expect("receive"),
            Some(BusEvent::Redraw)
        ));
        assert!(matches!(
            receiver.try_recv().expect("receive"),
            Some(BusEvent::PopupToggle)
        ));
        assert_eq!(receiver.try_recv().expect("empty"), None);
    }

    #[test]
    fn respects_bounded_capacity() {
        let bus = EventBus::new(NonZeroUsize::new(2).expect("capacity"));
        let sender = bus.sender();
        let mut receiver = bus.receiver();

        sender.try_send(BusEvent::Redraw).expect("first event");
        sender
            .try_send(BusEvent::Module(ModuleEvent::Updates(
                modules::updates::Message::CheckNow,
            )))
            .expect("second event");

        let overflow = sender.try_send(BusEvent::PopupToggle);
        assert!(matches!(
            overflow,
            Err(EventBusError::QueueFull { capacity: 2 })
        ));

        assert!(matches!(
            receiver.try_recv().expect("receive"),
            Some(BusEvent::Redraw)
        ));
        assert!(matches!(
            receiver.try_recv().expect("receive"),
            Some(BusEvent::Module(ModuleEvent::Updates(
                modules::updates::Message::CheckNow
            )))
        ));
        assert_eq!(receiver.try_recv().expect("empty"), None);
    }
}
