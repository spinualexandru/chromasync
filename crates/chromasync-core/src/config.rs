use std::{
    collections::BTreeSet,
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use chromasync_types::{ChromaStrategy, ContrastStrategy};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::CoreError;

/// Defines sync profiles and records where each configured target writes its
/// generated artifacts.
///
/// Stored at `~/.config/chromasync/config.toml` (see [`config_file_path`]).
/// Consumed by the CLI `generate`, `wallpaper`, `batch`, and `sync` commands so
/// that a configured target writes to its recorded directory instead of the
/// generic `--output` fallback.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChromasyncConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configs: Vec<SyncProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<ConfigTarget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<ConfigHook>,
}

/// One runnable generation profile under `[[configs]]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncProfile {
    /// Profile name selected by `chromasync sync [name]`.
    pub name: String,
    /// Seed color in #RRGGBB format. Mutually exclusive with `image`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    /// Wallpaper image path. Mutually exclusive with `seed` and
    /// `image_fetch_command`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<PathBuf>,
    /// Shell command whose stdout returns a wallpaper image path. Mutually
    /// exclusive with `seed` and `image`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_fetch_command: Option<String>,
    /// Template name or path to a template TOML file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Theme mode, with `auto` resolved by the CLI at runtime.
    #[serde(default)]
    pub mode: SyncMode,
    /// Contrast selection heuristic used when resolving readable foregrounds.
    #[serde(default)]
    pub contrast: ContrastStrategy,
    /// Chroma strategy used when generating palette families.
    #[serde(default)]
    pub chroma: ChromaStrategy,
    /// Target names or target TOML paths to generate.
    #[serde(default)]
    pub targets: Vec<String>,
    /// Fallback output directory for targets without a recorded `[[targets]]` entry.
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    /// Overwrite existing artifacts for targets without per-target overwrite.
    #[serde(default)]
    pub force: bool,
}

/// Sync profile mode. `Auto` is deliberately separate from `ThemeMode` because
/// resolving the user's current desktop preference belongs at the CLI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    Light,
    #[default]
    Dark,
    Auto,
}

/// One row of the chromasync config under `[[targets]]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// When true, generation overwrites existing artifacts for this target
    /// (acts like a per-target `--force`).
    #[serde(default)]
    pub overwrite: bool,
}

/// One command hook under `[[hooks]]`.
///
/// Events and filters are stored as written so the config round-trips
/// unchanged; [`ConfigHook::validate`] parses them into [`HookEvent`] and
/// [`HookFilter`] values. Validation is deliberately not part of
/// [`ChromasyncConfig::load`], so a hook typo cannot break commands that never
/// run hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigHook {
    /// Unique hook name used in messages. Surrounding whitespace is ignored.
    pub name: String,
    /// Event or events that trigger this hook.
    pub on: HookEvents,
    /// Shell command to execute when this hook matches.
    pub command: String,
    /// Optional filters, such as `config:default`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<String>,
}

/// Hook event list that accepts either `on = "event"` or `on = ["event"]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookEvents {
    One(String),
    Many(Vec<String>),
}

impl HookEvents {
    pub fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        match self {
            Self::One(event) => std::slice::from_ref(event),
            Self::Many(events) => events.as_slice(),
        }
        .iter()
        .map(String::as_str)
    }
}

/// A parsed hook event.
///
/// `Done` events fire on every successful sync that generated artifacts,
/// whether or not any file had to be rewritten. `Changed` events fire only
/// when at least one artifact's content differed from the file already on
/// disk, so that file was written; prefer them for reload hooks.
///
/// Target names are stored trimmed: `target: kitty :done` names `kitty`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HookEvent {
    /// `targets:done`: the sync generated at least one artifact.
    TargetsDone,
    /// `targets:changed`: at least one artifact's content changed and was
    /// written.
    TargetsChanged,
    /// `target:<name>:done`: the named target generated artifacts.
    TargetDone(String),
    /// `target:<name>:changed`: one of the named target's artifacts changed
    /// and was written.
    TargetChanged(String),
}

impl HookEvent {
    /// Target name for per-target events, `None` for `targets:*` events.
    pub fn target(&self) -> Option<&str> {
        match self {
            Self::TargetsDone | Self::TargetsChanged => None,
            Self::TargetDone(name) | Self::TargetChanged(name) => Some(name),
        }
    }
}

