use std::collections::HashMap;
use std::fmt::Write as _;

use crate::constants::TIME_SETTINGS;
use crate::domain::{Category, CategoryId};

use super::{ClassicRainMode, ClassicSandboxEngine, RAIN_FOCUS_BIAS_PROBABILITY};

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
    /// RAIN-004 remains the owner-selected reference baseline. This report does
    /// not promote its golden-ratio bias or current meander constants into
    /// permanent authority; it measures the natural spatial/temporal scales that
    /// a later morphology pass can derive from the canonical 1 grain/s ingress,
    /// visible airborne population, active width and actual rain kernel.
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
            "reference=RAIN-004 owner-selected best-so-far baseline; not a perfection/freeze target; future derived morphology must target equal-or-greater heterogeneity without an airborne-nozzle regression"
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
        bounds: super::ViewportBounds,
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
        bounds: super::ViewportBounds,
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
            "reference=RAIN-004 owner-selected best-so-far baseline; not a perfection/freeze target"
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
                + (halton(case, 2) * (MAX_WIDTH_CELLS - MIN_WIDTH_CELLS) as f64).round()
                    as usize;
            let height_cells = MIN_HEIGHT_CELLS
                + (halton(case, 3) * (MAX_HEIGHT_CELLS - MIN_HEIGHT_CELLS) as f64).round()
                    as usize;
            let seed = 0xA11C_005A_2000_0000u64
                ^ (case as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
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
            let kernel = rain_distribution_metrics(
                &free,
                center,
                RAIN_FOCUS_BIAS_PROBABILITY,
                width_dots,
            );
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
            assert!(profiles.len() >= CATEGORY_COUNT - 1, "case={case} profiles={}", profiles.len());
            assert!(pairs.len() >= CATEGORY_COUNT - 2, "case={case} pairs={}", pairs.len());

            let mean_cv = mean(&profiles.iter().map(|profile| profile.thickness_cv).collect::<Vec<_>>())
                .expect("profiles");
            let mean_corr = mean_defined(pairs.iter().map(|pair| pair.correlation)).unwrap_or(0.0);
            let mean_tv = mean(&pairs.iter().map(|pair| pair.total_variation).collect::<Vec<_>>())
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
            let distribution = rain_distribution_metrics(
                &free,
                focus,
                RAIN_FOCUS_BIAS_PROBABILITY,
                width_dots,
            );
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
                distribution.kl_bits_per_grain,
                distribution.focus_rms_width_fraction,
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

}
