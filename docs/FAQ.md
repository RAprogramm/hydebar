# Frequently Asked Questions

## General

### What is hydebar?

hydebar is a fast, beautiful Wayland status bar built specifically for Hyprland. It provides all the features you need in a modern desktop panel: workspaces, system info, media controls, and more.

### Why use hydebar instead of Waybar or HyprPanel?

**vs Waybar:**
- ✅ Faster (100% Rust vs C++)
- ✅ Better Hyprland integration
- ✅ Built-in themes
- ✅ Smooth animations
- ✅ Lower memory usage

**vs HyprPanel:**
- ✅ Much faster (Rust vs TypeScript/GTK)
- ✅ Lower resource usage
- ✅ Native Wayland (no GTK overhead)
- ✅ More stable
- ❌ Currently fewer widgets (weather, calendar coming soon)

### Is it stable for daily use?

Yes! hydebar is actively developed and tested. Current version (v0.6.7) is stable for daily use. Report any bugs on [GitHub Issues](https://github.com/RAprogramm/hydebar/issues).

---

## Installation

### How do I install on Arch Linux?

```bash
paru -S hydebar
```

Or for latest development version:
```bash
paru -S hydebar-git
```

### How do I install on other distros?

See [README.md](../README.md#installation) for:
- Nix/NixOS
- ALT Linux
- Building from source

### How do I auto-start with Hyprland?

Add to `~/.config/hypr/hyprland.conf`:
```conf
exec-once = hydebar
```

---

## Configuration

### Where is the config file?

`~/.config/hydebar/config.toml`

Create it if it doesn't exist. See [Getting Started](GETTING_STARTED.md) for examples.

### Do I need to restart after config changes?

No! hydebar automatically reloads when you save config changes.

### Can I use multiple config files?

Yes, pass a custom config path:
```bash
hydebar --config-path ~/my-config.toml
```

### How do I reset to defaults?

Delete or rename your config file:
```bash
mv ~/.config/hydebar/config.toml ~/.config/hydebar/config.toml.backup
```

hydebar will use built-in defaults.

---

## Themes

### How many themes are included?

11 preset themes:
- Catppuccin (4 variants)
- Dracula
- Nord
- Gruvbox (2 variants)
- Tokyo Night (3 variants)

See [THEMES.md](THEMES.md) for previews.

### How do I change themes?

Edit `~/.config/hydebar/config.toml`:
```toml
appearance = "catppuccin-mocha"
```

Changes apply instantly!

### Can I create custom themes?

Yes! Either:
1. Customize an existing theme
2. Define all colors manually

See [THEMES.md](THEMES.md#creating-custom-themes) for details.

### Can I submit new themes?

Yes! See [Contributing](#contributing) below.

---

## Modules

### What modules are available?

- Workspaces
- Window Title
- System Info (CPU, RAM, temp, disk, network)
- Clock
- Battery
- Network (WiFi, VPN)
- Audio
- Bluetooth
- Brightness
- Media Player
- Tray
- Updates
- Privacy (camera/mic indicators)
- Keyboard Layout
- App Launcher
- Settings Panel
- Custom modules

### Can I reorder modules?

Yes:
```toml
[modules]
left = ["Workspaces"]
center = ["WindowTitle"]
right = ["SystemInfo", "Clock", "Battery", "Settings"]
```

### Can I hide modules?

Yes, just remove them from your config:
```toml
[modules]
right = ["Clock"]  # Only show clock
```

### How do I create custom modules?

See configuration example:
```toml
[[CustomModule]]
name = "MyModule"
icon = "🔔"
command = "notify-send 'Clicked!'"
```

Advanced custom modules can update dynamically. See [README.md](../README.md#custom-modules).

---

## Performance

### How much RAM does hydebar use?

Typically < 10MB idle, target is < 5MB.

### How much CPU does it use?

< 1% idle, < 5% during active use (menu open, animations).

### How fast is startup?

Target is < 50ms first paint. Actual depends on your system and enabled modules.

### How can I reduce resource usage?

1. Disable animations:
```toml
[appearance.animations]
enabled = false
```

2. Use fewer modules:
```toml
[modules]
right = ["Clock"]
```

3. Reduce system info updates (future feature)

---

## Troubleshooting

### Transparency isn't working

Try forcing OpenGL:
```bash
WGPU_BACKEND=gl hydebar
```

### Icons show as boxes

Install icon fonts:
```bash
sudo pacman -S ttf-font-awesome ttf-nerd-fonts-symbols
```

### Battery module doesn't appear

Check UPower:
```bash
systemctl status upower
```

Or force show:
```toml
[battery]
show_when_unavailable = true
```

### More issues?

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for detailed solutions.

---

## Features

### Does it support multi-monitor?

Yes! hydebar automatically spawns on all outputs, or you can specify:
```toml
outputs = "All"  # Default
# outputs = "Active"
# outputs = { Targets = ["DP-1", "HDMI-1"] }
```

### Does it work on other Wayland compositors?

Partially. Some features require Hyprland:
- Workspaces
- Window title
- Keyboard layout

Generic modules (Clock, System Info, Tray) should work on other compositors, but this isn't officially supported yet.

### Is there a notification center?

Not yet. Planned for v0.9.0. Track progress: [#65](https://github.com/RAprogramm/hydebar/issues/65)

### Can I have a vertical panel?

Not yet. Planned for future versions.

### Can I auto-hide the panel?

Not yet. Planned for future versions.

---

## Development

### Is hydebar actively maintained?

Yes! Check [ROADMAP.md](../ROADMAP.md) for planned features and timeline.

### Can I contribute?

Yes! Contributions welcome. See [Contributing](#contributing) section below.

### What's the development stack?

- **Language:** 100% Rust (edition 2024)
- **GUI:** iced (Pop!_OS fork)
- **IPC:** Hyprland socket
- **D-Bus:** zbus for system integration
- **Build:** Cargo

### Where's the source code?

[GitHub: RAprogramm/hydebar](https://github.com/RAprogramm/hydebar)

---

## Contributing

### How can I contribute?

Several ways:
1. **Report bugs** - [Open an issue](https://github.com/RAprogramm/hydebar/issues/new)
2. **Request features** - [Start a discussion](https://github.com/RAprogramm/hydebar/discussions)
3. **Submit themes** - Create PR with new preset theme
4. **Write code** - Check [ROADMAP.md](../ROADMAP.md) for planned features
5. **Improve docs** - Fix typos, add examples

### What should I work on?

Check [ROADMAP.md](../ROADMAP.md) for:
- High priority features
- Good first issues
- Planned milestones

### How do I submit changes?

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

### Coding standards?

- Follow Rust conventions
- Run `cargo fmt` before committing
- Add tests for new features
- Update documentation

---

## Licensing

### What license is hydebar under?

MIT License. See [LICENSE](../LICENSE) for details.

### Can I use it commercially?

Yes, MIT license allows commercial use.

### Can I fork/modify it?

Yes! That's encouraged. Please keep the MIT license notice.

---

## Support

### Where do I get help?

1. Check this FAQ
2. Read [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
3. Search [existing issues](https://github.com/RAprogramm/hydebar/issues)
4. Ask in [Discussions](https://github.com/RAprogramm/hydebar/discussions)
5. Open a [new issue](https://github.com/RAprogramm/hydebar/issues/new)

### How do I report bugs?

Open an issue with:
- hydebar version
- System info (OS, Hyprland version)
- Config file (sanitized)
- Steps to reproduce
- Debug logs if relevant

### Can I request features?

Yes! Open a discussion or issue describing:
- What you want
- Why it's useful
- How it might work

---

## Roadmap

### What's planned for the future?

See [ROADMAP.md](../ROADMAP.md) for detailed timeline.

**Upcoming (v0.8.0):**
- Performance optimizations
- Memory improvements
- Faster startup

**Future (v0.9.0+):**
- Notification center
- Weather widget
- Calendar widget
- More module improvements

### When is v1.0.0?

Target: Q2 2025

v1.0.0 will include:
- Full feature parity with HyprPanel
- GUI configuration panel
- Professional documentation
- Stable API

---

## Comparison

### hydebar vs Waybar

| Feature | hydebar | Waybar |
|---------|---------|--------|
| Language | Rust | C++ |
| Startup | <50ms | ~100ms |
| Memory | <5MB | ~10MB |
| Themes | 11 built-in | Manual CSS |
| Animations | Yes, smooth | Limited |
| Hyprland | Deep integration | Generic |

### hydebar vs HyprPanel

| Feature | hydebar | HyprPanel |
|---------|---------|-----------|
| Language | Rust | TypeScript |
| Performance | Fast | Moderate |
| Memory | <10MB | ~50MB+ |
| Startup | <50ms | ~500ms |
| Widgets | Growing | More |
| Stability | High | Moderate |

---

**Have more questions?** Ask in [Discussions](https://github.com/RAprogramm/hydebar/discussions)!
