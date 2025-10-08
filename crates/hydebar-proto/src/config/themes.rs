use hex_color::HexColor;
use serde::{Deserialize, Deserializer};

use super::appearance::{
    AnimationConfig, Appearance, AppearanceColor, AppearanceStyle, MenuAppearance,
};

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq,)]
#[serde(rename_all = "kebab-case")]
pub enum PresetTheme
{
    CatppuccinMocha,
    CatppuccinMacchiato,
    CatppuccinFrappe,
    CatppuccinLatte,
    Dracula,
    Nord,
    GruvboxDark,
    GruvboxLight,
    TokyoNight,
    TokyoNightStorm,
    TokyoNightLight,
}

impl PresetTheme
{
    pub fn to_appearance(self,) -> Appearance
    {
        match self {
            Self::CatppuccinMocha => catppuccin_mocha(),
            Self::CatppuccinMacchiato => catppuccin_macchiato(),
            Self::CatppuccinFrappe => catppuccin_frappe(),
            Self::CatppuccinLatte => catppuccin_latte(),
            Self::Dracula => dracula(),
            Self::Nord => nord(),
            Self::GruvboxDark => gruvbox_dark(),
            Self::GruvboxLight => gruvbox_light(),
            Self::TokyoNight => tokyo_night(),
            Self::TokyoNightStorm => tokyo_night_storm(),
            Self::TokyoNightLight => tokyo_night_light(),
        }
    }
}

fn catppuccin_mocha() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(30, 30, 46,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(203, 166, 247,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(137, 180, 250,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(166, 227, 161,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(243, 139, 168,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(205, 214, 244,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(137, 180, 250,),),
            AppearanceColor::Simple(HexColor::rgb(203, 166, 247,),),
            AppearanceColor::Simple(HexColor::rgb(245, 194, 231,),),
            AppearanceColor::Simple(HexColor::rgb(250, 179, 135,),),
            AppearanceColor::Simple(HexColor::rgb(249, 226, 175,),),
            AppearanceColor::Simple(HexColor::rgb(166, 227, 161,),),
            AppearanceColor::Simple(HexColor::rgb(148, 226, 213,),),
            AppearanceColor::Simple(HexColor::rgb(137, 220, 235,),),
            AppearanceColor::Simple(HexColor::rgb(116, 199, 236,),),
            AppearanceColor::Simple(HexColor::rgb(180, 190, 254,),),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            235, 160, 172,
        ),)],),
    }
}

fn catppuccin_macchiato() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(36, 39, 58,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(198, 160, 246,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(138, 173, 244,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(166, 218, 149,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(237, 135, 150,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(202, 211, 245,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(138, 173, 244,),),
            AppearanceColor::Simple(HexColor::rgb(198, 160, 246,),),
            AppearanceColor::Simple(HexColor::rgb(245, 189, 230,),),
            AppearanceColor::Simple(HexColor::rgb(245, 169, 127,),),
            AppearanceColor::Simple(HexColor::rgb(238, 212, 159,),),
            AppearanceColor::Simple(HexColor::rgb(166, 218, 149,),),
            AppearanceColor::Simple(HexColor::rgb(139, 213, 202,),),
            AppearanceColor::Simple(HexColor::rgb(145, 215, 227,),),
            AppearanceColor::Simple(HexColor::rgb(125, 196, 228,),),
            AppearanceColor::Simple(HexColor::rgb(183, 189, 248,),),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            238, 153, 160,
        ),)],),
    }
}

fn catppuccin_frappe() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(48, 52, 70,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(202, 158, 230,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(140, 170, 238,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(166, 209, 137,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(231, 130, 132,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(198, 208, 245,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(140, 170, 238,),),
            AppearanceColor::Simple(HexColor::rgb(202, 158, 230,),),
            AppearanceColor::Simple(HexColor::rgb(244, 184, 228,),),
            AppearanceColor::Simple(HexColor::rgb(239, 159, 118,),),
            AppearanceColor::Simple(HexColor::rgb(229, 200, 144,),),
            AppearanceColor::Simple(HexColor::rgb(166, 209, 137,),),
            AppearanceColor::Simple(HexColor::rgb(129, 200, 190,),),
            AppearanceColor::Simple(HexColor::rgb(153, 209, 219,),),
            AppearanceColor::Simple(HexColor::rgb(133, 193, 220,),),
            AppearanceColor::Simple(HexColor::rgb(186, 187, 241,),),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            234, 153, 156,
        ),)],),
    }
}

