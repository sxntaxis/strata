use std::collections::BTreeMap;

use super::flowviz_unit::{
    FlowVizMotionId, UNIT_SEGMENT_STEP, UnitActiveSegment, UnitPhysicalState,
};
use super::*;

const UNIT_MICRO_TRACK_BLEND: f32 = 0.72;
const UNIT_MICRO_SETTLE_STEP: f32 = 0.72;
const UNIT_MICRO_EPSILON: f32 = 0.0001;

impl OsloSandboxEngine {
    fn unit_target_y(grid_height: usize, shadow_height: usize) -> f32 {
        grid_height.saturating_sub(shadow_height.saturating_add(1)) as f32
    }

    fn unit_settled_targets(
        &self,
        grid_height: usize,
    ) -> BTreeMap<FlowVizMotionId, (usize, f32)> {
        let mut targets = BTreeMap::new();
        for (site, custody_column) in self.flowviz_unit_custody.iter().enumerate() {
            for (depth, custody) in custody_column.iter().copied().enumerate() {
                if let Some(id) = custody {
                    targets.insert(
                        id,
                        (site, grid_height as f32 - depth as f32 - 1.0),
                    );
                }
            }
        }
        targets
    }

    fn unit_segment_target(
        carrier: &flowviz_unit::FlowVizUnitCarrier,
        segment: flowviz_unit::UnitSegment,
        grid_height: usize,
        visible_start: usize,
        visible_end: usize,
        heights: &[usize],
        settled_targets: &BTreeMap<FlowVizMotionId, (usize, f32)>,
    ) -> (f32, f32) {
        let target_x = segment.destination.map_or_else(
            || {
                if segment.source <= visible_start {
                    visible_start as f32 - 1.0
                } else {
                    visible_end as f32
                }
            },
            |site| site as f32,
        );
        let target_y = segment.destination.map_or_else(
            || (carrier.ideal_y + 1.0).min(grid_height.saturating_sub(1) as f32),
            |site| {
                if carrier.physical == UnitPhysicalState::Settled(site)
                    && carrier.queued.is_empty()
                    && let Some((target_site, target_y)) = settled_targets.get(&carrier.id)
                    && *target_site == site
                {
                    *target_y
                } else {
                    let height = heights.get(site).copied().unwrap_or(0);
                    Self::unit_target_y(grid_height, height)
                }
            },
        );
        (target_x, target_y)
    }

    fn move_unit_towards(
        x: f32,
        y: f32,
        target_x: f32,
        target_y: f32,
        max_step: f32,
    ) -> (f32, f32, bool) {
        let dx = target_x - x;
        let dy = target_y - y;
        let distance_sq = dx * dx + dy * dy;
        if distance_sq <= UNIT_MICRO_EPSILON {
            return (target_x, target_y, true);
        }
        let distance = distance_sq.sqrt();
        if distance <= max_step {
            return (target_x, target_y, true);
        }
        let scale = max_step / distance;
        (x + dx * scale, y + dy * scale, false)
    }

