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
        .stdout(predicate::str::contains("sync"))
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
fn targets_lists_built_in_renderers() {
    let workspace = temp_dir_path("list-targets");
    let mut command = isolated_command(&workspace);

    command.arg("targets");

    command
        .assert()
        .success()
        .stdout(predicate::str::contains("alacritty\tbuilt-in\talacritty"))
        .stdout(predicate::str::contains("chromium\tbuilt-in\tchromium"))
        .stdout(predicate::str::contains("ghostty\tbuilt-in\tghostty"))
        .stdout(predicate::str::contains(
            "google-chrome\tbuilt-in\tgoogle-chrome",
        ))
        .stdout(predicate::str::contains("gtk3\tbuilt-in\tgtk3"))
        .stdout(predicate::str::contains("gtk4\tbuilt-in\tgtk4"))
        .stdout(predicate::str::contains(
            "helium-browser\tbuilt-in\thelium-browser",
        ))
        .stdout(predicate::str::contains("hyprland\tbuilt-in\thyprland"))
        .stdout(predicate::str::contains(
            "hyprland-lua\tbuilt-in\thyprland-lua",
        ))
        .stdout(predicate::str::contains("kitty\tbuilt-in\tkitty"))
        .stdout(predicate::str::contains(
            "kcolorscheme\tbuilt-in\tkcolorscheme",
        ))
        .stdout(predicate::str::contains("micro\tbuilt-in\tmicro"))
        .stdout(predicate::str::contains("qt5\tbuilt-in\tqt5"))
        .stdout(predicate::str::contains("qt6\tbuilt-in\tqt6"))
        .stdout(predicate::str::contains("vscode\tbuilt-in\tvscode"))
        .stdout(predicate::str::contains(
            "vscode-insiders\tbuilt-in\tvscode-insiders",
        ))
        .stdout(predicate::str::contains("zed\tbuilt-in\tzed"))
        .stdout(predicate::str::contains("css").not())
        .stdout(predicate::str::contains("foot").not())
        .stdout(predicate::str::contains("waybar").not())
        .stdout(predicate::str::contains("editor").not())
        .stdout(predicate::str::contains("rofi").not());

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn generate_writes_requested_artifacts() {
    let output_dir = temp_dir_path("generate-output");
    let mut command = isolated_command(&output_dir);

    command.args(["generate", "--seed", "#ff6b6b", "--template", "brutalist"]);
    command.arg("--targets").arg(example_and_builtin_targets(&[
        "gtk.toml", "hyprland", "kitty", "css.toml",
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
    let mut command = isolated_command(&output_dir);

    command.args([
        "wallpaper",
        "--image",
        wallpaper.to_str().expect("wallpaper path should be utf-8"),
        "--template",
        "brutalist",
    ]);
    command.arg("--targets").arg(example_and_builtin_targets(&[
        "gtk.toml", "hyprland", "kitty", "css.toml",
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
    let mut command = isolated_command(&output_dir);

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
    let mut command = isolated_command(&output_dir);

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
fn generate_writes_ghostty_built_in_target() {
    let workspace = temp_dir_path("generate-ghostty-output");
    let output_dir = workspace.join("output");
    let mut command = isolated_command(&workspace);

    command.args(["generate", "--seed", "#4ecdc4", "--template", "terminal"]);
    command.arg("--targets").arg("ghostty");
    command.args([
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let theme_path = output_dir.join("chromasync.ghostty");

    assert!(
        stdout.contains(theme_path.to_str().expect("theme path should be utf-8")),
        "expected generate output to mention '{}', got:\n{stdout}",
        theme_path.display()
    );

    let content = fs::read_to_string(&theme_path).expect("ghostty theme should be readable");
    assert!(content.contains("background = #"));
    assert!(content.contains("cursor-color = #"));
    assert!(content.contains("palette = 15=#"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn generate_uses_zed_built_in_preferred_template() {
    let workspace = temp_dir_path("generate-zed-preferred-template");
    let output_dir = workspace.join("output");

    let mut command = isolated_command(&workspace);
    command.args(["generate", "--seed", "#4ecdc4", "--targets", "zed"]);
    command.args([
        "--output",
        output_dir.to_str().expect("output path should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let theme_path = output_dir.join("chromasync.json");

    assert!(
        stdout.contains(theme_path.to_str().expect("theme path should be utf-8")),
        "expected generate output to mention '{}', got:\n{stdout}",
        theme_path.display()
    );
    let content = fs::read_to_string(&theme_path).expect("zed theme should be readable");
    assert!(content.contains("\"name\": \"Chromasync\""));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_default_profile_writes_built_in_and_user_targets() {
    let workspace = temp_dir_path("sync-default-profile");
    let config_root = workspace.join("xdg-config").join("chromasync");
    let ghostty_out = workspace.join("ghostty-out");
    let kitty_out = workspace.join("kitty-out");

    fs::create_dir_all(&config_root).expect("config root should be created");
    fs::write(
        config_root.join("config.toml"),
        format!(
            r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
template = "terminal"
mode = "dark"
chroma = "industrial"
targets = ["ghostty", "kitty"]

[[targets]]
name = "ghostty"
output_dir = "{}"
overwrite = false

[[targets]]
name = "kitty"
output_dir = "{}"
overwrite = false
"##,
            ghostty_out.display(),
            kitty_out.display(),
        ),
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.arg("sync");

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let ghostty_theme = ghostty_out.join("chromasync.ghostty");
    let kitty_theme = kitty_out.join("kitty.conf");

    for path in [&ghostty_theme, &kitty_theme] {
        assert!(
            stdout.contains(path.to_str().expect("output path should be utf-8")),
            "expected sync output to mention '{}', got:\n{stdout}",
            path.display()
        );
        let metadata = fs::metadata(path).expect("sync artifact should exist");
        assert!(metadata.is_file());
        assert!(metadata.len() > 0);
    }

    let ghostty_content =
        fs::read_to_string(&ghostty_theme).expect("ghostty theme should be readable");
    assert!(ghostty_content.contains("background = #"));
    let kitty_content = fs::read_to_string(&kitty_theme).expect("kitty theme should be readable");
    assert!(kitty_content.contains("background #"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_named_profile_selects_requested_config() {
    let workspace = temp_dir_path("sync-named-profile");
    let config_root = workspace.join("xdg-config").join("chromasync");
    let targets_dir = config_root.join("targets");
    let output_dir = workspace.join("sync-out");

    fs::create_dir_all(&targets_dir).expect("user targets directory should be created");
    fs::write(
        targets_dir.join("sync_probe.toml"),
        r#"
name = "sync_probe"

[[artifacts]]
file_name = "mode.txt"
template = """
mode={{ctx.mode}}
seed={{ctx.seed}}
"""
"#,
    )
    .expect("sync probe target should be written");
    fs::write(
        config_root.join("config.toml"),
        format!(
            r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
template = "minimal"
mode = "dark"
targets = ["sync_probe"]

[[configs]]
name = "personalGreen"
seed = "#00ff00"
template = "minimal"
mode = "light"
targets = ["sync_probe"]

[[targets]]
name = "sync_probe"
output_dir = "{}"
source = "targets/sync_probe.toml"
overwrite = true
"##,
            output_dir.display(),
        ),
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.args(["sync", "personalGreen"]);

    let assert = command.assert().success();
    let artifact = output_dir.join("mode.txt");
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains(artifact.to_str().expect("output path should be utf-8")),
        "expected sync output to mention '{}', got:\n{stdout}",
        artifact.display()
    );

    let content = fs::read_to_string(&artifact).expect("sync probe should be readable");
    assert!(content.contains("mode=light"));
    assert!(content.contains("seed=#00ff00"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_mode_option_overrides_profile_mode() {
    let workspace = temp_dir_path("sync-mode-override");
    let config_root = workspace.join("xdg-config").join("chromasync");
    let targets_dir = config_root.join("targets");
    let output_dir = workspace.join("sync-out");

    fs::create_dir_all(&targets_dir).expect("user targets directory should be created");
    fs::write(
        targets_dir.join("sync_probe.toml"),
        r#"
name = "sync_probe"

[[artifacts]]
file_name = "mode.txt"
template = "mode={{ctx.mode}}"
"#,
    )
    .expect("sync probe target should be written");
    fs::write(
        config_root.join("config.toml"),
        format!(
            r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
template = "minimal"
mode = "dark"
targets = ["sync_probe"]

[[targets]]
name = "sync_probe"
output_dir = "{}"
source = "targets/sync_probe.toml"
overwrite = true
"##,
            output_dir.display(),
        ),
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.args(["sync", "--mode", "light"]);
    command.assert().success();

    let content =
        fs::read_to_string(output_dir.join("mode.txt")).expect("sync probe should be readable");
    assert!(content.contains("mode=light"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_profile_fetches_wallpaper_image_from_command() {
    let workspace = temp_dir_path("sync-image-fetch-command");
    let config_root = workspace.join("xdg-config").join("chromasync");
    let output_dir = workspace.join("kitty-out");
    let wallpaper = wallpaper_fixture("wallpaper-blocks.png");

    fs::create_dir_all(&config_root).expect("config root should be created");
    fs::write(
        config_root.join("config.toml"),
        format!(
            r#"
[[configs]]
name = "default"
image_fetch_command = "printf '%s\n' '{}'"
template = "terminal"
mode = "dark"
targets = ["kitty"]

[[targets]]
name = "kitty"
output_dir = "{}"
overwrite = true
"#,
            wallpaper.display(),
            output_dir.display(),
        ),
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.arg("sync");

    let assert = command.assert().success();
    let artifact = output_dir.join("kitty.conf");
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains(artifact.to_str().expect("output path should be utf-8")),
        "expected sync output to mention '{}', got:\n{stdout}",
        artifact.display()
    );

    let content = fs::read_to_string(&artifact).expect("kitty theme should be readable");
    assert!(content.contains("background #"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_runs_matching_hooks_after_artifacts_are_written() {
    let workspace = temp_dir_path("sync-hooks-run");
    let config_root = workspace.join("xdg-config").join("chromasync");
    let output_dir = workspace.join("sync-out");

    fs::create_dir_all(&config_root).expect("config root should be created");
    fs::write(
        config_root.join("config.toml"),
        format!(
            r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
template = "terminal"
mode = "dark"
targets = ["hyprland-lua", "kitty"]
output_dir = "{}"

[[hooks]]
name = "all-targets"
on = "targets:done"
command = "printf all > all-hook.txt"

[[hooks]]
name = "hyprland-lua-target"
on = ["target:hyprland-lua:done"]
command = "printf target > target-hook.txt"

[[hooks]]
name = "missing-target"
on = "target:ghostty:done"
command = "printf missing > missing-hook.txt"
"##,
            output_dir.display(),
        ),
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.arg("sync");

    command.assert().success();

    assert!(
        output_dir.join("hypr-chromasync.lua").is_file(),
        "sync should write the hyprland-lua artifact before hooks run"
    );
    assert_eq!(
        fs::read_to_string(config_root.join("all-hook.txt")).expect("all hook should run"),
        "all"
    );
    assert_eq!(
        fs::read_to_string(config_root.join("target-hook.txt")).expect("target hook should run"),
        "target"
    );
    assert!(
        !config_root.join("missing-hook.txt").exists(),
        "hook for a target that was not generated should not run"
    );

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_hook_filters_match_selected_profile() {
    let workspace = temp_dir_path("sync-hook-filters");
    let config_root = workspace.join("xdg-config").join("chromasync");
    let default_output = workspace.join("default-out");
    let personal_output = workspace.join("personal-out");

    fs::create_dir_all(&config_root).expect("config root should be created");
    fs::write(
        config_root.join("config.toml"),
        format!(
            r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
template = "terminal"
mode = "dark"
targets = ["kitty"]
output_dir = "{}"

[[configs]]
name = "personalGreen"
seed = "#00ff00"
template = "terminal"
mode = "dark"
targets = ["kitty"]
output_dir = "{}"

[[hooks]]
name = "default-only"
filters = ["config:default"]
on = "targets:done"
command = "printf default > default-hook.txt"

[[hooks]]
name = "personal-only"
filters = ["config:personalGreen"]
on = "targets:done"
command = "printf personal > personal-hook.txt"
"##,
            default_output.display(),
            personal_output.display(),
        ),
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.args(["sync", "personalGreen"]);

    command.assert().success();

    assert!(
        personal_output.join("kitty.conf").is_file(),
        "selected sync profile should write its artifact"
    );
    assert!(
        !config_root.join("default-hook.txt").exists(),
        "default profile hook should not run for personalGreen"
    );
    assert_eq!(
        fs::read_to_string(config_root.join("personal-hook.txt"))
            .expect("personal profile hook should run"),
        "personal"
    );

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_fails_when_matching_hook_fails_after_writing_artifacts() {
    let workspace = temp_dir_path("sync-hook-failure");
    let config_root = workspace.join("xdg-config").join("chromasync");
    let output_dir = workspace.join("sync-out");

    fs::create_dir_all(&config_root).expect("config root should be created");
    fs::write(
        config_root.join("config.toml"),
        format!(
            r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
template = "terminal"
mode = "dark"
targets = ["kitty"]
output_dir = "{}"

[[hooks]]
name = "reload"
on = "targets:done"
command = "printf hook-failed >&2; exit 7"
"##,
            output_dir.display(),
        ),
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.arg("sync");

    command
        .assert()
        .failure()
        .stderr(predicate::str::contains("hook 'reload' exited with"))
        .stderr(predicate::str::contains("hook-failed"));

    assert!(
        output_dir.join("kitty.conf").is_file(),
        "hook failure should not roll back written artifacts"
    );

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_reports_missing_profile() {
    let workspace = temp_dir_path("sync-missing-profile");
    let config_root = workspace.join("xdg-config").join("chromasync");

    fs::create_dir_all(&config_root).expect("config root should be created");
    fs::write(
        config_root.join("config.toml"),
        r##"
[[configs]]
name = "default"
seed = "#4ecdc4"
template = "minimal"
targets = ["kitty"]
"##,
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.args(["sync", "personalGreen"]);

    command.assert().failure().stderr(predicate::str::contains(
        "sync profile 'personalGreen' was not found",
    ));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn sync_requires_exactly_one_color_source() {
    let workspace = temp_dir_path("sync-missing-source");
    let config_root = workspace.join("xdg-config").join("chromasync");

    fs::create_dir_all(&config_root).expect("config root should be created");
    fs::write(
        config_root.join("config.toml"),
        r#"
[[configs]]
name = "default"
template = "minimal"
targets = ["kitty"]
"#,
    )
    .expect("sync config should be written");

    let mut command = isolated_command(&workspace);
    command.arg("sync");

    command.assert().failure().stderr(predicate::str::contains(
        "sync profile 'default' must define exactly one of 'seed', 'image', or 'image_fetch_command'",
    ));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn wallpaper_writes_phase_six_artifacts() {
    let output_dir = temp_dir_path("wallpaper-phase-six-output");
    let wallpaper = wallpaper_fixture("wallpaper-blocks.png");
    let mut command = isolated_command(&output_dir);

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

    let mut command = isolated_command(&output_dir);

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
fn generate_force_overwrites_existing_artifacts() {
    let output_dir = temp_dir_path("generate-force");
    fs::create_dir_all(&output_dir).expect("output directory should be created");
    fs::write(output_dir.join("theme.css"), "existing")
        .expect("existing artifact should be written");

    let mut command = isolated_command(&output_dir);

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
        "--force",
    ]);

    let assert = command.assert().success();
    let theme_path = output_dir.join("theme.css");

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains(theme_path.to_str().expect("theme path should be utf-8")),
        "expected --force output to mention '{}', got:\n{stdout}",
        theme_path.display()
    );

    let content = fs::read_to_string(&theme_path).expect("theme artifact should be readable");
    assert_ne!(
        content, "existing",
        "--force should replace the existing artifact with generated content"
    );
    assert!(content.contains("--chromasync-"));

    fs::remove_dir_all(output_dir).expect("output directory should be removed");
}

#[test]
fn generate_reports_invalid_seed_errors() {
    let workspace = temp_dir_path("generate-invalid-seed");
    let mut command = isolated_command(&workspace);

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

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn generate_reports_missing_template_errors() {
    let workspace = temp_dir_path("generate-missing-template");
    let mut command = isolated_command(&workspace);

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

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn generate_rejects_non_mvp_targets_at_cli_boundary() {
    let workspace = temp_dir_path("generate-unknown-target");
    let mut command = isolated_command(&workspace);

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

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn generate_reports_output_directory_creation_errors() {
    let workspace = temp_dir_path("generate-output-dir-workspace");
    let output_path = temp_file_path("generate-output-dir");
    fs::write(&output_path, "blocking file").expect("blocking file should be written");

    let mut command = isolated_command(&workspace);

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
    fs::remove_dir_all(workspace).expect("workspace should be removed");
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

    let mut command = isolated_command(&batch_dir);
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

    let mut command = isolated_command(&output_dir);
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

#[test]
fn target_install_copies_target_and_records_config() {
    let workspace = temp_dir_path("target-install");
    let outdir = workspace.join("install-out");
    let config_root = workspace.join("xdg-config").join("chromasync");
    let target_file = config_root.join("targets").join("gtk.toml");
    let config_file = config_root.join("config.toml");

    let mut command = isolated_command(&workspace);
    command.args([
        "target",
        "install",
        "--target",
        example_target_path("gtk.toml")
            .to_str()
            .expect("example target path should be utf-8"),
        "--outdir",
        outdir.to_str().expect("outdir should be utf-8"),
    ]);

    let assert = command.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains(target_file.to_str().expect("target path should be utf-8")),
        "expected install output to mention '{}', got:\n{stdout}",
        target_file.display()
    );
    assert!(
        stdout.contains(config_file.to_str().expect("config path should be utf-8")),
        "expected install output to mention '{}', got:\n{stdout}",
        config_file.display()
    );

    let target_content =
        fs::read_to_string(&target_file).expect("installed target file should exist");
    assert!(target_content.contains("name = \"gtk\""));

    let config_content = fs::read_to_string(&config_file).expect("config file should exist");
    assert!(config_content.contains("[[targets]]"));
    assert!(config_content.contains("name = \"gtk\""));
    assert!(config_content.contains(&outdir.display().to_string()));
    assert!(config_content.contains("source = \"targets/gtk.toml\""));
    assert!(config_content.contains("overwrite = false"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn generate_writes_to_installed_target_outdir() {
    let workspace = temp_dir_path("target-install-generate");
    let outdir = workspace.join("gtk-out");
    let artifact_path = outdir.join("gtk.css");

    let mut install = isolated_command(&workspace);
    install.args([
        "target",
        "install",
        "--target",
        example_target_path("gtk.toml")
            .to_str()
            .expect("example target path should be utf-8"),
        "--outdir",
        outdir.to_str().expect("outdir should be utf-8"),
    ]);
    install.assert().success();

    let mut generate = isolated_command(&workspace);
    generate.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        "gtk",
    ]);
    let assert = generate.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains(
            artifact_path
                .to_str()
                .expect("artifact path should be utf-8")
        ),
        "expected generate to write to installed outdir '{}', got:\n{stdout}",
        artifact_path.display()
    );

    let metadata = fs::metadata(&artifact_path).expect("installed artifact should exist");
    assert!(metadata.is_file());
    assert!(metadata.len() > 0);

    assert!(
        !workspace.join("chromasync").exists(),
        "fallback output dir should not be created when a target is installed"
    );

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn installed_target_context_output_dir_matches_resolved_outdir() {
    let workspace = temp_dir_path("target-install-context-output-dir");
    fs::create_dir_all(&workspace).expect("workspace should be created");
    let installed_outdir = workspace.join("installed-out");
    let artifact_path = installed_outdir.join("probe.txt");
    let target_path = workspace.join("outdir-probe.toml");
    fs::write(
        &target_path,
        r#"
name = "outdir-probe"

[[artifacts]]
file_name = "probe.txt"
template = "output={{ctx.output_dir}}"
"#,
    )
    .expect("target should be written");

    let mut install = isolated_command(&workspace);
    install.args([
        "target",
        "install",
        "--target",
        target_path.to_str().expect("target path should be utf-8"),
        "--outdir",
        installed_outdir
            .to_str()
            .expect("installed outdir should be utf-8"),
    ]);
    install.assert().success();

    let mut generate = isolated_command(&workspace);
    generate.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        "outdir-probe",
    ]);
    generate.assert().success();

    let content = fs::read_to_string(&artifact_path).expect("installed artifact should exist");
    assert_eq!(content, format!("output={}", installed_outdir.display()));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn explicit_path_target_uses_requested_output_even_when_same_name_is_installed() {
    let workspace = temp_dir_path("target-install-path-generate");
    let installed_outdir = workspace.join("gtk-installed-out");
    let requested_outdir = workspace.join("requested-out");
    let requested_artifact = requested_outdir.join("gtk.css");
    let installed_artifact = installed_outdir.join("gtk.css");

    let mut install = isolated_command(&workspace);
    install.args([
        "target",
        "install",
        "--target",
        example_target_path("gtk.toml")
            .to_str()
            .expect("example target path should be utf-8"),
        "--outdir",
        installed_outdir
            .to_str()
            .expect("installed outdir should be utf-8"),
    ]);
    install.assert().success();

    let mut generate = isolated_command(&workspace);
    generate.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        example_target_path("gtk.toml")
            .to_str()
            .expect("example target path should be utf-8"),
        "--output",
        requested_outdir
            .to_str()
            .expect("requested outdir should be utf-8"),
    ]);
    let assert = generate.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains(
            requested_artifact
                .to_str()
                .expect("requested artifact path should be utf-8")
        ),
        "expected generate to write explicit path target to requested outdir '{}', got:\n{stdout}",
        requested_artifact.display()
    );

    let metadata = fs::metadata(&requested_artifact).expect("requested artifact should exist");
    assert!(metadata.is_file());
    assert!(
        !installed_artifact.exists(),
        "explicit path render should not write to installed outdir '{}'",
        installed_artifact.display()
    );

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn explicit_path_target_keeps_requested_output_when_name_is_also_installed() {
    let workspace = temp_dir_path("target-install-name-and-path-generate");
    let installed_outdir = workspace.join("gtk-installed-out");
    let requested_outdir = workspace.join("requested-out");
    let requested_artifact = requested_outdir.join("gtk.css");
    let installed_artifact = installed_outdir.join("gtk.css");
    let target = example_target_path("gtk.toml");

    let mut install = isolated_command(&workspace);
    install.args([
        "target",
        "install",
        "--target",
        target
            .to_str()
            .expect("example target path should be utf-8"),
        "--outdir",
        installed_outdir
            .to_str()
            .expect("installed outdir should be utf-8"),
    ]);
    install.assert().success();

    let mut generate = isolated_command(&workspace);
    generate.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        &format!(
            "gtk,{}",
            target
                .to_str()
                .expect("example target path should be utf-8")
        ),
        "--output",
        requested_outdir
            .to_str()
            .expect("requested outdir should be utf-8"),
    ]);
    let assert = generate.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    assert!(
        stdout.contains(
            installed_artifact
                .to_str()
                .expect("installed artifact path should be utf-8")
        ),
        "expected generate output to mention installed outdir '{}', got:\n{stdout}",
        installed_artifact.display()
    );
    assert!(
        stdout.contains(
            requested_artifact
                .to_str()
                .expect("requested artifact path should be utf-8")
        ),
        "expected generate output to mention requested outdir '{}', got:\n{stdout}",
        requested_artifact.display()
    );
    assert!(installed_artifact.is_file());
    assert!(requested_artifact.is_file());

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn target_install_reinstall_requires_overwrite_flag() {
    let workspace = temp_dir_path("target-install-overwrite");
    let outdir = workspace.join("install-out");
    let config_file = workspace
        .join("xdg-config")
        .join("chromasync")
        .join("config.toml");
    let target = example_target_path("gtk.toml")
        .to_str()
        .expect("example target path should be utf-8")
        .to_owned();
    let outdir_str = outdir.to_str().expect("outdir should be utf-8").to_owned();

    let mut first = isolated_command(&workspace);
    first.args([
        "target",
        "install",
        "--target",
        &target,
        "--outdir",
        &outdir_str,
    ]);
    first.assert().success();

    let mut repeat = isolated_command(&workspace);
    repeat.args([
        "target",
        "install",
        "--target",
        &target,
        "--outdir",
        &outdir_str,
    ]);
    repeat
        .assert()
        .failure()
        .stderr(predicate::str::contains("is already installed"))
        .stderr(predicate::str::contains("pass --overwrite"));

    let mut overwrite = isolated_command(&workspace);
    overwrite.args([
        "target",
        "install",
        "--target",
        &target,
        "--outdir",
        &outdir_str,
        "--overwrite",
    ]);
    overwrite.assert().success();

    let config_content =
        fs::read_to_string(&config_file).expect("config file should exist after overwrite");
    assert!(config_content.contains("overwrite = true"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn target_install_rejects_built_in_name_collision() {
    let workspace = temp_dir_path("target-install-collision");
    let target_path = temp_file_path("kitty-collision");
    fs::write(
        &target_path,
        r#"
name = "kitty"

[[artifacts]]
file_name = "kitty.conf"
template = "foreground={{tokens.text}}"
"#,
    )
    .expect("collision target should be written");

    let mut command = isolated_command(&workspace);
    command.args([
        "target",
        "install",
        "--target",
        target_path.to_str().expect("target path should be utf-8"),
        "--outdir",
        workspace.to_str().expect("outdir should be utf-8"),
    ]);

    command.assert().failure().stderr(predicate::str::contains(
        "user target 'kitty' collides with a built-in renderer name",
    ));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
    fs::remove_file(target_path).expect("collision target should be removed");
}

#[test]
fn target_install_rejects_built_in_inheritance_without_persisting_target() {
    let workspace = temp_dir_path("target-install-built-in-inheritance");
    fs::create_dir_all(&workspace).expect("workspace should be created");
    let target_path = workspace.join("gtk-from-kitty.toml");
    fs::write(
        &target_path,
        r#"
name = "gtk-from-kitty"
extends = "kitty"

[[artifacts]]
file_name = "gtk.css"
template = "foreground={{tokens.text}}"
"#,
    )
    .expect("target should be written");

    let mut install = isolated_command(&workspace);
    install.args([
        "target",
        "install",
        "--target",
        target_path.to_str().expect("target path should be utf-8"),
        "--outdir",
        workspace.to_str().expect("outdir should be utf-8"),
    ]);

    install.assert().failure().stderr(predicate::str::contains(
        "target 'gtk-from-kitty' cannot inherit from built-in renderer 'kitty'",
    ));

    let config_root = workspace.join("xdg-config").join("chromasync");
    assert!(
        !config_root
            .join("targets")
            .join("gtk-from-kitty.toml")
            .exists(),
        "invalid target should not be copied into user config"
    );
    assert!(
        !config_root.join("config.toml").exists(),
        "invalid target should not create a config entry"
    );

    let mut targets = isolated_command(&workspace);
    targets.arg("targets");
    targets
        .assert()
        .success()
        .stdout(predicate::str::contains("kitty\tbuilt-in\tkitty"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn target_install_rejects_unknown_inheritance_without_persisting_target() {
    let workspace = temp_dir_path("target-install-unknown-inheritance");
    fs::create_dir_all(&workspace).expect("workspace should be created");
    let target_path = workspace.join("gtk-from-missing.toml");
    fs::write(
        &target_path,
        r#"
name = "gtk-from-missing"
extends = "missing-base"

[[artifacts]]
file_name = "gtk.css"
template = "foreground={{tokens.text}}"
"#,
    )
    .expect("target should be written");

    let mut install = isolated_command(&workspace);
    install.args([
        "target",
        "install",
        "--target",
        target_path.to_str().expect("target path should be utf-8"),
        "--outdir",
        workspace.to_str().expect("outdir should be utf-8"),
    ]);

    install.assert().failure().stderr(predicate::str::contains(
        "target 'gtk-from-missing' references unknown base target 'missing-base'",
    ));

    let config_root = workspace.join("xdg-config").join("chromasync");
    assert!(
        !config_root
            .join("targets")
            .join("gtk-from-missing.toml")
            .exists(),
        "invalid target should not be copied into user config"
    );
    assert!(
        !config_root.join("config.toml").exists(),
        "invalid target should not create a config entry"
    );

    let mut targets = isolated_command(&workspace);
    targets.arg("targets");
    targets
        .assert()
        .success()
        .stdout(predicate::str::contains("kitty\tbuilt-in\tkitty"));

    fs::remove_dir_all(workspace).expect("workspace should be removed");
}

#[test]
fn installed_overwrite_flag_forces_existing_artifact() {
    let workspace = temp_dir_path("target-install-overwrite-force");
    let outdir = workspace.join("force-out");
    fs::create_dir_all(&outdir).expect("outdir should be created");
    let artifact_path = outdir.join("gtk.css");
    fs::write(&artifact_path, "existing").expect("existing artifact should be written");

    let target = example_target_path("gtk.toml")
        .to_str()
        .expect("example target path should be utf-8")
        .to_owned();
    let outdir_str = outdir.to_str().expect("outdir should be utf-8").to_owned();

    let mut install = isolated_command(&workspace);
    install.args([
        "target",
        "install",
        "--target",
        &target,
        "--outdir",
        &outdir_str,
        "--overwrite",
    ]);
    install.assert().success();

    let mut generate = isolated_command(&workspace);
    generate.args([
        "generate",
        "--seed",
        "#4ecdc4",
        "--template",
        "minimal",
        "--targets",
        "gtk",
    ]);
    generate.assert().success();

    let content = fs::read_to_string(&artifact_path).expect("artifact should be readable");
    assert_ne!(
        content, "existing",
        "installed overwrite=true should force-overwrite the existing artifact"
    );

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
