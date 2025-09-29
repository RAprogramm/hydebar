use crate::config::{AppearanceColor, AppearanceStyle};
use iced::{
    Background, Border, Color, Theme,
    theme::palette,
    widget::button::{self, Status},
};

/// Builds the module button style closure based on the appearance configuration.
#[must_use]
pub fn module_button_style(
    style: AppearanceStyle,
    opacity: f32,
    transparent: bool,
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: match style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => None,
                AppearanceStyle::Islands => {
                    if transparent {
                        None
                    } else {
                        Some(theme.palette().background.scale_alpha(opacity).into())
                    }
                }
            },
            border: Border {
                width: 0.0,
                radius: 12.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

/// Builds a ghost button style closure that fades in on hover.
#[must_use]
pub fn ghost_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width: 0.0,
                radius: 4.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

/// Builds an outline button style closure that highlights borders on hover.
#[must_use]
pub fn outline_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width: 2.0,
                radius: 32.0.into(),
                color: theme.extended_palette().background.weak.color,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

/// Builds the confirm button style closure with filled background.
#[must_use]
pub fn confirm_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity)
                    .into(),
            ),
            border: Border {
                width: 2.0,
                radius: 32.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .strong
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

/// Builds the rounded settings button style closure.
#[must_use]
pub fn settings_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity)
                    .into(),
            ),
            border: Border {
                width: 0.0,
                radius: 32.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .strong
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

/// Builds the workspace button style closure, handling optional colours.
#[must_use]
pub fn workspace_button_style(
    is_empty: bool,
    colors: Option<Option<AppearanceColor>>,
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let (bg_color, fg_color) = colors
            .map(|c| {
                c.map_or(
                    (
                        theme.extended_palette().primary.base.color,
                        theme.extended_palette().primary.base.text,
                    ),
                    |c| {
                        let color = palette::Primary::generate(
                            c.get_base(),
                            theme.palette().background,
                            c.get_text().unwrap_or(theme.palette().text),
                        );
                        (color.base.color, color.base.text)
                    },
                )
            })
            .unwrap_or((
                theme.extended_palette().background.weak.color,
                theme.palette().text,
            ));
        let mut base = button::Style {
            background: Some(Background::Color(if is_empty {
                theme.extended_palette().background.weak.color
            } else {
                bg_color
            })),
            border: Border {
                width: if is_empty { 1.0 } else { 0.0 },
                color: bg_color,
                radius: 16.0.into(),
            },
            text_color: if is_empty {
                theme.extended_palette().background.weak.text
            } else {
                fg_color
            },
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                let (bg_color, fg_color) = colors
                    .map(|c| {
                        c.map_or(
                            (
                                theme.extended_palette().primary.strong.color,
                                theme.extended_palette().primary.strong.text,
                            ),
                            |c| {
                                let color = palette::Primary::generate(
                                    c.get_base(),
                                    theme.palette().background,
                                    c.get_text().unwrap_or(theme.palette().text),
                                );
                                (color.strong.color, color.strong.text)
                            },
                        )
                    })
                    .unwrap_or((
                        theme.extended_palette().background.strong.color,
                        theme.palette().text,
                    ));

                base.background = Some(Background::Color(if is_empty {
                    theme.extended_palette().background.strong.color
                } else {
                    bg_color
                }));
                base.text_color = if is_empty {
                    theme.extended_palette().background.weak.text
                } else {
                    fg_color
                };
                base
            }
            _ => base,
        }
    }
}

/// Builds the quick settings button style closure with active feedback.
#[must_use]
pub fn quick_settings_button_style(
    is_active: bool,
    opacity: f32,
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let mut base = button::Style {
            background: Some(
                if is_active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.color
                }
                .scale_alpha(opacity)
                .into(),
            ),
            border: Border {
                width: 0.0,
                radius: 32.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: if is_active {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            },
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                let peach = theme.extended_palette().primary.weak.color;
                base.background = Some(
                    if is_active {
                        peach
                    } else {
                        theme.extended_palette().background.strong.color
                    }
                    .scale_alpha(opacity)
                    .into(),
                );
                base
            }
            _ => base,
        }
    }
}