impl fmt::Display for HookEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetsDone => f.write_str("targets:done"),
            Self::TargetsChanged => f.write_str("targets:changed"),
            Self::TargetDone(name) => write!(f, "target:{name}:done"),
            Self::TargetChanged(name) => write!(f, "target:{name}:changed"),
        }
    }
}

/// Error returned when a hook event string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "invalid hook event '{event}': {reason}; expected 'targets:done', 'targets:changed', \
     'target:<name>:done' or 'target:<name>:changed'"
)]
pub struct ParseHookEventError {
    pub event: String,
    pub reason: &'static str,
}

impl FromStr for HookEvent {
    type Err = ParseHookEventError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let error = |reason| ParseHookEventError {
            event: value.to_owned(),
            reason,
        };

        if let Some(kind) = value.strip_prefix("targets:") {
            return match kind {
                "done" => Ok(Self::TargetsDone),
                "changed" => Ok(Self::TargetsChanged),
                _ => Err(error("unknown event kind")),
            };
        }

        let Some(rest) = value.strip_prefix("target:") else {
            return Err(error("unknown event"));
        };
        let Some((name, kind)) = rest.rsplit_once(':') else {
            return Err(error("missing target name or event kind"));
        };
        let name = name.trim();

        if name.is_empty() {
            return Err(error("target name must not be empty"));
        }

        if name.contains(':') {
            return Err(error("target names cannot contain ':'"));
        }

        match kind {
            "done" => Ok(Self::TargetDone(name.to_owned())),
            "changed" => Ok(Self::TargetChanged(name.to_owned())),
            _ => Err(error("unknown event kind")),
        }
    }
}

/// A parsed hook filter.
///
/// Filters of the same kind are OR'd together; filters of different kinds are
/// AND'd. Only `config:<profile>` exists today. The profile name is stored
/// trimmed: `config: default ` names `default`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HookFilter {
    /// `config:<profile-name>`: run only for the named `[[configs]]` profile.
    Config(String),
}

impl fmt::Display for HookFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(name) => write!(f, "config:{name}"),
        }
    }
}

/// Error returned when a hook filter string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid hook filter '{filter}': {reason}; expected 'config:<profile-name>'")]
pub struct ParseHookFilterError {
    pub filter: String,
    pub reason: &'static str,
}

impl FromStr for HookFilter {
    type Err = ParseHookFilterError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let error = |reason| ParseHookFilterError {
            filter: value.to_owned(),
            reason,
        };

        let Some((kind, argument)) = value.split_once(':') else {
            return Err(error("missing filter kind"));
        };

        let argument = argument.trim();
        match kind {
            "config" if argument.is_empty() => Err(error("profile name must not be empty")),
            "config" => Ok(Self::Config(argument.to_owned())),
            _ => Err(error("unknown filter kind")),
        }
    }
}

/// Error returned by [`ConfigHook::validate`] and
/// [`ChromasyncConfig::validate_hooks`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HookConfigError {
    #[error("a [[hooks]] entry has an empty 'name'; every hook needs a unique name")]
    EmptyName,
    #[error("hook '{name}' has an empty 'command'")]
    EmptyCommand { name: String },
    #[error("hook '{name}' has no events; 'on' must name at least one event")]
    NoEvents { name: String },
    #[error("hook '{name}': {error}")]
    InvalidEvent {
        name: String,
        error: ParseHookEventError,
    },
    #[error("hook '{name}': {error}")]
    InvalidFilter {
        name: String,
        error: ParseHookFilterError,
    },
    #[error(
        "hook name '{name}' is used by more than one [[hooks]] entry; hook names must be unique"
    )]
    DuplicateName { name: String },
}

/// A [`ConfigHook`] whose events and filters parsed successfully.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedHook<'a> {
    /// Hook name with surrounding whitespace trimmed.
    pub name: &'a str,
    pub command: &'a str,
    /// Parsed events in config order, without duplicates.
    pub events: Vec<HookEvent>,
    /// Parsed filters in config order.
    pub filters: Vec<HookFilter>,
}

impl ConfigHook {
    /// Parse this hook's events and filters, rejecting empty names, commands
    /// and event lists. The hook name is trimmed.
    pub fn validate(&self) -> Result<ValidatedHook<'_>, HookConfigError> {
        let hook_name = self.name.trim();
        if hook_name.is_empty() {
            return Err(HookConfigError::EmptyName);
        }

