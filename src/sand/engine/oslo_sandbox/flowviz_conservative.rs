use super::*;

const PARCEL_CAP: usize = 8192;
const PARCEL_MERGE_LOOKBACK: usize = 48;
const PARCEL_MAX_MASS: usize = 16;
pub(super) const PARCEL_RENDER_MASS_CAP: usize = 16;
const PARCEL_DEPOSIT_PER_FRAME: usize = 4;
const SINK_SEARCH_RADIUS: usize = 128;
const SURFACE_SPEED: f32 = 0.55;
const AIRBORNE_X_SPEED: f32 = 0.30;
const GRAVITY: f32 = 0.20;
const MAX_FALL_SPEED: f32 = 0.95;

/// Presentation custody for real CategoryId mass that has left the visual
/// settled surface but has not yet been shown depositing again.
///
/// Parcels have no sediment authority. They do not enter relief, support,
/// threshold, RNG, persistence, or physical mass accounting.
#[derive(Debug, Clone)]
pub(super) struct FlowVizParcel {
    pub(super) x: f32,
    pub(super) y: f32,
    vx: f32,
    vy: f32,
    pub(super) category_id: CategoryId,
    pub(super) mass: usize,
    age: u16,
}

impl OsloSandboxEngine {
    pub(super) fn enable_conservative_flowviz(&mut self, seed: u64) {
        self.flowviz_enabled = true;
        self.flowviz_conservative = true;
        self.flowviz_edge_flux = vec![0; self.columns.len()];
        self.flowviz_tracers.clear();
        self.flowviz_parcels.clear();
        self.flowviz_shadow_columns = self.columns.clone();
        self.flowviz_deposit_due = vec![Vec::new(); self.columns.len()];
        self.flowviz_spawned = 0;
        self.flowviz_peak_tracers = 0;
        self.flowviz_dropped_samples = 0;
        self.flowviz_rng_state = seed ^ 0xA6E8_71C4_4D39_20F5;
        if self.flowviz_rng_state == 0 {
            self.flowviz_rng_state = 0xA6E8_71C4_4D39_20F5;
        }
        self.reset_conservative_flowviz_diagnostics();
    }

    pub(super) fn clear_conservative_flowviz(&mut self) {
        self.flowviz_parcels.clear();
        self.flowviz_shadow_columns = self.columns.clone();
        self.flowviz_deposit_due = vec![Vec::new(); self.columns.len()];
        self.reset_conservative_flowviz_diagnostics();
    }

    pub(super) fn reset_conservative_flowviz_from_physics(&mut self) {
        if !self.flowviz_conservative {
            return;
        }
        self.flowviz_edge_flux = vec![0; self.columns.len()];
        self.flowviz_tracers.clear();
        self.flowviz_parcels.clear();
        self.flowviz_shadow_columns = self.columns.clone();
        self.flowviz_deposit_due = vec![Vec::new(); self.columns.len()];
        self.flowviz_spawned = 0;
        self.flowviz_peak_tracers = 0;
        self.flowviz_dropped_samples = 0;
        self.reset_conservative_flowviz_diagnostics();
    }

    fn reset_conservative_flowviz_diagnostics(&mut self) {
        self.flowviz_peak_parcels = 0;
        self.flowviz_mobile_mass = 0;
        self.flowviz_peak_mobile_mass = 0;
        self.flowviz_visual_withdrawals = 0;
        self.flowviz_visual_deposits = 0;
        self.flowviz_reused_deposits = 0;
        self.flowviz_coalesced_mass = 0;
        self.flowviz_shadow_misses = 0;
    }

    pub(super) fn resize_conservative_flowviz_for_growth(
        &mut self,
        left_added: usize,
        new_width: usize,
    ) {
        let old_shadow = std::mem::take(&mut self.flowviz_shadow_columns);
        self.flowviz_shadow_columns = vec![Vec::new(); new_width];
        for (index, column) in old_shadow.into_iter().enumerate() {
            let shifted = index.saturating_add(left_added);
            if shifted < new_width {
                self.flowviz_shadow_columns[shifted] = column;
            }
        }

        let old_due = std::mem::take(&mut self.flowviz_deposit_due);
        self.flowviz_deposit_due = vec![Vec::new(); new_width];
        for (index, due) in old_due.into_iter().enumerate() {
            let shifted = index.saturating_add(left_added);
            if shifted < new_width {
                self.flowviz_deposit_due[shifted] = due;
            }
        }

        if left_added > 0 {
            let shift = left_added as f32;
            for parcel in &mut self.flowviz_parcels {
                parcel.x += shift;
            }
        }
    }

