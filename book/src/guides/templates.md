# Templates

Templates are TOML files that map 17 semantic tokens to palette family and tone rules. When you run `generate` or `wallpaper`, the template controls which colors from the resolved palette end up as your background, text, accent, borders, and status colors. The same seed with different templates produces different-looking themes.

## Listing templates

```bash
chromasync templates
```

This prints all discovered templates with their name, mode, source type, and file location.

## Built-in templates

Chromasync ships with four templates, each available in dark and light modes:

| Name | Description |
| --- | --- |
| `minimal` | Restrained theme with subdued surfaces and a single clean accent |
| `materialish` | Softer system-style theme with layered surfaces and calmer accents |
| `terminal` | Terminal-oriented theme with deep backgrounds and crisp signal colors |
| `brutalist` | High-contrast theme with louder borders and harder accent separation |

Use them by name:

```bash
chromasync generate --seed "#4ecdc4" --template minimal --targets kitty
```

## Template sources

Templates are discovered from multiple locations. When multiple sources provide the same name and mode, the highest-precedence source wins:

| Precedence | Source | Location |
| --- | --- | --- |
| 0 (lowest) | Built-in | Embedded in the binary |
| 1 | Pack | `~/.local/share/chromasync/packs/*/templates/` |
| 2 | User config | `~/.config/chromasync/templates/` |
| 3 (highest) | Filesystem path | Direct path passed to `--template` |

When loading by name, Chromasync prefers the variant matching the requested `--mode`. If no exact mode match exists, it falls back to the other mode from the highest-precedence source.

If the `--template` value contains `/`, starts with an absolute path, or ends with `.toml`, it is loaded as a file path (precedence 3). Otherwise it is looked up by name.

## Writing a custom template

A template is a TOML file with three top-level fields and exactly 17 token sections:

```toml
name = "my-theme"
mode = "dark"
description = "Optional description of what this template looks like."

[tokens.bg]
family = "neutral"
tone = 0.10
chroma = 0.008

[tokens.accent]
family = "primary"
tone = 0.68
chroma_scale = 0.95

# ... remaining 15 tokens
```

### Top-level fields

| Field | Required | Description |
| --- | --- | --- |
| `name` | yes | Template name used for lookup |
| `mode` | yes | `dark` or `light` |
| `description` | no | Short description shown by `chromasync templates` |

### Token rule fields

Each `[tokens.<name>]` section defines how one semantic token is resolved from the palette:

| Field | Required | Default | Description |
| --- | --- | --- | --- |
| `family` | yes | — | Palette family to sample from |
| `tone` | yes | — | Lightness level, `0.0` (black) to `1.0` (white) |
| `chroma` | no | family base chroma | Absolute chroma override (must be ≥ 0.0) |
| `chroma_scale` | no | `1.0` | Multiplier applied to chroma (must be ≥ 0.0) |

The final chroma for a token is:

```
final_chroma = (chroma OR family.base_chroma) * chroma_scale
```

All 17 tokens must be defined. Omitting any token is a validation error.

## Semantic tokens

Every template must define all 17 tokens:

| Token | Role |
| --- | --- |
| `bg` | Primary background |
| `bg_secondary` | Secondary background (side panels, alternate rows) |
| `surface` | Interactive surface (buttons, inputs) |
| `surface_elevated` | Elevated surface (dialogs, tooltips, floating panels) |
| `text` | Primary text |
| `text_muted` | Secondary/muted text |
| `border` | Regular border |
| `border_strong` | Emphasized border |
| `accent` | Primary accent (buttons, highlights) |
| `accent_hover` | Accent hover state |
| `accent_active` | Accent active/pressed state |
| `accent_fg` | Text on accent backgrounds |
| `selection` | Text selection/highlight background |
| `link` | Link color |
| `success` | Success state |
| `warning` | Warning state |
| `error` | Error state |

## Palette families

Nine families are available for token rules:

