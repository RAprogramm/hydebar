# Troubleshooting Guide

Common issues and solutions for hydebar.

## Graphics Issues

### Transparency Not Working

**Symptoms:** Bar appears fully opaque despite opacity settings

**Solutions:**

1. Force OpenGL backend:
```bash
WGPU_BACKEND=gl hydebar
```

2. Check compositor support:
```bash
# Verify Hyprland is running
pidof Hyprland

# Check layer-shell protocol
wayland-info | grep layer_shell
```

3. Update graphics drivers:
```bash
# Arch Linux - Update all packages
sudo pacman -Syu

# Check Vulkan support
vulkaninfo | grep deviceName
```

### Visual Artifacts or Corruption

**Symptoms:** Flickering, garbled text, missing elements

**Solutions:**

1. Try OpenGL instead of Vulkan:
```bash
WGPU_BACKEND=gl hydebar
```

2. Disable animations temporarily:
```toml
[appearance.animations]
enabled = false
```

3. Reduce opacity:
```toml
[appearance]
opacity = 1.0  # Fully opaque
```

### Icons Not Displaying

**Symptoms:** Square boxes instead of icons

**Solutions:**

1. Install required fonts:
```bash
# Arch Linux
sudo pacman -S ttf-font-awesome ttf-nerd-fonts-symbols

# Ubuntu/Debian
sudo apt install fonts-font-awesome fonts-nerd-font
```

2. Check tray icon theme:
```bash
# Verify icon theme is installed
ls ~/.local/share/icons
ls /usr/share/icons
```

---

## Performance Issues

### High CPU Usage

**Symptoms:** CPU constantly above 5% when idle

**Solutions:**

1. Check what's updating:
```bash
RUST_LOG=debug hydebar 2>&1 | grep -i update
```

2. Disable expensive modules:
```toml
[modules]
# Remove or comment out heavy modules
right = ["Clock", "Settings"]  # Minimal config
```

3. Increase update intervals:
```toml
[system]
# Reduce system monitoring frequency
update_interval_ms = 2000  # Update every 2 seconds
```

### High Memory Usage

**Symptoms:** hydebar using > 50MB RAM

**Solutions:**

1. Check for memory leaks:
```bash
# Monitor memory over time
watch -n 1 'ps aux | grep hydebar'
```

2. Restart hydebar periodically:
```bash
# Add to Hyprland config for daily restart
exec-once = while true; do hydebar; sleep 86400; done
```

3. Report issue with details:
```bash
# Collect memory info
ps aux | grep hydebar > memory-report.txt
```

---

## Module Issues

### Workspaces Not Updating

**Symptoms:** Workspace indicator stuck or not changing

**Solutions:**

1. Verify Hyprland socket:
```bash
echo $HYPRLAND_INSTANCE_SIGNATURE
ls /tmp/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock
```

2. Restart Hyprland IPC:
```bash
killall -SIGUSR1 Hyprland
```

3. Check config:
```toml
[workspaces]
visibility_mode = "All"  # Or "MonitorSpecific"
```

### Battery Module Not Showing

**Symptoms:** Battery module missing even on laptop

**Solutions:**

1. Check UPower service:
```bash
systemctl status upower
```

2. Verify battery detection:
```bash
upower -e
upower -i /org/freedesktop/UPower/devices/battery_BAT0
```

3. Show battery even when unavailable:
```toml
[battery]
show_when_unavailable = true
```

### Tray Icons Missing

**Symptoms:** System tray empty or some apps missing

**Solutions:**

1. Check SNI protocol support:
```bash
# Verify apps support StatusNotifierItem
dbus-send --session --print-reply \
  --dest=org.freedesktop.DBus \
  /org/freedesktop/DBus \
  org.freedesktop.DBus.ListNames | grep StatusNotifier
```

2. Use SNI-compatible apps:
- Use `nm-applet` instead of older applets
- Use `blueman-applet` for Bluetooth

3. Restart tray apps:
```bash
killall nm-applet && nm-applet &
```

---

## Configuration Issues

### Config Not Loading

**Symptoms:** Changes to config.toml not applying

**Solutions:**

1. Check config file location:
```bash
ls -la ~/.config/hydebar/config.toml
```

2. Verify TOML syntax:
```bash
# Use online TOML validator or:
cargo install taplo-cli
taplo check ~/.config/hydebar/config.toml
```

3. Check for parse errors:
```bash
RUST_LOG=info hydebar 2>&1 | grep -i config
```

### Theme Not Applying

**Symptoms:** Theme name doesn't work

**Solutions:**

1. Use exact theme name:
```toml
appearance = "catppuccin-mocha"  # Correct
# appearance = "catppuccin mocha"  # Wrong - no spaces
# appearance = "CatppuccinMocha"   # Wrong - case sensitive
```

