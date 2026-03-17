use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use chromasync_types::ThemePack;
use directories::ProjectDirs;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Default)]
pub struct PackRegistry {
    packs: Vec<ThemePack>,
}

impl PackRegistry {
    pub fn discover() -> Result<Self, PackError> {
        Self::discover_in(&pack_search_roots())
    }

    pub fn discover_in(search_roots: &[PathBuf]) -> Result<Self, PackError> {
        let mut packs = Vec::new();
        let mut seen_names = std::collections::BTreeMap::new();

        for root in search_roots {
            if !root.exists() {
                continue;
            }

            let mut candidates = Vec::new();

            for entry in fs::read_dir(root).map_err(|source| PackError::ReadPacksDir {
                path: root.to_path_buf(),
                source,
            })? {
                let entry = entry.map_err(|source| PackError::ReadPacksDir {
                    path: root.to_path_buf(),
                    source,
                })?;
                let path = entry.path();

                if path.is_dir() && path.join("pack.toml").is_file() {
                    candidates.push(path);
                }
            }

            candidates.sort();

            for candidate in candidates {
                let pack = load_pack(&candidate)?;

                if let Some(previous) =
                    seen_names.insert(pack.name.clone(), pack.root_dir.display().to_string())
                {
                    return Err(PackError::DuplicatePackName {
                        name: pack.name,
                        first_source: previous,
                        second_source: candidate.display().to_string(),
                    });
                }

                packs.push(pack);
            }
        }

        packs.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(Self { packs })
    }

    pub fn packs(&self) -> &[ThemePack] {
        &self.packs
    }

    pub fn get(&self, name: &str) -> Option<&ThemePack> {
        self.packs.iter().find(|pack| pack.name == name)
    }
}

#[derive(Debug, Error)]
pub enum PackError {
    #[error("failed to read pack directory '{path}': {source}")]
    ReadPacksDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read pack manifest '{path}': {source}")]
    ReadPackManifest {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse pack manifest '{path}': {error}")]
    ParsePackManifest {
        path: PathBuf,
        #[source]
        error: Box<toml::de::Error>,
    },
    #[error("pack manifest '{path}' uses an invalid pack name '{name}'")]
    InvalidPackName { path: PathBuf, name: String },
    #[error(
        "pack '{name}' is defined multiple times: first at {first_source}, second at {second_source}"
    )]
    DuplicatePackName {
        name: String,
        first_source: String,
        second_source: String,
    },
    #[error("pack '{pack}' {kind} path '{path}' is invalid: {reason}")]
    InvalidAssetPath {
        pack: String,
        kind: &'static str,
        path: String,
        reason: String,
    },
    #[error("pack '{pack}' {kind} path '{path}' does not exist or is not a directory")]
    MissingAssetDir {
        pack: String,
        kind: &'static str,
        path: PathBuf,
    },
    #[error("pack '{pack}' does not define any templates or targets")]
    PackHasNoAssets { pack: String },
    #[error("pack '{pack}' was not found")]
    PackNotFound { pack: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackManifest {
    name: String,
    version: String,
    description: Option<String>,
    author: Option<String>,
    license: Option<String>,
    homepage: Option<String>,
    templates: Option<RawPackAssets>,
    targets: Option<RawPackAssets>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackAssets {
    #[serde(default)]
    paths: Vec<PathBuf>,
}

pub fn pack_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(dirs) = ProjectDirs::from("io", "chromasync", "chromasync") {
        roots.push(dirs.config_dir().join("packs"));
        roots.push(dirs.data_local_dir().join("packs"));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir.join(".chromasync").join("packs"));
    }

    let mut seen = BTreeSet::new();
    roots.retain(|path| seen.insert(path.clone()));

    roots
}

fn load_pack(path: &Path) -> Result<ThemePack, PackError> {
    let manifest_path = path.join("pack.toml");
    let content =
        fs::read_to_string(&manifest_path).map_err(|source| PackError::ReadPackManifest {
            path: manifest_path.clone(),
            source,
        })?;
    let manifest: RawPackManifest =
        toml::from_str(&content).map_err(|error| PackError::ParsePackManifest {
            path: manifest_path.clone(),
            error: Box::new(error),
        })?;

    validate_pack_name(&manifest_path, &manifest.name)?;

    let template_dirs = resolve_asset_dirs(
        path,
        &manifest.name,
        "template",
        &manifest.templates,
        "templates",
    )?;
    let target_dirs =
        resolve_asset_dirs(path, &manifest.name, "target", &manifest.targets, "targets")?;

    if template_dirs.is_empty() && target_dirs.is_empty() {
        return Err(PackError::PackHasNoAssets {
            pack: manifest.name.clone(),
        });
    }

    Ok(ThemePack {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        author: manifest.author,
        license: manifest.license,
        homepage: manifest.homepage,
        root_dir: path.to_path_buf(),
        template_dirs,
        target_dirs,
    })
}

fn validate_pack_name(path: &Path, name: &str) -> Result<(), PackError> {
    if !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        Ok(())
    } else {
        Err(PackError::InvalidPackName {
            path: path.to_path_buf(),
            name: name.to_owned(),
        })
    }
}

fn resolve_asset_dirs(
    pack_root: &Path,
    pack_name: &str,
    kind: &'static str,
    manifest_paths: &Option<RawPackAssets>,
    default_dir: &'static str,
) -> Result<Vec<PathBuf>, PackError> {
    let declared = match manifest_paths {
        Some(paths) => paths.paths.clone(),
        None => {
            let default_path = pack_root.join(default_dir);

            if default_path.is_dir() {
                vec![PathBuf::from(default_dir)]
            } else {
                Vec::new()
            }
        }
    };
    let mut resolved = Vec::new();
    let mut seen = BTreeSet::new();

    for relative in declared {
        validate_relative_asset_path(pack_name, kind, &relative)?;
        let absolute = pack_root.join(&relative);

        if !absolute.is_dir() {
            return Err(PackError::MissingAssetDir {
                pack: pack_name.to_owned(),
                kind,
                path: absolute,
            });
        }

        if seen.insert(absolute.clone()) {
            resolved.push(absolute);
        }
    }

    Ok(resolved)
}

fn validate_relative_asset_path(
    pack_name: &str,
    kind: &'static str,
    path: &Path,
) -> Result<(), PackError> {
    if path.as_os_str().is_empty() {
        return Err(PackError::InvalidAssetPath {
            pack: pack_name.to_owned(),
            kind,
            path: path.display().to_string(),
            reason: "expected a non-empty relative directory".to_owned(),
        });
    }

    if path.is_absolute() {
        return Err(PackError::InvalidAssetPath {
            pack: pack_name.to_owned(),
            kind,
            path: path.display().to_string(),
            reason: "absolute paths are not allowed".to_owned(),
        });
    }

    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => {
                return Err(PackError::InvalidAssetPath {
                    pack: pack_name.to_owned(),
                    kind,
                    path: path.display().to_string(),
                    reason: "parent path components are not allowed".to_owned(),
                });
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(PackError::InvalidAssetPath {
                    pack: pack_name.to_owned(),
                    kind,
                    path: path.display().to_string(),
                    reason: "only relative subdirectories are allowed".to_owned(),
                });
            }
        }
    }

    Ok(())
}