| Family | Description |
| --- | --- |
| `primary` | Main accent, derived from the seed color |
| `secondary` | Secondary accent, derived from the seed (or second wallpaper seed) |
| `tertiary` | Tertiary accent, derived from the seed (or third wallpaper seed) |
| `neutral` | Desaturated neutral tones, derived from the seed |
| `neutral_variant` | Slightly chromatic neutral, derived from the seed |
| `error` | Red tones |
| `success` | Green tones |
| `warning` | Yellow tones |
| `info` | Blue tones |

## Tone

Tone is OKLCH lightness mapped to a `0.0`–`1.0` range:

- `0.0` — black
- `1.0` — white

**Dark mode** templates typically use low tones for backgrounds (0.04–0.15) and high tones for text (0.90–0.98). **Light mode** reverses this — high tones for backgrounds (0.88–0.98) and low tones for text (0.08–0.14). Accents sit in the middle range in both modes.

## Chroma control

Use `chroma` to set an absolute saturation value, overriding the family's base chroma. This is common for backgrounds where you want near-neutral surfaces regardless of the seed color:

```toml
[tokens.bg]
family = "neutral"
tone = 0.10
chroma = 0.008
```

Use `chroma_scale` to proportionally adjust the family's base chroma. This preserves the seed's character while tuning intensity:

```toml
[tokens.accent]
family = "primary"
tone = 0.68
chroma_scale = 0.95
```

A scale of `1.0` keeps the family chroma unchanged. Values above `1.0` boost saturation (the `brutalist` template uses up to `1.20` on accents). Values below `1.0` mute it.

You can combine both — `chroma` replaces the base, then `chroma_scale` multiplies the result.

## Installing user templates

Drop `.toml` template files into:

```
~/.config/chromasync/templates/
```

All `.toml` files in this directory are auto-discovered. A user template with the same `name` and `mode` as a built-in template overrides it.

## Contrast adjustment

After token resolution, Chromasync validates that `text` has sufficient contrast against `bg` and that `accent_fg` has sufficient contrast against `accent`. If contrast is insufficient, the resolver falls back to neutral light or dark alternatives. See [Contrast strategies](./generate.md#contrast-strategies) for details on the available algorithms.

## Examples

A custom dark template using the tertiary family for links and secondary for selection:

```toml
name = "my-colorful"
mode = "dark"
description = "A more colorful theme using multiple palette families."

[tokens.bg]
family = "neutral"
tone = 0.08
chroma = 0.006

[tokens.bg_secondary]
family = "neutral"
tone = 0.12
chroma = 0.008

[tokens.surface]
family = "neutral_variant"
tone = 0.16
chroma = 0.012

[tokens.surface_elevated]
family = "neutral_variant"
tone = 0.22
chroma = 0.016

[tokens.text]
family = "neutral"
tone = 0.94

[tokens.text_muted]
family = "neutral_variant"
tone = 0.72
chroma_scale = 0.65

[tokens.border]
family = "neutral_variant"
tone = 0.30
chroma_scale = 0.60

[tokens.border_strong]
family = "primary"
tone = 0.42
chroma_scale = 0.50

[tokens.accent]
family = "primary"
tone = 0.70
chroma_scale = 1.10

[tokens.accent_hover]
family = "primary"
tone = 0.76
chroma_scale = 1.15

[tokens.accent_active]
family = "primary"
tone = 0.62
chroma_scale = 1.05

[tokens.accent_fg]
family = "neutral"
tone = 0.98

[tokens.selection]
family = "secondary"
tone = 0.32
chroma_scale = 0.70

[tokens.link]
family = "tertiary"
tone = 0.76
chroma_scale = 1.00

[tokens.success]
family = "success"
tone = 0.72
chroma_scale = 0.92

[tokens.warning]
family = "warning"
tone = 0.76
chroma_scale = 0.90

[tokens.error]
family = "error"
tone = 0.74
chroma_scale = 0.95
```

Use it with a file path:

```bash
chromasync generate \
  --seed "#e06c75" \
  --template ./my-colorful.toml \
  --targets kitty,alacritty
```

Or install it to `~/.config/chromasync/templates/my-colorful.toml` and use it by name:

```bash
chromasync generate \
  --seed "#e06c75" \
  --template my-colorful \
  --targets kitty,alacritty
```
