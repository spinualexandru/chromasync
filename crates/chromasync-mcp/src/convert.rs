use std::path::Path;

use chromasync_core::CoreError;
use chromasync_types::{ChromaStrategy, ContrastStrategy, GeneratedArtifact, ThemeMode};
use rmcp::ErrorData as McpError;
use serde_json::Value;

pub fn parse_mode(s: &str) -> Result<ThemeMode, String> {
    match s {
        "dark" => Ok(ThemeMode::Dark),
        "light" => Ok(ThemeMode::Light),
        other => Err(format!(
            "invalid mode '{other}': expected \"dark\" or \"light\""
        )),
    }
}

pub fn parse_contrast(s: &str) -> Result<ContrastStrategy, String> {
    match s {
        "relative-luminance" => Ok(ContrastStrategy::RelativeLuminance),
        "apca-experimental" => Ok(ContrastStrategy::ApcaExperimental),
        other => Err(format!(
            "invalid contrast strategy '{other}': expected \"relative-luminance\" or \"apca-experimental\""
        )),
    }
}

pub fn parse_chroma(s: &str) -> Result<ChromaStrategy, String> {
    match s {
        "subtle" => Ok(ChromaStrategy::Subtle),
        "normal" => Ok(ChromaStrategy::Normal),
        "vibrant" => Ok(ChromaStrategy::Vibrant),
        "muted" => Ok(ChromaStrategy::Muted),
        "industrial" => Ok(ChromaStrategy::Industrial),
        other => Err(format!(
            "invalid chroma strategy '{other}': expected \"subtle\", \"normal\", \"vibrant\", \"muted\", or \"industrial\""
        )),
    }
}

pub fn core_error_to_mcp(err: CoreError) -> McpError {
    McpError::internal_error(err.to_string(), None)
}

pub fn string_error_to_mcp(err: String) -> McpError {
    McpError::internal_error(err, None)
}

pub fn write_artifacts(
    output_dir: &Path,
    artifacts: &[GeneratedArtifact],
    force: bool,
) -> Result<Vec<Value>, McpError> {
    let paths = chromasync_core::write_artifacts(output_dir, artifacts, force)
        .map_err(core_error_to_mcp)?;

    Ok(artifacts
        .iter()
        .zip(&paths)
        .map(|(artifact, path)| {
            serde_json::json!({
                "target": artifact.target,
                "file_name": artifact.file_name,
                "path": path.display().to_string(),
            })
        })
        .collect())
}
