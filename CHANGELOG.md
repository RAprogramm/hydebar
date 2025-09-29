# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.6.5] - 2025-09-30

### Changed

- Split the system information module into dedicated `data`, `runtime`, and
  `view` components while keeping `system_info.rs` as a façade for
  registration and type exports.
- Centralised the polling task management inside the new runtime helper,
  simplifying module orchestration and test coverage.

### Added

- Unit tests for the system information data sampler and indicator builders
  covering sampling invariants and indicator selection edge cases.

## [0.6.4] - 2025-09-29

### Changed

- Extracted the outputs state management into dedicated `state`, `wayland`,
  and `config` helpers, keeping `outputs.rs` as a façade while improving
  testability and separation of concerns for layer-surface bookkeeping and
  configuration filtering.

### Added

- Unit coverage validating menu toggling and synchronisation behaviours of the
  outputs collection after the refactor.

## [0.6.3] - 2025-09-28

### Changed

- Extract the PipeWire runtime and webcam watchers into dedicated modules, wiring
  the privacy service through injectable traits for improved testability.
- Reuse a shared privacy event publisher abstraction across the service and new
  components while keeping data/state structures in the core module.

### Added

- Unit tests covering privacy updates from both PipeWire node events and
  inotify-driven webcam notifications.


## [0.6.3] - 2025-09-28

### Changed

- Extracted the MPRIS service into dedicated `data`, `ipc`, and `commands`
  modules, leaving the top-level orchestrator to re-export the data types and
  delegate to the new helpers while keeping the service wiring intact.
- Reworked the MPRIS command execution path to route through a proxy executor
  trait, reducing coupling to the service state and centralising error
  translation.

### Added

- Targeted unit coverage for the IPC helpers and command utilities alongside
  documentation examples for the newly public data types to guard the module
  boundaries.

### Changed

- Extracted configuration appearance, module layout, validation, and serde helper
  logic into dedicated submodules, leaving `config.rs` as the facade while
  preserving existing APIs.

### Added

- Unit tests covering appearance defaults, serde helpers, module layout
  deserialization, and configuration validation to guard the new structure.
- Reorganized the Hyprland client adapter into focused `config`, `sync_ops`,
  and `listeners` modules, keeping the facade slim while re-exporting the
  public surface.
- Centralized retry and backoff utilities for synchronous requests and event
  listeners to reuse.

### Added

- Unit tests covering the new retry delay helpers and listener backoff guard
  paths.

## [0.6.2] - 2025-09-28

### Changed

- Split the settings module into focused `state`, `commands`, `view`, and
  `event_forwarders` submodules, turning the top-level orchestrator into a thin
  re-export layer while preserving existing behaviour.

### Added

- Unit tests covering settings command spawning fallbacks, view builders, and
  event forwarders to guard the new module boundaries.

## [0.6.1] - 2025-09-27

### Changed

- Run custom module listeners on the shared runtime handle, caching module
  event senders from `ModuleContext` and aborting previous tasks when
  re-registering definitions.
- Publish custom module updates through `ModuleEvent::Custom` conversions,
  replacing the iced channel bridge and surfacing bus failures as `ModuleError`
  values.

### Added

- Unit tests covering error propagation from the custom module listener and
  validating that runtime-spawned tasks shut down cleanly on configuration
  changes.

## [0.6.0] - 2025-09-27

### Changed

- Move the media player module onto runtime-spawned listeners driven by the
  shared `ModuleContext`, caching the typed module sender and executing MPRIS
  commands on the runtime while forwarding results through the event bus.
- Expose asynchronous helpers from the MPRIS service that surface command
  failures as `ModuleError` values, aligning service interactions with the
  runtime-driven pattern.

### Added

- Regression tests covering media player command feedback and listener
  cancellation to ensure command results reach the UI and background tasks are
  aborted on re-registration.

## [0.5.4] - 2025-09-27

### Changed

- Move the system info module to a runtime-driven refresh loop powered by
  `ModuleContext`, publishing updates through typed module senders instead of
  iced subscriptions.

