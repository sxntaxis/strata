use super::*;

const FLOWVIZ_TRACER_CAP: usize = 4096;
const FLOWVIZ_TRACER_TTL_FRAMES: u16 = 240;
const FLOWVIZ_FLUX_CAP: i16 = 64;
const FLOWVIZ_FLUX_DECAY_NUM: i16 = 7;
const FLOWVIZ_FLUX_DECAY_DEN: i16 = 8;
const FLOWVIZ_SURFACE_SPEED: f32 = 0.55;
const FLOWVIZ_AIRBORNE_X_SPEED: f32 = 0.30;
const FLOWVIZ_GRAVITY: f32 = 0.20;
const FLOWVIZ_MAX_FALL_SPEED: f32 = 0.95;
const FLOWVIZ_SETTLE_FRAMES: u8 = 2;

#[derive(Debug, Clone, Copy)]
pub(super) struct FlowVizTracer {
    pub(super) x: f32,
    pub(super) y: f32,
    vx: f32,
    vy: f32,
    pub(super) category_id: CategoryId,
    age: u16,
    resting_frames: u8,
}

impl OsloSandboxEngine {
    pub(super) fn enable_flowviz(&mut self, seed: u64) {
        self.flowviz_enabled = true;
        self.flowviz_conservative = false;
        self.flowviz_edge_flux = vec![0; self.columns.len()];
        self.flowviz_tracers.clear();
        self.flowviz_rng_state = seed ^ 0xB529_7A4D_1C68_E9D7;
        if self.flowviz_rng_state == 0 {
            self.flowviz_rng_state = 0xB529_7A4D_1C68_E9D7;
        }
        self.flowviz_spawned = 0;
        self.flowviz_peak_tracers = 0;
        self.flowviz_dropped_samples = 0;
    }

    pub(super) fn clear_flowviz(&mut self) {
        self.flowviz_edge_flux = vec![0; self.columns.len()];
        self.flowviz_tracers.clear();
        self.flowviz_spawned = 0;
        self.flowviz_peak_tracers = 0;
        self.flowviz_dropped_samples = 0;
        if self.flowviz_conservative {
            self.clear_conservative_flowviz();
        }
    }

    pub(super) fn resize_flowviz_for_growth(&mut self, left_added: usize, new_width: usize) {
        if !self.flowviz_enabled {
            return;
        }
        let old = std::mem::take(&mut self.flowviz_edge_flux);
        self.flowviz_edge_flux = vec![0; new_width];
        for (index, flux) in old.into_iter().enumerate() {
            let shifted = index.saturating_add(left_added);
            if shifted < self.flowviz_edge_flux.len() {
                self.flowviz_edge_flux[shifted] = flux;
            }
        }
        if self.flowviz_conservative {
            self.resize_conservative_flowviz_for_growth(left_added, new_width);
        }
        if left_added > 0 {
            let shift = left_added as f32;
            for tracer in &mut self.flowviz_tracers {
                tracer.x += shift;
            }
        }
    }

    pub(super) fn flowviz_rand_unit(&mut self) -> f32 {
        self.flowviz_rng_state ^= self.flowviz_rng_state << 13;
        self.flowviz_rng_state ^= self.flowviz_rng_state >> 7;
        self.flowviz_rng_state ^= self.flowviz_rng_state << 17;
        let sample = (self.flowviz_rng_state >> 40) as u32;
        sample as f32 / 0x00FF_FFFFu32 as f32
    }

    pub(super) fn record_flowviz_edge_flux(&mut self, source: usize, destination: usize) {
        if !self.flowviz_enabled || source == destination {
            return;
        }
        let right = destination > source;
        let edge = source.min(destination);
        if let Some(flux) = self.flowviz_edge_flux.get_mut(edge) {
            let delta = if right { 1 } else { -1 };
            *flux = flux
                .saturating_add(delta)
                .clamp(-FLOWVIZ_FLUX_CAP, FLOWVIZ_FLUX_CAP);
        }
    }

