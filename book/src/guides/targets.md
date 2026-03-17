# Targets

Targets are declarative TOML files that define how semantic color tokens are rendered into application-specific configuration files. When you run `generate` or `wallpaper`, each requested target produces one or more output files with your resolved colors substituted into format-specific templates. The same tokens with different targets produce a GTK stylesheet, a terminal config, a Hyprland color scheme, or any other format you define.

## Listing targets

```bash
chromasync targets
```

This prints all discovered targets with their name, source type, and file location in tab-separated columns:

```
alacritty    built-in    alacritty
kitty        built-in    kitty
css          user-config /home/user/.config/chromasync/targets/css.toml
waybar       pack        catppuccin [/home/user/.config/chromasync/packs/catppuccin/targets/waybar.toml]
```

## Built-in targets

Chromasync ships with two built-in targets compiled into the binary:

| Name | Default artifact | Description |
| --- | --- | --- |
| `kitty` | `kitty.conf` | Kitty terminal emulator theme (foreground, background, cursor, selection, borders, tabs, 16-color ANSI palette) |
| `alacritty` | `alacritty.toml` | Alacritty terminal emulator theme (primary colors, cursor, selection, search, hints, 16-color ANSI palette) |

Use them by name:

```bash
chromasync generate --seed "#4ecdc4" --template minimal --targets kitty,alacritty
```

Built-in targets cannot be overridden or extended by user-defined targets.

## Target sources

Targets are discovered from multiple locations. When multiple sources provide the same name, the highest-precedence source wins:

| Precedence | Source | Location |
| --- | --- | --- |
| 0 (highest) | Built-in | Compiled into the binary |
| 1 | Pack | `~/.local/share/chromasync/packs/*/targets/` or other pack search locations |
| 2 | User config | `~/.config/chromasync/targets/` |
| 3 (lowest) | Filesystem path | Direct path passed to `--targets` |

If the `--targets` value contains `/`, starts with an absolute path, or ends with `.toml`, it is loaded as a file path. Otherwise it is looked up by name.

## Specifying targets

Pass a comma-separated list of target names or file paths to `--targets`:

```bash
chromasync generate \
  --seed "#89b4fa" \
  --template minimal \
  --targets kitty,gtk,/path/to/custom.toml
```

Duplicates in the list are silently deduplicated. Target names are trimmed of whitespace.

## Writing a custom target

A target is a TOML file with a name, optional description, and one or more artifact definitions:

```toml
name = "my-app"
description = "Theme output for My App."

[[artifacts]]
file_name = "theme.conf"
template = """
foreground={{tokens.text}}
background={{tokens.bg}}
accent={{tokens.accent}}
"""
```

Each `[[artifacts]]` entry produces one output file. Placeholders are substituted with resolved color values at generation time.

### Top-level fields

| Field | Required | Description |
| --- | --- | --- |
| `name` | yes | Identifier matching `[a-z0-9_-]+` |
| `description` | no | Human-readable description shown by `chromasync targets` |
| `extends` | no | Name of another user-defined target to inherit from |

Target names must not collide with built-in target names (`kitty`, `alacritty`).

### Artifact fields

| Field | Required | Description |
| --- | --- | --- |
| `file_name` | yes | Output file name (no path separators, no `.` or `..`) |
| `template` | yes | Template content with `{{...}}` placeholders |

A target must have at least one artifact unless it uses `extends`.

## Placeholders

Placeholders use the syntax `{{<value>}}` or `{{<value> | <transform>}}`. They are replaced with resolved values at generation time.

### Token values

Reference any of the 17 semantic tokens:

| Placeholder | Description |
| --- | --- |
| `{{tokens.bg}}` | Primary background |
| `{{tokens.bg_secondary}}` | Secondary background |
| `{{tokens.surface}}` | Interactive surface |
| `{{tokens.surface_elevated}}` | Elevated surface |
| `{{tokens.text}}` | Primary text |
| `{{tokens.text_muted}}` | Muted text |
| `{{tokens.border}}` | Regular border |
| `{{tokens.border_strong}}` | Strong border |
| `{{tokens.accent}}` | Primary accent |
| `{{tokens.accent_hover}}` | Accent hover state |
| `{{tokens.accent_active}}` | Accent active state |
| `{{tokens.accent_fg}}` | Text on accent backgrounds |
| `{{tokens.selection}}` | Selection background |
| `{{tokens.link}}` | Link color |
| `{{tokens.success}}` | Success state |
| `{{tokens.warning}}` | Warning state |
| `{{tokens.error}}` | Error state |