### Added

- Unit tests covering periodic refresh scheduling and task teardown to ensure
  polling loops honour cancellation on re-registration.

## [0.5.3] - 2025-09-27

### Changed

- Move the privacy module to runtime-spawned listeners driven by `ModuleContext`,
  replacing the iced subscription bridge and publishing `PrivacyMessage` events
  through the module event bus with typed senders.
- Expose a reusable privacy event publisher trait so `PrivacyService::start_listening`
  can be invoked directly by modules while propagating listener failures as
  structured errors.

### Added

- Unit tests covering privacy listener error propagation and task cancellation to
  guard the new runtime-driven flow.

## [0.5.2] - 2025-09-27

### Changed

- Move the tray module onto runtime-spawned listeners using typed module event
  senders, removing the iced subscription bridge and wiring command dispatch
  through the shared runtime.  
  (Refactors command execution to publish feedback via the module event bus.)

### Added

- Regression tests ensuring tray listener tasks are aborted on re-registration
  and that menu commands surface updates through the event bus.

## [0.5.1] - 2025-09-27

### Added

- Add option to remove the airplane button
- Add window title configuration
- Add modes to window title module.
- Add a optional command line parameter (`config-path`) to specify
  the path to the configuration file
- Add `scale_factor` configuration to change the scaling factor of the status bar
- Add custom commands for power menu actions
- Add battery module with configurable power-profile indicator and fallback view

### Changed

- Route the event bus sender into the GUI application so `App::new` provisions the
  shared `ModuleContext`, registers each module with its registration data, and
  keeps a runtime handle for modules to publish redraws without direct iced
  dependencies.
- Convert module subscriptions for clock, battery, keyboard layout/submap, window
  title, and workspaces into background tasks registered through typed
  `ModuleEventSender`s, eliminating direct iced subscriptions and aligning with
  the new module registration API.

## [0.4.0] - 2025-09-30

### Changed

- Replace module subscription configuration with a registration hook that receives
  `ModuleContext`, allowing modules to cache typed senders and initialise state
  before exposing subscriptions.
- Persist registration data for clock, updates, workspaces, and custom modules so
  subscriptions no longer require borrowed configuration.
- Wire the GUI to construct a shared `ModuleContext`, register modules on startup
  and configuration reloads, and batch module subscriptions with the existing
  application subscriptions.

## [0.3.6] - 2025-09-29

### Added

- Provide a shared `ModuleContext` with typed module event senders and redraw helpers for modules.

## [0.3.5] - 2025-09-28

### Changed

- Replaced per-module iced subscriptions with a micro-ticker that drains the
  shared event bus, batching redraws and popup toggles into 16–33 ms frames.
- Wired the GUI entrypoint to provision the bounded event bus so future module
  senders can publish without bespoke iced channels.

## [0.3.4] - 2025-09-27

### Added

- Introduced a bounded UI event bus with redraw/popup coalescing to reduce redundant work per frame.

## [0.3.3] - 2025-09-26

### Changed

- Guard configuration reloads behind a stateful manager that keeps the last valid settings and computes module-level impact before updates.
- Emit degradation events to the GUI instead of reverting to defaults when config files are removed or invalid.

### Added

- Added validation for custom module definitions and layout references during config reloads.
- Delivered partial reload support that refreshes outputs and custom modules only when their config changes.
- Extended configuration watcher tests to cover valid reloads, invalid TOML, and file removal without panics.

## [0.3.2] - 2025-09-26

### Changed

- Return typed errors from the configuration loader and application entrypoint to avoid process aborts.
- Handle channel backpressure gracefully across runtime modules, logging and skipping events instead of panicking.

### Fixed

- Added regression tests covering configuration read failures and channel send errors to ensure the application remains stable.

## [0.3.1] - 2025-09-26

### Added

- Introduced a Hyprland port abstraction with structured event types, keyboard state snapshots, and typed errors for adapters.
- Added a `HyprlandClient` adapter built on `hyprland-rs` with timeouts, retries, and mockable tests.

### Changed

