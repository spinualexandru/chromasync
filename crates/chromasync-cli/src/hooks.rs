//! `[[hooks]]` matching and execution for `chromasync sync`.
//!
//! Parsing and validation of the hook model live in `chromasync_core`
//! ([`chromasync_core::ConfigHook::validate`]); this module decides which
//! validated hooks match a finished sync and runs them.

use std::{
    collections::BTreeSet,
    ffi::OsString,
    fmt,
    fs::File,
    io::{self, Write},
    path::Path,
    process::{ExitStatus, Stdio},
};

use chromasync_core::{
    ChromasyncConfig, HookEvent, HookFilter, RoutedArtifact, ValidatedHook, WrittenArtifact,
};
use chromasync_types::ThemeMode;

use crate::shell_command;

/// Exit code used when artifacts were written but at least one hook failed.
pub const HOOK_FAILURE_EXIT_CODE: u8 = 3;

/// What a sync run produced, as seen by hooks.
#[derive(Debug)]
pub(crate) struct SyncEvents<'a> {
    events: BTreeSet<HookEvent>,
    /// Targets that produced artifacts, in first-appearance order.
    targets: Vec<&'a str>,
    artifacts: Vec<&'a Path>,
    changed_artifacts: Vec<&'a Path>,
}

impl<'a> SyncEvents<'a> {
    /// Build the event set from the generated artifacts and their write
    /// outcomes, which must be in the same order.
    ///
    /// Every generated artifact counts towards the `done` events and
    /// `CHROMASYNC_ARTIFACTS`, whether or not its file was rewritten; only
    /// artifacts whose content changed count towards the `changed` events.
    pub(crate) fn new(routed: &'a [RoutedArtifact], written: &'a [WrittenArtifact]) -> Self {
        debug_assert_eq!(routed.len(), written.len());

        let mut events = BTreeSet::new();
        let mut targets = Vec::new();
        let mut artifacts = Vec::with_capacity(written.len());
        let mut changed_artifacts = Vec::new();

        for (routed, written) in routed.iter().zip(written) {
            let target = routed.artifact.target.as_str();
            if !targets.contains(&target) {
                targets.push(target);
                events.insert(HookEvent::TargetDone(target.to_owned()));
            }

            artifacts.push(written.path.as_path());
            if written.changed {
                changed_artifacts.push(written.path.as_path());
                events.insert(HookEvent::TargetChanged(target.to_owned()));
            }
        }

        if !artifacts.is_empty() {
            events.insert(HookEvent::TargetsDone);
        }
        if !changed_artifacts.is_empty() {
            events.insert(HookEvent::TargetsChanged);
        }

        Self {
            events,
            targets,
            artifacts,
            changed_artifacts,
        }
    }

    fn contains(&self, event: &HookEvent) -> bool {
        self.events.contains(event)
    }
}

/// Run-level details exported to every hook process.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HookContext<'a> {
    pub profile: &'a str,
    pub mode: ThemeMode,
    pub config_dir: &'a Path,
}

/// One or more hooks failed after the sync wrote its artifacts.
///
/// Mapped to [`HOOK_FAILURE_EXIT_CODE`] by [`crate::exit_code`].
#[derive(Debug)]
pub struct HookFailure {
    ran: usize,
    failures: Vec<FailedHook>,
}

#[derive(Debug)]
struct FailedHook {
    name: String,
    reason: FailureReason,
}

#[derive(Debug)]
enum FailureReason {
    Spawn(io::Error),
    Exited(ExitStatus),
}

impl fmt::Display for FailedHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.reason {
            FailureReason::Spawn(error) => write!(f, "'{}' failed to start: {error}", self.name),
            FailureReason::Exited(status) => write!(f, "'{}' exited with {status}", self.name),
        }
    }
}

impl fmt::Display for HookFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let ([failure], 1) = (self.failures.as_slice(), self.ran) {
            return write!(f, "hook {failure}");
        }

        write!(f, "{} of {} hooks failed: ", self.failures.len(), self.ran)?;
        for (index, failure) in self.failures.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{failure}")?;
        }
        Ok(())
    }
}

impl std::error::Error for HookFailure {}

