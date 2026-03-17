use std::collections::BTreeMap;

use chromasync_types::{
    ContrastStrategy, GeneratedPalette, HexColor, PaletteFamily, PaletteFamilyName, ThemeMode,
    ToneSample,
};
use palette::{FromColor, LinSrgb, OklabHue, Oklch, Srgb, convert::FromColorUnclamped};
use thiserror::Error;

pub const MIN_CONTRAST_RATIO: f32 = 4.5;
pub const MIN_APCA_SCORE: f32 = 60.0;
pub const SAMPLE_TONES: [u8; 16] = [
    0, 6, 10, 14, 20, 30, 40, 45, 50, 60, 70, 80, 90, 94, 98, 100,
];

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSeedColor {
    pub hex: HexColor,
    pub rgb: [u8; 3],
    pub lightness: f32,
    pub chroma: f32,
    pub hue: f32,
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
    #[error("at least one foreground candidate is required")]
    MissingContrastCandidates,
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

pub fn generate_palette(seed: &str, mode: ThemeMode) -> Result<GeneratedPalette, ColorError> {
    let parsed_seed = parse_seed_color(seed)?;
    let mut families = BTreeMap::new();

    for spec in derive_family_specs(&parsed_seed) {
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

    Ok(0.18 + (bell * 0.82))
}

pub fn contrast_ratio(foreground: &str, background: &str) -> Result<f32, ColorError> {
    let foreground_luminance = relative_luminance(parse_rgb_hex(foreground, false)?);
    let background_luminance = relative_luminance(parse_rgb_hex(background, false)?);
    let lighter = foreground_luminance.max(background_luminance);
    let darker = foreground_luminance.min(background_luminance);

    Ok((lighter + 0.05) / (darker + 0.05))
}

pub fn apca_contrast_score(foreground: &str, background: &str) -> Result<f32, ColorError> {
    let foreground_luminance = apca_luminance(parse_rgb_hex(foreground, false)?);
    let background_luminance = apca_luminance(parse_rgb_hex(background, false)?);
    let delta = background_luminance - foreground_luminance;

    if delta.abs() < 0.0005 {
        return Ok(0.0);
    }

    let score = if delta.is_sign_positive() {
        (background_luminance.powf(0.56) - foreground_luminance.powf(0.57)) * 1.14 * 100.0
    } else {
        (background_luminance.powf(0.65) - foreground_luminance.powf(0.62)) * 1.14 * 100.0
    };

    Ok(score.abs())
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
    score >= minimum_contrast_score(strategy)
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
                    || (selection_meets == current_meets && selection.score > current.score)
            }
            None => true,
        };

        if replace {
            best = Some(selection);
        }
    }

    best.ok_or(ColorError::MissingContrastCandidates)
}

