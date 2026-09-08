use std::collections::HashMap;
use std::fmt::Write as _;

use crate::constants::TIME_SETTINGS;
use crate::domain::{Category, CategoryId};

use super::{ClassicRainMode, ClassicSandboxEngine, RAIN_FOCUS_BIAS_PROBABILITY};
#[cfg(test)]
use super::{RainBiasScheduleProbe, ReposeStabilityProbe};

const MILLIS_PER_SECOND: f64 = 1_000.0;
const SECONDS_PER_HOUR: f64 = 3_600.0;

#[derive(Debug, Clone)]
struct CategoryProfile {
    category_id: CategoryId,
    name: String,
    mass: usize,
    mean_y: f64,
    centroid_x_norm: f64,
    mean_thickness: f64,
    thickness_stddev: f64,
    thickness_cv: f64,
    thickness: Vec<f64>,
}

#[derive(Debug, Clone, Copy)]
struct RainDistributionMetrics {
    free_sites: usize,
    kl_bits_per_grain: f64,
    focus_rms_width_dots: f64,
    focus_rms_width_fraction: f64,
    positive_excess_per_ingress: f64,
    positive_excess_effective_width_dots: f64,
    one_dot_excess_mound_grains: f64,
}

impl ClassicSandboxEngine {
    /// Read-only RAIN-005A diagnostic report.
    ///
    /// RAIN-004 remains the owner-selected morphology reference. RAIN-005B2
    /// keeps its marginal golden-small bias and focused kernel but distributes
    /// focused authority through a seed-phased low-discrepancy schedule so a
    /// short airborne window cannot accumulate an IID burst of focused ingresses.
    pub(crate) fn rain_morphology_diagnostics_report(
        &self,
        categories: &[Category],
    ) -> Result<String, String> {
        let Some(bounds) = self.surface.viewport_bounds() else {
            return Err(
                "testingcheats classic rainmetrics requires a non-empty classic/hybrid viewport"
                    .to_string(),
            );
        };
        let width = bounds.x_end.saturating_sub(bounds.x_start);
        let height = bounds.y_end.saturating_sub(bounds.y_start);
        if width == 0 || height == 0 {
            return Err(
                "testingcheats classic rainmetrics requires a non-empty classic/hybrid viewport"
                    .to_string(),
            );
        }

        let ingress_y = bounds.y_start;
        let free_columns = (bounds.x_start..bounds.x_end)
            .filter(|x| self.surface.grid[ingress_y][*x].is_none())
            .collect::<Vec<_>>();
        let focus = self
            .rain_focus_x
            .unwrap_or(bounds.x_start + width.saturating_sub(1) / 2)
            .clamp(bounds.x_start, bounds.x_end - 1);

        let ingress_rate_hz = MILLIS_PER_SECOND / TIME_SETTINGS.tick_ms as f64;
        let gravity_step_ms = TIME_SETTINGS.physics_ms.saturating_mul(2);
        let vertical_rows_per_second = if gravity_step_ms == 0 {
            0.0
        } else {
            MILLIS_PER_SECOND / gravity_step_ms as f64
        };

        let (actual_airborne, grounded_count, mean_open_drop_rows, surface_stats) =
            self.airborne_and_surface_metrics(bounds);
        let geometry_expected_airborne = if vertical_rows_per_second > 0.0 {
            ingress_rate_hz * mean_open_drop_rows / vertical_rows_per_second
        } else {
            0.0
        };

        let distribution = if self.mode == ClassicRainMode::WanderingFocus {
            rain_distribution_metrics(&free_columns, focus, RAIN_FOCUS_BIAS_PROBABILITY, width)
        } else {
            uniform_distribution_metrics(free_columns.len())
        };
        let unobstructed_columns = (bounds.x_start..bounds.x_end).collect::<Vec<_>>();
        let unobstructed_distribution = if self.mode == ClassicRainMode::WanderingFocus {
            rain_distribution_metrics(
                &unobstructed_columns,
                focus,
                RAIN_FOCUS_BIAS_PROBABILITY,
                width,
            )
        } else {
            uniform_distribution_metrics(unobstructed_columns.len())
        };

        let instant_nozzle_bits = actual_airborne as f64 * distribution.kl_bits_per_grain;
        let geometry_nozzle_bits =
            geometry_expected_airborne * unobstructed_distribution.kl_bits_per_grain;
        let equivalent_layer_grains = width as f64;
        let equivalent_layers_per_hour = if equivalent_layer_grains > 0.0 {
            ingress_rate_hz * SECONDS_PER_HOUR / equivalent_layer_grains
        } else {
            0.0
        };
        let mound_seconds = if ingress_rate_hz > 0.0 {
            unobstructed_distribution.one_dot_excess_mound_grains / ingress_rate_hz
        } else {
            f64::INFINITY
        };
        let mound_equivalent_layers = if equivalent_layer_grains > 0.0 {
            unobstructed_distribution.one_dot_excess_mound_grains / equivalent_layer_grains
        } else {
            0.0
        };

        let profiles = self.category_profiles(bounds, categories);
        let pair_metrics = adjacent_profile_metrics(&profiles, width);

        let mut report = String::new();
        writeln!(report, "CLASSIC_RAIN_MORPHOLOGY_DIAGNOSTICS").expect("String write");
        writeln!(report, "model={}", self.model_name()).expect("String write");
        writeln!(report, "experiment={}", self.experiment_profile_name()).expect("String write");
        writeln!(
            report,
            "reference=RAIN-004 owner-selected morphology baseline; runtime_candidate=RAIN-005B2 golden low-discrepancy focus-authority schedule; marginal bias/kernel/meander unchanged"
        )
        .expect("String write");
        writeln!(report).expect("String write");

        writeln!(report, "RAIN_SCALE").expect("String write");
        writeln!(report, "width_dots={width} height_dots={height}").expect("String write");
        writeln!(
            report,
            "canonical_ingress_rate_grains_per_second={ingress_rate_hz:.6} tick_ms={}",
            TIME_SETTINGS.tick_ms
        )
        .expect("String write");
        writeln!(
            report,
            "gravity_step_ms={gravity_step_ms} vertical_rows_per_second={vertical_rows_per_second:.6}"
        )
        .expect("String write");
        writeln!(
            report,
            "visible_airborne_now={} visible_grounded_now={} mean_open_drop_rows={mean_open_drop_rows:.6} geometry_expected_airborne_1x={geometry_expected_airborne:.6}",
            actual_airborne, grounded_count
        )
        .expect("String write");
        writeln!(
            report,
            "surface_height_mean_dots={:.6} surface_height_stddev_dots={:.6} surface_height_min_dots={} surface_height_max_dots={} surface_relief_range_dots={}",
            surface_stats.mean,
            surface_stats.stddev,
            surface_stats.min,
            surface_stats.max,
            surface_stats.max.saturating_sub(surface_stats.min)
        )
        .expect("String write");
        writeln!(
            report,
            "free_ingress_sites={} focus_x={} focus_x_norm={:.6}",
            distribution.free_sites,
            focus,
            normalize_x(focus, bounds.x_start, width)
        )
        .expect("String write");
        writeln!(
            report,
            "focus_bias_reference={:.12} current_free_site_kernel_kl_bits_per_grain={:.9} unobstructed_kernel_kl_bits_per_grain={:.9}",
            if self.mode == ClassicRainMode::WanderingFocus {
                RAIN_FOCUS_BIAS_PROBABILITY
            } else {
                0.0
            },
            distribution.kl_bits_per_grain,
            unobstructed_distribution.kl_bits_per_grain
        )
        .expect("String write");
        if self.mode == ClassicRainMode::WanderingFocus {
            writeln!(
                report,
                "focus_bias_schedule=golden-low-discrepancy effective_probability={:.15} phase_modulus={} max_prefix_count_error_lt=1 max_contiguous_window_count_error_lt=1",
                ClassicSandboxEngine::rain_focus_bias_effective_probability(),
                super::RAIN_FOCUS_BIAS_PHASE_MODULUS,
            )
            .expect("String write");
        }
        writeln!(
            report,
            "nozzle_information_bits_now={instant_nozzle_bits:.9} nozzle_information_bits_geometry_1x={geometry_nozzle_bits:.9}"
        )
        .expect("String write");
        writeln!(
            report,
            "focus_kernel_rms_width_dots={:.6} focus_kernel_rms_width_fraction={:.9}",
            unobstructed_distribution.focus_rms_width_dots,
            unobstructed_distribution.focus_rms_width_fraction
        )
        .expect("String write");
        writeln!(
            report,
            "positive_excess_grains_per_ingress={:.9} positive_excess_effective_width_dots={:.6}",
            unobstructed_distribution.positive_excess_per_ingress,
            unobstructed_distribution.positive_excess_effective_width_dots
        )
        .expect("String write");
        writeln!(
            report,
            "equivalent_layer_grains={equivalent_layer_grains:.6} equivalent_layers_per_hour={equivalent_layers_per_hour:.6}"
        )
        .expect("String write");
        if unobstructed_distribution
            .one_dot_excess_mound_grains
            .is_finite()
        {
            writeln!(
                report,
                "one_dot_excess_mound_grains={:.6} one_dot_excess_mound_seconds_1x={mound_seconds:.6} mound_equivalent_layers={mound_equivalent_layers:.6}",
                unobstructed_distribution.one_dot_excess_mound_grains
            )
            .expect("String write");
        } else {
            writeln!(
                report,
                "one_dot_excess_mound_grains=inf one_dot_excess_mound_seconds_1x=inf mound_equivalent_layers=inf"
            )
            .expect("String write");
        }

        writeln!(report).expect("String write");
        writeln!(report, "STRATA_METRICS categories={}", profiles.len()).expect("String write");
        for (rank, profile) in profiles.iter().enumerate() {
            writeln!(
                report,
                "CATEGORY rank_bottom_to_top={rank} name={:?} id={} mass={} centroid_x_norm={:.9} mean_thickness_dots={:.6} thickness_stddev_dots={:.6} thickness_cv={:.9} mean_y={:.6}",
                profile.name,
                profile.category_id.0,
                profile.mass,
                profile.centroid_x_norm,
                profile.mean_thickness,
                profile.thickness_stddev,
                profile.thickness_cv,
                profile.mean_y
            )
            .expect("String write");
        }

        writeln!(report).expect("String write");
        writeln!(report, "ADJACENT_STRATA_PAIRS count={}", pair_metrics.len())
            .expect("String write");
        for pair in &pair_metrics {
            writeln!(
                report,
                "PAIR lower={:?} upper={:?} thickness_profile_corr={} profile_total_variation={:.9} centroid_shift_fraction={:.9}",
                pair.lower_name,
                pair.upper_name,
                format_optional_float(pair.correlation),
                pair.total_variation,
                pair.centroid_shift_fraction
            )
            .expect("String write");
        }
        if !pair_metrics.is_empty() {
            let valid_corr = pair_metrics
                .iter()
                .filter_map(|pair| pair.correlation)
                .collect::<Vec<_>>();
            let mean_corr = mean(&valid_corr);
            let mean_tv = pair_metrics
                .iter()
                .map(|pair| pair.total_variation)
                .sum::<f64>()
                / pair_metrics.len() as f64;
            let mean_shift = pair_metrics
                .iter()
                .map(|pair| pair.centroid_shift_fraction)
                .sum::<f64>()
                / pair_metrics.len() as f64;
            writeln!(
                report,
                "PAIR_SUMMARY mean_thickness_profile_corr={} mean_profile_total_variation={mean_tv:.9} mean_centroid_shift_fraction={mean_shift:.9}",
                format_optional_float(mean_corr)
            )
            .expect("String write");
        }

        writeln!(report).expect("String write");
        writeln!(
            report,
            "NOTE: RAIN-005A is measurement-only. No metric in this report currently steers rain, changes focus motion, changes category behavior, or constitutes an acceptance threshold."
        )
        .expect("String write");
        Ok(report)
    }

    fn airborne_and_surface_metrics(
        &self,
        bounds: super::super::ViewportBounds,
    ) -> (usize, usize, f64, SurfaceHeightStats) {
        let mut airborne = 0usize;
        let mut grounded = 0usize;
        let mut open_drop_sum = 0usize;
        let mut heights = Vec::with_capacity(bounds.x_end.saturating_sub(bounds.x_start));

        for x in bounds.x_start..bounds.x_end {
            let mut grounded_top = bounds.y_end;
            while grounded_top > bounds.y_start && self.surface.grid[grounded_top - 1][x].is_some()
            {
                grounded_top -= 1;
            }
            let height = bounds.y_end.saturating_sub(grounded_top);
            heights.push(height as f64);

            for y in bounds.y_start..bounds.y_end {
                if self.surface.grid[y][x].is_none() {
                    continue;
                }
                if y >= grounded_top {
                    grounded = grounded.saturating_add(1);
                } else {
                    airborne = airborne.saturating_add(1);
                }
            }

            if self.surface.grid[bounds.y_start][x].is_none() {
                let first_occupied = (bounds.y_start + 1..bounds.y_end)
                    .find(|y| self.surface.grid[*y][x].is_some())
                    .unwrap_or(bounds.y_end);
                open_drop_sum =
                    open_drop_sum.saturating_add(first_occupied.saturating_sub(bounds.y_start));
            }
        }

        let free_top_count = (bounds.x_start..bounds.x_end)
            .filter(|x| self.surface.grid[bounds.y_start][*x].is_none())
            .count();
        let mean_open_drop_rows = if free_top_count == 0 {
            0.0
        } else {
            open_drop_sum as f64 / free_top_count as f64
        };

        (
            airborne,
            grounded,
            mean_open_drop_rows,
            SurfaceHeightStats::from_samples(&heights),
        )
    }

    fn category_profiles(
        &self,
        bounds: super::super::ViewportBounds,
        categories: &[Category],
    ) -> Vec<CategoryProfile> {
        let width = bounds.x_end.saturating_sub(bounds.x_start);
        let names = categories
            .iter()
            .map(|category| (category.id, category.name.clone()))
            .collect::<HashMap<_, _>>();
        let mut thickness = HashMap::<CategoryId, Vec<f64>>::new();
        let mut mass = HashMap::<CategoryId, usize>::new();
        let mut x_sum = HashMap::<CategoryId, f64>::new();
        let mut y_sum = HashMap::<CategoryId, f64>::new();

        for x in bounds.x_start..bounds.x_end {
            let mut grounded_top = bounds.y_end;
            while grounded_top > bounds.y_start && self.surface.grid[grounded_top - 1][x].is_some()
            {
                grounded_top -= 1;
            }
            for y in grounded_top..bounds.y_end {
                let Some(category_id) = self.surface.grid[y][x] else {
                    continue;
                };
                let profile = thickness
                    .entry(category_id)
                    .or_insert_with(|| vec![0.0; width]);
                profile[x - bounds.x_start] += 1.0;
                *mass.entry(category_id).or_insert(0) += 1;
                *x_sum.entry(category_id).or_insert(0.0) += (x - bounds.x_start) as f64;
                *y_sum.entry(category_id).or_insert(0.0) += y as f64;
            }
        }

        let mut profiles = thickness
            .into_iter()
            .filter_map(|(category_id, thickness)| {
                let category_mass = mass.get(&category_id).copied().unwrap_or(0);
                if category_mass == 0 {
                    return None;
                }
                let mean_thickness = category_mass as f64 / width as f64;
                let variance = thickness
                    .iter()
                    .map(|value| {
                        let delta = *value - mean_thickness;
                        delta * delta
                    })
                    .sum::<f64>()
                    / width as f64;
                let thickness_stddev = variance.sqrt();
                let thickness_cv = if mean_thickness > 0.0 {
                    thickness_stddev / mean_thickness
                } else {
                    0.0
                };
                let mean_x = x_sum.get(&category_id).copied().unwrap_or(0.0) / category_mass as f64;
                let centroid_x_norm = if width <= 1 {
                    0.5
                } else {
                    mean_x / (width - 1) as f64
                };
                let mean_y = y_sum.get(&category_id).copied().unwrap_or(0.0) / category_mass as f64;
                Some(CategoryProfile {
                    category_id,
                    name: names
                        .get(&category_id)
                        .cloned()
                        .unwrap_or_else(|| format!("<category:{}>", category_id.0)),
                    mass: category_mass,
                    mean_y,
                    centroid_x_norm,
                    mean_thickness,
                    thickness_stddev,
                    thickness_cv,
                    thickness,
                })
            })
            .collect::<Vec<_>>();

        // Larger y is closer to the visible bottom and therefore older/lower in
        // the current depositional stack. This ordering is diagnostic only; it
        // does not claim historical provenance when categories interpenetrate.
        profiles.sort_by(|left, right| {
            right
                .mean_y
                .total_cmp(&left.mean_y)
                .then_with(|| left.category_id.0.cmp(&right.category_id.0))
        });
        profiles
    }
}