- Core modules now obtain Hyprland access through injected ports, and the GUI wires the new client implementation for runtime use.

## [0.3.0] - 2025-02-15

### Changed

- Reorganized the project into a Cargo workspace with dedicated proto, core, GUI, and application crates while updating configuration watching to operate through the shared APIs.

## [0.2.4] - 2025-09-26

### Fixed

- Restore the NetworkManager event subscription lifetime bounds and stream setup so
  the project builds on recent compilers and `zbus` versions.
- Update the PipeWire integration to the `pipewire` 0.9 runtime API, keeping the
  privacy service compatible with the latest dependencies.

### Changed

- Replace `mod.rs` hierarchies with flat module files and adjust public module
  exports to match the new structure.
- Migrate error handling from `thiserror` to `masterror`, updating existing
  error types and removing the dependency.
- Refine the custom module listener channel handling to use a lightweight
  `SendQueueError` helper and propagate parse errors with structured context.
- Update the launcher utilities to expose an async command runner returning
  captured output and reuse it for power actions.

### Added

- Expand unit tests for the launcher helper and the custom module listener to
  cover channel-closure and command-result scenarios.

## [0.2.3] - 2025-09-26

### Changed

- Replace `unwrap`/`expect` usage in the custom module listener with structured error
  handling and graceful shutdown semantics.
- Surface command failures to the UI via `ServiceEvent::Error` so custom modules can
  react to listener issues.
- Switch stdout processing to `next_line().await?` with explicit channel-closure
  handling to avoid panics.

### Added

- Unit test covering early process termination and closed channel scenarios for the
  custom module runtime.

### Changed

- Move "truncate_title_after_length" to the window_title configuration

### Fixed

- Bluetooth: use alias instead of name for device name
- Airplane button fail when the `rfkill` returns an error or is not present

## [0.2.2] - 2025-09-27

### Changed

- Launcher commands now execute via Tokio, logging failures instead of panicking and exposing a reusable async API for command status and output retrieval.

### Fixed

- Fire-and-forget power actions no longer abort the process on spawn failures or non-zero exit codes.

## [0.2.1] - 2025-09-26

### Changed

- Privacy service now exposes structured `PrivacyError` values and gracefully falls back when the webcam device is absent.

### Fixed

- Report PipeWire and inotify listener initialisation failures without panicking, allowing the UI to react to privacy service errors.

## [0.2.0] - 2025-09-26

### Added

- Introduced a dedicated `IdleInhibitorError` type for the idle inhibitor service.

### Changed

- `IdleInhibitorManager::new` now returns `Result<Self, IdleInhibitorError>` and surfaces initialization failures explicitly.
- Idle inhibitor initialization tests cover both missing and complete Wayland global scenarios.

## [0.1.3] - 2025-09-26

### Fixed

- Restore the configuration file watcher when the inotify stream closes and avoid tight loops on stream shutdown.

## [0.1.1] - 2025-05-23

### Added

- Curated collection of Codex task prompts for HyDEbar modernization in `docs/celi.md`.

### Changed

- README now highlights the goals/prompts document for contributors.

## [0.5.0] - 2025-05-20

### WARNING BREAKING CHANGES

The configuration switch from `yaml` to `toml` format.
The configuration file must be updated to adapt to the new format.
The `camelCase` format has been removed in favor of `snake_case`,
which better aligns with the `toml` syntax.

You could use an online tool like: <https://transform.tools/yaml-to-toml>
but remember to change the `camelCase` to `snake_case` format.

Now the configuration file is located in `~/.config/hydebar/config.toml`

### Added

- Add font name configuration
- Add main bar solid and gradient style
- Add main bar opacity settings
- Add menu opacity and backdrop settings
- Add experimental IWD support as fallback for the network module
- Handle system with multiple battery
- Allow to specify custom labels for keyboard layouts
- Allow to always show a specific number of workspaces,
  whether they have windows or not
- Added custom modules and their ability to receive events from external commands

### Changed

