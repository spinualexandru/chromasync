# Preview

The `preview` command shows the resolved palette families and semantic tokens for a seed color and template without writing any files. It is a quick way to inspect what colors a particular seed and template combination produces before committing to a full `generate` run.

## Usage

```bash
chromasync preview --seed "#ff6b6b" --template brutalist
```

Preview prints a plain-text summary to stdout and exits. No targets are invoked and no files are created.

## Options

| Flag | Required | Default | Description |
| --- | --- | --- | --- |
| `--seed` | yes | — | Seed color in `#RRGGBB` format |
| `--template` | yes | — | Template name or path to a template TOML file |
| `--mode` | no | `dark` | Theme mode: `dark` or `light` |
| `--contrast` | no | `relative-luminance` | Contrast heuristic: `relative-luminance` or `apca-experimental` |

The `--template` flag accepts the same values as in [`generate`](./generate.md) — a built-in template name, a user-config template name, or a file path.

## Output format

The output has three sections: a header with generation parameters, a palette families block, and a semantic tokens block.

### Header

```
Seed: #ff6b6b
Mode: dark
Template: brutalist
Contrast: relative-luminance
Template Source: built-in (brutalist-dark.toml)
Description: High-contrast theme with louder borders and harder accent separation
```

The header shows the seed, mode, template name, contrast strategy, where the template was loaded from, and its description (if any).

### Palette families

```
Palette Families
primary         hue=12.34 chroma=0.123  0=#111111 10=#1a1a1a 20=#222222 ...
secondary       hue=45.67 chroma=0.098  0=#111111 10=#1a1a1a 20=#222222 ...
neutral         hue=12.34 chroma=0.008  0=#111111 10=#1a1a1a 20=#222222 ...
...
```

Each row shows a palette family with its hue and chroma values, followed by tone samples rendered as hex colors. This lets you see the full color ramp available to template rules.

### Semantic tokens

```
Semantic Tokens
bg              #1a1a1a
bg_secondary    #202020
surface         #252525
surface_elevated #2a2a2a
text            #e0e0e0
text_muted      #a0a0a0
border          #404040
border_strong   #505050
accent          #ff6b6b
accent_hover    #ff5555
accent_active   #ff4444
accent_fg       #ffffff
selection       #ff6b6b
link            #6b9bff
success         #6bff6b
warning         #ffff6b
error           #ff6b6b
```

All 17 resolved tokens are listed with their final hex values. These are the exact colors that targets would substitute into output files during `generate`.

## Comparing templates

Preview is useful for comparing how different templates interpret the same seed:

```bash
chromasync preview --seed "#4ecdc4" --template minimal
chromasync preview --seed "#4ecdc4" --template brutalist
chromasync preview --seed "#4ecdc4" --template terminal
```

## Comparing modes

Check how dark and light modes differ for the same seed and template:

```bash
chromasync preview --seed "#4ecdc4" --template minimal --mode dark
chromasync preview --seed "#4ecdc4" --template minimal --mode light
```

## Contrast strategies

The `--contrast` flag controls how Chromasync ensures readable foreground colors. The default `relative-luminance` uses WCAG 2.x contrast ratio (minimum 4.5:1). The `apca-experimental` option uses the APCA algorithm instead. Preview shows which strategy was used in the header so you can verify the resolved tokens meet your accessibility requirements.

```bash
chromasync preview \
  --seed "#4ecdc4" \
  --template minimal \
  --contrast apca-experimental
```

See [Contrast strategies](./generate.md#contrast-strategies) for details on the available algorithms.

## Preview vs generate

| | preview | generate |
| --- | --- | --- |
| Output | Text to stdout | Files to disk |
| Targets | Not used | Required (`--targets`) |
| Output directory | Not used | Required (`--output`) |
| Purpose | Inspect colors | Create theme files |

Both commands use the same palette generation and token resolution pipeline, so the colors shown by `preview` are exactly what `generate` would produce.
