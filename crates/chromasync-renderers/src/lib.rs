mod alacritty;
mod kitty;
mod registry;

use chromasync_types::{
    GeneratedArtifact, GenerationContext, HexColor, RenderTarget, SemanticTokens,
};
use thiserror::Error;

pub use crate::registry::{
    ArtifactSpec, CompiledTarget, ListedTarget, OutputRegistry, RendererRegistry, TargetRegistry,
    TargetSource, TargetSpec, user_targets_dir,
};

pub const BUILTIN_TARGETS: [RenderTarget; 2] = [RenderTarget::Kitty, RenderTarget::Alacritty];

pub trait ArtifactGenerator: Send + Sync {
    fn name(&self) -> &str;
    fn generate(
        &self,
        tokens: &SemanticTokens,
        ctx: &GenerationContext,
    ) -> Result<Vec<GeneratedArtifact>, RendererError>;
}

pub trait Renderer: Send + Sync {
    fn target(&self) -> RenderTarget;
    fn name(&self) -> &'static str;

    fn file_name(&self) -> &'static str {
        self.target().file_name()
    }

    fn render_artifact(&self, tokens: &SemanticTokens) -> Result<GeneratedArtifact, RendererError>;
}

impl<T> ArtifactGenerator for T
where
    T: Renderer,
{
    fn name(&self) -> &str {
        Renderer::name(self)
    }

    fn generate(
        &self,
        tokens: &SemanticTokens,
        _ctx: &GenerationContext,
    ) -> Result<Vec<GeneratedArtifact>, RendererError> {
        Ok(vec![self.render_artifact(tokens)?])
    }
}

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("renderer '{target}' is not implemented yet")]
    UnsupportedTarget { target: RenderTarget },
    #[error("target '{requested}' was not found")]
    UnknownTarget { requested: String },
    #[error("color '{value}' must use the #RRGGBB format")]
    InvalidColorFormat { value: String },
    #[error("color '{value}' contains invalid hexadecimal digits")]
    InvalidHexDigits { value: String },
    #[error("failed to read targets directory '{path}': {source}")]
    ReadTargetsDir {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read target file '{path}': {source}")]
    ReadTargetFile {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse target '{name}' from {source}: {error}")]
    ParseTarget {
        name: String,
        source: String,
        #[source]
        error: Box<toml::de::Error>,
    },
    #[error("target '{name}' uses an invalid name: {reason}")]
    InvalidTargetName { name: String, reason: String },
    #[error("target '{target}' has no artifacts after inheritance is resolved")]
    TargetHasNoArtifacts { target: String },
    #[error("target '{target}' artifact '{file_name}' is invalid: {reason}")]
    InvalidArtifactFileName {
        target: String,
        file_name: String,
        reason: String,
    },
    #[error("target '{target}' artifact '{file_name}' uses an invalid placeholder '{placeholder}'")]
    InvalidPlaceholder {
        target: String,
        file_name: String,
        placeholder: String,
    },
    #[error("target '{target}' artifact '{file_name}' contains an unterminated placeholder")]
    UnterminatedPlaceholder { target: String, file_name: String },
    #[error("target '{target}' references unknown base target '{base}'")]
    UnknownBaseTarget { target: String, base: String },
    #[error("target '{target}' cannot inherit from built-in renderer '{base}'")]
    BuiltInTargetInheritance { target: String, base: String },
    #[error("target inheritance cycle detected: {cycle}")]
    TargetInheritanceCycle { cycle: String },
    #[error("user target '{name}' collides with a built-in renderer name")]
    TargetNameCollidesWithBuiltIn { name: String },
    #[error(
        "user target '{name}' is defined multiple times: first at {first_source}, second at {second_source}"
    )]
    DuplicateTargetName {
        name: String,
        first_source: String,
        second_source: String,
    },
}

pub fn built_in_targets() -> &'static [RenderTarget] {
    &BUILTIN_TARGETS
}

pub fn render_target(
    target: RenderTarget,
    tokens: &SemanticTokens,
) -> Result<GeneratedArtifact, RendererError> {
    let registry = RendererRegistry::new();
    let generator = registry
        .get(target.as_str())
        .ok_or(RendererError::UnsupportedTarget { target })?;
    let context = GenerationContext::default();
    let mut artifacts = generator.generate(tokens, &context)?;

    artifacts
        .pop()
        .ok_or(RendererError::UnsupportedTarget { target })
}

pub fn render_targets(
    targets: &[RenderTarget],
    tokens: &SemanticTokens,
) -> Result<Vec<GeneratedArtifact>, RendererError> {
    let registry = RendererRegistry::new();
    let requested = if targets.is_empty() {
        registry.built_in_names()
    } else {
        let mut seen = std::collections::BTreeSet::new();
        let mut names = Vec::with_capacity(targets.len());

        for target in targets {
            let name = target.as_str().to_owned();

            if seen.insert(name.clone()) {
                names.push(name);
            }
        }

        names
    };

    registry::OutputRegistry::default().generate(&requested, tokens, &GenerationContext::default())
}

fn parse_hex_color(value: &str) -> Result<[u8; 3], RendererError> {
    let normalized = value.strip_prefix('#').unwrap_or(value);

    if normalized.len() != 6 {
        return Err(RendererError::InvalidColorFormat {
            value: value.to_owned(),
        });
    }

    let red =
        u8::from_str_radix(&normalized[0..2], 16).map_err(|_| RendererError::InvalidHexDigits {
            value: value.to_owned(),
        })?;
    let green =
        u8::from_str_radix(&normalized[2..4], 16).map_err(|_| RendererError::InvalidHexDigits {
            value: value.to_owned(),
        })?;
    let blue =
        u8::from_str_radix(&normalized[4..6], 16).map_err(|_| RendererError::InvalidHexDigits {
            value: value.to_owned(),
        })?;

    Ok([red, green, blue])
}

fn hyprland_rgba(value: &str, alpha: u8) -> Result<String, RendererError> {
    let [red, green, blue] = parse_hex_color(value)?;

    Ok(format!("rgba({red:02X}{green:02X}{blue:02X}{alpha:02X})"))
}

fn normalized_hex(value: &str) -> Result<String, RendererError> {
    let [red, green, blue] = parse_hex_color(value)?;

    Ok(format!("#{red:02X}{green:02X}{blue:02X}"))
}

fn normalized_hex_without_hash(value: &str) -> Result<String, RendererError> {
    let [red, green, blue] = parse_hex_color(value)?;

    Ok(format!("{red:02x}{green:02x}{blue:02x}"))
}

fn terminal_ansi_colors(tokens: &SemanticTokens) -> [HexColor; 16] {
    [
        tokens.bg_secondary.clone(),
        tokens.error.clone(),
        tokens.success.clone(),
        tokens.warning.clone(),
        tokens.link.clone(),
        tokens.accent.clone(),
        tokens.selection.clone(),
        tokens.text_muted.clone(),
        tokens.surface_elevated.clone(),
        tokens.error.clone(),
        tokens.success.clone(),
        tokens.warning.clone(),
        tokens.accent_hover.clone(),
        tokens.accent_active.clone(),
        tokens.border_strong.clone(),
        tokens.text.clone(),
    ]
}