        let name = || hook_name.to_owned();

        if self.command.trim().is_empty() {
            return Err(HookConfigError::EmptyCommand { name: name() });
        }

        let mut events = Vec::new();
        for event in self.on.iter() {
            let event =
                event
                    .parse::<HookEvent>()
                    .map_err(|error| HookConfigError::InvalidEvent {
                        name: name(),
                        error,
                    })?;
            if !events.contains(&event) {
                events.push(event);
            }
        }

        if events.is_empty() {
            return Err(HookConfigError::NoEvents { name: name() });
        }

        let filters = self
            .filters
            .iter()
            .map(|filter| {
                filter
                    .parse::<HookFilter>()
                    .map_err(|error| HookConfigError::InvalidFilter {
                        name: name(),
                        error,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ValidatedHook {
            name: hook_name,
            command: &self.command,
            events,
            filters,
        })
    }
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

    /// Return the sync profile with `name`, if present.
    pub fn sync_profile(&self, name: &str) -> Option<&SyncProfile> {
        self.configs.iter().find(|entry| entry.name == name)
    }

    /// Validate every `[[hooks]]` entry and reject duplicate hook names
    /// (compared after trimming, so `reload` and `reload ` collide).
    ///
    /// Returns the parsed hooks in config order. This only checks the hooks'
    /// own syntax; whether filters and events refer to known profiles and
    /// targets is left to the caller, which knows what a run produced.
    pub fn validate_hooks(&self) -> Result<Vec<ValidatedHook<'_>>, HookConfigError> {
        let mut names = BTreeSet::new();

        self.hooks
            .iter()
            .map(|hook| {
                let validated = hook.validate()?;
                if !names.insert(validated.name) {
                    return Err(HookConfigError::DuplicateName {
                        name: validated.name.to_owned(),
                    });
                }
                Ok(validated)
            })
            .collect()
    }
}

/// Insert or replace the `[[targets]]` entry for `entry.name` in the TOML
/// `content` of a config file, preserving comments, formatting, and every
/// other entry.
///
/// An existing entry with the same name is updated in place; otherwise a new
/// `[[targets]]` table is appended. Both `[[targets]]` tables and an inline
/// `targets = [...]` array are supported.
fn upsert_target_entry(content: &str, entry: &ConfigTarget) -> Result<String, CoreError> {
    use toml_edit::{ArrayOfTables, DocumentMut, InlineTable, Item, Table, TableLike, Value};

    // Replace a value while keeping its key, position, and surrounding
    // comments.
    fn set(table: &mut dyn TableLike, key: &str, new_value: Value) {
        match table.get_mut(key) {
            Some(Item::Value(existing)) => {
                let decor = existing.decor().clone();
                *existing = new_value;
                *existing.decor_mut() = decor;
            }
            Some(item) => *item = Item::Value(new_value),
            None => {
                table.insert(key, Item::Value(new_value));
            }
        }
    }

    fn apply(table: &mut dyn TableLike, entry: &ConfigTarget, output_dir: &str) {
        set(table, "name", entry.name.as_str().into());
        set(table, "output_dir", output_dir.into());
        match &entry.source {
            Some(source) => set(table, "source", source.as_str().into()),
            None => {
                table.remove("source");
            }
        }
        set(table, "overwrite", entry.overwrite.into());
    }

    fn has_name(table: &dyn TableLike, name: &str) -> bool {
        table.get("name").and_then(Item::as_str) == Some(name)
    }

    let output_dir = entry
        .output_dir
        .to_str()
        .ok_or_else(|| CoreError::ConfigSerialize {
            error: format!(
                "output directory '{}' is not valid UTF-8",
                entry.output_dir.display()
            ),
        })?;
    let mut document =
        content
            .parse::<DocumentMut>()
            .map_err(|error| CoreError::ConfigSerialize {
                error: error.to_string(),
            })?;

    let trailing = document
        .trailing()
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let mut move_trailing = false;
    let targets = document
        .entry("targets")
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));

    match targets {
        Item::ArrayOfTables(tables) => {
            let existing = tables
                .iter_mut()
                .find(|table| has_name(*table, &entry.name));
            if let Some(table) = existing {
                apply(table, entry, output_dir);
            } else {
                let mut table = Table::new();
                apply(&mut table, entry, output_dir);

                let mut prefix = String::new();
                // A brand-new `[[targets]]` array renders at the end of the
                // file, after any trailing comments; keep those comments
                // where they were by moving them in front of the new table.
                if tables.is_empty() && !trailing.is_empty() {
                    prefix.push_str(&trailing);
                    prefix.push('\n');
                    move_trailing = true;
                }
                if !content.trim().is_empty() {
                    prefix.push('\n');
                }
                table.decor_mut().set_prefix(prefix);
                tables.push(table);
            }
        }
        Item::Value(Value::Array(array)) => {
            let existing = array.iter_mut().find_map(|item| {
                item.as_inline_table_mut()
                    .filter(|table| has_name(*table, &entry.name))
            });
            match existing {
                Some(table) => apply(table, entry, output_dir),
                None => {
                    let mut table = InlineTable::new();
                    apply(&mut table, entry, output_dir);
                    array.push(table);
                }
            }
        }
        _ => {
            return Err(CoreError::ConfigSerialize {
                error: "'targets' must be an array of tables".to_owned(),
            });
        }
    }

    if move_trailing {
        document.set_trailing("");
    }

    Ok(document.to_string())
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

    crate::load_output_registry()?.validate_path_target(target_path)?;

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

    let config_file = config_file_path().ok_or(CoreError::UserConfigDirUnavailable)?;
    record_installed_target(
        &config_file,
        &ConfigTarget {
            name: name.clone(),
            output_dir,
            source: Some(format!("targets/{name}.toml")),
            overwrite,
        },
    )?;

    Ok(InstallSummary {
        target_name: name,
        target_file: dest,
        config_file,
    })
}

