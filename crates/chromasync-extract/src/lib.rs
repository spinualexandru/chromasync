use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use image::{DynamicImage, GenericImageView, RgbaImage, imageops::FilterType};
use palette::{FromColor, Oklab, Srgb};
use thiserror::Error;

pub const MAX_PROCESSING_DIMENSION: u32 = 128;
const MAX_SEEDS: usize = 3;
const MAX_CLUSTERS: usize = 12;
const MAX_K_MEANS_ITERATIONS: usize = 16;
const MIN_VISIBLE_ALPHA: u8 = 16;
const MIN_SEED_DISTANCE: f64 = 0.06;
const FULL_DIVERSITY_DISTANCE: f64 = 0.18;
const CHROMA_REFERENCE: f64 = 0.22;
const POPULATION_WEIGHT: f64 = 0.70;
const CHROMA_WEIGHT: f64 = 0.20;
const CENTRALITY_WEIGHT: f64 = 0.10;

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedSeed {
    pub hex: String,
    pub dominance: f32,
    pub source_region: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractionResult {
    pub original_width: u32,
    pub original_height: u32,
    pub processed_width: u32,
    pub processed_height: u32,
    pub seeds: Vec<ExtractedSeed>,
}

#[derive(Debug, Clone, Default)]
struct PointAccumulator {
    weight: f64,
    sum_x: f64,
    sum_y: f64,
    sum_centrality: f64,
}

#[derive(Debug, Clone, Copy)]
struct ColorPoint {
    rgb: [u8; 3],
    lab: [f64; 3],
    weight: f64,
    average_x: f64,
    average_y: f64,
    centrality: f64,
}

#[derive(Debug, Clone, Default)]
struct ClusterAccumulator {
    weight: f64,
    sum_lab: [f64; 3],
    sum_rgb: [f64; 3],
    sum_x: f64,
    sum_y: f64,
    sum_centrality: f64,
}

#[derive(Debug, Clone)]
struct ClusterCandidate {
    rgb: [u8; 3],
    lab: [f64; 3],
    dominance: f64,
    average_x: f64,
    average_y: f64,
    score: f64,
}

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("failed to load image '{path}': {source}")]
    ImageLoad {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
    #[error("failed to decode image bytes: {source}")]
    ImageDecode {
        #[source]
        source: image::ImageError,
    },
    #[error("image '{path}' does not contain any visible pixels")]
    NoVisiblePixels { path: PathBuf },
}

pub fn extract_seed_candidates(image: &Path) -> Result<ExtractionResult, ExtractError> {
    let loaded = image::open(image).map_err(|source| ExtractError::ImageLoad {
        path: image.to_path_buf(),
        source,
    })?;
    extract_seed_candidates_from_image(loaded, image)
}

/// Extract seed candidates from an encoded PNG, JPEG, or WebP image.
///
/// This is the in-memory equivalent of [`extract_seed_candidates`] and is
/// suitable for browser, server, and embedded callers that do not have a file
/// path. Both entry points share the same resize and clustering pipeline.
pub fn extract_seed_candidates_from_bytes(
    image_bytes: &[u8],
) -> Result<ExtractionResult, ExtractError> {
    let loaded = image::load_from_memory(image_bytes)
        .map_err(|source| ExtractError::ImageDecode { source })?;
    extract_seed_candidates_from_image(loaded, Path::new("<memory>"))
}

fn extract_seed_candidates_from_image(
    loaded: DynamicImage,
    source: &Path,
) -> Result<ExtractionResult, ExtractError> {
    let (original_width, original_height) = loaded.dimensions();
    let processed = preprocess_image(loaded);
    let (processed_width, processed_height) = processed.dimensions();
    let seeds = cluster_image(&processed.to_rgba8(), source)?;

    Ok(ExtractionResult {
        original_width,
        original_height,
        processed_width,
        processed_height,
        seeds,
    })
}