    /// Enter one unit of real CategoryId mass into visual mobile custody.
    ///
    /// A physically settled grain may re-enter motion before its earlier parcel
    /// has visually deposited. In that case, consume the outstanding settlement
    /// credit and keep the already-mobile parcel mass instead of duplicating it.
    pub(super) fn record_flowviz_mobile_entry(
        &mut self,
        source: usize,
        destination: usize,
        category_id: CategoryId,
        physical_source_y: usize,
    ) {
        if !self.flowviz_enabled {
            return;
        }
        if !self.flowviz_conservative {
            self.record_flowviz_transfer(source, destination, category_id, physical_source_y);
            return;
        }

        self.record_flowviz_edge_flux(source, destination);
        if self.consume_flowviz_deposit_due(source, category_id, 1) == 1 {
            self.flowviz_reused_deposits = self.flowviz_reused_deposits.saturating_add(1);
            return;
        }

        let Some(source_y) = self.withdraw_flowviz_shadow_grain(source, category_id) else {
            self.flowviz_shadow_misses = self.flowviz_shadow_misses.saturating_add(1);
            return;
        };
        self.flowviz_visual_withdrawals = self.flowviz_visual_withdrawals.saturating_add(1);
        self.flowviz_mobile_mass = self.flowviz_mobile_mass.saturating_add(1);
        self.spawn_or_merge_flowviz_parcel(source, source_y, destination, category_id);
    }

    /// Mirror ordinary non-moving-phase Oslo one-site transfers immediately.
    /// Only mass that enters the explicit rolling/front phase is delayed behind
    /// the conservative visual surface.
    pub(super) fn mirror_flowviz_settled_transfer(
        &mut self,
        source: usize,
        destination: usize,
        category_id: CategoryId,
    ) {
        if !self.flowviz_conservative {
            return;
        }
        if self.consume_flowviz_deposit_due(source, category_id, 1) == 1 {
            // This physically settled unit is still represented by mobile visual
            // custody. Reroute its real settlement credit locally and expose
            // that adjacent transfer to the parcel flow field without creating
            // a second unit of visual mass.
            self.record_flowviz_edge_flux(source, destination);
            self.add_flowviz_deposit_due(destination, category_id, 1);
            return;
        }
        if self.withdraw_flowviz_shadow_grain(source, category_id).is_none() {
            self.flowviz_shadow_misses = self.flowviz_shadow_misses.saturating_add(1);
            return;
        }
        if let Some(column) = self.flowviz_shadow_columns.get_mut(destination) {
            column.push(category_id);
        }
    }

    pub(super) fn mirror_flowviz_settled_discharge(
        &mut self,
        source: usize,
        category_id: CategoryId,
    ) {
        if !self.flowviz_conservative {
            return;
        }
        if self.consume_flowviz_deposit_due(source, category_id, 1) == 1 {
            if self.consume_flowviz_parcel_mass(category_id, 1) {
                self.flowviz_mobile_mass = self.flowviz_mobile_mass.saturating_sub(1);
            } else {
                self.flowviz_shadow_misses = self.flowviz_shadow_misses.saturating_add(1);
            }
            return;
        }
        if self.withdraw_flowviz_shadow_grain(source, category_id).is_none() {
            self.flowviz_shadow_misses = self.flowviz_shadow_misses.saturating_add(1);
        }
    }

    pub(super) fn mirror_flowviz_settled_push(&mut self, site: usize, category_id: CategoryId) {
        if !self.flowviz_conservative {
            return;
        }
        if let Some(column) = self.flowviz_shadow_columns.get_mut(site) {
            column.push(category_id);
        }
    }

    pub(super) fn record_flowviz_settlement(&mut self, site: usize, category_id: CategoryId) {
        if self.flowviz_conservative {
            self.add_flowviz_deposit_due(site, category_id, 1);
        }
    }

    fn withdraw_flowviz_shadow_grain(
        &mut self,
        site: usize,
        category_id: CategoryId,
    ) -> Option<usize> {
        let column = self.flowviz_shadow_columns.get_mut(site)?;
        let depth = column.iter().rposition(|category| *category == category_id)?;
        column.remove(depth);
        self.surface
            .grid_height_dots
            .checked_sub(depth.saturating_add(1))
    }

    fn add_flowviz_deposit_due(&mut self, site: usize, category_id: CategoryId, count: usize) {
        if count == 0 || site >= self.flowviz_deposit_due.len() {
            return;
        }
        let due = &mut self.flowviz_deposit_due[site];
        if let Some((_, existing)) = due.iter_mut().find(|(category, _)| *category == category_id) {
            *existing = existing.saturating_add(count);
        } else {
            due.push((category_id, count));
        }
    }

