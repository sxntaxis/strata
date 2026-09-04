use super::*;

const PARCEL_CAP: usize = 8192;
const PARCEL_MERGE_LOOKBACK: usize = 48;
const PARCEL_MAX_MASS: usize = 16;
pub(super) const PARCEL_RENDER_MASS_CAP: usize = 16;
const PARCEL_DEPOSIT_PER_FRAME: usize = 4;
const PARCEL_EDGE_TRANSFER_PER_FRAME: usize = 4;
const GRAVITY: f32 = 0.20;
const MAX_FALL_SPEED: f32 = 0.95;

/// Presentation custody for real CategoryId mass that has left the visual
/// settled surface but has not yet been shown depositing again.
///
/// Parcels are deliberately *fungible by CategoryId*. They are not grain
/// identity and do not carry a prescribed future path. Physical adjacent
/// transfers accumulate directional, category-specific Eulerian transport
/// quotas; parcels consume those quotas locally while the visual shadow surface
/// remains mass-conservative.
///
/// Parcels have no sediment authority. They do not enter relief, support,
/// threshold, RNG, persistence, or physical mass accounting.
#[derive(Debug, Clone)]
pub(super) struct FlowVizParcel {
    pub(super) x: f32,
    pub(super) y: f32,
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
        self.flowviz_transport_left = vec![Vec::new(); self.columns.len()];
        self.flowviz_transport_right = vec![Vec::new(); self.columns.len()];
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
        self.flowviz_transport_left = vec![Vec::new(); self.columns.len()];
        self.flowviz_transport_right = vec![Vec::new(); self.columns.len()];
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
        self.flowviz_transport_left = vec![Vec::new(); self.columns.len()];
        self.flowviz_transport_right = vec![Vec::new(); self.columns.len()];
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

        let old_left = std::mem::take(&mut self.flowviz_transport_left);
        self.flowviz_transport_left = vec![Vec::new(); new_width];
        for (index, due) in old_left.into_iter().enumerate() {
            let shifted = index.saturating_add(left_added);
            if shifted < new_width {
                self.flowviz_transport_left[shifted] = due;
            }
        }

