use std::path::PathBuf;

use chromasync_renderers::{OutputRegistry, built_in_targets, render_target, render_targets};
use chromasync_types::{
    ChromaStrategy, GenerationContext, RenderTarget, SemanticTokens, ThemeMode,
};

#[test]
fn gtk_example_target_matches_golden_file() {
    assert_example_target_matches_golden(
        "gtk.toml",
        "gtk",
        "gtk.css",
        include_str!("fixtures/gtk.css.golden"),
    );
}

#[test]
fn hyprland_example_target_matches_golden_file() {
    assert_example_target_matches_golden(
        "hyprland.toml",
        "hyprland",
        "hyprland.conf",
        include_str!("fixtures/hyprland.conf.golden"),
    );
}

#[test]
fn kitty_renderer_matches_golden_file() {
    assert_matches_golden(
        RenderTarget::Kitty,
        include_str!("fixtures/kitty.conf.golden"),
    );
}

#[test]
fn css_example_target_matches_golden_file() {
    assert_example_target_matches_golden(
        "css.toml",
        "css",
        "theme.css",
        include_str!("fixtures/theme.css.golden"),
    );
}

#[test]
fn alacritty_renderer_matches_golden_file() {
    assert_matches_golden(
        RenderTarget::Alacritty,
        include_str!("fixtures/alacritty.toml.golden"),
    );
}

#[test]
fn foot_example_target_matches_golden_file() {
    assert_example_target_matches_golden(
        "foot.toml",
        "foot",
        "foot.ini",
        include_str!("fixtures/foot.ini.golden"),
    );
}

#[test]
fn ghostty_example_target_matches_golden_file() {
    assert_example_target_matches_golden(
        "ghostty.toml",
        "ghostty",
        "colors.txt",
        include_str!("fixtures/ghostty.colors.golden"),
    );
}

#[test]
fn waybar_example_target_matches_golden_file() {
    assert_example_target_matches_golden(
        "waybar.toml",
        "waybar",
        "style.css",
        include_str!("fixtures/style.css.golden"),
    );
}

#[test]
fn editor_example_target_matches_golden_file() {
    assert_example_target_matches_golden(
        "editor.toml",
        "editor",
        "theme.json",
        include_str!("fixtures/theme.json.golden"),
    );
}

#[test]
fn built_in_targets_render_to_generated_artifacts() {
    let artifacts = render_targets(built_in_targets(), &sample_tokens())
        .expect("built-in targets should render");

    assert_eq!(artifacts.len(), 2);
    assert_eq!(
        artifacts
            .iter()
            .map(|artifact| artifact.target.clone())
            .collect::<Vec<_>>(),
        vec!["kitty".to_owned(), "alacritty".to_owned()]
    );
    assert_eq!(
        artifacts
            .iter()
            .map(|artifact| artifact.file_name.as_str())
            .collect::<Vec<_>>(),
        vec!["kitty.conf", "alacritty.toml"]
    );
}

fn assert_matches_golden(target: RenderTarget, expected: &str) {
    let artifact = render_target(target, &sample_tokens()).expect("renderer should succeed");

    assert_eq!(artifact.target, target.as_str());
    assert_eq!(artifact.file_name, target.file_name());
    assert_eq!(artifact.content, expected);
}

fn assert_example_target_matches_golden(
    target_file: &str,
    expected_target: &str,
    expected_file_name: &str,
    expected: &str,
) {
    let target_path = example_target_path(target_file);
    let registry = OutputRegistry::default();
    let artifacts = registry
        .generate(
            &[target_path.display().to_string()],
            &sample_tokens(),
            &sample_context(),
        )
        .expect("example target should succeed");

    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].target, expected_target);
    assert_eq!(artifacts[0].file_name, expected_file_name);
    assert_eq!(artifacts[0].content, expected);
}

fn example_target_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/targets")
        .join(name)
}

fn sample_context() -> GenerationContext {
    GenerationContext {
        mode: ThemeMode::Dark,
        template_name: "minimal".to_owned(),
        chroma: ChromaStrategy::Normal,
        output_dir: PathBuf::from("/tmp/chromasync-test-output"),
        seed: Some("#4ecdc4".to_owned()),
    }
}

fn sample_tokens() -> SemanticTokens {
    SemanticTokens {
        bg: "#0F1115".to_owned(),
        bg_secondary: "#161B22".to_owned(),
        surface: "#1D232C".to_owned(),
        surface_elevated: "#252D38".to_owned(),
        text: "#F5F7FA".to_owned(),
        text_muted: "#B4BEC9".to_owned(),
        border: "#2F3947".to_owned(),
        border_strong: "#445264".to_owned(),
        accent: "#4ECDC4".to_owned(),
        accent_hover: "#68D8D1".to_owned(),
        accent_active: "#2FB6AE".to_owned(),
        accent_fg: "#081411".to_owned(),
        selection: "#1F5F66".to_owned(),
        link: "#7CC6FF".to_owned(),
        success: "#57CC99".to_owned(),
        warning: "#F4A261".to_owned(),
        error: "#E76F51".to_owned(),
    }
}
