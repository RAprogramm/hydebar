# HydeBar Architecture

**Professional, Clean, Event-Driven Architecture for Hyprland**

## Philosophy

> Modules provide DATA and LOGIC, not UI.
> GUI layer renders based on data.
> Communication through Event Bus.

## Layer Structure

```
┌──────────────────────────────────────┐
│         hydebar-proto                │
│  - Config types                      │
│  - Protocol definitions              │
│  - Shared data structures            │
└──────────────┬───────────────────────┘
               │
┌──────────────▼───────────────────────┐
│         hydebar-core                 │
│  ┌────────────────────────────────┐  │
│  │ Modules (Business Logic ONLY)  │  │
│  │  - battery: BatteryData        │  │
│  │  - clock: ClockData            │  │
│  │  - workspaces: WorkspaceData   │  │
│  │  ...                           │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │ Event Bus                      │  │
│  │  - Module events               │  │
│  │  - State changes               │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │ Services                       │  │
│  │  - DBus, IPC, Hyprland socket │  │
│  └────────────────────────────────┘  │
└──────────────┬───────────────────────┘
               │ Events only
┌──────────────▼───────────────────────┐
│         hydebar-gui                  │
│  ┌────────────────────────────────┐  │
│  │ View Layer                     │  │
│  │  - Renders modules to Elements │  │
│  │  - Styling, theming            │  │
│  │  - User interactions           │  │
│  └────────────────────────────────┘  │
│  ┌────────────────────────────────┐  │
│  │ App State                      │  │
│  │  - Subscribes to core events   │  │
│  │  - Handles user actions        │  │
│  └────────────────────────────────┘  │
└──────────────┬───────────────────────┘
               │
┌──────────────▼───────────────────────┐
│         hydebar-app                  │
│  - Main entry point                  │
│  - Wiring everything together        │
└──────────────────────────────────────┘
```

## Module Design Pattern

### Core Module (NO GUI!)

```rust
// hydebar-core/src/modules/battery.rs

/// Module data - pure state, no UI
#[derive(Debug, Clone)]
pub struct BatteryData {
    pub capacity: u8,
    pub charging: bool,
    pub icon: BatteryIcon,
    pub time_remaining: Option<Duration>,
    pub power_profile: PowerProfile,
}

/// Module events
#[derive(Debug, Clone)]
pub enum BatteryEvent {
    StatusChanged(BatteryData),
    ProfileChanged(PowerProfile),
    LowBattery(u8),
}

/// Module - business logic only
pub struct Battery {
    data: BatteryData,
    sender: EventSender<BatteryEvent>,
}

impl Battery {
    pub fn data(&self) -> &BatteryData {
        &self.data
    }

    pub fn set_power_profile(&mut self, profile: PowerProfile) {
        // Logic here
        self.sender.send(BatteryEvent::ProfileChanged(profile));
    }
}
```

### GUI View (iced rendering)

```rust
// hydebar-gui/src/views/battery.rs

pub fn render_battery(data: &BatteryData) -> Element<Message> {
    row![
        icon(data.icon),
        text(format!("{}%", data.capacity)),
        if data.charging {
            icon(Icons::Lightning)
        }
    ]
    .spacing(4)
    .into()
}

pub fn render_battery_menu(data: &BatteryData) -> Element<Message> {
    column![
        text(format!("Battery: {}%", data.capacity)),
        text(format!("Time: {:?}", data.time_remaining)),
        // Power profile buttons
        power_profile_selector(data.power_profile)
    ]
}
```

## Event Flow

```
User clicks → GUI sends Message → App updates Module →
Module publishes Event → Event Bus → GUI subscribes → Re-render
```

Example:
```
1. User clicks "Change Power Profile"
2. GUI: Message::Battery(BatteryAction::SetProfile(Performance))
3. App: battery.set_power_profile(Performance)
4. Module: sender.send(BatteryEvent::ProfileChanged(Performance))
5. GUI subscription receives event → update() → re-render
```

## Benefits

### ✅ Clean Separation
- Core = logic, no dependencies on GUI
- GUI = rendering, no business logic
- Easy to test each layer

### ✅ No Circular Dependencies
- Core never imports GUI types
- GUI imports Core data types only
- Uni-directional data flow

### ✅ Modularity
- Easy to add new modules
- Modules are independent
- Can reuse core in different GUIs (CLI, web, etc.)

### ✅ Performance
- Event-driven updates (only what changed)
- iced GPU acceleration
- Efficient rendering

### ✅ Maintainability
- Clear responsibility boundaries
- Easy to debug
- Professional codebase

## Migration Steps

1. **Define data structures** in core modules
2. **Remove GUI dependencies** from core
3. **Create view functions** in GUI layer
4. **Wire event bus** properly
5. **Test each module** independently

## Example: Complete Battery Module

See `docs/examples/battery-module.md` for full implementation example.
