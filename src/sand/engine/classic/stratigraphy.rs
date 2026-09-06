use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::{
    constants::SAND_ENGINE,
    domain::{Category, CategoryId},
};

use super::ClassicSandboxEngine;

#[derive(Debug, Clone)]
pub(super) struct ClassicFillBand {
    pub(super) category_id: CategoryId,
    pub(super) y_start: usize,
    pub(super) y_end: usize,
    pub(super) initial_count: usize,
}

#[derive(Debug, Clone)]
pub(super) struct ClassicRainbowFillDescriptor {
    pub(super) x_start: usize,
    pub(super) x_end: usize,
    pub(super) y_start: usize,
    pub(super) y_end: usize,
    pub(super) initial_total: usize,
    pub(super) bands: Vec<ClassicFillBand>,
}

impl ClassicRainbowFillDescriptor {
    pub(super) fn from_fill(
        category_ids: &[CategoryId],
        x_start: usize,
        x_end: usize,
        bottom_y_end: usize,
        fill_height: usize,
    ) -> Self {
        let fill_width = x_end.saturating_sub(x_start);
        let mut bands = category_ids
            .iter()
            .copied()
            .map(|category_id| ClassicFillBand {
                category_id,
                y_start: bottom_y_end,
                y_end: bottom_y_end.saturating_sub(fill_height),
                initial_count: 0,
            })
            .collect::<Vec<_>>();

        for depth in 0..fill_height {
            let layer = depth.saturating_mul(category_ids.len()) / fill_height;
            let band = &mut bands[layer.min(category_ids.len() - 1)];
            let y = bottom_y_end - 1 - depth;
            band.y_start = band.y_start.min(y);
            band.y_end = band.y_end.max(y.saturating_add(1));
            band.initial_count = band.initial_count.saturating_add(fill_width);
        }

        Self {
            x_start,
            x_end,
            y_start: bottom_y_end - fill_height,
            y_end: bottom_y_end,
            initial_total: fill_width.saturating_mul(fill_height),
            bands,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CategoryStats {
    current_count: usize,
    above_band: usize,
    below_band: usize,
    outside_span: usize,
    unsupported: usize,
    max_above: usize,
    max_below: usize,
    max_outside: usize,
}

impl ClassicSandboxEngine {
    pub(crate) fn rainbow_stratigraphy_report(
        &self,
        categories: &[Category],
    ) -> Result<String, String> {
        let Some(fill) = self.last_rainbow_fill.as_ref() else {
            return Err(
                "testingcheats classic stratigraphy requires a fresh testingcheats fill or fillhalf without a subsequent resize/clear"
                    .to_string(),
            );
        };

        let category_names = categories
            .iter()
            .map(|category| (category.id, category.name.as_str()))
            .collect::<HashMap<_, _>>();
        let ranks = fill
            .bands
            .iter()
            .enumerate()
            .map(|(rank, band)| (band.category_id, rank))
            .collect::<HashMap<_, _>>();
        let mut stats = fill
            .bands
            .iter()
            .map(|band| (band.category_id, CategoryStats::default()))
            .collect::<HashMap<_, _>>();
        let mut outliers = Vec::new();

        for (y, row) in self.surface.grid.iter().enumerate() {
            for (x, cell) in row.iter().copied().enumerate() {
                let Some(category_id) = cell else {
                    continue;
                };
                let Some(rank) = ranks.get(&category_id).copied() else {
                    continue;
                };
                let band = &fill.bands[rank];
                let stat = stats.get_mut(&category_id).expect("tracked category stats");
                stat.current_count = stat.current_count.saturating_add(1);

                let above = band.y_start.saturating_sub(y);
                let below = y.saturating_sub(band.y_end.saturating_sub(1));
                if above > 0 {
                    stat.above_band = stat.above_band.saturating_add(1);
                    stat.max_above = stat.max_above.max(above);
                }
                if below > 0 {
                    stat.below_band = stat.below_band.saturating_add(1);
                    stat.max_below = stat.max_below.max(below);
                }

                let outside = if x < fill.x_start {
                    fill.x_start - x
                } else if x >= fill.x_end {
                    x - fill.x_end + 1
                } else {
                    0
                };
                if outside > 0 {
                    stat.outside_span = stat.outside_span.saturating_add(1);
                    stat.max_outside = stat.max_outside.max(outside);
                }

                let unsupported = y + 1 < self.surface.grid.len()
                    && self.surface.grid[y + 1][x].is_none();
                if unsupported {
                    stat.unsupported = stat.unsupported.saturating_add(1);
                }

                let score = above
                    .saturating_add(below)
                    .saturating_add(outside.saturating_mul(2))
                    .saturating_add(if unsupported { 1 } else { 0 });
                if score > 0 {
                    outliers.push((score, category_id, x, y, above, below, outside, unsupported));
                }
            }
        }

        let rank_drops = self.stratigraphic_rank_drops(&ranks);
        let (tracked_braille_cells, mixed_braille_cells) = self.braille_mix_counts(&ranks);
        let current_tracked = stats
            .values()
            .map(|stat| stat.current_count)
            .sum::<usize>();

        let mut report = String::new();
        writeln!(report, "CLASSIC_STRATIGRAPHY_REPORT").expect("String write");
        writeln!(report, "model={}", self.model_name()).expect("String write");
        writeln!(report, "experiment={}", self.experiment_profile_name()).expect("String write");
        writeln!(report, "colorblend={}", self.color_blend_profile_name()).expect("String write");
        writeln!(
            report,
            "initial_fill_span=x[{}, {}) y[{}, {}) initial_grains={}",
            fill.x_start, fill.x_end, fill.y_start, fill.y_end, fill.initial_total
        )
        .expect("String write");
        writeln!(report, "current_tracked_grains={current_tracked}").expect("String write");
        writeln!(report, "stratigraphic_rank_drops={rank_drops}").expect("String write");
        writeln!(
            report,
            "braille_cells_tracked={tracked_braille_cells} braille_cells_mixed={mixed_braille_cells}"
        )
        .expect("String write");
        writeln!(report).expect("String write");

        for band in &fill.bands {
            let name = category_names
                .get(&band.category_id)
                .copied()
                .unwrap_or("<unknown>");
            let stat = stats.get(&band.category_id).expect("tracked category stats");
            writeln!(
                report,
                "CATEGORY name={name:?} id={} initial_band=y[{}, {}) initial={} current={} above={} max_above={} below={} max_below={} outside_fill_span={} max_outside={} unsupported={}",
                band.category_id.0,
                band.y_start,
                band.y_end,
                band.initial_count,
                stat.current_count,
                stat.above_band,
                stat.max_above,
                stat.below_band,
                stat.max_below,
                stat.outside_span,
                stat.max_outside,
                stat.unsupported
            )
            .expect("String write");
        }

        outliers.sort_unstable_by(|left, right| right.0.cmp(&left.0));
        writeln!(report).expect("String write");
        writeln!(report, "OUTLIERS top={}", outliers.len().min(24)).expect("String write");
        for (_, category_id, x, y, above, below, outside, unsupported) in
            outliers.into_iter().take(24)
        {
            let name = category_names
                .get(&category_id)
                .copied()
                .unwrap_or("<unknown>");
            writeln!(
                report,
                "cell=({x},{y}) category={name:?} id={} above_band={above} below_band={below} outside_fill_span={outside} unsupported={unsupported}",
                category_id.0
            )
            .expect("String write");
        }
        writeln!(report).expect("String write");
        writeln!(
            report,
            "NOTE: this is a read-only distribution diagnostic. It identifies where fill-origin categories ended up relative to their initial bands/span, but does not reconstruct an individual grain's historical path."
        )
        .expect("String write");
        Ok(report)
    }

    fn stratigraphic_rank_drops(&self, ranks: &HashMap<CategoryId, usize>) -> usize {
        let mut rank_drops = 0usize;
        let width = self.surface.grid.first().map_or(0, Vec::len);
        for x in 0..width {
            let mut previous_rank = None;
            for y in (0..self.surface.grid.len()).rev() {
                let Some(category_id) = self.surface.grid[y][x] else {
                    continue;
                };
                let Some(rank) = ranks.get(&category_id).copied() else {
                    continue;
                };
                if let Some(previous) = previous_rank {
                    if rank < previous {
                        rank_drops = rank_drops.saturating_add(1);
                    }
                }
                previous_rank = Some(rank);
            }
        }
        rank_drops
    }

    fn braille_mix_counts(&self, ranks: &HashMap<CategoryId, usize>) -> (usize, usize) {
        let mut mixed = 0usize;
        let mut tracked = 0usize;
        let Some(bounds) = self.surface.viewport_bounds() else {
            return (tracked, mixed);
        };

        for y in (bounds.y_start..bounds.y_end).step_by(SAND_ENGINE.dot_height) {
            for x in (bounds.x_start..bounds.x_end).step_by(SAND_ENGINE.dot_width) {
                let mut present = HashSet::new();
                for dy in 0..SAND_ENGINE.dot_height {
                    for dx in 0..SAND_ENGINE.dot_width {
                        let yy = y + dy;
                        let xx = x + dx;
                        if yy >= bounds.y_end || xx >= bounds.x_end {
                            continue;
                        }
                        if let Some(category_id) = self.surface.grid[yy][xx] {
                            if ranks.contains_key(&category_id) {
                                present.insert(category_id);
                            }
                        }
                    }
                }
                if !present.is_empty() {
                    tracked = tracked.saturating_add(1);
                    if present.len() > 1 {
                        mixed = mixed.saturating_add(1);
                    }
                }
            }
        }
        (tracked, mixed)
    }
}