pub fn gamut_map(color: Oklch) -> Oklch {
    let lightness = color.l.clamp(0.0, 1.0);
    let hue = sanitize_hue(color.hue.into_positive_degrees());
    let target = Oklch::new(
        lightness,
        color.chroma.max(0.0),
        OklabHue::from_degrees(hue),
    );

    if is_displayable(target) {
        return target;
    }

    let mut low = 0.0;
    let mut high = target.chroma.max(0.0);

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

fn derive_family_specs(seed: &ParsedSeedColor) -> [FamilySpec; 9] {
    let primary_chroma = clamp(seed.chroma, 0.06, 0.24);
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
            base_chroma: clamp(primary_chroma * 0.72, 0.045, 0.18),
        },
        FamilySpec {
            name: PaletteFamilyName::Tertiary,
            hue: shift_hue(seed_hue, 72.0),
            base_chroma: clamp(primary_chroma * 0.82, 0.055, 0.2),
        },
        FamilySpec {
            name: PaletteFamilyName::Neutral,
            hue: seed_hue,
            base_chroma: clamp(primary_chroma * 0.12, 0.008, 0.03),
        },
        FamilySpec {
            name: PaletteFamilyName::NeutralVariant,
            hue: shift_hue(seed_hue, 12.0),
            base_chroma: clamp(primary_chroma * 0.22, 0.014, 0.05),
        },
        FamilySpec {
            name: PaletteFamilyName::Error,
            hue: mix_hue(seed_hue, 25.0, 0.85),
            base_chroma: clamp(primary_chroma * 0.95, 0.14, 0.22),
        },
        FamilySpec {
            name: PaletteFamilyName::Success,
            hue: mix_hue(seed_hue, 145.0, 0.85),
            base_chroma: clamp(primary_chroma * 0.85, 0.12, 0.2),
        },
        FamilySpec {
            name: PaletteFamilyName::Warning,
            hue: mix_hue(seed_hue, 95.0, 0.85),
            base_chroma: clamp(primary_chroma, 0.16, 0.24),
        },
        FamilySpec {
            name: PaletteFamilyName::Info,
            hue: mix_hue(seed_hue, 230.0, 0.85),
            base_chroma: clamp(primary_chroma * 0.78, 0.1, 0.18),
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

fn apca_luminance(rgb: [u8; 3]) -> f32 {
    let [red, green, blue] = rgb.map(|channel| {
        let normalized = f32::from(channel) / 255.0;

        normalized.powf(2.4)
    });

    (0.2126729 * red) + (0.7151522 * green) + (0.072175 * blue)
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

fn validate_tone(tone: f32) -> Result<(), ColorError> {
    if tone.is_finite() && (0.0..=1.0).contains(&tone) {
        Ok(())
    } else {
        Err(ColorError::InvalidTone { tone })
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
    use chromasync_types::{ContrastStrategy, PaletteFamilyName};

    use super::{
        ColorError, MIN_APCA_SCORE, MIN_CONTRAST_RATIO, OklabHue, Oklch, SAMPLE_TONES, ThemeMode,
        apca_contrast_score, chroma_curve, contrast_ratio, gamut_map, generate_palette,
        parse_seed_color, resolve_family_color, select_readable_color,
        select_readable_color_with_strategy,
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
    fn chroma_curve_peaks_at_midtones() {
        let dark = chroma_curve(0.1).expect("dark tone should be valid");
        let middle = chroma_curve(0.5).expect("midtone should be valid");
        let light = chroma_curve(0.9).expect("light tone should be valid");

        assert!(middle > dark);
        assert!(middle > light);
        assert!((dark - light).abs() < 0.0001);
    }

    #[test]
    fn gamut_mapping_preserves_hue_and_lightness_while_reducing_chroma() {
        let color = Oklch::new(0.62, 1.0, OklabHue::from_degrees(32.0));
        assert!(!super::is_displayable(color));

        let mapped = gamut_map(color);

        assert!((mapped.l - color.l).abs() < 0.0001);
        assert!(
            (mapped.hue.into_positive_degrees() - color.hue.into_positive_degrees()).abs() < 0.0001
        );
        assert!(mapped.chroma < color.chroma);
        let mapped_hex = super::oklch_to_hex(mapped);
        assert!(mapped_hex.starts_with('#'));
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
    fn generated_palette_contains_all_families_and_sample_tones() {
        let palette =
            generate_palette("#ff6b6b", ThemeMode::Dark).expect("palette should generate");

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
        let left = generate_palette("#ff6b6b", ThemeMode::Dark).expect("palette should generate");
        let right = generate_palette("#ff6b6b", ThemeMode::Dark).expect("palette should generate");

        assert_eq!(left, right);
    }

    #[test]
    fn default_text_candidates_meet_contrast_heuristic_for_both_modes() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let palette = generate_palette("#4ecdc4", mode).expect("palette should generate");
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
    fn apca_contrast_score_rewards_stronger_separation() {
        let high = apca_contrast_score("#111827", "#F5F7FA").expect("contrast should compute");
        let low = apca_contrast_score("#7C8794", "#F5F7FA").expect("contrast should compute");

        assert!(high > low);
        assert!(high >= MIN_APCA_SCORE);
    }

    #[test]
    fn resolves_primary_sample_to_valid_hex() {
        let palette =
            generate_palette("#ff6b6b", ThemeMode::Dark).expect("palette should generate");
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
        let palette = generate_palette(seed, mode).expect("palette should generate");
        let actual = serde_json::to_string_pretty(&palette).expect("palette should serialize");

        assert_eq!(actual, expected.trim());
    }
}
