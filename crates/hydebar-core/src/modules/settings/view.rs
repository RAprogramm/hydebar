use super::{
    bluetooth::BluetoothState,
    power::power_menu,
    state::{Message, Settings, SubMenu},
};
use crate::{
    components::icons::{Icons, icon},
    config::{Position, SettingsModuleConfig},
    menu::MenuType,
    modules::OnModulePress,
    password_dialog,
    style::{
        quick_settings_button_style, quick_settings_submenu_button_style, settings_button_style,
    },
};
use iced::{
    Alignment, Background, Border, Element, Length, Padding, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Column, Row, Space, button, column, container, horizontal_space, row, text},
    window::Id,
};

pub(super) trait SettingsViewExt {
    type ViewData<'a>;

    fn settings_view<M>(
        &self,
        data: Self::ViewData<'_>,
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>;

    fn menu_view(
        &self,
        id: Id,
        config: &SettingsModuleConfig,
        opacity: f32,
        position: Position,
    ) -> Element<Message>;
}

impl SettingsViewExt for Settings {
    type ViewData<'a> = ();

    fn settings_view<M>(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<'static, M>, Option<OnModulePress<M>>)>
    where
        M: 'static + From<Message>,
    {
        Some((
            Row::new()
                .push_maybe(
                    self.idle_inhibitor
                        .as_ref()
                        .filter(|i| i.is_inhibited())
                        .map(|_| {
                            container(icon(Icons::EyeOpened)).style(|theme: &Theme| {
                                container::Style {
                                    text_color: Some(theme.palette().danger),
                                    ..Default::default()
                                }
                            })
                        }),
                )
                .push_maybe(
                    self.upower
                        .as_ref()
                        .and_then(|p| p.power_profile.indicator()),
                )
                .push_maybe(self.audio.as_ref().and_then(|a| a.sink_indicator()))
                .push(
                    Row::new()
                        .push_maybe(
                            self.network
                                .as_ref()
                                .and_then(|n| n.get_connection_indicator()),
                        )
                        .push_maybe(self.network.as_ref().and_then(|n| n.get_vpn_indicator()))
                        .spacing(4),
                )
                .push_maybe(
                    self.upower
                        .as_ref()
                        .and_then(|upower| upower.battery)
                        .map(|battery| battery.indicator()),
                )
                .spacing(8)
                .into(),
            Some(OnModulePress::ToggleMenu(MenuType::Settings)),
        ))
    }

    fn menu_view(
        &self,
        id: Id,
        config: &SettingsModuleConfig,
        opacity: f32,
        position: Position,
    ) -> Element<Message> {
        if let Some((ssid, current_password)) = &self.password_dialog {
            password_dialog::view(id, ssid, current_password, opacity).map(Message::PasswordDialog)
        } else {
            let battery_data = self
                .upower
                .as_ref()
                .and_then(|upower| upower.battery)
                .map(|battery| battery.settings_indicator());
            let right_buttons = Row::new()
                .push_maybe(config.lock_cmd.as_ref().map(|_| {
                    button(icon(Icons::Lock))
                        .padding([8, 13])
                        .on_press(Message::Lock)
                        .style(settings_button_style(opacity))
                }))
                .push(
                    button(icon(if self.sub_menu == Some(SubMenu::Power) {
                        Icons::Close
                    } else {
                        Icons::Power
                    }))
                    .padding([8, 13])
                    .on_press(Message::ToggleSubMenu(SubMenu::Power))
                    .style(settings_button_style(opacity)),
                )
                .spacing(8);

            let header = Row::new()
                .push_maybe(battery_data)
                .push(Space::with_width(Length::Fill))
                .push(right_buttons)
                .spacing(8)
                .width(Length::Fill);

            let (sink_slider, source_slider) = self
                .audio
                .as_ref()
                .map(|a| a.audio_sliders(self.sub_menu, opacity))
                .unwrap_or((None, None));

            let wifi_setting_button = self.network.as_ref().and_then(|n| {
                n.get_wifi_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.wifi_more_cmd.is_some(),
                    opacity,
                )
            });
            let quick_settings = quick_settings_section(
                vec![
                    wifi_setting_button,
                    self.bluetooth
                        .as_ref()
                        .filter(|b| b.state != BluetoothState::Unavailable)
                        .and_then(|b| {
                            b.get_quick_setting_button(
                                id,
                                self.sub_menu,
                                config.bluetooth_more_cmd.is_some(),
                                opacity,
                            )
                        }),
                    self.network.as_ref().and_then(|n| {
                        n.get_vpn_quick_setting_button(
                            id,
                            self.sub_menu,
                            config.vpn_more_cmd.is_some(),
                            opacity,
                        )
                    }),
                    self.network.as_ref().and_then(|n| {
                        if config.remove_airplane_btn {
                            None
                        } else {
                            Some(n.get_airplane_mode_quick_setting_button(opacity))
                        }
                    }),
                    self.idle_inhibitor.as_ref().and_then(|i| {
                        if config.remove_idle_btn {
                            None
                        } else {
                            Some((
                                quick_setting_button(
                                    if i.is_inhibited() {
                                        Icons::EyeOpened
                                    } else {
                                        Icons::EyeClosed
                                    },
                                    "Idle Inhibitor".to_string(),
                                    None,
                                    i.is_inhibited(),
                                    Message::ToggleInhibitIdle,
                                    None,
                                    opacity,
                                ),
                                None,
                            ))
                        }
                    }),
                    self.upower
                        .as_ref()
                        .and_then(|u| u.power_profile.get_quick_setting_button(opacity)),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
                opacity,
            );

            let (top_sink_slider, bottom_sink_slider) = match position {
                Position::Top => (sink_slider, None),
                Position::Bottom => (None, sink_slider),
            };
            let (top_source_slider, bottom_source_slider) = match position {
                Position::Top => (source_slider, None),
                Position::Bottom => (None, source_slider),
            };

            Column::new()
                .push(header)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Power)
                        .map(|_| {
                            sub_menu_wrapper(
                                power_menu(opacity, config).map(Message::Power),
                                opacity,
                            )
                        }),
                )
                .push_maybe(top_sink_slider)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sinks)
                        .and_then(|_| {
                            self.audio.as_ref().map(|a| {
                                sub_menu_wrapper(
                                    a.sinks_submenu(
                                        id,
                                        config.audio_sinks_more_cmd.is_some(),
                                        opacity,
                                    ),
                                    opacity,
                                )
                            })
                        }),
                )
                .push_maybe(bottom_sink_slider)
                .push_maybe(top_source_slider)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sources)
                        .and_then(|_| {
                            self.audio.as_ref().map(|a| {
                                sub_menu_wrapper(
                                    a.sources_submenu(
                                        id,
                                        config.audio_sources_more_cmd.is_some(),
                                        opacity,
                                    ),
                                    opacity,
                                )
                            })
                        }),
                )
                .push_maybe(bottom_source_slider)
                .push_maybe(self.brightness.as_ref().map(|b| b.brightness_slider()))
                .push(quick_settings)
                .spacing(16)
                .into()
        }
    }
}

