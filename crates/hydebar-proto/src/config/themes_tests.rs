use hex_color::HexColor;

use super::themes::PresetTheme;
use crate::config::{Appearance, AppearanceColor};

#[test]
fn catppuccin_mocha_has_correct_background()
{
    let appearance = PresetTheme::CatppuccinMocha.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(30, 30, 46)));
}

#[test]
fn catppuccin_mocha_has_correct_primary()
{
    let appearance = PresetTheme::CatppuccinMocha.to_appearance();
    assert_eq!(appearance.primary_color, AppearanceColor::Simple(HexColor::rgb(203, 166, 247)));
}

#[test]
fn catppuccin_mocha_has_workspace_colors()
{
    let appearance = PresetTheme::CatppuccinMocha.to_appearance();
    assert_eq!(appearance.workspace_colors.len(), 10);
}

#[test]
fn dracula_has_correct_background()
{
    let appearance = PresetTheme::Dracula.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(40, 42, 54)));
}

#[test]
fn dracula_has_correct_primary()
{
    let appearance = PresetTheme::Dracula.to_appearance();
    assert_eq!(appearance.primary_color, AppearanceColor::Simple(HexColor::rgb(189, 147, 249)));
}

#[test]
fn nord_has_correct_background()
{
    let appearance = PresetTheme::Nord.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(46, 52, 64)));
}

#[test]
fn gruvbox_dark_has_correct_background()
{
    let appearance = PresetTheme::GruvboxDark.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(40, 40, 40)));
}

#[test]
fn gruvbox_light_has_correct_background()
{
    let appearance = PresetTheme::GruvboxLight.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(251, 241, 199)));
}

#[test]
fn tokyo_night_has_correct_background()
{
    let appearance = PresetTheme::TokyoNight.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(26, 27, 38)));
}

#[test]
fn all_themes_have_opacity()
{
    let themes = vec![
        PresetTheme::CatppuccinMocha,
        PresetTheme::CatppuccinMacchiato,
        PresetTheme::CatppuccinFrappe,
        PresetTheme::CatppuccinLatte,
        PresetTheme::Dracula,
        PresetTheme::Nord,
        PresetTheme::GruvboxDark,
        PresetTheme::GruvboxLight,
        PresetTheme::TokyoNight,
        PresetTheme::TokyoNightStorm,
        PresetTheme::TokyoNightLight,
    ];

    for theme in themes {
        let appearance = theme.to_appearance();
        assert!(appearance.opacity > 0.0 && appearance.opacity <= 1.0);
    }
}

#[test]
fn all_themes_have_menu_opacity()
{
    let themes = vec![
        PresetTheme::CatppuccinMocha,
        PresetTheme::CatppuccinMacchiato,
        PresetTheme::CatppuccinFrappe,
        PresetTheme::CatppuccinLatte,
        PresetTheme::Dracula,
        PresetTheme::Nord,
        PresetTheme::GruvboxDark,
        PresetTheme::GruvboxLight,
        PresetTheme::TokyoNight,
        PresetTheme::TokyoNightStorm,
        PresetTheme::TokyoNightLight,
    ];

    for theme in themes {
        let appearance = theme.to_appearance();
        assert!(appearance.menu.opacity > 0.0 && appearance.menu.opacity <= 1.0);
    }
}

#[test]
fn all_themes_have_scale_factor()
{
    let themes = vec![
        PresetTheme::CatppuccinMocha,
        PresetTheme::CatppuccinMacchiato,
        PresetTheme::CatppuccinFrappe,
        PresetTheme::CatppuccinLatte,
        PresetTheme::Dracula,
        PresetTheme::Nord,
        PresetTheme::GruvboxDark,
        PresetTheme::GruvboxLight,
        PresetTheme::TokyoNight,
        PresetTheme::TokyoNightStorm,
        PresetTheme::TokyoNightLight,
    ];

    for theme in themes {
        let appearance = theme.to_appearance();
        assert!(appearance.scale_factor > 0.0);
    }
}

