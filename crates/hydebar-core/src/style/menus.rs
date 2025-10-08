use iced::{Border, Theme, widget::container::Style};

use super::theme::backdrop_color;

/// Builds the menu container style closure used for popup content.
pub fn menu_container_style(opacity: f32,) -> impl Fn(&Theme,) -> Style
{
    move |theme: &Theme| Style {
        background: Some(theme.palette().background.scale_alpha(opacity,).into(),),
        border: Border {
            color:  theme.extended_palette().secondary.base.color.scale_alpha(opacity,),
            width:  1.0,
            radius: 16.0.into(),
        },
        ..Style::default()
    }
}

/// Builds the menu backdrop style closure that applies the configured opacity.
pub fn menu_backdrop_style(backdrop: f32,) -> impl Fn(&Theme,) -> Style
{
    move |_| Style {
        background: Some(backdrop_color(backdrop,).into(),),
        ..Style::default()
    }
}

#[cfg(test)]
mod tests
{
    use iced::{Background, Color};

    use super::*;

    fn color(background: Option<Background,>,) -> Color
    {
        match background.expect("background should be set",) {
            Background::Color(color,) => color,
            other => panic!("unexpected background: {other:?}"),
        }
    }

    #[test]
    fn menu_container_style_scales_opacity()
    {
        let theme = Theme::default();
        let style_fn = menu_container_style(0.3,);
        let style = style_fn(&theme,);

        let background = color(style.background,);
        assert_eq!(background.a, 0.3 * theme.palette().background.a);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius, 16.0.into());
    }

    #[test]
    fn menu_backdrop_style_uses_backdrop_color()
    {
        let theme = Theme::default();
        let style_fn = menu_backdrop_style(0.6,);
        let style = style_fn(&theme,);

        let background = color(style.background,);
        assert!((background.a - 0.6).abs() < f32::EPSILON);
    }
}
