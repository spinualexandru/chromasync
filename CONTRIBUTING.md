# Contributing to Chromasync

Thanks for your interest in contributing.

Chromasync is a Rust workspace for generating theme artifacts from seed colors or wallpapers. Most contributions touch one of the crates under `crates/`, the built-in templates under `templates/`, the example targets under `examples/targets/`, or the docs under `docs/` and `book/`.

## Before You Start

Pull requests from first-time or otherwise unvouched contributors are automatically closed until a maintainer vouches for the author.

If you are not yet vouched, open an issue first and describe the change you want to make. A maintainer can comment `vouch`, `vouch @user`, `lgtm`, or `lgtm @user` on the issue or PR to add you to the trusted contributor list in `.github/VOUCHED.td`.

## Development Setup

Chromasync targets stable Rust.

Useful starting commands:

```bash
cargo run -- --help
cargo build --release -p chromasync-cli
```

## Repository Layout

- `crates/chromasync-cli`: CLI entrypoints and command wiring
- `crates/chromasync-core`: palette generation pipeline and pack discovery
- `crates/chromasync-color`: color math and palette helpers
- `crates/chromasync-extract`: wallpaper seed extraction
- `crates/chromasync-template`: template loading and token expansion
- `crates/chromasync-renderers`: built-in renderers and declarative target registry
- `crates/chromasync-types`: shared data structures
- `templates/`: built-in template TOML files
- `examples/targets/`: declarative example targets such as GTK, Hyprland, CSS, Waybar, Foot, and editor outputs
- `docs/`: longer design and packaging notes
- `book/`: mdBook source, including generated reference pages

Keep new code in the crate that owns the behavior. If multiple crates need a model, move it to `chromasync-types` instead of duplicating it.

## Making Changes

- Follow standard Rust naming and formatting conventions.
- Keep public APIs narrow and error messages explicit.
- Add or update tests in the crate that owns the behavior.
- Use `tests/fixtures/` for stable inputs and golden outputs.
- If renderer output changes, update the matching `.golden` fixture and rerun the relevant test, for example:

```bash
cargo test -p chromasync-renderers --test golden
```

- If you are working on the optional performance harness, run:

```bash
cargo test -p chromasync-core --test perf -- --ignored --nocapture
```

## Docs and Generated Files

The mdBook content is partly generated. If you change the README, CLI help text, built-in templates, or built-in targets, regenerate the book files:

```bash
cargo run -p chromasync-docs -- generate
```

Before opening a PR, verify generated docs are current:

```bash
cargo run -p chromasync-docs -- generate --check
```

If you want to preview the book locally, install `mdbook` and run:

```bash
mdbook serve --open
```

## Before Opening a PR

Run the same checks as CI:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release -p chromasync-cli
cargo run -p chromasync-docs -- generate --check
```

## Pull Request Notes

In the PR description, explain:

- the user-visible change
- the crates or areas you touched
- any fixture, golden file, or generated-doc updates

Small, focused PRs are easier to review than broad refactors mixed with feature work.
