use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use chromasync_color::{
    ColorError, contrast_score, meets_contrast_threshold, resolve_color_from_components,
    resolve_family_color, select_readable_color, select_readable_color_with_strategy,
};
use chromasync_types::{
    ContrastStrategy, GeneratedPalette, HexColor, PaletteFamilyName, SemanticTokenName,
    SemanticTokens, TemplateDefinition, TemplateTokenRule, ThemeMode, ThemePack,
};
use directories::ProjectDirs;
use serde::Deserialize;
use thiserror::Error;

const BUILTIN_TEMPLATES: [(&str, &str); 8] = [
    (
        "minimal-dark.toml",
        include_str!("../../../templates/minimal-dark.toml"),
    ),
    (
        "minimal-light.toml",
        include_str!("../../../templates/minimal-light.toml"),
    ),
    (
        "brutalist-dark.toml",
        include_str!("../../../templates/brutalist-dark.toml"),
    ),
    (
        "brutalist-light.toml",
        include_str!("../../../templates/brutalist-light.toml"),
    ),
    (
        "terminal-dark.toml",
        include_str!("../../../templates/terminal-dark.toml"),
    ),
    (
        "terminal-light.toml",
        include_str!("../../../templates/terminal-light.toml"),
    ),
    (
        "materialish-dark.toml",
        include_str!("../../../templates/materialish-dark.toml"),
    ),
    (
        "materialish-light.toml",
        include_str!("../../../templates/materialish-light.toml"),
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
    BuiltIn(&'static str),
    Filesystem(PathBuf),
    UserConfig(PathBuf),
    Pack { pack: String, path: PathBuf },
}

impl TemplateSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::BuiltIn(_) => "built-in",
            Self::Filesystem(_) => "filesystem",
            Self::UserConfig(_) => "user-config",
            Self::Pack { .. } => "pack",
        }
    }

    pub fn location(&self) -> String {
        match self {
            Self::BuiltIn(file_name) => (*file_name).to_owned(),
            Self::Filesystem(path) | Self::UserConfig(path) | Self::Pack { path, .. } => {
                path.display().to_string()
            }
        }
    }

    pub fn pack_name(&self) -> Option<&str> {
        match self {
            Self::Pack { pack, .. } => Some(pack.as_str()),
            _ => None,
        }
    }

    const fn precedence(&self) -> u8 {
        match self {
            Self::BuiltIn(_) => 0,
            Self::Pack { .. } => 1,
            Self::UserConfig(_) => 2,
            Self::Filesystem(_) => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListedTemplate {
    pub definition: TemplateDefinition,
    pub source: TemplateSource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTemplateDefinition {
    name: String,
    mode: ThemeMode,
    description: Option<String>,
    #[serde(default)]
    tokens: BTreeMap<String, RawTemplateTokenRule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTemplateTokenRule {
    family: String,
    tone: f32,
    chroma: Option<f32>,
    chroma_scale: Option<f32>,
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("failed to parse template '{name}' from {source}: {error}")]
    Parse {
        name: String,
        source: String,
        #[source]
        error: Box<toml::de::Error>,
    },
    #[error("failed to read templates directory '{path}': {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read template file '{path}': {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("template '{requested}' was not found")]
    TemplateNotFound { requested: String },
    #[error(
        "template '{name}' ({mode}) is defined multiple times: first at {first_source}, second at {second_source}"
    )]
    DuplicateTemplateName {
        name: String,
        mode: ThemeMode,
        first_source: String,
        second_source: String,
    },
    #[error("template '{template}' uses an unknown token '{token}'")]
    UnknownTokenName { template: String, token: String },
    #[error("template '{template}' token '{token}' uses an unknown family '{family}'")]
    UnknownFamily {
        template: String,
        token: String,
        family: String,
    },
    #[error("template '{template}' is missing required tokens: {missing}")]
    MissingRequiredTokens { template: String, missing: String },
    #[error("template '{template}' token '{token}' uses invalid tone {tone}; expected 0.0..=1.0")]
    InvalidTone {
        template: String,
        token: String,
        tone: f32,
    },
    #[error(
        "template '{template}' token '{token}' uses invalid chroma {chroma}; expected a finite value >= 0.0"
    )]
    InvalidChroma {
        template: String,
        token: String,
        chroma: f32,
    },
    #[error(
        "template '{template}' token '{token}' uses invalid chroma_scale {chroma_scale}; expected a finite value >= 0.0"
    )]
    InvalidChromaScale {
        template: String,
        token: String,
        chroma_scale: f32,
    },
    #[error("palette is missing family '{family}' required by the template")]
    MissingPaletteFamily { family: PaletteFamilyName },
    #[error(transparent)]
    Color(#[from] ColorError),
}

