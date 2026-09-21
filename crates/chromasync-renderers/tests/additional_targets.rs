use std::collections::BTreeMap;

use chromasync_color::{contrast_ratio, generate_palette};
use chromasync_renderers::OutputRegistry;
use chromasync_template::{built_in_templates, resolve_tokens_with_strategy};
use chromasync_types::{ChromaStrategy, ContrastStrategy, GenerationContext};

fn assignments(content: &str, separator: char) -> BTreeMap<String, String> {
    content
        .lines()
        .filter_map(|line| line.split_once(separator))
        .map(|(key, value)| {
            (
                key.trim().to_owned(),
                value.trim().trim_end_matches(';').to_owned(),
            )
        })
        .collect()
}

fn readable(foreground: &str, background: &str, usage: &str) {
    let ratio = contrast_ratio(foreground, background).expect("valid emitted colors");
    assert!(
        ratio >= 4.5,
        "{usage}: {foreground} on {background}: {ratio}"
    );
}

#[test]
fn additional_targets_preserve_readable_colors_and_native_formats() {
    let registry = OutputRegistry::default();
    let templates = built_in_templates().expect("built-in templates");
    let names = [
        "claude",
        "herdr",
        "hyprtoolkit",
        "kvantum",
        "limine",
        "steam",
        "vim",
    ]
    .map(str::to_owned);

    for template in templates {
        for seed in ["#000000", "#FFFFFF", "#FF0000", "#4ECDC4"] {
            for chroma in [
                ChromaStrategy::Subtle,
                ChromaStrategy::Normal,
                ChromaStrategy::Vibrant,
            ] {
                let palette = generate_palette(seed, template.definition.mode, chroma).unwrap();
                let tokens = resolve_tokens_with_strategy(
                    &palette,
                    &template.definition,
                    ContrastStrategy::RelativeLuminance,
                )
                .unwrap();
                let ctx = GenerationContext {
                    mode: template.definition.mode,
                    template_name: template.definition.name.clone(),
                    chroma,
                    output_dir: "/unused".into(),
                    seed: Some(seed.to_owned()),
                };
                let artifacts = registry.generate(&names, &tokens, &ctx).unwrap();
                let content = |target: &str, file: &str| {
                    artifacts
                        .iter()
                        .find(|a| a.target == target && a.file_name == file)
                        .unwrap()
                        .content
                        .as_str()
                };

                let claude: serde_json::Value =
                    serde_json::from_str(content("claude", "chromasync.json")).unwrap();
                assert_eq!(claude["base"], ctx.mode.as_str());
                let overrides = &claude["overrides"];
                for background in [
                    "userMessageBackground",
                    "userMessageBackgroundHover",
                    "bashMessageBackgroundColor",
                    "memoryBackgroundColor",
                    "selectionBg",
                ] {
                    readable(
                        overrides["text"].as_str().unwrap(),
                        overrides[background].as_str().unwrap(),
                        "Claude message text",
                    );
                }
                assert!(
                    overrides.get("diffAdded").is_none(),
                    "retain base diff backgrounds instead of using a foreground token as a background"
                );

                let herdr: toml::Value =
                    toml::from_str(content("herdr", "chromasync-herdr.toml")).unwrap();
                assert_eq!(herdr["theme"]["auto_switch"].as_bool(), Some(false));
                let colors = &herdr["theme"]["custom"];
                for foreground in ["text", "subtext0"] {
                    for background in [
                        "panel_bg",
                        "sidebar_bg",
                        "active_row_bg",
                        "selection_bg",
                        "surface0",
                        "surface1",
                    ] {
                        readable(
                            colors[foreground].as_str().unwrap(),
                            colors[background].as_str().unwrap(),
                            "Herdr text",
                        );
                    }
                }

                let kv = assignments(content("kvantum", "chromasync.kvconfig"), '=');
                for (fg, bg) in [
                    ("text.color", "base.color"),
                    ("window.text.color", "window.color"),
                    ("button.text.color", "button.color"),
                    ("highlight.text.color", "highlight.color"),
                    ("tooltip.text.color", "tooltip.base.color"),
                ] {
                    readable(&kv[fg], &kv[bg], "Kvantum palette");
                }

                let hypr = assignments(content("hyprtoolkit", "chromasync-hyprtoolkit.conf"), '=');
                assert_eq!(hypr.len(), 8);
                let color = |name: &str| format!("#{}", &hypr[name][4..]);
                readable(&color("text"), &color("base"), "hyprtoolkit text");
                // The toolkit uses background on bright accents and bright_text on dark accents.
                readable(
                    &color("bright_text"),
                    &color("accent"),
                    "hyprtoolkit dark accent label",
                );
                assert_eq!(color("bright_text"), tokens.accent_fg.to_lowercase());

                let limine = assignments(content("limine", "chromasync-limine.conf"), ':');
                assert_eq!(limine.len(), 9);
                for name in ["term_palette", "term_palette_bright"] {
                    let entries = limine[name].split(';').collect::<Vec<_>>();
                    assert_eq!(entries.len(), 8);
                    assert!(
                        entries.iter().all(|color| color.len() == 6
                            && color.chars().all(|c| c.is_ascii_hexdigit()))
                    );
                }
                assert!(limine["term_background"].starts_with("00"));
                readable(
                    &format!("#{}", limine["term_foreground"]),
                    &format!("#{}", &limine["term_background"][2..]),
                    "Limine normal text",
                );

                let manifest: serde_json::Value =
                    serde_json::from_str(content("steam", "skin.json")).unwrap();
                assert_eq!(manifest["Patches"][0]["TargetCss"], "chromasync.css");
                let css = assignments(content("steam", "chromasync.css"), ':');
                readable(
                    &css["--chromasync-text"],
                    &css["--chromasync-surface"],
                    "Steam shared controls",
                );
                readable(
                    &css["--chromasync-accent-fg"],
                    &css["--chromasync-accent"],
                    "Steam primary buttons",
                );

                let vim = content("vim", "chromasync.vim");
                assert!(vim.contains(&format!("set background={}", ctx.mode)));
                for line in vim.lines().filter(|line| line.starts_with("highlight ")) {
                    // FloatBorder draws a decorative outline, not readable text.
                    if line.starts_with("highlight FloatBorder ") {
                        continue;
                    }
                    let attrs = line
                        .split_whitespace()
                        .filter_map(|part| part.split_once('='))
                        .collect::<BTreeMap<_, _>>();
                    if let (Some(fg), Some(bg)) = (attrs.get("guifg"), attrs.get("guibg"))
                        && fg.starts_with('#')
                        && bg.starts_with('#')
                    {
                        readable(fg, bg, line);
                    }
                }
            }
        }
    }
}