### Context values

Reference generation context:

| Placeholder | Description |
| --- | --- |
| `{{ctx.mode}}` | Theme mode (`dark` or `light`) |
| `{{ctx.template_name}}` | Name of the template used |
| `{{ctx.output_dir}}` | Output directory path |
| `{{ctx.seed}}` | Seed color (if generation used one) |

Context values do not support transforms.

### Transforms

Transforms modify the output format of token values:

| Syntax | Output | Example |
| --- | --- | --- |
| `{{tokens.bg}}` | `#rrggbb` (default hex) | `#1a1b26` |
| `{{tokens.bg \| hex_no_hash}}` | `rrggbb` (hex without `#`) | `1a1b26` |
| `{{tokens.bg \| rgba(FF)}}` | `rgba(RRGGBBAA)` (uppercase hex with alpha) | `rgba(1A1B26FF)` |

The `rgba()` transform accepts any two-digit hex alpha value. Use `FF` for full opacity or lower values like `CC` for transparency.

## Target inheritance

A target can extend another user-defined target with the `extends` field. The child inherits all artifacts from the base and can add new ones or override existing ones by matching `file_name`:

```toml
name = "my-terminal"
extends = "base-terminal"

[[artifacts]]
file_name = "colors.txt"
template = """
fg={{tokens.text | hex_no_hash}}
bg={{tokens.bg | hex_no_hash}}
"""
```

### Inheritance rules

- The base target must be user-defined or from a pack — extending built-in targets (`kitty`, `alacritty`) is not allowed.
- Chains are supported: target A can extend B, which extends C. Cycles are detected and rejected.
- If a child artifact has the same `file_name` as a base artifact, the child's version replaces it.
- A target with `extends` can omit `[[artifacts]]` entirely to inherit the base unchanged under a new name.

## Installing user targets

Drop `.toml` target files into:

```
~/.config/chromasync/targets/
```

All `.toml` files in this directory are auto-discovered. Targets can also be distributed as part of a [pack](./packs.md).

## Validation

Chromasync validates targets at load time:

- Target names must match `[a-z0-9_-]+`.
- Target names must not collide with built-in target names.
- Artifact file names must be non-empty with no path separators.
- All placeholders must reference valid token or context names.
- Transforms must be valid (`hex_no_hash` or `rgba(XX)`).
- Unterminated placeholders (missing `}}`) are rejected.
- Duplicate target names across sources of equal precedence are an error.

## Examples

### CSS custom properties

A target that outputs CSS custom properties for use in web projects:

```toml
name = "css"
description = "CSS design token target."

[[artifacts]]
file_name = "theme.css"
template = """
:root {
  --chromasync-bg: {{tokens.bg}};
  --chromasync-bg-secondary: {{tokens.bg_secondary}};
  --chromasync-surface: {{tokens.surface}};
  --chromasync-surface-elevated: {{tokens.surface_elevated}};
  --chromasync-text: {{tokens.text}};
  --chromasync-text-muted: {{tokens.text_muted}};
  --chromasync-border: {{tokens.border}};
  --chromasync-border-strong: {{tokens.border_strong}};
  --chromasync-accent: {{tokens.accent}};
  --chromasync-accent-hover: {{tokens.accent_hover}};
  --chromasync-accent-active: {{tokens.accent_active}};
  --chromasync-accent-fg: {{tokens.accent_fg}};
  --chromasync-selection: {{tokens.selection}};
  --chromasync-link: {{tokens.link}};
  --chromasync-success: {{tokens.success}};
  --chromasync-warning: {{tokens.warning}};
  --chromasync-error: {{tokens.error}};
}
"""
```

### Hyprland with rgba transforms

Hyprland expects `rgba()` color values. The `rgba(FF)` transform outputs uppercase hex with an alpha suffix:

```toml
name = "hyprland"
description = "Hyprland window manager theme."

[[artifacts]]
file_name = "hyprland.conf"
template = """
$background = {{tokens.bg | rgba(FF)}}
$surface = {{tokens.surface | rgba(FF)}}
$text = {{tokens.text | rgba(FF)}}
$text_muted = {{tokens.text_muted | rgba(FF)}}
$accent = {{tokens.accent | rgba(FF)}}
$accent_hover = {{tokens.accent_hover | rgba(FF)}}
$border = {{tokens.border | rgba(FF)}}
$border_strong = {{tokens.border_strong | rgba(FF)}}
$shadow = {{tokens.bg | rgba(CC)}}

general {
    col.active_border = $accent $accent_hover 45deg
    col.inactive_border = $border
}

decoration {
    col.shadow = $shadow
    shadow_range = 12
    shadow_render_power = 3
}

group {
    col.border_active = $accent
    col.border_inactive = $border
    col.group_border = $border_strong
    col.group_border_active = $accent_hover
}

misc {
    background_color = $background
}
"""
```

