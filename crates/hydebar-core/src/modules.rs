/// Core module declarations - Business logic only, no GUI!
use std::borrow::Cow;

use masterror::AppError;

use crate::{event_bus::EventBusError, menu::MenuType};

pub mod app_launcher;
pub mod battery;
pub mod clipboard;
pub mod clock;
pub mod custom_module;
pub mod keyboard_layout;
pub mod keyboard_submap;
pub mod media_player;
pub mod notifications;
pub mod privacy;
pub mod settings;
pub mod system_info;
pub mod tray;
pub mod updates;
pub mod window_title;
pub mod workspaces;

/// Action to perform when a module is pressed
#[derive(Debug, Clone,)]
pub enum OnModulePress<M,>
{
    Action(Box<M,>,),
    ToggleMenu(MenuType,),
}

/// Module registration and operation errors
#[derive(Debug, Clone, PartialEq,)]
pub enum ModuleError
{
    EventBus(EventBusError,),
    Registration
    {
        reason: Cow<'static, str,>,
    },
}

impl std::fmt::Display for ModuleError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result
    {
        match self {
            Self::EventBus(err,) => write!(f, "Module event bus interaction failed: {}", err),
            Self::Registration {
                reason,
            } => write!(f, "Module registration failed: {}", reason),
        }
    }
}

impl std::error::Error for ModuleError {}

impl From<EventBusError,> for ModuleError
{
    fn from(err: EventBusError,) -> Self
    {
        Self::EventBus(err,)
    }
}

impl From<ModuleError,> for AppError
{
    fn from(err: ModuleError,) -> Self
    {
        match err {
            ModuleError::EventBus(_,) => AppError::internal(err.to_string(),),
            ModuleError::Registration {
                ..
            } => AppError::validation(err.to_string(),),
        }
    }
}

impl ModuleError
{
    pub fn registration(reason: impl Into<Cow<'static, str,>,>,) -> Self
    {
        Self::Registration {
            reason: reason.into(),
        }
    }
}

/// Behaviour shared by all UI modules rendered inside the bar.
///
/// NOTE: This trait is being phased out in favor of clean architecture.
/// New modules should follow the Battery pattern: separate data/logic (core)
/// from rendering (gui).
pub trait Module<Message,>
{
    type ViewData<'a,>;
    type RegistrationData<'a,>;

    fn register(
        &mut self,
        ctx: &crate::module_context::ModuleContext,
        data: Self::RegistrationData<'_,>,
    ) -> Result<(), ModuleError,>
    {
        let _ = (ctx, data,);
        Ok((),)
    }

    fn view(
        &self,
        data: Self::ViewData<'_,>,
    ) -> Option<(iced::Element<'static, Message,>, Option<OnModulePress<Message,>,>,),>
    {
        let _ = data;
        None
    }

    fn subscription(&self,) -> Option<iced::Subscription<Message,>,>
    {
        None
    }
}
