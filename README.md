# hydebar

A ready to go Wayland status bar for Hyprland.

## Development prompts and goals

The project vision, high-level goals, and ready-to-use task prompts for
HyDEbar evolution live in [docs/celi.md](docs/celi.md). The document contains
scoped tasks, explicit definitions of done, and command checklists tailored for
HyDE Project contributors who continue the modernization of this Hyprland panel.

Feel free to fork this project and customize it for your needs or just open an
issue to request a particular feature.

> If you have graphical issues like missing transparency or graphical artifact you could launch hydebar with WGPU_BACKEND=gl. This env var forces wgpu to use OpenGL instead of Vulkan

### Does it only work on Hyprland?

While it's currently tailored for Hyprland, it could work with other compositors.

However, it currently relies on [hyprland-rs](https://github.com/hyprland-community/hyprland-rs)
to gather information about the active window and workspaces. I haven't implemented any
feature flags to disable these functionalities or alternative methods to obtain this data.

## Install

[![Packaging status](https://repology.org/badge/vertical-allrepos/hydebar.svg)](https://repology.org/project/hydebar/versions)

See the instruction on the [website](https://raprogramm.github.io/hydebar/docs/installation) for more details.

### Arch Linux

You can get the official Arch Linux package from the AUR:

#### Tagged release

```
paru/yay -S hydebar
```

#### Main branch

```
paru/yay -S hydebar-git
```

### ALT Linux

```
su -
apt-get install hydebar
```

### Nix

To install hydebar using the nix package be sure to enable flakes and then run

#### Tagged release

```
nix profile install github:RAprogramm/hydebar?ref=0.3.1
```

#### Main branch

```
nix profile install github:RAprogramm/hydebar
```

### NixOS/Home-Manager

To use this flake do

```nix
flake.nix
inputs = {
  # ... other inputs
  hydebar.url = "github:RAprogramm/hydebar";
  # ... other inputs
};
outputs = {...} @ inputs: {<outputs>}; # Make sure to pass inputs to your specialArgs!
```

```nix
configuration.nix
{ pkgs, inputs, ... }:

{
  environment.systemPackages = [inputs.hydebar.defaultPackage.${pkgs.system}];
  # or home.packages = ...
}
```

This will build hydebar from source, but you can also use `pkgs.hydebar`
from nixpkgs which is cached.

## Features

- App Launcher button
- Сlipboard button
- OS Updates indicator
- Hyprland Active Window
- Hyprland Workspaces
- System Information (CPU, RAM, Temperature)
- Hyprland Keyboard Layout
- Hyprland Keyboard Submap
- Tray
- Date time
- Battery indicator
- Privacy (check microphone, camera and screenshare usage)
- Media Player
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
- Custom Modules
  - Simple (execute command on click)
  - Advanced (update UI with command output)

## Configuration

See more about all the possible configuration on the
[website](https://raprogramm.github.io/hydebar/docs/configuration)

> **Warning**
>
> This instruction are deprecated and will be removed in the future. See the
> [website](https://raprogramm.github.io/hydebar/docs/configuration) for the latest configuration instruction.

> The following are the configuration for the `main` branch and could contain breaking changes.
> For the tagged release you can find the configuration instruction in the corresponding README file.
> eg: <https://github.com/RAprogramm/hydebar/blob/0.1.0/README.md>

The configuration file uses the toml file format and is named `~/.config/hydebar/config.toml`

You can use a different file by passing the `--config-path` flag to hydebar, for example:

```bash
hydebar --config-path /path/to/config.toml
```

```toml
# hydebar log level filter, possible values "debug" | "info" | "warn" | "error". Needs reload
log_level = "warn"
# Possible status bar outputs, values could be: All, Active, or a list of outputs
# All: the status bar will be displayed on all the available outputs, example: outputs = "All"
# active: the status bar will be displayed on the active output, example: outputs = "Active"
# list of outputs: the status bar will be displayed on the outputs listed here, example: outputs = { Targets = ["DP-1", "eDP-1"] }
# if the outputs is not available the bar will be displayed in the active output
outputs = "All"
# Bar position, possible values Top | Bottom.
position = "Top"
# App launcher command, it will be used to open the launcher,
# without a value the related button will not appear
# optional, default None
app_launcher_cmd = "~/.config/rofi/launcher.sh"
# Clipboard command, it will be used to open the clipboard menu,
# without a value the related button will not appear
# optional, default None
clipboard_cmd = "cliphist-rofi-img | wl-copy"

# Declare which modules should be used and in which position in the status bar.
# This is the list of all possible modules
#  - AppLauncher
#  - Updates
#  - Clipboard
#  - Workspaces
#  - WindowTitle
#  - SystemInfo
#  - KeyboardLayout
#  - KeyboardSubmap
#  - Tray
#  - Clock
#  - Privacy
#  - MediaPlayer
#  - Settings
# optional, the following is the default configuration
[modules]
# The modules that will be displayed on the left side of the status bar
left = [ "Workspaces" ]
# The modules that will be displayed in the center of the status bar
center = [ "WindowTitle" ]
# The modules that will be displayed on the right side of the status bar
# The nested modules array will form a group sharing the same element in the status bar
# You can also use custom modules to extend the normal set of options, see configuration below
right = [ "SystemInfo", [ "Clock", "Privacy", "Settings" ], "CustomNotifications" ]

# Update module configuration.
# Without a value the related button will not appear.
# optional, default None
[updates]
# The check command will be used to retrieve the update list.
# It should return something like `package_name version_from -> version_to\n`
check_cmd = "checkupdates; paru -Qua"
# The update command is used to init the OS update process
update_cmd = 'alacritty -e bash -c "paru; echo Done - Press enter to exit; read" &'

# Workspaces module configuration, optional
[workspaces]
# The visibility mode of the workspaces, possible values are:
# All: all the workspaces will be displayed
# MonitorSpecific: only the workspaces of the related monitor will be displayed
# optional, default All
visibility_mode = "All"

# Enable filling with empty workspaces
# For example:
# With this flag set to true if there are only 2 workspaces,
# the workspace 1 and the workspace 4, the module will show also
# two more workspaces, the workspace 2 and the workspace 3
# optional, default false
enable_workspace_filling = false

# If you want to see more workspaces prefilled, set the number here:
# max_workspaces = 6
# In addition to the 4 workspaces described above it will also show workspaces 5 and 6
# Only works with `enable_workspace_filling = true`

# WindowTitle module configuration, optional
[window_title]
# The information to get from your active window.
# Possible modes are:
# - Title
# - Class
# optional, default Title
mode = "Title"

# Maximum number of chars that can be present in the window title
# after that the title will be truncated
# optional, default 150
truncate_title_after_length = 150


# keyboardLayout module configuration
# optional
# Maps layout names to arbitrary labels, which can be any text, including unicode symbols as shown below
# If using Hyprland the names can be found in `hyprctl devices | grep "active keymap"`
[keyboard_layout.labels]
"English (US)" = "🇺🇸"
"Russian" = "🇷🇺"

# The system module configuration
# optional
[system]
# System information shown in the status bar
# The possible values are:
#  - Cpu
#  - Memory
#  - MemorySwap
#  - Temperature
#  - { disk = "path" }
#  - IpAddress
#  - DownloadSpeed
#  - UploadSpeed
# optional, the following is the default configuration
# If for example you want to dispay the usage of the root and home partition
# you can use the following configuration
# systemInfo = [ { disk = "/" }, { disk = "/home" } ]
indicators = [ "Cpu", "Memory", "Temperature" ]

# CPU indicator thresholds
# optional
[system.cpu]
# cpu indicator warning level (default 60)
warn_threshold = 60
# cpu indicator alert level (default 80)
alert_threshold = 80

# Memory indicator thresholds
# optional
[system.memory]
# mem indicator warning level (default 70)
warn_threshold = 70
# mem indicator alert level (default 85)
alert_threshold = 85

# Memory swap indicator thresholds
# optional
[system.temperature]
# temperature indicator warning level (default 60)
warn_threshold = 60
# temperature indicator alert level (default 80)
alert_threshold = 80

# Disk indicator thresholds
# optional
[system.disk]
# disk indicator warning level (default 80)
warn_threshold = 80
# disk indicator alert level (default 90)
alert_threshold = 90

# Clock module configuration
[clock]
# clock format see: https://docs.rs/chrono/latest/chrono/format/strftime/index.html
format = "%a %d %b %R"

# Media player module configuration
[media_player]
# optional, default 100
max_title_length = 100

# Custom modules configuration (you can have multiple)
[[CustomModule]]
# The name will link the module in your left/center/right definition
name = "CustomNotifications"
# The default icon for this custom module
icon = ""
# The command that will be executed on click
command = "swaync-client -t -sw"
# You can optionally configure your custom module to update the UI using another command
# The output right now follows the waybar json-style output, using the `alt` and `text` field
# E.g. `{"text": "3", "alt": "notification"}`
listen_cmd = "swaync-client -swb"
# You can define behavior for the `text` and `alt` fields
# Any number of regex can be used to change the icon based on the alt field
icons.'dnd.*' = ""
# Another regex can optionally show a red "alert" dot on the icon
alert = ".*notification"

# Settings module configuration
[settings]
# command used for lock the system
# without a value the related button will not appear
# optional, default None
lock_cmd = "hyprlock &"
# commands used to respectively shutdown, suspend, reboot and logout
# all optional, without values the defaults shown here will be used
shutdown_cmd = "shutdown now"
suspend_cmd = "systemctl suspend"
reboot_cmd = "systemctl reboot"
logout_cmd = "loginctl kill-user $(whoami)"
# command used to open the sinks audio settings
# without a value the related button will not appear
# optional default None
audio_sinks_more_cmd = "pavucontrol -t 3"
# command used to open the sources audio settings
# without a value the related button will not appear
# optional, default None
audio_sources_more_cmd = "pavucontrol -t 4"
# command used to open the network settings
# without a value the related button will not appear
# optional, default None
wifi_more_cmd = "nm-connection-editor"
# command used to open the VPN settings
# without a value the related button will not appear
# optional, default None
vpn_more_cmd = "nm-connection-editor"
# command used to open the Bluetooth settings
# without a value the related button will not appear
# optional, default None
bluetooth_more_cmd = "blueman-manager"
# option to remove the airtplane button
# optional, default false
remove_airplane_btn = true
# option to remove the idle inhibitor button
# optional, default false
remove_idle_btn = true

# Appearance config
# Each color could be a simple hex color like #228800 or an
# object that define a base hex color and two optional variant of that color (a strong one and a weak one)
# and the text color that should be used with that base color
# example:
# [appearance.background_color]
# base = "#448877"
# strong = "#448888" # optional default autogenerated from base color
# weak = "#448855" # optional default autogenarated from base color
# text = "#ffffff" # optional default base text color
[appearance]
# optional, default iced.rs font
font_name = "Comic Sans MS"
# The style of the main bar, possible values are: Islands | Solid | Gradient
# optional, default Islands
style = "Islands"
# The opacity of the main bar, possible values are: 0.0 to 1.0
# optional, default 1.0
opacity = 0.7
# used as a base background color for header module button
background_color = "#1e1e2e"
# used as a accent color
primary_color = "#fab387"
# used for darker background color
secondary_color = "#11111b"
# used for success message or happy state
success_color = "#a6e3a1"
# used for danger message or danger state (the weak version is used for the warning state
danger_color = "#f38ba8"
# base default text color
text_color = "#f38ba8"
# this is a list of color that will be used in the workspace module (one color for each monitor)
workspace_colors = [ "#fab387", "#b4befe" ]
# this is a list of color that will be used in the workspace module
# for the special workspace (one color for each monitor)
# optional, default None
# without a value the workspaceColors list will be used
special_workspace_colors = [ "#a6e3a1", "#f38ba8" ]

# menu options
[appearance.menu]
# The opacity of the menu, possible values are: 0.0 to 1.0
# optional, default 1.0
opacity = 0.7
# The backdrop of the menu, possible values are: 0.0 to 1.0
# optional, default 0.0
backdrop = 0.3
```

## Some screenshots

I will try my best to keep these screenshots as updated as possible but some details
could be different

#### default style

<img src="https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/hydebar.png"></img>

#### solid style

<img src="https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/hydebar-solid.png"></img>

#### gradient style

<img src="https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/hydebar-gradient.png"></img>

#### opacity settings

<img src="https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/opacity.png"></img>

| ![](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/updates-panel.png) | ![](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/settings-panel.png)  |
| ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| ![](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/power-menu.png)    | ![](https://raw.githubusercontent.com/RAprogrammraprogramm/main/screenshots/sinks-selection.png) |
| ![](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/network-menu.png)  | ![](https://raw.githubusercontent.com/RAprogrammraprogramm/main/screenshots/bluetooth-menu.png)  |
| ![](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/vpn-menu.png)      | ![](https://raw.githubusercontent.com/RAprogramm/hydebar/main/screenshots/airplane-mode.png)   |
