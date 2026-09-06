use std::collections::HashMap;

use ratatui::style::Color;

use crate::domain::{CategoryId, DRIFT_CATEGORY_ID};

/// Debug-selectable reduction from the up-to-eight independently colored sand
/// dots inside one terminal Braille cell to the single foreground color that
/// the terminal can actually display. Production `SandEngine::render` remains
/// exactly on `Rgb`, the pre-VISUAL-001 behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrailleColorBlend {
    Rgb,
    Linear,
    Oklab,
    Dominant,
    DominantSoft,
}

impl BrailleColorBlend {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Rgb => "rgb",
            Self::Linear => "linear",
            Self::Oklab => "oklab",
            Self::Dominant => "dominant",
            Self::DominantSoft => "dominant-soft",
        }
    }

    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "rgb" => Some(Self::Rgb),
            "linear" => Some(Self::Linear),
            "oklab" => Some(Self::Oklab),
            "dominant" => Some(Self::Dominant),
            "dominant-soft" => Some(Self::DominantSoft),
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

pub(super) fn blend_braille_color(
    counts: &HashMap<CategoryId, usize>,
    category_colors: &HashMap<CategoryId, Color>,
    profile: BrailleColorBlend,
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
            // Preserve the exact pre-VISUAL-001 arithmetic control, including
            // f32 weighting and truncation.
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
            let mut best_id = None;
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
            let (r, g, b) = category_rgb(best_id.expect("colored category exists"), category_colors);
            Color::Rgb(r, g, b)
        }
        BrailleColorBlend::DominantSoft => {
            // Smooth majority emphasis without a hand-tuned fixed blend ratio:
            // squaring each category's dot count preserves exact ties while
            // progressively suppressing minority influence as one material wins.
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
                        blend_braille_color(&counts, &colors, BrailleColorBlend::Rgb),
                        legacy_rgb_reference(&counts, &colors),
                        "counts=({first},{second},{third})"
                    );
                }
            }
        }
    }
}
