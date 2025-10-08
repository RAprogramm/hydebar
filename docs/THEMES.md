# Theme Showcase

hydebar includes 11 carefully crafted preset themes inspired by popular color schemes.

## Using Themes

Add to your `~/.config/hydebar/config.toml`:

```toml
appearance = "theme-name"
```

Changes apply instantly!

---

## Catppuccin Themes

### Catppuccin Mocha

```toml
appearance = "catppuccin-mocha"
```

**Style:** Dark purple with soft pastels
**Best For:** Night coding, low-light environments
**Colors:** Lavender (#cba6f7), Peach (#fab387), Pink (#f5c2e7)

### Catppuccin Macchiato

```toml
appearance = "catppuccin-macchiato"
```

**Style:** Dark blue with muted tones
**Best For:** Easy on the eyes, professional look
**Colors:** Blue (#8aadf4), Mauve (#c6a0f6), Flamingo (#f0c6c6)

### Catppuccin Frappe

```toml
appearance = "catppuccin-frappe"
```

**Style:** Medium dark with rich colors
**Best For:** Balanced contrast
**Colors:** Mauve (#ca9ee6), Blue (#8caaee), Pink (#f4b8e4)

### Catppuccin Latte

```toml
appearance = "catppuccin-latte"
```

**Style:** Light theme with pastel accents
**Best For:** Daytime use, bright environments
**Colors:** Lavender (#7287fd), Peach (#fe640b), Sky (#04a5e5)

---

## Dracula

```toml
appearance = "dracula"
```

**Style:** Dark with vibrant neon accents
**Best For:** High contrast, colorful aesthetic
**Colors:** Purple (#bd93f9), Pink (#ff79c6), Cyan (#8be9fd)

---

## Nord

```toml
appearance = "nord"
```

**Style:** Cool arctic blue palette
**Best For:** Calm, professional, low eye strain
**Colors:** Frost blue (#88c0d0), Aurora green (#a3be8c), Frost cyan (#8fbcbb)

---

## Gruvbox Themes

### Gruvbox Dark

```toml
appearance = "gruvbox-dark"
```

**Style:** Warm retro colors, earthy tones
**Best For:** Cozy coding sessions, vintage aesthetic
**Colors:** Orange (#fe8019), Yellow (#fabd2f), Green (#b8bb26)

### Gruvbox Light

```toml
appearance = "gruvbox-light"
```

**Style:** Light with warm earth tones
**Best For:** Bright environments, retro light theme
**Colors:** Red (#9d0006), Orange (#af3a03), Yellow (#79740e)

---

## Tokyo Night Themes

### Tokyo Night

```toml
appearance = "tokyo-night"
```

**Style:** Dark with neon accents, cyberpunk vibes
**Best For:** Modern aesthetic, high contrast
**Colors:** Purple (#bb9af7), Blue (#7aa2f7), Cyan (#7dcfff)

### Tokyo Night Storm

```toml
appearance = "tokyo-night-storm"
```

**Style:** Darker variant with muted neon
**Best For:** Reduced brightness, late night
**Colors:** Same palette as Tokyo Night, darker background

### Tokyo Night Light

```toml
appearance = "tokyo-night-light"
```

**Style:** Clean light theme with subtle accents
**Best For:** Daytime coding, bright rooms
**Colors:** Purple (#5a4a78), Blue (#34548a), Cyan (#0f4b6e)

---

## Customizing Themes

### Override Theme Colors

Start with a theme and tweak specific colors:

```toml
appearance = "catppuccin-mocha"

[appearance]
# Override just the primary color
primary_color = "#ff0000"
```

### Adjust Opacity

Make themes more or less transparent:

```toml
appearance = "nord"

[appearance]
opacity = 0.85  # More transparent

[appearance.menu]
opacity = 0.90
backdrop = 0.5  # Stronger backdrop blur
```

### Change Visual Style

Themes work with all styles:

```toml
appearance = "dracula"

[appearance]
style = "Islands"   # Default
# style = "Solid"   # No gaps between modules
# style = "Gradient" # Gradient backgrounds
```

---

## Creating Custom Themes

Don't see your favorite theme? Create your own!

```toml
[appearance]
style = "Islands"
opacity = 0.95

# Base colors
background_color = "#1a1b26"
primary_color = "#7aa2f7"
secondary_color = "#16161e"
success_color = "#9ece6a"
danger_color = "#f7768e"
text_color = "#c0caf5"

# Workspace colors (one per monitor)
workspace_colors = [
    "#7aa2f7",
    "#bb9af7",
    "#7dcfff"
]

# Optional: Special workspace colors
special_workspace_colors = ["#f7768e"]
```

### Advanced Color Options

Each color can be a simple hex or a full palette:

```toml
[appearance.primary_color]
base = "#7aa2f7"
strong = "#89b4fa"    # Hover state
weak = "#6c8ec0"      # Disabled state
text = "#1a1b26"      # Text on this background
```

---

## Theme Comparison

| Theme | Style | Contrast | Best For |
|-------|-------|----------|----------|
| Catppuccin Mocha | Dark Purple | Medium | Night use, soft colors |
| Catppuccin Latte | Light Pastel | Low | Daytime, easy on eyes |
| Dracula | Dark Neon | High | Vibrant, colorful |
| Nord | Cool Blue | Medium | Professional, calm |
| Gruvbox Dark | Warm Retro | Medium | Cozy, vintage |
| Gruvbox Light | Warm Light | Medium | Bright, retro |
| Tokyo Night | Dark Neon | High | Modern, cyberpunk |
| Tokyo Night Light | Clean Light | Low | Daytime, minimal |

---

## Tips

1. **Try multiple themes** - Config reloads instantly
2. **Match your terminal** - Use same theme everywhere
3. **Consider lighting** - Dark themes for night, light for day
4. **Start with presets** - Easier than custom colors
5. **Customize gradually** - Override one color at a time

---

## Contributing Themes

Want to add a new theme? See [Contributing Guide](../CONTRIBUTING.md) for details.

Popular themes to consider:
- Solarized Dark/Light
- One Dark
- Ayu Dark/Light
- Everforest
- Rosé Pine

---

**Enjoy your beautiful new theme!**
