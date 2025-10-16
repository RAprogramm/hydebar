use iced::{
    Alignment, Border, Color, Element, Length, Theme,
    widget::{Column, Row, button, column, container, horizontal_rule, row, text},
};

use super::{CalendarState, Message};
use crate::components::icons::{Icons, icon};

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Renders the calendar menu view with month navigation and day grid.
pub fn build_calendar_menu_view(state: &CalendarState) -> Element<'_, Message> {
    let calendar_data = state.generate_calendar();

    let header = row![
        button(icon(Icons::LeftChevron))
            .on_press(Message::PreviousMonth)
            .style(nav_button_style),
        container(text(format!("{} {}", state.month_name(), state.year())).size(18))
            .center_x(Length::Fill)
            .align_x(Alignment::Center),
        button(icon(Icons::RightChevron))
            .on_press(Message::NextMonth)
            .style(nav_button_style),
    ]
    .align_y(Alignment::Center)
    .spacing(8);

    let weekday_header = Row::with_children(
        WEEKDAYS
            .iter()
            .map(|day| {
                container(text(*day).size(12))
                    .width(Length::Fixed(36.))
                    .center_x(Length::Fill)
                    .into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(4);

    let mut week_rows = Vec::new();
    for week in calendar_data.days.chunks(7) {
        let week_row = Row::with_children(
            week.iter()
                .map(|day_info| {
                    let day_text = text(day_info.day.to_string()).size(14);
                    let in_month = day_info.in_month;
                    let is_today = day_info.is_today;

                    let day_button = button(container(day_text).center(Length::Fill))
                        .width(Length::Fixed(36.))
                        .height(Length::Fixed(36.))
                        .style(move |theme: &Theme, status: button::Status| {
                            day_button_style(theme, status, in_month, is_today)
                        });

                    day_button.into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(4);

        week_rows.push(week_row.into());
    }

    let calendar_grid = Column::with_children(week_rows).spacing(4);

    column![
        header,
        horizontal_rule(1),
        weekday_header,
        calendar_grid
    ]
    .spacing(8)
    .padding(4)
    .into()
}

fn nav_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut base = button::Style {
        background: None,
        border:     Border {
            width:  0.0,
            radius: 4.0.into(),
            color:  Color::TRANSPARENT,
        },
        text_color: theme.palette().text,
        ..button::Style::default()
    };

    match status {
        button::Status::Hovered => {
            base.background = Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .into()
            );
            base
        }
        _ => base,
    }
}

fn day_button_style(
    theme: &Theme,
    status: button::Status,
    in_month: bool,
    is_today: bool,
) -> button::Style {
    let base_color = if in_month {
        theme.extended_palette().background.base.color
    } else {
        theme.extended_palette().background.weak.color
    };

    let text_color = if in_month {
        theme.palette().text
    } else {
        theme.extended_palette().background.weak.text
    };

    let border = if is_today {
        Border {
            color:  theme.palette().primary,
            width:  2.0,
            radius: 4.0.into(),
        }
    } else {
        Border {
            width:  0.0,
            radius: 4.0.into(),
            color:  Color::TRANSPARENT,
        }
    };

    let mut base = button::Style {
        background: Some(base_color.into()),
        border,
        text_color,
        ..button::Style::default()
    };

    match status {
        button::Status::Hovered => {
            base.background = Some(theme.extended_palette().primary.weak.color.into());
            base.text_color = theme.extended_palette().primary.weak.text;
            base
        }
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekdays_count_is_seven() {
        assert_eq!(WEEKDAYS.len(), 7);
    }

    #[test]
    fn weekdays_start_with_monday() {
        assert_eq!(WEEKDAYS[0], "Mon");
        assert_eq!(WEEKDAYS[6], "Sun");
    }
}
