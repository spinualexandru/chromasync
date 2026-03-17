use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_main_subcommands() {
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.arg("--help");

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("generate"))
        .stdout(predicate::str::contains("wallpaper"))
        .stdout(predicate::str::contains("batch"))
        .stdout(predicate::str::contains("templates"))
        .stdout(predicate::str::contains("packs"))
        .stdout(predicate::str::contains("pack"))
        .stdout(predicate::str::contains("targets"))
        .stdout(predicate::str::contains("preview"))
        .stdout(predicate::str::contains("tokens"));
}

#[test]
fn templates_lists_built_in_templates() {
    let workspace = temp_dir_path("list-templates");
    let mut command = isolated_command(&workspace);

    command.arg("templates");

    command
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "minimal\tdark\tbuilt-in\tminimal-dark.toml",
        ))
        .stdout(predicate::str::contains(
            "minimal\tlight\tbuilt-in\tminimal-light.toml",
        ))
        .stdout(predicate::str::contains(
            "materialish\tlight\tbuilt-in\tmaterialish-light.toml",
        ));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn targets_lists_only_current_built_in_renderers() {
    let workspace = temp_dir_path("list-targets");
    let mut command = isolated_command(&workspace);

    command.arg("targets");

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("kitty"))
        .stdout(predicate::str::contains("alacritty"))
        .stdout(predicate::str::contains("gtk").not())
        .stdout(predicate::str::contains("hyprland").not())
        .stdout(predicate::str::contains("css").not())
        .stdout(predicate::str::contains("foot").not())
        .stdout(predicate::str::contains("ghostty").not())
        .stdout(predicate::str::contains("waybar").not())
        .stdout(predicate::str::contains("editor").not())
        .stdout(predicate::str::contains("rofi").not());

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn generate_writes_requested_artifacts() {
    let output_dir = temp_dir_path("generate-output");
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args(["generate", "--seed", "#ff6b6b", "--template", "brutalist"]);
    command.arg("--targets").arg(example_and_builtin_targets(&[
        "gtk.toml",
        "hyprland.toml",
        "kitty",
        "css.toml",
    ]));
    command.args([
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    for file_name in ["gtk.css", "hyprland.conf", "kitty.conf", "theme.css"] {
        let path = output_dir.join(file_name);
        assert!(
            stdout.contains(path.to_str().expect("output path should be utf-8")),
            "expected generate output to mention '{}', got:\n{stdout}",
            path.display()
        );
    }

    for file_name in ["gtk.css", "hyprland.conf", "kitty.conf", "theme.css"] {
        let path = output_dir.join(file_name);
        let metadata = fs::metadata(&path).expect("artifact should exist");
        assert!(
            metadata.is_file(),
            "expected '{}' to be a file",
            path.display()
        );
        assert!(
            metadata.len() > 0,
            "expected '{}' to be non-empty",
            path.display()
        );
    }

    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn wallpaper_writes_requested_artifacts() {
    let output_dir = temp_dir_path("wallpaper-output");
    let wallpaper = wallpaper_fixture("wallpaper-blocks.png");
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "wallpaper",
        "--image",
        wallpaper.to_str().expect("wallpaper path should be utf-8"),
        "--template",
        "brutalist",
    ]);
    command.arg("--targets").arg(example_and_builtin_targets(&[
        "gtk.toml",
        "hyprland.toml",
        "kitty",
        "css.toml",
    ]));
    command.args([
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    for file_name in ["gtk.css", "hyprland.conf", "kitty.conf", "theme.css"] {
        let path = output_dir.join(file_name);
        assert!(
            stdout.contains(path.to_str().expect("output path should be utf-8")),
            "expected wallpaper output to mention '{}', got:\n{stdout}",
            path.display()
        );
    }

    for file_name in ["gtk.css", "hyprland.conf", "kitty.conf", "theme.css"] {
        let path = output_dir.join(file_name);
        let metadata = fs::metadata(&path).expect("artifact should exist");
        assert!(
            metadata.is_file(),
            "expected '{}' to be a file",
            path.display()
        );
        assert!(
            metadata.len() > 0,
            "expected '{}' to be non-empty",
            path.display()
        );
    }

    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn generate_writes_phase_six_artifacts() {
    let output_dir = temp_dir_path("generate-phase-six-output");
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args(["generate", "--seed", "#4ecdc4", "--template", "terminal"]);
    command.arg("--targets").arg(example_and_builtin_targets(&[
        "alacritty",
        "foot.toml",
        "waybar.toml",
        "editor.toml",
    ]));
    command.args([
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    for file_name in ["alacritty.toml", "foot.ini", "style.css", "theme.json"] {
        let path = output_dir.join(file_name);
        assert!(
            stdout.contains(path.to_str().expect("output path should be utf-8")),
            "expected generate output to mention '{}', got:\n{stdout}",
            path.display()
        );
    }

    for file_name in ["alacritty.toml", "foot.ini", "style.css", "theme.json"] {
        let path = output_dir.join(file_name);
        let metadata = fs::metadata(&path).expect("artifact should exist");
        assert!(
            metadata.is_file(),
            "expected '{}' to be a file",
            path.display()
        );
        assert!(
            metadata.len() > 0,
            "expected '{}' to be non-empty",
            path.display()
        );
    }

    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn generate_writes_light_mode_editor_theme() {
    let output_dir = temp_dir_path("generate-light-editor-output");
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "materialish",
        "--mode",
        "light",
    ]);
    command
        .arg("--targets")
        .arg(example_and_builtin_targets(&["editor.toml"]));
    command.args([
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let theme_path = output_dir.join("theme.json");

    assert!(
        stdout.contains(theme_path.to_str().expect("theme path should be utf-8")),
        "expected generate output to mention '{}', got:\n{stdout}",
        theme_path.display()
    );

    let content = fs::read_to_string(&theme_path).expect("editor theme should be readable");
    assert!(content.contains("\"name\": \"Chromasync light\""));
    assert!(content.contains("\"type\": \"light\""));

    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn generate_writes_ghostty_example_target() {
    let output_dir = temp_dir_path("generate-ghostty-output");
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args(["generate", "--seed", "#4ecdc4", "--template", "terminal"]);
    command
        .arg("--targets")
        .arg(example_and_builtin_targets(&["ghostty.toml"]));
    command.args([
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let theme_path = output_dir.join("colors.txt");

    assert!(
        stdout.contains(theme_path.to_str().expect("theme path should be utf-8")),
        "expected generate output to mention '{}', got:\n{stdout}",
        theme_path.display()
    );

    let content = fs::read_to_string(&theme_path).expect("ghostty theme should be readable");
    assert!(content.contains("background = #"));
    assert!(content.contains("cursor-color = #"));
    assert!(content.contains("palette = 15=#"));

    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn wallpaper_writes_phase_six_artifacts() {
    let output_dir = temp_dir_path("wallpaper-phase-six-output");
    let wallpaper = wallpaper_fixture("wallpaper-blocks.png");
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "wallpaper",
        "--image",
        wallpaper.to_str().expect("wallpaper path should be utf-8"),
        "--template",
        "terminal",
    ]);
    command.arg("--targets").arg(example_and_builtin_targets(&[
        "alacritty",
        "foot.toml",
        "waybar.toml",
        "editor.toml",
    ]));
    command.args([
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    for file_name in ["alacritty.toml", "foot.ini", "style.css", "theme.json"] {
        let path = output_dir.join(file_name);
        assert!(
            stdout.contains(path.to_str().expect("output path should be utf-8")),
            "expected wallpaper output to mention '{}', got:\n{stdout}",
            path.display()
        );
    }

    for file_name in ["alacritty.toml", "foot.ini", "style.css", "theme.json"] {
        let path = output_dir.join(file_name);
        let metadata = fs::metadata(&path).expect("artifact should exist");
        assert!(
            metadata.is_file(),
            "expected '{}' to be a file",
            path.display()
        );
        assert!(
            metadata.len() > 0,
            "expected '{}' to be non-empty",
            path.display()
        );
    }

    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn tokens_exports_json_for_built_in_templates() {
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "tokens",
        "--seed",
        "#7c3aed",
        "--template",
        "minimal",
        "--format",
        "json",
    ]);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("\"bg\""))
        .stdout(predicate::str::contains("\"accent\""))
        .stdout(predicate::str::contains("\"accent_fg\""));
}

#[test]
fn preview_displays_palette_families_and_semantic_tokens() {
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args(["preview", "--seed", "#ff6b6b", "--template", "brutalist"]);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("Palette Families"))
        .stdout(predicate::str::contains("Semantic Tokens"))
        .stdout(predicate::str::contains("Contrast: relative-luminance"))
        .stdout(predicate::str::contains("primary"))
        .stdout(predicate::str::contains("accent"));
}

#[test]
fn preview_accepts_experimental_apca_contrast_mode() {
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "preview",
        "--seed",
        "#ff6b6b",
        "--template",
        "brutalist",
        "--contrast",
        "apca-experimental",
    ]);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("Contrast: apca-experimental"));
}

#[test]
fn tokens_accepts_template_paths() {
    let path = temp_file_path("cli-template");
    fs::write(&path, include_str!("../../../templates/minimal-dark.toml"))
        .expect("temp template should be written");

    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "tokens",
        "--seed",
        "#4ecdc4",
        "--template",
        path.to_str().expect("temp path should be utf-8"),
        "--format",
        "json",
    ]);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("\"bg\""))
        .stdout(predicate::str::contains("\"success\""));

    fs::remove_file(path).expect("temp template should be removed");
}

#[test]
fn generate_refuses_to_overwrite_existing_artifacts() {
    let output_dir = temp_dir_path("generate-overwrite");
    fs::create_dir_all(&output_dir).expect("output directory should be created");
    fs::write(output_dir.join("theme.css"), "existing")
        .expect("existing artifact should be written");

    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        example_target_path("css.toml")
            .to_str()
            .expect("example target path should be utf-8"),
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    command.assert().failure().stderr(predicate::str::contains(
        "refusing to overwrite existing artifact",
    ));

    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn generate_reports_invalid_seed_errors() {
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "generate",
        "--seed",
        "nope",
        "--template",
        "minimal",
        "--targets",
        "kitty",
    ]);

    command.assert().failure().stderr(predicate::str::contains(
        "seed color 'nope' must use the #RRGGBB format",
    ));
}

#[test]
fn generate_reports_missing_template_errors() {
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "missing",
        "--targets",
        "kitty",
    ]);

    command
        .assert()
        .failure()
        .stderr(predicate::str::contains("template 'missing' was not found"));
}