### Foot terminal with hex_no_hash

Foot expects bare hex values without the `#` prefix:

```toml
name = "foot"
description = "Foot terminal emulator theme."

[[artifacts]]
file_name = "foot.ini"
template = """
[colors]
foreground={{tokens.text | hex_no_hash}}
background={{tokens.bg | hex_no_hash}}
selection-foreground={{tokens.text | hex_no_hash}}
selection-background={{tokens.selection | hex_no_hash}}
urls={{tokens.link | hex_no_hash}}
regular0={{tokens.bg_secondary | hex_no_hash}}
regular1={{tokens.error | hex_no_hash}}
regular2={{tokens.success | hex_no_hash}}
regular3={{tokens.warning | hex_no_hash}}
regular4={{tokens.link | hex_no_hash}}
regular5={{tokens.accent | hex_no_hash}}
regular6={{tokens.selection | hex_no_hash}}
regular7={{tokens.text_muted | hex_no_hash}}
bright0={{tokens.surface_elevated | hex_no_hash}}
bright1={{tokens.error | hex_no_hash}}
bright2={{tokens.success | hex_no_hash}}
bright3={{tokens.warning | hex_no_hash}}
bright4={{tokens.accent_hover | hex_no_hash}}
bright5={{tokens.accent_active | hex_no_hash}}
bright6={{tokens.border_strong | hex_no_hash}}
bright7={{tokens.text | hex_no_hash}}
"""
```

### Editor theme with context values

An editor theme target that uses `{{ctx.mode}}` to set the theme type dynamically:

```toml
name = "editor"
description = "VS Code editor theme."

[[artifacts]]
file_name = "theme.json"
template = """
{
  "name": "Chromasync {{ctx.mode}}",
  "type": "{{ctx.mode}}",
  "colors": {
    "editor.background": "{{tokens.bg}}",
    "editor.foreground": "{{tokens.text}}",
    "editor.selectionBackground": "{{tokens.selection}}",
    "editorCursor.foreground": "{{tokens.accent}}",
    "activityBar.background": "{{tokens.bg_secondary}}",
    "sideBar.background": "{{tokens.surface}}",
    "statusBar.background": "{{tokens.surface_elevated}}",
    "button.background": "{{tokens.accent}}",
    "button.foreground": "{{tokens.accent_fg}}",
    "textLink.foreground": "{{tokens.link}}"
  }
}
"""
```

### GTK stylesheet

A GTK target mapping semantic tokens to `@define-color` variables with widget styling rules:

```toml
name = "gtk"
description = "GTK theme."

[[artifacts]]
file_name = "gtk.css"
template = """
@define-color window_bg_color {{tokens.bg}};
@define-color window_fg_color {{tokens.text}};
@define-color view_bg_color {{tokens.surface}};
@define-color headerbar_bg_color {{tokens.surface_elevated}};
@define-color accent_bg_color {{tokens.accent}};
@define-color accent_fg_color {{tokens.accent_fg}};
@define-color selection_bg_color {{tokens.selection}};
@define-color border_color {{tokens.border}};
@define-color link_color {{tokens.link}};
@define-color success_color {{tokens.success}};
@define-color warning_color {{tokens.warning}};
@define-color error_color {{tokens.error}};

window, dialog, popover {
  background-color: @window_bg_color;
  color: @window_fg_color;
}

headerbar {
  background-color: @headerbar_bg_color;
  border-color: @border_color;
}

button.suggested-action, button:checked {
  background-color: @accent_bg_color;
  color: @accent_fg_color;
}
"""
```

### Using custom targets

Install a target to `~/.config/chromasync/targets/` and use it by name:

```bash
chromasync generate \
  --seed "#e06c75" \
  --template minimal \
  --targets kitty,foot,hyprland,gtk
```

Or reference a target file directly:

```bash
chromasync generate \
  --seed "#e06c75" \
  --template minimal \
  --targets kitty,/path/to/my-custom.toml
```
