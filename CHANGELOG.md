# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add option to remove the airplane button
- Add window title configuration
- Add modes to window title module.
- Add a optional command line parameter (`config-path`) to specify
  the path to the configuration file
- Add `scale_factor` configuration to change the scaling factor of the status bar
- Add custom commands for power menu actions
- Add battery module with configurable power-profile indicator and fallback view

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
