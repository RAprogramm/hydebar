use iced::Element;

use super::{Module, ModuleError, OnModulePress};
use crate::{
    ModuleContext,
    components::icons::{Icons, icon},
};

#[derive(Default, Debug, Clone,)]
pub struct AppLauncher;

impl<M,> Module<M,> for AppLauncher
where
    M: 'static + Clone,
{
    type ViewData<'a,> = &'a Option<String,>;
    type RegistrationData<'a,> = ();

    fn register(
        &mut self,
        _: &ModuleContext,
        _: Self::RegistrationData<'_,>,
    ) -> Result<(), ModuleError,>
    {
        Ok((),)
    }

    fn view(
        &self,
        config: Self::ViewData<'_,>,
    ) -> Option<(Element<'static, M,>, Option<OnModulePress<M,>,>,),>
    {
        if config.is_some() {
            Some((
                icon(Icons::AppLauncher,).into(),
                None, // Action handled in GUI layer
            ),)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests
{
    use std::num::NonZeroUsize;

    use super::*;
    use crate::event_bus::EventBus;

    #[test]
    fn default_creates_instance()
    {
        let launcher = AppLauncher::default();
        assert!(matches!(launcher, AppLauncher));
    }

    #[test]
    fn clone_creates_copy()
    {
        let launcher = AppLauncher::default();
        let cloned = launcher.clone();
        assert!(matches!(cloned, AppLauncher));
    }

    #[test]
    fn register_succeeds()
    {
        let runtime = tokio::runtime::Runtime::new().expect("runtime",);
        let bus = EventBus::new(NonZeroUsize::new(4,).expect("capacity",),);
        let ctx = ModuleContext::new(bus.sender(), runtime.handle().clone(),);
        let mut launcher = AppLauncher::default();

        let result = <AppLauncher as Module<(),>>::register(&mut launcher, &ctx, (),);
        assert!(result.is_ok());
    }

    #[test]
    fn view_returns_some_when_config_present()
    {
        let launcher = AppLauncher::default();
        let config = Some("wofi".to_string(),);

        let result = <AppLauncher as Module<(),>>::view(&launcher, &config,);
        assert!(result.is_some());

        if let Some((_, action,),) = result {
            assert!(action.is_none());
        }
    }

    #[test]
    fn view_returns_none_when_config_absent()
    {
        let launcher = AppLauncher::default();
        let config = None;

        let result = <AppLauncher as Module<(),>>::view(&launcher, &config,);
        assert!(result.is_none());
    }
}
