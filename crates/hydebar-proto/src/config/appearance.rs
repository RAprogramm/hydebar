use hex_color::HexColor;
use iced::{Color, theme::palette};
use serde::{Deserialize, Deserializer, de::Error as _};

/// Color palette configuration used to render UI elements.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum AppearanceColor {
    /// Simple color variant with a single hex value.
    Simple(HexColor),
    /// Complete palette variant with additional semantic colors.
    Complete {
        base:   HexColor,
        strong: Option<HexColor>,
        weak:   Option<HexColor>,
        text:   Option<HexColor>
    }
}

impl AppearanceColor {
    /// Returns the base [`Color`] representation for the configured palette.
    #[must_use]
    pub fn get_base(&self) -> Color {
        match self {
            AppearanceColor::Simple(color) => Color::from_rgb8(color.r, color.g, color.b),
            AppearanceColor::Complete {
                base, ..
            } => Color::from_rgb8(base.r, base.g, base.b)
        }
    }

    /// Returns the text [`Color`] if configured.
    #[must_use]
    pub fn get_text(&self) -> Option<Color> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete {
                text, ..
            } => text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
        }
    }

    /// Builds the weak [`palette::Pair`] variant if available.
    #[must_use]
    pub fn get_weak_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete {
                weak,
                text,
                ..
            } => weak.map(|color| {
                palette::Pair::new(
                    Color::from_rgb8(color.r, color.g, color.b),
                    text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
                        .unwrap_or(text_fallback)
                )
            })
        }
    }

    /// Builds the strong [`palette::Pair`] variant if available.
    #[must_use]
    pub fn get_strong_pair(&self, text_fallback: Color) -> Option<palette::Pair> {
        match self {
            AppearanceColor::Simple(_) => None,
            AppearanceColor::Complete {
                strong,
                text,
                ..
            } => strong.map(|color| {
                palette::Pair::new(
                    Color::from_rgb8(color.r, color.g, color.b),
                    text.map(|color| Color::from_rgb8(color.r, color.g, color.b))
                        .unwrap_or(text_fallback)
                )
            })
        }
    }
}

/// Enumeration of available appearance styles.
#[derive(Deserialize, Default, Copy, Clone, Eq, PartialEq, Debug)]
pub enum AppearanceStyle {
    /// Render modules with island-style backgrounds.
    #[default]
    Islands,
    /// Render modules with a flat solid background.
    Solid,
    /// Render modules with gradients.
    Gradient
}

/// Menu-specific appearance configuration.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct MenuAppearance {
    #[serde(deserialize_with = "opacity_deserializer", default = "default_opacity")]
    pub opacity:  f32,
    #[serde(default)]
    pub backdrop: f32
}

impl Default for MenuAppearance {
    fn default() -> Self {
        Self {
            opacity:  default_opacity(),
            backdrop: f32::default()
        }
    }
}

/// Animation configuration.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AnimationConfig {
    #[serde(default = "default_animations_enabled")]
    pub enabled:               bool,
    #[serde(default = "default_menu_fade_duration_ms")]
    pub menu_fade_duration_ms: u64,
    #[serde(default = "default_hover_duration_ms")]
    pub hover_duration_ms:     u64
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled:               default_animations_enabled(),
            menu_fade_duration_ms: default_menu_fade_duration_ms(),
            hover_duration_ms:     default_hover_duration_ms()
        }
    }
}

fn default_animations_enabled() -> bool {
    true
}

fn default_menu_fade_duration_ms() -> u64 {
    200
}

fn default_hover_duration_ms() -> u64 {
    100
}

/// Top-level appearance configuration.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Appearance {
    #[serde(default)]
    pub font_name:                Option<String>,
    #[serde(
        deserialize_with = "scale_factor_deserializer",
        default = "default_scale_factor"
    )]
    pub scale_factor:             f64,
    #[serde(default)]
    pub style:                    AppearanceStyle,
    #[serde(deserialize_with = "opacity_deserializer", default = "default_opacity")]
    pub opacity:                  f32,
    #[serde(default)]
    pub menu:                     MenuAppearance,
    #[serde(default)]
    pub animations:               AnimationConfig,
    #[serde(default = "default_background_color")]
    pub background_color:         AppearanceColor,
    #[serde(default = "default_primary_color")]
    pub primary_color:            AppearanceColor,
    #[serde(default = "default_secondary_color")]
    pub secondary_color:          AppearanceColor,
    #[serde(default = "default_success_color")]
    pub success_color:            AppearanceColor,
    #[serde(default = "default_danger_color")]
    pub danger_color:             AppearanceColor,
    #[serde(default = "default_text_color")]
    pub text_color:               AppearanceColor,
    #[serde(default = "default_workspace_colors")]
    pub workspace_colors:         Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>
}

static PRIMARY: HexColor = HexColor::rgb(250, 179, 135);

