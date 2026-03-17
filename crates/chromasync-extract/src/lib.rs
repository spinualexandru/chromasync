use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use image::{DynamicImage, GenericImageView, RgbaImage, imageops::FilterType};
use thiserror::Error;

pub const MAX_PROCESSING_DIMENSION: u32 = 128;
const MAX_SEEDS: usize = 3;
const QUANTIZATION_SHIFT: u8 = 4;
const MIN_VISIBLE_ALPHA: u8 = 16;
const NOISY_IMAGE_THRESHOLD: f32 = 0.10;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BucketKey(u8, u8, u8);

#[derive(Debug, Clone, Default)]
struct BucketAccumulator {
    count: u32,
    sum_r: u64,
    sum_g: u64,
    sum_b: u64,
    sum_x: u64,
    sum_y: u64,
}

#[derive(Debug, Clone)]
struct BucketSummary {
    key: BucketKey,
    count: u32,
    average_rgb: [u8; 3],
    average_x: f32,
    average_y: f32,
}

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("failed to load image '{path}': {source}")]
    ImageLoad {
        path: PathBuf,
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
    let (original_width, original_height) = loaded.dimensions();
    let processed = preprocess_image(loaded);
    let (processed_width, processed_height) = processed.dimensions();
    let seeds = cluster_image(&processed.to_rgba8(), image)?;

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
    let mut buckets = BTreeMap::<BucketKey, BucketAccumulator>::new();
    let mut overall = BucketAccumulator::default();

    for (x, y, pixel) in image.enumerate_pixels() {
        let [r, g, b, alpha] = pixel.0;

        if alpha < MIN_VISIBLE_ALPHA {
            continue;
        }

        overall.push([r, g, b], x, y);
        buckets
            .entry(BucketKey(
                r >> QUANTIZATION_SHIFT,
                g >> QUANTIZATION_SHIFT,
                b >> QUANTIZATION_SHIFT,
            ))
            .or_default()
            .push([r, g, b], x, y);
    }

    if overall.count == 0 {
        return Err(ExtractError::NoVisiblePixels {
            path: path.to_path_buf(),
        });
    }

    let mut summaries = buckets
        .into_iter()
        .map(|(key, bucket)| BucketSummary {
            key,
            count: bucket.count,
            average_rgb: bucket.average_rgb(),
            average_x: bucket.average_x(),
            average_y: bucket.average_y(),
        })
        .collect::<Vec<_>>();

    summaries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });

    if summaries
        .first()
        .is_some_and(|bucket| bucket.count as f32 / (overall.count as f32) < NOISY_IMAGE_THRESHOLD)
    {
        return Ok(vec![overall.average_seed(width, height)]);
    }

    let mut seen_hex = BTreeSet::new();
    let mut seeds = Vec::new();

    for summary in summaries {
        let hex = format_hex(summary.average_rgb);

        if !seen_hex.insert(hex.clone()) {
            continue;
        }

        seeds.push(ExtractedSeed {
            hex,
            dominance: summary.count as f32 / overall.count as f32,
            source_region: Some(region_label(
                summary.average_x / width as f32,
                summary.average_y / height as f32,
            )),
        });

        if seeds.len() == MAX_SEEDS {
            break;
        }
    }

    if seeds.is_empty() {
        seeds.push(overall.average_seed(width, height));
    }

    Ok(seeds)
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

impl BucketAccumulator {
    fn push(&mut self, rgb: [u8; 3], x: u32, y: u32) {
        self.count += 1;
        self.sum_r += u64::from(rgb[0]);
        self.sum_g += u64::from(rgb[1]);
        self.sum_b += u64::from(rgb[2]);
        self.sum_x += u64::from(x);
        self.sum_y += u64::from(y);
    }

    fn average_rgb(&self) -> [u8; 3] {
        [
            (self.sum_r / u64::from(self.count)) as u8,
            (self.sum_g / u64::from(self.count)) as u8,
            (self.sum_b / u64::from(self.count)) as u8,
        ]
    }

    fn average_x(&self) -> f32 {
        self.sum_x as f32 / self.count as f32
    }

    fn average_y(&self) -> f32 {
        self.sum_y as f32 / self.count as f32
    }

    fn average_seed(&self, width: u32, height: u32) -> ExtractedSeed {
        ExtractedSeed {
            hex: format_hex(self.average_rgb()),
            dominance: 1.0,
            source_region: Some(region_label(
                self.average_x() / width as f32,
                self.average_y() / height as f32,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, GenericImageView, RgbImage};

    use super::{MAX_PROCESSING_DIMENSION, preprocess_image, region_label};

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
}