pub fn built_in_templates() -> Result<Vec<ListedTemplate>, TemplateError> {
    BUILTIN_TEMPLATES
        .iter()
        .map(|(file_name, content)| {
            let definition =
                parse_template(file_name, content, format!("built-in template {file_name}"))?;
            Ok(ListedTemplate {
                definition,
                source: TemplateSource::BuiltIn(file_name),
            })
        })
        .collect()
}

pub fn user_templates_dir() -> Option<PathBuf> {
    ProjectDirs::from("io", "chromasync", "chromasync")
        .map(|dirs| dirs.config_dir().join("templates"))
}

pub fn template_from_file(path: &Path, user_config: bool) -> Result<ListedTemplate, TemplateError> {
    let source = if user_config {
        TemplateSource::UserConfig(path.to_path_buf())
    } else {
        TemplateSource::Filesystem(path.to_path_buf())
    };

    template_from_file_with_source(path, source)
}

fn template_from_file_with_source(
    path: &Path,
    source: TemplateSource,
) -> Result<ListedTemplate, TemplateError> {
    let content = fs::read_to_string(path).map_err(|source| TemplateError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let definition = parse_template(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("template.toml"),
        &content,
        path.display().to_string(),
    )?;

    Ok(ListedTemplate { definition, source })
}

pub fn templates_from_dir(
    path: &Path,
    user_config: bool,
) -> Result<Vec<ListedTemplate>, TemplateError> {
    let source = if user_config {
        TemplateSource::UserConfig(path.to_path_buf())
    } else {
        TemplateSource::Filesystem(path.to_path_buf())
    };

    templates_from_dir_with_source(path, &source)
}

fn templates_from_dir_with_source(
    path: &Path,
    source: &TemplateSource,
) -> Result<Vec<ListedTemplate>, TemplateError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut templates = Vec::new();

    for entry in fs::read_dir(path).map_err(|source| TemplateError::ReadDir {
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| TemplateError::ReadDir {
            path: path.to_path_buf(),
            source,
        })?;
        let entry_path = entry.path();

        if entry_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("toml")
        {
            continue;
        }

        let source = match source {
            TemplateSource::Pack { pack, .. } => TemplateSource::Pack {
                pack: pack.clone(),
                path: entry_path.clone(),
            },
            TemplateSource::UserConfig(_) => TemplateSource::UserConfig(entry_path.clone()),
            TemplateSource::Filesystem(_) => TemplateSource::Filesystem(entry_path.clone()),
            TemplateSource::BuiltIn(_) => TemplateSource::Filesystem(entry_path.clone()),
        };
        templates.push(template_from_file_with_source(&entry_path, source)?);
    }

    templates.sort_by(|left, right| left.definition.name.cmp(&right.definition.name));

    Ok(templates)
}

pub fn list_templates() -> Result<Vec<ListedTemplate>, TemplateError> {
    list_templates_with_packs(&[])
}

pub fn list_templates_with_packs(
    packs: &[ThemePack],
) -> Result<Vec<ListedTemplate>, TemplateError> {
    let mut templates = built_in_templates()?;

    if let Some(path) = user_templates_dir() {
        templates.extend(templates_from_dir(&path, true)?);
    }

    for pack in packs {
        for path in &pack.template_dirs {
            templates.extend(templates_from_dir_with_source(
                path,
                &TemplateSource::Pack {
                    pack: pack.name.clone(),
                    path: path.clone(),
                },
            )?);
        }
    }

    templates.sort_by(|left, right| {
        left.definition
            .name
            .cmp(&right.definition.name)
            .then(left.source.precedence().cmp(&right.source.precedence()))
            .then(left.source.location().cmp(&right.source.location()))
    });
    validate_pack_template_collisions(&templates)?;

    Ok(templates)
}

