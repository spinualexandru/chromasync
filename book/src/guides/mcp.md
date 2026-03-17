# MCP Server

The `chromasync-mcp` binary exposes Chromasync as a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server. This lets AI assistants generate themes, preview palettes, export tokens, and query available templates, targets, and packs — using the same pipeline as the CLI.

## Running the server

The server communicates over stdio using JSON-RPC:

```bash
chromasync-mcp
```

It reads MCP requests from stdin and writes responses to stdout. Logs go to stderr and can be controlled with the `RUST_LOG` environment variable:

```bash
RUST_LOG=info chromasync-mcp
```

## Configuring with Claude Code

Add to your Claude Code MCP settings (`.claude/settings.json` or project-level):

```json
{
  "mcpServers": {
    "chromasync": {
      "command": "chromasync-mcp"
    }
  }
}
```

If the binary is not on your `PATH`, use the full path:

```json
{
  "mcpServers": {
    "chromasync": {
      "command": "/path/to/chromasync-mcp"
    }
  }
}
```

## Available tools

The server exposes 10 tools:

### Theme generation

| Tool | Description |
| --- | --- |
| `generate` | Generate theme artifacts from a seed color and write them to disk |
| `wallpaper` | Generate theme artifacts from a wallpaper image and write them to disk |
| `batch` | Execute a TOML batch manifest containing multiple generation jobs |

### Inspection (read-only)

| Tool | Description |
| --- | --- |
| `preview` | Preview palette families and resolved semantic tokens for a seed color |
| `export_tokens` | Export the 17 resolved semantic token hex values as JSON |
| `generate_palette` | Generate the full OKLCH palette (9 families, 16 tones each) from a seed color |

### Discovery (read-only)

| Tool | Description |
| --- | --- |
| `list_templates` | List all available templates with their name, mode, source, and location |
| `list_targets` | List all available render targets with their name, source, and location |
| `list_packs` | List all discovered theme packs |
| `pack_info` | Get metadata, templates, and targets for a specific theme pack |

## Common parameters

Several tools share common parameters:

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `seed` | string | (required) | Seed color in `#RRGGBB` hex format |
| `template` | string | (required) | Template name or path to a `.toml` file |
| `mode` | string | `"dark"` | Theme mode: `"dark"` or `"light"` |
| `contrast` | string | `"relative-luminance"` | Contrast strategy: `"relative-luminance"` or `"apca-experimental"` |
| `targets` | string[] | (required for generation) | Target names or paths to target TOML files |
| `output_dir` | string | (required for generation) | Directory to write artifact files into |

## Example interactions

### Generate a theme

```json
{
  "name": "generate",
  "arguments": {
    "seed": "#ff6b6b",
    "template": "brutalist",
    "mode": "dark",
    "targets": ["kitty", "alacritty"],
    "output_dir": "./my-theme"
  }
}
```

Returns a JSON array of written artifact paths.

### Preview tokens without writing files

```json
{
  "name": "preview",
  "arguments": {
    "seed": "#4ecdc4",
    "template": "minimal"
  }
}
```

Returns a human-readable summary of palette families and semantic tokens.

### Export tokens as JSON

```json
{
  "name": "export_tokens",
  "arguments": {
    "seed": "#7c3aed",
    "template": "terminal",
    "mode": "light"
  }
}
```

Returns the 17 semantic token hex values as a JSON object.

### Discover available templates

```json
{
  "name": "list_templates",
  "arguments": {}
}
```

Returns a JSON array with each template's name, mode, description, source, and location.

## Output format

- **Generation tools** (`generate`, `wallpaper`, `batch`) return JSON arrays describing written files, including the target name, file name, and full path.
- **`preview`** returns plain text with palette families and semantic tokens.
- **`export_tokens`** and **`generate_palette`** return structured JSON.
- **Discovery tools** (`list_templates`, `list_targets`, `list_packs`, `pack_info`) return JSON arrays or objects.

## Overwrite protection

Like the CLI, the `generate`, `wallpaper`, and `batch` tools refuse to overwrite existing files. If an artifact already exists at the destination, the tool returns an error. Delete or move the existing output first, or use a different `output_dir`.

## Building from source

```bash
cargo build --release -p chromasync-mcp
```

The binary is written to `target/release/chromasync-mcp`.
