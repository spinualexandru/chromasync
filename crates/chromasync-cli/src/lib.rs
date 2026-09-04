use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

use anyhow::{Context, Result, bail};
use chromasync_types::{
    ChromaStrategy, ContrastStrategy, GeneratedArtifact, GenerationRequest, ThemeMode,
};
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
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
    /// Run a named sync profile from the user config.
    Sync(SyncArgs),
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
    /// Manage installed renderer targets.
    Target {
        #[command(subcommand)]
        command: TargetCommand,
    },
    /// Show palette families and resolved semantic tokens.
    Preview(PreviewArgs),
    /// Export resolved semantic tokens.
    Tokens(TokensArgs),
    /// Generate shell completion scripts.
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: Shell,
    },
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

#[derive(Debug, Clone, Subcommand)]
enum TargetCommand {
    /// Install a target TOML into the user config and record its output directory.
    Install(TargetInstallArgs),
}

#[derive(Debug, Clone, Args)]
struct TargetInstallArgs {
    /// Path to the target TOML file to install.
    #[arg(long)]
    target: PathBuf,
    /// Directory where this target's generated artifacts should be written.
    #[arg(long)]
    outdir: PathBuf,
    /// Replace an existing installed target and mark it for forced overwrite during generation.
    #[arg(long)]
    overwrite: bool,
}

