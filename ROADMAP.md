# hydebar Roadmap

**Goal:** Build the **fastest** and **most beautiful** Wayland panel for Hyprland.

**Vision:** Lighter and faster than Waybar, richer and more polished than HyprPanel.

---

## 🎯 Core Principles

1. **⚡ Blazing Fast** - < 5MB RAM, < 1% CPU idle, < 50ms startup
2. **🎨 Beautiful** - Preset themes, smooth animations, modern UI
3. **🛠️ Easy to Configure** - GUI config panel, hot-reload, sensible defaults
4. **🔧 Extensible** - Custom modules, plugin system (future)
5. **100% Rust** - Memory safe, zero-cost abstractions

---

## 📊 Current State (v0.6.7)

### ✅ Implemented Features

**Core Modules:**
- Workspaces (Hyprland integration)
- Window title
- Clock
- System info (CPU, RAM, temp, disk, network speeds)
- Battery
- Network (WiFi, VPN, connections)
- Bluetooth
- Audio (volume, sink/source control)
- Brightness
- Media player (MPRIS)
- Tray
- Updates
- Privacy indicators (camera/mic)
- Keyboard layout/submap
- Clipboard
- App launcher
- Power menu
- Custom modules

**Technical:**
- Multi-window support (multi-monitor)
- Wayland-native (layer-shell)
- Event-driven architecture
- Full test coverage (115+ tests)
- Config hot-reload

### 🔧 Current Limitations

- No preset themes (manual color config)
- Basic animations only
- TOML-only configuration (no GUI)
- No notification center
- No screenshot/recording integration
- Missing weather/calendar widgets

---

## 🗓️ Development Phases

## Phase 1: Visual Polish 🎨 (v0.7.0)

**Goal:** Match HyprPanel's visual appeal

**Duration:** 2-3 weeks

### Issues
- #61 🎨 **Preset color themes** (5-7h) - HIGH PRIORITY
  - Catppuccin, Dracula, Nord, Gruvbox, Tokyo Night
  - Simple config: `theme = "catppuccin-mocha"`
  - Instant visual impact

- #62 ✨ **Smooth animations** (8-10h) - MEDIUM PRIORITY
  - Menu fade in/out
  - Hover transitions
  - Workspace switch animations
  - Configurable duration

### Deliverables
- 5 beautiful preset themes
- Smooth, polished animations
- Theme showcase screenshots

---

## Phase 2: Performance Optimization ⚡ (v0.8.0)

**Goal:** Become the fastest Wayland panel

**Duration:** 2-3 weeks

### Issues
- #68 ⚡ **Performance optimization** (20-30h) - CRITICAL
  - Baseline measurements vs Waybar
  - Memory profiling and optimization
  - CPU usage optimization
  - Startup time optimization
  - Rendering performance
  - Benchmarks in CI

### Targets
- **RAM:** < 5MB idle, < 20MB with all modules
- **CPU:** < 1% idle, < 5% active
- **Startup:** < 50ms to first paint
- **FPS:** Solid 60 FPS

### Deliverables
- Performance benchmarks
- Comparison vs Waybar (documented)
- Automated performance regression tests

---

## Phase 3: Enhanced Features 🚀 (v0.9.0)

**Goal:** Feature parity with HyprPanel + unique features

**Duration:** 4-6 weeks

### Issues
- #65 🔔 **Notification center** (15-20h) - HIGH PRIORITY
  - History (last 50 notifications)
  - Do Not Disturb mode
  - Per-app settings
  - Notification actions

- #69 🎯 **Module improvements** (12-15h) - MEDIUM PRIORITY
  - Inline volume/brightness sliders
  - Per-app volume control
  - WiFi strength meter
  - Network speed graphs
  - Bluetooth device battery

- #64 📸 **Screenshot/recording** (6-8h) - MEDIUM PRIORITY
  - Grim/slurp integration
  - wf-recorder integration
  - Quick actions menu

### Deliverables
- Full-featured notification center
- Enhanced module controls
- Screenshot/recording tools

---

## Phase 4: User Experience 🎯 (v1.0.0)

**Goal:** Production-ready, best-in-class UX

**Duration:** 3-4 weeks

### Issues
- #63 ⚙️ **GUI configuration panel** (20-25h) - MEDIUM PRIORITY
  - Theme selector
  - Module enable/disable
  - Drag-and-drop module ordering
  - Color picker
  - Live preview

- #70 📚 **Comprehensive documentation** (10-15h) - HIGH PRIORITY
  - Getting started guide
  - Configuration reference
  - Theme guide
  - Video demos
  - Comparison tables
  - Website/docs site

### Deliverables
- In-app configuration GUI
- Professional documentation
- Demo videos and screenshots
- v1.0.0 stable release

---

## Phase 5: Extra Features 🌟 (v1.1.0+)

**Goal:** Nice-to-have features

**Duration:** Ongoing

### Issues
- #66 🌤️ **Weather widget** (10-12h) - LOW PRIORITY
- #67 📅 **Calendar widget** (8-10h) - LOW PRIORITY

### Future Ideas
- Plugin system (Lua/WASM)
- Multiple panel support (top + bottom)
- Vertical panel mode
- Panel auto-hide
- Gesture controls
- Mobile companion app
- Cloud sync for config

---

## 🎯 Success Metrics

### Performance (vs Waybar)
- ✅ **Faster startup:** < 50ms (Waybar: ~100ms)
- ✅ **Lower memory:** < 5MB (Waybar: ~10MB)
- ✅ **Lower CPU:** < 1% idle (Waybar: ~2%)

### Features (vs HyprPanel)
- ✅ All core features from HyprPanel
- ✅ Better performance (Rust vs TypeScript/GTK)
- ✅ More themes out-of-box
- ✅ Better Wayland integration

### Adoption
- 🎯 100+ GitHub stars
- 🎯 10+ contributors
- 🎯 Featured in Hyprland showcase
- 🎯 AUR package
- 🎯 Mentioned in r/hyprland

---

## 📋 Prioritization Framework

**Priority Levels:**

1. **CRITICAL** - Blocks release, major differentiator
2. **HIGH** - Important for UX, high impact
3. **MEDIUM** - Nice to have, improves experience
4. **LOW** - Future enhancement

**Current Priorities (for v0.7.0):**
1. #61 Preset themes (HIGH)
2. #68 Performance optimization (CRITICAL)
3. #65 Notification center (HIGH)
4. #70 Documentation (HIGH)

---

## 🤝 Contributing

Want to help? Check out:
- Issues labeled `good first issue`
- Issues with detailed implementation plans
- Our [Contributing Guide](CONTRIBUTING.md)

**High-impact, beginner-friendly:**
- #61 Preset color themes (well-defined, isolated)
- Individual theme implementations
- Documentation improvements
- Testing and bug reports

---

## 📈 Timeline Overview

```
v0.6.7 (Current) ──> v0.7.0 ──> v0.8.0 ──> v0.9.0 ──> v1.0.0
                     3 weeks   3 weeks    6 weeks    4 weeks

                     Themes    Perf       Features   UX/Docs
```

**Total to v1.0.0:** ~16 weeks (4 months)

**Target v1.0.0 release:** Q2 2025

---

## 📞 Feedback

Have ideas? Open an issue or discussion!

- 🐛 Bugs: [Issues](https://github.com/RAprogramm/hydebar/issues)
- 💡 Feature requests: [Discussions](https://github.com/RAprogramm/hydebar/discussions)
- 💬 Chat: [Matrix/Discord] (TBD)

---

**Last updated:** 2025-10-08

**Status:** Active development 🚧
