# Batch

The `batch` command runs multiple generation jobs from a single TOML manifest. Each job can be seed-based or wallpaper-based with its own template, mode, targets, and output directory. Jobs are processed sequentially in manifest order.

## Basic usage

```bash
chromasync batch --file themes.toml
```

Where `themes.toml` is a manifest defining one or more jobs.

## Options

| Flag | Required | Default | Description |
| --- | --- | --- | --- |
| `--file` | yes | — | Path to a TOML batch manifest |

## Manifest format

A batch manifest is a TOML file containing a `[[jobs]]` array. Each entry defines one generation job.

```toml
[[jobs]]
name = "dark-terminal"
seed = "#4ecdc4"
template = "minimal"
mode = "dark"
targets = ["kitty", "alacritty"]
output = "dark-terminal"

[[jobs]]
name = "light-editor"
seed = "#7c3aed"
template = "materialish"
mode = "light"
contrast = "apca-experimental"
targets = ["targets/editor.toml"]
output = "light-editor"
```

The singular `[[job]]` form is also accepted as an alias.

## Job fields

| Field | Required | Default | Description |
| --- | --- | --- | --- |
| `name` | no | `<unnamed>` | Label used in error messages |
| `seed` | conditionally | — | Seed color in `#RRGGBB` format |
| `image` | conditionally | — | Path to a wallpaper image |
| `template` | yes | — | Template name or path to a `.toml` file |
| `mode` | no | `dark` | Theme mode: `dark` or `light` |
| `contrast` | no | `relative-luminance` | Contrast strategy: `relative-luminance` or `apca-experimental` |
| `targets` | no | `[]` | List of target names or `.toml` paths |
| `output` | yes | — | Output directory for this job's artifacts |

Each job must define **exactly one** of `seed` or `image`. Defining both or neither is an error.

## Path resolution

All relative paths in a manifest — `image`, `template` (when it looks like a file path), and `targets` entries — are resolved relative to the **manifest file's parent directory**, not the working directory. This makes manifests portable: you can keep a manifest alongside its target specs and wallpapers and run it from any directory.

A value is treated as a file path if it contains a path separator (`/`), starts with an absolute path prefix, or ends with `.toml`. Otherwise it is looked up as a built-in name.

```toml
# Given a manifest at ~/themes/batch.toml:

[[jobs]]
image = "wallpapers/forest.jpg"          # resolves to ~/themes/wallpapers/forest.jpg
template = "minimal"                      # looked up as a built-in template name
targets = ["kitty", "targets/gtk.toml"]   # kitty = built-in, targets/gtk.toml = ~/themes/targets/gtk.toml
output = "forest-output"
```

## Mixing seed and wallpaper jobs

A manifest can freely mix seed-based and wallpaper-based jobs. Seed jobs behave identically to `generate` and wallpaper jobs behave identically to `wallpaper` — see those guides for details on [palette construction](./generate.md#how-it-works) and [color extraction](./wallpaper.md#color-extraction).

```toml
[[jobs]]
name = "seed-job"
seed = "#4ecdc4"
template = "minimal"
targets = ["kitty", "alacritty"]
output = "seed-output"

[[jobs]]
name = "wallpaper-job"
image = "wallpaper.png"
template = "terminal"
contrast = "apca-experimental"
targets = ["targets/waybar.toml", "targets/foot.toml"]
output = "wallpaper-output"
```

## Output

Each job writes its artifacts to its own `output` directory. On success, every written file path is printed to stdout. Output behavior per job is identical to `generate` — see [Output](./generate.md#output) for overwrite protection and collision detection.

## Error handling

Batch execution is **fail-fast**: if any job fails, the remaining jobs are skipped. Errors include context about which job failed:

- `batch job 3 failed for output 'dark-theme'` — wraps the underlying generation error with the 1-indexed job number and its output directory.
- `batch job 'my-job' must define exactly one of 'seed' or 'image'` — validation error when a job defines both or neither source.
- `batch manifest 'path.toml' does not define any jobs` — the manifest parsed successfully but the `jobs` array is empty.

## Examples

A single manifest generating dark and light variants of the same seed:

```toml
[[jobs]]
name = "dark"
seed = "#ff6b6b"
template = "brutalist"
mode = "dark"
targets = ["kitty", "alacritty", "targets/gtk.toml"]
output = "coral-dark"

[[jobs]]
name = "light"
seed = "#ff6b6b"
template = "brutalist"
mode = "light"
targets = ["kitty", "alacritty", "targets/gtk.toml"]
output = "coral-light"
```

A manifest generating themes from multiple wallpapers:

```toml
[[jobs]]
name = "forest"
image = "wallpapers/forest.jpg"
template = "minimal"
targets = ["kitty", "targets/hyprland.toml", "targets/waybar.toml"]
output = "forest-theme"

[[jobs]]
name = "sunset"
image = "wallpapers/sunset.png"
template = "minimal"
mode = "light"
targets = ["kitty", "targets/hyprland.toml", "targets/waybar.toml"]
output = "sunset-theme"

[[jobs]]
name = "abstract"
image = "wallpapers/abstract.png"
template = "materialish"
contrast = "apca-experimental"
targets = ["targets/editor.toml", "targets/css.toml"]
output = "abstract-theme"
```

Run either manifest:

```bash
chromasync batch --file themes.toml
```
