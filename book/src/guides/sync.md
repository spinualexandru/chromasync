# Sync

The `sync` command runs a named profile from `~/.config/chromasync/config.toml`.
It is intended for repeatable desktop theming setups where the seed, template,
mode, targets, and output directories should live in one config file.

## Basic usage

```bash
chromasync sync
```

This runs the profile named `default`.

```bash
chromasync sync personalGreen
```

This runs the profile named `personalGreen`.

## Config format

Sync profiles live under `[[configs]]`:

```toml
[[configs]]
name = "default"
image_fetch_command = "qs -c noctalia-shell ipc call wallpaper get"
template = "materialish"
mode = "auto"
targets = ["ghostty", "kitty"]
chroma = "industrial"

[[targets]]
name = "ghostty"
output_dir = "~/.config/ghostty/themes"
source = "targets/ghostty.toml"
overwrite = true

[[targets]]
name = "kitty"
output_dir = "~/.config/kitty"
overwrite = true
```

Each profile must define exactly one color source:

- `seed = "#RRGGBB"` for deterministic seed-based generation.
- `image = "wallpaper.png"` for wallpaper-based generation.
- `image_fetch_command = "qs -c noctalia-shell ipc call wallpaper get"` for
  wallpaper-based generation from a command that prints the current image path.

Relative `image`, fetched image path, template path, target TOML path, and
profile `output_dir` values are resolved from the directory containing
`config.toml`. The fetch command runs from that same directory and Chromasync
uses its first non-empty stdout line as the image path.

## Profile fields

| Field | Required | Default | Description |
| --- | --- | --- | --- |
| `name` | yes | - | Profile name selected by `chromasync sync [name]` |
| `seed`, `image`, or `image_fetch_command` | yes | - | Exactly one color source |
| `template` | no | target preference | Template name or `.toml` path |
| `mode` | no | `dark` | `dark`, `light`, or `auto` |
| `contrast` | no | `relative-luminance` | Contrast strategy |
| `chroma` | no | `normal` | Chroma strategy |
| `targets` | yes | - | Non-empty list of target names or paths |
| `output_dir` | no | `chromasync` | Fallback output directory |
| `force` | no | `false` | Overwrite fallback-target artifacts |

## Auto Mode

`mode = "auto"` runs:

```bash
gsettings get org.gnome.desktop.interface color-scheme
```

`prefer-light` maps to light mode and `prefer-dark` maps to dark mode. If the
desktop preference cannot be inferred, Chromasync uses dark mode.

## Target Output Directories

`sync` uses the same `[[targets]]` output mapping as `generate`, `wallpaper`, and
`batch`. A target listed in the profile writes to its configured `output_dir`
when a matching `[[targets]]` entry exists. Targets without an entry write to
the profile's `output_dir` fallback.

Custom targets such as Ghostty must still be discoverable, usually by placing
their TOML file under `~/.config/chromasync/targets/` or by installing them with
`chromasync target install`.
