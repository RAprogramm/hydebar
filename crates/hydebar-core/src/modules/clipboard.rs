use crate::{
    ModuleContext,
    components::icons::{Icons, icon},
};
use iced::Element;

use super::{Module, ModuleError, OnModulePress};

#[derive(Default, Debug, Clone)]
pub struct Clipboard;

impl Module for Clipboard {
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
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        if config.is_some() {
            Some((
                icon(Icons::Clipboard).into(),
                Some(OnModulePress::Action(Box::new(app::Message::OpenClipboard))),
            ))
        } else {
            None
        }
    }
}
