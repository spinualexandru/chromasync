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
fn hyprland_built_in_target_matches_golden_file() {
    assert_matches_golden(
        RenderTarget::Hyprland,
        include_str!("fixtures/hyprland.conf.golden"),
    );
}

#[test]
fn kitty_built_in_target_matches_golden_file() {
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
fn ghostty_built_in_target_matches_golden_file() {
    assert_matches_golden(
        RenderTarget::Ghostty,
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

    assert_eq!(artifacts.len(), 19);
    assert_eq!(
        artifacts
            .iter()
            .map(|artifact| artifact.target.clone())
            .collect::<Vec<_>>(),
        vec![
            "alacritty".to_owned(),
            "chromium".to_owned(),
            "ghostty".to_owned(),
            "google-chrome".to_owned(),
            "gtk3".to_owned(),
            "gtk4".to_owned(),
            "helium-browser".to_owned(),
            "hyprland".to_owned(),
            "hyprland-lua".to_owned(),
            "kcolorscheme".to_owned(),
            "kitty".to_owned(),
            "micro".to_owned(),
            "qt5".to_owned(),
            "qt6".to_owned(),
            "vscode".to_owned(),
            "vscode".to_owned(),
            "vscode-insiders".to_owned(),
            "vscode-insiders".to_owned(),
            "zed".to_owned(),
        ]
    );
    assert_eq!(
        artifacts
            .iter()
            .map(|artifact| artifact.file_name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "alacritty.toml",
            "manifest.json",
            "chromasync.ghostty",
            "manifest.json",
            "gtk.css",
            "gtk.css",
            "manifest.json",
            "hyprland.conf",
            "hypr-chromasync.lua",
            "chromasync.colors",
            "kitty.conf",
            "chromasync.micro",
            "chromasync.conf",
            "chromasync.conf",
            "package.json",
            "chromasync-color-theme.json",
            "package.json",
            "chromasync-color-theme.json",
            "chromasync.json",
        ]
    );
}

#[test]
fn browser_and_vscode_built_ins_render_valid_json() {
    let registry = OutputRegistry::default();
    let targets = [
        "chromium".to_owned(),
        "google-chrome".to_owned(),
        "helium-browser".to_owned(),
        "vscode".to_owned(),
        "vscode-insiders".to_owned(),
    ];
    let artifacts = registry
        .generate(&targets, &sample_tokens(), &sample_context())
        .expect("JSON targets should render");

    for artifact in &artifacts {
        serde_json::from_str::<serde_json::Value>(&artifact.content).unwrap_or_else(|error| {
            panic!(
                "{} artifact {} should be valid JSON: {error}",
                artifact.target, artifact.file_name
            )
        });
    }

    let chromium = artifacts
        .iter()
        .find(|artifact| artifact.target == "chromium")
        .expect("Chromium manifest should be generated");
    assert!(chromium.content.contains("\"frame\": [22, 27, 34]"));

    let vscode_manifest = artifacts
        .iter()
        .find(|artifact| artifact.target == "vscode" && artifact.file_name == "package.json")
        .expect("VS Code package manifest should be generated");
    assert!(vscode_manifest.content.contains("\"uiTheme\": \"vs-dark\""));
}

#[test]
fn vscode_light_mode_uses_light_ui_theme_identifier() {
    let registry = OutputRegistry::default();
    let mut context = sample_context();
    context.mode = ThemeMode::Light;
    let artifacts = registry
        .generate(&["vscode".to_owned()], &sample_tokens(), &context)
        .expect("VS Code target should render");
    let package = artifacts
        .iter()
        .find(|artifact| artifact.file_name == "package.json")
        .expect("VS Code package manifest should be generated");

    assert!(package.content.contains("\"uiTheme\": \"vs\""));
}

#[test]
fn single_artifact_api_rejects_multi_artifact_targets() {
    for target in [RenderTarget::VsCode, RenderTarget::VsCodeInsiders] {
        let error = render_target(target, &sample_tokens())
            .expect_err("single-artifact API should reject VS Code extension targets");

        assert!(matches!(
            error,
            chromasync_renderers::RendererError::MultiArtifactTarget {
                target: error_target,
                count: 2,
            } if error_target == target
        ));
    }
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