fn scale_factor_deserializer<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>
{
    let value = f64::deserialize(deserializer)?;

    if value <= 0.0 {
        return Err(D::Error::custom("Scale factor must be greater than 0.0"));
    }

    if value > 2.0 {
        return Err(D::Error::custom("Scale factor cannot be greater than 2.0"));
    }

    Ok(value)
}

fn default_scale_factor() -> f64 {
    1.0
}

fn opacity_deserializer<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>
{
    let value = f32::deserialize(deserializer)?;

    if value < 0.0 {
        return Err(D::Error::custom("Opacity cannot be negative"));
    }

    if value > 1.0 {
        return Err(D::Error::custom("Opacity cannot be greater than 1.0"));
    }

    Ok(value)
}

fn default_opacity() -> f32 {
    1.0
}

fn default_background_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base:   HexColor::rgb(30, 30, 46),
        strong: Some(HexColor::rgb(69, 71, 90)),
        weak:   Some(HexColor::rgb(49, 50, 68)),
        text:   None
    }
}

fn default_primary_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base:   PRIMARY,
        strong: None,
        weak:   None,
        text:   Some(HexColor::rgb(30, 30, 46))
    }
}

fn default_secondary_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base:   HexColor::rgb(17, 17, 27),
        strong: Some(HexColor::rgb(24, 24, 37)),
        weak:   None,
        text:   None
    }
}

fn default_success_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(166, 227, 161))
}

fn default_danger_color() -> AppearanceColor {
    AppearanceColor::Complete {
        base:   HexColor::rgb(243, 139, 168),
        weak:   Some(HexColor::rgb(249, 226, 175)),
        strong: None,
        text:   None
    }
}

fn default_text_color() -> AppearanceColor {
    AppearanceColor::Simple(HexColor::rgb(205, 214, 244))
}

fn default_workspace_colors() -> Vec<AppearanceColor> {
    vec![
        AppearanceColor::Simple(PRIMARY),
        AppearanceColor::Simple(HexColor::rgb(180, 190, 254)),
        AppearanceColor::Simple(HexColor::rgb(203, 166, 247)),
    ]
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font_name:                None,
            scale_factor:             1.0,
            style:                    AppearanceStyle::default(),
            opacity:                  default_opacity(),
            menu:                     MenuAppearance::default(),
            animations:               AnimationConfig::default(),
            background_color:         default_background_color(),
            primary_color:            default_primary_color(),
            secondary_color:          default_secondary_color(),
            success_color:            default_success_color(),
            danger_color:             default_danger_color(),
            text_color:               default_text_color(),
            workspace_colors:         default_workspace_colors(),
            special_workspace_colors: None
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::de::value::{Error as DeError, F32Deserializer, F64Deserializer};

    use super::*;

    #[test]
    fn default_appearance_has_expected_colors() {
        let appearance = Appearance::default();
        assert_eq!(appearance.opacity, 1.0);
        assert_eq!(appearance.workspace_colors.len(), 3);
        assert!(appearance.text_color.get_text().is_none());
    }

    #[test]
    fn scale_factor_deserializer_rejects_out_of_bounds_values() {
        let err_small: DeError = scale_factor_deserializer(F64Deserializer::<DeError>::new(0.0))
            .expect_err("scale factor <= 0 should error");
        assert!(err_small.to_string().contains("greater than 0.0"));

        let err_large: DeError = scale_factor_deserializer(F64Deserializer::<DeError>::new(2.1))
            .expect_err("scale factor > 2 should error");
        assert!(err_large.to_string().contains("greater than 2.0"));
    }

    #[test]
    fn opacity_deserializer_rejects_invalid_values() {
        let err_negative: DeError = opacity_deserializer(F32Deserializer::<DeError>::new(-0.1))
            .expect_err("negative opacity should error");
        assert!(err_negative.to_string().contains("cannot be negative"));

        let err_large: DeError = opacity_deserializer(F32Deserializer::<DeError>::new(1.1))
            .expect_err("opacity > 1 should error");
        assert!(err_large.to_string().contains("greater than 1.0"));
    }

    #[test]
    fn appearance_color_pairs_use_text_fallback() {
        let fallback = Color::from_rgb8(255, 255, 255);
        let color = AppearanceColor::Complete {
            base:   HexColor::rgb(1, 2, 3),
            strong: Some(HexColor::rgb(4, 5, 6)),
            weak:   Some(HexColor::rgb(7, 8, 9)),
            text:   None
        };

        let strong = color.get_strong_pair(fallback).expect("strong pair");
        assert_eq!(strong.text, fallback);

        let weak = color.get_weak_pair(fallback).expect("weak pair");
        assert_eq!(weak.text, fallback);
    }

    #[test]
    fn animation_config_default_values() {
        let config = AnimationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.menu_fade_duration_ms, 200);
        assert_eq!(config.hover_duration_ms, 100);
    }

    #[test]
    fn appearance_default_includes_animations() {
        let appearance = Appearance::default();
        assert!(appearance.animations.enabled);
        assert_eq!(appearance.animations.menu_fade_duration_ms, 200);
    }
}