pub fn load_template(requested: &str, mode: ThemeMode) -> Result<ListedTemplate, TemplateError> {
    load_template_with_packs(requested, mode, &[])
}

pub fn load_template_with_packs(
    requested: &str,
    mode: ThemeMode,
    packs: &[ThemePack],
) -> Result<ListedTemplate, TemplateError> {
    if looks_like_path(requested) || Path::new(requested).exists() {
        return template_from_file(Path::new(requested), false);
    }

    list_templates_with_packs(packs)?
        .into_iter()
        .filter(|template| template.definition.name == requested)
        .max_by_key(|template| {
            (
                u8::from(template.definition.mode == mode),
                template.source.precedence(),
            )
        })
        .ok_or_else(|| TemplateError::TemplateNotFound {
            requested: requested.to_owned(),
        })
}

pub fn pack_templates(pack: &ThemePack) -> Result<Vec<ListedTemplate>, TemplateError> {
    let mut templates = Vec::new();

    for path in &pack.template_dirs {
        templates.extend(templates_from_dir_with_source(
            path,
            &TemplateSource::Pack {
                pack: pack.name.clone(),
                path: path.clone(),
            },
        )?);
    }

    templates.sort_by(|left, right| {
        left.definition
            .name
            .cmp(&right.definition.name)
            .then(
                left.definition
                    .mode
                    .as_str()
                    .cmp(right.definition.mode.as_str()),
            )
            .then(left.source.location().cmp(&right.source.location()))
    });
    validate_pack_template_collisions(&templates)?;

    Ok(templates)
}

fn validate_pack_template_collisions(templates: &[ListedTemplate]) -> Result<(), TemplateError> {
    let mut seen: BTreeMap<(String, String), &ListedTemplate> = BTreeMap::new();

    for template in templates {
        let key = (
            template.definition.name.clone(),
            template.definition.mode.as_str().to_owned(),
        );

        if let Some(previous) = seen.get(&key) {
            if previous.source.pack_name().is_some() || template.source.pack_name().is_some() {
                return Err(TemplateError::DuplicateTemplateName {
                    name: template.definition.name.clone(),
                    mode: template.definition.mode,
                    first_source: previous.source.location(),
                    second_source: template.source.location(),
                });
            }
        } else {
            seen.insert(key, template);
        }
    }

    Ok(())
}

pub fn resolve_tokens(
    palette: &GeneratedPalette,
    template: &TemplateDefinition,
) -> Result<SemanticTokens, TemplateError> {
    resolve_tokens_with_strategy(palette, template, ContrastStrategy::RelativeLuminance)
}

pub fn resolve_tokens_with_strategy(
    palette: &GeneratedPalette,
    template: &TemplateDefinition,
    contrast_strategy: ContrastStrategy,
) -> Result<SemanticTokens, TemplateError> {
    let mut resolved = BTreeMap::new();

    for token_name in SemanticTokenName::ALL {
        let rule = template
            .tokens
            .get(&token_name)
            .expect("template validation should guarantee the canonical token set");
        let family =
            palette
                .families
                .get(&rule.family)
                .ok_or(TemplateError::MissingPaletteFamily {
                    family: rule.family,
                })?;
        let chroma = rule.chroma.unwrap_or(family.base_chroma) * rule.chroma_scale.unwrap_or(1.0);
        let color = resolve_color_from_components(family.hue, chroma, rule.tone)?;

        resolved.insert(token_name, color);
    }

    let neutral = palette.families.get(&PaletteFamilyName::Neutral).ok_or(
        TemplateError::MissingPaletteFamily {
            family: PaletteFamilyName::Neutral,
        },
    )?;
    let light_fallback = resolve_family_color(neutral, 0.98)?;
    let dark_fallback = resolve_family_color(neutral, 0.06)?;
    let bg = resolved
        .get(&SemanticTokenName::Bg)
        .cloned()
        .expect("bg token should be present");
    let accent = resolved
        .get(&SemanticTokenName::Accent)
        .cloned()
        .expect("accent token should be present");
    let text = ensure_preferred_readable(
        resolved
            .get(&SemanticTokenName::Text)
            .expect("text token should be present"),
        &bg,
        &[light_fallback.clone(), dark_fallback.clone()],
        contrast_strategy,
    )?;

    let mut accent_fg_fallbacks = vec![text.clone()];
    if let Some(primary) = palette.families.get(&PaletteFamilyName::Primary) {
        if let Ok(primary_light) = resolve_family_color(primary, 0.95) {
            accent_fg_fallbacks.push(primary_light);
        }
        if let Ok(primary_dark) = resolve_family_color(primary, 0.10) {
            accent_fg_fallbacks.push(primary_dark);
        }
    }
    accent_fg_fallbacks.push(light_fallback);
    accent_fg_fallbacks.push(dark_fallback);

    let accent_fg = ensure_preferred_readable(
        resolved
            .get(&SemanticTokenName::AccentFg)
            .expect("accent_fg token should be present"),
        &accent,
        &accent_fg_fallbacks,
        contrast_strategy,
    )?;

    resolved.insert(SemanticTokenName::Text, text);
    resolved.insert(SemanticTokenName::AccentFg, accent_fg);

    Ok(semantic_tokens_from_map(&resolved))
}