    pub(super) fn record_flowviz_transfer(
        &mut self,
        source: usize,
        destination: usize,
        category_id: CategoryId,
        source_y: usize,
    ) {
        if !self.flowviz_enabled || source == destination || self.columns.is_empty() {
            return;
        }
        self.record_flowviz_edge_flux(source, destination);
        if self.flowviz_conservative {
            self.add_flowviz_transport_due(source, destination, category_id, 1);
            return;
        }
        let right = destination > source;

        if self.flowviz_tracers.len() >= FLOWVIZ_TRACER_CAP {
            self.flowviz_dropped_samples = self.flowviz_dropped_samples.saturating_add(1);
            return;
        }

        // Tracers visualize transport flux, not one-to-one physical grain identity.
        // Small deterministic jitter prevents many successive transfer samples from
        // collapsing into artificial combs while remaining fully presentation-only.
        let jitter = self.flowviz_rand_unit() - 0.5;
        let vertical_jitter = self.flowviz_rand_unit() * 0.18;
        let direction = if right { 1.0 } else { -1.0 };
        self.flowviz_tracers.push_back(FlowVizTracer {
            x: source as f32 + jitter * 0.18,
            y: source_y as f32,
            vx: direction * (FLOWVIZ_AIRBORNE_X_SPEED + jitter.abs() * 0.10),
            vy: vertical_jitter,
            category_id,
            age: 0,
            resting_frames: 0,
        });
        self.flowviz_spawned = self.flowviz_spawned.saturating_add(1);
        self.flowviz_peak_tracers = self.flowviz_peak_tracers.max(self.flowviz_tracers.len());
    }

    fn flowviz_contact_y_from_heights(grid_height: usize, height: usize) -> f32 {
        if grid_height == 0 {
            return 0.0;
        }
        if height == 0 {
            return grid_height.saturating_sub(1) as f32;
        }
        grid_height.saturating_sub(height) as f32
    }

    pub(super) fn flowviz_flux_bias(&self, site: usize) -> i16 {
        let right = self.flowviz_edge_flux.get(site).copied().unwrap_or(0);
        let left = site
            .checked_sub(1)
            .and_then(|edge| self.flowviz_edge_flux.get(edge).copied())
            .unwrap_or(0);
        right.saturating_sub(left)
    }

    fn flowviz_downhill_direction(&self, site: usize) -> i8 {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let here = self.columns.get(site).map_or(0, Vec::len);
        let left = site
            .checked_sub(1)
            .filter(|candidate| *candidate >= visible_start)
            .and_then(|candidate| self.columns.get(candidate).map(Vec::len));
        let right = site
            .checked_add(1)
            .filter(|candidate| *candidate < visible_end)
            .and_then(|candidate| self.columns.get(candidate).map(Vec::len));

        match (left, right) {
            (Some(l), Some(r)) if l < here || r < here => {
                if l < r {
                    -1
                } else if r < l {
                    1
                } else {
                    0
                }
            }
            (Some(l), None) if l < here => -1,
            (None, Some(r)) if r < here => 1,
            _ => 0,
        }
    }

    pub(super) fn decay_flowviz_flux(&mut self) {
        for flux in &mut self.flowviz_edge_flux {
            *flux = (*flux as i32 * FLOWVIZ_FLUX_DECAY_NUM as i32 / FLOWVIZ_FLUX_DECAY_DEN as i32)
                as i16;
        }
    }

