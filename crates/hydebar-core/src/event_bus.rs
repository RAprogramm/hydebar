use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use crate::modules;
use masterror::AppError;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BusEvent {
    Redraw,
    PopupToggle,
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

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ModuleEvent {
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::TrayMessage),
    Clock(modules::clock::Message),
    Battery(modules::battery::Message),
    Privacy(modules::privacy::PrivacyMessage),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
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

#[derive(Debug, Clone)]
pub enum EventBusError {
    QueueFull { capacity: usize },
    Poisoned,
}

impl std::fmt::Display for EventBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueFull { capacity } => {
                write!(f, "Event queue is full (capacity: {})", capacity)
            }
            Self::Poisoned => write!(f, "Event queue state is poisoned"),
        }
    }
}

impl std::error::Error for EventBusError {}

impl From<EventBusError> for AppError {
    fn from(err: EventBusError) -> Self {
        match err {
            EventBusError::QueueFull { .. } => AppError::internal(err.to_string()),
            EventBusError::Poisoned => AppError::internal(err.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

impl EventBus {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            inner: Arc::new(EventBusInner::new(capacity)),
        }
    }

    pub fn sender(&self) -> EventSender {
        EventSender {
            inner: Arc::clone(&self.inner),
        }
    }

    pub fn receiver(&self) -> EventReceiver {
        EventReceiver {
            inner: Arc::clone(&self.inner),
        }
    }

    pub fn publish(&self, event: BusEvent) -> Result<(), EventBusError> {
        let mut queue = self
            .inner
            .queue
            .lock()
            .map_err(|_| EventBusError::Poisoned)?;

        if queue.len() >= self.inner.capacity {
            return Err(EventBusError::QueueFull {
                capacity: self.inner.capacity,
            });
        }

        if let Some(last) = queue.back() {
            if event.is_coalescable_with(last) {
                return Ok(());
            }
        }

        queue.push_back(event);
        Ok(())
    }

    pub fn drain(&self) -> Result<Vec<BusEvent>, EventBusError> {
        let mut queue = self
            .inner
            .queue
            .lock()
            .map_err(|_| EventBusError::Poisoned)?;

        Ok(queue.drain(..).collect())
    }
}

#[derive(Debug, Clone)]
pub struct EventSender {
    inner: Arc<EventBusInner>,
}

impl EventSender {
    pub fn try_send(&self, event: BusEvent) -> Result<(), EventBusError> {
        let mut queue = self
            .inner
            .queue
            .lock()
            .map_err(|_| EventBusError::Poisoned)?;

        if queue.len() >= self.inner.capacity {
            return Err(EventBusError::QueueFull {
                capacity: self.inner.capacity,
            });
        }

        if let Some(last) = queue.back() {
            if event.is_coalescable_with(last) {
                return Ok(());
            }
        }

        queue.push_back(event);
        Ok(())
    }
}

#[derive(Debug)]
pub struct EventReceiver {
    inner: Arc<EventBusInner>,
}

impl EventReceiver {
    pub fn try_recv(&mut self) -> Result<Option<BusEvent>, EventBusError> {
        let mut queue = self
            .inner
            .queue
            .lock()
            .map_err(|_| EventBusError::Poisoned)?;

        Ok(queue.pop_front())
    }
}
