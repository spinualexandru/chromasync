# Packs

Packs are self-contained theme collections that bundle templates and target specs into a single directory. They let you distribute complete theme kits — a template that defines the color mapping alongside targets that produce output for specific apps — as a unit that others can drop into their system and use by name.

## Listing packs

```bash
chromasync packs
```

This prints all discovered packs with their name, version, and root directory.

To inspect a specific pack's metadata, templates, and targets:

```bash
chromasync pack info aurora
```

## Pack directory structure

A pack is a directory containing a `pack.toml` manifest and one or more subdirectories of templates and/or targets:

```
my-pack/
├── pack.toml
├── templates/
│   ├── my-theme-dark.toml
│   └── my-theme-light.toml
└── targets/
    ├── ghostty.toml
    └── waybar.toml
```

If no `[templates]` or `[targets]` sections are declared in `pack.toml`, Chromasync auto-discovers `templates/` and `targets/` subdirectories if they exist.

## Manifest format

The `pack.toml` file declares the pack's metadata and asset paths:

```toml
name = "my-pack"
version = "1.0.0"
description = "A complete theme kit for my setup."
author = "Your Name"
license = "MIT"
homepage = "https://example.com/my-pack"

[templates]
paths = ["templates"]

[targets]
paths = ["targets"]
```

### Fields

| Field | Required | Description |
| --- | --- | --- |
| `name` | yes | Identifier matching `[a-z0-9_-]+` |
| `version` | yes | Version string (conventionally semver) |
| `description` | no | Human-readable description |
| `author` | no | Creator or maintainer |
| `license` | no | License identifier (e.g. `MIT`, `Apache-2.0`) |
| `homepage` | no | URL to project or repository |
| `[templates]` | no | Section with `paths = [...]` listing template subdirectories |
| `[targets]` | no | Section with `paths = [...]` listing target subdirectories |

Asset paths must be relative to the pack root. Parent directory references (`..`) are not allowed. A pack must contain at least one template or target directory.

## Installing packs

There is no install command — drop the pack directory into one of the search locations:

| Location | Use case |
| --- | --- |
| `~/.config/chromasync/packs/` | Personal packs |
| `~/.local/share/chromasync/packs/` | System-distributed packs |
| `.chromasync/packs/` | Project-local packs (relative to working directory) |

All three locations are scanned on every run. A pack with the same name must not appear in multiple locations.

## Using pack templates and targets

Pack templates and targets are referenced by name, the same way as built-in ones. You do not need to qualify them with the pack name:

```bash
chromasync generate \
  --seed "#89b4fa" \
  --template catppuccin \
  --targets ghostty,waybar
```

