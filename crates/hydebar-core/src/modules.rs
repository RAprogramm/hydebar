use std::borrow::Cow;

use crate::{
    app::{self, App, Message},
    config::{AppearanceStyle, ModuleDef, ModuleName},
    event_bus::EventBusError,
    menu::MenuType,
    module_context::ModuleContext,
    position_button::position_button,
    style::module_button_style,
};
use iced::{
    Alignment, Border, Color, Element, Length, Subscription,
    widget::{Row, container, row},
    window::Id,
};

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

use log::error;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum OnModulePress {
    Action(Box<Message>),
    ToggleMenu(MenuType),
}

/// Errors that can occur while registering a module.
#[derive(Debug, Error)]
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
        ctx: &ModuleContext,
        data: Self::RegistrationData<'_>,
    ) -> Result<(), ModuleError> {
        let _ = (ctx, data);
        Ok(())
    }

    fn view(
        &self,
        data: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)>;

    fn subscription(&self) -> Option<Subscription<app::Message>> {
        None
    }
}

impl App {
    pub fn modules_section(
        &self,
        modules_def: &[ModuleDef],
        id: Id,
        opacity: f32,
    ) -> Element<Message> {
        let mut row = row!()
            .height(Length::Shrink)
            .align_y(Alignment::Center)
            .spacing(4);

        for module_def in modules_def {
            row = row.push_maybe(match module_def {
                // life parsing of string to module
                ModuleDef::Single(module) => self.single_module_wrapper(module, id, opacity),
                ModuleDef::Group(group) => self.group_module_wrapper(group, id, opacity),
            });
        }

        row.into()
    }

    pub fn modules_subscriptions(&self, modules_def: &[ModuleDef]) -> Vec<Subscription<Message>> {
        let mut subscriptions = Vec::new();

        for module_def in modules_def {
            match module_def {
                ModuleDef::Single(module) => {
                    if let Some(subscription) = self.get_module_subscription(module) {
                        subscriptions.push(subscription);
                    }
                }
                ModuleDef::Group(group) => {
                    for module in group {
                        if let Some(subscription) = self.get_module_subscription(module) {
                            subscriptions.push(subscription);
                        }
                    }
                }
            }
        }

        subscriptions
    }

