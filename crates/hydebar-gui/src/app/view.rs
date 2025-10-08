use std::f32::consts::PI;

use hydebar_core::{
    HEIGHT,
    menu::{MenuSize, MenuType, menu_wrapper},
    modules::settings::SettingsViewExt,
    outputs::HasOutput,
    style::{backdrop_color, darken_color, hydebar_theme},
};
use hydebar_proto::config::{AppearanceStyle, Position};
use iced::{
    Alignment, Color, Element, Gradient, Length, Radians, Theme,
    daemon::Appearance,
    gradient::Linear,
    widget::{Row, container},
    window::Id,
};

use super::state::{App, Message};
use crate::centerbox;

impl App
{
    pub fn title(&self, _id: Id,) -> String
    {
        String::from("hydebar",)
    }

    pub fn theme(&self, _id: Id,) -> Theme
    {
        hydebar_theme(&self.config.appearance,)
    }

    pub fn style(&self, theme: &Theme,) -> Appearance
    {
        Appearance {
            background_color: Color::TRANSPARENT,
            text_color:       theme.palette().text,
            icon_color:       theme.palette().text,
        }
    }

    pub fn scale_factor(&self, _id: Id,) -> f64
    {
        self.config.appearance.scale_factor
    }

    pub fn view(&self, id: Id,) -> Element<Message,>
    {
        match self.outputs.has(id,) {
            Some(HasOutput::Main,) => {
                let left = self.modules_section(
                    &self.config.modules.left,
                    id,
                    self.config.appearance.opacity,
                );
                let center = self.modules_section(
                    &self.config.modules.center,
                    id,
                    self.config.appearance.opacity,
                );
                let right = self.modules_section(
                    &self.config.modules.right,
                    id,
                    self.config.appearance.opacity,
                );

                let centerbox = centerbox::Centerbox::new([left, center, right,],)
                    .spacing(4,)
                    .width(Length::Fill,)
                    .align_items(Alignment::Center,)
                    .height(if self.config.appearance.style == AppearanceStyle::Islands {
                        HEIGHT
                    } else {
                        HEIGHT - 8.
                    } as f32,)
                    .padding(if self.config.appearance.style == AppearanceStyle::Islands {
                        [4, 4,]
                    } else {
                        [0, 0,]
                    },);

                container(centerbox)
                    .style(|t| container::Style {
                        background: match self.config.appearance.style {
                            AppearanceStyle::Gradient => Some({
                                let start_color = t
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity);

                                let start_color = if self.outputs.menu_is_open() {
                                    darken_color(start_color, self.config.appearance.menu.backdrop)
                                } else {
                                    start_color
                                };

                                let end_color = if self.outputs.menu_is_open() {
                                    backdrop_color(self.config.appearance.menu.backdrop)
                                } else {
                                    Color::TRANSPARENT
                                };

                                Gradient::Linear(
                                    Linear::new(Radians(PI))
                                        .add_stop(
                                            0.0,
                                            match self.config.position {
                                                Position::Top => start_color,
                                                Position::Bottom => end_color,
                                            },
                                        )
                                        .add_stop(
                                            1.0,
                                            match self.config.position {
                                                Position::Top => end_color,
                                                Position::Bottom => start_color,
                                            },
                                        ),
                                )
                                .into()
                            }),
                            AppearanceStyle::Solid => Some({
                                let bg = t
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity);
                                if self.outputs.menu_is_open() {
                                    darken_color(bg, self.config.appearance.menu.backdrop)
                                } else {
                                    bg
                                }
                                .into()
                            }),
                            AppearanceStyle::Islands => {
                                if self.outputs.menu_is_open() {
                                    Some(
                                        backdrop_color(self.config.appearance.menu.backdrop).into(),
                                    )
                                } else {
                                    None
                                }
                            }
                        },
                        ..Default::default()
                    })
                    .into()
            }
            Some(HasOutput::Menu(menu_info,),) => {
                let animated_opacity = self.outputs.get_menu_opacity(id,);
                match menu_info {
                    Some((MenuType::Updates, button_ui_ref,),) => menu_wrapper(
                        id,
                        self.updates.menu_view(id, animated_opacity,).map(Message::Updates,),
                        MenuSize::Small,
                        *button_ui_ref,
                        self.config.position,
                        self.config.appearance.style,
                        animated_opacity,
                        self.config.appearance.menu.backdrop,
                        Message::None,
                        Message::CloseMenu(id,),
                    ),
                    Some((MenuType::Tray(name,), button_ui_ref,),) => menu_wrapper(
                        id,
                        self.tray.menu_view(name, animated_opacity,).map(Message::Tray,),
                        MenuSize::Small,
                        *button_ui_ref,
                        self.config.position,
                        self.config.appearance.style,
                        animated_opacity,
                        self.config.appearance.menu.backdrop,
                        Message::None,
                        Message::CloseMenu(id,),
                    ),
                    Some((MenuType::Settings, button_ui_ref,),) => menu_wrapper(
                        id,
                        self.settings
                            .menu_view(
                                id,
                                &self.config.settings,
                                animated_opacity,
                                self.config.position,
                            )
                            .map(Message::Settings,),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.config.position,
                        self.config.appearance.style,
                        animated_opacity,
                        self.config.appearance.menu.backdrop,
                        Message::None,
                        Message::CloseMenu(id,),
                    ),
                    Some((MenuType::MediaPlayer, button_ui_ref,),) => menu_wrapper(
                        id,
                        self.media_player
                            .menu_view(&self.config.media_player, animated_opacity,)
                            .map(Message::MediaPlayer,),
                        MenuSize::Large,
                        *button_ui_ref,
                        self.config.position,
                        self.config.appearance.style,
                        animated_opacity,
                        self.config.appearance.menu.backdrop,
                        Message::None,
                        Message::CloseMenu(id,),
                    ),
                    Some((MenuType::SystemInfo, button_ui_ref,),) => menu_wrapper(
                        id,
                        self.system_info.menu_view().map(Message::SystemInfo,),
                        MenuSize::Medium,
                        *button_ui_ref,
                        self.config.position,
                        self.config.appearance.style,
                        animated_opacity,
                        self.config.appearance.menu.backdrop,
                        Message::None,
                        Message::CloseMenu(id,),
                    ),
                    None => Row::new().into(),
                }
            }
            None => Row::new().into(),
        }
    }
}
