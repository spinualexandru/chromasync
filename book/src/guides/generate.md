# Generate

The `generate` command creates theme files from a seed color. You provide a hex color, a template, and one or more targets — Chromasync resolves a full color palette, maps it to semantic tokens through the template, and writes output files for each target.

## Basic usage

```bash
chromasync generate \
  --seed "#ff6b6b" \
  --template brutalist \
  --targets kitty
```

This writes `kitty.conf` into the default output directory (`./chromasync/`).

## Options

| Flag | Required | Default | Description |
| --- | --- | --- | --- |
| `--seed` | yes | — | Seed color in `#RRGGBB` format |
| `--template` | yes | — | Template name or path to a `.toml` file |
| `--targets` | yes | — | Comma-separated list of target names or `.toml` paths |
| `--mode` | no | `dark` | Theme mode: `dark` or `light` |
| `--contrast` | no | `relative-luminance` | Contrast strategy: `relative-luminance` or `apca-experimental` |
| `--output` | no | `chromasync` | Directory to write artifacts into |

## Choosing a template

Pass a built-in template by name:

```bash
--template minimal
```

Or point to a custom template file:

```bash
--template ./my-templates/warm-dark.toml
```

If the value contains `/`, `.`, or ends with `.toml`, it is treated as a file path. Otherwise it is looked up by name from built-in templates and installed packs.

List available templates with:

```bash
chromasync templates
```

## Choosing targets

Targets can be built-in names (`kitty`, `alacritty`), paths to declarative target TOML files, or targets provided by packs. Mix them freely in a comma-separated list:

```bash
--targets kitty,alacritty,examples/targets/gtk.toml,examples/targets/css.toml
```

Declarative example targets for GTK, Hyprland, CSS, Waybar, Foot, Ghostty, and Editor ship under `examples/targets/`.

List available targets with:

```bash
chromasync targets
```

## Output

Artifacts are written to the output directory, which is created if it does not exist. On success, each written file path is printed to stdout:

```
chromasync/kitty.conf
chromasync/gtk.css
chromasync/theme.css
```

### Overwrite protection

Generate refuses to overwrite existing files. If an artifact already exists at the destination, the command fails before writing anything. Delete or move the existing output directory first, or use a different `--output` path.

### Collision detection

If two targets would produce the same output file name, the command fails before writing anything.

## Contrast strategies

The `--contrast` flag controls how Chromasync picks foreground colors that are readable against their backgrounds.

- **`relative-luminance`** (default) — WCAG 2.0 luminance contrast ratio, targeting a minimum of 4.5:1.
- **`apca-experimental`** — APCA (Advanced Perceptual Contrast Algorithm) for more perceptually uniform results. This is experimental and may change.

## Examples

Generate a dark theme for multiple targets:

```bash
chromasync generate \
  --seed "#4ecdc4" \
  --template minimal \
  --mode dark \
  --targets kitty,alacritty,examples/targets/gtk.toml,examples/targets/hyprland.toml
```

Generate a light theme with APCA contrast into a custom directory:

```bash
chromasync generate \
  --seed "#7c3aed" \
  --template materialish \
  --mode light \
  --contrast apca-experimental \
  --targets examples/targets/editor.toml \
  --output ./my-theme
```

## How it works

1. The seed color is parsed and converted to OKLCH color space.
2. Nine palette families are derived (primary, secondary, tertiary, neutral, neutral-variant, error, success, warning, info), each with 16 tone samples spanning black to white.
3. The template maps each of 17 semantic tokens (like `bg`, `accent`, `text`) to a palette family and tone.
4. Each target substitutes the resolved token hex values into its output template and produces one or more artifact files.

Generation is deterministic — the same seed, template, and mode always produce identical output.
