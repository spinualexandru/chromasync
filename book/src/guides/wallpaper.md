# Wallpaper

The `wallpaper` command creates theme files from a wallpaper image. Chromasync extracts up to three dominant colors from the image, builds a multi-seed palette where each seed drives a separate palette family, maps it to semantic tokens through the template, and writes output files for each target.

## Basic usage

```bash
chromasync wallpaper \
  --image ~/wallpapers/mountain.png \
  --template brutalist \
  --targets kitty
```

This writes `kitty.conf` into the default output directory (`./chromasync/`).

## Options

| Flag | Required | Default | Description |
| --- | --- | --- | --- |
| `--image` | yes | — | Path to a wallpaper image file |
| `--template` | yes | — | Template name or path to a `.toml` file |
| `--targets` | yes | — | Comma-separated list of target names or `.toml` paths |
| `--mode` | no | `dark` | Theme mode: `dark` or `light` |
| `--contrast` | no | `relative-luminance` | Contrast strategy: `relative-luminance` or `apca-experimental` |
| `--output` | no | `chromasync` | Directory to write artifacts into |

## Choosing a template

See [Choosing a template](./generate.md#choosing-a-template) in the generate guide. The `--template` flag works identically.

## Choosing targets

See [Choosing targets](./generate.md#choosing-targets) in the generate guide. The `--targets` flag works identically.

## Output

Output behavior is identical to `generate` — see [Output](./generate.md#output) for details on overwrite protection and collision detection.

## Color extraction

Instead of a single `--seed` color, the wallpaper command extracts colors from the image and uses them to build a richer palette.

### Multi-seed palette construction

Extraction returns up to three dominant colors ranked by pixel count. Each seed drives a different palette family:

- **Seed 0** (most dominant) → primary family — also used as the base seed for all derived families (neutral, neutral-variant, error, success, warning, info).
- **Seed 1** (second most dominant) → secondary family, replacing the secondary family that would otherwise be derived from seed 0.
- **Seed 2** (third most dominant) → tertiary family, replacing the derived tertiary family.

If the image yields fewer than three seeds, the missing families remain derived from the primary seed, exactly as in `generate`.

### Region labeling

Each extracted seed is labeled with its average spatial position in the image using a 3x3 grid: `top-left`, `top-center`, `top-right`, `center-left`, `center`, `center-right`, `bottom-left`, `bottom-center`, `bottom-right`.

### Noisy image fallback

If no single color bucket accounts for at least 10% of visible pixels, the image is considered noisy. In this case, extraction returns a single seed computed as the average color of all visible pixels rather than attempting to separate clusters.

## Contrast strategies

See [Contrast strategies](./generate.md#contrast-strategies) in the generate guide. The `--contrast` flag works identically.

## Examples

Generate a dark theme for Kitty from a wallpaper:

```bash
chromasync wallpaper \
  --image ~/wallpapers/forest.jpg \
  --template minimal \
  --targets kitty
```

Generate a light theme for multiple targets:

```bash
chromasync wallpaper \
  --image ~/wallpapers/sunset.png \
  --template materialish \
  --mode light \
  --targets kitty,alacritty,examples/targets/gtk.toml,examples/targets/hyprland.toml
```

Generate with APCA contrast into a custom directory:

```bash
chromasync wallpaper \
  --image ~/wallpapers/abstract.png \
  --template brutalist \
  --contrast apca-experimental \
  --targets examples/targets/editor.toml \
  --output ./my-theme
```

## How it works

1. The image is loaded and downscaled to fit within 128x128 pixels (preserving aspect ratio) using triangle filtering.
2. Each pixel is quantized to 4-bit color (shifting RGB channels right by 4 bits), grouping similar colors into buckets. Pixels with alpha below 16 are skipped.
3. Buckets are sorted by pixel count. If the largest bucket holds less than 10% of visible pixels, the image is treated as noisy and a single average-color seed is returned.
4. Otherwise, up to three seeds are taken from the largest buckets, each with a dominance score and a region label derived from the bucket's average pixel position.
5. The primary seed generates the full nine-family OKLCH palette. If a second or third seed exists, it replaces the secondary or tertiary family respectively.
6. The template maps semantic tokens to the palette, and each target renders its output files — identical to the final steps of `generate`.