    pub(super) fn advance_unit_flowviz(&mut self) -> bool {
        if !self.flowviz_unit || self.flowviz_unit_carriers.is_empty() {
            return false;
        }
        let grid_height = self.surface.grid_height_dots;
        let observed_sequence_limit = self.flowviz_unit_next_sequence;
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let heights = self
            .flowviz_shadow_columns
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>();
        let settled_targets = self.unit_settled_targets(grid_height);
        let micro = self.flowviz_unit_micro;
        let mut changed = false;
        let mut discharged_done = Vec::new();
        let mut missing_settled_targets = 0usize;

        for carrier in self.flowviz_unit_carriers.values_mut() {
            if carrier.active.is_none()
                && let Some(segment) = carrier.queued.pop_front()
            {
                debug_assert!(
                    segment.sequence > 0 && segment.sequence < observed_sequence_limit,
                    "unit carrier can replay only an already-observed physical segment"
                );
                let (target_x, target_y) = Self::unit_segment_target(
                    carrier,
                    segment,
                    grid_height,
                    visible_start,
                    visible_end,
                    &heights,
                    &settled_targets,
                );
                carrier.active = Some(UnitActiveSegment {
                    segment,
                    start_x: if micro { carrier.ideal_x } else { carrier.x },
                    start_y: if micro { carrier.ideal_y } else { carrier.y },
                    target_x,
                    target_y,
                    progress: 0.0,
                });
            }

            if let Some(active) = &mut carrier.active {
                active.progress = (active.progress + UNIT_SEGMENT_STEP).min(1.0);
                let (target_x, target_y) = if micro {
                    (active.target_x, active.target_y)
                } else {
                    let target_x = active.segment.destination.map_or_else(
                        || {
                            if active.segment.source <= visible_start {
                                visible_start as f32 - 1.0
                            } else {
                                visible_end as f32
                            }
                        },
                        |site| site as f32,
                    );
                    let target_y = active.segment.destination.map_or_else(
                        || {
                            (active.start_y + 1.0)
                                .min(grid_height.saturating_sub(1) as f32)
                        },
                        |site| {
                            let height = heights.get(site).copied().unwrap_or(0);
                            Self::unit_target_y(grid_height, height)
                        },
                    );
                    (target_x, target_y)
                };
                let t = active.progress;
                carrier.ideal_x = active.start_x + (target_x - active.start_x) * t;
                carrier.ideal_y = active.start_y + (target_y - active.start_y) * t;
                if micro {
                    carrier.x += (carrier.ideal_x - carrier.x) * UNIT_MICRO_TRACK_BLEND;
                    carrier.y += (carrier.ideal_y - carrier.y) * UNIT_MICRO_TRACK_BLEND;
                } else {
                    carrier.x = carrier.ideal_x;
                    carrier.y = carrier.ideal_y;
                }
                changed = true;
                if active.progress >= 1.0 {
                    carrier.active = None;
                }
            }

            if carrier.active.is_none() && carrier.queued.is_empty() {
                match carrier.physical {
                    UnitPhysicalState::Rolling => {
                        if micro {
                            let next_x =
                                carrier.x + (carrier.ideal_x - carrier.x) * UNIT_MICRO_TRACK_BLEND;
                            let next_y =
                                carrier.y + (carrier.ideal_y - carrier.y) * UNIT_MICRO_TRACK_BLEND;
                            changed |= (next_x - carrier.x).abs() > UNIT_MICRO_EPSILON
                                || (next_y - carrier.y).abs() > UNIT_MICRO_EPSILON;
                            carrier.x = next_x;
                            carrier.y = next_y;
                        }
                    }
                    UnitPhysicalState::Settled(site) => {
                        if micro {
                            let Some((target_site, target_y)) = settled_targets.get(&carrier.id)
                            else {
                                missing_settled_targets = missing_settled_targets.saturating_add(1);
                                continue;
                            };
                            if *target_site != site {
                                missing_settled_targets = missing_settled_targets.saturating_add(1);
                                continue;
                            }
                            carrier.ideal_x = site as f32;
                            carrier.ideal_y = *target_y;
                            let (x, y, arrived) = Self::move_unit_towards(
                                carrier.x,
                                carrier.y,
                                carrier.ideal_x,
                                carrier.ideal_y,
                                UNIT_MICRO_SETTLE_STEP,
                            );
                            changed |= x != carrier.x || y != carrier.y;
                            carrier.x = x;
                            carrier.y = y;
                            carrier.arrived = arrived;
                        } else {
                            carrier.arrived = true;
                            let height = heights.get(site).copied().unwrap_or(0);
                            carrier.x = site as f32;
                            carrier.y = Self::unit_target_y(grid_height, height);
                            carrier.ideal_x = carrier.x;
                            carrier.ideal_y = carrier.y;
                        }
                    }
                    UnitPhysicalState::Discharged => discharged_done.push(carrier.id),
                }
            }
        }

        if missing_settled_targets > 0 {
            self.flowviz_unit_misses = self
                .flowviz_unit_misses
                .saturating_add(missing_settled_targets);
        }
        if micro {
            changed |= self.relax_unit_micro_positions();
        }

        for id in discharged_done {
            self.flowviz_unit_carriers.remove(&id);
            changed = true;
        }
        changed |= self.reveal_arrived_unit_settlement();
        changed
    }

