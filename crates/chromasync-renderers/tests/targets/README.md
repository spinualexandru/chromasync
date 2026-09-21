# Additional target validation

Run the Rust golden/format/contrast tests with:

```sh
cargo test -p chromasync-renderers
```

The format/contrast audit renders the new targets over all eight built-in
light/dark template variants, four seeds, and three chroma strategies. Golden
fixtures cover every new artifact. CLI smoke tests cover target discovery.

## Native editors and structured formats

Requires Python 3.11+, Vim with true-color support, and Neovim with `-l` support.
This command generates all nine targets in isolated XDG directories, validates
JSON, TOML, TextMate plist, Kvantum SVG IDs, and palette encodings, then loads
both editor colorschemes in native headless processes. It checks dark and light
modes, resolved highlight colors, Tree-sitter links, diagnostics, and terminal
palette entries.

```sh
cargo build -p chromasync
python crates/chromasync-renderers/tests/targets/verify.py
```

To retain generated files for inspection:

```sh
python crates/chromasync-renderers/tests/targets/verify.py --output target/targets-audit
```

## Kvantum runtime

`kvantum.cpp` loads the real Kvantum Qt 6 style plugin, checks eight palette roles,
and renders a widget sample. Requires Qt 6 development files and a Kvantum
plugin built against the installed Qt. Compile with:

```sh
c++ -std=c++17 -fPIC crates/chromasync-renderers/tests/targets/kvantum.cpp \
  -o target/targets-audit/kvantum-verify $(pkg-config --cflags --libs Qt6Widgets)
```

Use an isolated directory, never the user's live Kvantum configuration:

```sh
mkdir -p target/targets-audit/qt-dark/Kvantum/chromasync
cp target/targets-audit/dark/kvantum/* target/targets-audit/qt-dark/Kvantum/chromasync/
printf '[General]\ntheme=chromasync\n' > target/targets-audit/qt-dark/Kvantum/kvantum.kvconfig
XDG_CONFIG_HOME="$PWD/target/targets-audit/qt-dark" \
  QT_QPA_PLATFORM=offscreen QT_QPA_PLATFORMTHEME= \
  target/targets-audit/kvantum-verify \
  target/targets-audit/qt-dark/Kvantum/chromasync/chromasync.kvconfig \
  target/targets-audit/kvantum-dark.png
```

Repeat with `light` substituted for `dark`. For an uninstalled plugin, add
`QT_PLUGIN_PATH=/path/to/plugins`, with its library at
`/path/to/plugins/styles/libkvantum.so`. Inspect both PNGs.

These checks do not establish live Steam/Millennium selector coverage, a
booted Limine screen, or live Codex/Claude/Herdr/hyprtoolkit behavior. Steam is
explicitly limited to shared desktop dialogs and controls. Herdr and Limine
emit fragments that must be merged into the owning configuration.