        let old_right = std::mem::take(&mut self.flowviz_transport_right);
        self.flowviz_transport_right = vec![Vec::new(); new_width];
        for (index, due) in old_right.into_iter().enumerate() {
            let shifted = index.saturating_add(left_added);
            if shifted < new_width {
                self.flowviz_transport_right[shifted] = due;
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
    /// CategoryId is material identity, not persistent grain identity. If the
    /// visual shadow no longer contains a matching unit at this exact physical
    /// source, that physical unit is already represented by fungible same-color
    /// mobile custody. In that case we add only the newly observed local
    /// transport quota and do not manufacture a second parcel.
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
        self.add_flowviz_transport_due(source, destination, category_id, 1);

        let Some(source_y) = self.withdraw_flowviz_shadow_grain(source, category_id) else {
            // Anonymous same-category material can already be visually mobile
            // while physical custody has temporarily settled/re-entered at a
            // different site. This is reuse, not a conservation miss.
            self.flowviz_reused_deposits = self.flowviz_reused_deposits.saturating_add(1);
            return;
        };
        self.flowviz_visual_withdrawals = self.flowviz_visual_withdrawals.saturating_add(1);
        self.flowviz_mobile_mass = self.flowviz_mobile_mass.saturating_add(1);
        self.spawn_or_merge_flowviz_parcel(source, source_y, destination, category_id);
    }

    /// Mirror ordinary non-moving-phase Oslo one-site transfers immediately
    /// when the visual shadow still owns the local unit. If that same-category
    /// unit is already under mobile visual custody, preserve custody and record
    /// only the physical local transport quota for the existing parcel field.
    pub(super) fn mirror_flowviz_settled_transfer(
        &mut self,
        source: usize,
        destination: usize,
        category_id: CategoryId,
    ) {
        if !self.flowviz_conservative {
            return;
        }
        if self.withdraw_flowviz_shadow_grain(source, category_id).is_some() {
            if let Some(column) = self.flowviz_shadow_columns.get_mut(destination) {
                column.push(category_id);
            }
            return;
        }

        self.record_flowviz_edge_flux(source, destination);
        self.add_flowviz_transport_due(source, destination, category_id, 1);
        self.flowviz_reused_deposits = self.flowviz_reused_deposits.saturating_add(1);
    }

    pub(super) fn mirror_flowviz_settled_discharge(
        &mut self,
        source: usize,
        category_id: CategoryId,
    ) {
        if !self.flowviz_conservative {
            return;
        }
        if self.withdraw_flowviz_shadow_grain(source, category_id).is_some() {
            return;
        }
        if self.consume_flowviz_parcel_mass(category_id, 1) {
            self.flowviz_mobile_mass = self.flowviz_mobile_mass.saturating_sub(1);
            self.flowviz_reused_deposits = self.flowviz_reused_deposits.saturating_add(1);
            return;
        }
        if self
            .withdraw_nearest_flowviz_shadow_surplus(source, category_id)
            .is_some()
        {
            // CategoryId is fungible material identity. After a sequence of
            // anonymous settle/re-entry/local-topple cycles, the shadow can
            // legally own the category surplus at a different site than the
            // physical unit that is now discharging. Consume only a *current
            // physical-vs-shadow surplus* so total/category custody stays exact
            // without inventing persistent grain identity.
            self.flowviz_reused_deposits = self.flowviz_reused_deposits.saturating_add(1);
            return;
        }

        // Reaching this point means the physical discharge removed one unit
        // for which conservative visual custody has neither matching parcel
        // mass nor any same-category settled surplus. That is a true custody
        // invariant failure rather than an exact-site mismatch.
        self.flowviz_shadow_misses = self.flowviz_shadow_misses.saturating_add(1);
    }

    pub(super) fn mirror_flowviz_settled_push(&mut self, site: usize, category_id: CategoryId) {
        if !self.flowviz_conservative {
            return;
        }
        if let Some(column) = self.flowviz_shadow_columns.get_mut(site) {
            column.push(category_id);
        }
    }

    /// Physical settlement is intentionally not mirrored immediately. It
    /// changes the dynamic per-site CategoryId demand seen by parcels; visible
    /// settlement occurs only when mobile visual mass reaches that demand.
    pub(super) fn record_flowviz_settlement(&mut self, _site: usize, _category_id: CategoryId) {}

    fn withdraw_flowviz_shadow_grain(
        &mut self,
        site: usize,
        category_id: CategoryId,
    ) -> Option<usize> {
        let column = self.flowviz_shadow_columns.get_mut(site)?;
        let depth = column.iter().rposition(|category| *category == category_id)?;
        column.remove(depth);
        // Withdrawal success is a custody fact; render-space projection is not.
        // A shadow column may legitimately extend above the current viewport.
        // Saturate such a source to the top visible row rather than returning
        // `None` after already removing the CategoryId from conservative custody.
        Some(
            self.surface
                .grid_height_dots
                .saturating_sub(depth.saturating_add(1)),
        )
    }

    fn withdraw_nearest_flowviz_shadow_surplus(
        &mut self,
        preferred_site: usize,
        category_id: CategoryId,
    ) -> Option<usize> {
        let candidate = (0..self.flowviz_shadow_columns.len())
            .filter_map(|site| {
                let physical = self
                    .columns
                    .get(site)
                    .map_or(0, |column| Self::category_count(column, category_id));
                let visual = self
                    .flowviz_shadow_columns
                    .get(site)
                    .map_or(0, |column| Self::category_count(column, category_id));
                (visual > physical).then_some((site.abs_diff(preferred_site), site))
            })
            .min();

        let (_, site) = candidate?;
        self.withdraw_flowviz_shadow_grain(site, category_id)
    }

    fn category_count(column: &[CategoryId], category_id: CategoryId) -> usize {
        column
            .iter()
            .filter(|category| **category == category_id)
            .count()
    }

    fn flowviz_deposit_demand(&self, site: usize, category_id: CategoryId) -> usize {
        let physical = self
            .columns
            .get(site)
            .map_or(0, |column| Self::category_count(column, category_id));
        let visual = self
            .flowviz_shadow_columns
            .get(site)
            .map_or(0, |column| Self::category_count(column, category_id));
        physical.saturating_sub(visual)
    }

    fn add_category_count(
        ledger: &mut [Vec<(CategoryId, usize)>],
        site: usize,
        category_id: CategoryId,
        count: usize,
    ) {
        if count == 0 || site >= ledger.len() {
            return;
        }
        let entries = &mut ledger[site];
        if let Some((_, existing)) = entries
            .iter_mut()
            .find(|(category, _)| *category == category_id)
        {
            *existing = existing.saturating_add(count);
        } else {
            entries.push((category_id, count));
        }
    }

    fn category_count_in_ledger(
        ledger: &[Vec<(CategoryId, usize)>],
        site: usize,
        category_id: CategoryId,
    ) -> usize {
        ledger
            .get(site)
            .and_then(|entries| {
                entries
                    .iter()
                    .find(|(category, _)| *category == category_id)
            })
            .map_or(0, |(_, count)| *count)
    }

    fn consume_category_count(
        ledger: &mut [Vec<(CategoryId, usize)>],
        site: usize,
        category_id: CategoryId,
        requested: usize,
    ) -> usize {
        let Some(entries) = ledger.get_mut(site) else {
            return 0;
        };
        let Some(index) = entries
            .iter()
            .position(|(category, _)| *category == category_id)
        else {
            return 0;
        };
        let consumed = requested.min(entries[index].1);
        entries[index].1 -= consumed;
        if entries[index].1 == 0 {
            entries.swap_remove(index);
        }
        consumed
    }

    pub(super) fn add_flowviz_transport_due(
        &mut self,
        source: usize,
        destination: usize,
        category_id: CategoryId,
        count: usize,
    ) {
        if !self.flowviz_conservative || source == destination || count == 0 {
            return;
        }
        if destination > source {
            Self::add_category_count(
                &mut self.flowviz_transport_right,
                source,
                category_id,
                count,
            );
        } else {
            Self::add_category_count(
                &mut self.flowviz_transport_left,
                source,
                category_id,
                count,
            );
        }
    }

    fn flowviz_transport_due(
        &self,
        source: usize,
        direction: i8,
        category_id: CategoryId,
    ) -> usize {
        match direction {
            -1 => Self::category_count_in_ledger(
                &self.flowviz_transport_left,
                source,
                category_id,
            ),
            1 => Self::category_count_in_ledger(
                &self.flowviz_transport_right,
                source,
                category_id,
            ),
            _ => 0,
        }
    }

    fn consume_flowviz_transport_due(
        &mut self,
        source: usize,
        direction: i8,
        category_id: CategoryId,
        requested: usize,
    ) -> usize {
        match direction {
            -1 => Self::consume_category_count(
                &mut self.flowviz_transport_left,
                source,
                category_id,
                requested,
            ),
            1 => Self::consume_category_count(
                &mut self.flowviz_transport_right,
                source,
                category_id,
                requested,
            ),
            _ => 0,
        }
    }

    pub(crate) fn flowviz_total_transport_due(&self) -> usize {
        self.flowviz_transport_left
            .iter()
            .chain(self.flowviz_transport_right.iter())
            .flat_map(|entries| entries.iter())
            .map(|(_, count)| *count)
            .sum()
    }

    pub(crate) fn flowviz_total_dynamic_deposit_demand(&self) -> usize {
        let mut total = 0usize;
        for site in 0..self.columns.len() {
            let mut seen = Vec::<CategoryId>::new();
            for category_id in self.columns[site].iter().copied() {
                if seen.contains(&category_id) {
                    continue;
                }
                seen.push(category_id);
                total = total.saturating_add(self.flowviz_deposit_demand(site, category_id));
            }
        }
        total
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
        _destination: usize,
        category_id: CategoryId,
    ) {
        let start_x = source as f32;
        let start_y = source_y as f32;
        let source_site = source;

        let merged_nearby = if let Some(parcel) = self
            .flowviz_parcels
            .iter_mut()
            .rev()
            .take(PARCEL_MERGE_LOOKBACK)
            .find(|parcel| {
                parcel.category_id == category_id
                    && parcel.mass < PARCEL_MAX_MASS
                    && parcel.x.floor() as usize == source_site
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

        if self.flowviz_parcels.len() >= PARCEL_CAP {
            // The cap is soft for conservative mode. Exact visual mass custody
            // outranks bounded sample count; never merge across unrelated sites
            // and never drop mass merely to satisfy a presentation cap.
            self.flowviz_dropped_samples = self.flowviz_dropped_samples.saturating_add(1);
        }

        let initial_vy = self.flowviz_rand_unit() * 0.12;
        self.flowviz_parcels.push_back(FlowVizParcel {
            x: start_x,
            y: start_y,
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

    fn preferred_transport_direction(&self, site: usize, category_id: CategoryId) -> i8 {
        let left = self.flowviz_transport_due(site, -1, category_id);
        let right = self.flowviz_transport_due(site, 1, category_id);
        match left.cmp(&right) {
            std::cmp::Ordering::Greater => -1,
            std::cmp::Ordering::Less => 1,
            std::cmp::Ordering::Equal if left > 0 => {
                let flux = self.flowviz_flux_bias(site);
                if flux < 0 {
                    -1
                } else if flux > 0 {
                    1
                } else {
                    1
                }
            }
            _ => 0,
        }
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
                parcel.y = (parcel.y + parcel.vy).min(contact_y);
                changed = true;
                survivors.push_back(parcel);
                continue;
            }

            parcel.y = contact_y;
            parcel.vy = 0.0;

            let demand_here = self.flowviz_deposit_demand(site, parcel.category_id);
            let deposit = parcel
                .mass
                .min(demand_here)
                .min(PARCEL_DEPOSIT_PER_FRAME);
            if deposit > 0 {
                for _ in 0..deposit {
                    self.flowviz_shadow_columns[site].push(parcel.category_id);
                }
                heights[site] = heights[site].saturating_add(deposit);
                parcel.mass -= deposit;
                self.flowviz_mobile_mass = self.flowviz_mobile_mass.saturating_sub(deposit);
                self.flowviz_visual_deposits =
                    self.flowviz_visual_deposits.saturating_add(deposit);
                changed = true;
            }
            if parcel.mass == 0 {
                continue;
            }

            let direction = self.preferred_transport_direction(site, parcel.category_id);
            let next_site = match direction {
                -1 => site.checked_sub(1),
                1 => site.checked_add(1),
                _ => None,
            }
            .filter(|candidate| *candidate >= visible_start && *candidate < visible_end);

            let Some(next_site) = next_site else {
                survivors.push_back(parcel);
                continue;
            };

            let available = self.flowviz_transport_due(site, direction, parcel.category_id);
            let transfer = parcel
                .mass
                .min(available)
                .min(PARCEL_EDGE_TRANSFER_PER_FRAME);
            if transfer == 0 {
                survivors.push_back(parcel);
                continue;
            }

            let consumed = self.consume_flowviz_transport_due(
                site,
                direction,
                parcel.category_id,
                transfer,
            );
            if consumed == 0 {
                survivors.push_back(parcel);
                continue;
            }

            if consumed < parcel.mass {
                let mut moving = parcel.clone();
                moving.mass = consumed;
                parcel.mass -= consumed;
                survivors.push_back(parcel);
                parcel = moving;
            }

            let next_contact = Self::shadow_contact_y(grid_height, heights[next_site]);
            parcel.x = next_site as f32;
            // Authoritative adjacent flux says this material really crossed the
            // edge. Keep the visual move local and let gravity settle toward the
            // current shadow bed instead of blocking on stale shadow geometry.
            if next_contact >= contact_y {
                parcel.y = (parcel.y + GRAVITY).min(next_contact);
            }
            changed = true;
            survivors.push_back(parcel);
        }

        self.flowviz_parcels = survivors;
        self.record_flowviz_parcel_peaks();
        changed
    }
}
