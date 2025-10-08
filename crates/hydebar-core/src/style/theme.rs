use iced::{
    Border, Color, Theme,
    theme::{Palette, palette},
    widget::text_input::{self},
};

use crate::config::{Appearance, AppearanceColor};

/// Builds the HyDEbar [`Theme`] from the configured [`Appearance`].
///
/// # Parameters
/// - `appearance`: The appearance configuration provided by the user.
///
/// # Returns
/// A [`Theme`] with palette colours derived from the appearance configuration.
#[must_use]
pub fn hydebar_theme(appearance: &Appearance,) -> Theme
{
    Theme::custom_with_fn(
        "local".to_string(),
        Palette {
            background: appearance.background_color.get_base(),
            text:       appearance.text_color.get_base(),
            primary:    appearance.primary_color.get_base(),
            success:    appearance.success_color.get_base(),
            danger:     appearance.danger_color.get_base(),
        },
        |palette| build_extended_palette(appearance, palette,),
    )
}

fn build_extended_palette(appearance: &Appearance, palette: Palette,) -> palette::Extended
{
    let default_bg = palette::Background::new(
        palette.background,
        appearance.background_color.get_text().unwrap_or(palette.text,),
    );
    let default_primary = palette::Primary::generate(
        palette.primary,
        palette.background,
        appearance.primary_color.get_text().unwrap_or(palette.text,),
    );
    let default_secondary = palette::Primary::generate(
        appearance.secondary_color.get_base(),
        palette.background,
        appearance.secondary_color.get_text().unwrap_or(palette.text,),
    );
    let default_success = palette::Success::generate(
        palette.success,
        palette.background,
        appearance.success_color.get_text().unwrap_or(palette.text,),
    );
    let default_danger = palette::Danger::generate(
        palette.danger,
        palette.background,
        appearance.danger_color.get_text().unwrap_or(palette.text,),
    );

    palette::Extended {
        background: build_pair(
            &appearance.background_color,
            palette.text,
            default_bg.base,
            default_bg.weak,
            default_bg.strong,
        ),
        primary:    build_primary_pair(&appearance.primary_color, palette.text, default_primary,),
        secondary:  build_secondary_pair(
            &appearance.secondary_color,
            palette.text,
            default_secondary,
        ),
        success:    build_success_pair(&appearance.success_color, palette.text, default_success,),
        danger:     build_danger_pair(&appearance.danger_color, palette.text, default_danger,),
        is_dark:    true,
    }
}

fn build_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    base: palette::Pair,
    default_weak: palette::Pair,
    default_strong: palette::Pair,
) -> palette::Background
{
    palette::Background {
        base,
        weak: color.get_weak_pair(text_fallback,).unwrap_or(default_weak,),
        strong: color.get_strong_pair(text_fallback,).unwrap_or(default_strong,),
    }
}

fn build_primary_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    defaults: palette::Primary,
) -> palette::Primary
{
    palette::Primary {
        base:   defaults.base,
        weak:   color.get_weak_pair(text_fallback,).unwrap_or(defaults.weak,),
        strong: color.get_strong_pair(text_fallback,).unwrap_or(defaults.strong,),
    }
}

fn build_secondary_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    defaults: palette::Primary,
) -> palette::Secondary
{
    palette::Secondary {
        base:   defaults.base,
        weak:   color.get_weak_pair(text_fallback,).unwrap_or(defaults.weak,),
        strong: color.get_strong_pair(text_fallback,).unwrap_or(defaults.strong,),
    }
}

fn build_success_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    defaults: palette::Success,
) -> palette::Success
{
    palette::Success {
        base:   defaults.base,
        weak:   color.get_weak_pair(text_fallback,).unwrap_or(defaults.weak,),
        strong: color.get_strong_pair(text_fallback,).unwrap_or(defaults.strong,),
    }
}

fn build_danger_pair(
    color: &AppearanceColor,
    text_fallback: Color,
    defaults: palette::Danger,
) -> palette::Danger
{
    palette::Danger {
        base:   defaults.base,
        weak:   color.get_weak_pair(text_fallback,).unwrap_or(defaults.weak,),
        strong: color.get_strong_pair(text_fallback,).unwrap_or(defaults.strong,),
    }
}

/// Returns a [`Color`] representing the menu backdrop opacity overlay.
#[must_use]
pub fn backdrop_color(backdrop: f32,) -> Color
{
    Color::from_rgba(0.0, 0.0, 0.0, backdrop,)
}

/// Darkens a [`Color`] by applying the provided alpha factor.
#[must_use]
pub fn darken_color(color: Color, darkening_alpha: f32,) -> Color
{
    let new_r = color.r * (1.0 - darkening_alpha);
    let new_g = color.g * (1.0 - darkening_alpha);
    let new_b = color.b * (1.0 - darkening_alpha);
    let new_a = color.a + (1.0 - color.a) * darkening_alpha;

    Color::from([new_r, new_g, new_b, new_a,],)
}

