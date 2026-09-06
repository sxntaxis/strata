use std::collections::HashMap;

use ratatui::style::Color;

use crate::domain::{CategoryId, DRIFT_CATEGORY_ID};

/// Debug-selectable reduction from the up-to-eight independently colored sand
/// dots inside one terminal Braille cell to the single foreground color that
/// the terminal can actually display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrailleColorBlend {
    Rgb,
    RgbAdditive,
    RgbLuma,
    RgbLumaSafe,
    RgbMid,
    RgbContrast,
    Linear,
    Oklab,
    Dominant,
    DominantSoft,
}

impl BrailleColorBlend {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Rgb => "rgb",
            Self::RgbAdditive => "rgb-additive",
            Self::RgbLuma => "rgb-luma",
            Self::RgbLumaSafe => "rgb-luma-safe",
            Self::RgbMid => "rgb-mid",
            Self::RgbContrast => "rgb-contrast",
            Self::Linear => "linear",
            Self::Oklab => "oklab",
            Self::Dominant => "dominant",
            Self::DominantSoft => "dominant-soft",
        }
    }

    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "rgb" => Some(Self::Rgb),
            "rgb-additive" => Some(Self::RgbAdditive),
            "rgb-luma" => Some(Self::RgbLuma),
            "rgb-luma-safe" => Some(Self::RgbLumaSafe),
            "rgb-mid" => Some(Self::RgbMid),
            "rgb-contrast" => Some(Self::RgbContrast),
            "linear" => Some(Self::Linear),
            "oklab" => Some(Self::Oklab),
            "dominant" => Some(Self::Dominant),
            "dominant-soft" => Some(Self::DominantSoft),
            _ => None,
        }
    }
}

/// Render-only preview policy for the terminal background. It is deliberately
/// separate from color-mixing semantics so the owner can compare the same
/// `rgb-contrast` rule against dark/light terminal assumptions. No terminal I/O
/// or persistence is introduced by this review pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrailleColorBackground {
    Neutral,
    Dark,
    Light,
}

impl BrailleColorBackground {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "neutral" => Some(Self::Neutral),
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }
}

fn category_rgb(
    category_id: CategoryId,
    category_colors: &HashMap<CategoryId, Color>,
) -> (u8, u8, u8) {
    if category_id == DRIFT_CATEGORY_ID {
        return (255, 255, 255);
    }
    match category_colors
        .get(&category_id)
        .copied()
        .unwrap_or(Color::White)
    {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (255, 255, 255),
    }
}

