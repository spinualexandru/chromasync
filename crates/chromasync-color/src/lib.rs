use std::collections::BTreeMap;

use chromasync_types::{
    ChromaStrategy, ContrastStrategy, GeneratedPalette, HexColor, PaletteFamily, PaletteFamilyName,
    ThemeMode, ToneSample,
};
use palette::{
    FromColor, LinSrgb, Okhsl, Oklab, OklabHue, Oklch, Srgb, convert::FromColorUnclamped,
};
use thiserror::Error;

pub const MIN_CONTRAST_RATIO: f32 = 4.5;
pub const MIN_APCA_SCORE: f32 = 60.0;
pub const SAMPLE_TONES: [u8; 16] = [
    0, 6, 10, 14, 20, 30, 40, 45, 50, 60, 70, 80, 90, 94, 98, 100,
];

const GAMUT_JND: f32 = 0.02;
const GAMUT_EPSILON: f32 = 0.0001;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSeedColor {
    pub hex: HexColor,
    pub rgb: [u8; 3],
    pub lightness: f32,
    pub chroma: f32,
    pub hue: f32,
}

/// Human-adjustable Okhsl coordinates in the sRGB gamut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OkhslColor {
    /// Hue in degrees from 0 (inclusive) to 360 (exclusive).
    pub hue: f32,
    /// Perceptual saturation from 0.0 to 1.0.
    pub saturation: f32,
    /// Perceptual lightness from 0.0 to 1.0.
    pub lightness: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadableSelection {
    pub hex: HexColor,
    pub score: f32,
}

#[derive(Debug, Error)]
pub enum ColorError {
    #[error("seed color '{seed}' must use the #RRGGBB format")]
    InvalidSeedFormat { seed: String },
    #[error("color '{value}' must use the #RRGGBB format")]
    InvalidColorFormat { value: String },
    #[error("color '{value}' contains invalid hexadecimal digits")]
    InvalidHexDigits { value: String },
    #[error("tone value {tone} must be within 0.0..=1.0")]
    InvalidTone { tone: f32 },
    #[error("Okhsl {component} value {value} must be finite and within {range}")]
    InvalidOkhslComponent {
        component: &'static str,
        value: f32,
        range: &'static str,
    },
    #[error("at least one foreground candidate is required")]
    MissingContrastCandidates,
}

#[derive(Debug, Clone, Copy)]
struct ChromaModifier {
    primary_scale: f32,
    neutral_scale: f32,
    signal_scale: f32,
    min_primary: f32,
}