fn parse_template(
    name: &str,
    content: &str,
    source: String,
) -> Result<TemplateDefinition, TemplateError> {
    let raw: RawTemplateDefinition =
        toml::from_str(content).map_err(|error| TemplateError::Parse {
            name: name.to_owned(),
            source,
            error: Box::new(error),
        })?;

    validate_template(raw)
}

fn validate_template(raw: RawTemplateDefinition) -> Result<TemplateDefinition, TemplateError> {
    let template_name = raw.name.clone();
    let mut tokens = BTreeMap::new();

    for (token_name, raw_rule) in raw.tokens {
        let token = SemanticTokenName::from_str(&token_name).map_err(|_| {
            TemplateError::UnknownTokenName {
                template: template_name.clone(),
                token: token_name.clone(),
            }
        })?;
        let family = PaletteFamilyName::from_str(&raw_rule.family).map_err(|_| {
            TemplateError::UnknownFamily {
                template: template_name.clone(),
                token: token_name.clone(),
                family: raw_rule.family.clone(),
            }
        })?;

        validate_tone(&template_name, &token_name, raw_rule.tone)?;

        if let Some(chroma) = raw_rule.chroma {
            validate_chroma(&template_name, &token_name, chroma)?;
        }

        if let Some(chroma_scale) = raw_rule.chroma_scale {
            validate_chroma_scale(&template_name, &token_name, chroma_scale)?;
        }

        tokens.insert(
            token,
            TemplateTokenRule {
                family,
                tone: raw_rule.tone,
                chroma: raw_rule.chroma,
                chroma_scale: raw_rule.chroma_scale,
            },
        );
    }

    let missing = SemanticTokenName::ALL
        .into_iter()
        .filter(|token| !tokens.contains_key(token))
        .map(SemanticTokenName::as_str)
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        return Err(TemplateError::MissingRequiredTokens {
            template: template_name,
            missing: missing.join(", "),
        });
    }

    Ok(TemplateDefinition {
        name: raw.name,
        mode: raw.mode,
        description: raw.description,
        tokens,
    })
}

fn validate_tone(template: &str, token: &str, tone: f32) -> Result<(), TemplateError> {
    if tone.is_finite() && (0.0..=1.0).contains(&tone) {
        Ok(())
    } else {
        Err(TemplateError::InvalidTone {
            template: template.to_owned(),
            token: token.to_owned(),
            tone,
        })
    }
}

fn validate_chroma(template: &str, token: &str, chroma: f32) -> Result<(), TemplateError> {
    if chroma.is_finite() && chroma >= 0.0 {
        Ok(())
    } else {
        Err(TemplateError::InvalidChroma {
            template: template.to_owned(),
            token: token.to_owned(),
            chroma,
        })
    }
}

fn validate_chroma_scale(
    template: &str,
    token: &str,
    chroma_scale: f32,
) -> Result<(), TemplateError> {
    if chroma_scale.is_finite() && chroma_scale >= 0.0 {
        Ok(())
    } else {
        Err(TemplateError::InvalidChromaScale {
            template: template.to_owned(),
            token: token.to_owned(),
            chroma_scale,
        })
    }
}

