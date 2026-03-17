use std::{collections::BTreeMap, fmt, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

pub type HexColor = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

impl ThemeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub const fn default_background_tone(self) -> u8 {
        match self {
            Self::Light => 98,
            Self::Dark => 10,
        }
    }

    pub const fn default_surface_tone(self) -> u8 {
        match self {
            Self::Light => 95,
            Self::Dark => 14,
        }
    }

    pub const fn default_text_tone(self) -> u8 {
        match self {
            Self::Light => 12,
            Self::Dark => 94,
        }
    }

    pub const fn default_muted_text_tone(self) -> u8 {
        match self {
            Self::Light => 30,
            Self::Dark => 80,
        }
    }
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ContrastStrategy {
    #[default]
    RelativeLuminance,
    ApcaExperimental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ChromaStrategy {
    Subtle,
    #[default]
    Normal,
    Vibrant,
    Muted,
    Industrial,
}

impl ChromaStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Subtle => "subtle",
            Self::Normal => "normal",
            Self::Vibrant => "vibrant",
            Self::Muted => "muted",
            Self::Industrial => "industrial",
        }
    }
}

impl fmt::Display for ChromaStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ContrastStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RelativeLuminance => "relative-luminance",
            Self::ApcaExperimental => "apca-experimental",
        }
    }
}

impl fmt::Display for ContrastStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderTarget {
    Gtk,
    Hyprland,
    Kitty,
    Css,
    Waybar,
    Rofi,
    Alacritty,
    Foot,
    Ghostty,
    Editor,
}

impl RenderTarget {
    pub const MVP: [Self; 4] = [Self::Gtk, Self::Hyprland, Self::Kitty, Self::Css];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gtk => "gtk",
            Self::Hyprland => "hyprland",
            Self::Kitty => "kitty",
            Self::Css => "css",
            Self::Waybar => "waybar",
            Self::Rofi => "rofi",
            Self::Alacritty => "alacritty",
            Self::Foot => "foot",
            Self::Ghostty => "ghostty",
            Self::Editor => "editor",
        }
    }

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Gtk => "gtk.css",
            Self::Hyprland => "hyprland.conf",
            Self::Kitty => "kitty.conf",
            Self::Css => "theme.css",
            Self::Waybar => "style.css",
            Self::Rofi => "config.rasi",
            Self::Alacritty => "alacritty.toml",
            Self::Foot => "foot.ini",
            Self::Ghostty => "colors.txt",
            Self::Editor => "theme.json",
        }
    }
}

impl fmt::Display for RenderTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum PaletteFamilyName {
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Neutral,
    NeutralVariant,
    Error,
    Success,
    Warning,
    Info,
}

