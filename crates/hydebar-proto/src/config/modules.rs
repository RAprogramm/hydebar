use std::fmt;

use serde::{Deserialize, Deserializer, de::Error as _};

/// Bar placement configuration.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq,)]
pub enum Position
{
    /// Render the bar at the top of the output.
    #[default]
    Top,
    /// Render the bar at the bottom of the output.
    Bottom,
}

/// Named module variants supported by the bar.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,)]
pub enum ModuleName
{
    AppLauncher,
    Updates,
    Clipboard,
    Workspaces,
    WindowTitle,
    SystemInfo,
    KeyboardLayout,
    KeyboardSubmap,
    Tray,
    Clock,
    Battery,
    Privacy,
    Settings,
    MediaPlayer,
    Custom(String,),
}

impl<'de,> Deserialize<'de,> for ModuleName
{
    fn deserialize<D,>(deserializer: D,) -> Result<ModuleName, D::Error,>
    where
        D: Deserializer<'de,>,
    {
        struct ModuleNameVisitor;

        impl<'de,> serde::de::Visitor<'de,> for ModuleNameVisitor
        {
            type Value = ModuleName;

            fn expecting(&self, formatter: &mut fmt::Formatter,) -> fmt::Result
            {
                formatter.write_str("a string representing a ModuleName",)
            }

            fn visit_str<E,>(self, value: &str,) -> Result<ModuleName, E,>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "AppLauncher" => ModuleName::AppLauncher,
                    "Updates" => ModuleName::Updates,
                    "Clipboard" => ModuleName::Clipboard,
                    "Workspaces" => ModuleName::Workspaces,
                    "WindowTitle" => ModuleName::WindowTitle,
                    "SystemInfo" => ModuleName::SystemInfo,
                    "KeyboardLayout" => ModuleName::KeyboardLayout,
                    "KeyboardSubmap" => ModuleName::KeyboardSubmap,
                    "Tray" => ModuleName::Tray,
                    "Clock" => ModuleName::Clock,
                    "Battery" => ModuleName::Battery,
                    "Privacy" => ModuleName::Privacy,
                    "Settings" => ModuleName::Settings,
                    "MediaPlayer" => ModuleName::MediaPlayer,
                    other => ModuleName::Custom(other.to_string(),),
                },)
            }
        }

        deserializer.deserialize_str(ModuleNameVisitor,)
    }
}

/// Layout definition describing which modules render in each region.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq,)]
#[serde(untagged)]
pub enum ModuleDef
{
    Single(ModuleName,),
    Group(Vec<ModuleName,>,),
}

/// Overall module layout configuration.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq,)]
pub struct Modules
{
    #[serde(default)]
    pub left:   Vec<ModuleDef,>,
    #[serde(default)]
    pub center: Vec<ModuleDef,>,
    #[serde(default)]
    pub right:  Vec<ModuleDef,>,
}

impl Default for Modules
{
    fn default() -> Self
    {
        Self {
            left:   vec![ModuleDef::Single(ModuleName::Workspaces,)],
            center: vec![ModuleDef::Single(ModuleName::WindowTitle,)],
            right:  vec![ModuleDef::Group(vec![
                ModuleName::Clock,
                ModuleName::Privacy,
                ModuleName::Battery,
                ModuleName::Settings,
            ],)],
        }
    }
}

/// Output targeting configuration for module rendering.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Default,)]
pub enum Outputs
{
    /// Render on all outputs.
    #[default]
    All,
    /// Render on the currently focused output.
    Active,
    /// Render on the explicitly configured output list.
    #[serde(deserialize_with = "non_empty")]
    Targets(Vec<String,>,),
}

fn non_empty<'de, D, T,>(deserializer: D,) -> Result<Vec<T,>, D::Error,>
where
    D: Deserializer<'de,>,
    T: Deserialize<'de,>,
{
    let values = <Vec<T,>>::deserialize(deserializer,)?;

    if values.is_empty() { Err(D::Error::custom("need non-empty",),) } else { Ok(values,) }
}

#[cfg(test)]
mod tests
{
    use serde::de::value::{Error as DeError, SeqDeserializer, StrDeserializer};

    use super::*;

    #[test]
    fn default_modules_match_expected_layout()
    {
        let modules = Modules::default();
        assert_eq!(modules.left.len(), 1);
        assert_eq!(modules.center.len(), 1);
        assert_eq!(modules.right.len(), 1);
    }

    #[test]
    fn non_empty_rejects_empty_vectors()
    {
        let error: DeError = non_empty::<_, String,>(SeqDeserializer::<_, DeError,>::new(
            Vec::<String,>::new().into_iter(),
        ),)
        .expect_err("empty list should fail",);
        assert!(error.to_string().contains("non-empty"));
    }

    #[test]
    fn module_name_deserializes_custom_values()
    {
        let name = ModuleName::deserialize(StrDeserializer::<DeError,>::new("MyCustom",),)
            .expect("custom variant",);
        assert!(matches!(name, ModuleName::Custom(value) if value == "MyCustom"));
    }
}
