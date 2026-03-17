use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;

/// Parameters for generating theme artifacts from a seed color.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateParams {
    /// Seed color in #RRGGBB hex format.
    pub seed: String,
    /// Template name (e.g. "minimal", "brutalist", "terminal") or path to a TOML template file.
    pub template: String,
    /// Theme mode: "dark" or "light". Defaults to "dark".
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Contrast strategy: "relative-luminance" or "apca-experimental". Defaults to "relative-luminance".
    #[serde(default = "default_contrast")]
    pub contrast: String,
    /// List of target names (e.g. "gtk", "kitty", "css") or paths to target TOML files.
    pub targets: Vec<String>,
    /// Output directory for generated artifact files.
    pub output_dir: String,
}

/// Parameters for generating theme artifacts from a wallpaper image.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WallpaperParams {
    /// Path to the wallpaper image file (JPEG, PNG, or WebP).
    pub image: String,
    /// Template name (e.g. "minimal", "brutalist", "terminal") or path to a TOML template file.
    pub template: String,
    /// Theme mode: "dark" or "light". Defaults to "dark".
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Contrast strategy: "relative-luminance" or "apca-experimental". Defaults to "relative-luminance".
    #[serde(default = "default_contrast")]
    pub contrast: String,
    /// List of target names (e.g. "gtk", "kitty", "css") or paths to target TOML files.
    pub targets: Vec<String>,
    /// Output directory for generated artifact files.
    pub output_dir: String,
}

/// Parameters for running a batch manifest with multiple generation jobs.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchParams {
    /// Path to the TOML batch manifest file.
    pub manifest: String,
}

/// Parameters for previewing a palette and resolved semantic tokens.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PreviewParams {
    /// Seed color in #RRGGBB hex format.
    pub seed: String,
    /// Template name or path to a TOML template file.
    pub template: String,
    /// Theme mode: "dark" or "light". Defaults to "dark".
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Contrast strategy: "relative-luminance" or "apca-experimental". Defaults to "relative-luminance".
    #[serde(default = "default_contrast")]
    pub contrast: String,
}

/// Parameters for exporting resolved semantic tokens as JSON.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportTokensParams {
    /// Seed color in #RRGGBB hex format.
    pub seed: String,
    /// Template name or path to a TOML template file.
    pub template: String,
    /// Theme mode: "dark" or "light". Defaults to "dark".
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Contrast strategy: "relative-luminance" or "apca-experimental". Defaults to "relative-luminance".
    #[serde(default = "default_contrast")]
    pub contrast: String,
}

/// Parameters for generating the full OKLCH palette from a seed color.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GeneratePaletteParams {
    /// Seed color in #RRGGBB hex format.
    pub seed: String,
    /// Theme mode: "dark" or "light". Defaults to "dark".
    #[serde(default = "default_mode")]
    pub mode: String,
}

/// Parameters for inspecting a theme pack.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PackInfoParams {
    /// Name of the theme pack to inspect.
    pub name: String,
}

pub fn build_generation_request(
    seed: Option<String>,
    wallpaper: Option<PathBuf>,
    template: String,
    mode: String,
    contrast: String,
    targets: Vec<String>,
    output_dir: String,
) -> Result<chromasync_types::GenerationRequest, String> {
    Ok(chromasync_types::GenerationRequest {
        seed,
        wallpaper,
        template,
        mode: crate::convert::parse_mode(&mode)?,
        contrast: crate::convert::parse_contrast(&contrast)?,
        targets,
        output_dir: PathBuf::from(output_dir),
    })
}

fn default_mode() -> String {
    "dark".to_owned()
}

fn default_contrast() -> String {
    "relative-luminance".to_owned()
}