/// Run every hook that matches this sync, in config order.
///
/// A failing hook does not stop later hooks; all failures are reported
/// together afterwards.
pub(crate) fn run_matching_hooks(
    hooks: &[ValidatedHook<'_>],
    context: HookContext<'_>,
    events: &SyncEvents<'_>,
) -> Result<(), HookFailure> {
    let mut ran = 0;
    let mut failures = Vec::new();

    for hook in hooks
        .iter()
        .filter(|hook| hook_matches(hook, context.profile, events))
    {
        ran += 1;
        if let Err(reason) = run_hook(hook, context, events) {
            failures.push(FailedHook {
                name: hook.name.to_owned(),
                reason,
            });
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(HookFailure { ran, failures })
    }
}

fn hook_matches(hook: &ValidatedHook<'_>, profile: &str, events: &SyncEvents<'_>) -> bool {
    hook.events.iter().any(|event| events.contains(event)) && filters_match(&hook.filters, profile)
}

/// Filters of the same kind are OR'd; different kinds are AND'd. With
/// `config:` as the only kind, the hook matches when it has no filters or any
/// `config:` filter names `profile`.
fn filters_match(filters: &[HookFilter], profile: &str) -> bool {
    let mut profiles = filters
        .iter()
        .map(|filter| match filter {
            HookFilter::Config(name) => name.as_str(),
        })
        .peekable();

    profiles.peek().is_none() || profiles.any(|name| name == profile)
}

fn run_hook(
    hook: &ValidatedHook<'_>,
    context: HookContext<'_>,
    events: &SyncEvents<'_>,
) -> Result<(), FailureReason> {
    // The hook writes stdout and stderr into its own anonymous temp file,
    // which is copied to chromasync's stderr once the hook exits; stdout keeps
    // listing only artifact paths. The hook's stdio is never one of
    // chromasync's own descriptors, so a process it backgrounds (`daemon &`)
    // cannot hold a caller's pipe open and make the caller wait for that
    // process to exit. Only the hook's exit status is waited on.
    let output = tempfile::tempfile().map_err(|error| {
        FailureReason::Spawn(io::Error::new(
            error.kind(),
            format!("could not create a temporary file for its output: {error}"),
        ))
    })?;
    let stdout = output.try_clone().map_err(FailureReason::Spawn)?;
    let stderr = output.try_clone().map_err(FailureReason::Spawn)?;

    let status = shell_command(hook.command)
        .current_dir(context.config_dir)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .env("CHROMASYNC_PROFILE", context.profile)
        .env("CHROMASYNC_MODE", context.mode.as_str())
        .env("CHROMASYNC_CONFIG_DIR", context.config_dir)
        .env("CHROMASYNC_TARGETS", events.targets.join("\n"))
        .env("CHROMASYNC_ARTIFACTS", join_paths(&events.artifacts))
        .env(
            "CHROMASYNC_CHANGED_ARTIFACTS",
            join_paths(&events.changed_artifacts),
        )
        .status()
        .map_err(FailureReason::Spawn);

    // Best effort: losing hook output must not turn a successful hook into a
    // failure.
    let _ = copy_output(&output, &mut io::stderr().lock());
    let status = status?;

    if status.success() {
        Ok(())
    } else {
        Err(FailureReason::Exited(status))
    }
}

/// Copy what the hook wrote to `file` before it exited.
///
/// Stops at the length the file had when the hook exited; anything a
/// background process writes later is discarded. On Unix the reads are
/// positional, so they never move the file offset that such a process shares
/// and cannot make it overwrite the output being copied.
fn copy_output(file: &File, sink: &mut impl Write) -> io::Result<()> {
    let len = file.metadata()?.len();
    let mut buffer = [0; 8192];
    let mut offset = 0;

    while offset < len {
        let want =
            usize::try_from(len - offset).map_or(buffer.len(), |left| left.min(buffer.len()));
        let read = read_at(file, &mut buffer[..want], offset)?;
        if read == 0 {
            break;
        }
        sink.write_all(&buffer[..read])?;
        offset += read as u64;
    }

    sink.flush()
}

#[cfg(unix)]
fn read_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buffer, offset)
}

