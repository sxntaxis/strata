use std::collections::{BTreeMap, HashMap};

use super::*;

const MAX_REPORTED_LOW_PATHS: usize = 6;
const MAX_STORED_DIRECTIONS_PER_TRACE: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveTraceFinalState {
    Rolling,
    Settled { site: usize, depth: usize },
    Discharged,
}

#[derive(Debug, Clone)]
struct LiveTraceEpisode {
    category_id: CategoryId,
    start_site: usize,
    last_site: usize,
    hops: usize,
    max_abs_dx: usize,
    directions: Vec<ToppleDirection>,
    final_state: LiveTraceFinalState,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LiveRainbowProvenance {
    enabled: bool,
    categories: Vec<CategoryId>,
    initial_counts: HashMap<CategoryId, usize>,
    initial_visible_start: usize,
    initial_visible_end: usize,
    initial_green_band_height: usize,
    start_cell_width: u16,
    start_cell_height: u16,
    start_grid_width_dots: usize,
    start_grid_height_dots: usize,
    start_threshold_rng_state: u64,
    start_rain_rng_state: u64,
    start_relax_rng_state: u64,
    start_flowviz_rng_state: u64,
    episodes: BTreeMap<flowviz_unit::FlowVizMotionId, LiveTraceEpisode>,
}

impl OsloSandboxEngine {
    pub(super) fn reset_live_rainbow_provenance(
        &mut self,
        categories: &[CategoryId],
        initial_fill_start: usize,
        initial_fill_end: usize,
    ) {
        let mut initial_counts = HashMap::new();
        for category_id in self.columns.iter().flatten().copied() {
            if categories.contains(&category_id) {
                *initial_counts.entry(category_id).or_insert(0) += 1;
            }
        }
        let initial_green_band_height = categories.first().map_or(0, |green| {
            self.columns
                .iter()
                .find(|column| !column.is_empty())
                .map(|column| {
                    column
                        .iter()
                        .take_while(|category_id| **category_id == *green)
                        .count()
                })
                .unwrap_or(0)
        });

        self.flowviz_live_rainbow = LiveRainbowProvenance {
            enabled: self.flowviz_unit && !categories.is_empty(),
            categories: categories.to_vec(),
            initial_counts,
            initial_visible_start: initial_fill_start,
            initial_visible_end: initial_fill_end,
            initial_green_band_height,
            start_cell_width: self.surface.cell_width,
            start_cell_height: self.surface.cell_height,
            start_grid_width_dots: self.surface.grid_width_dots,
            start_grid_height_dots: self.surface.grid_height_dots,
            start_threshold_rng_state: self.threshold_rng_state,
            start_rain_rng_state: self.rain_rng_state,
            start_relax_rng_state: self.relax_rng_state,
            start_flowviz_rng_state: self.flowviz_rng_state,
            episodes: BTreeMap::new(),
        };
    }

    pub(super) fn note_live_rainbow_motion_start(
        &mut self,
        id: flowviz_unit::FlowVizMotionId,
        category_id: CategoryId,
        source: usize,
    ) {
        if !self.flowviz_live_rainbow.enabled
            || !self.flowviz_live_rainbow.categories.contains(&category_id)
        {
            return;
        }
        self.flowviz_live_rainbow
            .episodes
            .entry(id)
            .or_insert_with(|| LiveTraceEpisode {
                category_id,
                start_site: source,
                last_site: source,
                hops: 0,
                max_abs_dx: 0,
                directions: Vec::new(),
                final_state: LiveTraceFinalState::Rolling,
            });
    }

    pub(super) fn note_live_rainbow_segment(
        &mut self,
        id: flowviz_unit::FlowVizMotionId,
        source: usize,
        destination: Option<usize>,
    ) {
        let Some(destination) = destination else {
            return;
        };
        let Some(trace) = self.flowviz_live_rainbow.episodes.get_mut(&id) else {
            return;
        };
        if source.abs_diff(destination) != 1 {
            return;
        }
        let direction = if destination < source {
            ToppleDirection::Left
        } else {
            ToppleDirection::Right
        };
        trace.hops = trace.hops.saturating_add(1);
        trace.last_site = destination;
        trace.max_abs_dx = trace.max_abs_dx.max(trace.start_site.abs_diff(destination));
        trace.final_state = LiveTraceFinalState::Rolling;
        if trace.directions.len() < MAX_STORED_DIRECTIONS_PER_TRACE {
            trace.directions.push(direction);
        }
    }

