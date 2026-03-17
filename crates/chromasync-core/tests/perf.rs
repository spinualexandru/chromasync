use std::{
    hint::black_box,
    path::PathBuf,
    time::{Duration, Instant},
};

use chromasync_types::{ContrastStrategy, GenerationRequest, ThemeMode};

#[test]
#[ignore = "profiling harness for Phase 6 hot paths"]
fn profile_seed_generation_hot_path() {
    let request = GenerationRequest {
        seed: Some("#4ecdc4".to_owned()),
        wallpaper: None,
        template: "terminal".to_owned(),
        mode: ThemeMode::Dark,
        contrast: ContrastStrategy::RelativeLuminance,
        targets: vec![
            example_target_path("gtk.toml"),
            example_target_path("hyprland.toml"),
            "kitty".to_owned(),
            example_target_path("css.toml"),
            "alacritty".to_owned(),
            example_target_path("foot.toml"),
            example_target_path("waybar.toml"),
            example_target_path("editor.toml"),
        ],
        output_dir: "chromasync".into(),
    };

    let started = Instant::now();

    for _ in 0..200 {
        let artifacts = chromasync_core::generate(&request).expect("generation should succeed");
        black_box(artifacts);
    }

    let elapsed = started.elapsed();
    eprintln!("200 seed generations took {:?}", elapsed);

    assert!(
        elapsed < Duration::from_secs(5),
        "seed generation regression: {:?}",
        elapsed
    );
}

#[test]
#[ignore = "profiling harness for Phase 6 hot paths"]
fn profile_wallpaper_generation_hot_path() {
    let request = GenerationRequest {
        seed: None,
        wallpaper: Some(wallpaper_fixture("wallpaper-blocks.png")),
        template: "terminal".to_owned(),
        mode: ThemeMode::Dark,
        contrast: ContrastStrategy::ApcaExperimental,
        targets: vec![
            example_target_path("css.toml"),
            example_target_path("waybar.toml"),
            example_target_path("foot.toml"),
            example_target_path("editor.toml"),
        ],
        output_dir: "chromasync".into(),
    };

    let started = Instant::now();

    for _ in 0..40 {
        let artifacts =
            chromasync_core::generate_from_wallpaper(&request).expect("generation should succeed");
        black_box(artifacts);
    }

    let elapsed = started.elapsed();
    eprintln!("40 wallpaper generations took {:?}", elapsed);

    assert!(
        elapsed < Duration::from_secs(5),
        "wallpaper generation regression: {:?}",
        elapsed
    );
}

fn wallpaper_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../chromasync-extract/tests/fixtures")
        .join(name)
}

fn example_target_path(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/targets")
        .join(name)
        .display()
        .to_string()
}
