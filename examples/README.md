Declarative Chromasync target examples.

These target TOML files provide `gtk`, `hyprland`, `css`, `waybar`, `foot`,
`ghostty`, and `editor` outputs. Use them with commands such as:

```bash
cargo run -- generate \
  --seed "#4ecdc4" \
  --template minimal \
  --chroma vibrant \
  --targets examples/targets/gtk.toml,kitty
```

You can also specify `chroma` strategies (subtle, normal, vibrant, muted, industrial)
within a batch manifest or as a per-target override in target TOML files.