    fn flowviz_due_count(&self, site: usize, category_id: CategoryId) -> usize {
        self.flowviz_deposit_due
            .get(site)
            .and_then(|due| due.iter().find(|(category, _)| *category == category_id))
            .map_or(0, |(_, count)| *count)
    }

    fn consume_flowviz_deposit_due(
        &mut self,
        site: usize,
        category_id: CategoryId,
        requested: usize,
    ) -> usize {
        let Some(due) = self.flowviz_deposit_due.get_mut(site) else {
            return 0;
        };
        let Some(index) = due.iter().position(|(category, _)| *category == category_id) else {
            return 0;
        };
        let consumed = requested.min(due[index].1);
        due[index].1 -= consumed;
        if due[index].1 == 0 {
            due.swap_remove(index);
        }
        consumed
    }

    fn consume_flowviz_parcel_mass(&mut self, category_id: CategoryId, requested: usize) -> bool {
        let mut remaining = requested;
        for parcel in &mut self.flowviz_parcels {
            if parcel.category_id != category_id || remaining == 0 {
                continue;
            }
            let consumed = remaining.min(parcel.mass);
            parcel.mass -= consumed;
            remaining -= consumed;
        }
        self.flowviz_parcels.retain(|parcel| parcel.mass > 0);
        remaining == 0
    }

    fn spawn_or_merge_flowviz_parcel(
        &mut self,
        source: usize,
        source_y: usize,
        destination: usize,
        category_id: CategoryId,
    ) {
        let start_x = source as f32 + (self.flowviz_rand_unit() - 0.5) * 0.16;
        let start_y = source_y as f32;

        let merged_nearby = if let Some(parcel) = self
            .flowviz_parcels
            .iter_mut()
            .rev()
            .take(PARCEL_MERGE_LOOKBACK)
            .find(|parcel| {
                parcel.category_id == category_id
                    && parcel.mass < PARCEL_MAX_MASS
                    && (parcel.x - start_x).abs() <= 1.0
                    && (parcel.y - start_y).abs() <= 2.0
            })
        {
            parcel.mass = parcel.mass.saturating_add(1);
            true
        } else {
            false
        };
        if merged_nearby {
            self.flowviz_coalesced_mass = self.flowviz_coalesced_mass.saturating_add(1);
            self.record_flowviz_parcel_peaks();
            return;
        }

        let merged_at_cap = if self.flowviz_parcels.len() >= PARCEL_CAP {
            if let Some(parcel) = self
                .flowviz_parcels
                .iter_mut()
                .find(|parcel| parcel.category_id == category_id)
            {
                parcel.mass = parcel.mass.saturating_add(1);
                true
            } else {
                false
            }
        } else {
            false
        };
        if merged_at_cap {
            self.flowviz_coalesced_mass = self.flowviz_coalesced_mass.saturating_add(1);
            self.record_flowviz_parcel_peaks();
            return;
        }

        let direction = if destination > source { 1.0 } else { -1.0 };
        let initial_vy = self.flowviz_rand_unit() * 0.12;
        self.flowviz_parcels.push_back(FlowVizParcel {
            x: start_x,
            y: start_y,
            vx: direction * AIRBORNE_X_SPEED,
            vy: initial_vy,
            category_id,
            mass: 1,
            age: 0,
        });
        self.flowviz_spawned = self.flowviz_spawned.saturating_add(1);
        self.record_flowviz_parcel_peaks();
    }

    fn record_flowviz_parcel_peaks(&mut self) {
        let parcel_count = self.flowviz_parcels.len();
        self.flowviz_peak_parcels = self.flowviz_peak_parcels.max(parcel_count);
        self.flowviz_peak_mobile_mass = self.flowviz_peak_mobile_mass.max(self.flowviz_mobile_mass);
    }

    fn shadow_contact_y(grid_height: usize, height: usize) -> f32 {
        if height == 0 {
            grid_height.saturating_sub(1) as f32
        } else {
            grid_height.saturating_sub(height) as f32
        }
    }