#[test]
fn generate_rejects_non_mvp_targets_at_cli_boundary() {
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        "rofi",
    ]);

    command
        .assert()
        .failure()
        .stderr(predicate::str::contains("target 'rofi' was not found"));
}

#[test]
fn generate_reports_output_directory_creation_errors() {
    let output_path = temp_file_path("generate-output-dir");
    fs::write(&output_path, "blocking file").expect("blocking file should be written");

    let mut command = Command::cargo_bin("chromasync").expect("binary should build");

    command.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        "kitty",
        "--output",
        output_path.to_str().expect("output path should be utf-8"),
    ]);

    command.assert().failure().stderr(predicate::str::contains(
        "failed to create output directory",
    ));

    fs::remove_file(output_path).expect("blocking file should be removed");
}

#[test]
fn batch_runs_seed_and_wallpaper_jobs_from_relative_manifest_paths() {
    let batch_dir = temp_dir_path("batch-manifest");
    fs::create_dir_all(&batch_dir).expect("batch directory should be created");

    let wallpaper_src = wallpaper_fixture("wallpaper-blocks.png");
    let wallpaper_dest = batch_dir.join("wallpaper.png");
    fs::copy(&wallpaper_src, &wallpaper_dest).expect("wallpaper fixture should copy");
    let targets_dir = batch_dir.join("targets");
    fs::create_dir_all(&targets_dir).expect("batch targets directory should be created");
    fs::copy(
        example_target_path("css.toml"),
        targets_dir.join("css.toml"),
    )
    .expect("css example target should copy");
    fs::copy(
        example_target_path("editor.toml"),
        targets_dir.join("editor.toml"),
    )
    .expect("editor example target should copy");
    fs::copy(
        example_target_path("waybar.toml"),
        targets_dir.join("waybar.toml"),
    )
    .expect("waybar example target should copy");
    fs::copy(
        example_target_path("foot.toml"),
        targets_dir.join("foot.toml"),
    )
    .expect("foot example target should copy");

    let batch_file = batch_dir.join("jobs.toml");
    fs::write(
        &batch_file,
        r##"
[[jobs]]
name = "seed-job"
seed = "#4ecdc4"
template = "minimal"
mode = "dark"
contrast = "relative-luminance"
targets = ["targets/css.toml", "targets/editor.toml"]
output = "seed-output"

[[jobs]]
name = "wallpaper-job"
image = "wallpaper.png"
template = "terminal"
mode = "dark"
contrast = "apca-experimental"
targets = ["targets/waybar.toml", "targets/foot.toml"]
output = "wallpaper-output"
"##,
    )
    .expect("batch manifest should be written");

    let mut command = Command::cargo_bin("chromasync").expect("binary should build");
    command.args([
        "batch",
        "--file",
        batch_file
            .to_str()
            .expect("batch file path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    for relative in [
        "seed-output/theme.css",
        "seed-output/theme.json",
        "wallpaper-output/style.css",
        "wallpaper-output/foot.ini",
    ] {
        let path = batch_dir.join(relative);
        assert!(
            stdout.contains(path.to_str().expect("output path should be utf-8")),
            "expected batch output to mention '{}', got:\n{stdout}",
            path.display()
        );
        let metadata = fs::metadata(&path).expect("artifact should exist");
        assert!(
            metadata.is_file(),
            "expected '{}' to be a file",
            path.display()
        );
        assert!(
            metadata.len() > 0,
            "expected '{}' to be non-empty",
            path.display()
        );
    }

    fs::remove_dir_all(batch_dir).expect("batch directory should be removed");
}

#[test]
fn generate_accepts_target_toml_paths() {
    let output_dir = temp_dir_path("generate-custom-target-output");
    let target_path = temp_file_path("custom-target");
    fs::write(
        &target_path,
        r#"
name = "custom_preview"

[[artifacts]]
file_name = "custom-preview.conf"
template = """
accent={{tokens.accent}}
mode={{ctx.mode}}
template={{ctx.template_name}}
seed={{ctx.seed}}
"""
"#,
    )
    .expect("custom target should be written");

    let mut command = Command::cargo_bin("chromasync").expect("binary should build");
    command.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        target_path
            .to_str()
            .expect("target path should be valid utf-8"),
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let artifact_path = output_dir.join("custom-preview.conf");
    assert!(
        stdout.contains(
            artifact_path
                .to_str()
                .expect("artifact path should be utf-8")
        ),
        "expected custom target output to mention '{}', got:\n{stdout}",
        artifact_path.display()
    );

    let content = fs::read_to_string(&artifact_path).expect("custom artifact should be readable");
    assert!(content.contains("accent=#"));
    assert!(content.contains("mode=dark"));
    assert!(content.contains("template=minimal"));
    assert!(content.contains("seed=#4ecdc4"));

    fs::remove_file(target_path).expect("custom target should be removed");
    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn packs_lists_discovered_local_packs() {
    let workspace = temp_dir_path("packs-list");
    let pack_dir = write_pack_fixture(&workspace, "aurora", "aurora", "aurora_preview");
    let mut command = isolated_command(&workspace);

    command.arg("packs");

    command
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "aurora\t1.2.3\t{}",
            pack_dir.display()
        )));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn pack_info_lists_templates_and_targets_from_pack() {
    let workspace = temp_dir_path("pack-info");
    let pack_dir = write_pack_fixture(&workspace, "aurora", "aurora", "aurora_preview");
    let mut command = isolated_command(&workspace);

    command.args(["pack", "info", "aurora"]);

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("name\taurora"))
        .stdout(predicate::str::contains("version\t1.2.3"))
        .stdout(predicate::str::contains(format!(
            "root\t{}",
            pack_dir.display()
        )))
        .stdout(predicate::str::contains("templates"))
        .stdout(predicate::str::contains("aurora\tdark"))
        .stdout(predicate::str::contains("targets"))
        .stdout(predicate::str::contains("aurora_preview"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn templates_and_targets_include_pack_assets() {
    let workspace = temp_dir_path("pack-asset-listing");
    let pack_dir = write_pack_fixture(&workspace, "aurora", "aurora", "aurora_preview");
    let template_path = pack_dir.join("templates/aurora-dark.toml");
    let target_path = pack_dir.join("targets/aurora_preview.toml");

    let mut templates = isolated_command(&workspace);
    templates.arg("templates");
    templates
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "aurora\tdark\tpack\t{}",
            template_path.display()
        )));

    let mut targets = isolated_command(&workspace);
    targets.arg("targets");
    targets
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "aurora_preview\tpack\t{}",
            target_path.display()
        )));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn generate_uses_pack_templates_and_targets() {
    let workspace = temp_dir_path("generate-pack-assets");
    write_pack_fixture(&workspace, "aurora", "aurora", "aurora_preview");
    let output_dir = workspace.join("output");
    let mut command = isolated_command(&workspace);

    command.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "aurora",
        "--targets",
        "aurora_preview",
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let artifact_path = output_dir.join("aurora-preview.conf");

    assert!(
        stdout.contains(
            artifact_path
                .to_str()
                .expect("artifact path should be valid utf-8")
        ),
        "expected pack-backed generation to mention '{}', got:\n{stdout}",
        artifact_path.display()
    );

    let content = fs::read_to_string(&artifact_path).expect("pack artifact should be readable");
    assert!(content.contains("pack=aurora"));
    assert!(content.contains("template=aurora"));
    assert!(content.contains("accent=#"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn invalid_pack_target_collisions_fail_with_clear_errors() {
    let workspace = temp_dir_path("pack-target-collision");
    write_pack_fixture(&workspace, "broken", "broken", "kitty");
    let mut command = isolated_command(&workspace);

    command.arg("targets");

    command.assert().failure().stderr(predicate::str::contains(
        "user target 'kitty' collides with a built-in renderer name",
    ));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

fn isolated_command(working_dir: &Path) -> Command {
    let mut command = Command::cargo_bin("chromasync").expect("binary should build");
    let xdg_config = working_dir.join("xdg-config");
    let xdg_data = working_dir.join("xdg-data");

    fs::create_dir_all(&xdg_config).expect("isolated XDG config directory should be created");
    fs::create_dir_all(&xdg_data).expect("isolated XDG data directory should be created");

    command.current_dir(working_dir);
    command.env("XDG_CONFIG_HOME", &xdg_config);
    command.env("XDG_DATA_HOME", &xdg_data);

    command
}

fn write_pack_fixture(
    workspace: &Path,
    pack_name: &str,
    template_name: &str,
    target_name: &str,
) -> PathBuf {
    let pack_dir = workspace.join(".chromasync").join("packs").join(pack_name);
    let templates_dir = pack_dir.join("templates");
    let targets_dir = pack_dir.join("targets");

    fs::create_dir_all(&templates_dir).expect("pack templates directory should be created");
    fs::create_dir_all(&targets_dir).expect("pack targets directory should be created");

    fs::write(
        pack_dir.join("pack.toml"),
        format!(
            r#"
name = "{pack_name}"
version = "1.2.3"
description = "Fixture pack"
author = "Chromasync Tests"
license = "MIT"
homepage = "https://example.com/{pack_name}"

[templates]
paths = ["templates"]

[targets]
paths = ["targets"]
"#
        ),
    )
    .expect("pack manifest should be written");

    let template = include_str!("../../../templates/minimal-dark.toml").replacen(
        r#"name = "minimal""#,
        &format!(r#"name = "{template_name}""#),
        1,
    );
    fs::write(
        pack_dir
            .join("templates")
            .join(format!("{template_name}-dark.toml")),
        template,
    )
    .expect("pack template should be written");

    fs::write(
        pack_dir.join("targets").join(format!("{target_name}.toml")),
        format!(
            r#"
name = "{target_name}"

[[artifacts]]
file_name = "aurora-preview.conf"
template = """
pack={pack_name}
template={{{{ctx.template_name}}}}
accent={{{{tokens.accent}}}}
"""
"#
        ),
    )
    .expect("pack target should be written");

    pack_dir
}

fn example_and_builtin_targets(entries: &[&str]) -> String {
    entries
        .iter()
        .map(|entry| {
            if entry.ends_with(".toml") {
                example_target_path(entry)
                    .to_str()
                    .expect("example target path should be utf-8")
                    .to_owned()
            } else {
                (*entry).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn example_target_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/targets")
        .join(name)
}

fn temp_file_path(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "chromasync-cli-{label}-{}-{unique}.toml",
        std::process::id()
    ))
}

fn temp_dir_path(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be valid")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "chromasync-cli-{label}-{}-{unique}",
        std::process::id()
    ))
}

fn wallpaper_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../chromasync-extract/tests/fixtures")
        .join(name)
}