    fn reveal_arrived_unit_settlement(&mut self) -> bool {
        let mut changed = false;
        for site in 0..self.columns.len() {
            loop {
                let depth = self.flowviz_shadow_columns[site].len();
                if depth >= self.columns[site].len() {
                    break;
                }
                let Some(id) = self.flowviz_unit_custody[site]
                    .get(depth)
                    .copied()
                    .flatten()
                else {
                    self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
                    break;
                };
                let can_reveal = self.flowviz_unit_carriers.get(&id).is_some_and(|carrier| {
                    carrier.arrived
                        && carrier.category_id == self.columns[site][depth]
                        && carrier.physical == UnitPhysicalState::Settled(site)
                });
                if !can_reveal {
                    break;
                }
                self.flowviz_unit_custody[site][depth] = None;
                self.flowviz_shadow_columns[site].push(self.columns[site][depth]);
                self.flowviz_unit_carriers.remove(&id);
                self.flowviz_unit_reveals = self.flowviz_unit_reveals.saturating_add(1);
                changed = true;
            }
        }
        changed
    }

    pub(super) fn render_unit_flowviz_into_surface(&mut self) {
        if !self.flowviz_unit {
            return;
        }
        if self.flowviz_unit_micro {
            for (_, category_id, x, y) in self.collect_unit_micro_render_cells() {
                self.surface.grid[y][x] = Some(category_id);
                self.surface.mobilized[y][x] = true;
            }
            return;
        }

        let height = self.surface.grid_height_dots;
        let width = self.surface.grid_width_dots;
        for carrier in self.flowviz_unit_carriers.values() {
            let x = carrier.x.round() as isize;
            let y = carrier.y.round() as isize;
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as usize, y as usize);
            if x >= width || y >= height {
                continue;
            }
            self.surface.grid[y][x] = Some(carrier.category_id);
            self.surface.mobilized[y][x] = true;
        }
    }

    pub(crate) fn flowviz_unit_enabled(&self) -> bool {
        self.flowviz_unit
    }

    #[cfg(test)]
    pub(crate) fn flowviz_unit_micro_enabled(&self) -> bool {
        self.flowviz_unit_micro
    }

    pub(crate) fn flowviz_unit_carrier_count(&self) -> usize {
        self.flowviz_unit_carriers.len()
    }

    pub(crate) fn flowviz_unit_peak(&self) -> usize {
        self.flowviz_unit_peak
    }

    pub(crate) fn flowviz_unit_segments(&self) -> usize {
        self.flowviz_unit_segments
    }

    pub(crate) fn flowviz_unit_reveals(&self) -> usize {
        self.flowviz_unit_reveals
    }

    pub(crate) fn flowviz_unit_misses(&self) -> usize {
        self.flowviz_unit_misses
    }

    pub(crate) fn flowviz_unit_egress_pending(&self) -> usize {
        self.flowviz_unit_carriers
            .values()
            .filter(|carrier| carrier.physical == UnitPhysicalState::Discharged)
            .count()
    }

    #[cfg(test)]
    pub(super) fn flowviz_unit_settled_target_y(
        &self,
        id: FlowVizMotionId,
    ) -> Option<f32> {
        self.unit_settled_targets(self.surface.grid_height_dots)
            .get(&id)
            .map(|(_, y)| *y)
    }

    #[cfg(test)]
    pub(super) fn flowviz_unit_mass_matches_physics(&self) -> bool {
        self.flowviz_shadow_columns
            .iter()
            .map(Vec::len)
            .sum::<usize>()
            + self.flowviz_unit_carriers.len()
            == self.settled_count() + self.rolling_grains.len() + self.flowviz_unit_egress_pending()
    }
}