#[derive(Debug, Clone, Copy)]
struct SurfaceHeightStats {
    mean: f64,
    stddev: f64,
    min: usize,
    max: usize,
}

impl SurfaceHeightStats {
    fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self {
                mean: 0.0,
                stddev: 0.0,
                min: 0,
                max: 0,
            };
        }
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance = samples
            .iter()
            .map(|sample| {
                let delta = *sample - mean;
                delta * delta
            })
            .sum::<f64>()
            / samples.len() as f64;
        let min = samples
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
            .max(0.0) as usize;
        let max = samples.iter().copied().fold(0.0, f64::max) as usize;
        Self {
            mean,
            stddev: variance.sqrt(),
            min,
            max,
        }
    }
}

#[derive(Debug, Clone)]
struct AdjacentProfileMetric {
    lower_name: String,
    upper_name: String,
    correlation: Option<f64>,
    total_variation: f64,
    centroid_shift_fraction: f64,
}

fn adjacent_profile_metrics(
    profiles: &[CategoryProfile],
    _width: usize,
) -> Vec<AdjacentProfileMetric> {
    profiles
        .windows(2)
        .map(|pair| {
            let lower = &pair[0];
            let upper = &pair[1];
            AdjacentProfileMetric {
                lower_name: lower.name.clone(),
                upper_name: upper.name.clone(),
                correlation: pearson(&lower.thickness, &upper.thickness),
                total_variation: normalized_profile_total_variation(
                    &lower.thickness,
                    &upper.thickness,
                ),
                centroid_shift_fraction: (lower.centroid_x_norm - upper.centroid_x_norm).abs(),
            }
        })
        .collect()
}

fn rain_distribution_metrics(
    free_columns: &[usize],
    focus: usize,
    bias: f64,
    active_width: usize,
) -> RainDistributionMetrics {
    if free_columns.is_empty() {
        return RainDistributionMetrics {
            free_sites: 0,
            kl_bits_per_grain: 0.0,
            focus_rms_width_dots: 0.0,
            focus_rms_width_fraction: 0.0,
            positive_excess_per_ingress: 0.0,
            positive_excess_effective_width_dots: 0.0,
            one_dot_excess_mound_grains: f64::INFINITY,
        };
    }

    let m = free_columns.len() as f64;
    let uniform = 1.0 / m;
    let denominator = m * m;
    let mut focus_kernel = Vec::with_capacity(free_columns.len());

    for &x in free_columns {
        let distance = x.abs_diff(focus);
        let mut farther = 0usize;
        let mut equal = 0usize;
        for &other in free_columns {
            match other.abs_diff(focus).cmp(&distance) {
                std::cmp::Ordering::Greater => farther += 1,
                std::cmp::Ordering::Equal => equal += 1,
                std::cmp::Ordering::Less => {}
            }
        }
        // Exact distribution of a two-candidate tournament that picks the
        // candidate nearer the focus, with a fair tie break. This is the current
        // broad RAIN-004 focus kernel, expressed analytically rather than by a
        // histogram or a chosen bin count.
        focus_kernel.push((2 * farther + equal) as f64 / denominator);
    }

    let mixed = focus_kernel
        .iter()
        .map(|focused| (1.0 - bias) * uniform + bias * *focused)
        .collect::<Vec<_>>();
    let kl_bits_per_grain = mixed
        .iter()
        .filter(|probability| **probability > 0.0)
        .map(|probability| *probability * (*probability / uniform).log2())
        .sum::<f64>();

    let rms = focus_kernel
        .iter()
        .zip(free_columns.iter().copied())
        .map(|(probability, x)| *probability * x.abs_diff(focus).pow(2) as f64)
        .sum::<f64>()
        .sqrt();

    let positive_excess = mixed
        .iter()
        .map(|probability| (*probability - uniform).max(0.0))
        .collect::<Vec<_>>();
    let positive_total = positive_excess.iter().sum::<f64>();
    let effective_width = if positive_total > 0.0 {
        let concentration = positive_excess
            .iter()
            .map(|excess| {
                let normalized = *excess / positive_total;
                normalized * normalized
            })
            .sum::<f64>();
        if concentration > 0.0 {
            1.0 / concentration
        } else {
            0.0
        }
    } else {
        0.0
    };
    let mound_grains = if positive_total > 0.0 {
        effective_width / positive_total
    } else {
        f64::INFINITY
    };

    RainDistributionMetrics {
        free_sites: free_columns.len(),
        kl_bits_per_grain,
        focus_rms_width_dots: rms,
        focus_rms_width_fraction: if active_width > 0 {
            rms / active_width as f64
        } else {
            0.0
        },
        positive_excess_per_ingress: positive_total,
        positive_excess_effective_width_dots: effective_width,
        one_dot_excess_mound_grains: mound_grains,
    }
}

fn uniform_distribution_metrics(free_sites: usize) -> RainDistributionMetrics {
    RainDistributionMetrics {
        free_sites,
        kl_bits_per_grain: 0.0,
        focus_rms_width_dots: 0.0,
        focus_rms_width_fraction: 0.0,
        positive_excess_per_ingress: 0.0,
        positive_excess_effective_width_dots: 0.0,
        one_dot_excess_mound_grains: f64::INFINITY,
    }
}

fn normalize_x(x: usize, x_start: usize, width: usize) -> f64 {
    if width <= 1 {
        0.5
    } else {
        x.saturating_sub(x_start) as f64 / (width - 1) as f64
    }
}

fn pearson(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let mut numerator = 0.0;
    let mut left_sq = 0.0;
    let mut right_sq = 0.0;
    for (left_value, right_value) in left.iter().zip(right.iter()) {
        let left_delta = *left_value - left_mean;
        let right_delta = *right_value - right_mean;
        numerator += left_delta * right_delta;
        left_sq += left_delta * left_delta;
        right_sq += right_delta * right_delta;
    }
    let denominator = (left_sq * right_sq).sqrt();
    if denominator <= f64::EPSILON {
        None
    } else {
        Some(numerator / denominator)
    }
}

