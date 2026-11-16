mod config;
mod packs;

use std::{
    collections::BTreeSet,
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

use chromasync_color::ColorError;
use chromasync_extract::{ExtractError, ExtractedSeed};
use chromasync_renderers::{RendererError, RendererRegistry, TargetRegistry};
use chromasync_template::{ListedTemplate, TemplateError, resolve_tokens_with_strategy};
use chromasync_types::{
    ChromaStrategy, ContrastStrategy, GeneratedArtifact, GeneratedPalette, GenerationContext,
    GenerationRequest, PaletteFamilyName, SemanticTokens, ThemeMode, ThemePack,
};
use thiserror::Error;

pub use chromasync_renderers::{ListedTarget, OutputRegistry};
pub use config::{
    ChromasyncConfig, ConfigHook, ConfigTarget, HookConfigError, HookEvent, HookEvents, HookFilter,
    InstallSummary, ParseHookEventError, ParseHookFilterError, SyncMode, SyncProfile,
    ValidatedHook, config_file_path, expand_tilde, install_target,
};
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
    #[error(
        "{operation} requires a template via --template (target '{target}' does not specify a preferred_template)"
    )]
    MissingTemplate {
        operation: &'static str,
        target: String,
    },
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
    #[error("refusing to overwrite existing artifact '{path}'")]
    ArtifactExists { path: PathBuf },
    #[error("multiple artifacts would write to the same destination '{path}'")]
    ArtifactCollision { path: PathBuf },
    #[error("failed to create output directory '{path}': {source}")]
    CreateOutputDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write artifact '{path}': {source}")]
    WriteArtifact {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not resolve the chromasync user config directory")]
    UserConfigDirUnavailable,
    #[error("failed to read chromasync config '{path}': {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse chromasync config '{path}': {error}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        error: toml::de::Error,
    },
    #[error("failed to serialize chromasync config: {error}")]
    ConfigSerialize { error: String },
    #[error("failed to write chromasync config '{path}': {source}")]
    ConfigWrite {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("target '{name}' is already installed at '{path}'; pass --overwrite to replace it")]
    TargetAlreadyInstalled { name: String, path: PathBuf },
    #[error("failed to create targets directory '{path}': {source}")]
    CreateTargetsDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to copy target file from '{from}' to '{to}': {source}")]
    CopyTargetFile {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
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

pub fn generate_palette(
    seed: &str,
    mode: ThemeMode,
    chroma: ChromaStrategy,
) -> Result<GeneratedPalette, CoreError> {
    chromasync_color::generate_palette(seed, mode, chroma).map_err(CoreError::from)
}

pub fn generate(request: &GenerationRequest) -> Result<Vec<GeneratedArtifact>, CoreError> {
    let output_registry = load_output_registry()?;

    generate_with_output_registry(request, &output_registry)
}

pub fn preview(request: &GenerationRequest) -> Result<String, CoreError> {
    let palette = palette_from_request(request, "preview")?;
    let template_name = request.template.as_deref().ok_or(CoreError::MissingSeed {
        operation: "preview requires a template via --template",
    })?;
    let template = load_template(template_name, request.mode)?;
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
    let template_name = request.template.as_deref().ok_or(CoreError::MissingSeed {
        operation: "token export requires a template via --template",
    })?;
    let template = load_template(template_name, request.mode)?;

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
    render_from_palette(request, output_registry)
}

pub fn generate_from_wallpaper_with_output_registry(
    request: &GenerationRequest,
    output_registry: &OutputRegistry,
) -> Result<Vec<GeneratedArtifact>, CoreError> {
    render_from_palette_with_wallpaper(request, output_registry)
}

/// Generate artifacts with a target-specific output route while sharing the
/// expensive palette preparation across every target.
///
/// The route callback is evaluated once per requested target. Its output
/// directory is also exposed to declarative targets through `ctx.output_dir`.
pub fn generate_routed_with_output_registry<F>(
    request: &GenerationRequest,
    output_registry: &OutputRegistry,
    mut route: F,
) -> Result<Vec<RoutedArtifact>, CoreError>
where
    F: FnMut(&str) -> (PathBuf, bool),
{
    use std::collections::BTreeMap;

    if request.targets.is_empty() {
        return Ok(Vec::new());
    }

    let extracted_seeds = match request.wallpaper.as_deref() {
        Some(wallpaper) => Some(chromasync_extract::extract_seed_candidates(wallpaper)?.seeds),
        None => None,
    };

    if extracted_seeds.is_none() && request.seed.is_none() {
        return Err(CoreError::MissingSeed {
            operation: "theme generation",
        });
    }

    let mut palettes = BTreeMap::<ChromaStrategy, GeneratedPalette>::new();
    let mut render_inputs =
        BTreeMap::<(String, ChromaStrategy), (ListedTemplate, SemanticTokens)>::new();
    let mut routed = Vec::new();

    for target in &request.targets {
        let template_name = request
            .template
            .clone()
            .or_else(|| output_registry.resolve_preferred_template(target))
            .ok_or_else(|| CoreError::MissingTemplate {
                operation: "theme generation",
                target: target.clone(),
            })?;
        let chroma = output_registry
            .resolve_chroma_strategy(target)
            .unwrap_or(request.chroma);
        let key = (template_name.clone(), chroma);

        if let std::collections::btree_map::Entry::Vacant(render_input) =
            render_inputs.entry(key.clone())
        {
            if let std::collections::btree_map::Entry::Vacant(palette_entry) =
                palettes.entry(chroma)
            {
                let palette = match &extracted_seeds {
                    Some(seeds) => palette_from_extracted_seeds(seeds, request.mode, chroma)?,
                    None => generate_palette(
                        request.seed.as_deref().expect("seed checked above"),
                        request.mode,
                        chroma,
                    )?,
                };
                palette_entry.insert(palette);
            }

            let template = load_template(&template_name, request.mode)?;
            let tokens = resolve_tokens_with_strategy(
                palettes
                    .get(&chroma)
                    .expect("palette inserted before token resolution"),
                &template.definition,
                request.contrast,
            )?;
            render_input.insert((template, tokens));
        }

        let (template, tokens) = render_inputs
            .get(&key)
            .expect("render input inserted before target generation");
        let (output_dir, force) = route(target);
        let context = GenerationContext {
            mode: request.mode,
            template_name: template.definition.name.clone(),
            chroma,
            output_dir: output_dir.clone(),
            seed: request.seed.clone(),
        };

        let generated = output_registry
            .generate(std::slice::from_ref(target), tokens, &context)
            .map_err(CoreError::from)?;

        for artifact in generated {
            routed.push(RoutedArtifact {
                artifact,
                output_dir: output_dir.clone(),
                force,
            });
        }
    }

    Ok(routed)
}

/// Write generated artifacts into `output_dir`.
///
/// Returns the full paths of every artifact (in artifact order), whether or not
/// its file had to be rewritten, suitable for echoing to the user. Applies the
/// same policy as [`write_routed_artifacts`], with one `force` flag for every
/// artifact; the MCP server writes through this function.
pub fn write_artifacts(
    output_dir: &Path,
    artifacts: &[GeneratedArtifact],
    force: bool,
) -> Result<Vec<PathBuf>, CoreError> {
    let writes = artifacts
        .iter()
        .map(|artifact| ArtifactWrite {
            output_dir,
            file_name: &artifact.file_name,
            content: &artifact.content,
            force,
        })
        .collect::<Vec<_>>();

    let written = write_entries(&writes)?;
    Ok(written.into_iter().map(|artifact| artifact.path).collect())
}

/// A generated artifact paired with its target-specific write policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedArtifact {
    pub artifact: GeneratedArtifact,
    pub output_dir: PathBuf,
    pub force: bool,
}