fn srgb_channel_to_linear(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_channel_to_srgb(channel: f64) -> u8 {
    let value = channel.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn linear_rgb_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;
    let l = l.cbrt();
    let m = m.cbrt();
    let s = s.cbrt();
    (
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    )
}

fn oklab_to_linear_rgb(lab_l: f64, lab_a: f64, lab_b: f64) -> (f64, f64, f64) {
    let l = (lab_l + 0.396_337_78 * lab_a + 0.215_803_76 * lab_b).powi(3);
    let m = (lab_l - 0.105_561_346 * lab_a - 0.063_854_17 * lab_b).powi(3);
    let s = (lab_l - 0.089_484_18 * lab_a - 1.291_485_5 * lab_b).powi(3);
    (
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    )
}

fn legacy_rgb_mean(
    counts: &HashMap<CategoryId, usize>,
    category_colors: &HashMap<CategoryId, Color>,
    total_colored_dots: usize,
) -> (u8, u8, u8) {
    let mut blended_r = 0f32;
    let mut blended_g = 0f32;
    let mut blended_b = 0f32;
    for (category_id, count) in counts {
        let (r, g, b) = category_rgb(*category_id, category_colors);
        let weight = *count as f32 / total_colored_dots as f32;
        blended_r += f32::from(r) * weight;
        blended_g += f32::from(g) * weight;
        blended_b += f32::from(b) * weight;
    }
    (blended_r as u8, blended_g as u8, blended_b as u8)
}

fn encoded_rgb_chroma(r: u8, g: u8, b: u8) -> f64 {
    let maximum = r.max(g).max(b);
    let minimum = r.min(g).min(b);
    f64::from(maximum - minimum) / 255.0
}

fn cancellation_amount(
    counts: &HashMap<CategoryId, usize>,
    category_colors: &HashMap<CategoryId, Color>,
    total_colored_dots: usize,
    base: (u8, u8, u8),
) -> f64 {
    let mut source_chroma = 0.0f64;
    for (category_id, count) in counts {
        let (r, g, b) = category_rgb(*category_id, category_colors);
        let weight = *count as f64 / total_colored_dots as f64;
        source_chroma += encoded_rgb_chroma(r, g, b) * weight;
    }
    if source_chroma <= f64::EPSILON {
        return 0.0;
    }
    let mixed_chroma = encoded_rgb_chroma(base.0, base.1, base.2);
    (1.0 - mixed_chroma / source_chroma).clamp(0.0, 1.0)
}

fn weighted_source_oklab_lightness(
    counts: &HashMap<CategoryId, usize>,
    category_colors: &HashMap<CategoryId, Color>,
    total_colored_dots: usize,
) -> f64 {
    counts
        .iter()
        .map(|(category_id, count)| {
            let (r, g, b) = category_rgb(*category_id, category_colors);
            let (l, _, _) = linear_rgb_to_oklab(
                srgb_channel_to_linear(r),
                srgb_channel_to_linear(g),
                srgb_channel_to_linear(b),
            );
            l * (*count as f64 / total_colored_dots as f64)
        })
        .sum::<f64>()
        .clamp(0.0, 1.0)
}

fn neutral_rgb_at_oklab_lightness(lightness: f64) -> (u8, u8, u8) {
    let (r, g, b) = oklab_to_linear_rgb(lightness.clamp(0.0, 1.0), 0.0, 0.0);
    (
        linear_channel_to_srgb(r),
        linear_channel_to_srgb(g),
        linear_channel_to_srgb(b),
    )
}

fn blend_rgb_toward(base: (u8, u8, u8), target: (u8, u8, u8), amount: f64) -> Color {
    let blend = |from: u8, to: u8| {
        (f64::from(from) + (f64::from(to) - f64::from(from)) * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::Rgb(
        blend(base.0, target.0),
        blend(base.1, target.1),
        blend(base.2, target.2),
    )
}

fn cancellation_neutral_blend(
    counts: &HashMap<CategoryId, usize>,
    category_colors: &HashMap<CategoryId, Color>,
    total_colored_dots: usize,
    target_lightness: impl FnOnce(f64) -> f64,
) -> Color {
    let base = legacy_rgb_mean(counts, category_colors, total_colored_dots);
    let cancellation = cancellation_amount(counts, category_colors, total_colored_dots, base);
    if cancellation <= f64::EPSILON {
        return Color::Rgb(base.0, base.1, base.2);
    }
    let source_l = weighted_source_oklab_lightness(counts, category_colors, total_colored_dots);
    let target = neutral_rgb_at_oklab_lightness(target_lightness(source_l));
    blend_rgb_toward(base, target, cancellation * cancellation)
}

fn rgb_additive_cancellation_blend(
    counts: &HashMap<CategoryId, usize>,
    category_colors: &HashMap<CategoryId, Color>,
    total_colored_dots: usize,
) -> Color {
    let base = legacy_rgb_mean(counts, category_colors, total_colored_dots);
    let cancellation = cancellation_amount(counts, category_colors, total_colored_dots, base);
    if cancellation <= f64::EPSILON {
        return Color::Rgb(base.0, base.1, base.2);
    }
    blend_rgb_toward(base, (255, 255, 255), cancellation * cancellation)
}

fn rgb_contrast_target(source_l: f64, background: BrailleColorBackground) -> f64 {
    const SAFE_LOW: f64 = 0.42;
    const SAFE_HIGH: f64 = 0.72;
    let safe = source_l.clamp(SAFE_LOW, SAFE_HIGH);
    match background {
        BrailleColorBackground::Neutral => 0.57,
        BrailleColorBackground::Dark => safe.max(0.64),
        BrailleColorBackground::Light => safe.min(0.50),
    }
}

pub(super) fn blend_braille_color(
    counts: &HashMap<CategoryId, usize>,
    category_colors: &HashMap<CategoryId, Color>,
    profile: BrailleColorBlend,
    background: BrailleColorBackground,
) -> Color {
    let total_colored_dots: usize = counts.values().sum();
    if total_colored_dots == 0 {
        return Color::White;
    }
    if counts.len() == 1 {
        let category_id = *counts.keys().next().expect("single category exists");
        let (r, g, b) = category_rgb(category_id, category_colors);
        return Color::Rgb(r, g, b);
    }

    match profile {
        BrailleColorBlend::Rgb => {
            let (r, g, b) = legacy_rgb_mean(counts, category_colors, total_colored_dots);
            Color::Rgb(r, g, b)
        }
        BrailleColorBlend::RgbAdditive => {
            rgb_additive_cancellation_blend(counts, category_colors, total_colored_dots)
        }
        BrailleColorBlend::RgbLuma => cancellation_neutral_blend(
            counts,
            category_colors,
            total_colored_dots,
            |source_l| source_l,
        ),
        BrailleColorBlend::RgbLumaSafe => cancellation_neutral_blend(
            counts,
            category_colors,
            total_colored_dots,
            |source_l| source_l.clamp(0.42, 0.72),
        ),
        BrailleColorBlend::RgbMid => cancellation_neutral_blend(
            counts,
            category_colors,
            total_colored_dots,
            |_| 0.57,
        ),
        BrailleColorBlend::RgbContrast => cancellation_neutral_blend(
            counts,
            category_colors,
            total_colored_dots,
            |source_l| rgb_contrast_target(source_l, background),
        ),
        BrailleColorBlend::Linear => {
            let mut r = 0f64;
            let mut g = 0f64;
            let mut b = 0f64;
            for (category_id, count) in counts {
                let (sr, sg, sb) = category_rgb(*category_id, category_colors);
                let weight = *count as f64 / total_colored_dots as f64;
                r += srgb_channel_to_linear(sr) * weight;
                g += srgb_channel_to_linear(sg) * weight;
                b += srgb_channel_to_linear(sb) * weight;
            }
            Color::Rgb(
                linear_channel_to_srgb(r),
                linear_channel_to_srgb(g),
                linear_channel_to_srgb(b),
            )
        }
        BrailleColorBlend::Oklab => {
            let mut lab_l = 0f64;
            let mut lab_a = 0f64;
            let mut lab_b = 0f64;
            for (category_id, count) in counts {
                let (sr, sg, sb) = category_rgb(*category_id, category_colors);
                let (l, a, b) = linear_rgb_to_oklab(
                    srgb_channel_to_linear(sr),
                    srgb_channel_to_linear(sg),
                    srgb_channel_to_linear(sb),
                );
                let weight = *count as f64 / total_colored_dots as f64;
                lab_l += l * weight;
                lab_a += a * weight;
                lab_b += b * weight;
            }
            let (r, g, b) = oklab_to_linear_rgb(lab_l, lab_a, lab_b);
            Color::Rgb(
                linear_channel_to_srgb(r),
                linear_channel_to_srgb(g),
                linear_channel_to_srgb(b),
            )
        }
        BrailleColorBlend::Dominant => {
            let mut best_id: Option<CategoryId> = None;
            let mut best_count = 0usize;
            for (category_id, count) in counts {
                if *count > best_count
                    || (*count == best_count
                        && match best_id {
                            Some(best) => category_id.0 < best.0,
                            None => true,
                        })
                {
                    best_id = Some(*category_id);
                    best_count = *count;
                }
            }
            let (r, g, b) =
                category_rgb(best_id.expect("colored category exists"), category_colors);
            Color::Rgb(r, g, b)
        }
        BrailleColorBlend::DominantSoft => {
            let total_weight = counts
                .values()
                .map(|count| count.saturating_mul(*count))
                .sum::<usize>()
                .max(1);
            let mut blended_r = 0f32;
            let mut blended_g = 0f32;
            let mut blended_b = 0f32;
            for (category_id, count) in counts {
                let (r, g, b) = category_rgb(*category_id, category_colors);
                let squared = count.saturating_mul(*count);
                let weight = squared as f32 / total_weight as f32;
                blended_r += f32::from(r) * weight;
                blended_g += f32::from(g) * weight;
                blended_b += f32::from(b) * weight;
            }
            Color::Rgb(blended_r as u8, blended_g as u8, blended_b as u8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_rgb_reference(
        counts: &HashMap<CategoryId, usize>,
        category_colors: &HashMap<CategoryId, Color>,
    ) -> Color {
        let total_colored_dots: usize = counts.values().sum();
        if total_colored_dots == 0 {
            return Color::White;
        }
        let mut blended_r = 0f32;
        let mut blended_g = 0f32;
        let mut blended_b = 0f32;
        for (category_id, count) in counts {
            let (r, g, b) = category_rgb(*category_id, category_colors);
            let weight = *count as f32 / total_colored_dots as f32;
            blended_r += f32::from(r) * weight;
            blended_g += f32::from(g) * weight;
            blended_b += f32::from(b) * weight;
        }
        Color::Rgb(blended_r as u8, blended_g as u8, blended_b as u8)
    }

    #[test]
    fn rgb_profile_matches_legacy_weighted_srgb_reference_for_all_eight_dot_compositions() {
        let ids = [CategoryId(1), CategoryId(2), CategoryId(3)];
        let colors = HashMap::from([
            (ids[0], Color::Rgb(17, 203, 91)),
            (ids[1], Color::Rgb(241, 37, 119)),
            (ids[2], Color::Rgb(83, 149, 229)),
        ]);

        for first in 0..=8usize {
            for second in 0..=8usize.saturating_sub(first) {
                for third in 0..=8usize.saturating_sub(first + second) {
                    let mut counts = HashMap::new();
                    if first > 0 {
                        counts.insert(ids[0], first);
                    }
                    if second > 0 {
                        counts.insert(ids[1], second);
                    }
                    if third > 0 {
                        counts.insert(ids[2], third);
                    }
                    assert_eq!(
                        blend_braille_color(&counts, &colors, BrailleColorBlend::Rgb, BrailleColorBackground::Neutral),
                        legacy_rgb_reference(&counts, &colors),
                        "counts=({first},{second},{third})"
                    );
                }
            }
        }
    }
}
