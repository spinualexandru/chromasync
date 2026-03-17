Declarative Chromasync target examples.

These target TOML files match the former built-in `gtk`, `hyprland`, `css`,
`waybar`, `foot`, and `editor` outputs. Use them with commands such as:

```bash
cargo run -- generate \
  --seed "#4ecdc4" \
  --template minimal \
  --targets examples/targets/gtk.toml,kitty
```