#[cfg(windows)]
fn read_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buffer, offset)
}

fn join_paths(paths: &[&Path]) -> OsString {
    let mut joined = OsString::new();
    for (index, path) in paths.iter().enumerate() {
        if index > 0 {
            joined.push("\n");
        }
        joined.push(path);
    }
    joined
}

/// Describe hook references that can never match.
///
/// Reports `config:` filters naming no `[[configs]]` profile, and per-target
/// events naming a target that `is_known_target` rejects. Target events are
/// only checked for hooks whose filters select `profile`, so a hook meant for
/// another profile's path target does not warn on every run.
pub(crate) fn unknown_reference_warnings(
    hooks: &[ValidatedHook<'_>],
    config: &ChromasyncConfig,
    profile: &str,
    is_known_target: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut warnings = Vec::new();

    for hook in hooks {
        for filter in &hook.filters {
            match filter {
                HookFilter::Config(name) if config.sync_profile(name).is_none() => {
                    warnings.push(format!(
                        "hook '{}' filter '{filter}' names no [[configs]] profile",
                        hook.name
                    ));
                }
                HookFilter::Config(_) => {}
            }
        }

        if !filters_match(&hook.filters, profile) {
            continue;
        }

        for event in &hook.events {
            if let Some(target) = event.target()
                && !is_known_target(target)
            {
                warnings.push(format!(
                    "hook '{}' event '{event}' names target '{target}', which is not a known \
                     target and was not generated by this sync",
                    hook.name
                ));
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chromasync_core::{ConfigHook, HookEvents, SyncProfile};
    use chromasync_types::GeneratedArtifact;

    use super::*;

    fn routed(target: &str, file_name: &str) -> RoutedArtifact {
        RoutedArtifact {
            artifact: GeneratedArtifact {
                target: target.to_owned(),
                file_name: file_name.to_owned(),
                content: String::new(),
            },
            output_dir: PathBuf::from("/out"),
            force: true,
        }
    }

    fn written(file_name: &str, changed: bool) -> WrittenArtifact {
        WrittenArtifact {
            path: PathBuf::from("/out").join(file_name),
            changed,
        }
    }

    fn config_hook(name: &str, on: &[&str], filters: &[&str]) -> ConfigHook {
        ConfigHook {
            name: name.to_owned(),
            on: HookEvents::Many(on.iter().map(|event| (*event).to_owned()).collect()),
            command: "true".to_owned(),
            filters: filters.iter().map(|filter| (*filter).to_owned()).collect(),
        }
    }

    fn matches(on: &[&str], filters: &[&str], profile: &str, events: &SyncEvents<'_>) -> bool {
        let hook = config_hook("probe", on, filters);
        hook_matches(
            &hook.validate().expect("hook should validate"),
            profile,
            events,
        )
    }

    #[test]
    fn events_distinguish_done_from_changed() {
        let routed = [
            routed("kitty", "kitty.conf"),
            routed("gtk4", "gtk.css"),
            routed("gtk4", "gtk-dark.css"),
        ];
        let written = [
            written("kitty.conf", false),
            written("gtk.css", false),
            written("gtk-dark.css", true),
        ];
        let events = SyncEvents::new(&routed, &written);

        assert_eq!(events.targets, ["kitty", "gtk4"]);
        assert_eq!(events.artifacts.len(), 3);
        assert_eq!(events.changed_artifacts, [Path::new("/out/gtk-dark.css")]);

        assert!(matches(&["targets:done"], &[], "default", &events));
        assert!(matches(&["targets:changed"], &[], "default", &events));
        assert!(matches(&["target:kitty:done"], &[], "default", &events));
        assert!(!matches(&["target:kitty:changed"], &[], "default", &events));
        assert!(matches(&["target:gtk4:changed"], &[], "default", &events));
        assert!(!matches(&["target:ghostty:done"], &[], "default", &events));
    }

    #[test]
    fn unchanged_runs_only_emit_done_events() {
        let routed = [routed("kitty", "kitty.conf")];
        let written = [written("kitty.conf", false)];
        let events = SyncEvents::new(&routed, &written);

        assert!(matches(&["targets:done"], &[], "default", &events));
        assert!(!matches(&["targets:changed"], &[], "default", &events));
        assert!(
            matches(
                &["targets:changed", "target:kitty:done"],
                &[],
                "default",
                &events
            ),
            "any matching event should be enough"
        );
    }

    #[test]
    fn empty_runs_emit_no_events() {
        let events = SyncEvents::new(&[], &[]);

        assert!(!matches(&["targets:done"], &[], "default", &events));
        assert!(!matches(&["targets:changed"], &[], "default", &events));
    }

    #[test]
    fn config_filters_are_ored_together() {
        let routed = [routed("kitty", "kitty.conf")];
        let written = [written("kitty.conf", true)];
        let events = SyncEvents::new(&routed, &written);
        let filters = ["config:a", "config:b"];

        assert!(matches(&["targets:done"], &filters, "a", &events));
        assert!(matches(&["targets:done"], &filters, "b", &events));
        assert!(!matches(&["targets:done"], &filters, "c", &events));
        assert!(matches(&["targets:done"], &[], "c", &events));
    }

    #[test]
    fn failure_messages_name_every_failed_hook() {
        let single = HookFailure {
            ran: 1,
            failures: vec![FailedHook {
                name: "reload".to_owned(),
                reason: FailureReason::Spawn(io::Error::other("boom")),
            }],
        };
        assert_eq!(single.to_string(), "hook 'reload' failed to start: boom");

        let several = HookFailure {
            ran: 3,
            failures: vec![
                FailedHook {
                    name: "a".to_owned(),
                    reason: FailureReason::Spawn(io::Error::other("boom")),
                },
                FailedHook {
                    name: "b".to_owned(),
                    reason: FailureReason::Spawn(io::Error::other("bang")),
                },
            ],
        };
        assert_eq!(
            several.to_string(),
            "2 of 3 hooks failed: 'a' failed to start: boom, 'b' failed to start: bang"
        );
    }

    #[test]
    fn copy_output_reads_from_the_start_without_moving_the_shared_offset() {
        use std::io::{Seek, SeekFrom};

        let mut file = tempfile::tempfile().expect("temp file should be created");
        let content = "x".repeat(20_000) + "end";
        file.write_all(content.as_bytes())
            .expect("write should succeed");

        let mut copied = Vec::new();
        copy_output(&file, &mut copied).expect("copy should succeed");

        assert_eq!(copied, content.as_bytes());
        assert_eq!(
            file.stream_position().expect("position should be readable"),
            content.len() as u64,
            "a background writer's offset must not be reset"
        );
        assert_eq!(file.seek(SeekFrom::End(0)).unwrap(), content.len() as u64);
    }

    #[test]
    fn warnings_cover_unknown_profiles_and_targets() {
        let config = ChromasyncConfig {
            configs: vec![SyncProfile {
                name: "default".to_owned(),
                seed: Some("#4ecdc4".to_owned()),
                image: None,
                image_fetch_command: None,
                template: None,
                mode: Default::default(),
                contrast: Default::default(),
                chroma: Default::default(),
                targets: vec!["kitty".to_owned()],
                output_dir: PathBuf::from("out"),
                force: false,
            }],
            hooks: vec![
                config_hook("ok", &["target:kitty:done"], &["config:default"]),
                config_hook("typo-profile", &["targets:done"], &["config:defualt"]),
                config_hook("typo-target", &["target:kity:changed"], &[]),
                config_hook(
                    "other-profile",
                    &["target:only-work:done"],
                    &["config:work"],
                ),
            ],
            ..ChromasyncConfig::default()
        };
        let hooks = config.validate_hooks().expect("hooks should validate");

        let warnings =
            unknown_reference_warnings(&hooks, &config, "default", |name| name == "kitty");

        assert_eq!(warnings.len(), 3, "{warnings:#?}");
        assert!(warnings[0].contains("'typo-profile' filter 'config:defualt'"));
        assert!(warnings[1].contains("'typo-target' event 'target:kity:changed'"));
        assert!(warnings[2].contains("'other-profile' filter 'config:work'"));
    }
}