fn preprocess_image(image: DynamicImage) -> DynamicImage {
    let (width, height) = image.dimensions();

    if width <= MAX_PROCESSING_DIMENSION && height <= MAX_PROCESSING_DIMENSION {
        return image;
    }

    image.resize(
        MAX_PROCESSING_DIMENSION,
        MAX_PROCESSING_DIMENSION,
        FilterType::Triangle,
    )
}

fn cluster_image(image: &RgbaImage, path: &Path) -> Result<Vec<ExtractedSeed>, ExtractError> {
    let (width, height) = image.dimensions();
    let mut points = BTreeMap::<[u8; 3], PointAccumulator>::new();
    let mut total_weight = 0.0;

    for (x, y, pixel) in image.enumerate_pixels() {
        let [r, g, b, alpha] = pixel.0;

        if alpha < MIN_VISIBLE_ALPHA {
            continue;
        }

        let weight = f64::from(alpha) / 255.0;
        let normalized_x = normalized_coordinate(x, width);
        let normalized_y = normalized_coordinate(y, height);
        let centrality = radial_centrality(normalized_x, normalized_y);
        let point = points.entry([r, g, b]).or_default();
        point.weight += weight;
        // Retain the public region-label boundary used by the original
        // extractor while using edge-normalized coordinates for centrality.
        point.sum_x += (f64::from(x) / f64::from(width)) * weight;
        point.sum_y += (f64::from(y) / f64::from(height)) * weight;
        point.sum_centrality += centrality * weight;
        total_weight += weight;
    }

    if points.is_empty() {
        return Err(ExtractError::NoVisiblePixels {
            path: path.to_path_buf(),
        });
    }

    let points = points
        .into_iter()
        .map(|(rgb, point)| ColorPoint {
            rgb,
            lab: rgb_to_oklab(rgb),
            weight: point.weight,
            average_x: point.sum_x / point.weight,
            average_y: point.sum_y / point.weight,
            centrality: point.sum_centrality / point.weight,
        })
        .collect::<Vec<_>>();

    let centroids = initialize_centroids(&points, MAX_CLUSTERS.min(points.len()));
    let (assignments, centroids) = run_k_means(&points, centroids);
    let mut clusters = vec![ClusterAccumulator::default(); centroids.len()];

    for (point, assignment) in points.iter().zip(assignments) {
        clusters[assignment].push(point);
    }

    let mut candidates = clusters
        .into_iter()
        .filter(|cluster| cluster.weight > 0.0)
        .map(|cluster| {
            let rgb = cluster.average_rgb();
            // Seed diversity is enforced on the actual rounded color returned
            // to callers, rather than on an unobservable centroid.
            let lab = rgb_to_oklab(rgb);
            let dominance = cluster.weight / total_weight;
            let chroma = lab[1].hypot(lab[2]);
            let population_score = dominance.sqrt();
            let chroma_score = (chroma / CHROMA_REFERENCE).clamp(0.0, 1.0);
            let centrality_score = cluster.sum_centrality / cluster.weight;
            let score = population_score * POPULATION_WEIGHT
                + chroma_score * CHROMA_WEIGHT
                + centrality_score * CENTRALITY_WEIGHT;

            ClusterCandidate {
                rgb,
                lab,
                dominance,
                average_x: cluster.sum_x / cluster.weight,
                average_y: cluster.sum_y / cluster.weight,
                score,
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right.score.total_cmp(&left.score).then_with(|| {
            right
                .dominance
                .total_cmp(&left.dominance)
                .then_with(|| left.rgb.cmp(&right.rgb))
        })
    });

    let mut selected = Vec::<ClusterCandidate>::new();
    while selected.len() < MAX_SEEDS {
        let next = candidates
            .iter()
            .filter(|candidate| {
                selected.is_empty()
                    || selected
                        .iter()
                        .all(|seed| oklab_distance(candidate.lab, seed.lab) >= MIN_SEED_DISTANCE)
            })
            .max_by(|left, right| {
                diversity_adjusted_score(left, &selected)
                    .total_cmp(&diversity_adjusted_score(right, &selected))
                    .then_with(|| left.score.total_cmp(&right.score))
                    .then_with(|| right.rgb.cmp(&left.rgb))
            })
            .cloned();

        let Some(next) = next else {
            break;
        };
        candidates.retain(|candidate| candidate.rgb != next.rgb);
        selected.push(next);
    }

    Ok(selected
        .into_iter()
        .map(|candidate| ExtractedSeed {
            hex: format_hex(candidate.rgb),
            dominance: candidate.dominance as f32,
            source_region: Some(region_label(
                candidate.average_x as f32,
                candidate.average_y as f32,
            )),
        })
        .collect())
}

fn initialize_centroids(points: &[ColorPoint], count: usize) -> Vec<[f64; 3]> {
    let total_weight = points.iter().map(|point| point.weight).sum::<f64>();
    let mut mean = [0.0; 3];
    for point in points {
        for (sum, value) in mean.iter_mut().zip(point.lab) {
            *sum += value * point.weight;
        }
    }
    for value in &mut mean {
        *value /= total_weight;
    }

    let first = points
        .iter()
        .min_by(|left, right| {
            squared_oklab_distance(left.lab, mean)
                .total_cmp(&squared_oklab_distance(right.lab, mean))
                .then_with(|| left.rgb.cmp(&right.rgb))
        })
        .expect("non-empty color points");
    let mut centroids = vec![first.lab];

    while centroids.len() < count {
        let next = points.iter().max_by(|left, right| {
            weighted_distance_from_centroids(left, &centroids)
                .total_cmp(&weighted_distance_from_centroids(right, &centroids))
                .then_with(|| right.rgb.cmp(&left.rgb))
        });
        let Some(next) = next else {
            break;
        };
        if centroids.contains(&next.lab) {
            break;
        }
        centroids.push(next.lab);
    }

    centroids
}

fn weighted_distance_from_centroids(point: &ColorPoint, centroids: &[[f64; 3]]) -> f64 {
    point.weight
        * centroids
            .iter()
            .map(|centroid| squared_oklab_distance(point.lab, *centroid))
            .min_by(f64::total_cmp)
            .unwrap_or(0.0)
}

fn run_k_means(points: &[ColorPoint], mut centroids: Vec<[f64; 3]>) -> (Vec<usize>, Vec<[f64; 3]>) {
    let mut assignments = vec![usize::MAX; points.len()];

    for _ in 0..MAX_K_MEANS_ITERATIONS {
        let mut changed = false;
        for (index, point) in points.iter().enumerate() {
            let assignment = nearest_centroid(point.lab, &centroids);
            changed |= assignments[index] != assignment;
            assignments[index] = assignment;
        }
        if !changed {
            break;
        }

        let mut sums = vec![ClusterAccumulator::default(); centroids.len()];
        for (point, assignment) in points.iter().zip(&assignments) {
            sums[*assignment].push(point);
        }
        for (centroid, sum) in centroids.iter_mut().zip(sums) {
            if sum.weight > 0.0 {
                *centroid = sum.average_lab();
            }
        }
    }

    // The iteration limit can stop immediately after centroids move. Reassign
    // once so returned memberships always correspond to the returned centroids.
    for (assignment, point) in assignments.iter_mut().zip(points) {
        *assignment = nearest_centroid(point.lab, &centroids);
    }
    (assignments, centroids)
}

fn nearest_centroid(lab: [f64; 3], centroids: &[[f64; 3]]) -> usize {
    centroids
        .iter()
        .enumerate()
        .min_by(|(left_index, left), (right_index, right)| {
            squared_oklab_distance(lab, **left)
                .total_cmp(&squared_oklab_distance(lab, **right))
                .then_with(|| left_index.cmp(right_index))
        })
        .map(|(index, _)| index)
        .expect("at least one centroid")
}

fn diversity_adjusted_score(candidate: &ClusterCandidate, selected: &[ClusterCandidate]) -> f64 {
    if selected.is_empty() {
        return candidate.score;
    }

    let diversity = selected
        .iter()
        .map(|seed| oklab_distance(candidate.lab, seed.lab))
        .min_by(f64::total_cmp)
        .unwrap_or(FULL_DIVERSITY_DISTANCE);
    let diversity_factor = (diversity / FULL_DIVERSITY_DISTANCE).clamp(0.0, 1.0);
    candidate.score * (0.9 + 0.1 * diversity_factor)
}

fn normalized_coordinate(value: u32, extent: u32) -> f64 {
    if extent <= 1 {
        0.5
    } else {
        f64::from(value) / f64::from(extent - 1)
    }
}

fn radial_centrality(x: f64, y: f64) -> f64 {
    let distance = (x - 0.5).hypot(y - 0.5);
    (1.0 - distance / std::f64::consts::FRAC_1_SQRT_2).clamp(0.0, 1.0)
}

fn rgb_to_oklab(rgb: [u8; 3]) -> [f64; 3] {
    let encoded = Srgb::new(rgb[0], rgb[1], rgb[2]).into_format::<f64>();
    let lab = Oklab::from_color(encoded.into_linear());
    [lab.l, lab.a, lab.b]
}

fn squared_oklab_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum()
}