/// Upsert `entry` into the config file at `path` without disturbing the rest
/// of the file. A missing file is created with a short header comment.
fn record_installed_target(path: &Path, entry: &ConfigTarget) -> Result<(), CoreError> {
    let existing = match fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(CoreError::ConfigRead {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let body = match &existing {
        Some(content) => {
            // Refuse to edit a config that would not load afterwards anyway.
            toml::from_str::<ChromasyncConfig>(content).map_err(|error| {
                CoreError::ConfigParse {
                    path: path.to_path_buf(),
                    error,
                }
            })?;
            upsert_target_entry(content, entry)?
        }
        None => format!("{CONFIG_HEADER}\n{}", upsert_target_entry("", entry)?),
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| CoreError::ConfigWrite {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, body).map_err(|source| CoreError::ConfigWrite {
        path: path.to_path_buf(),
        source,
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

fn default_output_dir() -> PathBuf {
    PathBuf::from("chromasync")
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
            configs: Vec::new(),
            targets: vec![ConfigTarget {
                name: "gtk".to_owned(),
                output_dir: PathBuf::from("~/.config/gtk-4.0"),
                source: Some("targets/gtk.toml".to_owned()),
                overwrite: true,
            }],
            hooks: Vec::new(),
        };
        let (dir, force) = config.resolve("gtk", Path::new("fallback"), false);

        assert_eq!(dir, expand_tilde(&PathBuf::from("~/.config/gtk-4.0")));
        assert!(force);
    }

    #[test]
    fn resolve_global_force_overrides_installed_overwrite_false() {
        let config = ChromasyncConfig {
            configs: Vec::new(),
            targets: vec![ConfigTarget {
                name: "gtk".to_owned(),
                output_dir: PathBuf::from("/tmp/gtk"),
                source: Some("targets/gtk.toml".to_owned()),
                overwrite: false,
            }],
            hooks: Vec::new(),
        };
        let (_, force) = config.resolve("gtk", Path::new("fallback"), true);

        assert!(force);
    }

    fn gtk_entry(output_dir: &str, overwrite: bool) -> ConfigTarget {
        ConfigTarget {
            name: "gtk".to_owned(),
            output_dir: PathBuf::from(output_dir),
            source: Some("targets/gtk.toml".to_owned()),
            overwrite,
        }
    }

    #[test]
    fn upsert_target_entry_replaces_existing_entry_in_place() {
        let content = r#"# my config
[[targets]]
name = "kitty" # keep me
output_dir = "~/.config/kitty"

[[targets]]
# gtk comment
name = "gtk"
output_dir = "/old"
overwrite = false

[[hooks]]
name = "reload"
on = "targets:done"
command = "true"
"#;

        let updated =
            upsert_target_entry(content, &gtk_entry("/new", true)).expect("upsert should succeed");
        let config = toml::from_str::<ChromasyncConfig>(&updated).expect("result should parse");

        assert_eq!(config.targets.len(), 2);
        assert_eq!(config.targets[1].output_dir, PathBuf::from("/new"));
        assert!(config.targets[1].overwrite);
        assert_eq!(config.hooks.len(), 1);
        assert!(updated.starts_with("# my config\n"));
        assert!(updated.contains(r#"name = "kitty" # keep me"#));
        assert!(
            updated.contains("# gtk comment\nname = \"gtk\""),
            "got:\n{updated}"
        );
        assert!(
            updated.find("name = \"gtk\"") < updated.find("[[hooks]]"),
            "replaced entry should stay where it was:\n{updated}"
        );
    }

    #[test]
    fn upsert_target_entry_appends_without_expanding_profile_defaults() {
        let content = r##"[[configs]]
name = "default"
seed = "#4ecdc4"
targets = ["kitty"] # just kitty
"##;

        let updated =
            upsert_target_entry(content, &gtk_entry("/gtk", false)).expect("upsert should succeed");

        assert!(updated.starts_with(content), "got:\n{updated}");
        assert!(!updated.contains("mode"));
        assert!(!updated.contains("output_dir = \"chromasync\""));
        let config = toml::from_str::<ChromasyncConfig>(&updated).expect("result should parse");
        assert_eq!(config.targets, [gtk_entry("/gtk", false)]);
    }

    #[test]
    fn upsert_target_entry_updates_inline_target_arrays() {
        let content = r#"targets = [{ name = "gtk", output_dir = "/old" }]
"#;

        let updated =
            upsert_target_entry(content, &gtk_entry("/new", true)).expect("upsert should succeed");
        let config = toml::from_str::<ChromasyncConfig>(&updated).expect("result should parse");

        assert_eq!(config.targets, [gtk_entry("/new", true)]);
    }

    #[test]
    fn record_installed_target_adds_header_only_to_new_files() {
        let temp = tempfile::tempdir().expect("temp dir should be created");
        let path = temp.path().join("chromasync").join("config.toml");

        record_installed_target(&path, &gtk_entry("/gtk", false)).expect("create should work");
        let created = fs::read_to_string(&path).expect("config should exist");
        assert!(created.starts_with(CONFIG_HEADER));

        fs::write(&path, "# hand-written\n").expect("config should be rewritten");
        record_installed_target(&path, &gtk_entry("/gtk", false)).expect("update should work");
        let updated = fs::read_to_string(&path).expect("config should exist");
        assert!(updated.starts_with("# hand-written\n"), "got:\n{updated}");
        assert!(!updated.contains("Managed by"));

        fs::write(&path, "[[targets]]\nname = 1\n").expect("config should be rewritten");
        assert!(matches!(
            record_installed_target(&path, &gtk_entry("/gtk", false)),
            Err(CoreError::ConfigParse { .. })
        ));
    }

    #[test]
    fn hook_events_parse_and_display_round_trip() {
        for event in [
            "targets:done",
            "targets:changed",
            "target:kitty:done",
            "target:hyprland-lua:changed",
        ] {
            let parsed = event.parse::<HookEvent>().expect("event should parse");
            assert_eq!(parsed.to_string(), event);
        }

        assert_eq!(
            "target:kitty:changed".parse::<HookEvent>(),
            Ok(HookEvent::TargetChanged("kitty".to_owned()))
        );
        assert_eq!(
            "target:kitty:done"
                .parse::<HookEvent>()
                .expect("event should parse")
                .target(),
            Some("kitty")
        );
    }

    #[test]
    fn hook_events_reject_malformed_values() {
        for event in [
            "target:done",
            "targets:finished",
            "target::done",
            "target:kitty:finished",
            "target:a:b:done",
            "kitty:done",
            "",
        ] {
            let error = event
                .parse::<HookEvent>()
                .expect_err("malformed event should be rejected");
            assert!(
                error.to_string().contains(&format!("'{event}'")),
                "error should quote the event: {error}"
            );
        }
    }

    #[test]
    fn hook_filters_parse_config_and_reject_unknown_kinds() {
        assert_eq!(
            "config:default".parse::<HookFilter>(),
            Ok(HookFilter::Config("default".to_owned()))
        );

        for filter in ["profile:default", "config:", "default"] {
            assert!(
                filter.parse::<HookFilter>().is_err(),
                "filter '{filter}' should be rejected"
            );
        }
    }

    fn hook(name: &str, on: HookEvents, command: &str, filters: &[&str]) -> ConfigHook {
        ConfigHook {
            name: name.to_owned(),
            on,
            command: command.to_owned(),
            filters: filters.iter().map(|filter| (*filter).to_owned()).collect(),
        }
    }

    #[test]
    fn hook_validation_reports_each_misconfiguration() {
        let one = |event: &str| HookEvents::One(event.to_owned());

        assert_eq!(
            hook(" ", one("targets:done"), "true", &[]).validate(),
            Err(HookConfigError::EmptyName)
        );
        assert!(matches!(
            hook("a", one("targets:done"), "  ", &[]).validate(),
            Err(HookConfigError::EmptyCommand { .. })
        ));
        assert!(matches!(
            hook("a", HookEvents::Many(Vec::new()), "true", &[]).validate(),
            Err(HookConfigError::NoEvents { .. })
        ));
        assert!(matches!(
            hook("a", one("target:done"), "true", &[]).validate(),
            Err(HookConfigError::InvalidEvent { .. })
        ));
        assert!(matches!(
            hook("a", one("targets:done"), "true", &["profile:default"]).validate(),
            Err(HookConfigError::InvalidFilter { .. })
        ));

        let config = ChromasyncConfig {
            hooks: vec![
                hook("a", one("targets:done"), "true", &[]),
                hook("a", one("targets:changed"), "true", &[]),
            ],
            ..ChromasyncConfig::default()
        };
        assert_eq!(
            config.validate_hooks(),
            Err(HookConfigError::DuplicateName {
                name: "a".to_owned()
            })
        );
    }

    #[test]
    fn hook_validation_parses_events_and_filters_in_order() {
        let config_hook = hook(
            "reload",
            HookEvents::Many(vec![
                "target:kitty:changed".to_owned(),
                "targets:done".to_owned(),
                "targets:done".to_owned(),
            ]),
            "kitty @ load-config",
            &["config:a", "config:b"],
        );
        let validated = config_hook.validate().expect("hook should validate");

        assert_eq!(validated.name, "reload");
        assert_eq!(
            validated.events,
            [
                HookEvent::TargetChanged("kitty".to_owned()),
                HookEvent::TargetsDone
            ]
        );
        assert_eq!(
            validated.filters,
            [
                HookFilter::Config("a".to_owned()),
                HookFilter::Config("b".to_owned())
            ]
        );
    }

    #[test]
    fn hook_names_targets_and_profiles_are_trimmed() {
        assert_eq!(
            "target: kitty :changed".parse::<HookEvent>(),
            Ok(HookEvent::TargetChanged("kitty".to_owned()))
        );
        assert_eq!(
            "target:kitty\t:done"
                .parse::<HookEvent>()
                .expect("event should parse")
                .to_string(),
            "target:kitty:done"
        );
        assert_eq!(
            "config: default ".parse::<HookFilter>(),
            Ok(HookFilter::Config("default".to_owned()))
        );
        for (event, reason) in [
            ("target: :done", "target name must not be empty"),
            ("target: a:b :done", "target names cannot contain ':'"),
        ] {
            let error = event
                .parse::<HookEvent>()
                .expect_err("event should be rejected");
            assert_eq!(error.reason, reason, "{event}");
        }
        assert!("config: \t".parse::<HookFilter>().is_err());

        let validated = hook(
            " reload ",
            HookEvents::One("target: kitty :done".to_owned()),
            "true",
            &["config:default "],
        );
        let validated = validated.validate().expect("hook should validate");
        assert_eq!(validated.name, "reload");
        assert_eq!(
            validated.events,
            [HookEvent::TargetDone("kitty".to_owned())]
        );
        assert_eq!(
            validated.filters,
            [HookFilter::Config("default".to_owned())]
        );

        let config = ChromasyncConfig {
            hooks: vec![
                hook(
                    "reload",
                    HookEvents::One("targets:done".to_owned()),
                    "true",
                    &[],
                ),
                hook(
                    "reload ",
                    HookEvents::One("targets:done".to_owned()),
                    "true",
                    &[],
                ),
            ],
            ..ChromasyncConfig::default()
        };
        assert_eq!(
            config.validate_hooks(),
            Err(HookConfigError::DuplicateName {
                name: "reload".to_owned()
            })
        );
    }

    #[test]
    fn config_toml_accepts_sync_profiles() {
        let config = toml::from_str::<ChromasyncConfig>(
            r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
template = "materialish"
mode = "auto"
contrast = "apca-experimental"
chroma = "industrial"
targets = ["ghostty", "kitty"]
output_dir = "fallback-output"
force = true

[[targets]]
name = "kitty"
output_dir = "~/.config/kitty"
overwrite = true
"##,
        )
        .expect("sync profile config should parse");

        let profile = config
            .sync_profile("default")
            .expect("default profile should be present");
        assert_eq!(profile.seed.as_deref(), Some("#4ecdc4"));
        assert_eq!(profile.image_fetch_command, None);
        assert_eq!(profile.template.as_deref(), Some("materialish"));
        assert_eq!(profile.mode, SyncMode::Auto);
        assert_eq!(profile.contrast, ContrastStrategy::ApcaExperimental);
        assert_eq!(profile.chroma, ChromaStrategy::Industrial);
        assert_eq!(profile.targets, ["ghostty", "kitty"]);
        assert_eq!(profile.output_dir, PathBuf::from("fallback-output"));
        assert!(profile.force);
        assert_eq!(config.targets[0].source, None);
    }

    #[test]
    fn sync_profile_defaults_match_generate_defaults() {
        let config = toml::from_str::<ChromasyncConfig>(
            r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
"##,
        )
        .expect("minimal sync profile should parse");

        let profile = config
            .sync_profile("default")
            .expect("default profile should be present");
        assert_eq!(profile.mode, SyncMode::Dark);
        assert_eq!(profile.contrast, ContrastStrategy::RelativeLuminance);
        assert_eq!(profile.chroma, ChromaStrategy::Normal);
        assert_eq!(profile.targets, Vec::<String>::new());
        assert_eq!(profile.output_dir, PathBuf::from("chromasync"));
        assert!(!profile.force);
    }

    #[test]
    fn config_toml_accepts_image_fetch_command_source() {
        let config = toml::from_str::<ChromasyncConfig>(
            r#"
[[configs]]
name = "default"
image_fetch_command = "qs -c noctalia-shell ipc call wallpaper get"
targets = ["kitty"]
"#,
        )
        .expect("image fetch command profile should parse");

        let profile = config
            .sync_profile("default")
            .expect("default profile should be present");
        assert_eq!(
            profile.image_fetch_command.as_deref(),
            Some("qs -c noctalia-shell ipc call wallpaper get")
        );
        assert_eq!(profile.seed, None);
        assert_eq!(profile.image, None);
    }

    #[test]
    fn config_toml_accepts_hooks_with_single_or_multiple_events() {
        let config = toml::from_str::<ChromasyncConfig>(
            r##"
[[hooks]]
name = "all-targets"
on = "targets:done"
command = "printf all"

[[hooks]]
name = "hyprland-lua"
filters = ["config:default"]
on = ["target:hyprland-lua:done"]
command = "hyprctl reload"
"##,
        )
        .expect("hooks should deserialize");

        assert_eq!(config.hooks.len(), 2);
        assert_eq!(
            config.hooks[0].on.iter().collect::<Vec<_>>(),
            vec!["targets:done"]
        );
        assert_eq!(
            config.hooks[1].on.iter().collect::<Vec<_>>(),
            vec!["target:hyprland-lua:done"]
        );
        assert_eq!(config.hooks[1].filters, vec!["config:default"]);
    }

    #[test]
    fn config_toml_rejects_unknown_root_fields() {
        let error = toml::from_str::<ChromasyncConfig>(
            r#"
unknown = true
"#,
        )
        .expect_err("unknown root config fields should be rejected");

        assert!(
            error.to_string().contains("unknown field"),
            "expected unknown-field parse error, got: {error}"
        );
    }

    #[test]
    fn config_toml_rejects_unknown_target_fields() {
        let error = toml::from_str::<ChromasyncConfig>(
            r#"
[[targets]]
name = "gtk"
output_dir = "/tmp/gtk"
source = "targets/gtk.toml"
extra = true
"#,
        )
        .expect_err("unknown target config fields should be rejected");

        assert!(
            error.to_string().contains("unknown field"),
            "expected unknown-field parse error, got: {error}"
        );
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
