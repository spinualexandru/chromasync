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

Chromasync ships with built-in targets compiled into the binary:

| Name | Default artifact | Description |
| --- | --- | --- |
| `alacritty` | `alacritty.toml` | Alacritty terminal emulator theme (primary colors, cursor, selection, search, hints, 16-color ANSI palette) |
| `chromium` | `manifest.json` | Chromium browser theme extension manifest |
| `claude` | `chromasync.json` | Claude Code custom JSON theme |
| `codex` | `chromasync.tmTheme` | Codex CLI syntax highlighting theme |
| `ghostty` | `chromasync.ghostty` | Ghostty terminal theme |
| `google-chrome` | `manifest.json` | Google Chrome browser theme extension manifest |
| `gtk3` | `gtk.css`, `chromasync.css` | GTK 3 loader and generated user stylesheet |
| `gtk4` | `gtk.css`, `chromasync.css` | GTK 4 loader and libadwaita-compatible stylesheet for the requested mode |
| `helium-browser` | `manifest.json` | Helium Browser theme extension manifest |
| `herdr` | `chromasync-herdr.toml` | Herdr custom theme fragment |
| `hyprland` | `hyprland.conf` | Hyprland color configuration |
| `hyprland-lua` | `hypr-chromasync.lua` | Hyprland Lua color configuration |
| `hyprtoolkit` | `chromasync-hyprtoolkit.conf` | Hyprtoolkit palette include |
| `kcolorscheme` | `chromasync.colors` | KDE KColorScheme palette |
| `kitty` | `kitty.conf` | Kitty terminal emulator theme (foreground, background, cursor, selection, borders, tabs, 16-color ANSI palette) |
| `kvantum` | `chromasync.kvconfig`, `chromasync.svg` | Kvantum configuration and matching widget artwork |
| `limine` | `chromasync-limine.conf` | Limine graphical boot menu color fragment |
| `micro` | `chromasync.micro` | micro editor true-color theme |
| `neovim` | `chromasync.lua` | Neovim colorscheme, Tree-sitter, LSP, diagnostics, and terminal palette |
| `qt5` | `chromasync.conf` | Qt 5 qt5ct palette |
| `qt6` | `chromasync.conf` | Qt 6 qt6ct palette |
| `steam` | `skin.json`, `chromasync.css` | Steam desktop shared dialogs and controls; requires Millennium |
| `vim` | `chromasync.vim` | Vim true-color colorscheme and terminal palette |
| `vscode` | `package.json`, `chromasync-color-theme.json` | Visual Studio Code theme extension |
| `vscode-insiders` | `package.json`, `chromasync-color-theme.json` | Visual Studio Code Insiders theme extension |
| `zed` | `chromasync.json` | Zed editor theme |

Use them by name:

```bash
chromasync generate --seed "#4ecdc4" --template minimal --targets kitty,alacritty,ghostty,hyprland
```

Built-in targets cannot be overridden or extended by user-defined targets.

### Application setup

Generation writes theme artifacts; select or include them in the application
once. Give each target its own output directory: several targets intentionally
use the same application-required filename, such as `chromasync.json`.
For example, generate a Vim theme with:

```bash
chromasync generate --seed "#4ecdc4" --template minimal --mode dark \
  --targets vim --output ~/.vim/colors
```

