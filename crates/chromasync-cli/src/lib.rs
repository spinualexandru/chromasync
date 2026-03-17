use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chromasync_types::{ContrastStrategy, GeneratedArtifact, GenerationRequest, ThemeMode};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(
    name = "chromasync",
    version,
    about = "Dynamic color engine and theme generator CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate theme artifacts from a seed color.
    Generate(GenerateArgs),
    /// Generate theme artifacts from a wallpaper image.
    Wallpaper(WallpaperArgs),
    /// Execute a batch manifest with multiple generation jobs.
    Batch(BatchArgs),
    /// List the available templates and where they were loaded from.
    Templates,
    /// List the discovered theme packs.
    Packs,
    /// Inspect a discovered theme pack.
    Pack {
        #[command(subcommand)]
        command: PackCommand,
    },
    /// List available renderer targets and where they were loaded from.
    Targets,
    /// Show palette families and resolved semantic tokens.
    Preview(PreviewArgs),
    /// Export resolved semantic tokens.
    Tokens(TokensArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum PackCommand {
    /// Show metadata and assets for an installed pack.
    Info(PackInfoArgs),
}

#[derive(Debug, Clone, Args)]
struct PackInfoArgs {
    /// Pack name from pack.toml.
    name: String,
}

#[derive(Debug, Clone, Args)]
struct GenerateArgs {
    /// Seed color in #RRGGBB format.
    #[arg(long)]
    seed: String,
    /// Template name or path to a template TOML file.
    #[arg(long)]
    template: String,
    /// Theme mode to generate.
    #[arg(long, value_enum, default_value_t = CliMode::Dark)]
    mode: CliMode,
    /// Contrast selection heuristic used when resolving readable foregrounds.
    #[arg(long, value_enum, default_value_t = CliContrast::RelativeLuminance)]
    contrast: CliContrast,
    /// Comma-separated list of target names or target TOML paths to generate.
    #[arg(long, value_delimiter = ',', required = true)]
    targets: Vec<String>,
    /// Output directory for generated artifacts.
    #[arg(long, default_value = "chromasync")]
    output: PathBuf,
}

#[derive(Debug, Clone, Args)]
struct WallpaperArgs {
    /// Wallpaper image path.
    #[arg(long)]
    image: PathBuf,
    /// Template name or path to a template TOML file.
    #[arg(long)]
    template: String,
    /// Theme mode to generate.
    #[arg(long, value_enum, default_value_t = CliMode::Dark)]
    mode: CliMode,
    /// Contrast selection heuristic used when resolving readable foregrounds.
    #[arg(long, value_enum, default_value_t = CliContrast::RelativeLuminance)]
    contrast: CliContrast,
    /// Comma-separated list of target names or target TOML paths to generate.
    #[arg(long, value_delimiter = ',', required = true)]
    targets: Vec<String>,
    /// Output directory for generated artifacts.
    #[arg(long, default_value = "chromasync")]
    output: PathBuf,
}

#[derive(Debug, Clone, Args)]
struct PreviewArgs {
    /// Seed color in #RRGGBB format.
    #[arg(long)]
    seed: String,
    /// Template name or path to a template TOML file.
    #[arg(long)]
    template: String,
    /// Theme mode to preview.
    #[arg(long, value_enum, default_value_t = CliMode::Dark)]
    mode: CliMode,
    /// Contrast selection heuristic used when resolving readable foregrounds.
    #[arg(long, value_enum, default_value_t = CliContrast::RelativeLuminance)]
    contrast: CliContrast,
}

#[derive(Debug, Clone, Args)]
struct TokensArgs {
    /// Seed color in #RRGGBB format.
    #[arg(long)]
    seed: String,
    /// Template name or path to a template TOML file.
    #[arg(long)]
    template: String,
    /// Theme mode to resolve.
    #[arg(long, value_enum, default_value_t = CliMode::Dark)]
    mode: CliMode,
    /// Contrast selection heuristic used when resolving readable foregrounds.
    #[arg(long, value_enum, default_value_t = CliContrast::RelativeLuminance)]
    contrast: CliContrast,
    /// Serialization format for token export.
    #[arg(long, value_enum, default_value_t = CliFormat::Json)]
    format: CliFormat,
}

#[derive(Debug, Clone, Args)]
struct BatchArgs {
    /// Path to a TOML manifest containing multiple jobs.
    #[arg(long)]
    file: PathBuf,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliFormat {
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliContrast {
    RelativeLuminance,
    ApcaExperimental,
}

#[derive(Debug, Deserialize)]
struct BatchManifest {
    #[serde(default, alias = "job")]
    jobs: Vec<BatchJob>,
}

#[derive(Debug, Deserialize)]
struct BatchJob {
    name: Option<String>,
    seed: Option<String>,
    image: Option<PathBuf>,
    template: String,
    #[serde(default)]
    mode: ThemeMode,
    #[serde(default)]
    contrast: ContrastStrategy,
    #[serde(default)]
    targets: Vec<String>,
    output: PathBuf,
}

impl From<CliMode> for ThemeMode {
    fn from(value: CliMode) -> Self {
        match value {
            CliMode::Dark => Self::Dark,
            CliMode::Light => Self::Light,
        }
    }
}

impl From<CliContrast> for ContrastStrategy {
    fn from(value: CliContrast) -> Self {
        match value {
            CliContrast::RelativeLuminance => Self::RelativeLuminance,
            CliContrast::ApcaExperimental => Self::ApcaExperimental,
        }
    }
}

impl GenerateArgs {
    fn into_request(self) -> Result<GenerationRequest> {
        Ok(GenerationRequest {
            seed: Some(self.seed),
            wallpaper: None,
            template: self.template,
            mode: self.mode.into(),
            contrast: self.contrast.into(),
            targets: normalize_targets(self.targets)?,
            output_dir: self.output,
        })
    }
}

impl WallpaperArgs {
    fn into_request(self) -> Result<GenerationRequest> {
        Ok(GenerationRequest {
            seed: None,
            wallpaper: Some(self.image),
            template: self.template,
            mode: self.mode.into(),
            contrast: self.contrast.into(),
            targets: normalize_targets(self.targets)?,
            output_dir: self.output,
        })
    }
}

impl PreviewArgs {
    fn into_request(self) -> GenerationRequest {
        GenerationRequest {
            seed: Some(self.seed),
            wallpaper: None,
            template: self.template,
            mode: self.mode.into(),
            contrast: self.contrast.into(),
            targets: Vec::new(),
            output_dir: PathBuf::from("chromasync"),
        }
    }
}

impl TokensArgs {
    fn into_request(self) -> GenerationRequest {
        GenerationRequest {
            seed: Some(self.seed),
            wallpaper: None,
            template: self.template,
            mode: self.mode.into(),
            contrast: self.contrast.into(),
            targets: Vec::new(),
            output_dir: PathBuf::from("chromasync"),
        }
    }
}

pub fn run() -> Result<()> {
    run_with(Cli::parse())
}

pub fn run_with(cli: Cli) -> Result<()> {
    let output_registry = match &cli.command {
        Command::Generate(_) | Command::Wallpaper(_) | Command::Batch(_) | Command::Targets => {
            Some(chromasync_core::load_output_registry()?)
        }
        Command::Templates
        | Command::Packs
        | Command::Pack { .. }
        | Command::Preview(_)
        | Command::Tokens(_) => None,
    };

    match cli.command {
        Command::Generate(args) => {
            let request = args.into_request()?;
            let artifacts = chromasync_core::generate_with_output_registry(
                &request,
                output_registry
                    .as_ref()
                    .expect("output registry should be loaded for generate"),
            )?;

            write_artifacts(&request.output_dir, &artifacts)
        }
        Command::Wallpaper(args) => {
            let request = args.into_request()?;
            let artifacts = generate_artifacts(
                &request,
                output_registry
                    .as_ref()
                    .expect("output registry should be loaded for wallpaper"),
            )?;

            write_artifacts(&request.output_dir, &artifacts)
        }
        Command::Batch(args) => run_batch(
            args,
            output_registry
                .as_ref()
                .expect("output registry should be loaded for batch"),
        ),
        Command::Templates => print_templates(),
        Command::Packs => print_packs(),
        Command::Pack { command } => match command {
            PackCommand::Info(args) => print_pack_info(&args.name),
        },
        Command::Targets => print_targets(
            output_registry
                .as_ref()
                .expect("output registry should be loaded for targets"),
        ),
        Command::Preview(args) => {
            let preview = chromasync_core::preview(&args.into_request())?;
            println!("{preview}");
            Ok(())
        }
        Command::Tokens(args) => {
            let format = args.format;
            let tokens = chromasync_core::export_tokens(&args.into_request())?;

            match format {
                CliFormat::Json => {
                    let json = serde_json::to_string_pretty(&tokens)
                        .context("failed to serialize semantic tokens")?;
                    println!("{json}");
                }
            }

            Ok(())
        }
    }
}

fn generate_artifacts(
    request: &GenerationRequest,
    output_registry: &chromasync_core::OutputRegistry,
) -> Result<Vec<GeneratedArtifact>> {
    if request.wallpaper.is_some() {
        chromasync_core::generate_from_wallpaper_with_output_registry(request, output_registry)
            .map_err(Into::into)
    } else {
        chromasync_core::generate_with_output_registry(request, output_registry).map_err(Into::into)
    }
}

fn run_batch(args: BatchArgs, output_registry: &chromasync_core::OutputRegistry) -> Result<()> {
    let manifest_path = args.file;
    let manifest_dir = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let content = fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "failed to read batch manifest '{}'",
            manifest_path.display()
        )
    })?;
    let manifest: BatchManifest = toml::from_str(&content).with_context(|| {
        format!(
            "failed to parse batch manifest '{}'",
            manifest_path.display()
        )
    })?;

    if manifest.jobs.is_empty() {
        bail!(
            "batch manifest '{}' does not define any jobs",
            manifest_path.display()
        );
    }

    for (index, job) in manifest.jobs.into_iter().enumerate() {
        let request = batch_job_into_request(job, &manifest_dir)?;
        let artifacts = generate_artifacts(&request, output_registry).with_context(|| {
            format!(
                "batch job {} failed for output '{}'",
                index + 1,
                request.output_dir.display()
            )
        })?;

        write_artifacts(&request.output_dir, &artifacts)?;
    }

    Ok(())
}

fn batch_job_into_request(job: BatchJob, base_dir: &Path) -> Result<GenerationRequest> {
    if job.seed.is_some() == job.image.is_some() {
        let job_label = job.name.as_deref().unwrap_or("<unnamed>");

        bail!("batch job '{job_label}' must define exactly one of 'seed' or 'image'");
    }

    Ok(GenerationRequest {
        seed: job.seed,
        wallpaper: job.image.map(|path| resolve_relative_path(base_dir, &path)),
        template: resolve_template_reference(base_dir, &job.template),
        mode: job.mode,
        contrast: job.contrast,
        targets: normalize_targets_relative_to(base_dir, job.targets)?,
        output_dir: resolve_relative_path(base_dir, &job.output),
    })
}

fn resolve_template_reference(base_dir: &Path, value: &str) -> String {
    if looks_like_path(value) {
        resolve_relative_path(base_dir, Path::new(value))
            .display()
            .to_string()
    } else {
        value.to_owned()
    }
}

fn resolve_target_reference(base_dir: &Path, value: &str) -> String {
    if looks_like_path(value) {
        resolve_relative_path(base_dir, Path::new(value))
            .display()
            .to_string()
    } else {
        value.to_owned()
    }
}

fn resolve_relative_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn looks_like_path(value: &str) -> bool {
    let path = Path::new(value);

    path.is_absolute()
        || value.contains(std::path::MAIN_SEPARATOR)
        || path.extension().and_then(|extension| extension.to_str()) == Some("toml")
}

fn print_templates() -> Result<()> {
    let templates = chromasync_core::list_templates()?;
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    for template in templates {
        writeln!(
            stdout,
            "{}\t{}\t{}\t{}",
            template.definition.name,
            template.definition.mode,
            template.source.label(),
            template.source.location()
        )?;
    }

    Ok(())
}

fn print_packs() -> Result<()> {
    let packs = chromasync_core::list_packs()?;
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    for pack in packs {
        writeln!(
            stdout,
            "{}\t{}\t{}",
            pack.name,
            pack.version,
            pack.root_dir.display()
        )?;
    }

    Ok(())
}

fn print_pack_info(name: &str) -> Result<()> {
    let info = chromasync_core::pack_info(name)?;
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    writeln!(stdout, "name\t{}", info.pack.name)?;
    writeln!(stdout, "version\t{}", info.pack.version)?;
    writeln!(stdout, "root\t{}", info.pack.root_dir.display())?;

    if let Some(description) = &info.pack.description {
        writeln!(stdout, "description\t{description}")?;
    }

    if let Some(author) = &info.pack.author {
        writeln!(stdout, "author\t{author}")?;
    }

    if let Some(license) = &info.pack.license {
        writeln!(stdout, "license\t{license}")?;
    }

    if let Some(homepage) = &info.pack.homepage {
        writeln!(stdout, "homepage\t{homepage}")?;
    }

    writeln!(stdout)?;
    writeln!(stdout, "templates")?;

    for template in info.templates {
        writeln!(
            stdout,
            "{}\t{}\t{}",
            template.definition.name,
            template.definition.mode,
            template.source.location()
        )?;
    }

    writeln!(stdout)?;
    writeln!(stdout, "targets")?;

    for target in info.targets {
        writeln!(stdout, "{}\t{}", target.name, target.source.location())?;
    }

    Ok(())
}

fn print_targets(output_registry: &chromasync_core::OutputRegistry) -> Result<()> {
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    for target in output_registry.list_targets() {
        writeln!(
            stdout,
            "{}\t{}\t{}",
            target.name,
            target.source.label(),
            target.source.location()
        )?;
    }

    Ok(())
}

fn normalize_targets(targets: Vec<String>) -> Result<Vec<String>> {
    normalize_targets_with(targets, |target| target.to_owned())
}

fn normalize_targets_relative_to(base_dir: &Path, targets: Vec<String>) -> Result<Vec<String>> {
    normalize_targets_with(targets, |target| resolve_target_reference(base_dir, target))
}

fn normalize_targets_with<F>(targets: Vec<String>, resolve: F) -> Result<Vec<String>>
where
    F: Fn(&str) -> String,
{
    let normalized = targets
        .into_iter()
        .map(|target| target.trim().to_owned())
        .map(|target| resolve(&target))
        .collect::<Vec<_>>();

    if normalized.iter().any(|target| target.is_empty()) {
        bail!("target names must not be empty");
    }

    Ok(normalized)
}

fn write_artifacts(output_dir: &Path, artifacts: &[GeneratedArtifact]) -> Result<()> {
    if artifacts.is_empty() {
        return Ok(());
    }

    let mut seen_paths = BTreeSet::new();
    let destinations = artifacts
        .iter()
        .map(|artifact| {
            let path = output_dir.join(&artifact.file_name);

            if !seen_paths.insert(path.clone()) {
                bail!(
                    "multiple artifacts would write to the same destination '{}'",
                    path.display()
                );
            }

            if path.exists() {
                bail!(
                    "refusing to overwrite existing artifact '{}'",
                    path.display()
                );
            }

            Ok((artifact, path))
        })
        .collect::<Result<Vec<_>>>()?;

    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create output directory '{}'",
            output_dir.display()
        )
    })?;

    for (artifact, path) in &destinations {
        fs::write(path, &artifact.content)
            .with_context(|| format!("failed to write artifact '{}'", path.display()))?;
    }

    let mut stdout = io::BufWriter::new(io::stdout().lock());

    for (_, path) in &destinations {
        writeln!(stdout, "{}", path.display())?;
    }

    Ok(())
}
