Declarative Chromasync target examples.

These target TOML files provide `gtk`, `hyprland`, `css`, `waybar`, `foot`,
`ghostty`, and `editor` outputs. Use them with commands such as:

```bash
cargo run -- generate \
  --seed "#4ecdc4" \
  --template minimal \
  --targets examples/targets/gtk.toml,kitty
```