fn normalized_profile_total_variation(left: &[f64], right: &[f64]) -> f64 {
    let left_total = left.iter().sum::<f64>();
    let right_total = right.iter().sum::<f64>();
    if left_total <= 0.0 || right_total <= 0.0 {
        return 0.0;
    }
    left.iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| {
            (*left_value / left_total - *right_value / right_total).abs()
        })
        .sum::<f64>()
        * 0.5
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn format_optional_float(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |value| format!("{value:.9}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(id: u64, name: &str) -> Category {
        Category {
            id: CategoryId::new(id),
            name: name.to_string(),
            color: ratatui::style::Color::White,
            description: String::new(),
            balance_effect: 0,
        }
    }

    #[test]
    fn tournament_kernel_is_normalized_and_broader_than_a_nozzle() {
        let free = (0..100).collect::<Vec<_>>();
        let metrics = rain_distribution_metrics(&free, 50, RAIN_FOCUS_BIAS_PROBABILITY, 100);
        assert_eq!(metrics.free_sites, 100);
        assert!(metrics.kl_bits_per_grain > 0.0);
        assert!(metrics.focus_rms_width_dots > 10.0);
        assert!(metrics.focus_rms_width_fraction > 0.1);
        assert!(metrics.positive_excess_per_ingress > 0.0);
        assert!(metrics.one_dot_excess_mound_grains.is_finite());
    }

    #[test]
    fn uniform_control_has_zero_nozzle_information_and_no_mound_timescale() {
        let metrics = uniform_distribution_metrics(100);
        assert_eq!(metrics.kl_bits_per_grain, 0.0);
        assert_eq!(metrics.positive_excess_per_ingress, 0.0);
        assert!(metrics.one_dot_excess_mound_grains.is_infinite());
    }

    #[test]
    fn diagnostics_are_read_only_and_derive_one_dot_per_second_clock() {
        let mut engine =
            ClassicSandboxEngine::new(80, 30, 0xA11CE, ClassicRainMode::WanderingFocus);
        engine.spawn(CategoryId::new(1));
        for _ in 0..64 {
            engine.update();
        }
        let grid_before = engine.surface.grid.clone();
        let rain_rng_before = engine.rain_rng_state;
        let focus_before = engine.rain_focus_x;
        let report = engine
            .rain_morphology_diagnostics_report(&[category(1, "One")])
            .expect("diagnostic report");
        assert!(report.contains("canonical_ingress_rate_grains_per_second=1.000000"));
        assert!(report.contains(
            "reference=RAIN-004 owner-selected morphology baseline; runtime_candidate=RAIN-005B2"
        ));
        assert!(report.contains("RAIN_SCALE"));
        assert!(report.contains("STRATA_METRICS"));
        assert_eq!(engine.surface.grid, grid_before);
        assert_eq!(engine.rain_rng_state, rain_rng_before);
        assert_eq!(engine.rain_focus_x, focus_before);
    }

    #[test]
    fn equivalent_layer_scale_follows_active_dot_width() {
        let engine = ClassicSandboxEngine::new(73, 20, 0x51A1E, ClassicRainMode::WanderingFocus);
        let report = engine
            .rain_morphology_diagnostics_report(&[])
            .expect("diagnostic report");
        // Classic uses two horizontal Braille dots per terminal cell.
        assert!(report.contains("width_dots=146"));
        assert!(report.contains("equivalent_layer_grains=146.000000"));
    }

    #[test]
    fn adjacent_category_profiles_measure_real_lateral_difference() {
        let mut engine =
            ClassicSandboxEngine::new(20, 12, 0x57A7A, ClassicRainMode::WanderingFocus);
        let bounds = engine.surface.viewport_bounds().expect("viewport");
        let bottom = bounds.y_end - 1;
        for x in bounds.x_start..bounds.x_end {
            engine.surface.grid[bottom][x] = Some(CategoryId::new(1));
            if x < bounds.x_start + (bounds.x_end - bounds.x_start) / 2 {
                engine.surface.grid[bottom - 1][x] = Some(CategoryId::new(2));
                engine.surface.grid[bottom - 2][x] = Some(CategoryId::new(2));
            } else {
                engine.surface.grid[bottom - 1][x] = Some(CategoryId::new(3));
            }
        }
        let report = engine
            .rain_morphology_diagnostics_report(&[
                category(1, "Base"),
                category(2, "Left"),
                category(3, "Right"),
            ])
            .expect("diagnostic report");
        assert!(report.contains("ADJACENT_STRATA_PAIRS"));
        assert!(report.contains("profile_total_variation="));
        assert!(report.contains("centroid_shift_fraction="));
    }

    fn advance_exact_grains(
        engine: &mut ClassicSandboxEngine,
        grains: usize,
        category_id: CategoryId,
    ) {
        use std::time::Duration;

        let tick = Duration::from_millis(TIME_SETTINGS.tick_ms);
        let physics = Duration::from_millis(TIME_SETTINGS.physics_ms);
        let simulated = Duration::from_millis(
            TIME_SETTINGS
                .tick_ms
                .saturating_mul(u64::try_from(grains).expect("diagnostic grain count fits u64")),
        );
        let mut spawn_accumulator = Duration::ZERO;
        let mut physics_accumulator = Duration::ZERO;
        let mut remaining = simulated;

        while !remaining.is_zero() {
            let spawn_left = tick.saturating_sub(spawn_accumulator);
            let physics_left = physics.saturating_sub(physics_accumulator);
            let step = remaining.min(spawn_left.min(physics_left));
            spawn_accumulator += step;
            physics_accumulator += step;
            remaining = remaining.saturating_sub(step);

            let spawn_due = spawn_accumulator >= tick;
            let physics_due = physics_accumulator >= physics;
            if spawn_due {
                spawn_accumulator = spawn_accumulator.saturating_sub(tick);
                engine.spawn(category_id);
            }
            if physics_due {
                physics_accumulator = physics_accumulator.saturating_sub(physics);
                engine.update();
            }
            assert!(step > Duration::ZERO || spawn_due || physics_due);
        }
    }

    fn halton(mut index: usize, base: usize) -> f64 {
        let mut fraction = 1.0;
        let mut value = 0.0;
        while index > 0 {
            fraction /= base as f64;
            value += fraction * (index % base) as f64;
            index /= base;
        }
        value
    }

    fn percentile(values: &[f64], quantile: f64) -> f64 {
        assert!(!values.is_empty());
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let index = ((sorted.len() - 1) as f64 * quantile)
            .round()
            .clamp(0.0, (sorted.len() - 1) as f64) as usize;
        sorted[index]
    }

    fn mean_defined(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
        let defined = values.flatten().collect::<Vec<_>>();
        mean(&defined)
    }

    #[test]
    #[ignore = "native RAIN-005A2 low-discrepancy viewport/seed morphology ensemble"]
    fn rain_005a2_viewport_seed_morphology_ensemble_probe() {
        // This is deliberately NOT a list of popular display resolutions. A
        // low-discrepancy sequence covers the width/height plane continuously,
        // so tiled panes, portrait-ish terminals, zoomed fonts and uncommon
        // rectangles contribute to the baseline family instead of being rounded
        // to a handful of device presets.
        const CASES: usize = 16;
        const MIN_WIDTH_CELLS: usize = 20;
        const MAX_WIDTH_CELLS: usize = 200;
        const MIN_HEIGHT_CELLS: usize = 10;
        const MAX_HEIGHT_CELLS: usize = 60;
        const CATEGORY_COUNT: usize = 5;

        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("Ensemble {id}")))
            .collect::<Vec<_>>();

        let mut cvs = Vec::with_capacity(CASES);
        let mut correlations = Vec::with_capacity(CASES);
        let mut variations = Vec::with_capacity(CASES);
        let mut shifts = Vec::with_capacity(CASES);
        let mut centroid_spans = Vec::with_capacity(CASES);
        let mut mound_els = Vec::with_capacity(CASES);

        for case in 1..=CASES {
            let width_cells = MIN_WIDTH_CELLS
                + (halton(case, 2) * (MAX_WIDTH_CELLS - MIN_WIDTH_CELLS) as f64).round() as usize;
            let height_cells = MIN_HEIGHT_CELLS
                + (halton(case, 3) * (MAX_HEIGHT_CELLS - MIN_HEIGHT_CELLS) as f64).round() as usize;
            let seed = 0xA11C_005A_2000_0000u64 ^ (case as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let mut engine = ClassicSandboxEngine::new(
                u16::try_from(width_cells).expect("diagnostic width fits u16"),
                u16::try_from(height_cells).expect("diagnostic height fits u16"),
                seed,
                ClassicRainMode::WanderingFocus,
            );
            engine.force_reference_gravity = false;

            let bounds = engine.surface.viewport_bounds().expect("viewport");
            let width_dots = bounds.x_end - bounds.x_start;
            let free = (bounds.x_start..bounds.x_end).collect::<Vec<_>>();
            let center = bounds.x_start + width_dots.saturating_sub(1) / 2;
            let kernel =
                rain_distribution_metrics(&free, center, RAIN_FOCUS_BIAS_PROBABILITY, width_dots);
            let mound_grains = kernel.one_dot_excess_mound_grains.round().max(1.0) as usize;
            let mound_el = kernel.one_dot_excess_mound_grains / width_dots as f64;
            mound_els.push(mound_el);

            // Each category receives one analytically derived one-dot-excess
            // mound timescale. This normalizes the observation window to the
            // rain kernel instead of choosing a raw number of seconds/grains.
            for category_index in 1..=CATEGORY_COUNT {
                advance_exact_grains(
                    &mut engine,
                    mound_grains,
                    CategoryId::new(category_index as u64),
                );
            }

            let profiles = engine.category_profiles(bounds, &categories);
            let pairs = adjacent_profile_metrics(&profiles, width_dots);
            assert!(
                profiles.len() >= CATEGORY_COUNT - 1,
                "case={case} profiles={}",
                profiles.len()
            );
            assert!(
                pairs.len() >= CATEGORY_COUNT - 2,
                "case={case} pairs={}",
                pairs.len()
            );

            let mean_cv = mean(
                &profiles
                    .iter()
                    .map(|profile| profile.thickness_cv)
                    .collect::<Vec<_>>(),
            )
            .expect("profiles");
            let mean_corr = mean_defined(pairs.iter().map(|pair| pair.correlation)).unwrap_or(0.0);
            let mean_tv = mean(
                &pairs
                    .iter()
                    .map(|pair| pair.total_variation)
                    .collect::<Vec<_>>(),
            )
            .expect("pairs");
            let mean_shift = mean(
                &pairs
                    .iter()
                    .map(|pair| pair.centroid_shift_fraction)
                    .collect::<Vec<_>>(),
            )
            .expect("pairs");
            let min_centroid = profiles
                .iter()
                .map(|profile| profile.centroid_x_norm)
                .fold(f64::INFINITY, f64::min);
            let max_centroid = profiles
                .iter()
                .map(|profile| profile.centroid_x_norm)
                .fold(f64::NEG_INFINITY, f64::max);
            let centroid_span = max_centroid - min_centroid;

            cvs.push(mean_cv);
            correlations.push(mean_corr);
            variations.push(mean_tv);
            shifts.push(mean_shift);
            centroid_spans.push(centroid_span);

            println!(
                "RAIN_005A2_MORPH_SAMPLE case={case} terminal={}x{} dots={}x{} aspect={:.4} seed={seed} category_mound_grains={mound_grains} mound_el={mound_el:.6} mean_thickness_cv={mean_cv:.9} mean_pair_corr={mean_corr:.9} mean_pair_tv={mean_tv:.9} mean_pair_centroid_shift={mean_shift:.9} centroid_span={centroid_span:.9}",
                width_cells,
                height_cells,
                width_dots,
                bounds.y_end - bounds.y_start,
                width_cells as f64 / height_cells as f64,
            );
        }

        println!(
            "RAIN_005A2_MORPH_ENSEMBLE cases={CASES} cv_p10={:.9} cv_p50={:.9} cv_p90={:.9} corr_p10={:.9} corr_p50={:.9} corr_p90={:.9} tv_p10={:.9} tv_p50={:.9} tv_p90={:.9} shift_p10={:.9} shift_p50={:.9} shift_p90={:.9} centroid_span_p10={:.9} centroid_span_p50={:.9} centroid_span_p90={:.9} mound_el_p10={:.9} mound_el_p50={:.9} mound_el_p90={:.9}",
            percentile(&cvs, 0.10),
            percentile(&cvs, 0.50),
            percentile(&cvs, 0.90),
            percentile(&correlations, 0.10),
            percentile(&correlations, 0.50),
            percentile(&correlations, 0.90),
            percentile(&variations, 0.10),
            percentile(&variations, 0.50),
            percentile(&variations, 0.90),
            percentile(&shifts, 0.10),
            percentile(&shifts, 0.50),
            percentile(&shifts, 0.90),
            percentile(&centroid_spans, 0.10),
            percentile(&centroid_spans, 0.50),
            percentile(&centroid_spans, 0.90),
            percentile(&mound_els, 0.10),
            percentile(&mound_els, 0.50),
            percentile(&mound_els, 0.90),
        );
    }

    #[test]
    #[ignore = "native RAIN-005A2 analytical arbitrary-geometry/nozzle sweep"]
    fn rain_005a2_analytical_arbitrary_geometry_nozzle_sweep_probe() {
        const CASES: usize = 64;
        let ingress_rate_hz = MILLIS_PER_SECOND / TIME_SETTINGS.tick_ms as f64;
        let gravity_step_ms = TIME_SETTINGS.physics_ms.saturating_mul(2);
        let rows_per_second = MILLIS_PER_SECOND / gravity_step_ms as f64;

        let mut nozzle_bits = Vec::with_capacity(CASES);
        let mut kernel_width_fraction = Vec::with_capacity(CASES);
        let mut mound_el = Vec::with_capacity(CASES);

        for case in 1..=CASES {
            // Again use low-discrepancy coverage rather than named resolutions.
            let width_dots = 24 + (halton(case, 2) * (640 - 24) as f64).round() as usize;
            let height_dots = 24 + (halton(case, 3) * (320 - 24) as f64).round() as usize;
            let focus_norm = halton(case, 5);
            let sky_fraction = 0.05 + 0.90 * halton(case, 7);
            let focus = ((width_dots - 1) as f64 * focus_norm).round() as usize;
            let free = (0..width_dots).collect::<Vec<_>>();
            let distribution =
                rain_distribution_metrics(&free, focus, RAIN_FOCUS_BIAS_PROBABILITY, width_dots);
            let mean_drop_rows = height_dots as f64 * sky_fraction;
            let expected_airborne = ingress_rate_hz * mean_drop_rows / rows_per_second;
            let information = expected_airborne * distribution.kl_bits_per_grain;
            let mound_equivalent_layers =
                distribution.one_dot_excess_mound_grains / width_dots as f64;

            nozzle_bits.push(information);
            kernel_width_fraction.push(distribution.focus_rms_width_fraction);
            mound_el.push(mound_equivalent_layers);

            println!(
                "RAIN_005A2_NOZZLE_SAMPLE case={case} width_dots={width_dots} height_dots={height_dots} focus_norm={focus_norm:.6} sky_fraction={sky_fraction:.6} expected_airborne={expected_airborne:.6} kl_bits_per_grain={:.9} nozzle_information_bits={information:.9} kernel_rms_width_fraction={:.9} mound_el={mound_equivalent_layers:.9}",
                distribution.kl_bits_per_grain, distribution.focus_rms_width_fraction,
            );
        }

        println!(
            "RAIN_005A2_NOZZLE_ENSEMBLE cases={CASES} information_p10={:.9} information_p50={:.9} information_p90={:.9} information_max={:.9} kernel_width_fraction_p10={:.9} kernel_width_fraction_p50={:.9} kernel_width_fraction_p90={:.9} mound_el_p10={:.9} mound_el_p50={:.9} mound_el_p90={:.9}",
            percentile(&nozzle_bits, 0.10),
            percentile(&nozzle_bits, 0.50),
            percentile(&nozzle_bits, 0.90),
            nozzle_bits.iter().copied().fold(0.0, f64::max),
            percentile(&kernel_width_fraction, 0.10),
            percentile(&kernel_width_fraction, 0.50),
            percentile(&kernel_width_fraction, 0.90),
            percentile(&mound_el, 0.10),
            percentile(&mound_el, 0.50),
            percentile(&mound_el, 0.90),
        );
    }

    fn logarithmic_sample(min: f64, max: f64, unit: f64) -> f64 {
        assert!(min > 0.0 && max >= min);
        (min.ln() + unit.clamp(0.0, 1.0) * (max.ln() - min.ln())).exp()
    }

    fn rain_005a3_geometry(index: usize) -> (usize, usize) {
        // RAIN-005A3 deliberately samples geometry in area/aspect space rather
        // than enumerating named display resolutions. Both dimensions are
        // logarithmic so a tiled pane and a very large terminal receive useful
        // representation without the large end dominating the ensemble.
        const MIN_TERMINAL_AREA: f64 = 300.0;
        const MAX_TERMINAL_AREA: f64 = 50_000.0;
        const MIN_ASPECT: f64 = 0.18;
        const MAX_ASPECT: f64 = 18.0;
        const MIN_WIDTH: usize = 20;
        const MAX_WIDTH: usize = 420;
        const MIN_HEIGHT: usize = 10;
        const MAX_HEIGHT: usize = 140;

        let area = logarithmic_sample(MIN_TERMINAL_AREA, MAX_TERMINAL_AREA, halton(index, 2));
        let aspect = logarithmic_sample(MIN_ASPECT, MAX_ASPECT, halton(index, 3));
        let width = (area * aspect)
            .sqrt()
            .round()
            .max(MIN_WIDTH as f64)
            .min(MAX_WIDTH as f64) as usize;
        let height = (area / aspect)
            .sqrt()
            .round()
            .max(MIN_HEIGHT as f64)
            .min(MAX_HEIGHT as f64) as usize;
        (width, height)
    }

    #[derive(Debug, Clone, Copy)]
    struct MorphologySample {
        cv: f64,
        correlation: f64,
        total_variation: f64,
        centroid_shift: f64,
        centroid_span: f64,
        mound_el: f64,
    }

    fn morphology_sample_for_geometry(
        width_cells: usize,
        height_cells: usize,
        seed: u64,
        categories: &[Category],
    ) -> MorphologySample {
        morphology_sample_for_geometry_with_bias_schedule(
            width_cells,
            height_cells,
            seed,
            categories,
            RainBiasScheduleProbe::Runtime,
        )
    }

    fn morphology_sample_for_geometry_with_bias_schedule(
        width_cells: usize,
        height_cells: usize,
        seed: u64,
        categories: &[Category],
        schedule: RainBiasScheduleProbe,
    ) -> MorphologySample {
        let mut engine = ClassicSandboxEngine::new(
            u16::try_from(width_cells).expect("diagnostic width fits u16"),
            u16::try_from(height_cells).expect("diagnostic height fits u16"),
            seed,
            ClassicRainMode::WanderingFocus,
        );
        engine.force_reference_gravity = false;
        engine.rain_bias_schedule_probe = schedule;

        let bounds = engine.surface.viewport_bounds().expect("viewport");
        let width_dots = bounds.x_end - bounds.x_start;
        let free = (bounds.x_start..bounds.x_end).collect::<Vec<_>>();
        let center = bounds.x_start + width_dots.saturating_sub(1) / 2;
        let kernel =
            rain_distribution_metrics(&free, center, RAIN_FOCUS_BIAS_PROBABILITY, width_dots);
        let mound_grains = kernel.one_dot_excess_mound_grains.round().max(1.0) as usize;
        let mound_el = kernel.one_dot_excess_mound_grains / width_dots as f64;

        for category in categories {
            advance_exact_grains(&mut engine, mound_grains, category.id);
        }

        let profiles = engine.category_profiles(bounds, categories);
        let pairs = adjacent_profile_metrics(&profiles, width_dots);
        assert!(
            profiles.len() >= categories.len().saturating_sub(1),
            "terminal={width_cells}x{height_cells} profiles={} expected_at_least={}",
            profiles.len(),
            categories.len().saturating_sub(1)
        );
        assert!(
            pairs.len() >= categories.len().saturating_sub(2),
            "terminal={width_cells}x{height_cells} pairs={} expected_at_least={}",
            pairs.len(),
            categories.len().saturating_sub(2)
        );

        let cv = mean(
            &profiles
                .iter()
                .map(|profile| profile.thickness_cv)
                .collect::<Vec<_>>(),
        )
        .expect("profiles");
        let correlation = mean_defined(pairs.iter().map(|pair| pair.correlation)).unwrap_or(0.0);
        let total_variation = mean(
            &pairs
                .iter()
                .map(|pair| pair.total_variation)
                .collect::<Vec<_>>(),
        )
        .expect("pairs");
        let centroid_shift = mean(
            &pairs
                .iter()
                .map(|pair| pair.centroid_shift_fraction)
                .collect::<Vec<_>>(),
        )
        .expect("pairs");
        let min_centroid = profiles
            .iter()
            .map(|profile| profile.centroid_x_norm)
            .fold(f64::INFINITY, f64::min);
        let max_centroid = profiles
            .iter()
            .map(|profile| profile.centroid_x_norm)
            .fold(f64::NEG_INFINITY, f64::max);

        MorphologySample {
            cv,
            correlation,
            total_variation,
            centroid_shift,
            centroid_span: max_centroid - min_centroid,
            mound_el,
        }
    }

    fn abs_delta(left: f64, right: f64) -> f64 {
        (left - right).abs()
    }

    fn print_extended_percentiles(label: &str, values: &[f64]) {
        println!(
            "{label} p05={:.9} p10={:.9} p25={:.9} p50={:.9} p75={:.9} p90={:.9} p95={:.9} min={:.9} max={:.9}",
            percentile(values, 0.05),
            percentile(values, 0.10),
            percentile(values, 0.25),
            percentile(values, 0.50),
            percentile(values, 0.75),
            percentile(values, 0.90),
            percentile(values, 0.95),
            values.iter().copied().fold(f64::INFINITY, f64::min),
            values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
    }

    fn correlation_or_zero(left: &[f64], right: &[f64]) -> f64 {
        pearson(left, right).unwrap_or(0.0)
    }

    #[test]
    #[ignore = "native RAIN-005A3 extended geometry x paired-seed morphology ensemble; intentionally long-running"]
    fn rain_005a3_extended_geometry_paired_seed_morphology_ensemble_probe() {
        // 96 continuously sampled geometries x two independent seeds gives 192
        // morphology runs. The domain spans tiny panes through terminals larger
        // than common fullscreen use, portrait through extreme ultrawide. The
        // geometry generator is low-discrepancy and logarithmic; there are no
        // canonical user resolutions hidden in the test.
        const GEOMETRIES: usize = 96;
        const SEEDS_PER_GEOMETRY: usize = 2;
        const CATEGORY_COUNT: usize = 5;

        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("Extended {id}")))
            .collect::<Vec<_>>();

        let mut widths = Vec::with_capacity(GEOMETRIES);
        let mut heights = Vec::with_capacity(GEOMETRIES);
        let mut log_widths = Vec::with_capacity(GEOMETRIES);
        let mut log_heights = Vec::with_capacity(GEOMETRIES);
        let mut log_areas = Vec::with_capacity(GEOMETRIES);
        let mut log_aspects = Vec::with_capacity(GEOMETRIES);
        let mut abs_log_aspects = Vec::with_capacity(GEOMETRIES);

        let mut geometry_mean_cv = Vec::with_capacity(GEOMETRIES);
        let mut geometry_mean_corr = Vec::with_capacity(GEOMETRIES);
        let mut geometry_mean_tv = Vec::with_capacity(GEOMETRIES);
        let mut geometry_mean_shift = Vec::with_capacity(GEOMETRIES);
        let mut geometry_mean_span = Vec::with_capacity(GEOMETRIES);
        let mut geometry_mean_mound_el = Vec::with_capacity(GEOMETRIES);

        let mut all_cv = Vec::with_capacity(GEOMETRIES * SEEDS_PER_GEOMETRY);
        let mut all_corr = Vec::with_capacity(GEOMETRIES * SEEDS_PER_GEOMETRY);
        let mut all_tv = Vec::with_capacity(GEOMETRIES * SEEDS_PER_GEOMETRY);
        let mut all_shift = Vec::with_capacity(GEOMETRIES * SEEDS_PER_GEOMETRY);
        let mut all_span = Vec::with_capacity(GEOMETRIES * SEEDS_PER_GEOMETRY);
        let mut all_mound_el = Vec::with_capacity(GEOMETRIES * SEEDS_PER_GEOMETRY);

        let mut seed_delta_cv = Vec::with_capacity(GEOMETRIES);
        let mut seed_delta_corr = Vec::with_capacity(GEOMETRIES);
        let mut seed_delta_tv = Vec::with_capacity(GEOMETRIES);
        let mut seed_delta_shift = Vec::with_capacity(GEOMETRIES);
        let mut seed_delta_span = Vec::with_capacity(GEOMETRIES);

        for geometry in 1..=GEOMETRIES {
            let (width_cells, height_cells) = rain_005a3_geometry(geometry);
            let area = (width_cells * height_cells) as f64;
            let aspect = width_cells as f64 / height_cells as f64;
            widths.push(width_cells as f64);
            heights.push(height_cells as f64);
            log_widths.push((width_cells as f64).ln());
            log_heights.push((height_cells as f64).ln());
            log_areas.push(area.ln());
            log_aspects.push(aspect.ln());
            abs_log_aspects.push(aspect.ln().abs());

            let mut samples = Vec::with_capacity(SEEDS_PER_GEOMETRY);
            for seed_index in 0..SEEDS_PER_GEOMETRY {
                let run_index = (geometry - 1) * SEEDS_PER_GEOMETRY + seed_index + 1;
                let seed = 0xA11C_005A_3000_0000u64
                    ^ (geometry as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    ^ ((seed_index as u64) + 1).wrapping_mul(0xD1B5_4A32_D192_ED03);
                let sample =
                    morphology_sample_for_geometry(width_cells, height_cells, seed, &categories);
                all_cv.push(sample.cv);
                all_corr.push(sample.correlation);
                all_tv.push(sample.total_variation);
                all_shift.push(sample.centroid_shift);
                all_span.push(sample.centroid_span);
                all_mound_el.push(sample.mound_el);
                samples.push(sample);

                println!(
                    "RAIN_005A3_MORPH_SAMPLE run={run_index} geometry={geometry} seed_slot={} terminal={}x{} dots={}x{} area={} aspect={aspect:.6} seed={seed} mound_el={:.9} mean_thickness_cv={:.9} mean_pair_corr={:.9} mean_pair_tv={:.9} mean_pair_centroid_shift={:.9} centroid_span={:.9}",
                    seed_index + 1,
                    width_cells,
                    height_cells,
                    width_cells * 2,
                    height_cells * 4,
                    width_cells * height_cells,
                    sample.mound_el,
                    sample.cv,
                    sample.correlation,
                    sample.total_variation,
                    sample.centroid_shift,
                    sample.centroid_span,
                );
            }

            let first = samples[0];
            let second = samples[1];
            geometry_mean_cv.push((first.cv + second.cv) * 0.5);
            geometry_mean_corr.push((first.correlation + second.correlation) * 0.5);
            geometry_mean_tv.push((first.total_variation + second.total_variation) * 0.5);
            geometry_mean_shift.push((first.centroid_shift + second.centroid_shift) * 0.5);
            geometry_mean_span.push((first.centroid_span + second.centroid_span) * 0.5);
            geometry_mean_mound_el.push((first.mound_el + second.mound_el) * 0.5);

            seed_delta_cv.push(abs_delta(first.cv, second.cv));
            seed_delta_corr.push(abs_delta(first.correlation, second.correlation));
            seed_delta_tv.push(abs_delta(first.total_variation, second.total_variation));
            seed_delta_shift.push(abs_delta(first.centroid_shift, second.centroid_shift));
            seed_delta_span.push(abs_delta(first.centroid_span, second.centroid_span));
        }

        let min_width = widths.iter().copied().fold(f64::INFINITY, f64::min);
        let max_width = widths.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_height = heights.iter().copied().fold(f64::INFINITY, f64::min);
        let max_height = heights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_area = widths
            .iter()
            .zip(heights.iter())
            .map(|(width, height)| width * height)
            .fold(f64::INFINITY, f64::min);
        let max_area = widths
            .iter()
            .zip(heights.iter())
            .map(|(width, height)| width * height)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_aspect = widths
            .iter()
            .zip(heights.iter())
            .map(|(width, height)| width / height)
            .fold(f64::INFINITY, f64::min);
        let max_aspect = widths
            .iter()
            .zip(heights.iter())
            .map(|(width, height)| width / height)
            .fold(f64::NEG_INFINITY, f64::max);

        // Fail if later edits accidentally collapse this back into a small or
        // conventional-desktop-only ensemble. These are test-domain coverage
        // guards, never runtime morphology constants.
        assert!(
            min_width <= 24.0,
            "extended ensemble lost narrow-pane coverage"
        );
        assert!(
            max_width >= 360.0,
            "extended ensemble lost large-width coverage"
        );
        assert!(
            min_height <= 14.0,
            "extended ensemble lost shallow-pane coverage"
        );
        assert!(
            max_height >= 120.0,
            "extended ensemble lost tall-terminal coverage"
        );
        assert!(
            min_aspect <= 0.30,
            "extended ensemble lost portrait coverage"
        );
        assert!(
            max_aspect >= 10.0,
            "extended ensemble lost ultrawide coverage"
        );
        assert!(
            max_area >= 25_000.0,
            "extended ensemble lost large-area coverage"
        );

        println!(
            "RAIN_005A3_MORPH_ENSEMBLE geometries={GEOMETRIES} seeds_per_geometry={SEEDS_PER_GEOMETRY} runs={} width_min={} width_max={} height_min={} height_max={} area_min={} area_max={} aspect_min={:.6} aspect_max={:.6}",
            GEOMETRIES * SEEDS_PER_GEOMETRY,
            min_width as usize,
            max_width as usize,
            min_height as usize,
            max_height as usize,
            min_area as usize,
            max_area as usize,
            min_aspect,
            max_aspect,
        );

        print_extended_percentiles("RAIN_005A3_CV", &all_cv);
        print_extended_percentiles("RAIN_005A3_CORR", &all_corr);
        print_extended_percentiles("RAIN_005A3_TV", &all_tv);
        print_extended_percentiles("RAIN_005A3_SHIFT", &all_shift);
        print_extended_percentiles("RAIN_005A3_CENTROID_SPAN", &all_span);
        print_extended_percentiles("RAIN_005A3_MOUND_EL", &all_mound_el);

        println!(
            "RAIN_005A3_SEED_VARIATION cv_delta_p50={:.9} cv_delta_p90={:.9} cv_delta_p95={:.9} corr_delta_p50={:.9} corr_delta_p90={:.9} corr_delta_p95={:.9} tv_delta_p50={:.9} tv_delta_p90={:.9} tv_delta_p95={:.9} shift_delta_p50={:.9} shift_delta_p90={:.9} shift_delta_p95={:.9} span_delta_p50={:.9} span_delta_p90={:.9} span_delta_p95={:.9}",
            percentile(&seed_delta_cv, 0.50),
            percentile(&seed_delta_cv, 0.90),
            percentile(&seed_delta_cv, 0.95),
            percentile(&seed_delta_corr, 0.50),
            percentile(&seed_delta_corr, 0.90),
            percentile(&seed_delta_corr, 0.95),
            percentile(&seed_delta_tv, 0.50),
            percentile(&seed_delta_tv, 0.90),
            percentile(&seed_delta_tv, 0.95),
            percentile(&seed_delta_shift, 0.50),
            percentile(&seed_delta_shift, 0.90),
            percentile(&seed_delta_shift, 0.95),
            percentile(&seed_delta_span, 0.50),
            percentile(&seed_delta_span, 0.90),
            percentile(&seed_delta_span, 0.95),
        );

        println!(
            "RAIN_005A3_GEOMETRY_DEPENDENCE metric=cv log_width={:.6} log_height={:.6} log_area={:.6} log_aspect={:.6} abs_log_aspect={:.6}",
            correlation_or_zero(&log_widths, &geometry_mean_cv),
            correlation_or_zero(&log_heights, &geometry_mean_cv),
            correlation_or_zero(&log_areas, &geometry_mean_cv),
            correlation_or_zero(&log_aspects, &geometry_mean_cv),
            correlation_or_zero(&abs_log_aspects, &geometry_mean_cv),
        );
        println!(
            "RAIN_005A3_GEOMETRY_DEPENDENCE metric=corr log_width={:.6} log_height={:.6} log_area={:.6} log_aspect={:.6} abs_log_aspect={:.6}",
            correlation_or_zero(&log_widths, &geometry_mean_corr),
            correlation_or_zero(&log_heights, &geometry_mean_corr),
            correlation_or_zero(&log_areas, &geometry_mean_corr),
            correlation_or_zero(&log_aspects, &geometry_mean_corr),
            correlation_or_zero(&abs_log_aspects, &geometry_mean_corr),
        );
        println!(
            "RAIN_005A3_GEOMETRY_DEPENDENCE metric=tv log_width={:.6} log_height={:.6} log_area={:.6} log_aspect={:.6} abs_log_aspect={:.6}",
            correlation_or_zero(&log_widths, &geometry_mean_tv),
            correlation_or_zero(&log_heights, &geometry_mean_tv),
            correlation_or_zero(&log_areas, &geometry_mean_tv),
            correlation_or_zero(&log_aspects, &geometry_mean_tv),
            correlation_or_zero(&abs_log_aspects, &geometry_mean_tv),
        );
        println!(
            "RAIN_005A3_GEOMETRY_DEPENDENCE metric=shift log_width={:.6} log_height={:.6} log_area={:.6} log_aspect={:.6} abs_log_aspect={:.6}",
            correlation_or_zero(&log_widths, &geometry_mean_shift),
            correlation_or_zero(&log_heights, &geometry_mean_shift),
            correlation_or_zero(&log_areas, &geometry_mean_shift),
            correlation_or_zero(&log_aspects, &geometry_mean_shift),
            correlation_or_zero(&abs_log_aspects, &geometry_mean_shift),
        );
        println!(
            "RAIN_005A3_GEOMETRY_DEPENDENCE metric=centroid_span log_width={:.6} log_height={:.6} log_area={:.6} log_aspect={:.6} abs_log_aspect={:.6}",
            correlation_or_zero(&log_widths, &geometry_mean_span),
            correlation_or_zero(&log_heights, &geometry_mean_span),
            correlation_or_zero(&log_areas, &geometry_mean_span),
            correlation_or_zero(&log_aspects, &geometry_mean_span),
            correlation_or_zero(&abs_log_aspects, &geometry_mean_span),
        );
        println!(
            "RAIN_005A3_GEOMETRY_DEPENDENCE metric=mound_el log_width={:.6} log_height={:.6} log_area={:.6} log_aspect={:.6} abs_log_aspect={:.6}",
            correlation_or_zero(&log_widths, &geometry_mean_mound_el),
            correlation_or_zero(&log_heights, &geometry_mean_mound_el),
            correlation_or_zero(&log_areas, &geometry_mean_mound_el),
            correlation_or_zero(&log_aspects, &geometry_mean_mound_el),
            correlation_or_zero(&abs_log_aspects, &geometry_mean_mound_el),
        );
    }

    #[test]
    #[ignore = "native RAIN-005A3 extended analytical arbitrary-geometry/nozzle sweep"]
    fn rain_005a3_extended_analytical_nozzle_geometry_probe() {
        // This analytical sweep is intentionally much denser than the expensive
        // morphology ensemble. It explores width/height logarithmically, focus
        // across the entire active width, and nearly-empty through nearly-full
        // sky, without choosing any user resolution as canonical.
        const CASES: usize = 4096;
        const MIN_WIDTH_DOTS: f64 = 40.0;
        const MAX_WIDTH_DOTS: f64 = 840.0;
        const MIN_HEIGHT_DOTS: f64 = 40.0;
        const MAX_HEIGHT_DOTS: f64 = 560.0;

        let ingress_rate_hz = MILLIS_PER_SECOND / TIME_SETTINGS.tick_ms as f64;
        let gravity_step_ms = TIME_SETTINGS.physics_ms.saturating_mul(2);
        let rows_per_second = MILLIS_PER_SECOND / gravity_step_ms as f64;

        let mut expected_airborne_values = Vec::with_capacity(CASES);
        let mut kl_values = Vec::with_capacity(CASES);
        let mut information_values = Vec::with_capacity(CASES);
        let mut kernel_width_values = Vec::with_capacity(CASES);
        let mut mound_el_values = Vec::with_capacity(CASES);
        let mut log_widths = Vec::with_capacity(CASES);
        let mut log_heights = Vec::with_capacity(CASES);
        let mut focus_edge_distances = Vec::with_capacity(CASES);
        let mut sky_fractions = Vec::with_capacity(CASES);

        for case in 1..=CASES {
            let width_dots = logarithmic_sample(MIN_WIDTH_DOTS, MAX_WIDTH_DOTS, halton(case, 2))
                .round()
                .max(2.0) as usize;
            let height_dots = logarithmic_sample(MIN_HEIGHT_DOTS, MAX_HEIGHT_DOTS, halton(case, 3))
                .round()
                .max(2.0) as usize;
            let focus_norm = halton(case, 5);
            let sky_fraction = 0.02 + 0.96 * halton(case, 7);
            let focus = ((width_dots - 1) as f64 * focus_norm).round() as usize;
            let free = (0..width_dots).collect::<Vec<_>>();
            let distribution =
                rain_distribution_metrics(&free, focus, RAIN_FOCUS_BIAS_PROBABILITY, width_dots);
            let mean_drop_rows = height_dots as f64 * sky_fraction;
            let expected_airborne = ingress_rate_hz * mean_drop_rows / rows_per_second;
            let information = expected_airborne * distribution.kl_bits_per_grain;
            let mound_el = distribution.one_dot_excess_mound_grains / width_dots as f64;
            let edge_distance = focus_norm.min(1.0 - focus_norm);

            expected_airborne_values.push(expected_airborne);
            kl_values.push(distribution.kl_bits_per_grain);
            information_values.push(information);
            kernel_width_values.push(distribution.focus_rms_width_fraction);
            mound_el_values.push(mound_el);
            log_widths.push((width_dots as f64).ln());
            log_heights.push((height_dots as f64).ln());
            focus_edge_distances.push(edge_distance);
            sky_fractions.push(sky_fraction);
        }

        println!(
            "RAIN_005A3_NOZZLE_ENSEMBLE cases={CASES} width_dots_min={MIN_WIDTH_DOTS:.0} width_dots_max={MAX_WIDTH_DOTS:.0} height_dots_min={MIN_HEIGHT_DOTS:.0} height_dots_max={MAX_HEIGHT_DOTS:.0}"
        );
        print_extended_percentiles("RAIN_005A3_EXPECTED_AIRBORNE", &expected_airborne_values);
        print_extended_percentiles("RAIN_005A3_KL_BITS_PER_GRAIN", &kl_values);
        print_extended_percentiles("RAIN_005A3_NOZZLE_INFORMATION_BITS", &information_values);
        print_extended_percentiles("RAIN_005A3_KERNEL_WIDTH_FRACTION", &kernel_width_values);
        print_extended_percentiles("RAIN_005A3_NOZZLE_MOUND_EL", &mound_el_values);
        println!(
            "RAIN_005A3_NOZZLE_DEPENDENCE information_vs_airborne={:.9} information_vs_log_width={:.9} information_vs_log_height={:.9} information_vs_sky_fraction={:.9} kl_vs_log_width={:.9} kl_vs_focus_edge_distance={:.9} kernel_width_vs_focus_edge_distance={:.9} mound_el_vs_log_width={:.9}",
            correlation_or_zero(&expected_airborne_values, &information_values),
            correlation_or_zero(&log_widths, &information_values),
            correlation_or_zero(&log_heights, &information_values),
            correlation_or_zero(&sky_fractions, &information_values),
            correlation_or_zero(&log_widths, &kl_values),
            correlation_or_zero(&focus_edge_distances, &kl_values),
            correlation_or_zero(&focus_edge_distances, &kernel_width_values),
            correlation_or_zero(&log_widths, &mound_el_values),
        );
    }

    #[derive(Debug, Clone, Copy)]
    struct Rain004ReferenceSample {
        run: usize,
        geometry: usize,
        seed_slot: usize,
        width_cells: usize,
        height_cells: usize,
        seed: u64,
        mound_el: f64,
        cv: f64,
        correlation: f64,
        total_variation: f64,
        centroid_shift: f64,
        centroid_span: f64,
    }

    fn rain_005a3_rain004_reference_samples() -> Vec<Rain004ReferenceSample> {
        const REFERENCE: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/rain_005a3_rain004_morph_reference.csv"
        ));
        REFERENCE
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let fields = line.split(',').collect::<Vec<_>>();
                assert_eq!(fields.len(), 16, "invalid RAIN-005A3 reference row: {line}");
                Rain004ReferenceSample {
                    run: fields[0].parse().expect("run"),
                    geometry: fields[1].parse().expect("geometry"),
                    seed_slot: fields[2].parse().expect("seed slot"),
                    width_cells: fields[3].parse().expect("terminal width"),
                    height_cells: fields[4].parse().expect("terminal height"),
                    seed: fields[9].parse().expect("seed"),
                    mound_el: fields[10].parse().expect("mound el"),
                    cv: fields[11].parse().expect("cv"),
                    correlation: fields[12].parse().expect("correlation"),
                    total_variation: fields[13].parse().expect("total variation"),
                    centroid_shift: fields[14].parse().expect("centroid shift"),
                    centroid_span: fields[15].parse().expect("centroid span"),
                }
            })
            .collect()
    }

    #[derive(Debug, Clone, Copy)]
    struct Rain005B2RuntimeControl {
        run: usize,
        geometry: usize,
        seed_slot: usize,
        width_cells: usize,
        height_cells: usize,
        seed: u64,
        delta_cv: f64,
        delta_corr: f64,
        delta_tv: f64,
        delta_shift: f64,
        delta_span: f64,
        candidate_shift: f64,
        candidate_span: f64,
    }

    fn rain_005b2_seed1_runtime_control() -> Vec<Rain005B2RuntimeControl> {
        const CONTROL: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/rain_005b2_seed1_runtime_control.csv"
        ));
        CONTROL
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let fields = line.split(',').collect::<Vec<_>>();
                assert_eq!(fields.len(), 13, "invalid RAIN-005B2 control row: {line}");
                Rain005B2RuntimeControl {
                    run: fields[0].parse().expect("run"),
                    geometry: fields[1].parse().expect("geometry"),
                    seed_slot: fields[2].parse().expect("seed slot"),
                    width_cells: fields[3].parse().expect("terminal width"),
                    height_cells: fields[4].parse().expect("terminal height"),
                    seed: fields[5].parse().expect("seed"),
                    delta_cv: fields[6].parse().expect("delta cv"),
                    delta_corr: fields[7].parse().expect("delta corr"),
                    delta_tv: fields[8].parse().expect("delta tv"),
                    delta_shift: fields[9].parse().expect("delta shift"),
                    delta_span: fields[10].parse().expect("delta span"),
                    candidate_shift: fields[11].parse().expect("candidate shift"),
                    candidate_span: fields[12].parse().expect("candidate span"),
                }
            })
            .collect()
    }

    fn rain_005b2d2_selected_boundary_control() -> Vec<Rain005B2RuntimeControl> {
        const CONTROL: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/rain_005b2d2_selected_boundary_control.csv"
        ));
        CONTROL
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let fields = line.split(',').collect::<Vec<_>>();
                assert_eq!(
                    fields.len(),
                    13,
                    "invalid RAIN-005B2D2 selected control row: {line}"
                );
                Rain005B2RuntimeControl {
                    run: fields[0].parse().expect("run"),
                    geometry: fields[1].parse().expect("geometry"),
                    seed_slot: fields[2].parse().expect("seed slot"),
                    width_cells: fields[3].parse().expect("terminal width"),
                    height_cells: fields[4].parse().expect("terminal height"),
                    seed: fields[5].parse().expect("seed"),
                    delta_cv: fields[6].parse().expect("delta cv"),
                    delta_corr: fields[7].parse().expect("delta corr"),
                    delta_tv: fields[8].parse().expect("delta tv"),
                    delta_shift: fields[9].parse().expect("delta shift"),
                    delta_span: fields[10].parse().expect("delta span"),
                    candidate_shift: fields[11].parse().expect("candidate shift"),
                    candidate_span: fields[12].parse().expect("candidate span"),
                }
            })
            .collect()
    }

    fn assert_printed_nine_eq(label: &str, actual: f64, expected: f64) {
        assert_eq!(
            format!("{actual:.9}"),
            format!("{expected:.9}"),
            "{label}: actual={actual:.12} expected={expected:.12}"
        );
    }

    #[test]
    #[ignore = "native RAIN-005B2 exact RAIN-004 IID control reproduction on a bounded A3 subset"]
    fn rain_005b2_iid_rain004_control_reproduction_probe() {
        const CATEGORY_COUNT: usize = 5;
        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("B2 control {id}")))
            .collect::<Vec<_>>();
        let reference = rain_005a3_rain004_reference_samples();
        let selected = reference
            .iter()
            .filter(|sample| sample.seed_slot == 1 && sample.geometry % 8 == 1)
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), 12);

        for expected in selected {
            let control = morphology_sample_for_geometry_with_bias_schedule(
                expected.width_cells,
                expected.height_cells,
                expected.seed,
                &categories,
                RainBiasScheduleProbe::IidRain004,
            );
            assert_printed_nine_eq("mound_el", control.mound_el, expected.mound_el);
            assert_printed_nine_eq("cv", control.cv, expected.cv);
            assert_printed_nine_eq("corr", control.correlation, expected.correlation);
            assert_printed_nine_eq("tv", control.total_variation, expected.total_variation);
            assert_printed_nine_eq("shift", control.centroid_shift, expected.centroid_shift);
            assert_printed_nine_eq("span", control.centroid_span, expected.centroid_span);
        }
        println!("RAIN_005B2_CONTROL_REPRODUCTION runs=12 result=PASS_SAMPLE_LEVEL_9DP");
    }

    #[test]
    #[ignore = "native RAIN-005B2 paired morphology ensemble against exact RAIN-004 A3 runs; intentionally long-running"]
    fn rain_005b2_paired_morphology_against_rain004_probe() {
        const CATEGORY_COUNT: usize = 5;
        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("B2 {id}")))
            .collect::<Vec<_>>();
        let reference = rain_005a3_rain004_reference_samples();
        assert_eq!(reference.len(), 192);

        let mut delta_cv = Vec::with_capacity(reference.len());
        let mut delta_corr = Vec::with_capacity(reference.len());
        let mut delta_tv = Vec::with_capacity(reference.len());
        let mut delta_shift = Vec::with_capacity(reference.len());
        let mut delta_span = Vec::with_capacity(reference.len());
        let mut candidate_shift = Vec::with_capacity(reference.len());
        let mut candidate_span = Vec::with_capacity(reference.len());
        let mut log_width = Vec::with_capacity(reference.len());

        for expected in &reference {
            let candidate = morphology_sample_for_geometry_with_bias_schedule(
                expected.width_cells,
                expected.height_cells,
                expected.seed,
                &categories,
                RainBiasScheduleProbe::B2Control,
            );
            assert!(
                (candidate.mound_el - expected.mound_el).abs() < 1e-6,
                "run={} candidate_mound_el={} reference_mound_el={}",
                expected.run,
                candidate.mound_el,
                expected.mound_el
            );

            let cv = candidate.cv - expected.cv;
            let corr = candidate.correlation - expected.correlation;
            let tv = candidate.total_variation - expected.total_variation;
            let shift = candidate.centroid_shift - expected.centroid_shift;
            let span = candidate.centroid_span - expected.centroid_span;
            delta_cv.push(cv);
            delta_corr.push(corr);
            delta_tv.push(tv);
            delta_shift.push(shift);
            delta_span.push(span);
            candidate_shift.push(candidate.centroid_shift);
            candidate_span.push(candidate.centroid_span);
            log_width.push((expected.width_cells as f64).ln());

            println!(
                "RAIN_005B2_PAIRED_SAMPLE run={} geometry={} seed_slot={} terminal={}x{} seed={} delta_cv={cv:.9} delta_corr={corr:.9} delta_tv={tv:.9} delta_shift={shift:.9} delta_span={span:.9} candidate_shift={:.9} candidate_span={:.9}",
                expected.run,
                expected.geometry,
                expected.seed_slot,
                expected.width_cells,
                expected.height_cells,
                expected.seed,
                candidate.centroid_shift,
                candidate.centroid_span,
            );
        }

        print_extended_percentiles("RAIN_005B2_DELTA_CV", &delta_cv);
        print_extended_percentiles("RAIN_005B2_DELTA_CORR", &delta_corr);
        print_extended_percentiles("RAIN_005B2_DELTA_TV", &delta_tv);
        print_extended_percentiles("RAIN_005B2_DELTA_SHIFT", &delta_shift);
        print_extended_percentiles("RAIN_005B2_DELTA_SPAN", &delta_span);
        println!(
            "RAIN_005B2_GEOMETRY_DEPENDENCE shift_vs_log_width={:.9} span_vs_log_width={:.9}",
            correlation_or_zero(&log_width, &candidate_shift),
            correlation_or_zero(&log_width, &candidate_span),
        );
        println!(
            "RAIN_005B2_PARETO_MEDIANS cv={:.9} corr={:.9} tv={:.9} shift={:.9} span={:.9}",
            percentile(&delta_cv, 0.50),
            percentile(&delta_corr, 0.50),
            percentile(&delta_tv, 0.50),
            percentile(&delta_shift, 0.50),
            percentile(&delta_span, 0.50),
        );
    }

    #[test]
    #[ignore = "native RAIN-005B2R1 full paired morphology ensemble against exact RAIN-004 A3 runs; intentionally long-running"]
    fn rain_005b2r1_paired_morphology_against_rain004_probe() {
        const CATEGORY_COUNT: usize = 5;
        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("B2R1 {id}")))
            .collect::<Vec<_>>();
        let reference = rain_005a3_rain004_reference_samples();
        let selected_control = rain_005b2d2_selected_boundary_control();
        assert_eq!(reference.len(), 192);
        assert_eq!(selected_control.len(), 64);

        let mut delta_cv = Vec::with_capacity(reference.len());
        let mut delta_corr = Vec::with_capacity(reference.len());
        let mut delta_tv = Vec::with_capacity(reference.len());
        let mut delta_shift = Vec::with_capacity(reference.len());
        let mut delta_span = Vec::with_capacity(reference.len());
        let mut candidate_shift = Vec::with_capacity(reference.len());
        let mut candidate_span = Vec::with_capacity(reference.len());
        let mut log_width = Vec::with_capacity(reference.len());

        for expected in &reference {
            let candidate = morphology_sample_for_geometry(
                expected.width_cells,
                expected.height_cells,
                expected.seed,
                &categories,
            );
            assert!(
                (candidate.mound_el - expected.mound_el).abs() < 1e-6,
                "run={} candidate_mound_el={} reference_mound_el={}",
                expected.run,
                candidate.mound_el,
                expected.mound_el
            );

            let cv = candidate.cv - expected.cv;
            let corr = candidate.correlation - expected.correlation;
            let tv = candidate.total_variation - expected.total_variation;
            let shift = candidate.centroid_shift - expected.centroid_shift;
            let span = candidate.centroid_span - expected.centroid_span;
            if expected.seed_slot == 1 && expected.geometry <= 64 {
                let expected_control = &selected_control[expected.geometry - 1];
                assert_eq!(expected.run, expected_control.run);
                assert_eq!(expected.geometry, expected_control.geometry);
                assert_eq!(expected.seed_slot, expected_control.seed_slot);
                assert_eq!(expected.width_cells, expected_control.width_cells);
                assert_eq!(expected.height_cells, expected_control.height_cells);
                assert_eq!(expected.seed, expected_control.seed);
                assert_printed_nine_eq("R1 promotion delta cv", cv, expected_control.delta_cv);
                assert_printed_nine_eq(
                    "R1 promotion delta corr",
                    corr,
                    expected_control.delta_corr,
                );
                assert_printed_nine_eq("R1 promotion delta tv", tv, expected_control.delta_tv);
                assert_printed_nine_eq(
                    "R1 promotion delta shift",
                    shift,
                    expected_control.delta_shift,
                );
                assert_printed_nine_eq(
                    "R1 promotion delta span",
                    span,
                    expected_control.delta_span,
                );
                assert_printed_nine_eq(
                    "R1 promotion candidate shift",
                    candidate.centroid_shift,
                    expected_control.candidate_shift,
                );
                assert_printed_nine_eq(
                    "R1 promotion candidate span",
                    candidate.centroid_span,
                    expected_control.candidate_span,
                );
            }
            delta_cv.push(cv);
            delta_corr.push(corr);
            delta_tv.push(tv);
            delta_shift.push(shift);
            delta_span.push(span);
            candidate_shift.push(candidate.centroid_shift);
            candidate_span.push(candidate.centroid_span);
            log_width.push((expected.width_cells as f64).ln());

            println!(
                "RAIN_005B2R1_PAIRED_SAMPLE run={} geometry={} seed_slot={} terminal={}x{} seed={} delta_cv={cv:.9} delta_corr={corr:.9} delta_tv={tv:.9} delta_shift={shift:.9} delta_span={span:.9} candidate_shift={:.9} candidate_span={:.9}",
                expected.run,
                expected.geometry,
                expected.seed_slot,
                expected.width_cells,
                expected.height_cells,
                expected.seed,
                candidate.centroid_shift,
                candidate.centroid_span,
            );
        }

        println!(
            "RAIN_005B2R1_PROMOTION_CONTROL_REPRODUCTION result=PASS_SAMPLE_LEVEL_9DP runs=64"
        );
        print_extended_percentiles("RAIN_005B2R1_DELTA_CV", &delta_cv);
        print_extended_percentiles("RAIN_005B2R1_DELTA_CORR", &delta_corr);
        print_extended_percentiles("RAIN_005B2R1_DELTA_TV", &delta_tv);
        print_extended_percentiles("RAIN_005B2R1_DELTA_SHIFT", &delta_shift);
        print_extended_percentiles("RAIN_005B2R1_DELTA_SPAN", &delta_span);
        let shift_vs_log_width = correlation_or_zero(&log_width, &candidate_shift);
        let span_vs_log_width = correlation_or_zero(&log_width, &candidate_span);
        let median_cv = percentile(&delta_cv, 0.50);
        let median_corr = percentile(&delta_corr, 0.50);
        let median_tv = percentile(&delta_tv, 0.50);
        let median_shift = percentile(&delta_shift, 0.50);
        let median_span = percentile(&delta_span, 0.50);
        println!(
            "RAIN_005B2R1_GEOMETRY_DEPENDENCE shift_vs_log_width={shift_vs_log_width:.9} span_vs_log_width={span_vs_log_width:.9}"
        );
        println!(
            "RAIN_005B2R1_PARETO_MEDIANS cv={median_cv:.9} corr={median_corr:.9} tv={median_tv:.9} shift={median_shift:.9} span={median_span:.9}"
        );
        let pareto_pass = median_cv >= 0.0
            && median_corr <= 0.0
            && median_tv >= 0.0
            && median_shift > 0.0
            && median_span > 0.0
            && shift_vs_log_width > -0.412_362;
        println!(
            "RAIN_005B2R1_PARETO_GATE pass={pareto_pass} a3_shift_vs_log_width_reference=-0.412362000"
        );
    }

    #[derive(Debug, Clone, Copy)]
    struct MacroreliefSample {
        legacy: MorphologySample,
        relief_d2: f64,
        relief_d4: f64,
        relief_d8: f64,
        curvature_d4: f64,
        curvature_d8: f64,
        pinchout_fraction: f64,
        continuity_fraction: f64,
        avalanche_mean_moves: f64,
        avalanche_p95_moves: f64,
        avalanche_max_moves: f64,
        avalanche_cascade_fraction: f64,
        avalanche_p95_span: f64,
        avalanche_max_span: f64,
        avalanche_multicolumn_fraction: f64,
        pending_fraction: f64,
    }

    #[derive(Debug, Default)]
    struct AvalancheObservation {
        initialized: bool,
        last_diagonal_moves: usize,
        moves: Vec<f64>,
        spans: Vec<f64>,
    }

    fn advance_exact_grains_observed(
        engine: &mut ClassicSandboxEngine,
        grains: usize,
        category_id: CategoryId,
        observation: &mut AvalancheObservation,
    ) {
        use std::time::Duration;

        let tick = Duration::from_millis(TIME_SETTINGS.tick_ms);
        let physics = Duration::from_millis(TIME_SETTINGS.physics_ms);
        let simulated = Duration::from_millis(
            TIME_SETTINGS
                .tick_ms
                .saturating_mul(u64::try_from(grains).expect("diagnostic grain count fits u64")),
        );
        let mut spawn_accumulator = Duration::ZERO;
        let mut physics_accumulator = Duration::ZERO;
        let mut remaining = simulated;

        while !remaining.is_zero() {
            let spawn_left = tick.saturating_sub(spawn_accumulator);
            let physics_left = physics.saturating_sub(physics_accumulator);
            let step = remaining.min(spawn_left.min(physics_left));
            spawn_accumulator += step;
            physics_accumulator += step;
            remaining = remaining.saturating_sub(step);

            let spawn_due = spawn_accumulator >= tick;
            let physics_due = physics_accumulator >= physics;
            if spawn_due {
                spawn_accumulator = spawn_accumulator.saturating_sub(tick);
                if observation.initialized {
                    observation.moves.push(
                        engine
                            .diagonal_moves
                            .saturating_sub(observation.last_diagonal_moves)
                            as f64,
                    );
                    observation
                        .spans
                        .push(engine.diagnostic_diagonal_span() as f64);
                } else {
                    observation.initialized = true;
                }
                engine.reset_diagnostic_diagonal_span();
                observation.last_diagonal_moves = engine.diagonal_moves;
                engine.spawn(category_id);
            }
            if physics_due {
                physics_accumulator = physics_accumulator.saturating_sub(physics);
                engine.update();
            }
            assert!(step > Duration::ZERO || spawn_due || physics_due);
        }
    }

    fn supported_height_profile_for_bounds(
        engine: &ClassicSandboxEngine,
        bounds: super::super::ViewportBounds,
    ) -> Vec<f64> {
        (bounds.x_start..bounds.x_end)
            .map(|x| engine.supported_column_height(x) as f64)
            .collect()
    }

    fn dyadic_block_means(samples: &[f64], blocks: usize) -> Vec<f64> {
        assert!(blocks > 0 && samples.len() >= blocks);
        (0..blocks)
            .map(|block| {
                let start = block * samples.len() / blocks;
                let end = (block + 1) * samples.len() / blocks;
                mean(&samples[start..end]).expect("non-empty dyadic block")
            })
            .collect()
    }

    fn normalized_block_range(samples: &[f64], blocks: usize) -> f64 {
        let block_means = dyadic_block_means(samples, blocks);
        let low = block_means.iter().copied().fold(f64::INFINITY, f64::min);
        let high = block_means
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let scale = mean(samples).unwrap_or(0.0).max(1.0);
        (high - low) / scale
    }

    fn normalized_block_curvature(samples: &[f64], blocks: usize) -> f64 {
        let block_means = dyadic_block_means(samples, blocks);
        let curvature = block_means
            .windows(3)
            .map(|window| (window[0] - 2.0 * window[1] + window[2]).abs())
            .collect::<Vec<_>>();
        let scale = mean(samples).unwrap_or(0.0).max(1.0);
        mean(&curvature).unwrap_or(0.0) / scale
    }

    fn profile_pinchout_and_continuity(profiles: &[CategoryProfile]) -> (f64, f64) {
        let mut pinchout = Vec::with_capacity(profiles.len());
        let mut continuity = Vec::with_capacity(profiles.len());
        for profile in profiles {
            let width = profile.thickness.len();
            if width == 0 {
                continue;
            }
            let nonzero = profile
                .thickness
                .iter()
                .filter(|value| **value > 0.0)
                .count();
            let zero = width.saturating_sub(nonzero);
            pinchout.push(zero as f64 / width as f64);

            let mut largest_run = 0usize;
            let mut current_run = 0usize;
            for value in &profile.thickness {
                if *value > 0.0 {
                    current_run += 1;
                    largest_run = largest_run.max(current_run);
                } else {
                    current_run = 0;
                }
            }
            continuity.push(if nonzero == 0 {
                0.0
            } else {
                largest_run as f64 / nonzero as f64
            });
        }
        (
            mean(&pinchout).unwrap_or(0.0),
            mean(&continuity).unwrap_or(0.0),
        )
    }

    fn macrorelief_sample_for_geometry(
        width_cells: usize,
        height_cells: usize,
        seed: u64,
        categories: &[Category],
        probe: ReposeStabilityProbe,
    ) -> MacroreliefSample {
        let mut engine = ClassicSandboxEngine::new(
            u16::try_from(width_cells).expect("diagnostic width fits u16"),
            u16::try_from(height_cells).expect("diagnostic height fits u16"),
            seed,
            ClassicRainMode::WanderingFocus,
        );
        engine.force_reference_gravity = false;
        engine.repose_stability_probe = probe;
        engine.repose_rng_state = engine.initial_repose_rng_state;
        engine.resample_all_local_repose();

        let bounds = engine.surface.viewport_bounds().expect("viewport");
        let width_dots = bounds.x_end - bounds.x_start;
        let free = (bounds.x_start..bounds.x_end).collect::<Vec<_>>();
        let center = bounds.x_start + width_dots.saturating_sub(1) / 2;
        let kernel =
            rain_distribution_metrics(&free, center, RAIN_FOCUS_BIAS_PROBABILITY, width_dots);
        let mound_grains = kernel.one_dot_excess_mound_grains.round().max(1.0) as usize;
        let mound_el = kernel.one_dot_excess_mound_grains / width_dots as f64;

        let mut observation = AvalancheObservation::default();
        engine.reset_diagnostic_diagonal_span();
        for category in categories {
            advance_exact_grains_observed(&mut engine, mound_grains, category.id, &mut observation);
        }

        assert_eq!(
            engine.physical_grain_count() + engine.pending_count(),
            engine.grain_count(),
            "macrorelief diagnostic must conserve physical + pending mass"
        );

        let profiles = engine.category_profiles(bounds, categories);
        let pairs = adjacent_profile_metrics(&profiles, width_dots);
        assert!(profiles.len() >= categories.len().saturating_sub(1));
        assert!(pairs.len() >= categories.len().saturating_sub(2));

        let cv = mean(
            &profiles
                .iter()
                .map(|profile| profile.thickness_cv)
                .collect::<Vec<_>>(),
        )
        .expect("profiles");
        let correlation = mean_defined(pairs.iter().map(|pair| pair.correlation)).unwrap_or(0.0);
        let total_variation = mean(
            &pairs
                .iter()
                .map(|pair| pair.total_variation)
                .collect::<Vec<_>>(),
        )
        .expect("pairs");
        let centroid_shift = mean(
            &pairs
                .iter()
                .map(|pair| pair.centroid_shift_fraction)
                .collect::<Vec<_>>(),
        )
        .expect("pairs");
        let min_centroid = profiles
            .iter()
            .map(|profile| profile.centroid_x_norm)
            .fold(f64::INFINITY, f64::min);
        let max_centroid = profiles
            .iter()
            .map(|profile| profile.centroid_x_norm)
            .fold(f64::NEG_INFINITY, f64::max);

        let heights = supported_height_profile_for_bounds(&engine, bounds);
        let (pinchout_fraction, continuity_fraction) = profile_pinchout_and_continuity(&profiles);
        let cascade_fraction = if observation.moves.is_empty() {
            0.0
        } else {
            observation
                .moves
                .iter()
                .filter(|moves| **moves > 1.0)
                .count() as f64
                / observation.moves.len() as f64
        };
        let multicolumn_fraction = if observation.spans.is_empty() {
            0.0
        } else {
            observation.spans.iter().filter(|span| **span > 1.0).count() as f64
                / observation.spans.len() as f64
        };

        MacroreliefSample {
            legacy: MorphologySample {
                cv,
                correlation,
                total_variation,
                centroid_shift,
                centroid_span: max_centroid - min_centroid,
                mound_el,
            },
            relief_d2: normalized_block_range(&heights, 2),
            relief_d4: normalized_block_range(&heights, 4),
            relief_d8: normalized_block_range(&heights, 8),
            curvature_d4: normalized_block_curvature(&heights, 4),
            curvature_d8: normalized_block_curvature(&heights, 8),
            pinchout_fraction,
            continuity_fraction,
            avalanche_mean_moves: mean(&observation.moves).unwrap_or(0.0),
            avalanche_p95_moves: percentile(&observation.moves, 0.95),
            avalanche_max_moves: observation.moves.iter().copied().fold(0.0, f64::max),
            avalanche_cascade_fraction: cascade_fraction,
            avalanche_p95_span: percentile(&observation.spans, 0.95),
            avalanche_max_span: observation.spans.iter().copied().fold(0.0, f64::max),
            avalanche_multicolumn_fraction: multicolumn_fraction,
            pending_fraction: if engine.grain_count() == 0 {
                0.0
            } else {
                engine.pending_count() as f64 / engine.grain_count() as f64
            },
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct MacroreliefSummary {
        probe: ReposeStabilityProbe,
        runs: usize,
        relief_d2: f64,
        relief_d4: f64,
        relief_d8: f64,
        curvature_d4: f64,
        curvature_d8: f64,
        thickness_cv: f64,
        pinchout_fraction: f64,
        continuity_fraction: f64,
        legacy_corr: f64,
        legacy_tv: f64,
        legacy_shift: f64,
        legacy_span: f64,
        avalanche_mean_moves: f64,
        avalanche_p95_moves: f64,
        avalanche_max_moves: f64,
        avalanche_cascade_fraction: f64,
        avalanche_p95_span: f64,
        avalanche_max_span: f64,
        avalanche_multicolumn_fraction: f64,
        pending_fraction: f64,
    }

    impl MacroreliefSummary {
        fn morphology_objectives(self) -> [f64; 8] {
            [
                self.relief_d2,
                self.relief_d4,
                self.relief_d8,
                self.curvature_d4,
                self.curvature_d8,
                self.thickness_cv,
                self.pinchout_fraction,
                self.continuity_fraction,
            ]
        }

        fn macro_objectives(self) -> [f64; 5] {
            [
                self.relief_d2,
                self.relief_d4,
                self.relief_d8,
                self.curvature_d4,
                self.curvature_d8,
            ]
        }

        fn avalanche_semantically_alive(self) -> bool {
            self.avalanche_max_moves > 1.0
                && self.avalanche_max_span > 1.0
                && self.avalanche_cascade_fraction > 0.0
                && self.avalanche_multicolumn_fraction > 0.0
        }

        fn avalanche_not_jointly_worse_than(self, control: Self) -> bool {
            !(self.avalanche_p95_moves < control.avalanche_p95_moves
                && self.avalanche_p95_span < control.avalanche_p95_span
                && self.avalanche_cascade_fraction < control.avalanche_cascade_fraction
                && self.avalanche_multicolumn_fraction < control.avalanche_multicolumn_fraction)
        }

        fn avalanche_safe(self, control: Self) -> bool {
            self.pending_fraction <= control.pending_fraction
                && self.avalanche_semantically_alive()
                && self.avalanche_not_jointly_worse_than(control)
        }
    }

    fn summarize_macrorelief(
        probe: ReposeStabilityProbe,
        samples: &[MacroreliefSample],
    ) -> MacroreliefSummary {
        let med = |f: fn(&MacroreliefSample) -> f64| {
            percentile(&samples.iter().map(f).collect::<Vec<_>>(), 0.50)
        };
        MacroreliefSummary {
            probe,
            runs: samples.len(),
            relief_d2: med(|sample| sample.relief_d2),
            relief_d4: med(|sample| sample.relief_d4),
            relief_d8: med(|sample| sample.relief_d8),
            curvature_d4: med(|sample| sample.curvature_d4),
            curvature_d8: med(|sample| sample.curvature_d8),
            thickness_cv: med(|sample| sample.legacy.cv),
            pinchout_fraction: med(|sample| sample.pinchout_fraction),
            continuity_fraction: med(|sample| sample.continuity_fraction),
            legacy_corr: med(|sample| sample.legacy.correlation),
            legacy_tv: med(|sample| sample.legacy.total_variation),
            legacy_shift: med(|sample| sample.legacy.centroid_shift),
            legacy_span: med(|sample| sample.legacy.centroid_span),
            avalanche_mean_moves: med(|sample| sample.avalanche_mean_moves),
            avalanche_p95_moves: med(|sample| sample.avalanche_p95_moves),
            avalanche_max_moves: med(|sample| sample.avalanche_max_moves),
            avalanche_cascade_fraction: med(|sample| sample.avalanche_cascade_fraction),
            avalanche_p95_span: med(|sample| sample.avalanche_p95_span),
            avalanche_max_span: med(|sample| sample.avalanche_max_span),
            avalanche_multicolumn_fraction: med(|sample| sample.avalanche_multicolumn_fraction),
            pending_fraction: med(|sample| sample.pending_fraction),
        }
    }

    fn macrorelief_dominates(left: MacroreliefSummary, right: MacroreliefSummary) -> bool {
        let left = left.morphology_objectives();
        let right = right.morphology_objectives();
        left.iter().zip(right).all(|(l, r)| *l >= r) && left.iter().zip(right).any(|(l, r)| *l > r)
    }

    fn rank_for_metric(
        summaries: &[MacroreliefSummary],
        candidate: MacroreliefSummary,
        metric: usize,
        macro_only: bool,
    ) -> usize {
        let candidate_value = if macro_only {
            candidate.macro_objectives()[metric]
        } else {
            candidate.morphology_objectives()[metric]
        };
        1 + summaries
            .iter()
            .filter(|other| {
                let value = if macro_only {
                    other.macro_objectives()[metric]
                } else {
                    other.morphology_objectives()[metric]
                };
                value > candidate_value
            })
            .count()
    }

    fn worst_rank(
        summaries: &[MacroreliefSummary],
        candidate: MacroreliefSummary,
        macro_only: bool,
    ) -> usize {
        let count = if macro_only { 5 } else { 8 };
        (0..count)
            .map(|metric| rank_for_metric(summaries, candidate, metric, macro_only))
            .max()
            .unwrap_or(usize::MAX)
    }

    fn mechanism_count(probe: ReposeStabilityProbe) -> usize {
        match probe {
            ReposeStabilityProbe::RuntimeControl => 0,
            ReposeStabilityProbe::ConvexityAnchorLocked
            | ReposeStabilityProbe::ConvexityStrongTail => 2,
            _ => 1,
        }
    }

    fn print_macrorelief_summary(stage: &str, summary: MacroreliefSummary, safe: bool) {
        println!(
            "RAIN_005C1_{stage}_SUMMARY variant={} runs={} relief_d2={:.9} relief_d4={:.9} relief_d8={:.9} curvature_d4={:.9} curvature_d8={:.9} thickness_cv={:.9} pinchout={:.9} continuity={:.9} legacy_corr={:.9} legacy_tv={:.9} legacy_shift={:.9} legacy_span={:.9} avalanche_mean_moves={:.9} avalanche_p95_moves={:.9} avalanche_max_moves={:.9} avalanche_cascade_fraction={:.9} avalanche_p95_span={:.9} avalanche_max_span={:.9} avalanche_multicolumn_fraction={:.9} pending_fraction={:.9} avalanche_safe={safe}",
            summary.probe.name(),
            summary.runs,
            summary.relief_d2,
            summary.relief_d4,
            summary.relief_d8,
            summary.curvature_d4,
            summary.curvature_d8,
            summary.thickness_cv,
            summary.pinchout_fraction,
            summary.continuity_fraction,
            summary.legacy_corr,
            summary.legacy_tv,
            summary.legacy_shift,
            summary.legacy_span,
            summary.avalanche_mean_moves,
            summary.avalanche_p95_moves,
            summary.avalanche_max_moves,
            summary.avalanche_cascade_fraction,
            summary.avalanche_p95_span,
            summary.avalanche_max_span,
            summary.avalanche_multicolumn_fraction,
            summary.pending_fraction,
        );
    }

    fn macrorelief_probe_order() -> [ReposeStabilityProbe; 9] {
        [
            ReposeStabilityProbe::RuntimeControl,
            ReposeStabilityProbe::NoAnchor,
            ReposeStabilityProbe::StrongTail,
            ReposeStabilityProbe::AnchorLocked,
            ReposeStabilityProbe::StrongLocked,
            ReposeStabilityProbe::HeightRoute,
            ReposeStabilityProbe::ConvexityRoute,
            ReposeStabilityProbe::ConvexityAnchorLocked,
            ReposeStabilityProbe::ConvexityStrongTail,
        ]
    }

    fn select_stage1_finalists(
        summaries: &[MacroreliefSummary],
        control: MacroreliefSummary,
    ) -> Vec<ReposeStabilityProbe> {
        let safe = summaries
            .iter()
            .copied()
            .filter(|summary| {
                summary.probe == ReposeStabilityProbe::RuntimeControl
                    || summary.avalanche_safe(control)
            })
            .collect::<Vec<_>>();
        let frontier = safe
            .iter()
            .copied()
            .filter(|candidate| {
                !safe.iter().copied().any(|other| {
                    other.probe != candidate.probe && macrorelief_dominates(other, *candidate)
                })
            })
            .collect::<Vec<_>>();
        println!(
            "RAIN_005C1_STAGE1_SAFE_PARETO variants={}",
            frontier
                .iter()
                .map(|summary| summary.probe.name())
                .collect::<Vec<_>>()
                .join(",")
        );

        let non_control = frontier
            .iter()
            .copied()
            .filter(|summary| summary.probe != ReposeStabilityProbe::RuntimeControl)
            .collect::<Vec<_>>();
        if non_control.is_empty() {
            return Vec::new();
        }

        let min_mechanisms = non_control
            .iter()
            .map(|summary| mechanism_count(summary.probe))
            .min()
            .expect("non-control frontier");
        let simple_pool = non_control
            .iter()
            .copied()
            .filter(|summary| mechanism_count(summary.probe) == min_mechanisms)
            .collect::<Vec<_>>();
        let simple = simple_pool
            .iter()
            .copied()
            .min_by_key(|summary| (worst_rank(&safe, *summary, false), summary.probe.name()))
            .expect("simple pool");
        let macro_best = non_control
            .iter()
            .copied()
            .min_by_key(|summary| {
                (
                    worst_rank(&safe, *summary, true),
                    mechanism_count(summary.probe),
                    summary.probe.name(),
                )
            })
            .expect("non-control frontier");

        let mut selected = vec![simple.probe];
        if macro_best.probe != simple.probe {
            selected.push(macro_best.probe);
        }
        println!(
            "RAIN_005C1_STAGE1_SELECTION simple={} macro={} unique_finalists={}",
            simple.probe.name(),
            macro_best.probe.name(),
            selected.len()
        );
        selected
    }

    #[test]
    fn rain_005c1_runtime_control_preserves_r1_reference_sample() {
        const CATEGORY_COUNT: usize = 5;
        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("C1 control {id}")))
            .collect::<Vec<_>>();
        let reference = rain_005a3_rain004_reference_samples();
        let expected = reference
            .iter()
            .find(|sample| sample.geometry == 1 && sample.seed_slot == 1)
            .expect("A3 geometry 1 seed 1");
        let selected = rain_005b2d2_selected_boundary_control();
        let expected_r1 = &selected[0];
        let sample = macrorelief_sample_for_geometry(
            expected.width_cells,
            expected.height_cells,
            expected.seed,
            &categories,
            ReposeStabilityProbe::RuntimeControl,
        );
        assert_printed_nine_eq(
            "C1 control mound",
            sample.legacy.mound_el,
            expected.mound_el,
        );
        assert_printed_nine_eq(
            "C1 control delta cv",
            sample.legacy.cv - expected.cv,
            expected_r1.delta_cv,
        );
        assert_printed_nine_eq(
            "C1 control delta corr",
            sample.legacy.correlation - expected.correlation,
            expected_r1.delta_corr,
        );
        assert_printed_nine_eq(
            "C1 control delta tv",
            sample.legacy.total_variation - expected.total_variation,
            expected_r1.delta_tv,
        );
        assert_printed_nine_eq(
            "C1 control delta shift",
            sample.legacy.centroid_shift - expected.centroid_shift,
            expected_r1.delta_shift,
        );
        assert_printed_nine_eq(
            "C1 control delta span",
            sample.legacy.centroid_span - expected.centroid_span,
            expected_r1.delta_span,
        );
    }

    #[test]
    fn rain_005c1_repose_probe_is_test_only_and_budget_routing_is_permutative() {
        let mut engine = ClassicSandboxEngine::new(30, 20, 0xC1A5, ClassicRainMode::WanderingFocus);
        let before = engine.local_repose.clone();
        engine.repose_stability_probe = ReposeStabilityProbe::ConvexityRoute;
        engine.route_local_repose_probe(10);
        let mut before_sorted = before[9..12].to_vec();
        let mut after_sorted = engine.local_repose[9..12].to_vec();
        before_sorted.sort_unstable();
        after_sorted.sort_unstable();
        assert_eq!(before_sorted, after_sorted);
    }

    #[test]
    #[ignore = "native unattended RAIN-005C1 macrorelief stability suite: 64x9 stage-1 plus full 192 control and up to two automatically selected finalists"]
    fn rain_005c1_macrorelief_stability_unattended_suite() {
        const STAGE1_GEOMETRIES: usize = 64;
        const CATEGORY_COUNT: usize = 5;
        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("C1 {id}")))
            .collect::<Vec<_>>();
        let reference = rain_005a3_rain004_reference_samples();
        let selected_r1 = rain_005b2d2_selected_boundary_control();
        assert_eq!(reference.len(), 192);
        assert_eq!(selected_r1.len(), 64);

        let stage1_reference = reference
            .iter()
            .filter(|sample| sample.geometry <= STAGE1_GEOMETRIES && sample.seed_slot == 1)
            .collect::<Vec<_>>();
        assert_eq!(stage1_reference.len(), STAGE1_GEOMETRIES);

        let mut stage1_summaries = Vec::new();
        for probe in macrorelief_probe_order() {
            let mut samples = Vec::with_capacity(STAGE1_GEOMETRIES);
            for expected in &stage1_reference {
                let sample = macrorelief_sample_for_geometry(
                    expected.width_cells,
                    expected.height_cells,
                    expected.seed,
                    &categories,
                    probe,
                );
                if probe == ReposeStabilityProbe::RuntimeControl {
                    let expected_r1 = &selected_r1[expected.geometry - 1];
                    assert_printed_nine_eq(
                        "C1 stage1 control delta cv",
                        sample.legacy.cv - expected.cv,
                        expected_r1.delta_cv,
                    );
                    assert_printed_nine_eq(
                        "C1 stage1 control delta corr",
                        sample.legacy.correlation - expected.correlation,
                        expected_r1.delta_corr,
                    );
                    assert_printed_nine_eq(
                        "C1 stage1 control delta tv",
                        sample.legacy.total_variation - expected.total_variation,
                        expected_r1.delta_tv,
                    );
                    assert_printed_nine_eq(
                        "C1 stage1 control delta shift",
                        sample.legacy.centroid_shift - expected.centroid_shift,
                        expected_r1.delta_shift,
                    );
                    assert_printed_nine_eq(
                        "C1 stage1 control delta span",
                        sample.legacy.centroid_span - expected.centroid_span,
                        expected_r1.delta_span,
                    );
                }
                println!(
                    "RAIN_005C1_STAGE1_SAMPLE variant={} geometry={} terminal={}x{} seed={} relief_d2={:.9} relief_d4={:.9} relief_d8={:.9} curvature_d4={:.9} curvature_d8={:.9} thickness_cv={:.9} pinchout={:.9} continuity={:.9} avalanche_p95_moves={:.9} avalanche_p95_span={:.9}",
                    probe.name(),
                    expected.geometry,
                    expected.width_cells,
                    expected.height_cells,
                    expected.seed,
                    sample.relief_d2,
                    sample.relief_d4,
                    sample.relief_d8,
                    sample.curvature_d4,
                    sample.curvature_d8,
                    sample.legacy.cv,
                    sample.pinchout_fraction,
                    sample.continuity_fraction,
                    sample.avalanche_p95_moves,
                    sample.avalanche_p95_span,
                );
                samples.push(sample);
            }
            stage1_summaries.push(summarize_macrorelief(probe, &samples));
        }
        println!("RAIN_005C1_STAGE1_CONTROL_REPRODUCTION result=PASS_SAMPLE_LEVEL_9DP runs=64");
        let stage1_control = stage1_summaries
            .iter()
            .copied()
            .find(|summary| summary.probe == ReposeStabilityProbe::RuntimeControl)
            .expect("stage1 control");
        for summary in &stage1_summaries {
            print_macrorelief_summary(
                "STAGE1",
                *summary,
                summary.probe == ReposeStabilityProbe::RuntimeControl
                    || summary.avalanche_safe(stage1_control),
            );
        }

        let finalists = select_stage1_finalists(&stage1_summaries, stage1_control);
        if finalists.is_empty() {
            println!(
                "RAIN_005C1_FINAL classification=NO_SAFE_NONCONTROL_STAGE1_FRONTIER human_candidates=0"
            );
            return;
        }

        let mut full_probes = vec![ReposeStabilityProbe::RuntimeControl];
        for probe in finalists {
            if !full_probes.contains(&probe) {
                full_probes.push(probe);
            }
        }

        let mut full_summaries = Vec::new();
        for probe in full_probes {
            let mut samples = Vec::with_capacity(reference.len());
            for expected in &reference {
                let sample = macrorelief_sample_for_geometry(
                    expected.width_cells,
                    expected.height_cells,
                    expected.seed,
                    &categories,
                    probe,
                );
                println!(
                    "RAIN_005C1_FULL_SAMPLE variant={} run={} geometry={} seed_slot={} terminal={}x{} seed={} relief_d2={:.9} relief_d4={:.9} relief_d8={:.9} curvature_d4={:.9} curvature_d8={:.9} thickness_cv={:.9} pinchout={:.9} continuity={:.9} legacy_corr={:.9} legacy_tv={:.9} legacy_shift={:.9} legacy_span={:.9} avalanche_p95_moves={:.9} avalanche_p95_span={:.9}",
                    probe.name(),
                    expected.run,
                    expected.geometry,
                    expected.seed_slot,
                    expected.width_cells,
                    expected.height_cells,
                    expected.seed,
                    sample.relief_d2,
                    sample.relief_d4,
                    sample.relief_d8,
                    sample.curvature_d4,
                    sample.curvature_d8,
                    sample.legacy.cv,
                    sample.pinchout_fraction,
                    sample.continuity_fraction,
                    sample.legacy.correlation,
                    sample.legacy.total_variation,
                    sample.legacy.centroid_shift,
                    sample.legacy.centroid_span,
                    sample.avalanche_p95_moves,
                    sample.avalanche_p95_span,
                );
                samples.push(sample);
            }
            full_summaries.push(summarize_macrorelief(probe, &samples));
        }

        let full_control = full_summaries
            .iter()
            .copied()
            .find(|summary| summary.probe == ReposeStabilityProbe::RuntimeControl)
            .expect("full control");
        for summary in &full_summaries {
            print_macrorelief_summary(
                "FULL",
                *summary,
                summary.probe == ReposeStabilityProbe::RuntimeControl
                    || summary.avalanche_safe(full_control),
            );
        }

        let full_safe = full_summaries
            .iter()
            .copied()
            .filter(|summary| {
                summary.probe == ReposeStabilityProbe::RuntimeControl
                    || summary.avalanche_safe(full_control)
            })
            .collect::<Vec<_>>();
        let full_frontier = full_safe
            .iter()
            .copied()
            .filter(|candidate| {
                !full_safe.iter().copied().any(|other| {
                    other.probe != candidate.probe && macrorelief_dominates(other, *candidate)
                })
            })
            .collect::<Vec<_>>();
        let human = full_frontier
            .iter()
            .copied()
            .filter(|candidate| {
                candidate.probe != ReposeStabilityProbe::RuntimeControl
                    && candidate
                        .macro_objectives()
                        .iter()
                        .zip(full_control.macro_objectives())
                        .any(|(value, control)| *value > control)
            })
            .collect::<Vec<_>>();
        println!(
            "RAIN_005C1_FULL_SAFE_PARETO variants={}",
            full_frontier
                .iter()
                .map(|summary| summary.probe.name())
                .collect::<Vec<_>>()
                .join(",")
        );
        println!(
            "RAIN_005C1_FINAL classification={} human_candidates={} variants={}",
            if human.is_empty() {
                "NO_FULL_HUMAN_CANDIDATE"
            } else {
                "FULL_HUMAN_CANDIDATES_READY"
            },
            human.len(),
            human
                .iter()
                .map(|summary| summary.probe.name())
                .collect::<Vec<_>>()
                .join(",")
        );
    }

    #[test]
    #[ignore = "native RAIN-005B2D1 test-only focus-authority sweep; 64 A3 geometries x one exact seed x five dyadic authority levels"]
    fn rain_005b2d1_focus_authority_sweep_probe() {
        const MAX_GEOMETRY: usize = 64;
        const CATEGORY_COUNT: usize = 5;
        const VARIANTS: [RainBiasScheduleProbe; 5] = [
            RainBiasScheduleProbe::B2Control,
            RainBiasScheduleProbe::AuthorityTowardOne16,
            RainBiasScheduleProbe::AuthorityTowardOne8,
            RainBiasScheduleProbe::AuthorityTowardOne4,
            RainBiasScheduleProbe::AuthorityTowardOne2,
        ];

        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("B2D1 {id}")))
            .collect::<Vec<_>>();
        let reference = rain_005a3_rain004_reference_samples()
            .into_iter()
            .filter(|sample| sample.geometry <= MAX_GEOMETRY && sample.seed_slot == 1)
            .collect::<Vec<_>>();
        let control = rain_005b2_seed1_runtime_control();
        assert_eq!(reference.len(), MAX_GEOMETRY);
        assert_eq!(control.len(), MAX_GEOMETRY);

        let baseline_log_width = reference
            .iter()
            .map(|sample| (sample.width_cells as f64).ln())
            .collect::<Vec<_>>();
        let baseline_shift = reference
            .iter()
            .map(|sample| sample.centroid_shift)
            .collect::<Vec<_>>();
        let baseline_span = reference
            .iter()
            .map(|sample| sample.centroid_span)
            .collect::<Vec<_>>();
        let baseline_shift_vs_log_width = correlation_or_zero(&baseline_log_width, &baseline_shift);
        println!(
            "RAIN_005B2D1_BASELINE runs={} shift_vs_log_width={:.9} span_vs_log_width={:.9}",
            reference.len(),
            baseline_shift_vs_log_width,
            correlation_or_zero(&baseline_log_width, &baseline_span),
        );

        let mut selected: Option<(&'static str, f64)> = None;
        for variant in VARIANTS {
            let name = variant.diagnostic_name();
            let requested_probability = variant.focus_probability();
            let effective_probability =
                ClassicSandboxEngine::rain_focus_bias_effective_probability_for(
                    requested_probability,
                );
            let n4_max = (4.0 * effective_probability).ceil() as usize;
            let n20_max = (20.0 * effective_probability).ceil() as usize;

            let mut delta_cv = Vec::with_capacity(reference.len());
            let mut delta_corr = Vec::with_capacity(reference.len());
            let mut delta_tv = Vec::with_capacity(reference.len());
            let mut delta_shift = Vec::with_capacity(reference.len());
            let mut delta_span = Vec::with_capacity(reference.len());
            let mut candidate_shift = Vec::with_capacity(reference.len());
            let mut candidate_span = Vec::with_capacity(reference.len());
            let mut log_width = Vec::with_capacity(reference.len());

            for (expected, expected_control) in reference.iter().zip(&control) {
                assert_eq!(expected.run, expected_control.run);
                assert_eq!(expected.geometry, expected_control.geometry);
                assert_eq!(expected.seed_slot, expected_control.seed_slot);
                assert_eq!(expected.width_cells, expected_control.width_cells);
                assert_eq!(expected.height_cells, expected_control.height_cells);
                assert_eq!(expected.seed, expected_control.seed);

                let candidate = morphology_sample_for_geometry_with_bias_schedule(
                    expected.width_cells,
                    expected.height_cells,
                    expected.seed,
                    &categories,
                    variant,
                );
                assert!(
                    (candidate.mound_el - expected.mound_el).abs() < 1e-6,
                    "variant={name} run={} candidate_mound_el={} reference_mound_el={}",
                    expected.run,
                    candidate.mound_el,
                    expected.mound_el
                );

                let cv = candidate.cv - expected.cv;
                let corr = candidate.correlation - expected.correlation;
                let tv = candidate.total_variation - expected.total_variation;
                let shift = candidate.centroid_shift - expected.centroid_shift;
                let span = candidate.centroid_span - expected.centroid_span;
                if variant == RainBiasScheduleProbe::B2Control {
                    assert_printed_nine_eq("runtime delta cv", cv, expected_control.delta_cv);
                    assert_printed_nine_eq("runtime delta corr", corr, expected_control.delta_corr);
                    assert_printed_nine_eq("runtime delta tv", tv, expected_control.delta_tv);
                    assert_printed_nine_eq(
                        "runtime delta shift",
                        shift,
                        expected_control.delta_shift,
                    );
                    assert_printed_nine_eq("runtime delta span", span, expected_control.delta_span);
                    assert_printed_nine_eq(
                        "runtime candidate shift",
                        candidate.centroid_shift,
                        expected_control.candidate_shift,
                    );
                    assert_printed_nine_eq(
                        "runtime candidate span",
                        candidate.centroid_span,
                        expected_control.candidate_span,
                    );
                }

                delta_cv.push(cv);
                delta_corr.push(corr);
                delta_tv.push(tv);
                delta_shift.push(shift);
                delta_span.push(span);
                candidate_shift.push(candidate.centroid_shift);
                candidate_span.push(candidate.centroid_span);
                log_width.push((expected.width_cells as f64).ln());

                println!(
                    "RAIN_005B2D1_AUTHORITY_SAMPLE variant={name} p_requested={requested_probability:.12} p_effective={effective_probability:.12} run={} geometry={} seed_slot={} terminal={}x{} seed={} delta_cv={cv:.9} delta_corr={corr:.9} delta_tv={tv:.9} delta_shift={shift:.9} delta_span={span:.9} candidate_shift={:.9} candidate_span={:.9}",
                    expected.run,
                    expected.geometry,
                    expected.seed_slot,
                    expected.width_cells,
                    expected.height_cells,
                    expected.seed,
                    candidate.centroid_shift,
                    candidate.centroid_span,
                );
            }

            let median_cv = percentile(&delta_cv, 0.50);
            let median_corr = percentile(&delta_corr, 0.50);
            let median_tv = percentile(&delta_tv, 0.50);
            let median_shift = percentile(&delta_shift, 0.50);
            let median_span = percentile(&delta_span, 0.50);
            let shift_vs_log_width = correlation_or_zero(&log_width, &candidate_shift);
            let span_vs_log_width = correlation_or_zero(&log_width, &candidate_span);
            let subset_pass = median_cv >= 0.0
                && median_corr <= 0.0
                && median_tv >= 0.0
                && median_shift > 0.0
                && median_span > 0.0
                && shift_vs_log_width > baseline_shift_vs_log_width;

            println!(
                "RAIN_005B2D1_AUTHORITY_SUMMARY variant={name} runs={} p_requested={requested_probability:.12} p_effective={effective_probability:.12} n4_max={n4_max} n20_max={n20_max} median_delta_cv={median_cv:.9} median_delta_corr={median_corr:.9} median_delta_tv={median_tv:.9} median_delta_shift={median_shift:.9} median_delta_span={median_span:.9} shift_vs_log_width={shift_vs_log_width:.9} span_vs_log_width={span_vs_log_width:.9} subset_pass={subset_pass}",
                reference.len(),
            );

            if variant != RainBiasScheduleProbe::B2Control && subset_pass && selected.is_none() {
                selected = Some((name, requested_probability));
            }
        }

        match selected {
            Some((name, probability)) => println!(
                "RAIN_005B2D1_SELECTION classification=DERIVED_DYADIC_SUBSET_FRONTIER selected_variant={name} selected_p={probability:.12} rule=smallest_authority_increase_passing_all_subset_gates"
            ),
            None => println!(
                "RAIN_005B2D1_SELECTION classification=NO_DYADIC_SUBSET_FRONTIER rule=no_runtime_candidate_authored"
            ),
        }
    }

    #[test]
    #[ignore = "native RAIN-005B2D2 test-only boundary-avulsion factorial; 64 A3 geometries x one exact seed x three variants"]
    fn rain_005b2d2_boundary_avulsion_factorial_probe() {
        const MAX_GEOMETRY: usize = 64;
        const CATEGORY_COUNT: usize = 5;
        const VARIANTS: [RainBiasScheduleProbe; 3] = [
            RainBiasScheduleProbe::B2Control,
            RainBiasScheduleProbe::Runtime,
            RainBiasScheduleProbe::BoundaryAvulsionTowardOne16,
        ];

        let categories = (1..=CATEGORY_COUNT)
            .map(|id| category(id as u64, &format!("B2D2 {id}")))
            .collect::<Vec<_>>();
        let reference = rain_005a3_rain004_reference_samples()
            .into_iter()
            .filter(|sample| sample.geometry <= MAX_GEOMETRY && sample.seed_slot == 1)
            .collect::<Vec<_>>();
        let control = rain_005b2_seed1_runtime_control();
        assert_eq!(reference.len(), MAX_GEOMETRY);
        assert_eq!(control.len(), MAX_GEOMETRY);

        let baseline_log_width = reference
            .iter()
            .map(|sample| (sample.width_cells as f64).ln())
            .collect::<Vec<_>>();
        let baseline_shift = reference
            .iter()
            .map(|sample| sample.centroid_shift)
            .collect::<Vec<_>>();
        let baseline_span = reference
            .iter()
            .map(|sample| sample.centroid_span)
            .collect::<Vec<_>>();
        let baseline_shift_vs_log_width = correlation_or_zero(&baseline_log_width, &baseline_shift);
        println!(
            "RAIN_005B2D2_BASELINE runs={} shift_vs_log_width={:.9} span_vs_log_width={:.9}",
            reference.len(),
            baseline_shift_vs_log_width,
            correlation_or_zero(&baseline_log_width, &baseline_span),
        );

        let mut selected: Option<(&'static str, f64)> = None;
        for variant in VARIANTS {
            let name = variant.diagnostic_name();
            let requested_probability = variant.focus_probability();
            let effective_probability =
                ClassicSandboxEngine::rain_focus_bias_effective_probability_for(
                    requested_probability,
                );
            let n4_max = (4.0 * effective_probability).ceil() as usize;
            let n20_max = (20.0 * effective_probability).ceil() as usize;

            let mut delta_cv = Vec::with_capacity(reference.len());
            let mut delta_corr = Vec::with_capacity(reference.len());
            let mut delta_tv = Vec::with_capacity(reference.len());
            let mut delta_shift = Vec::with_capacity(reference.len());
            let mut delta_span = Vec::with_capacity(reference.len());
            let mut candidate_shift = Vec::with_capacity(reference.len());
            let mut candidate_span = Vec::with_capacity(reference.len());
            let mut log_width = Vec::with_capacity(reference.len());

            for (expected, expected_control) in reference.iter().zip(&control) {
                assert_eq!(expected.run, expected_control.run);
                assert_eq!(expected.geometry, expected_control.geometry);
                assert_eq!(expected.seed_slot, expected_control.seed_slot);
                assert_eq!(expected.width_cells, expected_control.width_cells);
                assert_eq!(expected.height_cells, expected_control.height_cells);
                assert_eq!(expected.seed, expected_control.seed);

                let candidate = morphology_sample_for_geometry_with_bias_schedule(
                    expected.width_cells,
                    expected.height_cells,
                    expected.seed,
                    &categories,
                    variant,
                );
                assert!(
                    (candidate.mound_el - expected.mound_el).abs() < 1e-6,
                    "variant={name} run={} candidate_mound_el={} reference_mound_el={}",
                    expected.run,
                    candidate.mound_el,
                    expected.mound_el
                );

                let cv = candidate.cv - expected.cv;
                let corr = candidate.correlation - expected.correlation;
                let tv = candidate.total_variation - expected.total_variation;
                let shift = candidate.centroid_shift - expected.centroid_shift;
                let span = candidate.centroid_span - expected.centroid_span;
                if variant == RainBiasScheduleProbe::B2Control {
                    assert_printed_nine_eq("runtime delta cv", cv, expected_control.delta_cv);
                    assert_printed_nine_eq("runtime delta corr", corr, expected_control.delta_corr);
                    assert_printed_nine_eq("runtime delta tv", tv, expected_control.delta_tv);
                    assert_printed_nine_eq(
                        "runtime delta shift",
                        shift,
                        expected_control.delta_shift,
                    );
                    assert_printed_nine_eq("runtime delta span", span, expected_control.delta_span);
                    assert_printed_nine_eq(
                        "runtime candidate shift",
                        candidate.centroid_shift,
                        expected_control.candidate_shift,
                    );
                    assert_printed_nine_eq(
                        "runtime candidate span",
                        candidate.centroid_span,
                        expected_control.candidate_span,
                    );
                }

                delta_cv.push(cv);
                delta_corr.push(corr);
                delta_tv.push(tv);
                delta_shift.push(shift);
                delta_span.push(span);
                candidate_shift.push(candidate.centroid_shift);
                candidate_span.push(candidate.centroid_span);
                log_width.push((expected.width_cells as f64).ln());

                println!(
                    "RAIN_005B2D2_BOUNDARY_SAMPLE variant={name} p_requested={requested_probability:.12} p_effective={effective_probability:.12} run={} geometry={} seed_slot={} terminal={}x{} seed={} delta_cv={cv:.9} delta_corr={corr:.9} delta_tv={tv:.9} delta_shift={shift:.9} delta_span={span:.9} candidate_shift={:.9} candidate_span={:.9}",
                    expected.run,
                    expected.geometry,
                    expected.seed_slot,
                    expected.width_cells,
                    expected.height_cells,
                    expected.seed,
                    candidate.centroid_shift,
                    candidate.centroid_span,
                );
            }

            let median_cv = percentile(&delta_cv, 0.50);
            let median_corr = percentile(&delta_corr, 0.50);
            let median_tv = percentile(&delta_tv, 0.50);
            let median_shift = percentile(&delta_shift, 0.50);
            let median_span = percentile(&delta_span, 0.50);
            let shift_vs_log_width = correlation_or_zero(&log_width, &candidate_shift);
            let span_vs_log_width = correlation_or_zero(&log_width, &candidate_span);
            let subset_pass = median_cv >= 0.0
                && median_corr <= 0.0
                && median_tv >= 0.0
                && median_shift > 0.0
                && median_span > 0.0
                && shift_vs_log_width > baseline_shift_vs_log_width;

            println!(
                "RAIN_005B2D2_BOUNDARY_SUMMARY variant={name} runs={} p_requested={requested_probability:.12} p_effective={effective_probability:.12} n4_max={n4_max} n20_max={n20_max} median_delta_cv={median_cv:.9} median_delta_corr={median_corr:.9} median_delta_tv={median_tv:.9} median_delta_shift={median_shift:.9} median_delta_span={median_span:.9} shift_vs_log_width={shift_vs_log_width:.9} span_vs_log_width={span_vs_log_width:.9} subset_pass={subset_pass}",
                reference.len(),
            );

            if variant.boundary_avulsion() && subset_pass && selected.is_none() {
                selected = Some((name, requested_probability));
            }
        }

        match selected {
            Some((name, probability)) => println!(
                "RAIN_005B2D2_SELECTION classification=BOUNDARY_AVULSION_SUBSET_FRONTIER selected_variant={name} selected_p={probability:.12} rule=smallest_focus_authority_passing_all_subset_gates"
            ),
            None => println!(
                "RAIN_005B2D2_SELECTION classification=NO_BOUNDARY_AVULSION_SUBSET_FRONTIER rule=no_runtime_candidate_authored"
            ),
        }
    }
}
