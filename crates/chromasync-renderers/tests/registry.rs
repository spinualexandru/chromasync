use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use chromasync_renderers::{ArtifactGenerator, RendererError, RendererRegistry, TargetRegistry};
use chromasync_types::{GenerationContext, SemanticTokens, ThemeMode};

#[test]
fn target_registry_compiles_inherited_targets() {
    let dir = temp_dir_path("target-registry-inheritance");
    fs::create_dir_all(&dir).expect("temp target directory should be created");

    fs::write(
        dir.join("base.toml"),
        r#"
name = "base_preview"

[[artifacts]]
file_name = "base.conf"
template = "bg={{tokens.bg}}\n"

[[artifacts]]
file_name = "shared.conf"
template = "template={{ctx.template_name}}\n"
"#,
    )
    .expect("base target should be written");

    fs::write(
        dir.join("derived.toml"),
        r#"
name = "derived_preview"
extends = "base_preview"

[[artifacts]]
file_name = "shared.conf"
template = "accent={{tokens.accent}}\nmode={{ctx.mode}}\nseed={{ctx.seed}}\n"

[[artifacts]]
file_name = "extra.conf"
template = "output={{ctx.output_dir}}\n"
"#,
    )
    .expect("derived target should be written");

    let built_in = RendererRegistry::new();
    let registry = TargetRegistry::from_dir(&dir, false, &built_in.built_in_name_set())
        .expect("target registry should load");
    let target = registry
        .get("derived_preview")
        .expect("derived target should be compiled");
    let artifacts = target
        .generate(&sample_tokens(), &sample_context())
        .expect("compiled target should render");

    assert_eq!(
        artifacts
            .iter()
            .map(|artifact| artifact.file_name.as_str())
            .collect::<Vec<_>>(),
        vec!["base.conf", "shared.conf", "extra.conf"]
    );
    assert_eq!(artifacts[0].content, "bg=#0F1115\n");
    assert_eq!(
        artifacts[1].content,
        "accent=#4ECDC4\nmode=dark\nseed=#4ecdc4\n"
    );
    assert_eq!(
        artifacts[2].content,
        format!("output={}\n", sample_context().output_dir.display())
    );
    assert_eq!(registry.list_targets().len(), 2);
    assert_eq!(registry.list_targets()[0].source.label(), "filesystem");

    fs::remove_dir_all(dir).expect("temp target directory should be removed");
}

#[test]
fn invalid_placeholders_fail_target_loading() {
    let dir = temp_dir_path("target-registry-invalid");
    fs::create_dir_all(&dir).expect("temp target directory should be created");

    fs::write(
        dir.join("invalid.toml"),
        r#"
name = "broken_target"

[[artifacts]]
file_name = "broken.conf"
template = "{{tokens.nope}}"
"#,
    )
    .expect("invalid target should be written");

    let built_in = RendererRegistry::new();
    let error = TargetRegistry::from_dir(&dir, false, &built_in.built_in_name_set())
        .expect_err("invalid placeholder should fail target loading");

    assert!(matches!(error, RendererError::InvalidPlaceholder { .. }));

    fs::remove_dir_all(dir).expect("temp target directory should be removed");
}

#[test]
fn built_in_name_collisions_fail_target_loading() {
    let dir = temp_dir_path("target-registry-collision");
    fs::create_dir_all(&dir).expect("temp target directory should be created");

    fs::write(
        dir.join("collision.toml"),
        r#"
name = "kitty"

[[artifacts]]
file_name = "custom.css"
template = "{{tokens.bg}}"
"#,
    )
    .expect("colliding target should be written");

    let built_in = RendererRegistry::new();
    let error = TargetRegistry::from_dir(&dir, false, &built_in.built_in_name_set())
        .expect_err("built-in collisions should fail target loading");

    assert!(matches!(
        error,
        RendererError::TargetNameCollidesWithBuiltIn { name } if name == "kitty"
    ));

    fs::remove_dir_all(dir).expect("temp target directory should be removed");
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

fn sample_context() -> GenerationContext {
    GenerationContext {
        mode: ThemeMode::Dark,
        template_name: "minimal".to_owned(),
        output_dir: PathBuf::from("/tmp/chromasync-test-output"),
        seed: Some("#4ecdc4".to_owned()),
    }
}

fn temp_dir_path(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "chromasync-renderers-{label}-{}-{unique}",
        std::process::id()
    ))
}
