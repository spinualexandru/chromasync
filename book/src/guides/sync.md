# Sync

The `sync` command runs a named profile from `~/.config/chromasync/config.toml`.
It is intended for repeatable desktop theming setups where the seed, template,
mode, targets, and output directories should live in one config file.

## Basic usage

```bash
chromasync sync
```

This runs the profile named `default`.

```bash
chromasync sync personalGreen
```

This runs the profile named `personalGreen`.

```bash
chromasync sync --mode dark
```

This overrides the profile's configured mode for one sync. The override is
useful when an external script, such as a desktop light/dark switcher, calls
`chromasync sync` and already knows the incoming mode before the desktop
setting has changed. (This is unrelated to Chromasync's own `[[hooks]]`, which
are described below.)

```bash
chromasync sync --no-hooks
```

This runs the profile without validating or running any `[[hooks]]`.

## Config format

Sync profiles live under `[[configs]]`:

```toml
[[configs]]
name = "default"
image_fetch_command = "qs -c noctalia-shell ipc call wallpaper get"
template = "materialish"
mode = "auto"
targets = ["ghostty", "kitty"]
chroma = "industrial"

[[targets]]
name = "ghostty"
output_dir = "~/.config/ghostty/themes"
overwrite = true

[[targets]]
name = "kitty"
output_dir = "~/.config/kitty"
overwrite = true

[[hooks]]
name = "reload-kitty"
on = "target:kitty:changed"
filters = ["config:default"]
command = "kitty @ load-config"
```

Each profile must define exactly one color source:

- `seed = "#RRGGBB"` for deterministic seed-based generation.
- `image = "wallpaper.png"` for wallpaper-based generation.
- `image_fetch_command = "qs -c noctalia-shell ipc call wallpaper get"` for
  wallpaper-based generation from a command that prints the current image path.

Relative `image`, fetched image path, template path, target TOML path, and
profile `output_dir` values are resolved from the directory containing
`config.toml`. The fetch command runs from that same directory and Chromasync
uses its first non-empty stdout line as the image path.

## Profile fields

| Field | Required | Default | Description |
| --- | --- | --- | --- |
| `name` | yes | - | Profile name selected by `chromasync sync [name]` |
| `seed`, `image`, or `image_fetch_command` | yes | - | Exactly one color source |
| `template` | no | target preference | Template name or `.toml` path |
| `mode` | no | `dark` | `dark`, `light`, or `auto` |
| `contrast` | no | `relative-luminance` | Contrast strategy |
| `chroma` | no | `normal` | Chroma strategy |
| `targets` | yes | - | Non-empty list of target names or paths |
| `output_dir` | no | `chromasync` | Fallback output directory |
| `force` | no | `false` | Overwrite fallback-target artifacts whose content differs |

Without `force`, a sync refuses to replace an existing file whose content
differs and exits with status 1 before writing anything. A file that already
holds exactly the generated content is accepted either way and left untouched,
so re-running a sync that produces the same theme succeeds without `force`.

## Auto Mode

`mode = "auto"` runs:

```bash
gsettings get org.gnome.desktop.interface color-scheme
```

`prefer-light` maps to light mode and `prefer-dark` maps to dark mode. If the
desktop preference cannot be inferred, Chromasync uses dark mode.

## Target Output Directories

`sync` uses the same `[[targets]]` output mapping as `generate`, `wallpaper`, and
`batch`. A target listed in the profile writes to its configured `output_dir`
when a matching `[[targets]]` entry exists. Targets without an entry write to
the profile's `output_dir` fallback.

Custom targets such as Ghostty must still be discoverable, usually by placing
their TOML file under `~/.config/chromasync/targets/` or by installing them with
`chromasync target install`.

## Hooks

Hooks live under top-level `[[hooks]]` entries in `config.toml` and run only for
`chromasync sync`, never for `generate`, `wallpaper`, or `batch`. They run after
every artifact has been written, or found already up to date.

```toml
[[hooks]]
name = "reload-hyprland"
on = "target:hyprland-lua:changed"
command = "hyprctl reload"

[[hooks]]
name = "notify"
on = "targets:done"
filters = ["config:default", "config:work"]
command = "notify-send Chromasync \"Synced $CHROMASYNC_PROFILE ($CHROMASYNC_MODE)\""
```

| Field | Required | Description |
| --- | --- | --- |
| `name` | yes | Unique, non-blank name used in messages; surrounding spaces are ignored |
| `on` | yes | One event, or a non-empty array of events |
| `command` | yes | Non-empty shell command |
| `filters` | no | Conditions on the run, such as `config:<profile-name>` |

### Events

| Event | Fires when |
| --- | --- |
| `targets:done` | The sync generated at least one artifact. Fires on every successful sync, even if no file had to be rewritten. |
| `targets:changed` | At least one artifact's content differed from the file on disk and was written. |
| `target:<name>:done` | Target `<name>` generated artifacts. Fires on every successful sync. |
| `target:<name>:changed` | At least one of target `<name>`'s artifacts differed and was written. |

Chromasync compares each artifact with the existing file and leaves an identical
file untouched, keeping its modification time. This works with the default
`force = false` too: only files whose content differs need `force`. Prefer the
`changed` events for reload hooks, so applications reload only when the theme
actually changed, for example not when a wallpaper command returns the same
image again. Every artifact path is still printed on stdout.