    fn shadow_downhill_direction(&self, heights: &[usize], site: usize) -> i8 {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let here = heights.get(site).copied().unwrap_or(0);
        let left = site
            .checked_sub(1)
            .filter(|candidate| *candidate >= visible_start)
            .and_then(|candidate| heights.get(candidate).copied());
        let right = site
            .checked_add(1)
            .filter(|candidate| *candidate < visible_end)
            .and_then(|candidate| heights.get(candidate).copied());
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

    fn sink_direction(&self, site: usize, category_id: CategoryId) -> i8 {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        for distance in 1..=SINK_SEARCH_RADIUS {
            let left = site.checked_sub(distance).filter(|candidate| *candidate >= visible_start);
            let right = site
                .checked_add(distance)
                .filter(|candidate| *candidate < visible_end);
            let left_due = left.map_or(0, |candidate| self.flowviz_due_count(candidate, category_id));
            let right_due = right.map_or(0, |candidate| self.flowviz_due_count(candidate, category_id));
            match (left_due > 0, right_due > 0) {
                (true, false) => return -1,
                (false, true) => return 1,
                (true, true) => return self.flowviz_flux_bias(site).signum() as i8,
                (false, false) => {}
            }
        }
        0
    }

    pub(super) fn advance_conservative_flowviz(&mut self) -> bool {
        self.decay_flowviz_flux();
        if self.flowviz_parcels.is_empty() {
            return false;
        }
        let grid_height = self.surface.grid_height_dots;
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        if visible_start >= visible_end {
            return false;
        }

        let mut heights = self
            .flowviz_shadow_columns
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>();
        let mut changed = false;
        let mut survivors = VecDeque::with_capacity(self.flowviz_parcels.len());

        while let Some(mut parcel) = self.flowviz_parcels.pop_front() {
            parcel.age = parcel.age.saturating_add(1);
            let site = (parcel.x.floor() as isize)
                .clamp(visible_start as isize, visible_end.saturating_sub(1) as isize)
                as usize;
            let contact_y = Self::shadow_contact_y(grid_height, heights[site]);

            if parcel.y + 0.05 < contact_y {
                parcel.vy = (parcel.vy + GRAVITY).min(MAX_FALL_SPEED);
                parcel.x = (parcel.x + parcel.vx)
                    .clamp(visible_start as f32, visible_end.saturating_sub(1) as f32 + 0.999);
                parcel.y = (parcel.y + parcel.vy).min(contact_y);
                changed = true;
            } else {
                parcel.y = contact_y;
                parcel.vy = 0.0;

                let due_here = self.flowviz_due_count(site, parcel.category_id);
                let deposit = parcel
                    .mass
                    .min(due_here)
                    .min(PARCEL_DEPOSIT_PER_FRAME);
                if deposit > 0 {
                    let consumed =
                        self.consume_flowviz_deposit_due(site, parcel.category_id, deposit);
                    for _ in 0..consumed {
                        self.flowviz_shadow_columns[site].push(parcel.category_id);
                    }
                    heights[site] = heights[site].saturating_add(consumed);
                    parcel.mass -= consumed;
                    self.flowviz_mobile_mass = self.flowviz_mobile_mass.saturating_sub(consumed);
                    self.flowviz_visual_deposits =
                        self.flowviz_visual_deposits.saturating_add(consumed);
                    changed = true;
                }
                if parcel.mass == 0 {
                    continue;
                }

                let flux = self.flowviz_flux_bias(site);
                let sink = self.sink_direction(site, parcel.category_id);
                let downhill = self.shadow_downhill_direction(&heights, site);
                let direction = if flux > 0 {
                    1
                } else if flux < 0 {
                    -1
                } else if sink != 0 {
                    sink
                } else {
                    downhill
                };
                let next_site = match direction {
                    -1 => site.checked_sub(1),
                    1 => site.checked_add(1),
                    _ => None,
                }
                .filter(|candidate| *candidate >= visible_start && *candidate < visible_end);

                if let Some(next_site) = next_site {
                    let next_contact = Self::shadow_contact_y(grid_height, heights[next_site]);
                    if next_contact >= contact_y {
                        parcel.vx = if next_site > site {
                            SURFACE_SPEED
                        } else {
                            -SURFACE_SPEED
                        };
                        parcel.x = (parcel.x + parcel.vx).clamp(
                            visible_start as f32,
                            visible_end.saturating_sub(1) as f32 + 0.999,
                        );
                        parcel.y = (parcel.y + GRAVITY).min(next_contact);
                        changed = true;
                    } else {
                        // A visual parcel never climbs through a higher shadow bed
                        // merely to reconcile a distant credit. If the target is
                        // not reachable by local flow, wait for bed/flux evolution.
                        parcel.vx = 0.0;
                    }
                } else {
                    parcel.vx = 0.0;
                }
            }
            survivors.push_back(parcel);
        }

        self.flowviz_parcels = survivors;
        self.record_flowviz_parcel_peaks();
        changed
    }
}
