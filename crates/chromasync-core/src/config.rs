use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::CoreError;

/// Records where each installed target writes its generated artifacts.
///
/// Stored at `~/.config/chromasync/config.toml` (see [`config_file_path`]).
/// Consumed by the CLI `generate`, `wallpaper`, and `batch` commands so that an
/// installed target writes to its recorded directory instead of the generic
/// `--output` fallback.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChromasyncConfig {
    #[serde(default)]
    pub targets: Vec<ConfigTarget>,
}

/// One row of the chromasync config under `[[targets]]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigTarget {
    /// Target name (matches the `name` field of the installed target TOML).
    pub name: String,
    /// Directory where this target's generated artifacts are written.
    ///
    /// Stored verbatim; a leading `~` is expanded to the user's home directory
    /// at resolve time (see [`expand_tilde`]).
    pub output_dir: PathBuf,
    /// Location of the installed target file relative to the config root
    /// (e.g. `targets/gtk.toml`).
    pub source: String,
    /// When true, generation overwrites existing artifacts for this target
    /// (acts like a per-target `--force`).
    #[serde(default)]
    pub overwrite: bool,
}

/// Summary of a successful [`install_target`] call, for echoing back to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallSummary {
    pub target_name: String,
    pub target_file: PathBuf,
    pub config_file: PathBuf,
}

const CONFIG_HEADER: &str = "# Managed by `chromasync target install`. Records where each installed target writes its generated artifacts.\n";

/// Resolve the user config directory (`~/.config/chromasync` on Linux).
fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("io", "chromasync", "chromasync")
}

/// Path to the managed chromasync config file, if a user config directory can be located.
pub fn config_file_path() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.config_dir().join("config.toml"))
}

impl ChromasyncConfig {
    /// Load the config from disk, returning an empty config when the file does
    /// not exist yet (the common case before any target has been installed).
    pub fn load() -> Result<Self, CoreError> {
        let Some(path) = config_file_path() else {
            return Ok(Self::default());
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path).map_err(|source| CoreError::ConfigRead {
            path: path.clone(),
            source,
        })?;
        let config: Self =
            toml::from_str(&content).map_err(|error| CoreError::ConfigParse { path, error })?;
        Ok(config)
    }

    /// Write the config back to disk, creating the config directory if needed.
    pub fn save(&self) -> Result<(), CoreError> {
        let path = config_file_path().ok_or(CoreError::UserConfigDirUnavailable)?;
        let body = toml::to_string(self).map_err(|error| CoreError::ConfigSerialize {
            error: error.to_string(),
        })?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::ConfigWrite {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let serialized = format!("{CONFIG_HEADER}\n{body}");
        fs::write(&path, &serialized).map_err(|source| CoreError::ConfigWrite { path, source })?;
        Ok(())
    }

    /// Resolve the effective output directory and force flag for a target name.
    ///
    /// Installed targets (present in the config) drive their own `output_dir`
    /// and `overwrite`. Targets without an entry fall back to the caller-provided
    /// defaults. A global `--force` (`fallback_force = true`) forces every target.
    pub fn resolve(
        &self,
        target_name: &str,
        fallback_dir: &Path,
        fallback_force: bool,
    ) -> (PathBuf, bool) {
        match self.targets.iter().find(|entry| entry.name == target_name) {
            Some(entry) => (
                expand_tilde(&entry.output_dir),
                entry.overwrite || fallback_force,
            ),
            None => (fallback_dir.to_path_buf(), fallback_force),
        }
    }

    /// Insert or replace the config entry for `name`.
    fn upsert(&mut self, entry: ConfigTarget) {
        if let Some(existing) = self.targets.iter_mut().find(|t| t.name == entry.name) {
            *existing = entry;
        } else {
            self.targets.push(entry);
        }
    }
}

/// Install a target TOML into the user config and record its output directory.
///
/// The target file is validated (same rules as discovery), copied to
/// `~/.config/chromasync/targets/<name>.toml`, and a `[[targets]]` entry is
/// upserted into the config. Refuses to replace an already-installed target
/// file unless `overwrite` is set; the same flag is recorded in the config so
/// subsequent generation force-overwrites that target's artifacts.
pub fn install_target(
    target_path: &Path,
    output_dir: PathBuf,
    overwrite: bool,
) -> Result<InstallSummary, CoreError> {
    let spec = chromasync_renderers::parse_target_file(target_path)?;
    let name = spec.name.clone();

    let registry = chromasync_renderers::RendererRegistry::new();
    if registry.contains(&name) {
        return Err(CoreError::Renderer(
            chromasync_renderers::RendererError::TargetNameCollidesWithBuiltIn { name },
        ));
    }

    let targets_dir =
        chromasync_renderers::user_targets_dir().ok_or(CoreError::UserConfigDirUnavailable)?;
    fs::create_dir_all(&targets_dir).map_err(|source| CoreError::CreateTargetsDir {
        path: targets_dir.clone(),
        source,
    })?;

    let dest = targets_dir.join(format!("{name}.toml"));
    if dest.exists() && !overwrite {
        return Err(CoreError::TargetAlreadyInstalled {
            name,
            path: dest.clone(),
        });
    }

    fs::copy(target_path, &dest).map_err(|source| CoreError::CopyTargetFile {
        from: target_path.to_path_buf(),
        to: dest.clone(),
        source,
    })?;

    let mut config = ChromasyncConfig::load()?;
    config.upsert(ConfigTarget {
        name: name.clone(),
        output_dir,
        source: format!("targets/{name}.toml"),
        overwrite,
    });
    config.save()?;

    Ok(InstallSummary {
        target_name: name,
        target_file: dest,
        config_file: config_file_path().ok_or(CoreError::UserConfigDirUnavailable)?,
    })
}

/// Expand a leading `~` or `~/` to the user's home directory.
///
/// Paths without a leading tilde are returned unchanged. When the home
/// directory cannot be determined, the original path is returned as a fallback.
pub fn expand_tilde(path: &Path) -> PathBuf {
    let lossy = path.to_string_lossy();

    if lossy == "~" {
        return home_dir().unwrap_or_else(|| path.to_path_buf());
    }

    if let Some(rest) = lossy.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }

    path.to_path_buf()
}

fn home_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|base| base.home_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn resolve_falls_back_when_target_not_installed() {
        let config = ChromasyncConfig::default();
        let (dir, force) = config.resolve("missing", Path::new("fallback"), false);

        assert_eq!(dir, PathBuf::from("fallback"));
        assert!(!force);
    }

    #[test]
    fn resolve_uses_installed_output_dir_and_overwrite() {
        let config = ChromasyncConfig {
            targets: vec![ConfigTarget {
                name: "gtk".to_owned(),
                output_dir: PathBuf::from("~/.config/gtk-4.0"),
                source: "targets/gtk.toml".to_owned(),
                overwrite: true,
            }],
        };
        let (dir, force) = config.resolve("gtk", Path::new("fallback"), false);

        assert_eq!(dir, expand_tilde(&PathBuf::from("~/.config/gtk-4.0")));
        assert!(force);
    }

    #[test]
    fn resolve_global_force_overrides_installed_overwrite_false() {
        let config = ChromasyncConfig {
            targets: vec![ConfigTarget {
                name: "gtk".to_owned(),
                output_dir: PathBuf::from("/tmp/gtk"),
                source: "targets/gtk.toml".to_owned(),
                overwrite: false,
            }],
        };
        let (_, force) = config.resolve("gtk", Path::new("fallback"), true);

        assert!(force);
    }

    #[test]
    fn upsert_replaces_existing_entry() {
        let mut config = ChromasyncConfig {
            targets: vec![ConfigTarget {
                name: "gtk".to_owned(),
                output_dir: PathBuf::from("/old"),
                source: "targets/gtk.toml".to_owned(),
                overwrite: false,
            }],
        };
        config.upsert(ConfigTarget {
            name: "gtk".to_owned(),
            output_dir: PathBuf::from("/new"),
            source: "targets/gtk.toml".to_owned(),
            overwrite: true,
        });

        assert_eq!(config.targets.len(), 1);
        assert_eq!(config.targets[0].output_dir, PathBuf::from("/new"));
        assert!(config.targets[0].overwrite);
    }

    #[test]
    fn expand_tilde_leaves_absolute_paths_untouched() {
        assert_eq!(
            expand_tilde(&PathBuf::from("/etc/x")),
            PathBuf::from("/etc/x")
        );
        assert_eq!(
            expand_tilde(&PathBuf::from("relative")),
            PathBuf::from("relative")
        );
    }
}
