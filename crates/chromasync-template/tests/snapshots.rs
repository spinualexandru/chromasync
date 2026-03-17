use chromasync_color::generate_palette;
use chromasync_template::{built_in_templates, resolve_tokens_with_strategy};
use chromasync_types::{ContrastStrategy, SemanticTokens, ThemeMode};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct TemplateSnapshot<'a> {
    template: &'a str,
    mode: ThemeMode,
    contrast: ContrastStrategy,
    tokens: SemanticTokens,
}

#[test]
fn built_in_template_tokens_match_snapshot() {
    let templates = built_in_templates().expect("templates should load");
    let mut snapshots = Vec::with_capacity(templates.len() * 2);

    for template in &templates {
        let palette =
            generate_palette("#4ecdc4", template.definition.mode).expect("palette should build");

        for contrast in [
            ContrastStrategy::RelativeLuminance,
            ContrastStrategy::ApcaExperimental,
        ] {
            let tokens = resolve_tokens_with_strategy(&palette, &template.definition, contrast)
                .expect("template should resolve");

            snapshots.push(TemplateSnapshot {
                template: &template.definition.name,
                mode: template.definition.mode,
                contrast,
                tokens,
            });
        }
    }

    let actual = serde_json::to_string_pretty(&snapshots).expect("snapshots should serialize");
    let expected = include_str!("fixtures/builtin-template-tokens.golden.json");

    assert_eq!(actual, expected.trim());
}
