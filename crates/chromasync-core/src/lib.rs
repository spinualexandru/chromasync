mod packs;

use std::fmt::Write;

use chromasync_color::ColorError;
use chromasync_extract::{ExtractError, ExtractedSeed};
use chromasync_renderers::{RendererError, RendererRegistry, TargetRegistry};
use chromasync_template::{ListedTemplate, TemplateError, resolve_tokens_with_strategy};
use chromasync_types::{
    ContrastStrategy, GeneratedArtifact, GeneratedPalette, GenerationContext, GenerationRequest,
    PaletteFamilyName, SemanticTokens, ThemeMode, ThemePack,
};
use thiserror::Error;

pub use chromasync_renderers::{ListedTarget, OutputRegistry};
pub use packs::{PackError, PackRegistry, pack_search_roots};

#[derive(Debug, Clone)]
pub struct PackInfo {
    pub pack: ThemePack,
    pub templates: Vec<ListedTemplate>,
    pub targets: Vec<ListedTarget>,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{feature} is not implemented yet")]
    NotYetImplemented { feature: &'static str },
    #[error("{operation} requires a seed color via --seed")]
    MissingSeed { operation: &'static str },
    #[error("{operation} requires an image path via --image")]
    MissingWallpaper { operation: &'static str },
    #[error(transparent)]
    Pack(#[from] PackError),
    #[error(transparent)]
    Template(#[from] TemplateError),
    #[error(transparent)]
    Renderer(#[from] RendererError),
    #[error(transparent)]
    Extract(#[from] ExtractError),
    #[error(transparent)]
    Color(#[from] ColorError),
}

pub fn list_templates() -> Result<Vec<ListedTemplate>, CoreError> {
    let packs = PackRegistry::discover()?;

    chromasync_template::list_templates_with_packs(packs.packs()).map_err(CoreError::from)
}

pub fn list_targets() -> Result<Vec<ListedTarget>, CoreError> {
    Ok(load_output_registry()?.list_targets())
}

pub fn load_output_registry() -> Result<OutputRegistry, CoreError> {
    let packs = PackRegistry::discover()?;

    OutputRegistry::discover_with_packs(packs.packs()).map_err(CoreError::from)
}

pub fn list_packs() -> Result<Vec<ThemePack>, CoreError> {
    Ok(PackRegistry::discover()?.packs().to_vec())
}

pub fn pack_info(name: &str) -> Result<PackInfo, CoreError> {
    let packs = PackRegistry::discover()?;
    let pack = packs
        .get(name)
        .cloned()
        .ok_or_else(|| PackError::PackNotFound {
            pack: name.to_owned(),
        })?;
    let templates = chromasync_template::pack_templates(&pack)?;
    let built_in = RendererRegistry::new();
    let targets = TargetRegistry::from_theme_packs(
        std::slice::from_ref(&pack),
        &built_in.built_in_name_set(),
    )?
    .list_targets()
    .to_vec();

    Ok(PackInfo {
        pack,
        templates,
        targets,
    })
}

pub fn generate_palette(seed: &str, mode: ThemeMode) -> Result<GeneratedPalette, CoreError> {
    chromasync_color::generate_palette(seed, mode).map_err(CoreError::from)
}

pub fn generate(request: &GenerationRequest) -> Result<Vec<GeneratedArtifact>, CoreError> {
    let output_registry = load_output_registry()?;

    generate_with_output_registry(request, &output_registry)
}

pub fn preview(request: &GenerationRequest) -> Result<String, CoreError> {
    let palette = palette_from_request(request, "preview")?;
    let template = load_template(&request.template, request.mode)?;
    let tokens = resolve_tokens_with_strategy(&palette, &template.definition, request.contrast)?;

    Ok(render_preview(
        &palette,
        &template,
        &tokens,
        request.contrast,
    ))
}

pub fn export_tokens(request: &GenerationRequest) -> Result<SemanticTokens, CoreError> {
    let palette = palette_from_request(request, "token export")?;
    let template = load_template(&request.template, request.mode)?;

    resolve_tokens_with_strategy(&palette, &template.definition, request.contrast)
        .map_err(CoreError::from)
}

pub fn generate_from_wallpaper(
    request: &GenerationRequest,
) -> Result<Vec<GeneratedArtifact>, CoreError> {
    let output_registry = load_output_registry()?;

    generate_from_wallpaper_with_output_registry(request, &output_registry)
}

pub fn generate_with_output_registry(
    request: &GenerationRequest,
    output_registry: &OutputRegistry,
) -> Result<Vec<GeneratedArtifact>, CoreError> {
    let palette = palette_from_request(request, "theme generation")?;

    render_from_palette(request, &palette, output_registry)
}

pub fn generate_from_wallpaper_with_output_registry(
    request: &GenerationRequest,
    output_registry: &OutputRegistry,
) -> Result<Vec<GeneratedArtifact>, CoreError> {
    let palette = palette_from_wallpaper_request(request)?;

    render_from_palette(request, &palette, output_registry)
}

fn palette_from_request(
    request: &GenerationRequest,
    operation: &'static str,
) -> Result<GeneratedPalette, CoreError> {
    let seed = request
        .seed
        .as_deref()
        .ok_or(CoreError::MissingSeed { operation })?;

    generate_palette(seed, request.mode)
}

fn palette_from_wallpaper_request(
    request: &GenerationRequest,
) -> Result<GeneratedPalette, CoreError> {
    let wallpaper = request
        .wallpaper
        .as_deref()
        .ok_or(CoreError::MissingWallpaper {
            operation: "wallpaper theme generation",
        })?;
    let extraction = chromasync_extract::extract_seed_candidates(wallpaper)?;

    palette_from_extracted_seeds(&extraction.seeds, request.mode)
}

fn palette_from_extracted_seeds(
    seeds: &[ExtractedSeed],
    mode: ThemeMode,
) -> Result<GeneratedPalette, CoreError> {
    let primary_seed = seeds.first().ok_or(CoreError::NotYetImplemented {
        feature: "wallpaper extraction returned no seed candidates",
    })?;
    let mut palette = generate_palette(&primary_seed.hex, mode)?;

    apply_seed_metadata(&mut palette, primary_seed, 0);

    if let Some(seed) = seeds.get(1) {
        replace_family_from_seed(&mut palette, PaletteFamilyName::Secondary, seed, 1, mode)?;
    }

    if let Some(seed) = seeds.get(2) {
        replace_family_from_seed(&mut palette, PaletteFamilyName::Tertiary, seed, 2, mode)?;
    }

    Ok(palette)
}

fn replace_family_from_seed(
    palette: &mut GeneratedPalette,
    family_name: PaletteFamilyName,
    seed: &ExtractedSeed,
    seed_index: usize,
    mode: ThemeMode,
) -> Result<(), CoreError> {
    let source_palette = generate_palette(&seed.hex, mode)?;
    let mut family = source_palette
        .families
        .get(&PaletteFamilyName::Primary)
        .cloned()
        .ok_or(CoreError::NotYetImplemented {
            feature: "primary palette family generation",
        })?;

    family.name = family_name;
    family.dominance = Some(seed.dominance);
    family.source_region = seed.source_region.clone();
    family.seed_index = Some(seed_index);
    palette.families.insert(family_name, family);

    Ok(())
}

fn apply_seed_metadata(palette: &mut GeneratedPalette, seed: &ExtractedSeed, seed_index: usize) {
    for family in palette.families.values_mut() {
        family.dominance = Some(seed.dominance);
        family.source_region = seed.source_region.clone();
        family.seed_index = Some(seed_index);
    }
}

fn render_from_palette(
    request: &GenerationRequest,
    palette: &GeneratedPalette,
    output_registry: &OutputRegistry,
) -> Result<Vec<GeneratedArtifact>, CoreError> {
    let template = load_template(&request.template, request.mode)?;
    let tokens = resolve_tokens_with_strategy(palette, &template.definition, request.contrast)?;
    let context = GenerationContext {
        mode: request.mode,
        template_name: template.definition.name.clone(),
        output_dir: request.output_dir.clone(),
        seed: request.seed.clone(),
    };

    output_registry
        .generate(&request.targets, &tokens, &context)
        .map_err(CoreError::from)
}

fn load_template(requested: &str, mode: ThemeMode) -> Result<ListedTemplate, CoreError> {
    let packs = PackRegistry::discover()?;

    chromasync_template::load_template_with_packs(requested, mode, packs.packs())
        .map_err(CoreError::from)
}

fn render_preview(
    palette: &GeneratedPalette,
    template: &ListedTemplate,
    tokens: &SemanticTokens,
    contrast: ContrastStrategy,
) -> String {
    let mut output = String::with_capacity(2048);

    let _ = writeln!(output, "Seed: {}", palette.seed);
    let _ = writeln!(output, "Mode: {}", palette.mode);
    let _ = writeln!(output, "Template: {}", template.definition.name);
    let _ = writeln!(output, "Contrast: {contrast}");
    let _ = writeln!(
        output,
        "Template Source: {} ({})",
        template.source.label(),
        template.source.location()
    );

    if let Some(description) = &template.definition.description {
        let _ = writeln!(output, "Description: {description}");
    }

    let _ = writeln!(output);
    let _ = writeln!(output, "Palette Families");

    for family_name in PaletteFamilyName::ALL {
        if let Some(family) = palette.families.get(&family_name) {
            let _ = write!(
                output,
                "{} hue={:.2} chroma={:.3}",
                family_name, family.hue, family.base_chroma
            );

            for tone in &family.tones {
                let _ = write!(output, " {}={}", tone.tone, tone.hex);
            }

            let _ = writeln!(output);
        }
    }

    let _ = writeln!(output);
    let _ = writeln!(output, "Semantic Tokens");

    for (name, value) in semantic_token_rows(tokens) {
        let _ = writeln!(output, "{name:<16} {value}");
    }

    output
}

fn semantic_token_rows(tokens: &SemanticTokens) -> [(&'static str, &str); 17] {
    [
        ("bg", tokens.bg.as_str()),
        ("bg_secondary", tokens.bg_secondary.as_str()),
        ("surface", tokens.surface.as_str()),
        ("surface_elevated", tokens.surface_elevated.as_str()),
        ("text", tokens.text.as_str()),
        ("text_muted", tokens.text_muted.as_str()),
        ("border", tokens.border.as_str()),
        ("border_strong", tokens.border_strong.as_str()),
        ("accent", tokens.accent.as_str()),
        ("accent_hover", tokens.accent_hover.as_str()),
        ("accent_active", tokens.accent_active.as_str()),
        ("accent_fg", tokens.accent_fg.as_str()),
        ("selection", tokens.selection.as_str()),
        ("link", tokens.link.as_str()),
        ("success", tokens.success.as_str()),
        ("warning", tokens.warning.as_str()),
        ("error", tokens.error.as_str()),
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chromasync_color::contrast_ratio;
    use chromasync_types::{ContrastStrategy, GenerationRequest, PaletteFamilyName, ThemeMode};

    use super::{export_tokens, generate_palette, palette_from_wallpaper_request, preview};

    #[test]
    fn palette_generation_is_available_without_renderer_code() {
        let palette =
            generate_palette("#ff6b6b", ThemeMode::Dark).expect("palette generation should work");

        assert!(palette.families.contains_key(&PaletteFamilyName::Primary));
        assert!(palette.families.contains_key(&PaletteFamilyName::Neutral));
    }

    #[test]
    fn token_export_resolves_built_in_templates() {
        let request = GenerationRequest {
            seed: Some("#4ecdc4".to_owned()),
            wallpaper: None,
            template: "minimal".to_owned(),
            mode: ThemeMode::Dark,
            contrast: ContrastStrategy::RelativeLuminance,
            targets: Vec::new(),
            output_dir: "chromasync".into(),
        };

        let tokens = export_tokens(&request).expect("token export should work");

        assert!(tokens.bg.starts_with('#'));
        assert!(tokens.accent.starts_with('#'));
        assert!(
            contrast_ratio(&tokens.text, &tokens.bg).expect("contrast should compute")
                >= chromasync_color::MIN_CONTRAST_RATIO
        );
    }

    #[test]
    fn preview_includes_palette_and_semantic_token_sections() {
        let request = GenerationRequest {
            seed: Some("#ff6b6b".to_owned()),
            wallpaper: None,
            template: "brutalist".to_owned(),
            mode: ThemeMode::Dark,
            contrast: ContrastStrategy::RelativeLuminance,
            targets: Vec::new(),
            output_dir: "chromasync".into(),
        };

        let rendered = preview(&request).expect("preview should render");

        assert!(rendered.contains("Palette Families"));
        assert!(rendered.contains("Semantic Tokens"));
        assert!(rendered.contains("primary"));
        assert!(rendered.contains("accent"));
    }

    #[test]
    fn generate_returns_requested_artifacts() {
        let request = GenerationRequest {
            seed: Some("#4ecdc4".to_owned()),
            wallpaper: None,
            template: "terminal".to_owned(),
            mode: ThemeMode::Dark,
            contrast: ContrastStrategy::RelativeLuminance,
            targets: vec![
                example_target_path("gtk.toml"),
                example_target_path("hyprland.toml"),
                "kitty".to_owned(),
                example_target_path("css.toml"),
            ],
            output_dir: "chromasync".into(),
        };

        let artifacts = super::generate(&request).expect("generation should render artifacts");

        assert_eq!(artifacts.len(), 4);
        assert_eq!(
            artifacts
                .iter()
                .map(|artifact| artifact.target.clone())
                .collect::<Vec<_>>(),
            vec![
                "gtk".to_owned(),
                "hyprland".to_owned(),
                "kitty".to_owned(),
                "css".to_owned(),
            ]
        );
        assert_eq!(
            artifacts
                .iter()
                .map(|artifact| artifact.file_name.as_str())
                .collect::<Vec<_>>(),
            vec!["gtk.css", "hyprland.conf", "kitty.conf", "theme.css"]
        );
        assert!(
            artifacts
                .iter()
                .all(|artifact| !artifact.content.is_empty())
        );
    }

    #[test]
    fn wallpaper_generation_assigns_metadata_and_multi_seed_families() {
        let request = GenerationRequest {
            seed: None,
            wallpaper: Some(wallpaper_fixture("wallpaper-blocks.png")),
            template: "minimal".to_owned(),
            mode: ThemeMode::Dark,
            contrast: ContrastStrategy::RelativeLuminance,
            targets: vec![example_target_path("css.toml")],
            output_dir: "chromasync".into(),
        };

        let palette = palette_from_wallpaper_request(&request)
            .expect("wallpaper palette generation should succeed");

        let primary = palette
            .families
            .get(&PaletteFamilyName::Primary)
            .expect("primary family should exist");
        let secondary = palette
            .families
            .get(&PaletteFamilyName::Secondary)
            .expect("secondary family should exist");
        let tertiary = palette
            .families
            .get(&PaletteFamilyName::Tertiary)
            .expect("tertiary family should exist");

        assert_eq!(primary.seed_index, Some(0));
        assert_eq!(secondary.seed_index, Some(1));
        assert_eq!(tertiary.seed_index, Some(2));
        assert_eq!(primary.source_region.as_deref(), Some("center-left"));
        assert_eq!(secondary.source_region.as_deref(), Some("center"));
        assert_eq!(tertiary.source_region.as_deref(), Some("center-right"));
        assert!(primary.dominance.expect("primary dominance should exist") > 0.45);
        assert!(
            secondary
                .dominance
                .expect("secondary dominance should exist")
                > 0.30
        );
        assert!(tertiary.dominance.expect("tertiary dominance should exist") > 0.15);
    }

    fn wallpaper_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../chromasync-extract/tests/fixtures")
            .join(name)
    }

    fn example_target_path(name: &str) -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/targets")
            .join(name)
            .display()
            .to_string()
    }
}