fn ensure_preferred_readable(
    preferred: &str,
    background: &str,
    fallbacks: &[HexColor],
    contrast_strategy: ContrastStrategy,
) -> Result<HexColor, TemplateError> {
    let preferred_score = contrast_score(preferred, background, contrast_strategy)?;

    if meets_contrast_threshold(preferred_score, contrast_strategy) {
        return Ok(preferred.to_owned());
    }

    let mut candidates = Vec::with_capacity(fallbacks.len() + 1);
    candidates.push(preferred.to_owned());

    for fallback in fallbacks {
        if !candidates.iter().any(|candidate| candidate == fallback) {
            candidates.push(fallback.clone());
        }
    }

    let selection = match contrast_strategy {
        ContrastStrategy::RelativeLuminance => select_readable_color(background, &candidates)?,
        ContrastStrategy::ApcaExperimental => {
            select_readable_color_with_strategy(background, &candidates, contrast_strategy)?
        }
    };

    Ok(selection.hex)
}

fn semantic_tokens_from_map(tokens: &BTreeMap<SemanticTokenName, HexColor>) -> SemanticTokens {
    SemanticTokens {
        bg: token(tokens, SemanticTokenName::Bg),
        bg_secondary: token(tokens, SemanticTokenName::BgSecondary),
        surface: token(tokens, SemanticTokenName::Surface),
        surface_elevated: token(tokens, SemanticTokenName::SurfaceElevated),
        text: token(tokens, SemanticTokenName::Text),
        text_muted: token(tokens, SemanticTokenName::TextMuted),
        border: token(tokens, SemanticTokenName::Border),
        border_strong: token(tokens, SemanticTokenName::BorderStrong),
        accent: token(tokens, SemanticTokenName::Accent),
        accent_hover: token(tokens, SemanticTokenName::AccentHover),
        accent_active: token(tokens, SemanticTokenName::AccentActive),
        accent_fg: token(tokens, SemanticTokenName::AccentFg),
        selection: token(tokens, SemanticTokenName::Selection),
        link: token(tokens, SemanticTokenName::Link),
        success: token(tokens, SemanticTokenName::Success),
        warning: token(tokens, SemanticTokenName::Warning),
        error: token(tokens, SemanticTokenName::Error),
    }
}

fn token(tokens: &BTreeMap<SemanticTokenName, HexColor>, name: SemanticTokenName) -> HexColor {
    tokens
        .get(&name)
        .cloned()
        .expect("resolved tokens should include the full canonical token set")
}

