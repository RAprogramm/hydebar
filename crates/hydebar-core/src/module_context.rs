use std::sync::Arc;

use tokio::runtime::Handle;

use crate::event_bus::{BusEvent, EventBusError, EventSender, ModuleEvent};

/// Shared utilities exposed to individual modules when they need to interact with
/// the core event loop.
///
/// The context owns an [`EventSender`] used to push [`BusEvent`] values into the UI
/// queue and a [`Handle`] tied to the runtime powering background tasks. Modules can
/// use the handle to spawn asynchronous work; those tasks must cooperate with
/// cancellation by completing promptly when dropped. Tokio ensures that futures
/// aborted through [`Handle::spawn`] tear down without panicking, and because event
/// publication is synchronous, no pending publishes are left behind when a task is
/// cancelled.
#[derive(Debug, Clone)]
pub struct ModuleContext {
    event_sender: EventSender,
    runtime_handle: Handle,
}

impl ModuleContext {
    /// Create a new context bound to the provided event sender and runtime handle.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::{event_bus::EventBus, module_context::ModuleContext};
    /// # use std::num::NonZeroUsize;
    /// # let runtime = tokio::runtime::Runtime::new().expect("runtime");
    /// let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
    /// let context = ModuleContext::new(bus.sender(), runtime.handle().clone());
    /// # drop(context);
    /// ```
    pub fn new(event_sender: EventSender, runtime_handle: Handle) -> Self {
        Self {
            event_sender,
            runtime_handle,
        }
    }

    /// Access the runtime handle used for spawning background tasks.
    ///
    /// # Safety and cancellation
    ///
    /// Futures spawned via this handle should be written to observe cooperative
    /// cancellation. When a task is aborted, Tokio guarantees that the future is
    /// dropped without panicking, ensuring that no partially published events remain
    /// in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::{event_bus::EventBus, module_context::ModuleContext};
    /// # use std::num::NonZeroUsize;
    /// # let runtime = tokio::runtime::Runtime::new().expect("runtime");
    /// let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
    /// let context = ModuleContext::new(bus.sender(), runtime.handle().clone());
    /// let handle = context.runtime_handle();
    /// handle.spawn(async {});
    /// ```
    pub fn runtime_handle(&self) -> &Handle {
        &self.runtime_handle
    }

    /// Request a redraw of the UI surface.
    ///
    /// # Postconditions
    ///
    /// - Enqueues a [`BusEvent::Redraw`] if the bus has remaining capacity, otherwise
    ///   returns [`EventBusError::QueueFull`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::{event_bus::EventBus, module_context::ModuleContext};
    /// # use std::num::NonZeroUsize;
    /// # let runtime = tokio::runtime::Runtime::new().expect("runtime");
    /// let bus = EventBus::new(NonZeroUsize::new(1).expect("capacity"));
    /// let context = ModuleContext::new(bus.sender(), runtime.handle().clone());
    /// context.request_redraw().expect("queued");
    /// ```
    pub fn request_redraw(&self) -> Result<(), EventBusError> {
        self.event_sender.try_send(BusEvent::Redraw)
    }

    /// Toggle the popup menu visibility.
    ///
    /// # Postconditions
    ///
    /// - Enqueues a [`BusEvent::PopupToggle`] if the bus has capacity, otherwise
    ///   returns [`EventBusError::QueueFull`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::{event_bus::EventBus, module_context::ModuleContext};
    /// # use std::num::NonZeroUsize;
    /// # let runtime = tokio::runtime::Runtime::new().expect("runtime");
    /// let bus = EventBus::new(NonZeroUsize::new(1).expect("capacity"));
    /// let context = ModuleContext::new(bus.sender(), runtime.handle().clone());
    /// context.toggle_popup().expect("queued");
    /// ```
    pub fn toggle_popup(&self) -> Result<(), EventBusError> {
        self.event_sender.try_send(BusEvent::PopupToggle)
    }

    fn publish_module_event(&self, event: ModuleEvent) -> Result<(), EventBusError> {
        self.event_sender.try_send(BusEvent::Module(event))
    }

