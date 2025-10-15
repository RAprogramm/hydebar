use iced::Element;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext,
    components::icons::{Icons, icon}
};

#[derive(Default, Debug, Clone)]
pub struct Clipboard;

impl<M> Module<M> for Clipboard
where
    M: 'static + Clone
{
    type ViewData<'a> = &'a Option<String>;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        _: &ModuleContext,
        _: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn view(
        &self,
        config: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if config.is_some() {
            Some((
                icon(Icons::Clipboard).into(),
                None // Action handled in GUI layer
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::event_bus::EventBus;

    #[test]
    fn default_creates_instance() {
        let clipboard = Clipboard::default();
        assert!(matches!(clipboard, Clipboard));
    }

    #[test]
    fn clone_creates_copy() {
        let clipboard = Clipboard::default();
        let cloned = clipboard.clone();
        assert!(matches!(cloned, Clipboard));
    }

    #[test]
    fn register_succeeds() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let bus = EventBus::new(NonZeroUsize::new(4).expect("capacity"));
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone());
        let mut clipboard = Clipboard::default();

        let result = <Clipboard as Module<()>>::register(&mut clipboard, &ctx, ());
        assert!(result.is_ok());
    }

    #[test]
    fn view_returns_some_when_config_present() {
        let clipboard = Clipboard::default();
        let config = Some("cliphist".to_string());

        let result = <Clipboard as Module<()>>::view(&clipboard, &config);
        assert!(result.is_some());

        if let Some((_, action)) = result {
            assert!(action.is_none());
        }
    }

    #[test]
    fn view_returns_none_when_config_absent() {
        let clipboard = Clipboard::default();
        let config = None;

        let result = <Clipboard as Module<()>>::view(&clipboard, &config);
        assert!(result.is_none());
    }
}