/// Builds the submenu button style closure used inside quick settings menus.
#[must_use]
pub fn quick_settings_submenu_button_style(
    is_active: bool,
    opacity: f32,
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width: 0.0,
                radius: 16.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: if is_active {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            },
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base.text_color = theme.palette().text;
                base
            }
            _ => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{Background, Theme};

    fn color(background: Option<Background>) -> Color {
        match background.expect("background should be set") {
            Background::Color(color) => color,
            other => panic!("unexpected background: {other:?}"),
        }
    }

    #[test]
    fn module_button_style_respects_transparency() {
        let theme = Theme::default();
        let style_fn = module_button_style(AppearanceStyle::Islands, 0.5, true);

        let active = style_fn(&theme, Status::Active);
        assert!(active.background.is_none());

        let hover_fn = module_button_style(AppearanceStyle::Islands, 0.5, false);
        let hovered = hover_fn(&theme, Status::Hovered);
        assert_eq!(
            color(hovered.background),
            theme
                .extended_palette()
                .background
                .weak
                .color
                .scale_alpha(0.5)
        );
    }

    #[test]
    fn ghost_button_style_sets_hover_background() {
        let theme = Theme::default();
        let style_fn = ghost_button_style(0.4);

        let hovered = style_fn(&theme, Status::Hovered);
        assert_eq!(
            color(hovered.background),
            theme
                .extended_palette()
                .background
                .weak
                .color
                .scale_alpha(0.4)
        );
    }

    #[test]
    fn outline_button_style_has_border() {
        let theme = Theme::default();
        let style_fn = outline_button_style(0.2);
        let active = style_fn(&theme, Status::Active);

        assert_eq!(active.border.width, 2.0);
        assert_eq!(active.border.radius, 32.0.into());
        assert_eq!(
            active.border.color,
            theme.extended_palette().background.weak.color
        );
    }

    #[test]
    fn confirm_button_style_hoveres_to_strong_background() {
        let theme = Theme::default();
        let style_fn = confirm_button_style(0.8);

        let hovered = style_fn(&theme, Status::Hovered);
        assert_eq!(
            color(hovered.background),
            theme
                .extended_palette()
                .background
                .strong
                .color
                .scale_alpha(0.8)
        );
    }

    #[test]
    fn settings_button_style_hoveres_to_strong_background() {
        let theme = Theme::default();
        let style_fn = settings_button_style(0.6);

        let hovered = style_fn(&theme, Status::Hovered);
        assert_eq!(
            color(hovered.background),
            theme
                .extended_palette()
                .background
                .strong
                .color
                .scale_alpha(0.6)
        );
    }

    #[test]
    fn workspace_button_style_handles_empty_state() {
        let theme = Theme::default();
        let style_fn = workspace_button_style(true, None);

        let active = style_fn(&theme, Status::Active);
        assert_eq!(
            color(active.background),
            theme.extended_palette().background.weak.color
        );
        assert_eq!(active.border.width, 1.0);
    }

    #[test]
    fn workspace_button_style_uses_custom_colors() {
        let theme = Theme::default();
        let color = AppearanceColor::Complete {
            base: hex_color::HexColor::rgb(200, 100, 50),
            strong: Some(hex_color::HexColor::rgb(210, 110, 60)),
            weak: Some(hex_color::HexColor::rgb(190, 90, 40)),
            text: Some(hex_color::HexColor::rgb(10, 20, 30)),
        };
        let style_fn = workspace_button_style(false, Some(Some(color)));

        let hovered = style_fn(&theme, Status::Hovered);
        assert_eq!(hovered.border.radius, 16.0.into());
        assert_eq!(color(hovered.background).a, 1.0);
        assert_eq!(hovered.text_color, Color::from_rgb8(10, 20, 30));
    }

    #[test]
    fn quick_settings_button_style_switches_palette() {
        let theme = Theme::default();
        let inactive = quick_settings_button_style(false, 0.5);
        let active = quick_settings_button_style(true, 0.5);

        let inactive_hover = inactive(&theme, Status::Hovered);
        let active_hover = active(&theme, Status::Hovered);

        assert_eq!(
            color(inactive_hover.background),
            theme
                .extended_palette()
                .background
                .strong
                .color
                .scale_alpha(0.5)
        );
        assert_eq!(
            color(active_hover.background),
            theme.extended_palette().primary.weak.color.scale_alpha(0.5)
        );
    }

    #[test]
    fn quick_settings_submenu_button_style_hover_changes_text_color() {
        let theme = Theme::default();
        let style_fn = quick_settings_submenu_button_style(false, 0.4);

        let hovered = style_fn(&theme, Status::Hovered);
        assert_eq!(hovered.text_color, theme.palette().text);
        assert_eq!(
            color(hovered.background),
            theme
                .extended_palette()
                .background
                .weak
                .color
                .scale_alpha(0.4)
        );
    }
}