If a pack template has the same name and mode as a built-in template, the pack version takes precedence. See the [Template sources](./templates.md#template-sources) table for the full precedence order.

## Writing pack templates

Pack templates use the same TOML format as any other template. See the [Templates](./templates.md#writing-a-custom-template) guide for the full schema. The template's `name` field is what users pass to `--template`.

## Writing pack targets

Pack targets are declarative TOML files that define output artifacts using `{{tokens.<name>}}` placeholders:

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

Each `[[artifacts]]` entry produces one output file. Placeholders are substituted with resolved hex values at generation time.

### Placeholder transforms

Placeholders support transforms for different output formats:

| Syntax | Output |
| --- | --- |
| `{{tokens.bg}}` | `#rrggbb` (default hex) |
| `{{tokens.bg \| hex_no_hash}}` | `rrggbb` (hex without `#`) |
| `{{tokens.bg \| rgba(FF)}}` | `rgba(RRGGBBFF)` (uppercase hex with alpha) |

### Target inheritance

A target can extend another user-defined target with the `extends` field. The child inherits all artifacts from the base and can add new ones or override existing ones by matching `file_name`:

```toml
name = "my-child"
extends = "my-base"

[[artifacts]]
file_name = "extra.conf"
template = """
color={{tokens.accent}}
"""
```

Inheritance rules:

- The base target must be a user-defined or pack target — extending built-in targets (`kitty`, `alacritty`) is not allowed.
- Chains are supported: target A can extend B, which extends C.
- If a child artifact has the same `file_name` as a base artifact, the child's version replaces it.
- A target with `extends` can omit `[[artifacts]]` entirely to inherit the base unchanged under a new name.

## Validation

Chromasync validates packs at discovery time:

- Pack names must match `[a-z0-9_-]+`.
- Duplicate pack names across search locations are an error.
- All declared asset paths must exist and be relative.
- Templates and targets within packs are validated the same way as standalone files.

## Example: Catppuccin pack with Ghostty and Waybar targets

This example builds a complete pack that provides a Catppuccin-style template and targets for Ghostty and Waybar. It uses target inheritance to share a common terminal palette definition between terminal targets and a common GTK color block between GTK-based targets.

### Directory layout

```
~/.config/chromasync/packs/catppuccin/
├── pack.toml
├── templates/
│   ├── catppuccin-dark.toml
│   └── catppuccin-light.toml
└── targets/
    ├── base-terminal.toml
    ├── ghostty.toml
    ├── base-gtk-colors.toml
    └── waybar.toml
```

### pack.toml

```toml
name = "catppuccin"
version = "1.0.0"
description = "Catppuccin-inspired theme with Ghostty and Waybar targets."
author = "Your Name"
license = "MIT"

[templates]
paths = ["templates"]

[targets]
paths = ["targets"]
```

### templates/catppuccin-dark.toml

```toml
name = "catppuccin"
mode = "dark"
description = "Catppuccin-inspired dark theme with pastel accents on deep surfaces."

[tokens.bg]
family = "neutral"
tone = 0.12
chroma = 0.014

[tokens.bg_secondary]
family = "neutral"
tone = 0.15
chroma = 0.016

[tokens.surface]
family = "neutral_variant"
tone = 0.19
chroma = 0.018

[tokens.surface_elevated]
family = "neutral_variant"
tone = 0.24
chroma = 0.020

[tokens.text]
family = "neutral"
tone = 0.92

[tokens.text_muted]
family = "neutral_variant"
tone = 0.70
chroma_scale = 0.60

[tokens.border]
family = "neutral_variant"
tone = 0.30
chroma_scale = 0.50

[tokens.border_strong]
family = "neutral_variant"
tone = 0.40
chroma_scale = 0.65

[tokens.accent]
family = "primary"
tone = 0.74
chroma_scale = 0.85

[tokens.accent_hover]
family = "primary"
tone = 0.80
chroma_scale = 0.90

[tokens.accent_active]
family = "primary"
tone = 0.66
chroma_scale = 0.80

[tokens.accent_fg]
family = "neutral"
tone = 0.12

[tokens.selection]
family = "primary"
tone = 0.30
chroma_scale = 0.45

[tokens.link]
family = "info"
tone = 0.76
chroma_scale = 0.88

[tokens.success]
family = "success"
tone = 0.72
chroma_scale = 0.82

[tokens.warning]
family = "warning"
tone = 0.78
chroma_scale = 0.80

[tokens.error]
family = "error"
tone = 0.70
chroma_scale = 0.85
```

### templates/catppuccin-light.toml

```toml
name = "catppuccin"
mode = "light"
description = "Catppuccin-inspired light theme with pastel accents on warm surfaces."

[tokens.bg]
family = "neutral"
tone = 0.95
chroma = 0.010

[tokens.bg_secondary]
family = "neutral"
tone = 0.91
chroma = 0.012

[tokens.surface]
family = "neutral_variant"
tone = 0.87
chroma = 0.014

[tokens.surface_elevated]
family = "neutral_variant"
tone = 0.82
chroma = 0.016

[tokens.text]
family = "neutral"
tone = 0.14

[tokens.text_muted]
family = "neutral_variant"
tone = 0.40
chroma_scale = 0.58

[tokens.border]
family = "neutral_variant"
tone = 0.76
chroma_scale = 0.45

[tokens.border_strong]
family = "neutral_variant"
tone = 0.64
chroma_scale = 0.60

[tokens.accent]
family = "primary"
tone = 0.50
chroma_scale = 0.82

[tokens.accent_hover]
family = "primary"
tone = 0.56
chroma_scale = 0.88

[tokens.accent_active]
family = "primary"
tone = 0.44
chroma_scale = 0.78

[tokens.accent_fg]
family = "neutral"
tone = 0.96

[tokens.selection]
family = "primary"
tone = 0.84
chroma_scale = 0.30

[tokens.link]
family = "info"
tone = 0.48
chroma_scale = 0.85

[tokens.success]
family = "success"
tone = 0.50
chroma_scale = 0.80

[tokens.warning]
family = "warning"
tone = 0.56
chroma_scale = 0.78

[tokens.error]
family = "error"
tone = 0.52
chroma_scale = 0.82
```

### targets/base-terminal.toml

This base target defines the 16-color terminal palette mapping. Terminal targets extend it and add their app-specific settings.

```toml
name = "base-terminal"
description = "Shared 16-color terminal palette. Not useful on its own — extend it."

[[artifacts]]
file_name = "colors.txt"
template = """
color0={{tokens.bg_secondary | hex_no_hash}}
color1={{tokens.error | hex_no_hash}}
color2={{tokens.success | hex_no_hash}}
color3={{tokens.warning | hex_no_hash}}
color4={{tokens.link | hex_no_hash}}
color5={{tokens.accent | hex_no_hash}}
color6={{tokens.selection | hex_no_hash}}
color7={{tokens.text_muted | hex_no_hash}}
color8={{tokens.surface_elevated | hex_no_hash}}
color9={{tokens.error | hex_no_hash}}
color10={{tokens.success | hex_no_hash}}
color11={{tokens.warning | hex_no_hash}}
color12={{tokens.accent_hover | hex_no_hash}}
color13={{tokens.accent_active | hex_no_hash}}
color14={{tokens.border_strong | hex_no_hash}}
color15={{tokens.text | hex_no_hash}}
"""
```

### targets/ghostty.toml

Ghostty extends the base terminal palette. It overrides the `colors.txt` artifact with Ghostty's config format (using `palette=N` syntax) and adds Ghostty-specific keys like `cursor-color` and selection colors.

```toml
name = "ghostty"
description = "Ghostty terminal theme, extends base-terminal."
extends = "base-terminal"

[[artifacts]]
file_name = "colors.txt"
template = """
background={{tokens.bg | hex_no_hash}}
foreground={{tokens.text | hex_no_hash}}
cursor-color={{tokens.accent | hex_no_hash}}
selection-background={{tokens.selection | hex_no_hash}}
selection-foreground={{tokens.text | hex_no_hash}}
palette=0={{tokens.bg_secondary | hex_no_hash}}
palette=1={{tokens.error | hex_no_hash}}
palette=2={{tokens.success | hex_no_hash}}
palette=3={{tokens.warning | hex_no_hash}}
palette=4={{tokens.link | hex_no_hash}}
palette=5={{tokens.accent | hex_no_hash}}
palette=6={{tokens.selection | hex_no_hash}}
palette=7={{tokens.text_muted | hex_no_hash}}
palette=8={{tokens.surface_elevated | hex_no_hash}}
palette=9={{tokens.error | hex_no_hash}}
palette=10={{tokens.success | hex_no_hash}}
palette=11={{tokens.warning | hex_no_hash}}
palette=12={{tokens.accent_hover | hex_no_hash}}
palette=13={{tokens.accent_active | hex_no_hash}}
palette=14={{tokens.border_strong | hex_no_hash}}
palette=15={{tokens.text | hex_no_hash}}
"""
```

Because the child's `colors.txt` artifact has the same `file_name` as the base's, it replaces the base version entirely. The result is a single file with Ghostty's format. If you later added a Foot or WezTerm target, each could extend `base-terminal` the same way — overriding `colors.txt` with its own format while keeping the palette mapping consistent.

### targets/base-gtk-colors.toml

This base target defines GTK `@define-color` variables. GTK-based targets extend it and add their own widget styles.

```toml
name = "base-gtk-colors"
description = "Shared GTK @define-color block. Extend to add widget styles."

[[artifacts]]
file_name = "style.css"
template = """
@define-color bg {{tokens.bg}};
@define-color bg_secondary {{tokens.bg_secondary}};
@define-color surface {{tokens.surface}};
@define-color surface_elevated {{tokens.surface_elevated}};
@define-color text {{tokens.text}};
@define-color text_muted {{tokens.text_muted}};
@define-color border {{tokens.border}};
@define-color border_strong {{tokens.border_strong}};
@define-color accent {{tokens.accent}};
@define-color accent_hover {{tokens.accent_hover}};
@define-color accent_active {{tokens.accent_active}};
@define-color selection {{tokens.selection}};
@define-color success {{tokens.success}};
@define-color warning {{tokens.warning}};
@define-color error {{tokens.error}};
"""
```

### targets/waybar.toml

Waybar extends the base GTK colors. It overrides `style.css` to include both the color definitions and Waybar-specific widget rules in a single file.

```toml
name = "waybar"
description = "Waybar GTK CSS theme, extends base-gtk-colors."
extends = "base-gtk-colors"

[[artifacts]]
file_name = "style.css"
template = """
@define-color bg {{tokens.bg}};
@define-color bg_secondary {{tokens.bg_secondary}};
@define-color surface {{tokens.surface}};
@define-color surface_elevated {{tokens.surface_elevated}};
@define-color text {{tokens.text}};
@define-color text_muted {{tokens.text_muted}};
@define-color border {{tokens.border}};
@define-color border_strong {{tokens.border_strong}};
@define-color accent {{tokens.accent}};
@define-color accent_hover {{tokens.accent_hover}};
@define-color accent_active {{tokens.accent_active}};
@define-color selection {{tokens.selection}};
@define-color success {{tokens.success}};
@define-color warning {{tokens.warning}};
@define-color error {{tokens.error}};

* {
  border: none;
  border-radius: 0;
  font-family: monospace;
  font-size: 13px;
  min-height: 0;
}

window#waybar {
  background: @bg;
  color: @text;
}

tooltip {
  background: @surface_elevated;
  color: @text;
  border: 1px solid @border_strong;
}

#workspaces {
  background: @bg_secondary;
  margin: 4px 6px;
  padding: 0 4px;
  border: 1px solid @border;
  border-radius: 8px;
}

#workspaces button {
  padding: 0 10px;
  color: @text_muted;
  background: transparent;
  border-bottom: 2px solid transparent;
}

#workspaces button:hover {
  background: @surface;
  color: @text;
  box-shadow: inset 0 -2px @accent_hover;
}

#workspaces button.active {
  background: @surface;
  color: @accent;
  border-bottom-color: @accent;
}

#workspaces button.urgent {
  background: @accent_active;
  color: @text;
}

#clock,
#tray,
#cpu,
#memory,
#network,
#pulseaudio,
#battery {
  margin: 4px 6px;
  padding: 0 10px;
  background: @surface;
  color: @text;
  border: 1px solid @border;
  border-radius: 8px;
}

#battery.charging,
#battery.plugged {
  color: @success;
  border-color: @success;
}

#battery.warning:not(.charging) {
  color: @warning;
  border-color: @warning;
}

#battery.critical:not(.charging) {
  color: @error;
  border-color: @error;
  background: @selection;
}
"""
```

### Generate the theme

With the pack installed, generate a Catppuccin Mocha-style dark theme using a lavender seed:

```bash
chromasync generate \
  --seed "#b4befe" \
  --template catppuccin \
  --mode dark \
  --targets ghostty,waybar \
  --output ~/.config/chromasync-themes/catppuccin-mocha
```

Or a Latte-style light theme with a pink seed:

```bash
chromasync generate \
  --seed "#ea76cb" \
  --template catppuccin \
  --mode light \
  --targets ghostty,waybar \
  --output ~/.config/chromasync-themes/catppuccin-latte
```

Copy the outputs to their final locations:

```bash
cp ~/.config/chromasync-themes/catppuccin-mocha/colors.txt \
   ~/.config/ghostty/themes/catppuccin-mocha

cp ~/.config/chromasync-themes/catppuccin-mocha/style.css \
   ~/.config/waybar/style.css
```