/// Outcome of writing one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenArtifact {
    /// Destination path.
    pub path: PathBuf,
    /// `false` when the destination already held identical content, in which
    /// case the file was not rewritten (its modification time is preserved).
    pub changed: bool,
}

/// Write artifacts to their individually routed destinations and report, per
/// destination, whether its content changed.
///
/// This is the single source of truth for write policy; [`write_artifacts`]
/// applies it too. Each entry carries its own `output_dir` and `force` flag, so
/// installed targets can land in different directories and opt into
/// per-target overwrite. Missing output directories are created on demand.
///
/// - A destination that already holds byte-identical content is left
///   untouched and reported with `changed: false`, with or without `force`.
/// - A destination that exists with different content (or is not a regular
///   file) is refused with [`CoreError::ArtifactExists`] unless `force` is set.
/// - Two artifacts mapping to one destination are always refused with
///   [`CoreError::ArtifactCollision`], since that signals a target-design bug
///   rather than stale output.
///
/// All checks run before anything is written. If a write then fails midway,
/// earlier files stay updated.
pub fn write_routed_artifacts(
    artifacts: &[RoutedArtifact],
) -> Result<Vec<WrittenArtifact>, CoreError> {
    let writes = artifacts
        .iter()
        .map(|routed| ArtifactWrite {
            output_dir: &routed.output_dir,
            file_name: &routed.artifact.file_name,
            content: &routed.artifact.content,
            force: routed.force,
        })
        .collect::<Vec<_>>();

    write_entries(&writes)
}