The `<name>` in a target event is the name the target generated under. For a
target listed by path, such as `targets/my-theme.toml`, that is the `name` field
inside the TOML file, not its file name. Spaces around the name are ignored, so
`target: kitty :done` is the same as `target:kitty:done`.

A hook runs at most once per sync, even if several of its events fire.

### Filters

`config:<profile-name>` limits a hook to the named `[[configs]]` profile.
Filters of the same kind are OR'd, so `filters = ["config:default",
"config:work"]` runs for either profile. Filters of different kinds would be
AND'd, but `config:` is currently the only kind. A hook without filters runs for
every profile. Spaces around the profile name are ignored.

### Validation

`chromasync sync` checks every hook before it generates or writes anything. The
following are errors, and the sync exits with status 1 without touching any
files:

- an event that is not one of the four forms above, such as `target:done` or
  `targets:finished`, or a target event with an empty target name
  (`target::done`)
- a filter other than `config:<profile-name>`, such as `profile:default`
- an empty `on = []`, or an empty or blank `name` or `command`
- two hooks with the same `name`, ignoring surrounding spaces

These are warnings, printed to stderr, and the sync continues:

- a `config:` filter that names no `[[configs]]` profile
- a target event, on a hook that applies to the current profile, naming a target
  that is neither installed nor built in and was not generated by this sync

Hooks are not checked by `generate`, `wallpaper`, or `batch`, so a mistake in
`[[hooks]]` does not break them. `chromasync sync --no-hooks` skips both the
checks and the hooks.

### How hooks run

- Hooks run in the order they appear in `config.toml`, one at a time.
- Commands run with `sh -c` (`cmd /C` on Windows), not your login shell, so
  fish or zsh-only syntax does not work. Call `fish -c '...'` explicitly if you
  need it.
- The working directory is the directory containing `config.toml`.
- A hook's stdout and stderr are captured together and copied to Chromasync's
  stderr when the hook exits, so hook output appears after each hook finishes
  rather than as it is produced. Chromasync's stdout lists only artifact paths,
  one per line, including unchanged ones, so it can be piped. If the reader
  closes the pipe early (`chromasync sync | head -1`), Chromasync stops
  printing paths but still finishes the sync and runs its hooks.
- Stdin is closed (`/dev/null`).

Each hook receives these environment variables:

| Variable | Value |
| --- | --- |
| `CHROMASYNC_PROFILE` | The profile name, such as `default` |
| `CHROMASYNC_MODE` | The resolved mode, `light` or `dark`, after `--mode` and `mode = "auto"` |
| `CHROMASYNC_CONFIG_DIR` | The directory containing `config.toml` |
| `CHROMASYNC_TARGETS` | Names of the targets that produced artifacts, one per line |
| `CHROMASYNC_ARTIFACTS` | Every artifact path generated by this sync, one per line, whether or not the file was rewritten |
| `CHROMASYNC_CHANGED_ARTIFACTS` | Paths whose content changed and were written in this run, one per line (empty if none) |

### Failures and exit status

If a hook exits non-zero or cannot be started, the remaining matching hooks
still run. Afterwards `chromasync sync` reports every failed hook in one error,
such as `2 of 3 hooks failed: 'a' exited with exit status: 1, 'c' exited with
exit status: 2`, and exits with status **3**. Artifacts have already been
written at that point and are not rolled back.

| Exit status | Meaning |
| --- | --- |
| 0 | Success |
| 1 | Config, validation, generation, or write error; no hooks ran. Also used, after hooks have run, when the artifact paths could not be printed for a reason other than a closed stdout. |
| 2 | Invalid command-line usage |
| 3 | Artifacts were written or already up to date, but at least one hook failed |

All existence and collision checks run before the first file is written. If a
write still fails partway through, for example because a disk is full, files
written before the failure stay updated, the sync exits with status 1, and no
hooks run.

### Long-running and background commands

Chromasync waits for each hook command to exit, with no timeout. To bound a
command that might hang, wrap it with `timeout`:

```toml
[[hooks]]
name = "reload-kitty"
on = "target:kitty:changed"
command = "timeout 5 kitty @ load-config"
```

To restart a daemon, start it in the background:

```toml
[[hooks]]
name = "restart-waybar"
on = "targets:changed"
command = "pkill waybar; waybar >/dev/null 2>&1 &"
```

`sync` returns as soon as the shell exits and does not wait for the background
process. A background process never inherits Chromasync's own stdout or stderr,
only the hook's temporary output file, so a caller that captures Chromasync's
output, such as `out=$(chromasync sync 2>&1)`, is not held up by it either.

Output that a background process writes after its hook has exited is
discarded, but it still accumulates in that deleted temporary file until the
process exits. Redirect a long-running daemon's output, as above, or detach it
with `setsid -f waybar >/dev/null 2>&1`.

### Example: merging a fragment into another config

Some targets, such as [Herdr](targets.md#herdr), produce a fragment that must
be merged into an existing config file. A hook can find the generated file via
`CHROMASYNC_ARTIFACTS`:

```toml
[[hooks]]
name = "merge-herdr"
on = "target:herdr:changed"
command = """
fragment=$(printf '%s\n' "$CHROMASYNC_ARTIFACTS" | grep '/chromasync-herdr.toml$')
./merge-herdr-theme "$fragment" ~/.config/herdr/config.toml
"""
```

Here `merge-herdr-theme` is your own TOML-aware merge script, stored next to
`config.toml`.
