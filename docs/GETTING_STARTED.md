# Getting Started with hydebar

This guide will help you install and configure hydebar in just a few minutes.

## Prerequisites

- **Hyprland** compositor
- **Wayland** session
- **Rust** toolchain (for building from source)

## Installation

### Arch Linux (Recommended)

The easiest way to install on Arch:

```bash
paru -S hydebar
```

Or for the latest development version:

```bash
paru -S hydebar-git
```

### Other Distributions

See [README.md](../README.md#installation) for Nix, ALT Linux, and other options.

### Building from Source

```bash
# Clone repository
git clone https://github.com/RAprogramm/hydebar.git
cd hydebar

# Build release version
cargo build --release

# Binary will be at: target/release/hydebar-app
```

## First Run

### Basic Setup

1. Create config directory:
```bash
mkdir -p ~/.config/hydebar
```

2. Create minimal config file `~/.config/hydebar/config.toml`:
```toml
# Use a preset theme
appearance = "catppuccin-mocha"
```

3. Run hydebar:
```bash
hydebar
```

That's it! You should see a beautiful status bar with the Catppuccin Mocha theme.

### Auto-start with Hyprland

Add to your `~/.config/hypr/hyprland.conf`:

```conf
exec-once = hydebar
```

## Choosing a Theme

hydebar includes 11 beautiful preset themes. Try them by editing your config:

```toml
# Dark themes
appearance = "catppuccin-mocha"      # Purple/pink (default)
appearance = "dracula"               # Purple/pink
appearance = "nord"                  # Cool blue
appearance = "gruvbox-dark"          # Warm retro
appearance = "tokyo-night"           # Neon accents

# Light themes
appearance = "catppuccin-latte"      # Pastel light
appearance = "gruvbox-light"         # Warm light
appearance = "tokyo-night-light"     # Clean light
```

Changes apply instantly - no restart needed!

## Customizing Layout

Configure which modules appear and where:

```toml
[modules]
left = ["Workspaces"]
center = ["WindowTitle"]
right = ["SystemInfo", "Clock", "Battery", "Settings"]
```

Available modules:
- `Workspaces` - Hyprland workspaces
- `WindowTitle` - Active window
- `SystemInfo` - CPU/RAM/temp
- `Clock` - Date and time
- `Battery` - Battery status
- `MediaPlayer` - Music controls
- `Tray` - System tray
- `Privacy` - Camera/mic indicators
- `Settings` - Settings panel
- Custom modules (see Advanced section)

## Common Configurations

### Minimal Setup

```toml
appearance = "nord"

[modules]
left = ["Workspaces"]
center = []
right = ["Clock"]
```

### Full-Featured

```toml
appearance = "catppuccin-mocha"

[modules]
left = ["Workspaces"]
center = ["WindowTitle"]
right = [
    "SystemInfo",
    ["Clock", "Privacy", "Battery", "Settings"]
]

# Show system info
[system]
indicators = ["Cpu", "Memory", "Temperature", "DownloadSpeed"]

# Configure clock
[clock]
format = "%a %d %b %H:%M"
```

### Custom Colors

Instead of a preset theme, you can customize every color:

```toml
[appearance]
style = "Islands"
opacity = 0.95

background_color = "#1e1e2e"
primary_color = "#cba6f7"
secondary_color = "#11111b"
success_color = "#a6e3a1"
danger_color = "#f38ba8"
text_color = "#cdd6f4"
```

## Animations

Control menu animations:

```toml
[appearance.animations]
enabled = true
menu_fade_duration_ms = 200  # Fade duration in milliseconds
hover_duration_ms = 100      # Hover effect duration
```

Disable animations entirely:

```toml
[appearance.animations]
enabled = false
```

## Next Steps

- [Full Configuration Guide](CONFIGURATION.md) - All options explained
- [Theme Showcase](THEMES.md) - Preview all themes
- [Troubleshooting](TROUBLESHOOTING.md) - Common issues
- [Module Reference](MODULES.md) - Per-module settings

## Quick Tips

1. **Config reloads automatically** - Edit and save, changes appear instantly
2. **Use preset themes** - Easier than manual colors
3. **Group modules** - Use nested arrays: `["Clock", "Battery"]`
4. **Check logs** - Run with `RUST_LOG=debug hydebar` for debugging

## Getting Help

- [GitHub Issues](https://github.com/RAprogramm/hydebar/issues) - Bug reports
- [Discussions](https://github.com/RAprogramm/hydebar/discussions) - Questions
- [ROADMAP.md](../ROADMAP.md) - Planned features

---

**Welcome to hydebar!** Enjoy your beautiful new status bar.
