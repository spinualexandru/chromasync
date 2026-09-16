//! In-memory theme previews using the same embedded templates and renderers as the CLI.
use std::collections::BTreeMap;

use chromasync_color::{contrast_ratio, resolve_color_from_components};
use chromasync_renderers::OutputRegistry;
use chromasync_template::{built_in_templates, resolve_tokens};
use chromasync_types::{
    GeneratedArtifact, GeneratedPalette, GenerationContext, SemanticTokenName, SemanticTokens,
    TemplateDefinition,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use super::{extraction_output, js_error, palette_output, parse_options, serialize_to_js};

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"
export type SemanticTokenName = "bg" | "bg_secondary" | "surface" | "surface_elevated"
  | "text" | "text_muted" | "border" | "border_strong" | "accent" | "accent_hover"
  | "accent_active" | "accent_fg" | "selection" | "link" | "success" | "warning" | "error";
export interface ThemeOptions extends PaletteOptions {
  /** Embedded style name. Defaults to minimal. */
  template?: "minimal" | "materialish" | "terminal" | "brutalist";
  /** Override the text token's requested OKLCH lightness, from 0 to 1. */
  textTone?: number;
}
export interface TokenRule { family: PaletteFamilyName; tone: number; chroma: number | null; chroma_scale: number | null; }
export interface ContrastPair { foreground: SemanticTokenName; background: SemanticTokenName; requestedRatio: number; resolvedRatio: number; }
export interface ThemePreview {
  palette: GeneratedPalette;
  template: { name: string; mode: ThemeMode; description: string | null; tokens: Record<SemanticTokenName, TokenRule> };
  requestedTokens: Record<SemanticTokenName, string>;
  tokens: Record<SemanticTokenName, string>;
  contrast: ContrastPair[];
  artifacts: { target: string; file_name: string; content: string }[];
  extraction: ExtractionResult | null;
}
/** Resolve an embedded style and render Kitty, Zed, GTK4 and Hyprland entirely in memory. */
export function generateTheme(seed: string, options?: ThemeOptions): ThemePreview;
/** Extract wallpaper colors, resolve an embedded style and render theme files entirely in memory. */
export function generateThemeFromImage(bytes: Uint8Array, options?: ThemeOptions): ThemePreview;
"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContrastPair {
    foreground: SemanticTokenName,
    background: SemanticTokenName,
    requested_ratio: f32,
    resolved_ratio: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ThemePreview {
    palette: super::PaletteOutput,
    template: TemplateDefinition,
    requested_tokens: BTreeMap<SemanticTokenName, String>,
    tokens: SemanticTokens,
    contrast: Vec<ContrastPair>,
    artifacts: Vec<GeneratedArtifact>,
    extraction: Option<super::ExtractionOutput>,
}

fn parse_theme_options(options: &Option<JsValue>) -> Result<(String, Option<f32>), JsError> {
    let Some(options) = options
        .as_ref()
        .filter(|v| !v.is_null() && !v.is_undefined())
    else {
        return Ok(("minimal".to_owned(), None));
    };
    let template = super::parse_string_option(options, "template", "minimal".to_owned(), |v| {
        matches!(v, "minimal" | "materialish" | "terminal" | "brutalist").then(|| v.to_owned())
    })?;
    let tone = js_sys::Reflect::get(options, &"textTone".into())
        .map_err(|_| JsError::new("failed to read textTone"))?;
    let tone = if tone.is_null() || tone.is_undefined() {
        None
    } else {
        Some(
            tone.as_f64()
                .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
                .ok_or_else(|| JsError::new("textTone must be a number from 0 to 1"))?
                as f32,
        )
    };
    Ok((template, tone))
}

#[wasm_bindgen(js_name = generateTheme, skip_typescript)]
pub fn generate_theme(seed: &str, options: Option<JsValue>) -> Result<JsValue, JsError> {
    let palette_options = parse_options(options.clone())?;
    let (template, tone) = parse_theme_options(&options)?;
    let palette =
        chromasync_core::generate_palette(seed, palette_options.mode, palette_options.chroma)
            .map_err(js_error)?;
    serialize_to_js(&preview(palette, &template, tone).map_err(js_error)?)
}

#[wasm_bindgen(js_name = generateThemeFromImage, skip_typescript)]
pub fn generate_theme_from_image(
    bytes: &[u8],
    options: Option<JsValue>,
) -> Result<JsValue, JsError> {
    let palette_options = parse_options(options.clone())?;
    let (template, tone) = parse_theme_options(&options)?;
    let extraction =
        chromasync_extract::extract_seed_candidates_from_bytes(bytes).map_err(js_error)?;
    let seeds = &extraction.seeds[..extraction.seeds.len().min(palette_options.max_seeds)];
    let palette = chromasync_core::palette_from_extracted_seeds(
        seeds,
        palette_options.mode,
        palette_options.chroma,
    )
    .map_err(js_error)?;
    let mut result = preview(palette, &template, tone).map_err(js_error)?;
    result.extraction = Some(extraction_output(&extraction, palette_options.max_seeds));
    serialize_to_js(&result)
}

fn preview(
    palette: GeneratedPalette,
    name: &str,
    text_tone: Option<f32>,
) -> Result<ThemePreview, Box<dyn std::error::Error>> {
    let mut template = built_in_templates()?
        .into_iter()
        .find(|t| t.definition.name == name && t.definition.mode == palette.mode)
        .ok_or("unknown embedded template")?
        .definition;
    if let Some(tone) = text_tone {
        if !tone.is_finite() || !(0.0..=1.0).contains(&tone) {
            return Err("textTone must be from 0 to 1".into());
        }
        template
            .tokens
            .get_mut(&SemanticTokenName::Text)
            .ok_or("missing text rule")?
            .tone = tone;
    }
    let requested_tokens = template
        .tokens
        .iter()
        .map(|(name, rule)| {
            let family = &palette.families[&rule.family];
            let chroma =
                rule.chroma.unwrap_or(family.base_chroma) * rule.chroma_scale.unwrap_or(1.0);
            resolve_color_from_components(family.hue, chroma, rule.tone).map(|color| (*name, color))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let tokens = resolve_tokens(&palette, &template)?;
    let context = GenerationContext {
        mode: palette.mode,
        template_name: name.to_owned(),
        chroma: palette.chroma,
        seed: Some(palette.seed.clone()),
        ..Default::default()
    };
    let targets = ["kitty", "zed", "gtk4", "hyprland"].map(str::to_owned);
    let artifacts = OutputRegistry::default().generate(&targets, &tokens, &context)?;
    let contrast = [
        (
            SemanticTokenName::Text,
            SemanticTokenName::Bg,
            &tokens.text,
            &tokens.bg,
        ),
        (
            SemanticTokenName::AccentFg,
            SemanticTokenName::Accent,
            &tokens.accent_fg,
            &tokens.accent,
        ),
    ]
    .into_iter()
    .map(|(foreground, background, fg, bg)| {
        Ok(ContrastPair {
            foreground,
            background,
            requested_ratio: contrast_ratio(
                &requested_tokens[&foreground],
                &requested_tokens[&background],
            )?,
            resolved_ratio: contrast_ratio(fg, bg)?,
        })
    })
    .collect::<Result<Vec<_>, chromasync_color::ColorError>>()?;
    Ok(ThemePreview {
        palette: palette_output(palette),
        template,
        requested_tokens,
        tokens,
        contrast,
        artifacts,
        extraction: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chromasync_types::{ChromaStrategy, ThemeMode};

    #[test]
    fn previews_use_shared_tokens_and_real_renderers_in_both_modes() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            for style in ["minimal", "materialish", "terminal", "brutalist"] {
                let palette =
                    chromasync_core::generate_palette("#4ecdc4", mode, ChromaStrategy::Normal)
                        .unwrap();
                let result = preview(palette.clone(), style, None).unwrap();
                assert_eq!(
                    result.tokens,
                    resolve_tokens(&palette, &result.template).unwrap()
                );
                assert_eq!(result.artifacts.len(), 5);
                let kitty = result
                    .artifacts
                    .iter()
                    .find(|a| a.target == "kitty")
                    .unwrap();
                assert!(
                    kitty
                        .content
                        .contains(&format!("foreground {}", result.tokens.text))
                );
                let zed = result.artifacts.iter().find(|a| a.target == "zed").unwrap();
                let zed: serde_json::Value = serde_json::from_str(&zed.content).unwrap();
                assert_eq!(zed["themes"][0]["appearance"], mode.to_string());
                assert!(
                    result
                        .contrast
                        .iter()
                        .all(|pair| pair.resolved_ratio >= 4.5)
                );
            }
        }
    }

    #[test]
    fn inspector_reports_real_text_fallback_and_validates_tone() {
        let palette =
            chromasync_core::generate_palette("#4ecdc4", ThemeMode::Dark, ChromaStrategy::Normal)
                .unwrap();
        let result = preview(palette.clone(), "minimal", Some(0.1)).unwrap();
        assert!(result.contrast[0].requested_ratio < 4.5);
        assert!(result.contrast[0].resolved_ratio >= 4.5);
        assert_ne!(
            result.requested_tokens[&SemanticTokenName::Text],
            result.tokens.text
        );
        assert!(preview(palette, "minimal", Some(f32::NAN)).is_err());
    }
}
