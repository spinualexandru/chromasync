use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Mutex,
};

use chromasync_types::{
    GeneratedArtifact, GenerationContext, SemanticTokenName, SemanticTokens, ThemePack,
};
use directories::ProjectDirs;
use serde::Deserialize;

use crate::{
    ArtifactGenerator, BUILTIN_TARGETS, RendererError, alacritty::AlacrittyRenderer, hyprland_rgba,
    kitty::KittyRenderer, normalized_hex_without_hash,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetSource {
    BuiltIn(&'static str),
    Filesystem(PathBuf),
    UserConfig(PathBuf),
    Pack { pack: String, path: PathBuf },
}

impl TargetSource {
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
            Self::BuiltIn(name) => (*name).to_owned(),
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

    pub(crate) const fn precedence(&self) -> u8 {
        match self {
            Self::BuiltIn(_) => 0,
            Self::Pack { .. } => 1,
            Self::UserConfig(_) => 2,
            Self::Filesystem(_) => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedTarget {
    pub name: String,
    pub source: TargetSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSpec {
    pub file_name: String,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    pub name: String,
    pub description: Option<String>,
    pub extends: Option<String>,
    pub artifacts: Vec<ArtifactSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledTarget {
    pub name: String,
    pub artifacts: Vec<CompiledArtifactSpec>,
    pub source: TargetSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifactSpec {
    pub file_name: String,
    template: CompiledTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledTemplate {
    segments: Vec<CompiledSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CompiledSegment {
    Literal(String),
    Placeholder(CompiledPlaceholder),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledPlaceholder {
    value: PlaceholderValue,
    transforms: Vec<PlaceholderTransform>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlaceholderValue {
    Token(SemanticTokenName),
    Context(ContextField),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlaceholderTransform {
    HexNoHash,
    Rgba { alpha: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextField {
    Mode,
    TemplateName,
    OutputDir,
    Seed,
}

#[derive(Debug, Clone)]
struct LoadedTarget {
    spec: TargetSpec,
    source: TargetSource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTargetSpec {
    name: String,
    description: Option<String>,
    extends: Option<String>,
    #[serde(default)]
    artifacts: Vec<RawArtifactSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawArtifactSpec {
    file_name: String,
    template: String,
}

pub struct RendererRegistry {
    renderers: BTreeMap<String, Box<dyn ArtifactGenerator>>,
}

impl RendererRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            renderers: BTreeMap::new(),
        };

        registry.register(KittyRenderer);
        registry.register(AlacrittyRenderer);

        registry
    }

    pub fn register<G>(&mut self, generator: G)
    where
        G: ArtifactGenerator + 'static,
    {
        let name = generator.name().to_owned();
        self.renderers.insert(name, Box::new(generator));
    }

    pub fn get(&self, name: &str) -> Option<&dyn ArtifactGenerator> {
        self.renderers.get(name).map(|generator| generator.as_ref())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.renderers.contains_key(name)
    }

    pub fn built_in_names(&self) -> Vec<String> {
        BUILTIN_TARGETS
            .iter()
            .map(|target| target.as_str().to_owned())
            .collect()
    }

    pub fn built_in_name_set(&self) -> BTreeSet<String> {
        self.built_in_names().into_iter().collect()
    }

    pub fn list_targets(&self) -> Vec<ListedTarget> {
        BUILTIN_TARGETS
            .iter()
            .filter_map(|target| {
                self.renderers.get(target.as_str()).map(|_| ListedTarget {
                    name: target.as_str().to_owned(),
                    source: TargetSource::BuiltIn(target.as_str()),
                })
            })
            .collect()
    }
}

impl Default for RendererRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct TargetRegistry {
    targets: BTreeMap<String, CompiledTarget>,
    listed: Vec<ListedTarget>,
}

impl TargetRegistry {
    pub fn discover(built_in_names: &BTreeSet<String>) -> Result<Self, RendererError> {
        Self::discover_with_packs(&[], built_in_names)
    }

    pub fn discover_with_packs(
        packs: &[ThemePack],
        built_in_names: &BTreeSet<String>,
    ) -> Result<Self, RendererError> {
        let mut loaded = Vec::new();

        if let Some(path) = user_targets_dir() {
            loaded.extend(targets_from_dir(&path, true)?);
        }

        loaded.extend(loaded_targets_from_packs(packs)?);

        Self::from_loaded_targets(loaded, built_in_names)
    }

    pub fn from_dir(
        path: &Path,
        user_config: bool,
        built_in_names: &BTreeSet<String>,
    ) -> Result<Self, RendererError> {
        let loaded = targets_from_dir(path, user_config)?;
        Self::from_loaded_targets(loaded, built_in_names)
    }

    pub fn from_theme_packs(
        packs: &[ThemePack],
        built_in_names: &BTreeSet<String>,
    ) -> Result<Self, RendererError> {
        let loaded = loaded_targets_from_packs(packs)?;

        Self::from_loaded_targets(loaded, built_in_names)
    }

    pub fn get(&self, name: &str) -> Option<&CompiledTarget> {
        self.targets.get(name)
    }

    pub fn list_targets(&self) -> &[ListedTarget] {
        &self.listed
    }

    fn empty() -> Self {
        Self {
            targets: BTreeMap::new(),
            listed: Vec::new(),
        }
    }

    fn from_loaded_targets(
        loaded: Vec<LoadedTarget>,
        built_in_names: &BTreeSet<String>,
    ) -> Result<Self, RendererError> {
        let targets = compile_registry_targets(loaded, built_in_names)?;
        let mut listed = targets
            .values()
            .map(|target| ListedTarget {
                name: target.name.clone(),
                source: target.source.clone(),
            })
            .collect::<Vec<_>>();
        listed.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.source.precedence().cmp(&right.source.precedence()))
                .then(left.source.location().cmp(&right.source.location()))
        });

        Ok(Self { targets, listed })
    }
}

impl Default for TargetRegistry {
    fn default() -> Self {
        Self::empty()
    }
}

pub struct OutputRegistry {
    built_in: RendererRegistry,
    user_defined: TargetRegistry,
    path_cache: Mutex<BTreeMap<PathBuf, CompiledTarget>>,
}

impl OutputRegistry {
    pub fn discover() -> Result<Self, RendererError> {
        Self::discover_with_packs(&[])
    }

    pub fn discover_with_packs(packs: &[ThemePack]) -> Result<Self, RendererError> {
        let built_in = RendererRegistry::new();
        let user_defined =
            TargetRegistry::discover_with_packs(packs, &built_in.built_in_name_set())?;

        Ok(Self {
            built_in,
            user_defined,
            path_cache: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn resolve(&self, name: &str) -> Option<&dyn ArtifactGenerator> {
        if let Some(target) = self.user_defined.get(name) {
            return Some(target);
        }

        self.built_in.get(name)
    }

    pub fn list_targets(&self) -> Vec<ListedTarget> {
        let mut targets = self.built_in.list_targets();
        targets.extend(self.user_defined.list_targets().iter().cloned());
        targets.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.source.precedence().cmp(&right.source.precedence()))
                .then(left.source.location().cmp(&right.source.location()))
        });
        targets
    }

    pub fn generate(
        &self,
        requested: &[String],
        tokens: &SemanticTokens,
        ctx: &GenerationContext,
    ) -> Result<Vec<GeneratedArtifact>, RendererError> {
        let requested = normalized_target_requests(requested, &self.built_in);
        let mut artifacts = Vec::new();

        for name in requested {
            if let Some(generator) = self.resolve(&name) {
                artifacts.extend(generator.generate(tokens, ctx)?);
                continue;
            }

            let compiled = self.load_path_target(Path::new(&name))?;
            artifacts.extend(compiled.generate(tokens, ctx)?);
        }

        Ok(artifacts)
    }

    fn load_path_target(&self, path: &Path) -> Result<CompiledTarget, RendererError> {
        if !looks_like_path(path.to_string_lossy().as_ref()) && !path.exists() {
            return Err(RendererError::UnknownTarget {
                requested: path.display().to_string(),
            });
        }

        let cache_key = path.to_path_buf();

        if let Some(cached) = self
            .path_cache
            .lock()
            .expect("path target cache should not be poisoned")
            .get(&cache_key)
            .cloned()
        {
            return Ok(cached);
        }

        let loaded = target_from_file(path, false)?;

        if self.built_in.contains(&loaded.spec.name) {
            return Err(RendererError::TargetNameCollidesWithBuiltIn {
                name: loaded.spec.name,
            });
        }

        let base = match loaded.spec.extends.as_deref() {
            Some(base) if self.built_in.contains(base) => {
                return Err(RendererError::BuiltInTargetInheritance {
                    target: loaded.spec.name,
                    base: base.to_owned(),
                });
            }
            Some(base) => self.user_defined.get(base).cloned().ok_or_else(|| {
                RendererError::UnknownBaseTarget {
                    target: loaded.spec.name.clone(),
                    base: base.to_owned(),
                }
            })?,
            None => CompiledTarget {
                name: loaded.spec.name.clone(),
                artifacts: Vec::new(),
                source: loaded.source.clone(),
            },
        };

        let compiled = compile_loaded_target(&loaded, &base)?;
        self.path_cache
            .lock()
            .expect("path target cache should not be poisoned")
            .insert(cache_key, compiled.clone());

        Ok(compiled)
    }
}

impl Default for OutputRegistry {
    fn default() -> Self {
        Self {
            built_in: RendererRegistry::new(),
            user_defined: TargetRegistry::default(),
            path_cache: Mutex::new(BTreeMap::new()),
        }
    }
}

impl ArtifactGenerator for CompiledTarget {
    fn name(&self) -> &str {
        &self.name
    }

    fn generate(
        &self,
        tokens: &SemanticTokens,
        ctx: &GenerationContext,
    ) -> Result<Vec<GeneratedArtifact>, RendererError> {
        self.artifacts
            .iter()
            .map(|artifact| {
                Ok(GeneratedArtifact {
                    target: self.name.clone(),
                    file_name: artifact.file_name.clone(),
                    content: artifact.template.render(tokens, ctx),
                })
            })
            .collect()
    }
}

pub fn user_targets_dir() -> Option<PathBuf> {
    ProjectDirs::from("io", "chromasync", "chromasync")
        .map(|dirs| dirs.config_dir().join("targets"))
}

fn targets_from_dir(path: &Path, user_config: bool) -> Result<Vec<LoadedTarget>, RendererError> {
    let source = if user_config {
        TargetSource::UserConfig(path.to_path_buf())
    } else {
        TargetSource::Filesystem(path.to_path_buf())
    };

    targets_from_dir_with_source(path, &source)
}

fn targets_from_dir_with_source(
    path: &Path,
    source: &TargetSource,
) -> Result<Vec<LoadedTarget>, RendererError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut loaded = Vec::new();

    for entry in fs::read_dir(path).map_err(|source| RendererError::ReadTargetsDir {
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| RendererError::ReadTargetsDir {
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
            TargetSource::Pack { pack, .. } => TargetSource::Pack {
                pack: pack.clone(),
                path: entry_path.clone(),
            },
            TargetSource::UserConfig(_) => TargetSource::UserConfig(entry_path.clone()),
            TargetSource::Filesystem(_) => TargetSource::Filesystem(entry_path.clone()),
            TargetSource::BuiltIn(_) => TargetSource::Filesystem(entry_path.clone()),
        };
        loaded.push(target_from_file_with_source(&entry_path, source)?);
    }

    loaded.sort_by(|left, right| left.spec.name.cmp(&right.spec.name));

    Ok(loaded)
}

fn target_from_file(path: &Path, user_config: bool) -> Result<LoadedTarget, RendererError> {
    let source = if user_config {
        TargetSource::UserConfig(path.to_path_buf())
    } else {
        TargetSource::Filesystem(path.to_path_buf())
    };

    target_from_file_with_source(path, source)
}

fn target_from_file_with_source(
    path: &Path,
    source: TargetSource,
) -> Result<LoadedTarget, RendererError> {
    let content = fs::read_to_string(path).map_err(|source| RendererError::ReadTargetFile {
        path: path.to_path_buf(),
        source,
    })?;
    let spec = parse_target(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("target.toml"),
        &content,
        path.display().to_string(),
    )?;

    Ok(LoadedTarget { spec, source })
}

fn loaded_targets_from_packs(packs: &[ThemePack]) -> Result<Vec<LoadedTarget>, RendererError> {
    let mut loaded = Vec::new();

    for pack in packs {
        for path in &pack.target_dirs {
            loaded.extend(targets_from_dir_with_source(
                path,
                &TargetSource::Pack {
                    pack: pack.name.clone(),
                    path: path.clone(),
                },
            )?);
        }
    }

    Ok(loaded)
}

fn parse_target(name: &str, content: &str, source: String) -> Result<TargetSpec, RendererError> {
    let raw: RawTargetSpec =
        toml::from_str(content).map_err(|error| RendererError::ParseTarget {
            name: name.to_owned(),
            source,
            error: Box::new(error),
        })?;

    validate_target(raw)
}

fn validate_target(raw: RawTargetSpec) -> Result<TargetSpec, RendererError> {
    validate_target_name(&raw.name)?;

    if let Some(base) = &raw.extends {
        validate_target_name(base)?;
    }

    if raw.artifacts.is_empty() && raw.extends.is_none() {
        return Err(RendererError::TargetHasNoArtifacts {
            target: raw.name.clone(),
        });
    }

    let mut artifacts = Vec::with_capacity(raw.artifacts.len());

    for artifact in raw.artifacts {
        validate_artifact_file_name(&raw.name, &artifact.file_name)?;
        compile_template(&raw.name, &artifact.file_name, &artifact.template)?;
        artifacts.push(ArtifactSpec {
            file_name: artifact.file_name,
            template: artifact.template,
        });
    }

    Ok(TargetSpec {
        name: raw.name,
        description: raw.description,
        extends: raw.extends,
        artifacts,
    })
}

fn validate_target_name(name: &str) -> Result<(), RendererError> {
    if name.is_empty() {
        return Err(RendererError::InvalidTargetName {
            name: name.to_owned(),
            reason: "expected a non-empty lowercase identifier".to_owned(),
        });
    }

    if name
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        Ok(())
    } else {
        Err(RendererError::InvalidTargetName {
            name: name.to_owned(),
            reason: "expected only lowercase ASCII letters, digits, '-' or '_'".to_owned(),
        })
    }
}

fn validate_artifact_file_name(target: &str, file_name: &str) -> Result<(), RendererError> {
    if file_name.is_empty() {
        return Err(RendererError::InvalidArtifactFileName {
            target: target.to_owned(),
            file_name: file_name.to_owned(),
            reason: "expected a non-empty file name".to_owned(),
        });
    }

    if file_name.contains('/') || file_name.contains('\\') {
        return Err(RendererError::InvalidArtifactFileName {
            target: target.to_owned(),
            file_name: file_name.to_owned(),
            reason: "expected a single file name without path separators".to_owned(),
        });
    }

    if file_name == "." || file_name == ".." {
        return Err(RendererError::InvalidArtifactFileName {
            target: target.to_owned(),
            file_name: file_name.to_owned(),
            reason: "expected a normal relative file name".to_owned(),
        });
    }

    Ok(())
}

fn compile_registry_targets(
    loaded: Vec<LoadedTarget>,
    built_in_names: &BTreeSet<String>,
) -> Result<BTreeMap<String, CompiledTarget>, RendererError> {
    let mut pending = BTreeMap::new();

    for target in loaded {
        if built_in_names.contains(&target.spec.name) {
            return Err(RendererError::TargetNameCollidesWithBuiltIn {
                name: target.spec.name,
            });
        }

        if let Some(previous) = pending.insert(target.spec.name.clone(), target.clone()) {
            return Err(RendererError::DuplicateTargetName {
                name: target.spec.name,
                first_source: previous.source.location(),
                second_source: target.source.location(),
            });
        }
    }

    let mut compiler = TargetCompiler {
        pending: &pending,
        built_in_names,
        compiled: BTreeMap::new(),
        stack: Vec::new(),
    };

    for name in pending.keys() {
        compiler.compile(name)?;
    }

    Ok(compiler.compiled)
}

struct TargetCompiler<'a> {
    pending: &'a BTreeMap<String, LoadedTarget>,
    built_in_names: &'a BTreeSet<String>,
    compiled: BTreeMap<String, CompiledTarget>,
    stack: Vec<String>,
}

impl<'a> TargetCompiler<'a> {
    fn compile(&mut self, name: &str) -> Result<CompiledTarget, RendererError> {
        if let Some(compiled) = self.compiled.get(name) {
            return Ok(compiled.clone());
        }

        if let Some(position) = self.stack.iter().position(|active| active == name) {
            let mut cycle = self.stack[position..].to_vec();
            cycle.push(name.to_owned());
            return Err(RendererError::TargetInheritanceCycle {
                cycle: cycle.join(" -> "),
            });
        }

        let loaded =
            self.pending
                .get(name)
                .cloned()
                .ok_or_else(|| RendererError::UnknownBaseTarget {
                    target: self
                        .stack
                        .last()
                        .cloned()
                        .unwrap_or_else(|| name.to_owned()),
                    base: name.to_owned(),
                })?;

        self.stack.push(name.to_owned());

        let base = match loaded.spec.extends.as_deref() {
            Some(base) if self.built_in_names.contains(base) => {
                return Err(RendererError::BuiltInTargetInheritance {
                    target: loaded.spec.name,
                    base: base.to_owned(),
                });
            }
            Some(base) => self.compile(base)?,
            None => CompiledTarget {
                name: loaded.spec.name.clone(),
                artifacts: Vec::new(),
                source: loaded.source.clone(),
            },
        };

        let compiled = compile_loaded_target(&loaded, &base)?;
        self.stack.pop();
        self.compiled.insert(name.to_owned(), compiled.clone());

        Ok(compiled)
    }
}

fn compile_loaded_target(
    loaded: &LoadedTarget,
    base: &CompiledTarget,
) -> Result<CompiledTarget, RendererError> {
    let mut artifacts = base.artifacts.clone();

    for artifact in &loaded.spec.artifacts {
        let compiled = CompiledArtifactSpec {
            file_name: artifact.file_name.clone(),
            template: compile_template(&loaded.spec.name, &artifact.file_name, &artifact.template)?,
        };

        if let Some(existing) = artifacts
            .iter_mut()
            .find(|existing| existing.file_name == compiled.file_name)
        {
            *existing = compiled;
        } else {
            artifacts.push(compiled);
        }
    }

    if artifacts.is_empty() {
        return Err(RendererError::TargetHasNoArtifacts {
            target: loaded.spec.name.clone(),
        });
    }

    Ok(CompiledTarget {
        name: loaded.spec.name.clone(),
        artifacts,
        source: loaded.source.clone(),
    })
}

fn compile_template(
    target: &str,
    file_name: &str,
    template: &str,
) -> Result<CompiledTemplate, RendererError> {
    let mut segments = Vec::new();
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        if start > 0 {
            segments.push(CompiledSegment::Literal(rest[..start].to_owned()));
        }

        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            return Err(RendererError::UnterminatedPlaceholder {
                target: target.to_owned(),
                file_name: file_name.to_owned(),
            });
        };

        let raw_placeholder = after_start[..end].trim();
        let placeholder = compile_placeholder(target, file_name, raw_placeholder)?;
        segments.push(CompiledSegment::Placeholder(placeholder));
        rest = &after_start[end + 2..];
    }

    if !rest.is_empty() {
        segments.push(CompiledSegment::Literal(rest.to_owned()));
    }

    Ok(CompiledTemplate { segments })
}

fn compile_placeholder(
    target: &str,
    file_name: &str,
    placeholder: &str,
) -> Result<CompiledPlaceholder, RendererError> {
    if placeholder.is_empty() {
        return Err(RendererError::InvalidPlaceholder {
            target: target.to_owned(),
            file_name: file_name.to_owned(),
            placeholder: placeholder.to_owned(),
        });
    }

    let mut parts = placeholder.split('|').map(str::trim);
    let Some(value) = parts.next() else {
        return Err(RendererError::InvalidPlaceholder {
            target: target.to_owned(),
            file_name: file_name.to_owned(),
            placeholder: placeholder.to_owned(),
        });
    };

    let value = if let Some(token_name) = value.strip_prefix("tokens.") {
        let token = SemanticTokenName::from_str(token_name).map_err(|_| {
            RendererError::InvalidPlaceholder {
                target: target.to_owned(),
                file_name: file_name.to_owned(),
                placeholder: placeholder.to_owned(),
            }
        })?;

        PlaceholderValue::Token(token)
    } else if let Some(field) = value.strip_prefix("ctx.") {
        let field = match field {
            "mode" => ContextField::Mode,
            "template_name" => ContextField::TemplateName,
            "output_dir" => ContextField::OutputDir,
            "seed" => ContextField::Seed,
            _ => {
                return Err(RendererError::InvalidPlaceholder {
                    target: target.to_owned(),
                    file_name: file_name.to_owned(),
                    placeholder: placeholder.to_owned(),
                });
            }
        };

        PlaceholderValue::Context(field)
    } else {
        return Err(RendererError::InvalidPlaceholder {
            target: target.to_owned(),
            file_name: file_name.to_owned(),
            placeholder: placeholder.to_owned(),
        });
    };

    let mut transforms = Vec::new();

    for raw_transform in parts {
        transforms.push(compile_placeholder_transform(
            target,
            file_name,
            placeholder,
            raw_transform,
        )?);
    }

    if matches!(value, PlaceholderValue::Context(_)) && !transforms.is_empty() {
        return Err(RendererError::InvalidPlaceholder {
            target: target.to_owned(),
            file_name: file_name.to_owned(),
            placeholder: placeholder.to_owned(),
        });
    }

    Ok(CompiledPlaceholder { value, transforms })
}

fn compile_placeholder_transform(
    target: &str,
    file_name: &str,
    placeholder: &str,
    raw_transform: &str,
) -> Result<PlaceholderTransform, RendererError> {
    if raw_transform == "hex_no_hash" {
        return Ok(PlaceholderTransform::HexNoHash);
    }

    if let Some(alpha) = raw_transform
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    {
        if alpha.len() != 2 {
            return Err(RendererError::InvalidPlaceholder {
                target: target.to_owned(),
                file_name: file_name.to_owned(),
                placeholder: placeholder.to_owned(),
            });
        }

        let alpha =
            u8::from_str_radix(alpha, 16).map_err(|_| RendererError::InvalidPlaceholder {
                target: target.to_owned(),
                file_name: file_name.to_owned(),
                placeholder: placeholder.to_owned(),
            })?;

        return Ok(PlaceholderTransform::Rgba { alpha });
    }

    Err(RendererError::InvalidPlaceholder {
        target: target.to_owned(),
        file_name: file_name.to_owned(),
        placeholder: placeholder.to_owned(),
    })
}

impl CompiledTemplate {
    fn render(&self, tokens: &SemanticTokens, ctx: &GenerationContext) -> String {
        let mut rendered = String::new();

        for segment in &self.segments {
            match segment {
                CompiledSegment::Literal(literal) => rendered.push_str(literal),
                CompiledSegment::Placeholder(placeholder) => {
                    rendered.push_str(&resolve_placeholder(placeholder, tokens, ctx));
                }
            }
        }

        rendered
    }
}

fn resolve_placeholder(
    placeholder: &CompiledPlaceholder,
    tokens: &SemanticTokens,
    ctx: &GenerationContext,
) -> String {
    let mut value = match &placeholder.value {
        PlaceholderValue::Token(token) => match token {
            SemanticTokenName::Bg => tokens.bg.clone(),
            SemanticTokenName::BgSecondary => tokens.bg_secondary.clone(),
            SemanticTokenName::Surface => tokens.surface.clone(),
            SemanticTokenName::SurfaceElevated => tokens.surface_elevated.clone(),
            SemanticTokenName::Text => tokens.text.clone(),
            SemanticTokenName::TextMuted => tokens.text_muted.clone(),
            SemanticTokenName::Border => tokens.border.clone(),
            SemanticTokenName::BorderStrong => tokens.border_strong.clone(),
            SemanticTokenName::Accent => tokens.accent.clone(),
            SemanticTokenName::AccentHover => tokens.accent_hover.clone(),
            SemanticTokenName::AccentActive => tokens.accent_active.clone(),
            SemanticTokenName::AccentFg => tokens.accent_fg.clone(),
            SemanticTokenName::Selection => tokens.selection.clone(),
            SemanticTokenName::Link => tokens.link.clone(),
            SemanticTokenName::Success => tokens.success.clone(),
            SemanticTokenName::Warning => tokens.warning.clone(),
            SemanticTokenName::Error => tokens.error.clone(),
        },
        PlaceholderValue::Context(field) => match field {
            ContextField::Mode => ctx.mode.to_string(),
            ContextField::TemplateName => ctx.template_name.clone(),
            ContextField::OutputDir => ctx.output_dir.display().to_string(),
            ContextField::Seed => ctx.seed.clone().unwrap_or_default(),
        },
    };

    for transform in &placeholder.transforms {
        value = apply_placeholder_transform(&value, transform);
    }

    value
}

fn apply_placeholder_transform(value: &str, transform: &PlaceholderTransform) -> String {
    match transform {
        PlaceholderTransform::HexNoHash => normalized_hex_without_hash(value)
            .expect("color transforms should resolve from valid semantic tokens"),
        PlaceholderTransform::Rgba { alpha } => hyprland_rgba(value, *alpha)
            .expect("color transforms should resolve from valid semantic tokens"),
    }
}

fn normalized_target_requests(requested: &[String], built_in: &RendererRegistry) -> Vec<String> {
    if requested.is_empty() {
        return built_in.built_in_names();
    }

    let mut seen = BTreeSet::new();
    let mut normalized = Vec::with_capacity(requested.len());

    for target in requested {
        if seen.insert(target.clone()) {
            normalized.push(target.clone());
        }
    }

    normalized
}

fn looks_like_path(value: &str) -> bool {
    let path = Path::new(value);

    path.is_absolute()
        || value.contains(std::path::MAIN_SEPARATOR)
        || path.extension().and_then(|extension| extension.to_str()) == Some("toml")
}
