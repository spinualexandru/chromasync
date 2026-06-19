# Chromasync

A Rust CLI that generates consistent theme files for desktop apps and editors from a seed color or wallpaper image.

**Seed/Wallpaper → OKLCH Palette → Template Rules → Theme Files**

## Install

```bash
cargo install --locked --path crates/chromasync-cli
```

Or build from source:

```bash
cargo build --release -p chromasync-cli
```

## Usage

```bash
# Generate from a seed color
chromasync generate --seed "#ff6b6b" --template brutalist --mode dark \
  --targets kitty,alacritty,examples/targets/gtk.toml

# Generate from a wallpaper
chromasync wallpaper --image wallpaper.png --template materialish --mode light \
  --targets kitty,examples/targets/css.toml

# Run the default sync profile from ~/.config/chromasync/config.toml
chromasync sync

# Run a named sync profile
chromasync sync work

# Install a custom target and record where its artifacts should be written
chromasync target install --target examples/targets/gtk.toml --outdir ~/.config/gtk-4.0

# Preview palette and tokens without writing files
chromasync preview --seed "#4ecdc4" --template minimal --mode light

# Export tokens as JSON
chromasync tokens --seed "#7c3aed" --template terminal --mode dark --format json

# Batch multiple jobs from a manifest
chromasync batch --file jobs.toml
```

Output is written to `./chromasync` by default.

## Sync

Use `chromasync sync` when you want Chromasync to read a saved profile from
`~/.config/chromasync/config.toml` and write each target to its configured
destination. A profile can use a fixed seed, a fixed wallpaper image, or a
command that returns the current wallpaper path:

```toml
[[configs]]
name = "default"
image_fetch_command = "qs -c noctalia-shell ipc call wallpaper get"
template = "materialish"
mode = "auto"
targets = ["kitty"]
chroma = "industrial"

[[targets]]
name = "kitty"
output_dir = "~/.config/kitty"
overwrite = true

[[hooks]]
name = "reload-kitty"
on = "target:kitty:done"
filters = ["config:default"]
command = "kitty @ load-config"
```

Run it with:

```bash
chromasync sync          # uses the profile named "default"
chromasync sync work     # uses the profile named "work"
```

`mode = "auto"` follows the desktop color-scheme when it can be detected, and
falls back to dark mode.

Hooks run after a successful `chromasync sync` write. Use `targets:done` to run
after every target is generated, or `target:<name>:done` for a specific target.
Hook commands run from the directory containing `config.toml`; a failing hook
makes `sync` exit with an error after artifacts have been written.

## Installing Targets

Built-in targets can be used by name. Declarative target specs, such as the
examples under [`examples/targets/`](examples/targets/), can be installed into
the user config and assigned an output directory:

```bash
chromasync target install \
  --target examples/targets/gtk.toml \
  --outdir ~/.config/gtk-4.0
```

This copies the target TOML into `~/.config/chromasync/targets/` and records a
matching `[[targets]]` entry in `~/.config/chromasync/config.toml`. Add
`--overwrite` to replace an existing installed target and mark its generated
artifacts as overwrite-safe during generation.

Once installed, the target can be referenced by name:

```bash
chromasync generate --seed "#4ecdc4" --targets gtk
chromasync sync
```

## Built-in Templates & Targets

| Templates                                          | Targets                                                       |
| -------------------------------------------------- | ------------------------------------------------------------- |
| `minimal`, `brutalist`, `terminal`, `materialish`  | `kitty`, `alacritty`, `ghostty`, `hyprland`, `hyprland-lua`, `zed` |

Additional targets (GTK, CSS, Waybar, Foot, Editor) are
available as declarative TOML specs under
[`examples/targets/`](examples/targets/).

```bash
chromasync templates   # list available templates
chromasync targets     # list available targets
```

## Documentation

An [mdBook](https://rust-lang.github.io/mdBook/) is included under `book/`:

```bash
mdbook serve --open       # preview locally
```

To regenerate book source from CLI metadata:

```bash
cargo run -p chromasync-docs -- generate
```

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p chromasync-docs -- generate --check
```

## Contributing

Pull requests from first-time or otherwise unvouched contributors are automatically closed until a maintainer vouches for the author. This is in place to reduce spammy or low-signal PRs.

If you want to contribute and are not yet vouched, open an issue describing the change you want to make or the area you want to work on. A maintainer can then comment `vouch`, `vouch @user`, `lgtm`, or `lgtm @user` on the issue or PR to add you to the trusted contributor list.

Maintainers can also comment `unvouch` or `denounce` to remove trust or explicitly block an account when needed. The trust list lives in `.github/VOUCHED.td`.

See the [Packaging guide](./docs/PACKAGING.md) for release and packaging details.
