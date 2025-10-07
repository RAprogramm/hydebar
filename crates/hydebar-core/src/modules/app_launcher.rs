use crate::{
    ModuleContext,
    components::icons::{Icons, icon},
};
use iced::Element;

use super::{Module, ModuleError, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct AppLauncher;

impl<M> Module<M> for AppLauncher {
    type ViewData<'a> = &'a Option<String>;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        _: &ModuleContext,
        _: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn view(
        &self,
        config: Self::ViewData<'_>,
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        if config.is_some() {
            Some((
                icon(Icons::AppLauncher).into(),
                None, // Action handled in GUI layer
            ))
        } else {
            None
        }
    }
}