impl PaletteFamilyName {
    pub const ALL: [Self; 9] = [
        Self::Primary,
        Self::Secondary,
        Self::Tertiary,
        Self::Neutral,
        Self::NeutralVariant,
        Self::Error,
        Self::Success,
        Self::Warning,
        Self::Info,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Tertiary => "tertiary",
            Self::Neutral => "neutral",
            Self::NeutralVariant => "neutral_variant",
            Self::Error => "error",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

impl fmt::Display for PaletteFamilyName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PaletteFamilyName {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "primary" => Ok(Self::Primary),
            "secondary" => Ok(Self::Secondary),
            "tertiary" => Ok(Self::Tertiary),
            "neutral" => Ok(Self::Neutral),
            "neutral_variant" => Ok(Self::NeutralVariant),
            "error" => Ok(Self::Error),
            "success" => Ok(Self::Success),
            "warning" => Ok(Self::Warning),
            "info" => Ok(Self::Info),
            _ => Err(()),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum SemanticTokenName {
    #[default]
    Bg,
    BgSecondary,
    Surface,
    SurfaceElevated,
    Text,
    TextMuted,
    Border,
    BorderStrong,
    Accent,
    AccentHover,
    AccentActive,
    AccentFg,
    Selection,
    Link,
    Success,
    Warning,
    Error,
}

impl SemanticTokenName {
    pub const ALL: [Self; 17] = [
        Self::Bg,
        Self::BgSecondary,
        Self::Surface,
        Self::SurfaceElevated,
        Self::Text,
        Self::TextMuted,
        Self::Border,
        Self::BorderStrong,
        Self::Accent,
        Self::AccentHover,
        Self::AccentActive,
        Self::AccentFg,
        Self::Selection,
        Self::Link,
        Self::Success,
        Self::Warning,
        Self::Error,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bg => "bg",
            Self::BgSecondary => "bg_secondary",
            Self::Surface => "surface",
            Self::SurfaceElevated => "surface_elevated",
            Self::Text => "text",
            Self::TextMuted => "text_muted",
            Self::Border => "border",
            Self::BorderStrong => "border_strong",
            Self::Accent => "accent",
            Self::AccentHover => "accent_hover",
            Self::AccentActive => "accent_active",
            Self::AccentFg => "accent_fg",
            Self::Selection => "selection",
            Self::Link => "link",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for SemanticTokenName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SemanticTokenName {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "bg" => Ok(Self::Bg),
            "bg_secondary" => Ok(Self::BgSecondary),
            "surface" => Ok(Self::Surface),
            "surface_elevated" => Ok(Self::SurfaceElevated),
            "text" => Ok(Self::Text),
            "text_muted" => Ok(Self::TextMuted),
            "border" => Ok(Self::Border),
            "border_strong" => Ok(Self::BorderStrong),
            "accent" => Ok(Self::Accent),
            "accent_hover" => Ok(Self::AccentHover),
            "accent_active" => Ok(Self::AccentActive),
            "accent_fg" => Ok(Self::AccentFg),
            "selection" => Ok(Self::Selection),
            "link" => Ok(Self::Link),
            "success" => Ok(Self::Success),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToneSample {
    pub tone: u8,
    pub hex: HexColor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaletteFamily {
    pub name: PaletteFamilyName,
    pub hue: f32,
    pub base_chroma: f32,
    #[serde(default)]
    pub tones: Vec<ToneSample>,
    pub dominance: Option<f32>,
    pub source_region: Option<String>,
    pub seed_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedPalette {
    pub seed: HexColor,
    pub mode: ThemeMode,
    pub chroma: ChromaStrategy,
    pub families: BTreeMap<PaletteFamilyName, PaletteFamily>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub seed: Option<String>,
    pub wallpaper: Option<PathBuf>,
    pub template: Option<String>,
    pub mode: ThemeMode,
    #[serde(default)]
    pub contrast: ContrastStrategy,
    #[serde(default)]
    pub chroma: ChromaStrategy,
    #[serde(default)]
    pub targets: Vec<String>,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedArtifact {
    pub target: String,
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GenerationContext {
    pub mode: ThemeMode,
    pub template_name: String,
    pub chroma: ChromaStrategy,
    pub output_dir: PathBuf,
    pub seed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ThemePack {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub root_dir: PathBuf,
    #[serde(default)]
    pub template_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub target_dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TemplateTokenRule {
    pub family: PaletteFamilyName,
    pub tone: f32,
    pub chroma: Option<f32>,
    pub chroma_scale: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TemplateDefinition {
    pub name: String,
    pub mode: ThemeMode,
    pub description: Option<String>,
    #[serde(default)]
    pub tokens: BTreeMap<SemanticTokenName, TemplateTokenRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SemanticTokens {
    pub bg: HexColor,
    pub bg_secondary: HexColor,
    pub surface: HexColor,
    pub surface_elevated: HexColor,
    pub text: HexColor,
    pub text_muted: HexColor,
    pub border: HexColor,
    pub border_strong: HexColor,
    pub accent: HexColor,
    pub accent_hover: HexColor,
    pub accent_active: HexColor,
    pub accent_fg: HexColor,
    pub selection: HexColor,
    pub link: HexColor,
    pub success: HexColor,
    pub warning: HexColor,
    pub error: HexColor,
}