    pub(super) fn note_live_rainbow_settled(
        &mut self,
        id: flowviz_unit::FlowVizMotionId,
        site: usize,
    ) {
        let depth = self.columns.get(site).map_or(0, Vec::len).saturating_sub(1);
        if let Some(trace) = self.flowviz_live_rainbow.episodes.get_mut(&id) {
            trace.last_site = site;
            trace.max_abs_dx = trace.max_abs_dx.max(trace.start_site.abs_diff(site));
            trace.final_state = LiveTraceFinalState::Settled { site, depth };
        }
    }

    pub(super) fn note_live_rainbow_discharged(&mut self, id: flowviz_unit::FlowVizMotionId) {
        if let Some(trace) = self.flowviz_live_rainbow.episodes.get_mut(&id) {
            trace.final_state = LiveTraceFinalState::Discharged;
        }
    }

    pub(crate) fn live_rainbow_provenance_report(&self) -> Result<String, String> {
        if !self.flowviz_live_rainbow.enabled {
            return Err(
                "testingcheats provenance requires oslo-vessel-front-grains after testingcheats fill"
                    .to_string(),
            );
        }
        let labels = ["green", "yellow", "red", "purple", "blue", "cyan"];
        let mut lines = Vec::new();
        lines.push(format!(
            "RAINBOW_LIVE_ENV cell={}x{} grid_dots={}x{} visible_sites={}..{} threshold_rng={} rain_rng={} relax_rng={} flowviz_rng={} active_cell={}x{} active_grid_dots={}x{}",
            self.flowviz_live_rainbow.start_cell_width,
            self.flowviz_live_rainbow.start_cell_height,
            self.flowviz_live_rainbow.start_grid_width_dots,
            self.flowviz_live_rainbow.start_grid_height_dots,
            self.flowviz_live_rainbow.initial_visible_start,
            self.flowviz_live_rainbow.initial_visible_end,
            self.flowviz_live_rainbow.start_threshold_rng_state,
            self.flowviz_live_rainbow.start_rain_rng_state,
            self.flowviz_live_rainbow.start_relax_rng_state,
            self.flowviz_live_rainbow.start_flowviz_rng_state,
            self.surface.cell_width,
            self.surface.cell_height,
            self.surface.grid_width_dots,
            self.surface.grid_height_dots,
        ));

        let mut final_counts = HashMap::new();
        for category_id in self.columns.iter().flatten().copied() {
            if self.flowviz_live_rainbow.categories.contains(&category_id) {
                *final_counts.entry(category_id).or_insert(0) += 1;
            }
        }

        for (index, category_id) in self
            .flowviz_live_rainbow
            .categories
            .iter()
            .copied()
            .enumerate()
        {
            let traces = self
                .flowviz_live_rainbow
                .episodes
                .values()
                .filter(|trace| trace.category_id == category_id)
                .collect::<Vec<_>>();
            let movement_entries = traces.len();
            let hops = traces.iter().map(|trace| trace.hops).sum::<usize>();
            let max_hops = traces.iter().map(|trace| trace.hops).max().unwrap_or(0);
            let max_abs_dx = traces
                .iter()
                .map(|trace| trace.max_abs_dx)
                .max()
                .unwrap_or(0);
            let discharged = traces
                .iter()
                .filter(|trace| trace.final_state == LiveTraceFinalState::Discharged)
                .count();
            lines.push(format!(
                "RAINBOW_TRACE layer={} category={} initial={} movement_entries={} hops={} max_hops={} max_abs_dx={} discharged={} final={}",
                labels.get(index).copied().unwrap_or("extra"),
                category_id.0,
                self.flowviz_live_rainbow
                    .initial_counts
                    .get(&category_id)
                    .copied()
                    .unwrap_or(0),
                movement_entries,
                hops,
                max_hops,
                max_abs_dx,
                discharged,
                final_counts.get(&category_id).copied().unwrap_or(0),
            ));
        }

        if let Some(green) = self.flowviz_live_rainbow.categories.first().copied() {
            let mut inside = 0usize;
            let mut outside = 0usize;
            for (site, column) in self.columns.iter().enumerate() {
                let count = column
                    .iter()
                    .filter(|category_id| **category_id == green)
                    .count();
                if site >= self.flowviz_live_rainbow.initial_visible_start
                    && site < self.flowviz_live_rainbow.initial_visible_end
                {
                    inside = inside.saturating_add(count);
                } else {
                    outside = outside.saturating_add(count);
                }
            }
            let movement_entries = self
                .flowviz_live_rainbow
                .episodes
                .values()
                .filter(|trace| trace.category_id == green)
                .count();
            lines.push(format!(
                "RAINBOW_GREEN_MASS initial={} final_inside_original_footprint={} final_outside_original_footprint={} movement_entries={}",
                self.flowviz_live_rainbow
                    .initial_counts
                    .get(&green)
                    .copied()
                    .unwrap_or(0),
                inside,
                outside,
                movement_entries,
            ));
        }

        if let Some(cyan) = self.flowviz_live_rainbow.categories.get(5).copied() {
            let green_height = self.flowviz_live_rainbow.initial_green_band_height;
            let final_units_below = self
                .columns
                .iter()
                .map(|column| {
                    column
                        .iter()
                        .take(green_height)
                        .filter(|category_id| **category_id == cyan)
                        .count()
                })
                .sum::<usize>();
            let low_traces = self
                .flowviz_live_rainbow
                .episodes
                .iter()
                .filter_map(|(id, trace)| {
                    if trace.category_id != cyan {
                        return None;
                    }
                    match trace.final_state {
                        LiveTraceFinalState::Settled { site, depth } if depth < green_height => {
                            Some((*id, trace, site, depth))
                        }
                        _ => None,
                    }
                })
                .collect::<Vec<_>>();
            lines.push(format!(
                "RAINBOW_CYAN_LOW final_units_below_initial_green_band={} traced_settlement_episodes_below_band={} initial_green_band_height={}",
                final_units_below,
                low_traces.len(),
                green_height,
            ));
            for (id, trace, site, depth) in low_traces.into_iter().take(MAX_REPORTED_LOW_PATHS) {
                let mut cursor = trace.start_site;
                let mut path = vec![cursor.to_string()];
                for direction in &trace.directions {
                    cursor = match direction {
                        ToppleDirection::Left => cursor.saturating_sub(1),
                        ToppleDirection::Right => cursor.saturating_add(1),
                    };
                    path.push(cursor.to_string());
                }
                lines.push(format!(
                    "RAINBOW_CYAN_LOW_PATH motion={} start={} final={} depth={} hops={} path={}",
                    id.0,
                    trace.start_site,
                    site,
                    depth,
                    trace.hops,
                    path.join("->"),
                ));
            }
        }
        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: u64 = 0x51A7_0157_D4A6_0001;

    #[test]
    fn live_rainbow_trace_records_real_adjacent_unit_motion_without_changing_physics() {
        let categories = [
            CategoryId(101),
            CategoryId(102),
            CategoryId(103),
            CategoryId(104),
            CategoryId(105),
            CategoryId(106),
        ];
        let mut engine =
            OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 10, TEST_SEED);
        engine.debug_fill_rainbow_80(&categories).unwrap();
        let physical_before = engine.columns.clone();
        let (start, end) = engine.visible_lattice_bounds();
        let source = start + 1;
        let destination = source + 1;
        assert!(destination < end);
        let category_id = *engine.columns[source].last().unwrap();
        let source_y = engine.top_grain_y(source).unwrap();
        let id = engine
            .begin_unit_motion(source, Some(destination), category_id, source_y, None)
            .unwrap();
        engine.mark_unit_settled(id, destination);

        let report = engine.live_rainbow_provenance_report().unwrap();
        assert!(report.contains("layer=cyan"));
        assert!(report.contains("movement_entries=1"));
        assert!(report.contains("hops=1"));
        assert_eq!(engine.columns, physical_before);
    }

    #[test]
    fn live_rainbow_trace_requires_fill_on_unit_model() {
        let engine =
            OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 10, TEST_SEED);
        assert!(engine.live_rainbow_provenance_report().is_err());
    }
}