pub(crate) fn quick_settings_section<'a>(
    buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
    opacity: f32,
) -> Element<'a, Message> {
    let mut section = column!().spacing(8);

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons.into_iter() {
        match before.take() {
            Some((before_button, before_menu)) => {
                section = section.push(row![before_button, button].width(Length::Fill).spacing(8));

                if let Some(menu) = before_menu {
                    section = section.push(sub_menu_wrapper(menu, opacity));
                }

                if let Some(menu) = menu {
                    section = section.push(sub_menu_wrapper(menu, opacity));
                }
            }
            _ => {
                before = Some((button, menu));
            }
        }
    }

    if let Some((before_button, before_menu)) = before.take() {
        section = section.push(
            row![before_button, horizontal_space()]
                .width(Length::Fill)
                .spacing(8),
        );

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(menu, opacity));
        }
    }

    section.into()
}

pub(crate) fn sub_menu_wrapper<Msg: 'static>(content: Element<Msg>, opacity: f32) -> Element<Msg> {
    container(content)
        .style(move |theme: &Theme| container::Style {
            background: Background::Color(
                theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color
                    .scale_alpha(opacity),
            )
            .into(),
            border: Border::default().rounded(16),
            ..container::Style::default()
        })
        .padding(16)
        .width(Length::Fill)
        .into()
}

pub fn quick_setting_button<'a, Msg: Clone + 'static>(
    icon_type: Icons,
    title: String,
    subtitle: Option<String>,
    active: bool,
    on_press: Msg,
    with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>,
    opacity: f32,
) -> Element<'a, Msg> {
    let main_content = row!(
        icon(icon_type).size(20),
        Column::new()
            .push(text(title).size(12))
            .push_maybe(subtitle.map(|s| text(s).size(10)))
            .spacing(4)
    )
    .spacing(8)
    .padding(Padding::ZERO.left(4))
    .width(Length::Fill)
    .align_y(Alignment::Center);

    button(
        Row::new()
            .push(main_content)
            .push_maybe(with_submenu.map(|(menu_type, submenu, msg)| {
                button(
                    container(icon(if Some(menu_type) == submenu {
                        Icons::Close
                    } else {
                        Icons::RightChevron
                    }))
                    .align_y(Vertical::Center)
                    .align_x(Horizontal::Center),
                )
                .padding([4, if Some(menu_type) == submenu { 9 } else { 12 }])
                .style(quick_settings_submenu_button_style(active, opacity))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .on_press(msg)
            }))
            .spacing(4)
            .align_y(Alignment::Center)
            .height(Length::Fill),
    )
    .padding([4, 8])
    .on_press(on_press)
    .height(Length::Fill)
    .width(Length::Fill)
    .style(quick_settings_button_style(active, opacity))
    .width(Length::Fill)
    .height(Length::Fixed(50.))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::{button, text};

    #[test]
    fn quick_settings_section_pairs_buttons() {
        let button_a: Element<'_, Message> = button(text("a"))
            .on_press(Message::ToggleInhibitIdle)
            .into();
        let button_b: Element<'_, Message> = button(text("b"))
            .on_press(Message::ToggleInhibitIdle)
            .into();

        let section = quick_settings_section(vec![(button_a, None), (button_b, None)], 1.0);
        let children = section.as_widget().children();
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn quick_settings_section_renders_menu_when_present() {
        let button_a: Element<'_, Message> = button(text("a"))
            .on_press(Message::ToggleInhibitIdle)
            .into();
        let menu: Element<'_, Message> = text("menu").into();

        let section = quick_settings_section(vec![(button_a, Some(menu))], 1.0);
        let children = section.as_widget().children();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn quick_setting_button_can_render_submenu_toggle() {
        let element: Element<'_, Message> = quick_setting_button(
            Icons::Power,
            "Test".into(),
            None,
            true,
            Message::ToggleInhibitIdle,
            Some((
                SubMenu::Wifi,
                Some(SubMenu::Wifi),
                Message::ToggleInhibitIdle,
            )),
            1.0,
        );

        // A button renders a single row child that contains the submenu toggle.
        let children = element.as_widget().children();
        assert_eq!(children.len(), 1);
    }
}
