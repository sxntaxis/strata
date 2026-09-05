use std::collections::{BTreeMap, BTreeSet};

use super::flowviz_unit::{FlowVizMotionId, UnitPhysicalState};
use super::*;

const UNIT_MICRO_ITERATIONS: usize = 4;
const UNIT_MICRO_MIN_DISTANCE: f32 = 0.92;
const UNIT_MICRO_TETHER_RADIUS: f32 = 0.68;
const UNIT_MICRO_EPSILON: f32 = 0.0001;
const UNIT_MICRO_X_MARGIN: f32 = 0.34;
const UNIT_MICRO_RASTER_RADIUS: isize = 4;

#[derive(Debug, Clone, Copy)]
struct UnitMicroSample {
    id: FlowVizMotionId,
    x: f32,
    y: f32,
    ideal_x: f32,
    ideal_y: f32,
    min_x: f32,
    max_x: f32,
}

impl OsloSandboxEngine {
    fn unit_micro_x_bounds(carrier: &flowviz_unit::FlowVizUnitCarrier) -> (f32, f32) {
        if let Some(active) = carrier.active {
            let min_x = active.start_x.min(active.target_x) - UNIT_MICRO_X_MARGIN;
            let max_x = active.start_x.max(active.target_x) + UNIT_MICRO_X_MARGIN;
            (min_x, max_x)
        } else {
            (
                carrier.ideal_x - UNIT_MICRO_X_MARGIN,
                carrier.ideal_x + UNIT_MICRO_X_MARGIN,
            )
        }
    }

    pub(super) fn relax_unit_micro_positions(&mut self) -> bool {
        if !self.flowviz_unit_micro || self.flowviz_unit_carriers.len() < 2 {
            return false;
        }
        let grid_height = self.surface.grid_height_dots;
        let mut changed = false;

        for _ in 0..UNIT_MICRO_ITERATIONS {
            let samples = self
                .flowviz_unit_carriers
                .values()
                .filter(|carrier| {
                    carrier.active.is_some()
                        || !carrier.queued.is_empty()
                        || carrier.physical == UnitPhysicalState::Rolling
                })
                .map(|carrier| {
                    let (min_x, max_x) = Self::unit_micro_x_bounds(carrier);
                    UnitMicroSample {
                        id: carrier.id,
                        x: carrier.x,
                        y: carrier.y,
                        ideal_x: carrier.ideal_x,
                        ideal_y: carrier.ideal_y,
                        min_x,
                        max_x,
                    }
                })
                .collect::<Vec<_>>();
            if samples.len() < 2 {
                break;
            }

            let mut buckets: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
            for (index, sample) in samples.iter().enumerate() {
                buckets
                    .entry((sample.x.floor() as i32, sample.y.floor() as i32))
                    .or_default()
                    .push(index);
            }

            let mut corrections = vec![(0.0f32, 0.0f32); samples.len()];
            for (index, sample) in samples.iter().enumerate() {
                let bucket_x = sample.x.floor() as i32;
                let bucket_y = sample.y.floor() as i32;
                for neighbour_y in (bucket_y - 1)..=(bucket_y + 1) {
                    for neighbour_x in (bucket_x - 1)..=(bucket_x + 1) {
                        let Some(neighbours) = buckets.get(&(neighbour_x, neighbour_y)) else {
                            continue;
                        };
                        for &other_index in neighbours {
                            if other_index <= index {
                                continue;
                            }
                            let other = samples[other_index];
                            let mut dx = other.x - sample.x;
                            let mut dy = other.y - sample.y;
                            let distance_sq = dx * dx + dy * dy;
                            if distance_sq >= UNIT_MICRO_MIN_DISTANCE * UNIT_MICRO_MIN_DISTANCE {
                                continue;
                            }
                            let (distance, overlap) = if distance_sq <= UNIT_MICRO_EPSILON {
                                match (sample.id.0 ^ other.id.0) & 0b11 {
                                    0 => {
                                        dx = 0.0;
                                        dy = 1.0;
                                    }
                                    1 => {
                                        dx = 1.0;
                                        dy = 0.0;
                                    }
                                    2 => {
                                        dx = 0.707_106_77;
                                        dy = 0.707_106_77;
                                    }
                                    _ => {
                                        dx = -0.707_106_77;
                                        dy = 0.707_106_77;
                                    }
                                }
                                (1.0, UNIT_MICRO_MIN_DISTANCE)
                            } else {
                                let distance = distance_sq.sqrt();
                                (distance, UNIT_MICRO_MIN_DISTANCE - distance)
                            };
                            if overlap <= 0.0 {
                                continue;
                            }
                            let correction = overlap * 0.5 / distance;
                            let correction_x = dx * correction;
                            let correction_y = dy * correction;
                            corrections[index].0 -= correction_x;
                            corrections[index].1 -= correction_y;
                            corrections[other_index].0 += correction_x;
                            corrections[other_index].1 += correction_y;
                        }
                    }
                }
            }

            for (sample, (correction_x, correction_y)) in
                samples.iter().zip(corrections.into_iter())
            {
                if correction_x.abs() <= UNIT_MICRO_EPSILON
                    && correction_y.abs() <= UNIT_MICRO_EPSILON
                {
                    continue;
                }
                let Some(carrier) = self.flowviz_unit_carriers.get_mut(&sample.id) else {
                    continue;
                };
                let mut x = sample.x + correction_x;
                let mut y = sample.y + correction_y;

                let tether_x = x - sample.ideal_x;
                let tether_y = y - sample.ideal_y;
                let tether_sq = tether_x * tether_x + tether_y * tether_y;
                if tether_sq > UNIT_MICRO_TETHER_RADIUS * UNIT_MICRO_TETHER_RADIUS {
                    let scale = UNIT_MICRO_TETHER_RADIUS / tether_sq.sqrt();
                    x = sample.ideal_x + tether_x * scale;
                    y = sample.ideal_y + tether_y * scale;
                }
                x = x.clamp(sample.min_x, sample.max_x);
                if grid_height > 0 {
                    y = y.clamp(0.0, grid_height.saturating_sub(1) as f32);
                }
                changed |= (x - carrier.x).abs() > UNIT_MICRO_EPSILON
                    || (y - carrier.y).abs() > UNIT_MICRO_EPSILON;
                carrier.x = x;
                carrier.y = y;
            }
        }

        changed
    }