struct ArtifactWrite<'a> {
    output_dir: &'a Path,
    file_name: &'a str,
    content: &'a str,
    force: bool,
}

fn write_entries(entries: &[ArtifactWrite<'_>]) -> Result<Vec<WrittenArtifact>, CoreError> {
    let mut seen_paths = BTreeSet::new();
    let mut written = Vec::with_capacity(entries.len());

    for entry in entries {
        let path = entry.output_dir.join(entry.file_name);

        if !seen_paths.insert(path.clone()) {
            return Err(CoreError::ArtifactCollision { path });
        }

        let changed = match destination_state(&path, entry.content.as_bytes()) {
            Destination::Missing => true,
            Destination::Identical => false,
            Destination::Different if entry.force => true,
            Destination::Different => return Err(CoreError::ArtifactExists { path }),
        };

        written.push(WrittenArtifact { path, changed });
    }

    for entry in entries {
        fs::create_dir_all(entry.output_dir).map_err(|source| CoreError::CreateOutputDir {
            path: entry.output_dir.to_path_buf(),
            source,
        })?;
    }

    for (entry, artifact) in entries.iter().zip(&written) {
        if artifact.changed {
            fs::write(&artifact.path, entry.content).map_err(|source| {
                CoreError::WriteArtifact {
                    path: artifact.path.clone(),
                    source,
                }
            })?;
        }
    }

    Ok(written)
}

/// What currently sits at an artifact's destination.
enum Destination {
    /// Nothing, or nothing we can inspect; writing will surface any error.
    Missing,
    /// A regular file whose bytes equal the new content.
    Identical,
    /// Something else: a file with other content, a directory, and so on.
    Different,
}

fn destination_state(path: &Path, content: &[u8]) -> Destination {
    let Ok(metadata) = fs::metadata(path) else {
        return Destination::Missing;
    };

    // Compare sizes first so a changed file of a different length is never
    // read.
    if !metadata.is_file() || metadata.len() != content.len() as u64 {
        return Destination::Different;
    }

    match fs::read(path) {
        Ok(existing) if existing == content => Destination::Identical,
        _ => Destination::Different,
    }
}

fn palette_from_request(
    request: &GenerationRequest,
    operation: &'static str,
) -> Result<GeneratedPalette, CoreError> {
    let seed = request
        .seed
        .as_deref()
        .ok_or(CoreError::MissingSeed { operation })?;

    generate_palette(seed, request.mode, request.chroma)
}

/// Compose a palette from ranked image seed candidates.
///
/// The dominant seed drives the full palette. The second and third candidates,
/// when present, replace the secondary and tertiary families respectively.
/// This is shared by the native wallpaper pipeline and in-memory consumers such
/// as the WebAssembly package.
pub fn palette_from_extracted_seeds(
    seeds: &[ExtractedSeed],
    mode: ThemeMode,
    chroma: ChromaStrategy,
) -> Result<GeneratedPalette, CoreError> {
    let primary_seed = seeds.first().ok_or(CoreError::NotYetImplemented {
        feature: "wallpaper extraction returned no seed candidates",
    })?;
    let mut palette = generate_palette(&primary_seed.hex, mode, chroma)?;

    apply_seed_metadata(&mut palette, primary_seed, 0);

    if let Some(seed) = seeds.get(1) {
        replace_family_from_seed(
            &mut palette,
            PaletteFamilyName::Secondary,
            seed,
            1,
            mode,
            chroma,
        )?;
    }

    if let Some(seed) = seeds.get(2) {
        replace_family_from_seed(
            &mut palette,
            PaletteFamilyName::Tertiary,
            seed,
            2,
            mode,
            chroma,
        )?;
    }

    Ok(palette)
}

fn replace_family_from_seed(
    palette: &mut GeneratedPalette,
    family_name: PaletteFamilyName,
    seed: &ExtractedSeed,
    seed_index: usize,
    mode: ThemeMode,
    chroma: ChromaStrategy,
) -> Result<(), CoreError> {
    let source_palette = generate_palette(&seed.hex, mode, chroma)?;
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
    output_registry: &OutputRegistry,
) -> Result<Vec<GeneratedArtifact>, CoreError> {
    use std::collections::BTreeMap;

    // Grouping by (template, chroma) to handle overrides
    let mut groups: BTreeMap<(String, ChromaStrategy), Vec<String>> = BTreeMap::new();

    for target in &request.targets {
        let template_name = request
            .template
            .clone()
            .or_else(|| output_registry.resolve_preferred_template(target))
            .ok_or_else(|| CoreError::MissingTemplate {
                operation: "theme generation",
                target: target.clone(),
            })?;
        let chroma = output_registry
            .resolve_chroma_strategy(target)
            .unwrap_or(request.chroma);

        groups
            .entry((template_name, chroma))
            .or_default()
            .push(target.clone());
    }

    let mut artifacts = Vec::new();

    for ((template_name, chroma), targets) in &groups {
        let template = load_template(template_name, request.mode)?;
        let palette = if *chroma == request.chroma {
            palette_from_request(request, "theme generation")?
        } else {
            let mut req = request.clone();
            req.chroma = *chroma;
            palette_from_request(&req, "theme generation (chroma override)")?
        };

        let tokens =
            resolve_tokens_with_strategy(&palette, &template.definition, request.contrast)?;
        let context = GenerationContext {
            mode: request.mode,
            template_name: template.definition.name.clone(),
            chroma: *chroma,
            output_dir: request.output_dir.clone(),
            seed: request.seed.clone(),
        };

        artifacts.extend(
            output_registry
                .generate(targets, &tokens, &context)
                .map_err(CoreError::from)?,
        );
    }

    Ok(artifacts)
}

fn render_from_palette_with_wallpaper(
    request: &GenerationRequest,
    output_registry: &OutputRegistry,
) -> Result<Vec<GeneratedArtifact>, CoreError> {
    use std::collections::BTreeMap;

    if request.targets.is_empty() {
        return Ok(Vec::new());
    }

    let wallpaper = request
        .wallpaper
        .as_deref()
        .ok_or(CoreError::MissingWallpaper {
            operation: "wallpaper theme generation",
        })?;
    let extraction = chromasync_extract::extract_seed_candidates(wallpaper)?;

    // Grouping by (template, chroma) to handle overrides
    let mut groups: BTreeMap<(String, ChromaStrategy), Vec<String>> = BTreeMap::new();

    for target in &request.targets {
        let template_name = request
            .template
            .clone()
            .or_else(|| output_registry.resolve_preferred_template(target))
            .ok_or_else(|| CoreError::MissingTemplate {
                operation: "theme generation",
                target: target.clone(),
            })?;
        let chroma = output_registry
            .resolve_chroma_strategy(target)
            .unwrap_or(request.chroma);

        groups
            .entry((template_name, chroma))
            .or_default()
            .push(target.clone());
    }

    let mut artifacts = Vec::new();

    for ((template_name, chroma), targets) in &groups {
        let template = load_template(template_name, request.mode)?;
        let palette = palette_from_extracted_seeds(&extraction.seeds, request.mode, *chroma)?;

        let tokens =
            resolve_tokens_with_strategy(&palette, &template.definition, request.contrast)?;
        let context = GenerationContext {
            mode: request.mode,
            template_name: template.definition.name.clone(),
            chroma: *chroma,
            output_dir: request.output_dir.clone(),
            seed: request.seed.clone(),
        };

        artifacts.extend(
            output_registry
                .generate(targets, &tokens, &context)
                .map_err(CoreError::from)?,
        );
    }

    Ok(artifacts)
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
    use chromasync_types::{
        ChromaStrategy, ContrastStrategy, GenerationRequest, PaletteFamilyName, ThemeMode,
    };

    use super::{
        CoreError, GeneratedArtifact, RoutedArtifact, WrittenArtifact, export_tokens,
        generate_palette, palette_from_extracted_seeds, preview, write_artifacts,
        write_routed_artifacts,
    };

    #[test]
    fn palette_generation_is_available_without_renderer_code() {
        let palette = generate_palette("#ff6b6b", ThemeMode::Dark, ChromaStrategy::Normal)
            .expect("palette generation should work");

        assert!(palette.families.contains_key(&PaletteFamilyName::Primary));
        assert!(palette.families.contains_key(&PaletteFamilyName::Neutral));
    }

    #[test]
    fn token_export_resolves_built_in_templates() {
        let request = GenerationRequest {
            seed: Some("#4ecdc4".to_owned()),
            wallpaper: None,
            template: Some("minimal".to_owned()),
            mode: ThemeMode::Dark,
            chroma: ChromaStrategy::Normal,
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
            template: Some("brutalist".to_owned()),
            mode: ThemeMode::Dark,
            chroma: ChromaStrategy::Normal,
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
            template: Some("terminal".to_owned()),
            mode: ThemeMode::Dark,
            chroma: ChromaStrategy::Normal,
            contrast: ContrastStrategy::RelativeLuminance,
            targets: vec![
                example_target_path("gtk.toml"),
                "hyprland".to_owned(),
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
    fn gtk4_generation_emits_one_unguarded_stylesheet() {
        let request = GenerationRequest {
            seed: Some("#4ecdc4".to_owned()),
            wallpaper: None,
            template: Some("materialish".to_owned()),
            mode: ThemeMode::Dark,
            chroma: ChromaStrategy::Normal,
            contrast: ContrastStrategy::RelativeLuminance,
            targets: vec!["gtk4".to_owned()],
            output_dir: "chromasync".into(),
        };

        let artifacts = super::generate(&request).expect("GTK4 generation should succeed");
        assert_eq!(artifacts.len(), 2);
        assert_eq!(artifacts[0].file_name, "gtk.css");
        assert_eq!(artifacts[0].content, "@import url(\"chromasync.css\");\n");

        let stylesheet = &artifacts[1];
        assert_eq!(stylesheet.file_name, "chromasync.css");
        assert!(!stylesheet.content.contains("prefers-color-scheme"));
        assert_eq!(
            stylesheet
                .content
                .matches("@define-color window_bg_color")
                .count(),
            1
        );
    }

    #[test]
    fn gtk4_filesystem_template_does_not_invent_an_alternate_mode() {
        let template =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../templates/materialish-dark.toml");
        let request = GenerationRequest {
            seed: Some("#4ecdc4".to_owned()),
            wallpaper: None,
            template: Some(template.display().to_string()),
            mode: ThemeMode::Dark,
            chroma: ChromaStrategy::Normal,
            contrast: ContrastStrategy::RelativeLuminance,
            targets: vec!["gtk4".to_owned()],
            output_dir: "chromasync".into(),
        };

        let artifacts = super::generate(&request).expect("GTK4 generation should succeed");
        let stylesheet = artifacts
            .iter()
            .find(|artifact| artifact.file_name == "chromasync.css")
            .expect("GTK4 stylesheet should be generated");

        assert!(!stylesheet.content.contains("prefers-color-scheme"));
        assert_eq!(
            stylesheet
                .content
                .matches("@define-color window_bg_color")
                .count(),
            1
        );
    }

    #[test]
    fn wallpaper_generation_with_no_targets_does_not_access_the_image() {
        let request = GenerationRequest {
            seed: None,
            wallpaper: Some(wallpaper_fixture("does-not-exist.png")),
            template: Some("minimal".to_owned()),
            mode: ThemeMode::Dark,
            chroma: ChromaStrategy::Normal,
            contrast: ContrastStrategy::RelativeLuminance,
            targets: Vec::new(),
            output_dir: "chromasync".into(),
        };
        let registry = super::OutputRegistry::default();

        let artifacts = super::generate_from_wallpaper_with_output_registry(&request, &registry)
            .expect("empty target generation should be a no-op");

        assert!(artifacts.is_empty());
    }

    #[test]
    fn wallpaper_generation_assigns_metadata_and_multi_seed_families() {
        let extraction =
            chromasync_extract::extract_seed_candidates(&wallpaper_fixture("wallpaper-blocks.png"))
                .expect("wallpaper extraction should succeed");
        let palette = palette_from_extracted_seeds(
            &extraction.seeds,
            ThemeMode::Dark,
            ChromaStrategy::Normal,
        )
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

    fn routed(output_dir: &std::path::Path, file_name: &str, content: &str) -> RoutedArtifact {
        RoutedArtifact {
            artifact: GeneratedArtifact {
                target: "probe".to_owned(),
                file_name: file_name.to_owned(),
                content: content.to_owned(),
            },
            output_dir: output_dir.to_path_buf(),
            force: true,
        }
    }

    fn unforced(output_dir: &std::path::Path, file_name: &str, content: &str) -> RoutedArtifact {
        RoutedArtifact {
            force: false,
            ..routed(output_dir, file_name, content)
        }
    }

    fn set_old_mtime(path: &std::path::Path) -> std::time::SystemTime {
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        std::fs::File::options()
            .write(true)
            .open(path)
            .and_then(|file| file.set_modified(old))
            .expect("mtime should be set");
        old
    }

    fn mtime(path: &std::path::Path) -> std::time::SystemTime {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .expect("mtime should be readable")
    }

    #[test]
    fn routed_writes_report_changes_and_skip_identical_files() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let dir = temp.path().join("out");

        let first =
            write_routed_artifacts(&[routed(&dir, "a.txt", "a"), routed(&dir, "b.txt", "b")])
                .expect("first write should succeed");
        assert!(first.iter().all(|written| written.changed));

        let old = set_old_mtime(&dir.join("a.txt"));

        // `B` has the same length as `b`, so the content comparison decides.
        let second =
            write_routed_artifacts(&[routed(&dir, "a.txt", "a"), routed(&dir, "b.txt", "B")])
                .expect("second write should succeed");
        assert_eq!(
            second,
            [
                WrittenArtifact {
                    path: dir.join("a.txt"),
                    changed: false
                },
                WrittenArtifact {
                    path: dir.join("b.txt"),
                    changed: true
                },
            ]
        );
        assert_eq!(
            mtime(&dir.join("a.txt")),
            old,
            "identical content should not be rewritten"
        );
        assert_eq!(std::fs::read_to_string(dir.join("b.txt")).unwrap(), "B");
    }

    #[test]
    fn identical_existing_files_are_accepted_without_force() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let dir = temp.path();

        write_routed_artifacts(&[unforced(dir, "a.txt", "a")]).expect("first write should succeed");
        let old = set_old_mtime(&dir.join("a.txt"));

        let rerun = write_routed_artifacts(&[unforced(dir, "a.txt", "a")])
            .expect("an identical rerun should succeed without force");
        assert_eq!(
            rerun,
            [WrittenArtifact {
                path: dir.join("a.txt"),
                changed: false
            }]
        );
        assert_eq!(mtime(&dir.join("a.txt")), old);

        let paths = write_artifacts(
            dir,
            &[GeneratedArtifact {
                target: "probe".to_owned(),
                file_name: "a.txt".to_owned(),
                content: "a".to_owned(),
            }],
            false,
        )
        .expect("write_artifacts should accept identical files without force");
        assert_eq!(paths, [dir.join("a.txt")]);
    }

    #[test]
    fn differing_existing_files_are_refused_without_force_before_any_write() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let dir = temp.path();

        // Same length as the new content and different length: both refused.
        for existing in ["xyz", "longer content"] {
            std::fs::write(dir.join("b.txt"), existing).expect("existing file should be written");

            let result = write_routed_artifacts(&[
                unforced(dir, "a.txt", "a"),
                unforced(dir, "b.txt", "new"),
            ]);
            assert!(
                matches!(&result, Err(CoreError::ArtifactExists { path }) if path == &dir.join("b.txt")),
                "{result:?}"
            );
            assert!(
                !dir.join("a.txt").exists(),
                "nothing should be written when a later check fails"
            );
            assert_eq!(
                std::fs::read_to_string(dir.join("b.txt")).unwrap(),
                existing
            );
        }
    }

    #[test]
    fn non_file_destinations_are_never_treated_as_identical() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let dir = temp.path();
        std::fs::create_dir(dir.join("a.txt")).expect("directory should be created");

        assert!(matches!(
            write_routed_artifacts(&[unforced(dir, "a.txt", "")]),
            Err(CoreError::ArtifactExists { .. })
        ));
        assert!(matches!(
            write_routed_artifacts(&[routed(dir, "a.txt", "")]),
            Err(CoreError::WriteArtifact { .. })
        ));
    }

    #[test]
    fn colliding_destinations_are_refused_even_when_identical() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let dir = temp.path();

        assert!(matches!(
            write_routed_artifacts(&[routed(dir, "a.txt", "a"), routed(dir, "a.txt", "a")]),
            Err(CoreError::ArtifactCollision { .. })
        ));
        assert!(!dir.join("a.txt").exists());
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