fn oklab_distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    squared_oklab_distance(left, right).sqrt()
}

fn region_label(normalized_x: f32, normalized_y: f32) -> String {
    let horizontal = axis_label(normalized_x, "left", "center", "right");
    let vertical = axis_label(normalized_y, "top", "center", "bottom");

    if horizontal == "center" && vertical == "center" {
        "center".to_owned()
    } else {
        format!("{vertical}-{horizontal}")
    }
}

fn axis_label(
    value: f32,
    low: &'static str,
    middle: &'static str,
    high: &'static str,
) -> &'static str {
    if value < (1.0 / 3.0) {
        low
    } else if value < (2.0 / 3.0) {
        middle
    } else {
        high
    }
}

fn format_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

impl ClusterAccumulator {
    fn push(&mut self, point: &ColorPoint) {
        self.weight += point.weight;
        for ((lab_sum, rgb_sum), (lab, rgb)) in self
            .sum_lab
            .iter_mut()
            .zip(&mut self.sum_rgb)
            .zip(point.lab.into_iter().zip(point.rgb))
        {
            *lab_sum += lab * point.weight;
            *rgb_sum += f64::from(rgb) * point.weight;
        }
        self.sum_x += point.average_x * point.weight;
        self.sum_y += point.average_y * point.weight;
        self.sum_centrality += point.centrality * point.weight;
    }