impl ChromaModifier {
    fn from_strategy(strategy: ChromaStrategy) -> Self {
        match strategy {
            ChromaStrategy::Subtle => Self {
                primary_scale: 0.6,
                neutral_scale: 0.5,
                signal_scale: 0.7,
                min_primary: 0.04,
            },
            ChromaStrategy::Normal => Self {
                primary_scale: 1.0,
                neutral_scale: 1.0,
                signal_scale: 1.0,
                min_primary: 0.08,
            },
            ChromaStrategy::Vibrant => Self {
                primary_scale: 1.3,
                neutral_scale: 1.8,
                signal_scale: 1.2,
                min_primary: 0.12,
            },
            ChromaStrategy::Muted => Self {
                primary_scale: 0.7,
                neutral_scale: 0.8,
                signal_scale: 0.6,
                min_primary: 0.02,
            },
            ChromaStrategy::Industrial => Self {
                primary_scale: 1.0,
                neutral_scale: 0.0,
                signal_scale: 0.8,
                min_primary: 0.08,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FamilySpec {
    name: PaletteFamilyName,
    hue: f32,
    base_chroma: f32,
}

pub fn parse_seed_color(seed: &str) -> Result<ParsedSeedColor, ColorError> {
    let rgb = parse_rgb_hex(seed, true)?;
    let (lightness, chroma, hue) = rgb_to_oklch(rgb);

    Ok(ParsedSeedColor {
        hex: format_hex(rgb),
        rgb,
        lightness,
        chroma,
        hue,
    })
}

/// Converts a `#RRGGBB` sRGB color into Okhsl coordinates.
pub fn hex_to_okhsl(value: &str) -> Result<OkhslColor, ColorError> {
    let rgb = parse_rgb_hex(value, false)?;
    let srgb = Srgb::new(rgb[0], rgb[1], rgb[2]).into_format::<f32>();
    let oklab = Oklab::from_color(srgb.into_linear());
    let okhsl = Okhsl::from_color_unclamped(oklab);

    Ok(OkhslColor {
        hue: sanitize_hue(okhsl.hue.into_positive_degrees()),
        saturation: okhsl.saturation.clamp(0.0, 1.0),
        lightness: okhsl.lightness.clamp(0.0, 1.0),
    })
}

/// Converts sRGB-referenced Okhsl coordinates into a `#RRGGBB` color.
pub fn okhsl_to_hex(hue: f32, saturation: f32, lightness: f32) -> Result<HexColor, ColorError> {
    validate_okhsl_component("hue", hue, 0.0, 360.0, "0.0..=360.0")?;
    validate_okhsl_component("saturation", saturation, 0.0, 1.0, "0.0..=1.0")?;
    validate_okhsl_component("lightness", lightness, 0.0, 1.0, "0.0..=1.0")?;

    let okhsl = Okhsl::new(
        OklabHue::from_degrees(sanitize_hue(hue)),
        saturation,
        lightness,
    );
    let oklab = Oklab::from_color_unclamped(okhsl);
    let encoded = Srgb::from_linear(LinSrgb::from_color_unclamped(oklab));

    Ok(format!(
        "#{red:02X}{green:02X}{blue:02X}",
        red = channel_to_u8(encoded.red),
        green = channel_to_u8(encoded.green),
        blue = channel_to_u8(encoded.blue),
    ))
}

pub fn generate_palette(
    seed: &str,
    mode: ThemeMode,
    chroma: ChromaStrategy,
) -> Result<GeneratedPalette, ColorError> {
    let parsed_seed = parse_seed_color(seed)?;
    let mut families = BTreeMap::new();

    for spec in derive_family_specs(&parsed_seed, chroma) {
        let mut tones = Vec::with_capacity(SAMPLE_TONES.len());

        for tone in SAMPLE_TONES {
            tones.push(ToneSample {
                tone,
                hex: resolve_color(spec.hue, spec.base_chroma, f32::from(tone) / 100.0)?,
            });
        }

        families.insert(
            spec.name,
            PaletteFamily {
                name: spec.name,
                hue: spec.hue,
                base_chroma: spec.base_chroma,
                tones,
                dominance: None,
                source_region: None,
                seed_index: Some(0),
            },
        );
    }

    Ok(GeneratedPalette {
        seed: parsed_seed.hex,
        mode,
        chroma,
        families,
    })
}

pub fn resolve_family_color(family: &PaletteFamily, tone: f32) -> Result<HexColor, ColorError> {
    resolve_color(family.hue, family.base_chroma, tone)
}

pub fn resolve_color_from_components(
    hue: f32,
    base_chroma: f32,
    tone: f32,
) -> Result<HexColor, ColorError> {
    resolve_color(hue, base_chroma, tone)
}

pub fn chroma_curve(tone: f32) -> Result<f32, ColorError> {
    validate_tone(tone)?;

    let centered = (tone * 2.0) - 1.0;
    let bell = (1.0 - centered * centered).max(0.0);

    // Previously: 0.18 + (bell * 0.82)
    // New: 0.45 + (bell * 0.55) - allows 45% of base chroma even at extreme tones.
    Ok(0.45 + (bell * 0.55))
}

pub fn contrast_ratio(foreground: &str, background: &str) -> Result<f32, ColorError> {
    let foreground_luminance = relative_luminance(parse_rgb_hex(foreground, false)?);
    let background_luminance = relative_luminance(parse_rgb_hex(background, false)?);
    let lighter = foreground_luminance.max(background_luminance);
    let darker = foreground_luminance.min(background_luminance);

    Ok((lighter + 0.05) / (darker + 0.05))
}

/// Calculates the signed APCA Lc value for a text color on a background color.
///
/// Positive values indicate dark text on a light background; negative values
/// indicate light text on a dark background. Threshold checks should compare
/// the magnitude while retaining this sign as polarity information.
pub fn apca_contrast_score(foreground: &str, background: &str) -> Result<f32, ColorError> {
    let foreground_luminance = apca_soft_clamp(apca_luminance(parse_rgb_hex(foreground, false)?));
    let background_luminance = apca_soft_clamp(apca_luminance(parse_rgb_hex(background, false)?));

    if (background_luminance - foreground_luminance).abs() < APCA_DELTA_Y_MIN {
        return Ok(0.0);
    }

    // SAPC-8 / APCA 0.0.98G-4g core formula, matching apca-w3 0.1.9.
    // https://github.com/Myndex/apca-w3/blob/master/src/apca-w3.js
    let sapc = if background_luminance > foreground_luminance {
        (background_luminance.powf(APCA_NORM_BG) - foreground_luminance.powf(APCA_NORM_TEXT))
            * APCA_SCALE_BOW
    } else {
        (background_luminance.powf(APCA_REVERSE_BG) - foreground_luminance.powf(APCA_REVERSE_TEXT))
            * APCA_SCALE_WOB
    };

    let score = if background_luminance > foreground_luminance {
        if sapc < APCA_LOW_CLIP {
            0.0
        } else {
            sapc - APCA_LOW_BOW_OFFSET
        }
    } else if sapc > -APCA_LOW_CLIP {
        0.0
    } else {
        sapc + APCA_LOW_WOB_OFFSET
    };

    Ok((score * 100.0) as f32)
}

pub fn contrast_score(
    foreground: &str,
    background: &str,
    strategy: ContrastStrategy,
) -> Result<f32, ColorError> {
    match strategy {
        ContrastStrategy::RelativeLuminance => contrast_ratio(foreground, background),
        ContrastStrategy::ApcaExperimental => apca_contrast_score(foreground, background),
    }
}

pub fn minimum_contrast_score(strategy: ContrastStrategy) -> f32 {
    match strategy {
        ContrastStrategy::RelativeLuminance => MIN_CONTRAST_RATIO,
        ContrastStrategy::ApcaExperimental => MIN_APCA_SCORE,
    }
}

pub fn meets_contrast_threshold(score: f32, strategy: ContrastStrategy) -> bool {
    contrast_score_magnitude(score, strategy) >= minimum_contrast_score(strategy)
}

pub fn select_readable_color(
    background: &str,
    candidates: &[HexColor],
) -> Result<ReadableSelection, ColorError> {
    select_readable_color_with_strategy(background, candidates, ContrastStrategy::RelativeLuminance)
}

pub fn select_readable_color_with_strategy(
    background: &str,
    candidates: &[HexColor],
    strategy: ContrastStrategy,
) -> Result<ReadableSelection, ColorError> {
    let mut best: Option<ReadableSelection> = None;

    for candidate in candidates {
        let score = contrast_score(candidate, background, strategy)?;
        let selection = ReadableSelection {
            hex: candidate.clone(),
            score,
        };

        let replace = match &best {
            Some(current) => {
                let current_meets = meets_contrast_threshold(current.score, strategy);
                let selection_meets = meets_contrast_threshold(selection.score, strategy);

                (selection_meets && !current_meets)
                    || (selection_meets == current_meets
                        && contrast_score_magnitude(selection.score, strategy)
                            > contrast_score_magnitude(current.score, strategy))
            }
            None => true,
        };

        if replace {
            best = Some(selection);
        }
    }

    best.ok_or(ColorError::MissingContrastCandidates)
}

/// Maps an OkLCh color into sRGB using CSS Color 4 local-MINDE chroma reduction.
///
/// The returned value is converted back to OkLCh to preserve this crate's
/// public API. Local clipping can therefore introduce a small, bounded change
/// in lightness and hue for colors just outside sRGB.
pub fn gamut_map(color: Oklch) -> Oklch {
    let lightness = if color.l.is_nan() { 0.0 } else { color.l };
    let hue = sanitize_hue(color.hue.into_positive_degrees());
    let chroma = if color.chroma.is_finite() {
        color.chroma.max(0.0)
    } else {
        0.0
    };
    let target = Oklch::new(lightness, chroma, OklabHue::from_degrees(hue));

    // CSS Color 4 defines explicit endpoints before testing the destination
    // gamut. Chroma and hue do not survive beyond the SDR lightness range.
    if lightness >= 1.0 {
        return Oklch::new(1.0, 0.0, OklabHue::from_degrees(0.0));
    }
    if lightness <= 0.0 {
        return Oklch::new(0.0, 0.0, OklabHue::from_degrees(0.0));
    }

    if is_displayable(target) {
        return target;
    }

    let mut clipped = clip_to_srgb(target);
    let mut difference = delta_e_ok(target, clipped);

    if difference < GAMUT_JND {
        return clipped;
    }

    let mut min = 0.0;
    let mut max = target.chroma;
    let mut min_in_gamut = true;

    while max - min > GAMUT_EPSILON {
        let chroma = (min + max) / 2.0;
        let current = Oklch::new(lightness, chroma, OklabHue::from_degrees(hue));

        if min_in_gamut && is_displayable(current) {
            min = chroma;
            continue;
        }

        clipped = clip_to_srgb(current);
        difference = delta_e_ok(current, clipped);

        if difference < GAMUT_JND {
            if GAMUT_JND - difference < GAMUT_EPSILON {
                return clipped;
            }

            min_in_gamut = false;
            min = chroma;
        } else {
            max = chroma;
        }
    }

    clipped
}

fn derive_family_specs(seed: &ParsedSeedColor, strategy: ChromaStrategy) -> [FamilySpec; 9] {
    let modifier = ChromaModifier::from_strategy(strategy);
    let primary_chroma = clamp(
        seed.chroma * modifier.primary_scale,
        modifier.min_primary,
        0.32,
    );
    let seed_hue = seed.hue;

    [
        FamilySpec {
            name: PaletteFamilyName::Primary,
            hue: seed_hue,
            base_chroma: primary_chroma,
        },
        FamilySpec {
            name: PaletteFamilyName::Secondary,
            hue: shift_hue(seed_hue, 28.0),
            base_chroma: clamp(primary_chroma * 0.72, 0.045, 0.24),
        },
        FamilySpec {
            name: PaletteFamilyName::Tertiary,
            hue: shift_hue(seed_hue, 72.0),
            base_chroma: clamp(primary_chroma * 0.82, 0.055, 0.26),
        },
        FamilySpec {
            name: PaletteFamilyName::Neutral,
            hue: seed_hue,
            base_chroma: clamp(primary_chroma * 0.22 * modifier.neutral_scale, 0.0, 0.12),
        },
        FamilySpec {
            name: PaletteFamilyName::NeutralVariant,
            hue: shift_hue(seed_hue, 12.0),
            base_chroma: clamp(primary_chroma * 0.32 * modifier.neutral_scale, 0.0, 0.16),
        },
        FamilySpec {
            name: PaletteFamilyName::Error,
            hue: mix_hue(seed_hue, 25.0, 0.85),
            base_chroma: clamp(
                primary_chroma * 0.95 * modifier.signal_scale,
                0.16 * modifier.signal_scale,
                0.32,
            ),
        },
        FamilySpec {
            name: PaletteFamilyName::Success,
            hue: mix_hue(seed_hue, 145.0, 0.85),
            base_chroma: clamp(
                primary_chroma * 0.85 * modifier.signal_scale,
                0.14 * modifier.signal_scale,
                0.30,
            ),
        },
        FamilySpec {
            name: PaletteFamilyName::Warning,
            hue: mix_hue(seed_hue, 95.0, 0.85),
            base_chroma: clamp(
                primary_chroma * modifier.signal_scale,
                0.18 * modifier.signal_scale,
                0.32,
            ),
        },
        FamilySpec {
            name: PaletteFamilyName::Info,
            hue: mix_hue(seed_hue, 230.0, 0.85),
            base_chroma: clamp(
                primary_chroma * 0.78 * modifier.signal_scale,
                0.12 * modifier.signal_scale,
                0.28,
            ),
        },
    ]
}

fn resolve_color(hue: f32, base_chroma: f32, tone: f32) -> Result<HexColor, ColorError> {
    validate_tone(tone)?;
    let chroma = base_chroma.max(0.0) * chroma_curve(tone)?;
    let mapped = gamut_map(Oklch::new(
        tone,
        chroma,
        OklabHue::from_degrees(sanitize_hue(hue)),
    ));

    Ok(oklch_to_hex(mapped))
}

fn parse_rgb_hex(value: &str, seed_context: bool) -> Result<[u8; 3], ColorError> {
    let normalized = value.strip_prefix('#').unwrap_or(value);

    if normalized.len() != 6 {
        return Err(if seed_context {
            ColorError::InvalidSeedFormat {
                seed: value.to_owned(),
            }
        } else {
            ColorError::InvalidColorFormat {
                value: value.to_owned(),
            }
        });
    }

    let red =
        u8::from_str_radix(&normalized[0..2], 16).map_err(|_| ColorError::InvalidHexDigits {
            value: value.to_owned(),
        })?;
    let green =
        u8::from_str_radix(&normalized[2..4], 16).map_err(|_| ColorError::InvalidHexDigits {
            value: value.to_owned(),
        })?;
    let blue =
        u8::from_str_radix(&normalized[4..6], 16).map_err(|_| ColorError::InvalidHexDigits {
            value: value.to_owned(),
        })?;

    Ok([red, green, blue])
}

fn rgb_to_oklch(rgb: [u8; 3]) -> (f32, f32, f32) {
    let srgb = Srgb::new(rgb[0], rgb[1], rgb[2]).into_format::<f32>();
    let color = Oklch::from_color(srgb.into_linear());

    (
        color.l,
        color.chroma,
        sanitize_hue(color.hue.into_positive_degrees()),
    )
}

fn oklch_to_hex(color: Oklch) -> HexColor {
    let encoded = Srgb::from_linear(LinSrgb::from_color_unclamped(color));

    format!(
        "#{red:02X}{green:02X}{blue:02X}",
        red = channel_to_u8(encoded.red),
        green = channel_to_u8(encoded.green),
        blue = channel_to_u8(encoded.blue),
    )
}

fn channel_to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn relative_luminance(rgb: [u8; 3]) -> f32 {
    let [red, green, blue] = rgb.map(srgb_channel_to_linear);

    (0.2126 * red) + (0.7152 * green) + (0.0722 * blue)
}

const APCA_MAIN_TRC: f64 = 2.4;
const APCA_RED_COEFFICIENT: f64 = 0.2126729;
const APCA_GREEN_COEFFICIENT: f64 = 0.7151522;
const APCA_BLUE_COEFFICIENT: f64 = 0.072175;
const APCA_NORM_BG: f64 = 0.56;
const APCA_NORM_TEXT: f64 = 0.57;
const APCA_REVERSE_TEXT: f64 = 0.62;
const APCA_REVERSE_BG: f64 = 0.65;
const APCA_BLACK_THRESHOLD: f64 = 0.022;
const APCA_BLACK_CLAMP: f64 = 1.414;
const APCA_SCALE_BOW: f64 = 1.14;
const APCA_SCALE_WOB: f64 = 1.14;
const APCA_LOW_BOW_OFFSET: f64 = 0.027;
const APCA_LOW_WOB_OFFSET: f64 = 0.027;
const APCA_DELTA_Y_MIN: f64 = 0.0005;
const APCA_LOW_CLIP: f64 = 0.1;

fn apca_luminance(rgb: [u8; 3]) -> f64 {
    let [red, green, blue] = rgb.map(|channel| {
        let normalized = f64::from(channel) / 255.0;

        normalized.powf(APCA_MAIN_TRC)
    });

    (APCA_RED_COEFFICIENT * red) + (APCA_GREEN_COEFFICIENT * green) + (APCA_BLUE_COEFFICIENT * blue)
}

fn apca_soft_clamp(luminance: f64) -> f64 {
    if luminance > APCA_BLACK_THRESHOLD {
        luminance
    } else {
        luminance + (APCA_BLACK_THRESHOLD - luminance).powf(APCA_BLACK_CLAMP)
    }
}

fn contrast_score_magnitude(score: f32, strategy: ContrastStrategy) -> f32 {
    match strategy {
        ContrastStrategy::RelativeLuminance => score,
        ContrastStrategy::ApcaExperimental => score.abs(),
    }
}

fn srgb_channel_to_linear(channel: u8) -> f32 {
    let normalized = f32::from(channel) / 255.0;

    if normalized <= 0.04045 {
        normalized / 12.92
    } else {
        ((normalized + 0.055) / 1.055).powf(2.4)
    }
}

fn is_displayable(color: Oklch) -> bool {
    let linear = LinSrgb::from_color_unclamped(color);

    [linear.red, linear.green, linear.blue]
        .into_iter()
        .all(|channel| channel.is_finite() && (0.0..=1.0).contains(&channel))
}

fn clip_to_srgb(color: Oklch) -> Oklch {
    let linear = LinSrgb::from_color_unclamped(color);
    let clipped = LinSrgb::new(
        linear.red.clamp(0.0, 1.0),
        linear.green.clamp(0.0, 1.0),
        linear.blue.clamp(0.0, 1.0),
    );

    Oklch::from_color(clipped)
}

fn delta_e_ok(left: Oklch, right: Oklch) -> f32 {
    let left = Oklab::from_color(left);
    let right = Oklab::from_color(right);

    ((left.l - right.l).powi(2) + (left.a - right.a).powi(2) + (left.b - right.b).powi(2)).sqrt()
}

#[cfg(test)]
fn gamut_map_exact_boundary(color: Oklch) -> Oklch {
    let lightness = if !color.l.is_nan() {
        color.l.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let hue = sanitize_hue(color.hue.into_positive_degrees());
    let chroma = if color.chroma.is_finite() {
        color.chroma.max(0.0)
    } else {
        0.0
    };
    let target = Oklch::new(lightness, chroma, OklabHue::from_degrees(hue));

    if is_displayable(target) {
        return target;
    }

    let mut low = 0.0;
    let mut high = target.chroma;

    for _ in 0..24 {
        let mid = (low + high) / 2.0;
        let candidate = Oklch::new(lightness, mid, OklabHue::from_degrees(hue));

        if is_displayable(candidate) {
            low = mid;
        } else {
            high = mid;
        }
    }

    Oklch::new(lightness, low, OklabHue::from_degrees(hue))
}

fn validate_tone(tone: f32) -> Result<(), ColorError> {
    if tone.is_finite() && (0.0..=1.0).contains(&tone) {
        Ok(())
    } else {
        Err(ColorError::InvalidTone { tone })
    }
}

fn validate_okhsl_component(
    component: &'static str,
    value: f32,
    min: f32,
    max: f32,
    range: &'static str,
) -> Result<(), ColorError> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(ColorError::InvalidOkhslComponent {
            component,
            value,
            range,
        })
    }
}

fn format_hex(rgb: [u8; 3]) -> HexColor {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

fn sanitize_hue(hue: f32) -> f32 {
    if hue.is_finite() {
        hue.rem_euclid(360.0)
    } else {
        0.0
    }
}

fn shift_hue(hue: f32, offset: f32) -> f32 {
    sanitize_hue(hue + offset)
}

fn mix_hue(from: f32, to: f32, amount: f32) -> f32 {
    let amount = clamp(amount, 0.0, 1.0);
    let delta = shortest_hue_delta(from, to);

    sanitize_hue(from + (delta * amount))
}

fn shortest_hue_delta(from: f32, to: f32) -> f32 {
    let delta = sanitize_hue(to) - sanitize_hue(from);

    if delta > 180.0 {
        delta - 360.0
    } else if delta < -180.0 {
        delta + 360.0
    } else {
        delta
    }
}

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use std::{hint::black_box, time::Instant};

    use chromasync_types::{ContrastStrategy, PaletteFamilyName};

    use super::{
        ChromaStrategy, ColorError, MIN_APCA_SCORE, MIN_CONTRAST_RATIO, OklabHue, Oklch,
        SAMPLE_TONES, ThemeMode, apca_contrast_score, chroma_curve, contrast_ratio, delta_e_ok,
        gamut_map, gamut_map_exact_boundary, generate_palette, hex_to_okhsl,
        meets_contrast_threshold, okhsl_to_hex, parse_seed_color, resolve_family_color,
        select_readable_color, select_readable_color_with_strategy,
    };

    #[test]
    fn parses_rrggbb_seed_colors() {
        let parsed = parse_seed_color("#ff6b6b").expect("seed should parse");

        assert_eq!(parsed.hex, "#FF6B6B");
        assert_eq!(parsed.rgb, [255, 107, 107]);
        assert!(parsed.lightness > 0.0);
        assert!(parsed.chroma > 0.0);
        assert!((0.0..360.0).contains(&parsed.hue));
    }

    #[test]
    fn rejects_non_rrggbb_seed_colors() {
        let error = parse_seed_color("#abc").expect_err("short hex should be rejected");

        assert!(matches!(error, ColorError::InvalidSeedFormat { .. }));
    }

    #[test]
    fn converts_hex_to_okhsl_reference_coordinates() {
        let color = hex_to_okhsl("#834941").expect("valid hex should convert");

        assert!((color.hue - 28.773_829).abs() < 0.000_1);
        assert!((color.saturation - 0.462_921_7).abs() < 0.000_1);
        assert!((color.lightness - 0.390_099_82).abs() < 0.000_1);
    }

    #[test]
    fn okhsl_round_trips_srgb_colors() {
        for hex in [
            "#000000", "#FFFFFF", "#808080", "#FF0000", "#00FF00", "#0000FF", "#4ECDC4", "#834941",
        ] {
            let color = hex_to_okhsl(hex).expect("valid hex should convert");
            let round_trip = okhsl_to_hex(color.hue, color.saturation, color.lightness)
                .expect("Okhsl should convert");

            assert_eq!(round_trip, hex, "round trip changed {hex}");
        }
    }

    #[test]
    fn okhsl_to_hex_validates_components_and_wraps_360_degrees() {
        assert_eq!(
            okhsl_to_hex(360.0, 0.5, 0.5).expect("360 degrees should wrap"),
            okhsl_to_hex(0.0, 0.5, 0.5).expect("zero degrees should convert")
        );

        for result in [
            okhsl_to_hex(f32::NAN, 0.5, 0.5),
            okhsl_to_hex(0.0, -0.01, 0.5),
            okhsl_to_hex(0.0, 0.5, 1.01),
        ] {
            assert!(matches!(
                result,
                Err(ColorError::InvalidOkhslComponent { .. })
            ));
        }
    }

    #[test]
    fn chroma_curve_peaks_at_midtones() {
        let dark = chroma_curve(0.1).expect("dark tone should be valid");
        let middle = chroma_curve(0.5).expect("midtone should be valid");
        let light = chroma_curve(0.9).expect("light tone should be valid");

        assert!(middle > dark);
        assert!(middle > light);
        assert!((dark - light).abs() < 0.0001);
    }

    #[test]
    fn local_minde_gamut_mapping_returns_clipped_srgb_color() {
        let color = Oklch::new(0.62, 1.0, OklabHue::from_degrees(32.0));
        assert!(!super::is_displayable(color));

        let mapped = gamut_map(color);

        assert!(mapped.chroma < color.chroma);
        assert!((mapped.l - 0.627_955_4).abs() < 0.000_2);
        assert!((mapped.chroma - 0.257_683_3).abs() < 0.000_2);
        assert!((mapped.hue.into_positive_degrees() - 29.233_88).abs() < 0.01);
        assert_eq!(super::oklch_to_hex(mapped), "#FF0000");
    }

    #[test]
    fn local_minde_matches_color_js_reference_vectors() {
        // colorjs.io 0.6.0 `toGamut({ space: "srgb", method: "css" })`,
        // which implements the CSS Color 4 sample algorithm.
        let vectors = [
            (
                [0.96476, 0.24503, 110.23],
                [0.966_684_76, 0.211_008_34, 109.976_92],
            ),
            ([0.7, 0.4, 40.0], [0.683_260_44, 0.212_333_07, 40.489_47]),
            ([0.5, 0.4, 264.0], [0.485_361_96, 0.290_735_48, 264.115_02]),
            ([0.8, 0.3, 150.0], [0.809_138_6, 0.237_875_42, 147.405_82]),
            ([0.2, 0.3, 300.0], [0.213_486_18, 0.115_525_21, 297.562_93]),
        ];

        for ([lightness, chroma, hue], expected) in vectors {
            let mapped = gamut_map(Oklch::new(lightness, chroma, OklabHue::from_degrees(hue)));
            let actual = [mapped.l, mapped.chroma, mapped.hue.into_positive_degrees()];

            for (actual, expected) in actual.into_iter().zip(expected) {
                assert!(
                    (actual - expected).abs() < 0.001,
                    "expected {expected}, got {actual} for OkLCh({lightness}, {chroma}, {hue})"
                );
            }
        }
    }

    #[test]
    fn gamut_mapping_leaves_in_gamut_colors_unchanged() {
        let color = Oklch::new(0.62, 0.04, OklabHue::from_degrees(210.0));

        assert!(super::is_displayable(color));
        assert_eq!(gamut_map(color), color);
    }

    #[test]
    fn gamut_mapping_uses_css_black_and_white_endpoints() {
        for lightness in [1.0, 1.2, f32::INFINITY] {
            let mapped = gamut_map(Oklch::new(lightness, 0.4, OklabHue::from_degrees(123.0)));

            assert_eq!(mapped.l, 1.0);
            assert_eq!(mapped.chroma, 0.0);
            assert_eq!(super::oklch_to_hex(mapped), "#FFFFFF");
        }

        for lightness in [0.0, -0.2, f32::NEG_INFINITY, f32::NAN] {
            let mapped = gamut_map(Oklch::new(lightness, 0.4, OklabHue::from_degrees(123.0)));

            assert_eq!(mapped.l, 0.0);
            assert_eq!(mapped.chroma, 0.0);
            assert_eq!(super::oklch_to_hex(mapped), "#000000");
        }
    }

    #[test]
    fn gamut_mapping_sanitizes_invalid_chroma_and_hue() {
        let mapped = gamut_map(Oklch::new(
            0.5,
            f32::INFINITY,
            OklabHue::from_degrees(f32::NAN),
        ));

        assert!(mapped.l.is_finite());
        assert_eq!(mapped.chroma, 0.0);
        assert_eq!(mapped.hue.into_positive_degrees(), 0.0);
        assert_eq!(super::oklch_to_hex(mapped), "#636363");
    }

    #[test]
    #[ignore = "diagnostic A/B benchmark for gamut mapping strategies"]
    fn compare_local_minde_to_exact_boundary() {
        let mut colors = Vec::new();

        for hue in (0..360).step_by(5) {
            for lightness_step in 1..40 {
                for chroma_step in 1..=25 {
                    colors.push(Oklch::new(
                        lightness_step as f32 / 40.0,
                        chroma_step as f32 / 50.0,
                        OklabHue::from_degrees(hue as f32),
                    ));
                }
            }
        }

        run_gamut_ab("oklch_grid", &colors);

        let seeds = [
            "#000000", "#FFFFFF", "#808080", "#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF",
            "#FF00FF", "#FF6B6B", "#4ECDC4", "#264653", "#2A9D8F", "#E9C46A", "#F4A261", "#E76F51",
            "#5B4B8A", "#D7263D", "#1B998B", "#2E294E", "#F46036", "#C5D86D", "#111827", "#F5F7FA",
        ];
        let strategies = [
            ChromaStrategy::Subtle,
            ChromaStrategy::Normal,
            ChromaStrategy::Vibrant,
            ChromaStrategy::Muted,
            ChromaStrategy::Industrial,
        ];
        let mut palette_colors = Vec::new();

        for seed in seeds {
            for strategy in strategies {
                let palette = generate_palette(seed, ThemeMode::Dark, strategy)
                    .expect("A/B seed should generate");

                for family in palette.families.values() {
                    for tone in SAMPLE_TONES {
                        let lightness = f32::from(tone) / 100.0;
                        let chroma = family.base_chroma
                            * chroma_curve(lightness).expect("sample tone should be valid");

                        palette_colors.push(Oklch::new(
                            lightness,
                            chroma,
                            OklabHue::from_degrees(family.hue),
                        ));
                    }
                }
            }
        }

        run_gamut_ab("generated_palettes", &palette_colors);
    }

    fn run_gamut_ab(label: &str, colors: &[Oklch]) {
        const TIMING_PASSES: usize = 10;

        let out_of_gamut = colors
            .iter()
            .filter(|color| !super::is_displayable(**color))
            .count();

        let exact_started = Instant::now();
        for _ in 0..TIMING_PASSES {
            for color in colors {
                black_box(gamut_map_exact_boundary(black_box(*color)));
            }
        }
        let exact_elapsed = exact_started.elapsed();

        let local_started = Instant::now();
        for _ in 0..TIMING_PASSES {
            for color in colors {
                black_box(gamut_map(black_box(*color)));
            }
        }
        let local_elapsed = local_started.elapsed();

        let exact: Vec<_> = colors
            .iter()
            .map(|color| gamut_map_exact_boundary(*color))
            .collect();
        let local: Vec<_> = colors.iter().map(|color| gamut_map(*color)).collect();

        let mut affected = 0;
        let mut chroma_improved = 0;
        let mut chroma_reduced = 0;
        let mut chroma_gain = 0.0;
        let mut max_chroma_gain = 0.0_f32;
        let mut min_chroma_gain = 0.0_f32;
        let mut total_difference = 0.0;
        let mut max_difference = 0.0_f32;
        let mut max_lightness_shift = 0.0_f32;
        let mut max_hue_shift = 0.0_f32;
        let mut max_chromatic_hue_shift = 0.0_f32;

        for ((origin, exact), local) in colors.iter().zip(&exact).zip(&local) {
            let difference = delta_e_ok(*exact, *local);

            if difference > f32::EPSILON {
                affected += 1;
                let gain = local.chroma - exact.chroma;
                chroma_gain += gain;
                max_chroma_gain = max_chroma_gain.max(gain);
                min_chroma_gain = min_chroma_gain.min(gain);
                if gain > f32::EPSILON {
                    chroma_improved += 1;
                } else if gain < -f32::EPSILON {
                    chroma_reduced += 1;
                }
                total_difference += difference;
                max_difference = max_difference.max(difference);
                max_lightness_shift = max_lightness_shift.max((local.l - origin.l).abs());
                let hue_shift = hue_difference(
                    local.hue.into_positive_degrees(),
                    origin.hue.into_positive_degrees(),
                );
                max_hue_shift = max_hue_shift.max(hue_shift);
                if local.chroma >= 0.02 {
                    max_chromatic_hue_shift = max_chromatic_hue_shift.max(hue_shift);
                }
            }
        }

        eprintln!(
            "{label}: samples={} out_of_gamut={} affected={} affected_rate={:.2}% oog_affected_rate={:.2}% chroma_improved={} chroma_reduced={} avg_chroma_gain={:.6} min_chroma_gain={:.6} max_chroma_gain={:.6} avg_delta_e_ok={:.6} max_delta_e_ok={:.6} max_lightness_shift={:.6} max_hue_shift_deg={:.3} max_hue_shift_deg_at_c_ge_0.02={:.3} exact_ms_per_pass={:.3} local_minde_ms_per_pass={:.3}",
            colors.len(),
            out_of_gamut,
            affected,
            affected as f64 * 100.0 / colors.len() as f64,
            affected as f64 * 100.0 / out_of_gamut as f64,
            chroma_improved,
            chroma_reduced,
            chroma_gain / affected as f32,
            min_chroma_gain,
            max_chroma_gain,
            total_difference / affected as f32,
            max_difference,
            max_lightness_shift,
            max_hue_shift,
            max_chromatic_hue_shift,
            exact_elapsed.as_secs_f64() * 1_000.0 / TIMING_PASSES as f64,
            local_elapsed.as_secs_f64() * 1_000.0 / TIMING_PASSES as f64,
        );

        assert!(affected > 0);
    }

    fn hue_difference(left: f32, right: f32) -> f32 {
        let difference = (left - right).abs().rem_euclid(360.0);

        difference.min(360.0 - difference)
    }

    #[test]
    fn contrast_selection_prefers_readable_candidates() {
        let background = "#111827".to_owned();
        let candidates = vec!["#A3A3A3".to_owned(), "#F9FAFB".to_owned()];

        let selected = select_readable_color(&background, &candidates)
            .expect("a readable candidate should be selected");

        assert_eq!(selected.hex, "#F9FAFB");
        assert!(selected.score >= MIN_CONTRAST_RATIO);
    }

    #[test]
    fn apca_experimental_prefers_high_contrast_candidates() {
        let background = "#F5F7FA".to_owned();
        let candidates = vec!["#4ECDC4".to_owned(), "#111827".to_owned()];

        let selected = select_readable_color_with_strategy(
            &background,
            &candidates,
            ContrastStrategy::ApcaExperimental,
        )
        .expect("a readable candidate should be selected");

        assert_eq!(selected.hex, "#111827");
        assert!(selected.score >= MIN_APCA_SCORE);
    }

    #[test]
    fn apca_experimental_preserves_reverse_polarity_when_selecting() {
        let background = "#111827".to_owned();
        let candidates = vec!["#4ECDC4".to_owned(), "#F5F7FA".to_owned()];

        let selected = select_readable_color_with_strategy(
            &background,
            &candidates,
            ContrastStrategy::ApcaExperimental,
        )
        .expect("a readable candidate should be selected");

        assert_eq!(selected.hex, "#F5F7FA");
        assert!(selected.score <= -MIN_APCA_SCORE);
        assert!(meets_contrast_threshold(
            selected.score,
            ContrastStrategy::ApcaExperimental
        ));
    }

    #[test]
    fn apca_threshold_compares_lc_magnitude() {
        assert!(meets_contrast_threshold(
            MIN_APCA_SCORE,
            ContrastStrategy::ApcaExperimental
        ));
        assert!(meets_contrast_threshold(
            -MIN_APCA_SCORE,
            ContrastStrategy::ApcaExperimental
        ));
        assert!(!meets_contrast_threshold(
            -(MIN_APCA_SCORE - 0.01),
            ContrastStrategy::ApcaExperimental
        ));
    }

    #[test]
    fn generated_palette_contains_all_families_and_sample_tones() {
        let palette = generate_palette("#ff6b6b", ThemeMode::Dark, ChromaStrategy::Normal)
            .expect("palette should generate");

        assert_eq!(palette.families.len(), PaletteFamilyName::ALL.len());

        for family_name in PaletteFamilyName::ALL {
            let family = palette
                .families
                .get(&family_name)
                .expect("all palette families should be present");
            assert_eq!(family.tones.len(), SAMPLE_TONES.len());
        }
    }

    #[test]
    fn palette_generation_is_deterministic() {
        let left = generate_palette("#ff6b6b", ThemeMode::Dark, ChromaStrategy::Normal)
            .expect("palette should generate");
        let right = generate_palette("#ff6b6b", ThemeMode::Dark, ChromaStrategy::Normal)
            .expect("palette should generate");

        assert_eq!(left, right);
    }

    #[test]
    fn default_text_candidates_meet_contrast_heuristic_for_both_modes() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let palette = generate_palette("#4ecdc4", mode, ChromaStrategy::Normal)
                .expect("palette should generate");
            let neutral = palette
                .families
                .get(&PaletteFamilyName::Neutral)
                .expect("neutral family should be present");
            let background =
                resolve_family_color(neutral, f32::from(mode.default_background_tone()) / 100.0)
                    .expect("background tone should resolve");
            let preferred_text =
                resolve_family_color(neutral, f32::from(mode.default_text_tone()) / 100.0)
                    .expect("text tone should resolve");
            let alternate_text =
                resolve_family_color(neutral, if mode == ThemeMode::Dark { 0.98 } else { 0.06 })
                    .expect("alternate text tone should resolve");
            let selection = select_readable_color(
                &background,
                &[preferred_text.clone(), alternate_text.clone()],
            )
            .expect("text should be selected");

            assert!(selection.score >= MIN_CONTRAST_RATIO);
        }
    }

    #[test]
    fn contrast_ratio_is_symmetric() {
        let left = contrast_ratio("#FFFFFF", "#111827").expect("contrast should compute");
        let right = contrast_ratio("#111827", "#FFFFFF").expect("contrast should compute");

        assert!((left - right).abs() < 0.0001);
    }

    #[test]
    fn apca_contrast_score_matches_official_reference_vectors() {
        // Generated with apca-w3 0.1.9, which implements the 0.0.98G-4g
        // constants used by the core algorithm.
        let vectors = [
            ("#000000", "#FFFFFF", 106.040_67),
            ("#FFFFFF", "#000000", -107.884_735),
            ("#777777", "#FFFFFF", 71.111_1),
            ("#FFFFFF", "#777777", -76.581_95),
            ("#123456", "#ABCDEF", 67.495_81),
            ("#ABCDEF", "#123456", -68.094_25),
        ];

        for (foreground, background, expected) in vectors {
            let actual =
                apca_contrast_score(foreground, background).expect("contrast should compute");

            assert!(
                (actual - expected).abs() < 0.001,
                "expected {foreground} on {background} to be {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn apca_contrast_score_applies_black_soft_clamp_and_low_clip() {
        let near_black =
            apca_contrast_score("#000000", "#444444").expect("contrast should compute");
        let low_normal =
            apca_contrast_score("#888888", "#999999").expect("contrast should compute");
        let low_reverse =
            apca_contrast_score("#999999", "#888888").expect("contrast should compute");

        assert!((near_black - 11.333_981).abs() < 0.001);
        assert_eq!(low_normal, 0.0);
        assert!((low_reverse - -7.849_647_5).abs() < 0.001);
    }

    #[test]
    fn apca_contrast_score_rewards_stronger_separation_by_magnitude() {
        let high = apca_contrast_score("#111827", "#F5F7FA").expect("contrast should compute");
        let low = apca_contrast_score("#7C8794", "#F5F7FA").expect("contrast should compute");
        let reverse = apca_contrast_score("#F5F7FA", "#111827").expect("contrast should compute");

        assert!(high > low);
        assert!(high >= MIN_APCA_SCORE);
        assert!(reverse < 0.0);
        assert!(reverse.abs() >= MIN_APCA_SCORE);
        assert!(meets_contrast_threshold(
            reverse,
            ContrastStrategy::ApcaExperimental
        ));
    }

    #[test]
    fn resolves_primary_sample_to_valid_hex() {
        let palette = generate_palette("#ff6b6b", ThemeMode::Dark, ChromaStrategy::Normal)
            .expect("palette should generate");
        let primary = palette
            .families
            .get(&PaletteFamilyName::Primary)
            .expect("primary family should exist");
        let accent = resolve_family_color(primary, 0.7).expect("primary tone should resolve");

        assert_eq!(accent.len(), 7);
        assert!(accent.starts_with('#'));
    }

    #[test]
    fn regression_fixture_matches_ff6b6b_dark() {
        assert_regression_fixture(
            "#ff6b6b",
            ThemeMode::Dark,
            include_str!("../tests/fixtures/seed_ff6b6b_dark.json"),
        );
    }

    #[test]
    fn regression_fixture_matches_4ecdc4_dark() {
        assert_regression_fixture(
            "#4ecdc4",
            ThemeMode::Dark,
            include_str!("../tests/fixtures/seed_4ecdc4_dark.json"),
        );
    }

    fn assert_regression_fixture(seed: &str, mode: ThemeMode, expected: &str) {
        let palette =
            generate_palette(seed, mode, ChromaStrategy::Normal).expect("palette should generate");
        let actual = serde_json::to_string_pretty(&palette).expect("palette should serialize");

        assert_eq!(actual, expected.trim());
    }
}
