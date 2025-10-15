use std::borrow::Cow;

use iced::{
    Alignment, Element, Length, Padding,
    alignment::Horizontal,
    widget::{Column, button, column, container, horizontal_rule, row, scrollable, text},
    window::Id
};

use super::state::{CheckState, Message, Updates};
use crate::{
    components::icons::{Icons, icon as icon_component},
    style::ghost_button_style
};

pub(super) fn menu_view(updates: &Updates, id: Id, opacity: f32) -> Element<'_, Message> {
    column!(
        if updates.updates().is_empty() {
            container(text("Up to date ;)")).padding([8, 8]).into()
        } else {
            build_updates_list(updates, opacity)
        },
        horizontal_rule(1),
        action_button("Update", Message::Update(id), opacity),
        check_now_button(updates, opacity),
    )
    .spacing(4)
    .into()
}

pub(super) fn icon(state: &CheckState, update_count: usize) -> Element<'static, Message> {
    let icon = match state {
        CheckState::Checking => Icons::Refresh,
        CheckState::Ready if update_count == 0 => Icons::NoUpdatesAvailable,
        _ => Icons::UpdatesAvailable
    };

    let mut content = row!(container(icon_component(icon)))
        .align_y(Alignment::Center)
        .spacing(4);

    if update_count > 0 {
        content = content.push(text(update_count));
    }

    content.into()
}

fn build_updates_list(updates: &Updates, opacity: f32) -> Element<'_, Message> {
    let mut elements = column!(
        button(row!(
            text(format!("{} Updates available", updates.updates().len())).width(Length::Fill),
            icon_component(if updates.is_updates_list_open() {
                Icons::MenuClosed
            } else {
                Icons::MenuOpen
            })
        ))
        .style(ghost_button_style(opacity))
        .padding([8, 8])
        .on_press(Message::ToggleUpdatesList)
        .width(Length::Fill),
    );

    if updates.is_updates_list_open() {
        elements = elements.push(
            container(scrollable(
                Column::with_children(
                    updates
                        .updates()
                        .iter()
                        .map(|update| build_update_entry(update))
                        .collect::<Vec<Element<'_, Message>>>()
                )
                .padding(Padding::ZERO.right(16))
                .spacing(4)
            ))
            .padding([8, 0])
            .max_height(300)
        );
    }

    elements.into()
}

fn build_update_entry(update: &super::state::Update) -> Element<'_, Message> {
    column!(
        text(update.package.as_str()).size(10).width(Length::Fill),
        text(format!(
            "{} -> {}",
            truncated(&update.from, 18),
            truncated(&update.to, 18)
        ))
        .width(Length::Fill)
        .align_x(Horizontal::Right)
        .size(10),
    )
    .into()
}

fn action_button<'a>(
    label: &'a str,
    message: Message,
    opacity: f32
) -> iced::widget::Button<'a, Message> {
    button(label)
        .style(ghost_button_style(opacity))
        .padding([8, 8])
        .on_press(message)
        .width(Length::Fill)
}

fn check_now_button(updates: &Updates, opacity: f32) -> iced::widget::Button<'static, Message> {
    let mut content = row!(text("Check now").width(Length::Fill));

    if matches!(updates.state(), CheckState::Checking) {
        content = content.push(icon_component(Icons::Refresh));
    }

    button(content)
        .style(ghost_button_style(opacity))
        .padding([8, 8])
        .on_press(Message::CheckNow)
        .width(Length::Fill)
}

fn truncated(value: &str, max: usize) -> Cow<'_, str> {
    if value.chars().count() <= max {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(value.chars().take(max).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_returns_borrowed_when_short_enough() {
        let value = "short";

        assert!(matches!(truncated(value, 10), Cow::Borrowed("short")));
    }

    #[test]
    fn truncated_returns_owned_when_too_long() {
        let value = "averylongstring";

        let truncated = truncated(value, 5);

        assert!(matches!(truncated, Cow::Owned(ref owned) if owned == "avery"));
    }
}