Use `--force` when replacing a previously generated theme. Persistent
`[[targets]]` entries in the Chromasync configuration can route each built-in
directly to its application directory; see [the config example below](#the-chromasync-config).

#### Kvantum

Generate both files into `~/.config/Kvantum/chromasync/`, then select
`chromasync` in Kvantum Manager and select the Kvantum widget style in your Qt
platform theme settings. The directory, `.kvconfig`, and `.svg` basenames must
match. Restart Qt applications to load changes.

The target supplies a flat widget theme with matching palette, surfaces,
buttons, inputs, selections, check/radio indicators, and arrows. Kvantum's
default theme supplies unspecified metrics and decorations. Keep the SVG and
configuration together: changing only the palette cannot recolor SVG artwork.
See [Kvantum's theme configuration reference](https://github.com/tsujan/Kvantum/blob/master/Kvantum/doc/Theme-Config).

#### Steam (Millennium)

Install [Millennium](https://docs.steambrew.app/) first. Generate both files into
`~/.steam/steam/steamui/skins/chromasync/` on Linux, or your installation's
Millennium theme directory, then select Chromasync in Millennium's theme
settings. Reload the theme or restart Steam after generation.

This target recolors shared **desktop dialogs, inputs, buttons, and labels**.
It does not provide a complete library, friends, store, or Big Picture skin.
Steam's internal CSS classes change between client releases; this target uses
the shared `Dialog*` classes instead of pinning opaque library class names.
Its manifest loads only local CSS and requires no JavaScript. See Millennium's
[theme structure](https://docs.steambrew.app/themes/basics/structure) and
[patch configuration](https://docs.steambrew.app/themes/intermediate/custom-structure).

#### hyprqt6engine and hyprtoolkit

**hyprqt6engine does not need another target.** It reads the existing `qt6`
palette format, and builds with KDE Frameworks support also read `kcolorscheme`
files. Generate `qt6` into a chosen directory and reference its absolute path
in `~/.config/hypr/hyprqt6engine.conf`:

```ini
theme {
    color_scheme = /home/you/.config/qt6ct/colors/chromasync.conf
}
```

Use `QT_QPA_PLATFORMTHEME=hyprqt6engine` for applications that should use it.
KDE-enabled builds can instead point `color_scheme` to `chromasync.colors`.
Do not add a `source` directive: this engine's parser does not register one.
See the engine's [palette loader](https://github.com/hyprwm/hyprqt6engine/blob/main/common/common.cpp).

**hyprtoolkit has a separate palette**, so use `--targets hyprtoolkit`. Generate
into `~/.config/hypr/` and add to `~/.config/hypr/hyprtoolkit.conf`:

```ini
source = ~/.config/hypr/chromasync-hyprtoolkit.conf
```

The generated fragment contains only the eight supported color keys, in
`0xAARRGGBB` format, preserving your font and sizing settings. Restart applications
after generation, or touch the main `hyprtoolkit.conf` to trigger its file
watcher; the inspected implementation watches the main file rather than sourced
files. See [hyprtoolkit's palette configuration](https://github.com/hyprwm/hyprtoolkit/blob/main/src/palette/ConfigManager.cpp).

#### Herdr

Generate `--targets herdr` to a staging directory. Merge the emitted `[theme]`
and `[theme.custom]` settings into `~/.config/herdr/config.toml`, replacing
existing keys in those tables. Preserve the rest of your configuration. Use
Herdr's reload-config binding (default `prefix+Shift+R`) or restart it.

Herdr accepts custom color overrides but has no theme-file include in the
inspected loader. Merely placing the generated fragment next to `config.toml`
will not load it. For repeated syncing, use a TOML-aware merge in your own
post-generation hook. The fragment disables Herdr's automatic preset switching
so the current Chromasync palette stays in control. See Herdr's
[theme configuration](https://github.com/herdrdev/herdr/blob/main/src/config/theme.rs).

#### Codex CLI and Claude Code

- **Codex:** generate `--targets codex` into `$CODEX_HOME/themes` (normally
  `~/.codex/themes`), then select Chromasync with `/theme`. This controls syntax
  highlighting in code blocks and diffs; use a terminal target for the terminal
  palette. See [OpenAI's CLI customization documentation](https://learn.chatgpt.com/docs/cli-customization).
- **Claude Code:** generate `--targets claude` into `~/.claude/themes`, then
  select Chromasync with `/theme` (`custom:chromasync`). The JSON uses the
  requested light/dark base and documented color overrides. A version with
  custom JSON themes is required; older preset-only versions cannot load it.
  Claude watches changes to this directory; restart once if you created the
  directory after starting Claude. The terminal background and syntax/diff
  colors not overridden by the target retain their application defaults.
  See [Claude's custom-theme reference](https://code.claude.com/docs/en/terminal-config#create-a-custom-theme).

**Nanocoder:** no target is provided. The inspected checkout loads its bundled
`source/config/themes.json` and selects a preset by name; it does not load user
theme files or custom color overrides. Generating an unused JSON file, or
overwriting the application's bundled theme catalog, would not be a supported
integration. A user-theme loader is needed upstream first.

#### Vim and Neovim

- **Vim:** generate `--targets vim` into `~/.vim/colors/`, then add
  `set termguicolors` and `colorscheme chromasync` to your vimrc. GUI Vim can
  use the GUI colors directly.
- **Neovim:** generate `--targets neovim` into `~/.config/nvim/colors/`, then
  set `vim.opt.termguicolors = true` and `vim.cmd.colorscheme("chromasync")`
  in `init.lua`.

Both themes set the requested background mode, standard syntax/UI groups,
spell indicators, diffs, and all 16 terminal colors. Neovim additionally maps
Tree-sitter captures, LSP groups, and diagnostics. Run `:colorscheme chromasync`
again after regenerating. See [Vim's colorscheme reference](https://vimhelp.org/syntax.txt.html#color-schemes)
and [Neovim's highlighting API](https://neovim.io/doc/user/api/#nvim_set_hl()).

#### Limine

Generate `--targets limine` into a staging directory. Merge its keys into the
**global section before the first menu entry** of the active `limine.conf`,
replacing existing color keys while preserving every boot entry and path.
The fragment is not a complete bootloader configuration and is not automatically
included. Do not replace `limine.conf` with it.

The target follows the current lowercase `key: value` format and emits the two
eight-color palettes, text/background colors, and branding/help colors for the
graphical terminal. Colors omit `#`; `term_background` uses `00RRGGBB` for an
opaque background (Limine's leading byte is transparency). Text-mode firmware
does not use these graphical palette settings. See the upstream
[Limine configuration reference](https://github.com/limine-bootloader/limine/blob/trunk/CONFIG.md).

### GTK 3 coverage

The `gtk3` target recolors standard GTK 3.24 surfaces and controls, including
notebooks and tabs, checkboxes and radio buttons, spin buttons, sliders,
scrollbars, progress and level bars, calendars, lists, text views, menus,
tooltips, and info bars. It uses the generated mode even when an application
requests the opposite base-theme variant. Restart GTK3 applications after
regenerating their stylesheet if they retain the previous colors.

The base theme still supplies layout, icons, and animations. Color-picker
gradients, image content, and application-rendered graphics retain their own
colors; fixed-color images such as Pitivi's white favorite stars cannot be
recolored through ordinary CSS foreground rules. Custom application widgets
may need additional selectors.

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
  --targets kitty,hyprland,gtk,/path/to/custom.toml
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

Target names must not collide with built-in target names.

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

The mode value also supports `mode(dark=VALUE,light=VALUE)` when an output
format uses different identifiers for dark and light themes. Other context
values do not support transforms.

### Transforms

Transforms modify the output format of token values:

Each placeholder accepts at most one transform. Transform chaining is rejected
during target validation.

| Syntax | Output | Example |
| --- | --- | --- |
| `{{tokens.bg}}` | `#rrggbb` (default hex) | `#1a1b26` |
| `{{tokens.bg \| hex_no_hash}}` | `rrggbb` (hex without `#`) | `1a1b26` |
| `{{tokens.bg \| rgb}}` | Decimal RGB components | `26, 27, 38` |
| `{{tokens.bg \| rgba(FF)}}` | `rgba(RRGGBBAA)` (uppercase hex with alpha) | `rgba(1A1B26FF)` |
| `{{ctx.mode \| mode(dark=vs-dark,light=vs)}}` | Mode-specific value | `vs-dark` |

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

- The base target must be user-defined or from a pack — extending built-in targets is not allowed.
- Chains are supported: target A can extend B, which extends C. Cycles are detected and rejected.
- If a child artifact has the same `file_name` as a base artifact, the child's version replaces it.
- A target with `extends` can omit `[[artifacts]]` entirely to inherit the base unchanged under a new name.

## Installing targets

The `target install` command installs a target TOML into the user config and records where its generated artifacts should be written:

```bash
chromasync target install \
  --target examples/targets/gtk.toml \
  --outdir ~/.config/gtk-4.0
```

This:

1. Validates the target and copies it to `~/.config/chromasync/targets/<name>.toml` (discovered by name from then on).
2. Records a `[[targets]]` entry in `~/.config/chromasync/config.toml` mapping the target to its output directory.

The command prints the installed target file path and the config file path.

Re-running install for an already-installed target is refused unless you pass `--overwrite`. The same flag is recorded in the config as `overwrite = true`, which makes `generate`, `wallpaper`, and `batch` overwrite existing artifacts for that target (a per-target `--force`).

You can still drop `.toml` files directly into `~/.config/chromasync/targets/` — all `.toml` files there are auto-discovered. The difference is that `target install` also wires up the per-target output directory. Targets can also be distributed as part of a [pack](./packs.md).

### The chromasync config

`~/.config/chromasync/config.toml` records each installed target's output directory (and overwrite flag):

```toml
[[targets]]
name = "gtk"
output_dir = "~/.config/gtk-4.0"
source = "targets/gtk.toml"
overwrite = false
```

The `generate`, `wallpaper`, and `batch` commands read this config. Each installed target writes its artifacts to the recorded `output_dir` instead of the generic `--output` directory; targets without an entry (built-ins, ad-hoc path targets, or pack targets) fall back to `--output`. A leading `~` in `output_dir` expands to your home directory. Passing `--force` forces every target regardless of the recorded `overwrite` flag.

## Validation

Chromasync validates targets at load time:

- Target names must match `[a-z0-9_-]+`.
- Target names must not collide with built-in target names.
- Artifact file names must be non-empty with no path separators.
- All placeholders must reference valid token or context names.
- Transforms must be valid (`hex_no_hash`, `rgb`, `rgba(XX)`, or a mode mapping).
- Each placeholder may apply at most one transform.
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
