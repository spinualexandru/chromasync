# `chromasync-wasm`

Browser WebAssembly bindings for Chromasync's image extraction and OKLCH
palette generation.

## Build

Install [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) and run
this command from the repository root:

```bash
wasm-pack build crates/chromasync-wasm \
  --target web \
  --release \
  --out-dir ../../dist/wasm
```

The generated `dist/wasm` directory is an ESM package containing JavaScript,
TypeScript declarations, and the `.wasm` binary. It can be copied into another
project or published to an npm registry.

## Use in a browser

```ts
import init, {
  generatePalette,
  generatePaletteFromImage,
  hexToOkhsl,
  okhslToHex,
} from "./dist/wasm/chromasync_wasm.js";

await init();

const seeded = generatePalette("#4ecdc4", {
  mode: "dark",
  chroma: "vibrant",
});

const response = await fetch("/wallpaper.webp");
const imageBytes = new Uint8Array(await response.arrayBuffer());
const { palette, extraction } = generatePaletteFromImage(imageBytes, {
  mode: "light",
  chroma: "normal",
  maxSeeds: 3,
});

console.log(palette.families.primary.tones);
console.log(extraction.seeds);

const coordinates = hexToOkhsl("#4ecdc4");
const adjustedSeed = okhslToHex(
  coordinates.hue + 12,
  coordinates.saturation * 0.9,
  coordinates.lightness,
);
```

Encoded PNG, JPEG, and WebP bytes are supported. All processing happens
locally; the library does not access the network or browser DOM.

## API

- `version()` returns the Chromasync package version.
- `generatePalette(seed, options?)` generates all nine families from a
  `#RRGGBB` seed.
- `hexToOkhsl(hex)` converts an sRGB hex color to `{ hue, saturation,
  lightness }` coordinates. Hue is expressed in degrees; saturation and
  lightness use the `0..=1` range.
- `okhslToHex(hue, saturation, lightness)` converts those coordinates back to
  an in-gamut `#RRGGBB` sRGB color. Hue accepts `0..=360`; saturation and
  lightness accept `0..=1`.
- `extractColors(imageBytes, options?)` returns ranked seed colors and image
  dimensions without generating a palette.
- `generatePaletteFromImage(imageBytes, options?)` extracts up to three ranked
  colors and returns both the composed palette and extraction metadata.

Options default to `{ mode: "dark", chroma: "normal", maxSeeds: 3 }`. Invalid
input throws a JavaScript `Error` with the Rust validation message.

## Theme previews

`generateTheme(seed, options?)` and `generateThemeFromImage(bytes, options?)`
resolve an embedded style and return the palette, template rules, requested
colors, resolved semantic tokens, and complete Kitty, Zed, GTK4, and Hyprland
artifacts. Everything is generated in memory; no files are read or written.

```ts
const theme = generateTheme("#4ecdc4", {
  template: "materialish",
  mode: "dark",
  chroma: "normal",
});
console.log(theme.tokens.accent, theme.artifacts);
```

`template` defaults to `minimal` and accepts the four embedded style names.
The optional `textTone` overrides the requested text lightness (`0..=1`), useful
for inspecting contrast fallback. `requestedTokens` contains the colors before
foreground adjustment; `contrast` reports requested and resolved ratios for
the text/background and accent foreground/accent pairs.

All preview artifacts use the explicitly selected shared palette and template.
They do not apply the CLI router's target-specific chroma or preferred-template
defaults. The preview API is intended for controlled visual comparisons.