fn looks_like_path(value: &str) -> bool {
    let path = Path::new(value);

    path.is_absolute()
        || value.contains(std::path::MAIN_SEPARATOR)
        || path.extension().and_then(|extension| extension.to_str()) == Some("toml")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use chromasync_color::{contrast_ratio, generate_palette};

    use super::{
        TemplateError, TemplateSource, built_in_templates, load_template, parse_template,
        resolve_tokens,
    };
    use chromasync_types::{ChromaStrategy, PaletteFamilyName, SemanticTokenName, ThemeMode};

    #[test]
    fn built_in_templates_parse_and_define_all_canonical_tokens() {
        let templates = built_in_templates().expect("built-in templates should parse");

        assert_eq!(templates.len(), 8);
        assert!(
            templates
                .iter()
                .any(|template| template.definition.name == "minimal")
        );
        assert_eq!(
            templates
                .iter()
                .filter(|template| template.definition.name == "minimal")
                .count(),
            2
        );
        assert!(
            templates
                .iter()
                .all(|template| template.definition.tokens.len() == SemanticTokenName::ALL.len())
        );
    }

    #[test]
    fn load_template_by_name_uses_embedded_assets() {
        let template = load_template("minimal", ThemeMode::Dark).expect("template should load");

        assert_eq!(template.definition.name, "minimal");
        assert_eq!(template.definition.mode, ThemeMode::Dark);
        assert!(matches!(template.source, TemplateSource::BuiltIn(_)));
    }

    #[test]
    fn load_template_by_name_prefers_requested_mode() {
        let template = load_template("minimal", ThemeMode::Light).expect("template should load");

        assert_eq!(template.definition.name, "minimal");
        assert_eq!(template.definition.mode, ThemeMode::Light);
        assert!(matches!(template.source, TemplateSource::BuiltIn(_)));
    }

    #[test]
    fn load_template_by_path_uses_filesystem_source() {
        let path = temp_file_path("template-path");
        fs::write(&path, include_str!("../../../templates/minimal-dark.toml"))
            .expect("temp template should be written");

        let template = load_template(
            path.to_str().expect("path should be utf-8"),
            ThemeMode::Dark,
        )
        .expect("template should load from path");

        assert!(matches!(template.source, TemplateSource::Filesystem(_)));

        fs::remove_file(path).expect("temp template should be removed");
    }

    #[test]
    fn built_in_templates_resolve_to_semantic_tokens() {
        let palette = generate_palette("#4ecdc4", ThemeMode::Light, ChromaStrategy::Normal)
            .expect("palette should build");
        let template =
            load_template("materialish", ThemeMode::Light).expect("template should load");

        let tokens = resolve_tokens(&palette, &template.definition)
            .expect("template should resolve to semantic tokens");

        assert!(tokens.bg.starts_with('#'));
        assert!(tokens.accent.starts_with('#'));
        assert!(tokens.text.starts_with('#'));
        assert!(
            contrast_ratio(&tokens.text, &tokens.bg).expect("contrast should compute")
                >= chromasync_color::MIN_CONTRAST_RATIO
        );
        assert!(
            contrast_ratio(&tokens.accent_fg, &tokens.accent).expect("contrast should compute")
                >= chromasync_color::MIN_CONTRAST_RATIO
        );
    }

    #[test]
    fn invalid_templates_report_missing_required_tokens() {
        let error = parse_template(
            "broken.toml",
            r#"
name = "broken"
mode = "dark"

[tokens.bg]
family = "neutral"
tone = 0.1
"#,
            "test fixture".to_owned(),
        )
        .expect_err("template should fail validation");

        assert!(matches!(error, TemplateError::MissingRequiredTokens { .. }));
    }

    #[test]
    fn invalid_templates_report_unknown_families() {
        let error = parse_template(
            "broken.toml",
            r#"
name = "broken"
mode = "dark"

[tokens.bg]
family = "bogus"
tone = 0.1

[tokens.bg_secondary]
family = "neutral"
tone = 0.14

[tokens.surface]
family = "neutral"
tone = 0.16

[tokens.surface_elevated]
family = "neutral"
tone = 0.18

[tokens.text]
family = "neutral"
tone = 0.94

[tokens.text_muted]
family = "neutral_variant"
tone = 0.8

[tokens.border]
family = "neutral_variant"
tone = 0.24

[tokens.border_strong]
family = "neutral_variant"
tone = 0.34

[tokens.accent]
family = "primary"
tone = 0.7

[tokens.accent_hover]
family = "primary"
tone = 0.76

[tokens.accent_active]
family = "primary"
tone = 0.64

[tokens.accent_fg]
family = "neutral"
tone = 0.98

[tokens.selection]
family = "primary"
tone = 0.32

[tokens.link]
family = "info"
tone = 0.74

[tokens.success]
family = "success"
tone = 0.72

[tokens.warning]
family = "warning"
tone = 0.74

[tokens.error]
family = "error"
tone = 0.72
"#,
            "test fixture".to_owned(),
        )
        .expect_err("template should fail validation");

        assert!(matches!(error, TemplateError::UnknownFamily { .. }));
    }

    #[test]
    fn resolved_templates_stay_bound_to_palette_families() {
        let palette = generate_palette("#ff6b6b", ThemeMode::Light, ChromaStrategy::Normal)
            .expect("palette should build");
        let template = load_template("minimal", ThemeMode::Light).expect("template should load");

        let tokens = resolve_tokens(&palette, &template.definition)
            .expect("template should resolve to semantic tokens");

        assert!(palette.families.contains_key(&PaletteFamilyName::Primary));
        assert_ne!(tokens.bg, tokens.accent);
    }

    fn temp_file_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "chromasync-template-{label}-{}-{unique}.toml",
            std::process::id()
        ))
    }
}
