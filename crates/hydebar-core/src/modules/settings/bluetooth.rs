use iced::{
    Element, Length, Theme,
    widget::{Column, Row, button, column, container, horizontal_rule, row, text},
    window::Id
};

use super::{Message, SubMenu, quick_setting_button};
use crate::{
    components::icons::{Icons, icon},
    services::{
        ServiceEvent,
        bluetooth::{BluetoothData, BluetoothService, BluetoothState}
    },
    style::ghost_button_style
};

#[derive(Debug, Clone)]
pub enum BluetoothMessage {
    Event(ServiceEvent<BluetoothService>),
    Toggle,
    ConnectDevice(zbus::zvariant::OwnedObjectPath),
    DisconnectDevice(zbus::zvariant::OwnedObjectPath),
    More(Id)
}

impl BluetoothData {
    pub fn get_quick_setting_button(
        &self,
        id: Id,
        sub_menu: Option<SubMenu>,
        show_more_button: bool,
        opacity: f32
    ) -> Option<(Element<'_, Message>, Option<Element<'_, Message>>)> {
        Some((
            quick_setting_button(
                Icons::Bluetooth,
                "Bluetooth".to_owned(),
                None,
                self.state == BluetoothState::Active,
                Message::Bluetooth(BluetoothMessage::Toggle),
                Some((
                    SubMenu::Bluetooth,
                    sub_menu,
                    Message::ToggleSubMenu(SubMenu::Bluetooth)
                ))
                .filter(|_| self.state == BluetoothState::Active),
                opacity
            ),
            sub_menu
                .filter(|menu_type| *menu_type == SubMenu::Bluetooth)
                .map(|_| self.bluetooth_menu(id, show_more_button, opacity))
        ))
    }

    pub fn bluetooth_menu(
        &self,
        id: Id,
        show_more_button: bool,
        opacity: f32
    ) -> Element<'_, Message> {
        let main = if self.devices.is_empty() {
            text("No paired devices").into()
        } else {
            Column::with_children(
                self.devices
                    .iter()
                    .map(|d| {
                        Row::new()
                            .push(text(d.name.to_string()).width(Length::Fill))
                            .push_maybe(d.battery.map(Self::battery_level))
                            .push(
                                button(text(if d.connected { "Disconnect" } else { "Connect" }))
                                    .padding([4, 12])
                                    .style(ghost_button_style(opacity))
                                    .on_press(Message::Bluetooth(if d.connected {
                                        BluetoothMessage::DisconnectDevice(d.path.clone())
                                    } else {
                                        BluetoothMessage::ConnectDevice(d.path.clone())
                                    }))
                            )
                            .spacing(8)
                            .align_y(iced::Alignment::Center)
                            .into()
                    })
                    .collect::<Vec<Element<'_, Message>>>()
            )
            .spacing(8)
            .into()
        };

        if show_more_button {
            column!(
                main,
                horizontal_rule(1),
                button("More")
                    .on_press(Message::Bluetooth(BluetoothMessage::More(id)))
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(ghost_button_style(opacity))
            )
            .spacing(12)
            .into()
        } else {
            main
        }
    }

    fn battery_level<'a>(battery: u8) -> Element<'a, Message> {
        container(
            row!(
                icon(match battery {
                    0..=20 => Icons::Battery0,
                    21..=40 => Icons::Battery1,
                    41..=60 => Icons::Battery2,
                    61..=80 => Icons::Battery3,
                    _ => Icons::Battery4
                }),
                text(format!("{battery}%"))
            )
            .spacing(8)
            .width(Length::Shrink)
        )
        .style(move |theme: &Theme| container::Style {
            text_color: Some(if battery <= 20 {
                theme.palette().danger
            } else {
                theme.palette().text
            }),
            ..container::Style::default()
        })
        .into()
    }
}