    fn average_rgb(&self) -> [u8; 3] {
        self.sum_rgb
            .map(|sum| (sum / self.weight).round().clamp(0.0, 255.0) as u8)
    }

    fn average_lab(&self) -> [f64; 3] {
        self.sum_lab.map(|sum| sum / self.weight)
    }
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, GenericImageView, RgbImage, Rgba, RgbaImage};

    use super::{
        ExtractError, MAX_PROCESSING_DIMENSION, MIN_SEED_DISTANCE, cluster_image, oklab_distance,
        preprocess_image, region_label, rgb_to_oklab,
    };

    #[test]
    fn preprocess_resizes_large_images() {
        let image = DynamicImage::ImageRgb8(RgbImage::new(4096, 2048));

        let processed = preprocess_image(image);

        assert_eq!(processed.dimensions(), (128, 64));
        assert!(processed.width() <= MAX_PROCESSING_DIMENSION);
        assert!(processed.height() <= MAX_PROCESSING_DIMENSION);
    }

    #[test]
    fn region_labels_cover_grid_positions() {
        assert_eq!(region_label(0.5, 0.5), "center");
        assert_eq!(region_label(0.1, 0.1), "top-left");
        assert_eq!(region_label(0.9, 0.2), "top-right");
        assert_eq!(region_label(0.5, 0.9), "bottom-center");
        assert_eq!(region_label(0.1, 0.6), "center-left");
    }

    #[test]
    fn perceptually_near_colors_do_not_become_duplicate_seeds() {
        let image = row_image(&[
            ([127, 48, 48, 255], 4),
            ([128, 48, 48, 255], 4),
            ([30, 80, 220, 255], 2),
        ]);

        let seeds = cluster_image(&image, std::path::Path::new("synthetic"))
            .expect("synthetic image should extract");

        assert_eq!(seeds.len(), 2);
        assert!(seeds.iter().any(|seed| seed.hex == "#1e50dc"));
        assert_eq!(
            seeds
                .iter()
                .filter(|seed| seed.hex == "#7f3030" || seed.hex == "#803030")
                .count(),
            1
        );
    }

    #[test]
    fn selected_seeds_have_a_perceptual_diversity_floor() {
        let image = row_image(&[
            ([190, 190, 190, 255], 8),
            ([220, 40, 40, 255], 3),
            ([35, 70, 220, 255], 3),
            ([35, 190, 95, 255], 3),
        ]);

        let seeds = cluster_image(&image, std::path::Path::new("synthetic"))
            .expect("synthetic image should extract");

        assert_eq!(seeds.len(), 3);
        for (index, seed) in seeds.iter().enumerate() {
            for other in seeds.iter().skip(index + 1) {
                let distance = oklab_distance(hex_to_oklab(&seed.hex), hex_to_oklab(&other.hex));
                assert!(distance >= MIN_SEED_DISTANCE, "{seed:?} and {other:?}");
            }
        }
    }

    #[test]
    fn small_colorful_accents_survive_a_large_neutral_background() {
        let mut image = RgbaImage::from_pixel(20, 20, Rgba([105, 105, 105, 255]));
        for y in 8..12 {
            for x in 8..12 {
                image.put_pixel(x, y, Rgba([235, 35, 45, 255]));
            }
        }

        let seeds = cluster_image(&image, std::path::Path::new("synthetic"))
            .expect("synthetic image should extract");

        assert_eq!(seeds.len(), 2);
        assert!(seeds.iter().any(|seed| seed.hex == "#eb232d"));
        let accent = seeds
            .iter()
            .find(|seed| seed.hex == "#eb232d")
            .expect("accent should be selected");
        assert!((accent.dominance - 0.04).abs() < 0.001);
        assert_eq!(accent.source_region.as_deref(), Some("center"));
    }

    #[test]
    fn clustering_is_deterministic_and_ignores_transparent_noise() {
        let mut image = RgbaImage::from_pixel(8, 8, Rgba([20, 90, 180, 255]));
        for x in 0..8 {
            image.put_pixel(x, 0, Rgba([255, (x * 20) as u8, 10, 0]));
        }

        let first = cluster_image(&image, std::path::Path::new("synthetic"))
            .expect("synthetic image should extract");
        let second = cluster_image(&image, std::path::Path::new("synthetic"))
            .expect("synthetic image should extract");

        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].hex, "#145ab4");
        assert!((first[0].dominance - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fully_transparent_images_return_no_visible_pixels() {
        let image = RgbaImage::from_pixel(4, 4, Rgba([240, 30, 40, 0]));

        let error = cluster_image(&image, std::path::Path::new("transparent"))
            .expect_err("transparent image should not produce a seed");

        assert!(matches!(error, ExtractError::NoVisiblePixels { .. }));
    }

    fn row_image(runs: &[([u8; 4], u32)]) -> RgbaImage {
        let width = runs.iter().map(|(_, count)| count).sum();
        let mut image = RgbaImage::new(width, 1);
        let mut x = 0;
        for (rgba, count) in runs {
            for _ in 0..*count {
                image.put_pixel(x, 0, Rgba(*rgba));
                x += 1;
            }
        }
        image
    }

    fn hex_to_oklab(hex: &str) -> [f64; 3] {
        let component = |start| u8::from_str_radix(&hex[start..start + 2], 16).unwrap();
        rgb_to_oklab([component(1), component(3), component(5)])
    }
}