    fn single_module_wrapper(
        &self,
        module_name: &ModuleName,
        id: Id,
        opacity: f32,
    ) -> Option<Element<Message>> {
        let module = self.get_module_view(module_name, id, opacity);

        module.map(|(content, action)| match action {
            Some(action) => {
                let button = position_button(
                    container(content)
                        .align_y(Alignment::Center)
                        .height(Length::Fill),
                )
                .padding([2, 8])
                .height(Length::Fill)
                .style(module_button_style(
                    self.config.appearance.style,
                    self.config.appearance.opacity,
                    false,
                ));

                match action {
                    OnModulePress::Action(action) => button.on_press(*action),
                    OnModulePress::ToggleMenu(menu_type) => {
                        button.on_press_with_position(move |button_ui_ref| {
                            Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                        })
                    }
                }
                .into()
            }
            _ => {
                let container = container(content)
                    .padding([2, 8])
                    .height(Length::Fill)
                    .align_y(Alignment::Center);

                match self.config.appearance.style {
                    AppearanceStyle::Solid | AppearanceStyle::Gradient => container.into(),
                    AppearanceStyle::Islands => container
                        .style(|theme| container::Style {
                            background: Some(
                                theme
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity)
                                    .into(),
                            ),
                            border: Border {
                                width: 0.0,
                                radius: 12.0.into(),
                                color: Color::TRANSPARENT,
                            },
                            ..container::Style::default()
                        })
                        .into(),
                }
            }
        })
    }

    fn group_module_wrapper(
        &self,
        group: &[ModuleName],
        id: Id,
        opacity: f32,
    ) -> Option<Element<Message>> {
        let modules = group
            .iter()
            .filter_map(|module| self.get_module_view(module, id, opacity))
            .collect::<Vec<_>>();

        if modules.is_empty() {
            None
        } else {
            Some({
                let group = Row::with_children(
                    modules
                        .into_iter()
                        .map(|(content, action)| match action {
                            Some(action) => {
                                let button = position_button(
                                    container(content)
                                        .align_y(Alignment::Center)
                                        .height(Length::Fill),
                                )
                                .padding([2, 8])
                                .height(Length::Fill)
                                .style(module_button_style(
                                    self.config.appearance.style,
                                    self.config.appearance.opacity,
                                    true,
                                ));

                                match action {
                                    OnModulePress::Action(action) => button.on_press(*action),
                                    OnModulePress::ToggleMenu(menu_type) => button
                                        .on_press_with_position(move |button_ui_ref| {
                                            Message::ToggleMenu(
                                                menu_type.clone(),
                                                id,
                                                button_ui_ref,
                                            )
                                        }),
                                }
                                .into()
                            }
                            _ => container(content)
                                .padding([2, 8])
                                .height(Length::Fill)
                                .align_y(Alignment::Center)
                                .into(),
                        })
                        .collect::<Vec<_>>(),
                );

                match self.config.appearance.style {
                    AppearanceStyle::Solid | AppearanceStyle::Gradient => group.into(),
                    AppearanceStyle::Islands => container(group)
                        .style(|theme| container::Style {
                            background: Some(
                                theme
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity)
                                    .into(),
                            ),
                            border: Border {
                                width: 0.0,
                                radius: 12.0.into(),
                                color: Color::TRANSPARENT,
                            },
                            ..container::Style::default()
                        })
                        .into(),
                }
            })
        }
    }

    fn get_module_view(
        &self,
        module_name: &ModuleName,
        id: Id,
        opacity: f32,
    ) -> Option<(Element<Message>, Option<OnModulePress>)> {
        match module_name {
            ModuleName::AppLauncher => self.app_launcher.view(&self.config.app_launcher_cmd),
            ModuleName::Custom(name) => self
                .config
                .custom_modules
                .iter()
                .find(|m| &m.name == name)
                .and_then(|mc| self.custom.get(name).map(|cm| cm.view(mc)))
                .unwrap_or_else(|| {
                    error!("Custom module `{name}` not found");
                    None
                }),
            ModuleName::Updates => self.updates.view(&self.config.updates),
            ModuleName::Clipboard => self.clipboard.view(&self.config.clipboard_cmd),
            ModuleName::Workspaces => self.workspaces.view((
                &self.outputs,
                id,
                &self.config.workspaces,
                &self.config.appearance.workspace_colors,
                self.config.appearance.special_workspace_colors.as_deref(),
            )),
            ModuleName::WindowTitle => self.window_title.view(()),
            ModuleName::SystemInfo => self.system_info.view(&self.config.system),
            ModuleName::KeyboardLayout => self.keyboard_layout.view(&self.config.keyboard_layout),
            ModuleName::KeyboardSubmap => self.keyboard_submap.view(()),
            ModuleName::Tray => self.tray.view((id, opacity)),
            ModuleName::Clock => self.clock.view(&self.config.clock.format),
            ModuleName::Battery => self.battery.view(&self.config.battery),
            ModuleName::Privacy => self.privacy.view(()),
            ModuleName::Settings => self.settings.view(()),
            ModuleName::MediaPlayer => self.media_player.view(&self.config.media_player),
        }
    }

    fn get_module_subscription(&self, module_name: &ModuleName) -> Option<Subscription<Message>> {
        match module_name {
            ModuleName::AppLauncher => self.app_launcher.subscription(),
            ModuleName::Custom(name) => {
                let Some(module) = self.custom.get(name) else {
                    error!("Custom module `{name}` not found");
                    return None;
                };

                if self
                    .config
                    .custom_modules
                    .iter()
                    .any(|definition| &definition.name == name)
                {
                    module.subscription()
                } else {
                    error!("Custom module def `{name}` not found");
                    None
                }
            }
            ModuleName::Updates => self.updates.subscription(),
            ModuleName::Clipboard => self.clipboard.subscription(),
            ModuleName::Workspaces => self.workspaces.subscription(),
            ModuleName::WindowTitle => self.window_title.subscription(),
            ModuleName::SystemInfo => self.system_info.subscription(),
            ModuleName::KeyboardLayout => self.keyboard_layout.subscription(),
            ModuleName::KeyboardSubmap => self.keyboard_submap.subscription(),
            ModuleName::Tray => self.tray.subscription(),
            ModuleName::Clock => self.clock.subscription(),
            ModuleName::Battery => self.battery.subscription(),
            ModuleName::Privacy => self.privacy.subscription(),
            ModuleName::Settings => self.settings.subscription(),
            ModuleName::MediaPlayer => self.media_player.subscription(),
        }
    }
}
