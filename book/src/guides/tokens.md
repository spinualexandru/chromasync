# Tokens

The `tokens` command exports the 17 resolved semantic color tokens as structured data. It runs the same palette generation and template resolution pipeline as `generate` but outputs the token values instead of rendering target artifacts. This is useful for feeding colors into scripts, custom tooling, or formats that Chromasync does not have a target for.

## Usage

```bash
chromasync tokens --seed "#4ecdc4" --template minimal
```

The command prints the resolved tokens to stdout and exits. No files are written.

## Options

| Flag | Required | Default | Description |
| --- | --- | --- | --- |
| `--seed` | yes | — | Seed color in `#RRGGBB` format |
| `--template` | yes | — | Template name or path to a template TOML file |
| `--mode` | no | `dark` | Theme mode: `dark` or `light` |
| `--contrast` | no | `relative-luminance` | Contrast heuristic: `relative-luminance` or `apca-experimental` |
| `--format` | no | `json` | Serialization format: `json` |

The `--template` flag accepts the same values as in [`generate`](./generate.md) — a built-in name, a user-config name, or a file path.

## Output format

The default (and currently only) format is JSON. The output is a flat object with all 17 token names as keys and `#RRGGBB` hex strings as values:

```json
{
  "bg": "#040303",
  "bg_secondary": "#090706",
  "surface": "#0E0A0A",
  "surface_elevated": "#1B1413",
  "text": "#F0E9E9",
  "text_muted": "#C4B3AF",
  "border": "#342521",
  "border_strong": "#533C36",
  "accent": "#E86C6C",
  "accent_hover": "#FA827F",
  "accent_active": "#CF5051",
  "accent_fg": "#010100",
  "selection": "#511718",
  "link": "#78AFEE",
  "success": "#8BAC55",
  "warning": "#D1A33C",
  "error": "#EC817A"
}
```

The JSON is pretty-printed with indentation. Token names use `snake_case`.

## Semantic tokens

Every template resolves all 17 tokens. They are grouped by role:

### Backgrounds and surfaces

| Token | Role |
| --- | --- |
| `bg` | Primary background — the main canvas of the application |
| `bg_secondary` | Secondary background — side panels, alternate rows, grouped sections |
| `surface` | Interactive surface — buttons, inputs, cards |
| `surface_elevated` | Elevated surface — dialogs, tooltips, floating panels, popovers |

### Text

| Token | Role |
| --- | --- |
| `text` | Primary text — body copy, headings |
| `text_muted` | Secondary text — placeholders, captions, disabled labels |

### Borders

| Token | Role |
| --- | --- |
| `border` | Regular border — input outlines, dividers, separators |
| `border_strong` | Emphasized border — focused inputs, section boundaries |

### Accent

| Token | Role |
| --- | --- |
| `accent` | Primary accent — buttons, highlights, active indicators |
| `accent_hover` | Accent hover state — slightly lighter or more saturated variant |
| `accent_active` | Accent active/pressed state — slightly darker or less saturated variant |
| `accent_fg` | Text on accent backgrounds — guaranteed readable against `accent` |

### Selection and links

| Token | Role |
| --- | --- |
| `selection` | Text selection or highlight background |
| `link` | Hyperlink color |

### Status

| Token | Role |
| --- | --- |
| `success` | Success indicator — confirmations, passing checks |
| `warning` | Warning indicator — caution states, degraded status |
| `error` | Error indicator — failures, validation errors, destructive actions |

## Resolution pipeline

The `tokens` command runs the same pipeline as `generate` and `preview`:

1. **Palette generation** — the seed color is converted to OKLCH and used to derive 9 palette families (primary, secondary, tertiary, neutral, neutral\_variant, error, success, warning, info), each with 16 tone samples spanning black to white.

2. **Template loading** — the template is loaded from built-in, user config, pack, or filesystem sources. Each template defines a rule for every token specifying which palette family and tone to sample, with optional chroma overrides.

3. **Token resolution** — for each of the 17 tokens, the resolver looks up the rule's palette family, computes the final chroma as `(chroma OR family.base_chroma) * chroma_scale`, and converts the OKLCH triplet (family hue, final chroma, rule tone) to a `#RRGGBB` hex color.

4. **Contrast adjustment** — after all tokens are resolved, the resolver checks that `text` is readable against `bg` and that `accent_fg` is readable against `accent`. If a pair fails the contrast threshold, the resolver tries neutral light (`tone=0.98`) and dark (`tone=0.06`) fallbacks and picks the candidate with the best score. This ensures the exported tokens always meet accessibility requirements.

## Contrast strategies

The `--contrast` flag selects the algorithm used in step 4:

| Strategy | Algorithm | Minimum threshold |
| --- | --- | --- |
| `relative-luminance` | WCAG 2.0 relative luminance ratio | 4.5:1 |
| `apca-experimental` | APCA (Advanced Perceptual Contrast Algorithm) | Score of 60 |

The default `relative-luminance` is the widely adopted WCAG standard. The `apca-experimental` option uses a perceptually uniform model that better accounts for how the eye perceives contrast at different luminance levels. It is marked experimental and may change in future versions.

See [Contrast strategies](./generate.md#contrast-strategies) for more details.

## Scripting with tokens

Because `tokens` outputs structured JSON, it integrates well with tools like `jq`:

### Extract a single token

```bash
chromasync tokens --seed "#4ecdc4" --template minimal | jq -r '.accent'
# #4CBDB5
```

### Build a shell color palette

```bash
eval "$(chromasync tokens --seed "#4ecdc4" --template minimal \
  | jq -r 'to_entries[] | "CHROMASYNC_\(.key | ascii_upcase)=\(.value)"')"

echo "$CHROMASYNC_ACCENT"
# #4CBDB5
```

### Generate a custom config file

```bash
chromasync tokens --seed "#4ecdc4" --template minimal \
  | jq -r '"cursor_color=\(.accent)\nforeground=\(.text)\nbackground=\(.bg)"' \
  > ~/.config/myapp/colors.conf
```

### Compare dark and light tokens

```bash
diff <(chromasync tokens --seed "#4ecdc4" --template minimal --mode dark) \
     <(chromasync tokens --seed "#4ecdc4" --template minimal --mode light)
```

## Tokens vs preview

Both commands resolve the same tokens from the same pipeline. The difference is in output format and detail:

| | tokens | preview |
| --- | --- | --- |
| Output format | JSON (machine-readable) | Plain text (human-readable) |
| Palette families | Not shown | Shown with hue, chroma, and tone samples |
| Metadata | Not shown | Seed, mode, template source, contrast strategy |
| Use case | Scripting, piping to other tools | Quick visual inspection |

Use `preview` when you want to eyeball the colors. Use `tokens` when you need to feed them into another program.

## Tokens vs generate

| | tokens | generate |
| --- | --- | --- |
| Output | Token values to stdout | Artifact files to disk |
| Targets | Not used | Required (`--targets`) |
| Purpose | Export raw color data | Produce app-specific config files |

If you need output for a specific application, write a [target](./targets.md) instead of processing token JSON manually. Targets handle format-specific details like placeholder substitution and file naming.