2. Available themes:
```
catppuccin-mocha
catppuccin-macchiato
catppuccin-frappe
catppuccin-latte
dracula
nord
gruvbox-dark
gruvbox-light
tokyo-night
tokyo-night-storm
tokyo-night-light
```

3. Fall back to custom colors:
```toml
[appearance]
# If theme fails, use manual colors
background_color = "#1e1e2e"
primary_color = "#cba6f7"
```

---

## Build Issues

### Compilation Errors

**Symptoms:** `cargo build` fails

**Solutions:**

1. Update Rust:
```bash
rustup update stable
rustc --version  # Should be 1.70+
```

2. Clean and rebuild:
```bash
cargo clean
cargo build --release
```

3. Check dependencies:
```bash
# Arch Linux
sudo pacman -S base-devel wayland wayland-protocols

# Ubuntu/Debian
sudo apt install build-essential libwayland-dev
```

### Missing Wayland Protocols

**Symptoms:** Build fails with wayland-scanner errors

**Solutions:**

1. Install wayland development packages:
```bash
# Arch Linux
sudo pacman -S wayland-protocols

# Ubuntu/Debian
sudo apt install wayland-protocols libwayland-dev
```

2. Set PKG_CONFIG_PATH:
```bash
export PKG_CONFIG_PATH=/usr/lib/pkgconfig:$PKG_CONFIG_PATH
cargo build --release
```

---

## Hyprland Integration

### Not Working on Other Compositors

**Symptoms:** Features missing on non-Hyprland compositors

**Current Status:** hydebar is designed primarily for Hyprland. Other compositors have limited support.

**Workaround:**

1. Disable Hyprland-specific modules:
```toml
[modules]
# Remove Hyprland-specific features
left = []  # No Workspaces
center = ["Clock"]  # Generic modules only
```

2. Use generic alternatives:
- Remove `Workspaces` module
- Remove `WindowTitle` module
- Remove `KeyboardLayout` module

**Future:** Feature flags for compositor-agnostic mode planned.

---

## Network Issues

### WiFi Not Showing

**Symptoms:** Network module empty or not updating

**Solutions:**

1. Check NetworkManager:
```bash
systemctl status NetworkManager
nmcli device status
```

2. Install backend dependencies:
```bash
# Ensure NetworkManager is installed
sudo pacman -S networkmanager
```

3. Configure network command:
```toml
[settings]
wifi_more_cmd = "nm-connection-editor"
```

---

## Audio Issues

### Volume Control Not Working

**Symptoms:** Volume slider doesn't change system volume

**Solutions:**

1. Check PulseAudio/PipeWire:
```bash
# For PulseAudio
pactl list sinks

# For PipeWire
wpctl status
```

2. Verify audio server:
```bash
# Check what's running
ps aux | grep -E 'pulseaudio|pipewire'
```

3. Install audio tools:
```bash
sudo pacman -S pulseaudio pulseaudio-alsa pavucontrol
# OR
sudo pacman -S pipewire pipewire-pulse pavucontrol
```

---

## Logging and Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug hydebar 2>&1 | tee hydebar.log
```

### Module-Specific Logging

```bash
RUST_LOG=hydebar_core::modules::workspaces=trace hydebar
```

### Check System Logs

```bash
journalctl --user -u hydebar -f
```

---

## Getting Help

If none of these solutions work:

1. **Search existing issues:** [GitHub Issues](https://github.com/RAprogramm/hydebar/issues)

2. **Create a bug report** with:
   - hydebar version: `hydebar --version`
   - System info: `uname -a`
   - Hyprland version: `hyprctl version`
   - Config file (sanitized)
   - Debug logs

3. **Ask in discussions:** [GitHub Discussions](https://github.com/RAprogramm/hydebar/discussions)

---

## Common Error Messages

### "Failed to connect to Hyprland socket"

**Solution:** Verify Hyprland is running and `$HYPRLAND_INSTANCE_SIGNATURE` is set:
```bash
echo $HYPRLAND_INSTANCE_SIGNATURE
```

### "Could not load config"

**Solution:** Check TOML syntax and file permissions:
```bash
chmod 644 ~/.config/hydebar/config.toml
```

### "Failed to create layer surface"

**Solution:** Verify Wayland compositor supports layer-shell protocol.

---

## Performance Tips

1. **Disable animations on low-end hardware:**
```toml
[appearance.animations]
enabled = false
```

2. **Reduce module count:**
```toml
[modules]
right = ["Clock"]  # Minimal
```

3. **Use lightweight themes:**
```toml
appearance = "nord"  # Simpler colors
```

---

**Still having issues?** Open an issue with full details: [Report Bug](https://github.com/RAprogramm/hydebar/issues/new)