#[test]
fn deserialize_preset_theme_from_string()
{
    let toml_content = r#"
        appearance = "catppuccin-mocha"
    "#;

    #[derive(serde::Deserialize,)]
    struct TestConfig
    {
        #[serde(deserialize_with = "super::themes::deserialize_theme_or_appearance")]
        appearance: Appearance,
    }

    let config: TestConfig = ::toml::from_str(toml_content,).expect("Failed to deserialize",);
    assert_eq!(
        config.appearance.background_color,
        AppearanceColor::Simple(HexColor::rgb(30, 30, 46))
    );
}

#[test]
fn deserialize_custom_appearance()
{
    let toml_content = r###"
        [appearance]
        opacity = 0.85
        background_color = "#1a1b26"
    "###;

    #[derive(serde::Deserialize,)]
    struct TestConfig
    {
        #[serde(deserialize_with = "super::themes::deserialize_theme_or_appearance")]
        appearance: Appearance,
    }

    let config: TestConfig = ::toml::from_str(toml_content,).expect("Failed to deserialize",);
    assert_eq!(config.appearance.opacity, 0.85);
    assert_eq!(
        config.appearance.background_color,
        AppearanceColor::Simple(HexColor::rgb(26, 27, 38))
    );
}

#[test]
fn preset_theme_takes_precedence_over_appearance_fields()
{
    let toml_content = r#"
        appearance = "dracula"
    "#;

    #[derive(serde::Deserialize,)]
    struct TestConfig
    {
        #[serde(deserialize_with = "super::themes::deserialize_theme_or_appearance")]
        appearance: Appearance,
    }

    let config: TestConfig = ::toml::from_str(toml_content,).expect("Failed to deserialize",);
    assert_eq!(
        config.appearance.background_color,
        AppearanceColor::Simple(HexColor::rgb(40, 42, 54))
    );
}

#[test]
fn catppuccin_macchiato_colors()
{
    let appearance = PresetTheme::CatppuccinMacchiato.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(36, 39, 58)));
    assert_eq!(appearance.text_color, AppearanceColor::Simple(HexColor::rgb(202, 211, 245)));
}

#[test]
fn catppuccin_frappe_colors()
{
    let appearance = PresetTheme::CatppuccinFrappe.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(48, 52, 70)));
    assert_eq!(appearance.text_color, AppearanceColor::Simple(HexColor::rgb(198, 208, 245)));
}

#[test]
fn catppuccin_latte_colors()
{
    let appearance = PresetTheme::CatppuccinLatte.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(239, 241, 245)));
    assert_eq!(appearance.text_color, AppearanceColor::Simple(HexColor::rgb(76, 79, 105)));
}

#[test]
fn tokyo_night_storm_colors()
{
    let appearance = PresetTheme::TokyoNightStorm.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(36, 40, 59)));
}

#[test]
fn tokyo_night_light_colors()
{
    let appearance = PresetTheme::TokyoNightLight.to_appearance();
    assert_eq!(appearance.background_color, AppearanceColor::Simple(HexColor::rgb(213, 214, 219)));
}

#[test]
fn all_themes_have_animations_enabled()
{
    let themes = vec![
        PresetTheme::CatppuccinMocha,
        PresetTheme::CatppuccinMacchiato,
        PresetTheme::CatppuccinFrappe,
        PresetTheme::CatppuccinLatte,
        PresetTheme::Dracula,
        PresetTheme::Nord,
        PresetTheme::GruvboxDark,
        PresetTheme::GruvboxLight,
        PresetTheme::TokyoNight,
        PresetTheme::TokyoNightStorm,
        PresetTheme::TokyoNightLight,
    ];

    for theme in themes {
        let appearance = theme.to_appearance();
        assert!(appearance.animations.enabled);
        assert_eq!(appearance.animations.menu_fade_duration_ms, 200);
        assert_eq!(appearance.animations.hover_duration_ms, 100);
    }
}
