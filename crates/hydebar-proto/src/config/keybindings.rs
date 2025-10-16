use serde::Deserialize;

/// Keybindings configuration for keyboard navigation
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Keybindings {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub global:  GlobalKeybindings,
    #[serde(default)]
    pub menu:    MenuKeybindings,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            global:  GlobalKeybindings::default(),
            menu:    MenuKeybindings::default(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

/// Global keybindings for hydebar navigation mode
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GlobalKeybindings {
    #[serde(default = "default_activate_navigation")]
    pub activate_navigation: String,
}

impl Default for GlobalKeybindings {
    fn default() -> Self {
        Self {
            activate_navigation: default_activate_navigation(),
        }
    }
}

fn default_activate_navigation() -> String {
    "Super+h+b".to_owned()
}

/// Keybindings for menu navigation
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MenuKeybindings {
    #[serde(default = "default_up")]
    pub up:    String,
    #[serde(default = "default_down")]
    pub down:  String,
    #[serde(default = "default_left")]
    pub left:  String,
    #[serde(default = "default_right")]
    pub right: String,
}

impl Default for MenuKeybindings {
    fn default() -> Self {
        Self {
            up:    default_up(),
            down:  default_down(),
            left:  default_left(),
            right: default_right(),
        }
    }
}

fn default_up() -> String {
    "k".to_owned()
}

fn default_down() -> String {
    "j".to_owned()
}

fn default_left() -> String {
    "h".to_owned()
}

fn default_right() -> String {
    "l".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keybindings_default_is_enabled() {
        let kb = Keybindings::default();
        assert!(kb.enabled);
    }

    #[test]
    fn global_keybindings_default_activation() {
        let global = GlobalKeybindings::default();
        assert_eq!(global.activate_navigation, "Super+h+b");
    }

    #[test]
    fn menu_keybindings_defaults_are_vim_style() {
        let menu = MenuKeybindings::default();
        assert_eq!(menu.up, "k");
        assert_eq!(menu.down, "j");
        assert_eq!(menu.left, "h");
        assert_eq!(menu.right, "l");
    }

    #[test]
    fn keybindings_can_be_disabled() {
        let kb = Keybindings {
            enabled: false,
            global:  GlobalKeybindings::default(),
            menu:    MenuKeybindings::default(),
        };
        assert!(!kb.enabled);
    }
}
