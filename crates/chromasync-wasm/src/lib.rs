use std::collections::BTreeMap;

use chromasync_color::{OkhslColor, hex_to_okhsl as convert_hex_to_okhsl};
use chromasync_extract::{ExtractionResult, extract_seed_candidates_from_bytes};
use chromasync_types::{
    ChromaStrategy, GeneratedPalette, HexColor, PaletteFamilyName, ThemeMode, ToneSample,
};
use js_sys::Reflect;
use serde::Serialize;
use wasm_bindgen::prelude::*;

const DEFAULT_MAX_SEEDS: usize = 3;

#[derive(Debug, Clone, Copy)]
struct PaletteOptions {
    mode: ThemeMode,
    chroma: ChromaStrategy,
    max_seeds: usize,
}

impl Default for PaletteOptions {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
            chroma: ChromaStrategy::Normal,
            max_seeds: DEFAULT_MAX_SEEDS,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImagePaletteResult {
    palette: PaletteOutput,
    extraction: ExtractionOutput,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PaletteOutput {
    seed: HexColor,
    mode: ThemeMode,
    chroma: ChromaStrategy,
    families: BTreeMap<PaletteFamilyName, PaletteFamilyOutput>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PaletteFamilyOutput {
    name: PaletteFamilyName,
    hue: f32,
    base_chroma: f32,
    tones: Vec<ToneSample>,
    dominance: Option<f32>,
    source_region: Option<String>,
    seed_index: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractionOutput {
    original_width: u32,
    original_height: u32,
    processed_width: u32,
    processed_height: u32,
    seeds: Vec<SeedOutput>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SeedOutput {
    hex: String,
    dominance: f32,
    source_region: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OkhslOutput {
    hue: f32,
    saturation: f32,
    lightness: f32,
}

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT_TYPES: &'static str = r#"
export type ThemeMode = "light" | "dark";
export type ChromaStrategy = "subtle" | "normal" | "vibrant" | "muted" | "industrial";
export type PaletteFamilyName =
  | "primary"
  | "secondary"
  | "tertiary"
  | "neutral"
  | "neutral_variant"
  | "error"
  | "success"
  | "warning"
  | "info";

export interface PaletteOptions {
  /** Theme mode used for semantic defaults. Defaults to "dark". */
  mode?: ThemeMode;
  /** Chroma treatment applied during palette generation. Defaults to "normal". */
  chroma?: ChromaStrategy;
  /** Number of ranked image seeds to compose, from 1 to 3. Defaults to 3. */
  maxSeeds?: number;
}

export interface ToneSample {
  tone: number;
  hex: string;
}

export interface PaletteFamily {
  name: PaletteFamilyName;
  hue: number;
  baseChroma: number;
  tones: ToneSample[];
  dominance: number | null;
  sourceRegion: string | null;
  seedIndex: number | null;
}

export interface GeneratedPalette {
  seed: string;
  mode: ThemeMode;
  chroma: ChromaStrategy;
  families: Record<PaletteFamilyName, PaletteFamily>;
}

export interface ExtractedSeed {
  hex: string;
  dominance: number;
  sourceRegion: string | null;
}

export interface ExtractionResult {
  originalWidth: number;
  originalHeight: number;
  processedWidth: number;
  processedHeight: number;
  seeds: ExtractedSeed[];
}

export interface ImagePaletteResult {
  palette: GeneratedPalette;
  extraction: ExtractionResult;
}

export interface OkhslColor {
  /** Hue in degrees from 0 (inclusive) to 360 (exclusive). */
  hue: number;
  /** Perceptual saturation from 0 to 1. */
  saturation: number;
  /** Perceptual lightness from 0 to 1. */
  lightness: number;
}

/** Return the Chromasync package version. */
export function version(): string;

/** Generate a palette directly from a #RRGGBB seed color. */
export function generatePalette(seed: string, options?: PaletteOptions): GeneratedPalette;

/** Convert a #RRGGBB sRGB color into human-adjustable Okhsl coordinates. */
export function hexToOkhsl(hex: string): OkhslColor;

/** Convert sRGB-referenced Okhsl coordinates into a #RRGGBB color. */
export function okhslToHex(hue: number, saturation: number, lightness: number): string;

/** Extract ranked colors from encoded PNG, JPEG, or WebP bytes. */
export function extractColors(imageBytes: Uint8Array, options?: PaletteOptions): ExtractionResult;

/** Extract colors and compose a complete palette from encoded image bytes. */
export function generatePaletteFromImage(
  imageBytes: Uint8Array,
  options?: PaletteOptions,
): ImagePaletteResult;
"#;

#[wasm_bindgen(js_name = version, skip_typescript)]
pub fn package_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[wasm_bindgen(js_name = generatePalette, skip_typescript)]
pub fn generate_palette(seed: &str, options: Option<JsValue>) -> Result<JsValue, JsError> {
    let options = parse_options(options)?;
    let palette =
        chromasync_core::generate_palette(seed, options.mode, options.chroma).map_err(js_error)?;

    serialize_to_js(&palette_output(palette))
}

#[wasm_bindgen(js_name = hexToOkhsl, skip_typescript)]
pub fn hex_to_okhsl(hex: &str) -> Result<JsValue, JsError> {
    let color = convert_hex_to_okhsl(hex).map_err(js_error)?;

    serialize_to_js(&okhsl_output(color))
}

#[wasm_bindgen(js_name = okhslToHex, skip_typescript)]
pub fn okhsl_to_hex(hue: f32, saturation: f32, lightness: f32) -> Result<String, JsError> {
    chromasync_color::okhsl_to_hex(hue, saturation, lightness).map_err(js_error)
}

#[wasm_bindgen(js_name = extractColors, skip_typescript)]
pub fn extract_colors(image_bytes: &[u8], options: Option<JsValue>) -> Result<JsValue, JsError> {
    let options = parse_options(options)?;
    let extraction = extract_seed_candidates_from_bytes(image_bytes).map_err(js_error)?;
    let output = extraction_output(&extraction, options.max_seeds);

    serialize_to_js(&output)
}

#[wasm_bindgen(js_name = generatePaletteFromImage, skip_typescript)]
pub fn generate_palette_from_image(
    image_bytes: &[u8],
    options: Option<JsValue>,
) -> Result<JsValue, JsError> {
    let options = parse_options(options)?;
    let extraction = extract_seed_candidates_from_bytes(image_bytes).map_err(js_error)?;
    let selected_seeds = &extraction.seeds[..extraction.seeds.len().min(options.max_seeds)];
    let palette =
        chromasync_core::palette_from_extracted_seeds(selected_seeds, options.mode, options.chroma)
            .map_err(js_error)?;
    let result = ImagePaletteResult {
        palette: palette_output(palette),
        extraction: extraction_output(&extraction, options.max_seeds),
    };

    serialize_to_js(&result)
}

fn parse_options(options: Option<JsValue>) -> Result<PaletteOptions, JsError> {
    let Some(options) = options.filter(|value| !value.is_null() && !value.is_undefined()) else {
        return Ok(PaletteOptions::default());
    };

    if !options.is_object() {
        return Err(JsError::new("options must be an object"));
    }

    Ok(PaletteOptions {
        mode: parse_string_option(&options, "mode", ThemeMode::Dark, |value| match value {
            "light" => Some(ThemeMode::Light),
            "dark" => Some(ThemeMode::Dark),
            _ => None,
        })?,
        chroma: parse_string_option(
            &options,
            "chroma",
            ChromaStrategy::Normal,
            |value| match value {
                "subtle" => Some(ChromaStrategy::Subtle),
                "normal" => Some(ChromaStrategy::Normal),
                "vibrant" => Some(ChromaStrategy::Vibrant),
                "muted" => Some(ChromaStrategy::Muted),
                "industrial" => Some(ChromaStrategy::Industrial),
                _ => None,
            },
        )?,
        max_seeds: parse_max_seeds(&options)?,
    })
}

fn parse_string_option<T>(
    options: &JsValue,
    name: &str,
    default: T,
    parse: impl FnOnce(&str) -> Option<T>,
) -> Result<T, JsError> {
    let value = Reflect::get(options, &JsValue::from_str(name))
        .map_err(|_| JsError::new(&format!("failed to read the '{name}' option")))?;

    if value.is_null() || value.is_undefined() {
        return Ok(default);
    }

    let value = value
        .as_string()
        .ok_or_else(|| JsError::new(&format!("option '{name}' must be a string")))?;

    parse(&value).ok_or_else(|| JsError::new(&format!("unsupported {name} option '{value}'")))
}

fn parse_max_seeds(options: &JsValue) -> Result<usize, JsError> {
    let value = Reflect::get(options, &JsValue::from_str("maxSeeds"))
        .map_err(|_| JsError::new("failed to read the 'maxSeeds' option"))?;

    if value.is_null() || value.is_undefined() {
        return Ok(DEFAULT_MAX_SEEDS);
    }

    let value = value
        .as_f64()
        .filter(|value| value.fract() == 0.0)
        .ok_or_else(|| JsError::new("option 'maxSeeds' must be an integer from 1 to 3"))?;

    if !(1.0..=3.0).contains(&value) {
        return Err(JsError::new(
            "option 'maxSeeds' must be an integer from 1 to 3",
        ));
    }

    Ok(value as usize)
}

fn extraction_output(extraction: &ExtractionResult, max_seeds: usize) -> ExtractionOutput {
    ExtractionOutput {
        original_width: extraction.original_width,
        original_height: extraction.original_height,
        processed_width: extraction.processed_width,
        processed_height: extraction.processed_height,
        seeds: extraction
            .seeds
            .iter()
            .take(max_seeds)
            .map(|seed| SeedOutput {
                hex: seed.hex.clone(),
                dominance: seed.dominance,
                source_region: seed.source_region.clone(),
            })
            .collect(),
    }
}

fn okhsl_output(color: OkhslColor) -> OkhslOutput {
    OkhslOutput {
        hue: color.hue,
        saturation: color.saturation,
        lightness: color.lightness,
    }
}

fn palette_output(palette: GeneratedPalette) -> PaletteOutput {
    PaletteOutput {
        seed: palette.seed,
        mode: palette.mode,
        chroma: palette.chroma,
        families: palette
            .families
            .into_iter()
            .map(|(name, family)| {
                (
                    name,
                    PaletteFamilyOutput {
                        name: family.name,
                        hue: family.hue,
                        base_chroma: family.base_chroma,
                        tones: family.tones,
                        dominance: family.dominance,
                        source_region: family.source_region,
                        seed_index: family.seed_index,
                    },
                )
            })
            .collect(),
    }
}

fn serialize_to_js(value: &impl Serialize) -> Result<JsValue, JsError> {
    let json = serde_json::to_string(value).map_err(js_error)?;
    js_sys::JSON::parse(&json)
        .map_err(|_| JsError::new("failed to convert the result into a JavaScript object"))
}

fn js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

#[cfg(test)]
mod tests {
    use chromasync_types::{ChromaStrategy, ThemeMode};

    use super::{okhsl_output, palette_output};

    #[test]
    fn palette_output_uses_browser_friendly_field_names() {
        let palette =
            chromasync_core::generate_palette("#4ecdc4", ThemeMode::Dark, ChromaStrategy::Normal)
                .expect("palette should generate");
        let json =
            serde_json::to_value(palette_output(palette)).expect("palette output should serialize");
        let primary = &json["families"]["primary"];

        assert!(primary.get("baseChroma").is_some());
        assert!(primary.get("sourceRegion").is_some());
        assert!(primary.get("seedIndex").is_some());
        assert!(primary.get("base_chroma").is_none());
    }

    #[test]
    fn okhsl_output_uses_normalized_browser_coordinates() {
        let color = chromasync_color::hex_to_okhsl("#4ecdc4").expect("hex should convert");
        let output = okhsl_output(color);
        let json = serde_json::to_value(output).expect("Okhsl output should serialize");

        assert!(json["hue"].as_f64().is_some_and(|value| value > 0.0));
        assert!(
            json["saturation"]
                .as_f64()
                .is_some_and(|value| (0.0..=1.0).contains(&value))
        );
        assert!(
            json["lightness"]
                .as_f64()
                .is_some_and(|value| (0.0..=1.0).contains(&value))
        );
    }
}