#[derive(Debug, Clone, Args)]
struct GenerateArgs {
    /// Seed color in #RRGGBB format.
    #[arg(long)]
    seed: String,
    /// Template name or path to a template TOML file. Optional if targets specify preferred_template.
    #[arg(long)]
    template: Option<String>,
    /// Theme mode to generate.
    #[arg(long, value_enum, default_value_t = CliMode::Dark)]
    mode: CliMode,
    /// Contrast selection heuristic used when resolving readable foregrounds.
    #[arg(long, value_enum, default_value_t = CliContrast::RelativeLuminance)]
    contrast: CliContrast,
    /// Chroma strategy used when generating palette families.
    #[arg(long, value_enum, default_value_t = CliChroma::Normal)]
    chroma: CliChroma,
    /// Comma-separated list of target names or target TOML paths to generate.
    #[arg(long, value_delimiter = ',', required = true)]
    targets: Vec<String>,
    /// Output directory for generated artifacts.
    #[arg(long, default_value = "chromasync")]
    output: PathBuf,
    /// Overwrite existing artifacts instead of refusing when a file already exists at the destination.
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Clone, Args)]
struct WallpaperArgs {
    /// Wallpaper image path.
    #[arg(long)]
    image: PathBuf,
    /// Template name or path to a template TOML file. Optional if targets specify preferred_template.
    #[arg(long)]
    template: Option<String>,
    /// Theme mode to generate.
    #[arg(long, value_enum, default_value_t = CliMode::Dark)]
    mode: CliMode,
    /// Contrast selection heuristic used when resolving readable foregrounds.
    #[arg(long, value_enum, default_value_t = CliContrast::RelativeLuminance)]
    contrast: CliContrast,
    /// Chroma strategy used when generating palette families.
    #[arg(long, value_enum, default_value_t = CliChroma::Normal)]
    chroma: CliChroma,
    /// Comma-separated list of target names or target TOML paths to generate.
    #[arg(long, value_delimiter = ',', required = true)]
    targets: Vec<String>,
    /// Output directory for generated artifacts.
    #[arg(long, default_value = "chromasync")]
    output: PathBuf,
    /// Overwrite existing artifacts instead of refusing when a file already exists at the destination.
    #[arg(long)]
    force: bool,
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
    /// Chroma strategy used when generating palette families.
    #[arg(long, value_enum, default_value_t = CliChroma::Normal)]
    chroma: CliChroma,
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
    /// Chroma strategy used when generating palette families.
    #[arg(long, value_enum, default_value_t = CliChroma::Normal)]
    chroma: CliChroma,
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

#[derive(Debug, Clone, Args)]
struct SyncArgs {
    /// Profile name under [[configs]]. Defaults to "default".
    profile: Option<String>,
    /// Override the theme mode configured by the selected profile.
    #[arg(long, value_enum)]
    mode: Option<CliMode>,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliChroma {
    Subtle,
    Normal,
    Vibrant,
    Muted,
    Industrial,
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
    template: Option<String>,
    #[serde(default)]
    mode: ThemeMode,
    #[serde(default)]
    contrast: ContrastStrategy,
    #[serde(default)]
    chroma: ChromaStrategy,
    #[serde(default)]
    targets: Vec<String>,
    output: PathBuf,
    #[serde(default)]
    force: bool,
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

impl From<CliChroma> for ChromaStrategy {
    fn from(value: CliChroma) -> Self {
        match value {
            CliChroma::Subtle => Self::Subtle,
            CliChroma::Normal => Self::Normal,
            CliChroma::Vibrant => Self::Vibrant,
            CliChroma::Muted => Self::Muted,
            CliChroma::Industrial => Self::Industrial,
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
            chroma: self.chroma.into(),
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
            chroma: self.chroma.into(),
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
            template: Some(self.template),
            mode: self.mode.into(),
            contrast: self.contrast.into(),
            chroma: self.chroma.into(),
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
            template: Some(self.template),
            mode: self.mode.into(),
            contrast: self.contrast.into(),
            chroma: self.chroma.into(),
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
        Command::Generate(_)
        | Command::Wallpaper(_)
        | Command::Batch(_)
        | Command::Sync(_)
        | Command::Targets => Some(chromasync_core::load_output_registry()?),
        Command::Templates
        | Command::Packs
        | Command::Pack { .. }
        | Command::Target { .. }
        | Command::Preview(_)
        | Command::Tokens(_)
        | Command::Completions { .. } => None,
    };

    let config = match &cli.command {
        Command::Generate(_) | Command::Wallpaper(_) | Command::Batch(_) | Command::Sync(_) => {
            Some(chromasync_core::ChromasyncConfig::load()?)
        }
        Command::Targets
        | Command::Templates
        | Command::Packs
        | Command::Pack { .. }
        | Command::Target { .. }
        | Command::Preview(_)
        | Command::Tokens(_)
        | Command::Completions { .. } => None,
    };

    match cli.command {
        Command::Generate(args) => {
            let force = args.force;
            let request = args.into_request()?;
            let artifacts = generate_routed_artifacts(
                &request,
                output_registry
                    .as_ref()
                    .expect("output registry should be loaded for generate"),
                config
                    .as_ref()
                    .expect("config should be loaded for generate"),
                force,
            )?;

            write_and_print_routed(&artifacts).map(|_| ())
        }
        Command::Wallpaper(args) => {
            let force = args.force;
            let request = args.into_request()?;
            let artifacts = generate_routed_artifacts(
                &request,
                output_registry
                    .as_ref()
                    .expect("output registry should be loaded for wallpaper"),
                config
                    .as_ref()
                    .expect("config should be loaded for wallpaper"),
                force,
            )?;

            write_and_print_routed(&artifacts).map(|_| ())
        }
        Command::Batch(args) => run_batch(
            args,
            output_registry
                .as_ref()
                .expect("output registry should be loaded for batch"),
            config.as_ref().expect("config should be loaded for batch"),
        ),
        Command::Sync(args) => run_sync(
            args,
            output_registry
                .as_ref()
                .expect("output registry should be loaded for sync"),
            config.as_ref().expect("config should be loaded for sync"),
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
        Command::Target { command } => match command {
            TargetCommand::Install(args) => run_target_install(args),
        },
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
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "chromasync", &mut std::io::stdout());
            Ok(())
        }
    }
}

fn generate_routed_artifacts(
    request: &GenerationRequest,
    output_registry: &chromasync_core::OutputRegistry,
    config: &chromasync_core::ChromasyncConfig,
    fallback_force: bool,
) -> Result<Vec<chromasync_core::RoutedArtifact>> {
    chromasync_core::generate_routed_with_output_registry(request, output_registry, |target| {
        output_route_for_target(target, request, fallback_force, config)
    })
    .map_err(Into::into)
}

fn output_route_for_target(
    target: &str,
    request: &GenerationRequest,
    fallback_force: bool,
    config: &chromasync_core::ChromasyncConfig,
) -> (PathBuf, bool) {
    if !looks_like_path(target) {
        config.resolve(target, &request.output_dir, fallback_force)
    } else {
        (request.output_dir.clone(), fallback_force)
    }
}

fn run_batch(
    args: BatchArgs,
    output_registry: &chromasync_core::OutputRegistry,
    config: &chromasync_core::ChromasyncConfig,
) -> Result<()> {
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
        let force = job.force;
        let request = batch_job_into_request(job, &manifest_dir)?;
        let artifacts = generate_routed_artifacts(&request, output_registry, config, force)
            .with_context(|| {
                format!(
                    "batch job {} failed for output '{}'",
                    index + 1,
                    request.output_dir.display()
                )
            })?;

        write_and_print_routed(&artifacts)?;
    }

    Ok(())
}

fn run_sync(
    args: SyncArgs,
    output_registry: &chromasync_core::OutputRegistry,
    config: &chromasync_core::ChromasyncConfig,
) -> Result<()> {
    let profile_name = args.profile.unwrap_or_else(|| "default".to_owned());
    let config_path = chromasync_core::config_file_path()
        .context("could not resolve the chromasync user config directory")?;

    if !config_path.exists() {
        bail!(
            "chromasync config '{}' does not exist; create a [[configs]] profile first",
            config_path.display()
        );
    }

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let profile = config.sync_profile(&profile_name).ok_or_else(|| {
        anyhow::anyhow!(
            "sync profile '{}' was not found in '{}'",
            profile_name,
            config_path.display()
        )
    })?;

    let force = profile.force;
    let request = sync_profile_into_request(profile, config_dir, args.mode.map(ThemeMode::from))?;
    let artifacts = generate_routed_artifacts(&request, output_registry, config, force)
        .with_context(|| {
            format!(
                "sync profile '{}' failed for output '{}'",
                profile.name,
                request.output_dir.display()
            )
        })?;

    let report = write_and_print_routed(&artifacts)?;
    run_matching_hooks(config, &profile.name, config_dir, &report)
}

fn sync_profile_into_request(
    profile: &chromasync_core::SyncProfile,
    config_dir: &Path,
    mode_override: Option<ThemeMode>,
) -> Result<GenerationRequest> {
    let color_source_count = usize::from(profile.seed.is_some())
        + usize::from(profile.image.is_some())
        + usize::from(profile.image_fetch_command.is_some());

    if color_source_count != 1 {
        bail!(
            "sync profile '{}' must define exactly one of 'seed', 'image', or 'image_fetch_command'",
            profile.name
        );
    }

    if profile.targets.is_empty() {
        bail!(
            "sync profile '{}' must define at least one target",
            profile.name
        );
    }

    Ok(GenerationRequest {
        seed: profile.seed.clone(),
        wallpaper: sync_profile_wallpaper(profile, config_dir)?,
        template: profile
            .template
            .as_ref()
            .map(|template| resolve_template_reference(config_dir, template)),
        mode: mode_override.unwrap_or_else(|| resolve_sync_mode(profile.mode)),
        contrast: profile.contrast,
        chroma: profile.chroma,
        targets: normalize_targets_relative_to(config_dir, profile.targets.clone())?,
        output_dir: resolve_relative_path(config_dir, &profile.output_dir),
    })
}

fn sync_profile_wallpaper(
    profile: &chromasync_core::SyncProfile,
    config_dir: &Path,
) -> Result<Option<PathBuf>> {
    if let Some(command) = &profile.image_fetch_command {
        return fetch_sync_profile_image(&profile.name, command, config_dir).map(Some);
    }

    Ok(profile
        .image
        .as_ref()
        .map(|path| resolve_relative_path(config_dir, path)))
}

fn fetch_sync_profile_image(
    profile_name: &str,
    command_line: &str,
    config_dir: &Path,
) -> Result<PathBuf> {
    let output = shell_command(command_line)
        .current_dir(config_dir)
        .output()
        .with_context(|| {
            format!("sync profile '{profile_name}' image_fetch_command failed to start")
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        if detail.is_empty() {
            bail!(
                "sync profile '{}' image_fetch_command exited with {}",
                profile_name,
                output.status
            );
        }

        bail!(
            "sync profile '{}' image_fetch_command exited with {}: {}",
            profile_name,
            output.status,
            detail
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(path) = stdout.lines().map(str::trim).find(|line| !line.is_empty()) else {
        bail!("sync profile '{profile_name}' image_fetch_command did not print an image path");
    };

    Ok(resolve_relative_path(config_dir, Path::new(path)))
}

fn shell_command(command_line: &str) -> ProcessCommand {
    #[cfg(windows)]
    {
        let mut command = ProcessCommand::new("cmd");
        command.args(["/C", command_line]);
        command
    }

    #[cfg(not(windows))]
    {
        let mut command = ProcessCommand::new("sh");
        command.args(["-c", command_line]);
        command
    }
}

fn resolve_sync_mode(mode: chromasync_core::SyncMode) -> ThemeMode {
    match mode {
        chromasync_core::SyncMode::Light => ThemeMode::Light,
        chromasync_core::SyncMode::Dark => ThemeMode::Dark,
        chromasync_core::SyncMode::Auto => detect_desktop_theme_mode(),
    }
}

fn detect_desktop_theme_mode() -> ThemeMode {
    let output = ProcessCommand::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            desktop_mode_from_gsettings_output(&stdout).unwrap_or(ThemeMode::Dark)
        }
        _ => ThemeMode::Dark,
    }
}

fn desktop_mode_from_gsettings_output(output: &str) -> Option<ThemeMode> {
    let value = output
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .to_ascii_lowercase();

    match value.as_str() {
        "prefer-light" => Some(ThemeMode::Light),
        "prefer-dark" => Some(ThemeMode::Dark),
        _ => None,
    }
}

fn batch_job_into_request(job: BatchJob, base_dir: &Path) -> Result<GenerationRequest> {
    if job.seed.is_some() == job.image.is_some() {
        let job_label = job.name.as_deref().unwrap_or("<unnamed>");

        bail!("batch job '{job_label}' must define exactly one of 'seed' or 'image'");
    }

    Ok(GenerationRequest {
        seed: job.seed,
        wallpaper: job.image.map(|path| resolve_relative_path(base_dir, &path)),
        template: job
            .template
            .map(|t| resolve_template_reference(base_dir, &t)),
        mode: job.mode,
        contrast: job.contrast,
        chroma: job.chroma,
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

fn write_and_print_routed(artifacts: &[chromasync_core::RoutedArtifact]) -> Result<WriteReport> {
    let entries: Vec<chromasync_core::ResolvedArtifact> = artifacts
        .iter()
        .map(|artifact| chromasync_core::ResolvedArtifact {
            output_dir: artifact.output_dir.clone(),
            file_name: artifact.artifact.file_name.clone(),
            content: artifact.artifact.content.clone(),
            force: artifact.force,
        })
        .collect();

    let written = chromasync_core::write_resolved_artifacts(&entries)?;
    let generated_artifacts = artifacts
        .iter()
        .map(|artifact| artifact.artifact.clone())
        .collect::<Vec<_>>();
    let report = WriteReport::new(&generated_artifacts);

    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for path in &written {
        writeln!(stdout, "{}", path.display())?;
    }

    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriteReport {
    events: BTreeSet<String>,
}

impl WriteReport {
    fn new(artifacts: &[GeneratedArtifact]) -> Self {
        let mut events = artifacts
            .iter()
            .map(|artifact| format!("target:{}:done", artifact.target))
            .collect::<BTreeSet<_>>();

        if !artifacts.is_empty() {
            events.insert("targets:done".to_owned());
        }

        Self { events }
    }

    fn has_event(&self, event: &str) -> bool {
        self.events.contains(event)
    }
}

fn run_matching_hooks(
    config: &chromasync_core::ChromasyncConfig,
    profile_name: &str,
    config_dir: &Path,
    report: &WriteReport,
) -> Result<()> {
    for hook in &config.hooks {
        if hook_matches(hook, profile_name, report) {
            run_hook(hook, config_dir)?;
        }
    }

    Ok(())
}

fn hook_matches(
    hook: &chromasync_core::ConfigHook,
    profile_name: &str,
    report: &WriteReport,
) -> bool {
    hook.on.iter().any(|event| report.has_event(event))
        && hook
            .filters
            .iter()
            .all(|filter| filter == &format!("config:{profile_name}"))
}

fn run_hook(hook: &chromasync_core::ConfigHook, config_dir: &Path) -> Result<()> {
    let output = shell_command(&hook.command)
        .current_dir(config_dir)
        .output()
        .with_context(|| format!("hook '{}' failed to start", hook.name))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr.trim();
    if detail.is_empty() {
        bail!("hook '{}' exited with {}", hook.name, output.status);
    }

    bail!(
        "hook '{}' exited with {}: {}",
        hook.name,
        output.status,
        detail
    )
}

fn run_target_install(args: TargetInstallArgs) -> Result<()> {
    let summary = chromasync_core::install_target(&args.target, args.outdir, args.overwrite)?;

    let mut stdout = io::BufWriter::new(io::stdout().lock());
    writeln!(stdout, "{}", summary.target_file.display())?;
    writeln!(stdout, "{}", summary.config_file.display())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ThemeMode, desktop_mode_from_gsettings_output};

    #[test]
    fn gsettings_prefer_light_maps_to_light_mode() {
        assert_eq!(
            desktop_mode_from_gsettings_output("'prefer-light'\n"),
            Some(ThemeMode::Light)
        );
    }

    #[test]
    fn gsettings_prefer_dark_maps_to_dark_mode() {
        assert_eq!(
            desktop_mode_from_gsettings_output("'prefer-dark'\n"),
            Some(ThemeMode::Dark)
        );
    }

    #[test]
    fn unknown_gsettings_color_scheme_is_not_inferred() {
        assert_eq!(desktop_mode_from_gsettings_output("'default'\n"), None);
    }
}