fn catppuccin_latte() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(239, 241, 245,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(136, 57, 239,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(30, 102, 245,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(64, 160, 43,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(210, 15, 57,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(76, 79, 105,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(30, 102, 245,),),
            AppearanceColor::Simple(HexColor::rgb(136, 57, 239,),),
            AppearanceColor::Simple(HexColor::rgb(234, 118, 203,),),
            AppearanceColor::Simple(HexColor::rgb(254, 100, 11,),),
            AppearanceColor::Simple(HexColor::rgb(223, 142, 29,),),
            AppearanceColor::Simple(HexColor::rgb(64, 160, 43,),),
            AppearanceColor::Simple(HexColor::rgb(4, 165, 159,),),
            AppearanceColor::Simple(HexColor::rgb(23, 146, 153,),),
            AppearanceColor::Simple(HexColor::rgb(4, 165, 229,),),
            AppearanceColor::Simple(HexColor::rgb(114, 135, 253,),),
        ],
        special_workspace_colors: Some(vec![
            AppearanceColor::Simple(HexColor::rgb(230, 69, 83,),),
        ],),
    }
}

fn dracula() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(40, 42, 54,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(189, 147, 249,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(139, 233, 253,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(80, 250, 123,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(255, 85, 85,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(248, 248, 242,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(139, 233, 253,),),
            AppearanceColor::Simple(HexColor::rgb(189, 147, 249,),),
            AppearanceColor::Simple(HexColor::rgb(255, 121, 198,),),
            AppearanceColor::Simple(HexColor::rgb(255, 184, 108,),),
            AppearanceColor::Simple(HexColor::rgb(241, 250, 140,),),
            AppearanceColor::Simple(HexColor::rgb(80, 250, 123,),),
        ],
        special_workspace_colors: Some(vec![
            AppearanceColor::Simple(HexColor::rgb(255, 85, 85,),),
        ],),
    }
}

fn nord() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(46, 52, 64,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(136, 192, 208,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(129, 161, 193,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(163, 190, 140,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(191, 97, 106,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(236, 239, 244,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(129, 161, 193,),),
            AppearanceColor::Simple(HexColor::rgb(136, 192, 208,),),
            AppearanceColor::Simple(HexColor::rgb(143, 188, 187,),),
            AppearanceColor::Simple(HexColor::rgb(163, 190, 140,),),
            AppearanceColor::Simple(HexColor::rgb(235, 203, 139,),),
            AppearanceColor::Simple(HexColor::rgb(208, 135, 112,),),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(
            HexColor::rgb(191, 97, 106,),
        )],),
    }
}

fn gruvbox_dark() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(40, 40, 40,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(211, 134, 155,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(131, 165, 152,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(184, 187, 38,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(251, 73, 52,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(235, 219, 178,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(131, 165, 152,),),
            AppearanceColor::Simple(HexColor::rgb(211, 134, 155,),),
            AppearanceColor::Simple(HexColor::rgb(177, 98, 134,),),
            AppearanceColor::Simple(HexColor::rgb(254, 128, 25,),),
            AppearanceColor::Simple(HexColor::rgb(250, 189, 47,),),
            AppearanceColor::Simple(HexColor::rgb(184, 187, 38,),),
        ],
        special_workspace_colors: Some(vec![
            AppearanceColor::Simple(HexColor::rgb(251, 73, 52,),),
        ],),
    }
}

fn gruvbox_light() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(251, 241, 199,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(157, 0, 6,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(7, 102, 120,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(121, 116, 14,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(204, 36, 29,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(60, 56, 54,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(7, 102, 120,),),
            AppearanceColor::Simple(HexColor::rgb(157, 0, 6,),),
            AppearanceColor::Simple(HexColor::rgb(143, 63, 113,),),
            AppearanceColor::Simple(HexColor::rgb(175, 58, 3,),),
            AppearanceColor::Simple(HexColor::rgb(181, 118, 20,),),
            AppearanceColor::Simple(HexColor::rgb(121, 116, 14,),),
        ],
        special_workspace_colors: Some(vec![
            AppearanceColor::Simple(HexColor::rgb(204, 36, 29,),),
        ],),
    }
}

fn tokyo_night() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(26, 27, 38,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(187, 154, 247,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(122, 162, 247,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(158, 206, 106,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(247, 118, 142,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(192, 202, 245,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(122, 162, 247,),),
            AppearanceColor::Simple(HexColor::rgb(187, 154, 247,),),
            AppearanceColor::Simple(HexColor::rgb(255, 117, 127,),),
            AppearanceColor::Simple(HexColor::rgb(255, 158, 100,),),
            AppearanceColor::Simple(HexColor::rgb(224, 175, 104,),),
            AppearanceColor::Simple(HexColor::rgb(158, 206, 106,),),
            AppearanceColor::Simple(HexColor::rgb(115, 218, 202,),),
            AppearanceColor::Simple(HexColor::rgb(125, 207, 255,),),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            247, 118, 142,
        ),)],),
    }
}

fn tokyo_night_storm() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(36, 40, 59,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(187, 154, 247,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(122, 162, 247,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(158, 206, 106,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(247, 118, 142,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(166, 173, 200,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(122, 162, 247,),),
            AppearanceColor::Simple(HexColor::rgb(187, 154, 247,),),
            AppearanceColor::Simple(HexColor::rgb(255, 117, 127,),),
            AppearanceColor::Simple(HexColor::rgb(255, 158, 100,),),
            AppearanceColor::Simple(HexColor::rgb(224, 175, 104,),),
            AppearanceColor::Simple(HexColor::rgb(158, 206, 106,),),
            AppearanceColor::Simple(HexColor::rgb(115, 218, 202,),),
            AppearanceColor::Simple(HexColor::rgb(125, 207, 255,),),
        ],
        special_workspace_colors: Some(vec![AppearanceColor::Simple(HexColor::rgb(
            247, 118, 142,
        ),)],),
    }
}

fn tokyo_night_light() -> Appearance
{
    Appearance {
        font_name:                None,
        scale_factor:             1.0,
        style:                    AppearanceStyle::Islands,
        opacity:                  0.95,
        menu:                     MenuAppearance {
            opacity: 0.95, backdrop: 0.3,
        },
        animations:               AnimationConfig::default(),
        background_color:         AppearanceColor::Simple(HexColor::rgb(213, 214, 219,),),
        primary_color:            AppearanceColor::Simple(HexColor::rgb(121, 94, 172,),),
        secondary_color:          AppearanceColor::Simple(HexColor::rgb(52, 108, 197,),),
        success_color:            AppearanceColor::Simple(HexColor::rgb(51, 153, 51,),),
        danger_color:             AppearanceColor::Simple(HexColor::rgb(185, 29, 71,),),
        text_color:               AppearanceColor::Simple(HexColor::rgb(60, 62, 73,),),
        workspace_colors:         vec![
            AppearanceColor::Simple(HexColor::rgb(52, 108, 197,),),
            AppearanceColor::Simple(HexColor::rgb(121, 94, 172,),),
            AppearanceColor::Simple(HexColor::rgb(185, 29, 71,),),
            AppearanceColor::Simple(HexColor::rgb(166, 88, 24,),),
            AppearanceColor::Simple(HexColor::rgb(143, 94, 21,),),
            AppearanceColor::Simple(HexColor::rgb(51, 153, 51,),),
            AppearanceColor::Simple(HexColor::rgb(15, 155, 142,),),
            AppearanceColor::Simple(HexColor::rgb(29, 130, 183,),),
        ],
        special_workspace_colors: Some(vec![
            AppearanceColor::Simple(HexColor::rgb(185, 29, 71,),),
        ],),
    }
}

pub fn deserialize_theme_or_appearance<'de, D,>(deserializer: D,) -> Result<Appearance, D::Error,>
where
    D: Deserializer<'de,>,
{
    #[derive(Deserialize,)]
    #[serde(untagged)]
    enum ThemeOrAppearance
    {
        Theme(PresetTheme,),
        Appearance(Appearance,),
    }

    match ThemeOrAppearance::deserialize(deserializer,)? {
        ThemeOrAppearance::Theme(theme,) => Ok(theme.to_appearance(),),
        ThemeOrAppearance::Appearance(appearance,) => Ok(appearance,),
    }
}