    /// Build a type-safe module event sender from the provided conversion function.
    ///
    /// # Preconditions
    ///
    /// - `convert` must transform the module-specific payload into a [`ModuleEvent`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::{event_bus::EventBus, module_context::ModuleContext};
    /// # use hydebar_core::event_bus::ModuleEvent;
    /// # use hydebar_core::modules;
    /// # use std::num::NonZeroUsize;
    /// # let runtime = tokio::runtime::Runtime::new().expect("runtime");
    /// let bus = EventBus::new(NonZeroUsize::new(2).expect("capacity"));
    /// let context = ModuleContext::new(bus.sender(), runtime.handle().clone());
    /// let sender = context.module_sender(ModuleEvent::Updates);
    /// sender
    ///     .try_send(modules::updates::Message::CheckNow)
    ///     .expect("queued");
    /// ```
    pub fn module_sender<T, F>(&self, convert: F) -> ModuleEventSender<T>
    where
        T: Send + 'static,
        F: Fn(T) -> ModuleEvent + Send + Sync + 'static,
    {
        ModuleEventSender {
            context: self.clone(),
            convert: Arc::new(convert),
        }
    }
}

/// Strongly-typed wrapper around [`ModuleContext::publish_module_event`].
///
/// # Examples
///
/// ```
/// # use hydebar_core::{event_bus::EventBus, module_context::ModuleContext};
/// # use hydebar_core::event_bus::ModuleEvent;
/// # use hydebar_core::modules;
/// # use std::num::NonZeroUsize;
/// # let runtime = tokio::runtime::Runtime::new().expect("runtime");
/// let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
/// let context = ModuleContext::new(bus.sender(), runtime.handle().clone());
/// let sender = context.module_sender(ModuleEvent::Updates);
/// sender
///     .try_send(modules::updates::Message::CheckNow)
///     .expect("queued");
/// ```
#[derive(Clone)]
pub struct ModuleEventSender<T> {
    context: ModuleContext,
    convert: Arc<dyn Fn(T) -> ModuleEvent + Send + Sync>,
}

impl<T> ModuleEventSender<T>
where
    T: Send + 'static,
{
    /// Convert the payload into a [`ModuleEvent`] and enqueue it on the bus.
    ///
    /// # Postconditions
    ///
    /// - Returns [`Ok`] if the event is successfully queued, otherwise propagates
    ///   [`EventBusError`] from the underlying [`EventSender`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use hydebar_core::{event_bus::EventBus, module_context::ModuleContext};
    /// # use hydebar_core::event_bus::ModuleEvent;
    /// # use hydebar_core::modules;
    /// # use std::num::NonZeroUsize;
    /// # let runtime = tokio::runtime::Runtime::new().expect("runtime");
    /// let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
    /// let context = ModuleContext::new(bus.sender(), runtime.handle().clone());
    /// let sender = context.module_sender(ModuleEvent::Updates);
    /// sender
    ///     .try_send(modules::updates::Message::CheckNow)
    ///     .expect("queued");
    /// ```
    pub fn try_send(&self, payload: T) -> Result<(), EventBusError> {
        let event = (self.convert)(payload);
        self.context.publish_module_event(event)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use tokio::runtime::Runtime;

    use crate::event_bus::{BusEvent, EventBus, ModuleEvent};
    use crate::modules;

    use super::ModuleContext;

    #[test]
    fn request_redraw_enqueues_event() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let sender = bus.sender();
        let mut receiver = bus.receiver();
        let context = ModuleContext::new(sender, runtime.handle().clone());

        context.request_redraw().expect("redraw enqueued");

        let event = receiver.try_recv().expect("receive");
        assert!(matches!(event, Some(BusEvent::Redraw)));
    }

    #[test]
    fn toggle_popup_enqueues_event() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let sender = bus.sender();
        let mut receiver = bus.receiver();
        let context = ModuleContext::new(sender, runtime.handle().clone());

        context.toggle_popup().expect("popup enqueued");

        let event = receiver.try_recv().expect("receive");
        assert!(matches!(event, Some(BusEvent::PopupToggle)));
    }

    #[test]
    fn module_sender_enqueues_module_event() {
        let runtime = Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let sender = bus.sender();
        let mut receiver = bus.receiver();
        let context = ModuleContext::new(sender, runtime.handle().clone());

        let updates_sender = context.module_sender(ModuleEvent::Updates);
        updates_sender
            .try_send(modules::updates::Message::CheckNow)
            .expect("module enqueued");

        let event = receiver.try_recv().expect("receive");
        assert!(matches!(
            event,
            Some(BusEvent::Module(ModuleEvent::Updates(
                modules::updates::Message::CheckNow
            )))
        ));
    }
}
