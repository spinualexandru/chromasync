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
overwrite = true

[[targets]]
name = "kitty"
output_dir = "~/.config/kitty"
overwrite = true

[[hooks]]
name = "reload-kitty"
on = "target:kitty:done"
filters = ["config:default"]
command = "kitty @ load-config"
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

## Hooks

Hooks live under top-level `[[hooks]]` entries in `config.toml` and run only for
`chromasync sync`. They execute after all artifacts have been written
successfully.

```toml
[[hooks]]
name = "reload-all"
on = "targets:done"
command = "hyprctl reload"

[[hooks]]
name = "reload-hyprland-lua"
on = "target:hyprland-lua:done"
command = "hyprctl reload"

[[hooks]]
name = "reload-default-hyprland-lua"
filters = ["config:default"]
on = ["target:hyprland-lua:done"]
command = "hyprctl reload"
```

Supported events are:

- `targets:done` after the sync command writes all generated artifacts.
- `target:<target-name>:done` after a specific target was generated, such as
  `target:hyprland-lua:done`.

`on` can be a single string or an array of strings. A hook runs at most once per
sync command if any event matches and all filters match. The supported filter is
`config:<profile-name>`, for example `config:default`.

Hook commands run from the directory containing `config.toml`, using the same
shell command behavior as `image_fetch_command`. If a hook exits non-zero,
`chromasync sync` exits with an error after the artifacts have already been
written.