/// Computes the [`text_input::Style`] for the given [`text_input::Status`].
#[must_use]
pub fn text_input_style(theme: &Theme, status: text_input::Status,) -> text_input::Style
{
    let mut base = text_input::Style {
        background:  theme.palette().background.into(),
        border:      Border {
            width:  2.0,
            radius: 32.0.into(),
            color:  theme.extended_palette().background.weak.color,
        },
        icon:        theme.palette().text,
        placeholder: theme.palette().text,
        value:       theme.palette().text,
        selection:   theme.palette().primary,
    };
    match status {
        text_input::Status::Active => base,
        text_input::Status::Focused | text_input::Status::Hovered => {
            base.border.color = theme.extended_palette().background.strong.color;
            base
        }
        text_input::Status::Disabled => {
            base.background = theme.extended_palette().background.weak.color.into();
            base.border.color = Color::TRANSPARENT;
            base
        }
    }
}

#[cfg(test)]
mod tests
{
    use hex_color::HexColor;
    use iced::Color;

    use super::*;
    use crate::config::{Appearance, AppearanceColor, AppearanceStyle};

    #[test]
    fn hydebar_theme_respects_custom_palette()
    {
        let appearance = Appearance {
            background_color: AppearanceColor::Complete {
                base:   HexColor::rgb(10, 20, 30,),
                strong: Some(HexColor::rgb(40, 50, 60,),),
                weak:   Some(HexColor::rgb(70, 80, 90,),),
                text:   Some(HexColor::rgb(200, 210, 220,),),
            },
            primary_color: AppearanceColor::Complete {
                base:   HexColor::rgb(120, 60, 30,),
                strong: Some(HexColor::rgb(160, 90, 45,),),
                weak:   Some(HexColor::rgb(100, 50, 25,),),
                text:   Some(HexColor::rgb(255, 255, 255,),),
            },
            secondary_color: AppearanceColor::Complete {
                base:   HexColor::rgb(15, 25, 35,),
                strong: Some(HexColor::rgb(45, 55, 65,),),
                weak:   Some(HexColor::rgb(75, 85, 95,),),
                text:   None,
            },
            success_color: AppearanceColor::Complete {
                base:   HexColor::rgb(20, 120, 20,),
                strong: Some(HexColor::rgb(30, 140, 30,),),
                weak:   Some(HexColor::rgb(10, 80, 10,),),
                text:   Some(HexColor::rgb(0, 0, 0,),),
            },
            danger_color: AppearanceColor::Complete {
                base:   HexColor::rgb(180, 20, 20,),
                strong: Some(HexColor::rgb(200, 40, 40,),),
                weak:   Some(HexColor::rgb(160, 10, 10,),),
                text:   Some(HexColor::rgb(250, 250, 250,),),
            },
            text_color: AppearanceColor::Simple(HexColor::rgb(250, 250, 250,),),
            style: AppearanceStyle::Islands,
            ..Appearance::default()
        };

        let theme = hydebar_theme(&appearance,);
        let palette = theme.extended_palette();

        assert_eq!(palette.background.base.color, Color::from_rgb8(10, 20, 30));
        assert_eq!(palette.background.weak.color, Color::from_rgb8(70, 80, 90));
        assert_eq!(palette.background.strong.color, Color::from_rgb8(40, 50, 60));
        assert_eq!(palette.primary.base.color, Color::from_rgb8(120, 60, 30));
        assert_eq!(palette.primary.strong.color, Color::from_rgb8(160, 90, 45));
        assert_eq!(palette.primary.base.text, Color::from_rgb8(255, 255, 255));
        assert_eq!(palette.success.weak.color, Color::from_rgb8(10, 80, 10));
        assert_eq!(palette.danger.strong.color, Color::from_rgb8(200, 40, 40));
        assert!(palette.is_dark);
    }

    #[test]
    fn text_input_style_transitions_states()
    {
        let theme = Theme::default();

        let active = text_input_style(&theme, text_input::Status::Active,);
        assert_eq!(active.border.width, 2.0);
        assert_eq!(active.border.radius, 32.0.into());
        assert_eq!(active.border.color, theme.extended_palette().background.weak.color);

        let hovered = text_input_style(&theme, text_input::Status::Hovered,);
        assert_eq!(hovered.border.color, theme.extended_palette().background.strong.color);

        let disabled = text_input_style(&theme, text_input::Status::Disabled,);
        assert_eq!(disabled.background, theme.extended_palette().background.weak.color.into());
        assert_eq!(disabled.border.color, Color::TRANSPARENT);
    }

    #[test]
    fn backdrop_color_applies_alpha_channel()
    {
        let color = backdrop_color(0.42,);
        assert!((color.a - 0.42).abs() < f32::EPSILON);
        assert!(color.r.abs() < f32::EPSILON);
        assert!(color.g.abs() < f32::EPSILON);
        assert!(color.b.abs() < f32::EPSILON);
    }

    #[test]
    fn darken_color_scales_channels()
    {
        let color = Color::from_rgb(0.8, 0.6, 0.4,);
        let darkened = darken_color(color, 0.5,);

        assert!((darkened.r - 0.4).abs() < 0.0001);
        assert!((darkened.g - 0.3).abs() < 0.0001);
        assert!((darkened.b - 0.2).abs() < 0.0001);
        assert!((darkened.a - (color.a + (1.0 - color.a) * 0.5)).abs() < 0.0001);
    }
}
