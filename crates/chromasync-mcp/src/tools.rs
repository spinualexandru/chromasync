use std::path::{Path, PathBuf};

use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::CallToolResult,
    model::Content, tool, tool_router,
};

use crate::ChromasyncServer;
use crate::convert::{core_error_to_mcp, string_error_to_mcp, write_artifacts};
use crate::params::{
    BatchParams, ExportTokensParams, GeneratePaletteParams, GenerateParams, PackInfoParams,
    PreviewParams, WallpaperParams, build_generation_request,
};

#[tool_router]
impl ChromasyncServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Generate theme artifacts from a seed color and write them to disk")]
    fn generate(
        &self,
        Parameters(params): Parameters<GenerateParams>,
    ) -> Result<CallToolResult, McpError> {
        let request = build_generation_request(
            Some(params.seed),
            None,
            params.template,
            params.mode,
            params.contrast,
            params.targets,
            params.output_dir.clone(),
        )
        .map_err(string_error_to_mcp)?;

        let artifacts = chromasync_core::generate(&request).map_err(core_error_to_mcp)?;
        let written = write_artifacts(Path::new(&params.output_dir), &artifacts)?;
        let json = serde_json::to_string_pretty(&written)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Generate theme artifacts from a wallpaper image and write them to disk")]
    fn wallpaper(
        &self,
        Parameters(params): Parameters<WallpaperParams>,
    ) -> Result<CallToolResult, McpError> {
        let request = build_generation_request(
            None,
            Some(PathBuf::from(&params.image)),
            params.template,
            params.mode,
            params.contrast,
            params.targets,
            params.output_dir.clone(),
        )
        .map_err(string_error_to_mcp)?;

        let artifacts =
            chromasync_core::generate_from_wallpaper(&request).map_err(core_error_to_mcp)?;
        let written = write_artifacts(Path::new(&params.output_dir), &artifacts)?;
        let json = serde_json::to_string_pretty(&written)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Execute a TOML batch manifest containing multiple generation jobs")]
    fn batch(
        &self,
        Parameters(params): Parameters<BatchParams>,
    ) -> Result<CallToolResult, McpError> {
        let manifest_path = PathBuf::from(&params.manifest);
        let manifest_dir = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
            McpError::internal_error(
                format!(
                    "failed to read batch manifest '{}': {e}",
                    manifest_path.display()
                ),
                None,
            )
        })?;

        let manifest: BatchManifest = toml::from_str(&content).map_err(|e| {
            McpError::internal_error(
                format!(
                    "failed to parse batch manifest '{}': {e}",
                    manifest_path.display()
                ),
                None,
            )
        })?;

        if manifest.jobs.is_empty() {
            return Err(McpError::internal_error(
                format!(
                    "batch manifest '{}' does not define any jobs",
                    manifest_path.display()
                ),
                None,
            ));
        }

        let output_registry = chromasync_core::load_output_registry().map_err(core_error_to_mcp)?;

        let mut all_results = Vec::new();

        for (index, job) in manifest.jobs.into_iter().enumerate() {
            let job_name = job.name.clone();
            let request = batch_job_into_request(job, &manifest_dir)?;
            let artifacts = if request.wallpaper.is_some() {
                chromasync_core::generate_from_wallpaper_with_output_registry(
                    &request,
                    &output_registry,
                )
            } else {
                chromasync_core::generate_with_output_registry(&request, &output_registry)
            }
            .map_err(|e| {
                McpError::internal_error(
                    format!(
                        "batch job {} failed for output '{}': {e}",
                        index + 1,
                        request.output_dir.display()
                    ),
                    None,
                )
            })?;

            let written = write_artifacts(&request.output_dir, &artifacts)?;
            all_results.push(serde_json::json!({
                "job": job_name.unwrap_or_else(|| format!("job_{}", index + 1)),
                "output_dir": request.output_dir.display().to_string(),
                "artifacts": written,
            }));
        }

        let json = serde_json::to_string_pretty(&all_results)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Preview palette families and resolved semantic tokens for a seed color")]
    fn preview(
        &self,
        Parameters(params): Parameters<PreviewParams>,
    ) -> Result<CallToolResult, McpError> {
        let request = build_generation_request(
            Some(params.seed),
            None,
            params.template,
            params.mode,
            params.contrast,
            Vec::new(),
            "chromasync".to_owned(),
        )
        .map_err(string_error_to_mcp)?;

        let output = chromasync_core::preview(&request).map_err(core_error_to_mcp)?;

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(description = "Export the 17 resolved semantic token hex values as JSON")]
    fn export_tokens(
        &self,
        Parameters(params): Parameters<ExportTokensParams>,
    ) -> Result<CallToolResult, McpError> {
        let request = build_generation_request(
            Some(params.seed),
            None,
            params.template,
            params.mode,
            params.contrast,
            Vec::new(),
            "chromasync".to_owned(),
        )
        .map_err(string_error_to_mcp)?;

        let tokens = chromasync_core::export_tokens(&request).map_err(core_error_to_mcp)?;
        let json = serde_json::to_string_pretty(&tokens)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "Generate the full OKLCH palette (9 families, 16 tones each) from a seed color"
    )]
    fn generate_palette(
        &self,
        Parameters(params): Parameters<GeneratePaletteParams>,
    ) -> Result<CallToolResult, McpError> {
        let mode = crate::convert::parse_mode(&params.mode).map_err(string_error_to_mcp)?;
        let palette =
            chromasync_core::generate_palette(&params.seed, mode).map_err(core_error_to_mcp)?;
        let json = serde_json::to_string_pretty(&palette)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        description = "List all available templates with their name, mode, source, and location"
    )]
    fn list_templates(&self) -> Result<CallToolResult, McpError> {
        let templates = chromasync_core::list_templates().map_err(core_error_to_mcp)?;
        let entries: Vec<serde_json::Value> = templates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.definition.name,
                    "mode": t.definition.mode.as_str(),
                    "description": t.definition.description,
                    "source": t.source.label(),
                    "location": t.source.location(),
                })
            })
            .collect();

        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List all available render targets with their name, source, and location")]
    fn list_targets(&self) -> Result<CallToolResult, McpError> {
        let targets = chromasync_core::list_targets().map_err(core_error_to_mcp)?;
        let entries: Vec<serde_json::Value> = targets
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "source": t.source.label(),
                    "location": t.source.location(),
                })
            })
            .collect();

        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "List all discovered theme packs")]
    fn list_packs(&self) -> Result<CallToolResult, McpError> {
        let packs = chromasync_core::list_packs().map_err(core_error_to_mcp)?;
        let entries: Vec<serde_json::Value> = packs
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "version": p.version,
                    "description": p.description,
                    "root_dir": p.root_dir.display().to_string(),
                })
            })
            .collect();

        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get metadata, templates, and targets for a specific theme pack")]
    fn pack_info(
        &self,
        Parameters(params): Parameters<PackInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let info = chromasync_core::pack_info(&params.name).map_err(core_error_to_mcp)?;

        let templates: Vec<serde_json::Value> = info
            .templates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.definition.name,
                    "mode": t.definition.mode.as_str(),
                    "description": t.definition.description,
                    "location": t.source.location(),
                })
            })
            .collect();

        let targets: Vec<serde_json::Value> = info
            .targets
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "location": t.source.location(),
                })
            })
            .collect();

        let result = serde_json::json!({
            "name": info.pack.name,
            "version": info.pack.version,
            "description": info.pack.description,
            "author": info.pack.author,
            "license": info.pack.license,
            "homepage": info.pack.homepage,
            "root_dir": info.pack.root_dir.display().to_string(),
            "templates": templates,
            "targets": targets,
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

// Batch manifest types (mirrored from CLI)

#[derive(Debug, serde::Deserialize)]
struct BatchManifest {
    #[serde(default, alias = "job")]
    jobs: Vec<BatchJob>,
}

#[derive(Debug, serde::Deserialize)]
struct BatchJob {
    name: Option<String>,
    seed: Option<String>,
    image: Option<PathBuf>,
    template: String,
    #[serde(default)]
    mode: chromasync_types::ThemeMode,
    #[serde(default)]
    contrast: chromasync_types::ContrastStrategy,
    #[serde(default)]
    targets: Vec<String>,
    output: PathBuf,
}

fn batch_job_into_request(
    job: BatchJob,
    base_dir: &Path,
) -> Result<chromasync_types::GenerationRequest, McpError> {
    if job.seed.is_some() == job.image.is_some() {
        let job_label = job.name.as_deref().unwrap_or("<unnamed>");
        return Err(McpError::internal_error(
            format!("batch job '{job_label}' must define exactly one of 'seed' or 'image'"),
            None,
        ));
    }

    let targets = job
        .targets
        .into_iter()
        .map(|t| resolve_reference(base_dir, &t))
        .collect();

    Ok(chromasync_types::GenerationRequest {
        seed: job.seed,
        wallpaper: job.image.map(|p| resolve_path(base_dir, &p)),
        template: resolve_reference(base_dir, &job.template),
        mode: job.mode,
        contrast: job.contrast,
        targets,
        output_dir: resolve_path(base_dir, &job.output),
    })
}

fn resolve_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn resolve_reference(base_dir: &Path, value: &str) -> String {
    let path = Path::new(value);
    if path.is_absolute()
        || value.contains(std::path::MAIN_SEPARATOR)
        || path.extension().and_then(|e| e.to_str()) == Some("toml")
    {
        resolve_path(base_dir, path).display().to_string()
    } else {
        value.to_owned()
    }
}
