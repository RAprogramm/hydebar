mod data;
mod runtime;
mod view;

pub use data::{NetworkData, SystemInfoData, SystemInfoSampler};
use hydebar_proto::config::SystemModuleConfig;
use iced::Element;
pub use runtime::REFRESH_INTERVAL;
pub use view::{build_indicator_view, build_menu_view, indicator_elements};

use super::{Module, ModuleError, OnModulePress};
use crate::{ModuleContext, event_bus::ModuleEvent};

/// Messages published by the system information module.
#[derive(Debug, Clone)]
pub enum Message {
    Update
}

/// Module responsible for sampling and presenting local system metrics.
pub struct SystemInfo {
    sampler: SystemInfoSampler,
    data:    SystemInfoData,
    polling: runtime::PollingTask
}

impl Default for SystemInfo {
    fn default() -> Self {
        let mut sampler = SystemInfoSampler::new();
        let data = sampler.sample();

        Self {
            sampler,
            data,
            polling: runtime::PollingTask::new()
        }
    }
}

impl SystemInfo {
    /// React to module messages by updating cached metrics when necessary.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.data = self.sampler.sample();
            }
        }
    }

    /// Render the menu entry exposing detailed system information.
    pub fn menu_view(&self) -> Element<'_, Message> {
        view::build_menu_view(&self.data)
    }
}

impl<M> Module<M> for SystemInfo
where
    M: 'static + Clone + From<Message>
{
    type ViewData<'a> = &'a SystemModuleConfig;
    type RegistrationData<'a> = ();

    fn register(
        &mut self,
        ctx: &ModuleContext,
        _: Self::RegistrationData<'_>
    ) -> Result<(), ModuleError> {
        let sender = ctx.module_sender(ModuleEvent::SystemInfo);
        self.polling.spawn(ctx, sender);

        Ok(())
    }

    fn view(
        &self,
        config: Self::ViewData<'_>
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)> {
        view::build_indicator_view(&self.data, config)
    }
}