- Change configuration file format
- Enhance the system info module adding network and disk usage
- Simplify style of "expand" button on wifi/bluetooth buttons
- Allow to specify custom labels for keyboard layouts
- Removed background on power info in menu

### Fixed

- Fix missing tray icons
- Fix hide vpn button when no vpn is configured

### Thanks

- @JumpIn-Git for fixing nix flake instruction
- @ineu for the custom labels for keyboard layouts, the `max_workspaces` settings and for fixing some bugs
- @rahatarmanahmed for the new settings button style, the new battery info style and for fixing some bugs
- Huge thanks to @clotodex for the `iwd` network support and the custom modules
- @tqwewe for fixing the audio sink menu position with bottom bar

## [0.4.1] - 2025-03-16

### Added

- Media player module

### Fixed

- Fix bluetooth service in NixOS systems
- Fix brightness wrong value in some situations
- Fix settings menu not resetting it's state when closed
- Fix bluetooth service crash during listing of devices without battery info
- Fix centerbox children positioning

### Thanks

- Huge thanks to @mazei513 for the implementation of the media player module

## [0.4.0] - 2025-01-19

A big update with new features and new configurations!

The configuration file must be updated to adapt to the new stuff.

### Added

- Multi monitor support
- Tray module
- Dynamic modules system configuration
- New workspace module configuration

### Changed

- Update to pop-os Iced 14.0-dev
- Dynamic menu positioning

### Thanks

- @fiersik for participating in the discussions
- @ReshetnikovPavel for the proposal of the new dynamic modules system configuration

## [0.3.1] - 2024-12-13

### Fixed

- Fix upower service startup fail in case of missing `org.freedesktop.UPower.PowerProfiles` dbus interface

## [0.3.0] - 2024-11-26

A small release with some new Hyprland related modules

Thanks @fiersik for the new modules and contributions to the project

### Added

- Hyprland Keyboard Layout module
- Hyprland Keyboard Submap module

### Changed

- Change main surface layer from Top to Bottom

## [0.2.0] - 2024-11-08

### Added

- Support for special workspaces

### Fixed

- hydebar crash when the title module try to split a multi-byte character
- Removed fixed monitor name in the workspace module
- Fix privacy webcam usage check during initialization
- Fix microphone selection
- Fix sink and source slider toggle button state
- Fix brightness initial value

### Thanks

- @fiersik for all the feedback
- @leftas for the PRs to fix the special workspace crash and the title module

## [0.1.5] - 2024-11-04

### Added

- Added a clipboard button

### Fixed

- Fix workspace indicator foreground color selection

### Changed

- Nerd fonts are now included in the binary
- Workspace indicator now has an hover state

### Thanks

- @fiersik for the clipboard button and the ALT Linux package

## [0.1.4] - 2024-10-23

### Fixed

- bluetooth quick toggle button no longer appear when no bluetooth device is available
- rfkill absence doesn't cause an error during network service initialization
- rfkill is launched using absolute path to avoid issues with $PATH
- webcam absence doesn't cause an error during privacy service initialization

### Changed

- added more logging to the services in case of errors

## [0.1.3] - 2024-10-22

### Fixed

- resolved problem with `cargo vendor` command

## [0.1.2] - 2024-10-17

### Added

- Privacy module: webcam usage indicator

### Changed

- Reduced clock refresh rate to 5 sec
- Increased update check frequency to 3600 sec

### Removed

- Privacy module: removed privacy sub-menu

### Fixed

- Improve wifi indicator

## [0.1.1] - 2024-10-03

### Fixed

- re-added vpn toggle functionality that was removed during the services refactor

## [0.1.0] - 2024-09-30

### Added

- First release
- Configuration system
- Lancher button
- OS Updates indicator
- Hyprland Active Window
- Hyprland Workspaces
- System Information (CPU, RAM, Temperature)
- Date time
- Settings panel
  - Power menu
  - Battery information
  - Audio sources and sinks
  - Screen brightness
  - Network stuff
  - VPN
  - Bluetooth
  - Power profiles
  - Idle inhibitor
  - Airplane mode
