/// Core module declarations - Business logic only, no GUI!
use std::borrow::Cow;

use crate::{event_bus::EventBusError, menu::MenuType};
use thiserror::Error;

// Module declarations - each contains business logic
pub mod app_launcher;
pub mod battery;
pub mod clipboard;
pub mod clock;
pub mod custom_module;
pub mod keyboard_layout;
pub mod keyboard_submap;
pub mod media_player;
pub mod privacy;
pub mod settings;
pub mod system_info;
pub mod tray;
pub mod updates;
pub mod window_title;
pub mod workspaces;

/// Action to perform when a module is pressed
/// Generic over Message type to avoid GUI dependencies in core
#[derive(Debug, Clone)]
pub enum OnModulePress<M> {
    Action(Box<M>),
    ToggleMenu(MenuType),
}

/// Errors that can occur while registering a module.
#[derive(Debug, Error, Clone)]
pub enum ModuleError {
    /// Propagates failures originating from the event bus.
    #[error("module event bus interaction failed: {0}")]
    EventBus(#[from] EventBusError),
    /// Domain-specific registration failures surfaced by the module.
    #[error("module registration failed: {reason}")]
    Registration { reason: Cow<'static, str> },
}

impl ModuleError {
    /// Construct a registration error with the provided reason.
    pub fn registration(reason: impl Into<Cow<'static, str>>) -> Self {
        Self::Registration {
            reason: reason.into(),
        }
    }
}

/// Behaviour shared by all UI modules rendered inside the bar.
///
/// NOTE: This trait is being phased out in favor of clean architecture.
/// New modules should follow the Battery pattern: separate data/logic (core) from rendering (gui).
///
/// Modules receive configuration snapshots as [`ViewData`](Module::ViewData) when rendering and
/// may opt into background work by overriding [`subscription`](Module::subscription). The
/// [`register`](Module::register) hook exposes the shared [`ModuleContext`], allowing modules to
/// cache typed event senders or eagerly request redraws during initialisation.
pub trait Module {
    type ViewData<'a>;
    type RegistrationData<'a>;

    /// Register the module with the shared runtime context.
    ///
    /// The default implementation performs no work. Implementations can use the [`ModuleContext`]
    /// to, for example, acquire a [`ModuleEventSender`](crate::ModuleEventSender) tied to their
    /// event enum:
    ///
    /// ```no_run
    /// use hydebar_core::event_bus::ModuleEvent;
    /// use hydebar_core::modules::{Module, ModuleError};
    /// use hydebar_core::ModuleContext;
    ///
    /// #[derive(Default)]
    /// struct ExampleModule {
    ///     sender: Option<hydebar_core::ModuleEventSender<ExampleMessage>>,
    /// }
    ///
    /// #[derive(Debug, Clone)]
    /// enum ExampleMessage {
    ///     Tick,
    /// }
    ///
    /// impl Module for ExampleModule {
    ///     type ViewData<'a> = ();
    ///     type RegistrationData<'a> = ();
    ///
    ///     fn register(
    ///         &mut self,
    ///         ctx: &ModuleContext,
    ///         _data: Self::RegistrationData<'_>,
    ///     ) -> Result<(), ModuleError> {
    ///         self.sender = Some(ctx.module_sender(ModuleEvent::Clock));
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn register(
        &mut self,
        ctx: &crate::module_context::ModuleContext,
        data: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        let _ = (ctx, data);
        Ok(())
    }

    // NOTE: view() and subscription() methods are implemented per-module
    // in the GUI layer now. This trait is kept for backward compatibility
    // with modules that haven't been refactored yet.
}
