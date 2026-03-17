use std::collections::BTreeSet;
use std::fs;
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
) -> Result<Vec<Value>, McpError> {
    if artifacts.is_empty() {
        return Ok(Vec::new());
    }

    let mut seen_paths = BTreeSet::new();
    let destinations = artifacts
        .iter()
        .map(|artifact| {
            let path = output_dir.join(&artifact.file_name);

            if !seen_paths.insert(path.clone()) {
                return Err(McpError::internal_error(
                    format!(
                        "multiple artifacts would write to the same destination '{}'",
                        path.display()
                    ),
                    None,
                ));
            }

            if path.exists() {
                return Err(McpError::internal_error(
                    format!(
                        "refusing to overwrite existing artifact '{}'",
                        path.display()
                    ),
                    None,
                ));
            }

            Ok((artifact, path))
        })
        .collect::<Result<Vec<_>, _>>()?;

    fs::create_dir_all(output_dir).map_err(|e| {
        McpError::internal_error(
            format!(
                "failed to create output directory '{}': {e}",
                output_dir.display()
            ),
            None,
        )
    })?;

    let mut results = Vec::with_capacity(destinations.len());

    for (artifact, path) in &destinations {
        fs::write(path, &artifact.content).map_err(|e| {
            McpError::internal_error(
                format!("failed to write artifact '{}': {e}", path.display()),
                None,
            )
        })?;

        results.push(serde_json::json!({
            "target": artifact.target,
            "file_name": artifact.file_name,
            "path": path.display().to_string(),
        }));
    }

    Ok(results)
}