    pub(super) fn advance_flowviz_tracers(&mut self) -> bool {
        if !self.flowviz_enabled {
            return false;
        }
        if self.flowviz_conservative {
            return self.advance_conservative_flowviz();
        }

        self.decay_flowviz_flux();
        if self.flowviz_tracers.is_empty() {
            self.flowviz_edge_flux.fill(0);
            return false;
        }

        let grid_height = self.surface.grid_height_dots;
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        if visible_start >= visible_end {
            self.flowviz_tracers.clear();
            return true;
        }
        let heights = self.columns.iter().map(Vec::len).collect::<Vec<_>>();
        let mut changed = false;
        let mut survivors = VecDeque::with_capacity(self.flowviz_tracers.len());

        while let Some(mut tracer) = self.flowviz_tracers.pop_front() {
            tracer.age = tracer.age.saturating_add(1);
            if tracer.age > FLOWVIZ_TRACER_TTL_FRAMES {
                changed = true;
                continue;
            }

            let mut site = tracer.x.floor() as isize;
            site = site.clamp(
                visible_start as isize,
                visible_end.saturating_sub(1) as isize,
            );
            let site = site as usize;
            let contact_y = Self::flowviz_contact_y_from_heights(grid_height, heights[site]);

            if tracer.y + 0.05 < contact_y {
                tracer.vy = (tracer.vy + FLOWVIZ_GRAVITY).min(FLOWVIZ_MAX_FALL_SPEED);
                tracer.x += tracer.vx;
                tracer.y = (tracer.y + tracer.vy).min(contact_y);
                tracer.resting_frames = 0;
                changed = true;
            } else {
                tracer.y = contact_y;
                tracer.vy = 0.0;
                let flux_bias = self.flowviz_flux_bias(site);
                let direction = if flux_bias > 0 {
                    1
                } else if flux_bias < 0 {
                    -1
                } else {
                    self.flowviz_downhill_direction(site)
                };

                let next_site = match direction {
                    -1 => site.checked_sub(1),
                    1 => site.checked_add(1),
                    _ => None,
                }
                .filter(|candidate| *candidate >= visible_start && *candidate < visible_end);

                if let Some(next_site) = next_site {
                    let next_contact =
                        Self::flowviz_contact_y_from_heights(grid_height, heights[next_site]);
                    let dx = if next_site > site { 1.0 } else { -1.0 };
                    tracer.vx = dx * FLOWVIZ_SURFACE_SPEED;
                    tracer.x += tracer.vx;
                    // A tracer follows the local surface/rolling layer instead of
                    // targeting a precomputed destination. Downward surface steps
                    // are traversed under gravity; uphill steps stop the sample.
                    if next_contact >= contact_y {
                        tracer.y = (tracer.y + FLOWVIZ_GRAVITY).min(next_contact);
                        tracer.resting_frames = 0;
                        changed = true;
                    } else {
                        tracer.resting_frames = tracer.resting_frames.saturating_add(1);
                    }
                } else {
                    tracer.resting_frames = tracer.resting_frames.saturating_add(1);
                }
            }

            if tracer.x < visible_start as f32
                || tracer.x >= visible_end as f32
                || tracer.y >= grid_height as f32
                || tracer.resting_frames >= FLOWVIZ_SETTLE_FRAMES
            {
                changed = true;
                continue;
            }
            survivors.push_back(tracer);
        }

        self.flowviz_tracers = survivors;
        changed
    }

    pub(super) fn render_flowviz_tracers_into_surface(&mut self) {
        if !self.flowviz_enabled {
            return;
        }
        if self.flowviz_conservative {
            self.render_conservative_flowviz_parcels_into_surface();
            return;
        }
        let grid_height = self.surface.grid_height_dots;
        let grid_width = self.surface.grid_width_dots;
        for tracer in &self.flowviz_tracers {
            let x = tracer.x.round() as isize;
            let y = tracer.y.round() as isize;
            if x < 0 || y < 0 {
                continue;
            }
            let x = x as usize;
            let y = y as usize;
            if x >= grid_width || y >= grid_height {
                continue;
            }
            self.surface.grid[y][x] = Some(tracer.category_id);
            self.surface.mobilized[y][x] = true;
        }
    }

    pub(crate) fn flowviz_enabled(&self) -> bool {
        self.flowviz_enabled
    }

    pub(crate) fn flowviz_conservative_enabled(&self) -> bool {
        self.flowviz_conservative
    }

    pub(crate) fn flowviz_tracer_count(&self) -> usize {
        if self.flowviz_conservative {
            self.flowviz_parcels.len()
        } else {
            self.flowviz_tracers.len()
        }
    }

    pub(crate) fn flowviz_peak_tracers(&self) -> usize {
        if self.flowviz_conservative {
            self.flowviz_peak_parcels
        } else {
            self.flowviz_peak_tracers
        }
    }

    pub(crate) fn flowviz_spawned(&self) -> usize {
        self.flowviz_spawned
    }

    pub(crate) fn flowviz_dropped_samples(&self) -> usize {
        self.flowviz_dropped_samples
    }

    pub(crate) fn flowviz_active_flux_edges(&self) -> usize {
        self.flowviz_edge_flux
            .iter()
            .filter(|flux| **flux != 0)
            .count()
    }
}
