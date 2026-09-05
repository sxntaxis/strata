use super::flowviz_unit::{UNIT_SEGMENT_STEP, UnitActiveSegment, UnitPhysicalState};
use super::*;

impl OsloSandboxEngine {
    fn unit_target_y(grid_height: usize, shadow_height: usize) -> f32 {
        grid_height.saturating_sub(shadow_height.saturating_add(1)) as f32
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
        let mut changed = false;
        let mut discharged_done = Vec::new();

        for carrier in self.flowviz_unit_carriers.values_mut() {
            if carrier.active.is_none()
                && let Some(segment) = carrier.queued.pop_front()
            {
                debug_assert!(
                    segment.sequence > 0 && segment.sequence < observed_sequence_limit,
                    "unit carrier can replay only an already-observed physical segment"
                );
                carrier.active = Some(UnitActiveSegment {
                    segment,
                    start_x: carrier.x,
                    start_y: carrier.y,
                    progress: 0.0,
                });
            }

            if let Some(active) = &mut carrier.active {
                active.progress = (active.progress + UNIT_SEGMENT_STEP).min(1.0);
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
                    || (active.start_y + 1.0).min(grid_height.saturating_sub(1) as f32),
                    |site| {
                        let height = heights.get(site).copied().unwrap_or(0);
                        Self::unit_target_y(grid_height, height)
                    },
                );
                let t = active.progress;
                carrier.x = active.start_x + (target_x - active.start_x) * t;
                carrier.y = active.start_y + (target_y - active.start_y) * t;
                changed = true;
                if active.progress >= 1.0 {
                    carrier.active = None;
                }
            }

            if carrier.active.is_none() && carrier.queued.is_empty() {
                match carrier.physical {
                    UnitPhysicalState::Rolling => {}
                    UnitPhysicalState::Settled(site) => {
                        carrier.arrived = true;
                        let height = heights.get(site).copied().unwrap_or(0);
                        carrier.x = site as f32;
                        carrier.y = Self::unit_target_y(grid_height, height);
                    }
                    UnitPhysicalState::Discharged => discharged_done.push(carrier.id),
                }
            }
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
    pub(super) fn flowviz_unit_mass_matches_physics(&self) -> bool {
        self.flowviz_shadow_columns
            .iter()
            .map(Vec::len)
            .sum::<usize>()
            + self.flowviz_unit_carriers.len()
            == self.settled_count() + self.rolling_grains.len() + self.flowviz_unit_egress_pending()
    }
}