    fn unit_micro_static_occupied_cells(&self) -> BTreeSet<(usize, usize)> {
        let mut occupied = BTreeSet::new();
        let height = self.surface.grid_height_dots;
        let width = self.surface.grid_width_dots;
        for (site, column) in self.flowviz_shadow_columns.iter().enumerate() {
            if site >= width {
                break;
            }
            for depth in 0..column.len() {
                let Some(y) = height.checked_sub(depth.saturating_add(1)) else {
                    break;
                };
                occupied.insert((site, y));
            }
        }
        for falling in &self.falling_drives {
            if falling.x < width && falling.y < height {
                occupied.insert((falling.x, falling.y));
            }
        }
        occupied
    }

    fn unit_micro_raster_x_bounds(
        carrier: &flowviz_unit::FlowVizUnitCarrier,
        width: usize,
    ) -> Option<(usize, usize)> {
        if width == 0 {
            return None;
        }
        if let Some(active) = carrier.active {
            let source = active.segment.source.min(width - 1);
            if let Some(destination) = active.segment.destination {
                let destination = destination.min(width - 1);
                return Some((source.min(destination), source.max(destination)));
            }
            return Some((source, source));
        }
        let x = carrier
            .ideal_x
            .round()
            .clamp(0.0, width.saturating_sub(1) as f32) as usize;
        Some((x, x))
    }

    pub(super) fn collect_unit_micro_render_cells(
        &self,
    ) -> Vec<(FlowVizMotionId, CategoryId, usize, usize)> {
        let height = self.surface.grid_height_dots;
        let width = self.surface.grid_width_dots;
        if width == 0 || height == 0 {
            return Vec::new();
        }
        let static_occupied = self.unit_micro_static_occupied_cells();
        let mut mobile_occupied = BTreeSet::new();
        let mut placements = Vec::with_capacity(self.flowviz_unit_carriers.len());

        for carrier in self.flowviz_unit_carriers.values() {
            if carrier.x < -0.5
                || carrier.x > width as f32 - 0.5
                || carrier.y < -0.5
                || carrier.y > height as f32 - 0.5
            {
                continue;
            }
            let Some((min_x, max_x)) = Self::unit_micro_raster_x_bounds(carrier, width) else {
                continue;
            };
            let base_y = carrier.y.round() as isize;
            let mut chosen = None;

            for allow_static_overlap in [false, true] {
                let mut best: Option<(f32, usize, usize)> = None;
                for x in min_x..=max_x {
                    for offset in -UNIT_MICRO_RASTER_RADIUS..=UNIT_MICRO_RASTER_RADIUS {
                        let y = base_y + offset;
                        if y < 0 || y >= height as isize {
                            continue;
                        }
                        let y = y as usize;
                        if mobile_occupied.contains(&(x, y)) {
                            continue;
                        }
                        if !allow_static_overlap && static_occupied.contains(&(x, y)) {
                            continue;
                        }
                        let dx = x as f32 - carrier.x;
                        let dy = y as f32 - carrier.y;
                        let score = dx * dx + dy * dy;
                        let replace = match best {
                            None => true,
                            Some((best_score, best_x, best_y)) => {
                                score < best_score
                                    || (score == best_score && (y, x) < (best_y, best_x))
                            }
                        };
                        if replace {
                            best = Some((score, x, y));
                        }
                    }
                }
                if let Some((_, x, y)) = best {
                    chosen = Some((x, y));
                    break;
                }
            }

            if let Some((x, y)) = chosen {
                mobile_occupied.insert((x, y));
                placements.push((carrier.id, carrier.category_id, x, y));
            }
        }
        placements
    }

    #[cfg(test)]
    pub(super) fn flowviz_unit_micro_render_cell_count(&self) -> usize {
        self.collect_unit_micro_render_cells().len()
    }

    #[cfg(test)]
    pub(super) fn flowviz_unit_micro_min_pair_distance(&self) -> Option<f32> {
        let carriers = self
            .flowviz_unit_carriers
            .values()
            .filter(|carrier| {
                carrier.active.is_some()
                    || !carrier.queued.is_empty()
                    || carrier.physical == UnitPhysicalState::Rolling
            })
            .collect::<Vec<_>>();
        let mut minimum: Option<f32> = None;
        for (index, carrier) in carriers.iter().enumerate() {
            for other in carriers.iter().skip(index + 1) {
                let dx = other.x - carrier.x;
                let dy = other.y - carrier.y;
                let distance = (dx * dx + dy * dy).sqrt();
                minimum = Some(minimum.map_or(distance, |current| current.min(distance)));
            }
        }
        minimum
    }
}
