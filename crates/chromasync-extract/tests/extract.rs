use std::path::PathBuf;

use chromasync_extract::extract_seed_candidates;

#[test]
fn extracts_dominant_seed_candidates_from_wallpaper_fixture() {
    let result = extract_seed_candidates(&fixture("wallpaper-blocks.png"))
        .expect("wallpaper extraction should succeed");

    assert_eq!(result.original_width, 6);
    assert_eq!(result.original_height, 4);
    assert_eq!(result.processed_width, 6);
    assert_eq!(result.processed_height, 4);
    assert_eq!(result.seeds.len(), 3);
    assert_eq!(result.seeds[0].hex, "#d1495b");
    assert_eq!(result.seeds[1].hex, "#2b9eb3");
    assert_eq!(result.seeds[2].hex, "#4361ee");
    assert_eq!(
        result.seeds[0].source_region.as_deref(),
        Some("center-left")
    );
    assert_eq!(result.seeds[1].source_region.as_deref(), Some("center"));
    assert_eq!(
        result.seeds[2].source_region.as_deref(),
        Some("center-right")
    );
    assert_close(result.seeds[0].dominance, 0.5);
    assert_close(result.seeds[1].dominance, 8.0 / 24.0);
    assert_close(result.seeds[2].dominance, 4.0 / 24.0);
}

#[test]
fn low_color_images_return_a_single_seed() {
    let result = extract_seed_candidates(&fixture("wallpaper-monochrome.png"))
        .expect("monochrome extraction should succeed");

    assert_eq!(result.seeds.len(), 1);
    assert_eq!(result.seeds[0].hex, "#204060");
    assert_eq!(result.seeds[0].source_region.as_deref(), Some("center"));
    assert_close(result.seeds[0].dominance, 1.0);
}

#[test]
fn noisy_images_extract_individual_seeds_above_threshold() {
    let result = extract_seed_candidates(&fixture("wallpaper-noisy.png"))
        .expect("noisy extraction should succeed");

    assert_eq!(result.seeds.len(), 3);
    assert!(result.seeds[0].dominance > 0.0);
    assert!(result.seeds[0].source_region.is_some());
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn assert_close(actual: f32, expected: f32) {
    let delta = (actual - expected).abs();
    assert!(
        delta < 0.001,
        "expected {actual} to be within 0.001 of {expected}"
    );
}
