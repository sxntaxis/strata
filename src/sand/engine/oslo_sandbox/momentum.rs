use super::*;

impl ToppleDirection {
    fn step(self, site: usize) -> Option<usize> {
        match self {
            Self::Left => site.checked_sub(1),
            Self::Right => site.checked_add(1),
        }
    }

    fn uphill_step(self, site: usize) -> Option<usize> {
        match self {
            Self::Left => site.checked_add(1),
            Self::Right => site.checked_sub(1),
        }
    }
}

impl OsloSandboxEngine {
    #[cfg(test)]
    pub(super) fn momentum_trigger_relief(&self) -> usize {
        MOMENTUM_TRIGGER_RELIEF
    }

    #[cfg(test)]
    pub(super) fn front_erosion_relief(&self) -> usize {
        FRONT_EROSION_RELIEF
    }

    #[cfg(test)]
    pub(super) fn front_support_loss_relief(&self) -> usize {
        FRONT_SUPPORT_LOSS_RELIEF
    }

    pub(super) fn should_seed_momentum(&self, relief: usize) -> bool {
        self.momentum_enabled && relief >= MOMENTUM_TRIGGER_RELIEF
    }

    pub(super) fn rolling_depth_at(&self, site: usize) -> usize {
        self.rolling_grains
            .iter()
            .filter(|rolling| rolling.site == site)
            .count()
    }

    fn flat_coast_budget(&self, site: usize) -> u8 {
        if !self.front_enabled {
            return MOMENTUM_FLAT_COAST_STEPS;
        }
        let depth_bonus = u8::try_from(self.rolling_depth_at(site) / 3)
            .unwrap_or(u8::MAX)
            .min(FRONT_MAX_FLAT_COAST_STEPS - FRONT_BASE_FLAT_COAST_STEPS);
        FRONT_BASE_FLAT_COAST_STEPS
            .saturating_add(depth_bonus)
            .min(FRONT_MAX_FLAT_COAST_STEPS)
    }

    pub(super) fn top_grain_y(&self, site: usize) -> Option<usize> {
        let height = self.columns.get(site)?.len();
        (height > 0).then(|| self.surface.grid_height_dots.saturating_sub(height))
    }

    fn normalize_column_visual_shape(&mut self, site: usize) {
        let Some(column) = self.columns.get(site) else {
            return;
        };
        let target = column.len();
        let Some(visual) = self.column_visual_y.get_mut(site) else {
            return;
        };
        if visual.len() < target {
            visual.resize(target, None);
        } else if visual.len() > target {
            visual.truncate(target);
        }
    }

    /// Pop the authoritative top grain while preserving its current presentation
    /// elevation. `column_visual_y` is presentation-only and aligned with the
    /// settled CategoryId stack; physics continues to mutate `columns` at the
    /// same event as before SEDIMENT-010.
    pub(super) fn pop_settled_grain(&mut self, site: usize) -> Option<(CategoryId, usize)> {
        self.normalize_column_visual_shape(site);
        let physical_y = self.top_grain_y(site)?;
        let visual_y = if self.flowviz_enabled {
            let _ = self.column_visual_y.get_mut(site)?.pop();
            physical_y
        } else {
            self.column_visual_y
                .get_mut(site)?
                .pop()
                .flatten()
                .unwrap_or(physical_y)
        };
        let category_id = self.columns.get_mut(site)?.pop()?;
        Some((category_id, visual_y))
    }

    /// Push authoritative settled mass immediately, exactly as the frozen front
    /// solver did, while optionally keeping that grain's CategoryId at a prior
    /// visual elevation until the presentation catches up. This avoids changing
    /// support/relief timing merely to animate a tall drop.
    pub(super) fn push_settled_grain_at_visual_y(
        &mut self,
        site: usize,
        category_id: CategoryId,
        visual_y: Option<usize>,
    ) {
        self.normalize_column_visual_shape(site);
        self.columns[site].push(category_id);
        let target_y = self
            .surface
            .grid_height_dots
            .saturating_sub(self.columns[site].len());
        let visual_y = if self.flowviz_enabled {
            None
        } else {
            visual_y.filter(|value| *value != target_y)
        };
        self.column_visual_y[site].push(visual_y);
    }

    pub(super) fn settled_visual_in_transit_count(&self) -> usize {
        self.column_visual_y
            .iter()
            .map(|column| column.iter().filter(|value| value.is_some()).count())
            .sum()
    }

    pub(super) fn advance_settled_visual_motion(&mut self) -> bool {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let grid_height = self.surface.grid_height_dots;
        let mut changed = false;
        for site in visible_start..visible_end.min(self.columns.len()) {
            self.normalize_column_visual_shape(site);
            for (depth, visual_y) in self.column_visual_y[site].iter_mut().enumerate() {
                let Some(current) = *visual_y else {
                    continue;
                };
                let target = grid_height.saturating_sub(depth.saturating_add(1));
                let next = match current.cmp(&target) {
                    std::cmp::Ordering::Less => current.saturating_add(1),
                    std::cmp::Ordering::Greater => current.saturating_sub(1),
                    std::cmp::Ordering::Equal => current,
                };
                changed |= next != current;
                *visual_y = (next != target).then_some(next);
            }
        }
        changed
    }

    fn rolling_target_y(&self, site: usize, offset: usize) -> usize {
        self.surface
            .grid_height_dots
            .saturating_sub(self.columns[site].len().saturating_add(offset).saturating_add(1))
    }

    pub(super) fn seed_rolling_grain(
        &mut self,
        site: usize,
        category_id: CategoryId,
        direction: ToppleDirection,
    ) {
        let visual_y = self.rolling_target_y(site, self.rolling_depth_at(site));
        self.seed_rolling_grain_at_y(site, category_id, direction, visual_y);
    }

    pub(super) fn seed_rolling_grain_at_y(
        &mut self,
        site: usize,
        category_id: CategoryId,
        direction: ToppleDirection,
        visual_y: usize,
    ) {
        let flat_coast_remaining = self.flat_coast_budget(site);
        self.rolling_grains.push_back(RollingGrain {
            site,
            visual_y,
            category_id,
            direction,
            flat_coast_remaining,
        });
        self.momentum_seeds = self.momentum_seeds.saturating_add(1);
        self.momentum_event_seeds = self.momentum_event_seeds.saturating_add(1);
        self.record_rolling_peak();
    }

    pub(super) fn push_recruited_rolling_grain_at_y(
        &mut self,
        site: usize,
        category_id: CategoryId,
        direction: ToppleDirection,
        visual_y: usize,
    ) {
        let flat_coast_remaining = self.flat_coast_budget(site);
        self.rolling_grains.push_back(RollingGrain {
            site,
            visual_y,
            category_id,
            direction,
            flat_coast_remaining,
        });
        self.record_rolling_peak();
    }

    pub(crate) fn rolling_visual_motion_active(&self) -> bool {
        if self.flowviz_conservative {
            return !self.flowviz_parcels.is_empty();
        }
        if self.flowviz_enabled {
            return !self.flowviz_tracers.is_empty();
        }
        self.rolling_visual_in_transit_count() > 0 || self.settled_visual_in_transit_count() > 0
    }

    pub(crate) fn rolling_visual_in_transit_count(&self) -> usize {
        if self.flowviz_conservative {
            return self.flowviz_parcel_mass();
        }
        if self.flowviz_enabled {
            return self.flowviz_tracers.len();
        }
        if self.rolling_grains.is_empty() {
            return 0;
        }
        let mut offsets = vec![0usize; self.columns.len()];
        self.rolling_grains
            .iter()
            .filter(|rolling| {
                let offset = offsets[rolling.site];
                offsets[rolling.site] = offset.saturating_add(1);
                rolling.visual_y != self.rolling_target_y(rolling.site, offset)
            })
            .count()
    }

    pub(crate) fn total_visual_in_transit_count(&self) -> usize {
        if self.flowviz_conservative {
            return self.flowviz_parcel_mass();
        }
        if self.flowviz_enabled {
            return self.flowviz_tracers.len();
        }
        self.rolling_visual_in_transit_count()
            .saturating_add(self.settled_visual_in_transit_count())
    }

    pub(crate) fn advance_rolling_visual_motion(&mut self) -> bool {
        if self.flowviz_enabled {
            return self.advance_flowviz_tracers();
        }
        let grid_height = self.surface.grid_height_dots;
        let settled_heights = self.columns.iter().map(Vec::len).collect::<Vec<_>>();
        let mut offsets = vec![0usize; self.columns.len()];
        let mut changed = false;
        for rolling in &mut self.rolling_grains {
            let offset = offsets[rolling.site];
            offsets[rolling.site] = offset.saturating_add(1);
            let target = grid_height.saturating_sub(
                settled_heights[rolling.site]
                    .saturating_add(offset)
                    .saturating_add(1),
            );
            let next = match rolling.visual_y.cmp(&target) {
                std::cmp::Ordering::Less => rolling.visual_y.saturating_add(1),
                std::cmp::Ordering::Greater => rolling.visual_y.saturating_sub(1),
                std::cmp::Ordering::Equal => rolling.visual_y,
            };
            changed |= next != rolling.visual_y;
            rolling.visual_y = next;
        }
        changed | self.advance_settled_visual_motion()
    }

    fn record_rolling_peak(&mut self) {
        let active = self.rolling_grains.len();
        self.momentum_peak_active = self.momentum_peak_active.max(active);
        self.momentum_event_peak_active = self.momentum_event_peak_active.max(active);
    }

    fn next_visible_site(&self, rolling: RollingGrain) -> Option<usize> {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        rolling
            .direction
            .step(rolling.site)
            .filter(|site| *site >= visible_start && *site < visible_end)
    }

    fn visible_site(&self, site: usize) -> bool {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        site >= visible_start && site < visible_end
    }

    fn settle_rolling_grain(&mut self, rolling: RollingGrain) {
        self.push_settled_grain_at_visual_y(
            rolling.site,
            rolling.category_id,
            Some(rolling.visual_y),
        );
        self.record_flowviz_settlement(rolling.site, rolling.category_id);
        self.momentum_settles = self.momentum_settles.saturating_add(1);
        self.enqueue_neighborhood(rolling.site);
    }

    fn move_rolling_grain(&mut self, rolling: &mut RollingGrain, next_site: usize) {
        let source = rolling.site;
        let source_y = self
            .top_grain_y(source)
            .unwrap_or(rolling.visual_y);
        self.record_flowviz_transfer(source, next_site, rolling.category_id, source_y);
        rolling.site = next_site;
        self.momentum_hops = self.momentum_hops.saturating_add(1);
        self.momentum_event_hops = self.momentum_event_hops.saturating_add(1);
        self.avalanche_moves = self.avalanche_moves.saturating_add(1);
    }

    fn record_front_recruit_move(&mut self, erosion: bool) {
        self.momentum_hops = self.momentum_hops.saturating_add(1);
        self.momentum_event_hops = self.momentum_event_hops.saturating_add(1);
        self.avalanche_moves = self.avalanche_moves.saturating_add(1);
        if erosion {
            self.front_erosions = self.front_erosions.saturating_add(1);
            self.front_event_erosions = self.front_event_erosions.saturating_add(1);
        } else {
            self.front_support_recruits = self.front_support_recruits.saturating_add(1);
            self.front_event_support_recruits =
                self.front_event_support_recruits.saturating_add(1);
        }
    }

    /// BCRE-like erosion: moving material crossing a settled downhill relief of
    /// at least two can entrain one additional static grain from the source.
    /// At most one settled grain per source site is entrained during a single
    /// rolling update, preventing a numerical instantaneous column deletion.
    fn front_erode_after_hop(
        &mut self,
        source: usize,
        destination: usize,
        direction: ToppleDirection,
        eroded_this_tick: &mut [bool],
    ) -> bool {
        if !self.front_enabled
            || source >= eroded_this_tick.len()
            || eroded_this_tick[source]
            || self.columns[source].is_empty()
        {
            return false;
        }
        let relief = self.columns[source]
            .len()
            .saturating_sub(self.columns[destination].len());
        if relief < FRONT_EROSION_RELIEF {
            return false;
        }

        let (category_id, visual_y) = self
            .pop_settled_grain(source)
            .expect("front erosion source was checked non-empty");
        self.record_flowviz_mobile_entry(source, destination, category_id, visual_y);
        eroded_this_tick[source] = true;
        self.push_recruited_rolling_grain_at_y(destination, category_id, direction, visual_y);
        self.record_front_recruit_move(true);
        self.enqueue_neighborhood(source);
        self.enqueue_neighborhood(destination);
        let _ = self.front_recruit_support_after_loss(source, direction);
        true
    }

    /// Daerr-Douady-like uphill propagation by loss of support. Once a moving
    /// front exists, removing settled mass can expose the immediately uphill
    /// column. If that static column now exceeds the higher *start* relief, one
    /// top grain joins the moving layer. The moving phase can then continue on
    /// gentler terrain, implementing start/stop hysteresis without lowering the
    /// ordinary Oslo thresholds globally.
    pub(super) fn front_recruit_support_after_loss(
        &mut self,
        support_site: usize,
        direction: ToppleDirection,
    ) -> bool {
        if !self.front_enabled || self.rolling_grains.is_empty() {
            return false;
        }
        let Some(uphill) = direction.uphill_step(support_site) else {
            return false;
        };
        if !self.visible_site(uphill) || self.columns[uphill].is_empty() {
            return false;
        }
        let relief = self.columns[uphill]
            .len()
            .saturating_sub(self.columns[support_site].len());
        if relief < FRONT_SUPPORT_LOSS_RELIEF {
            return false;
        }

        let (category_id, visual_y) = self
            .pop_settled_grain(uphill)
            .expect("support-loss source was checked non-empty");
        self.record_flowviz_mobile_entry(uphill, support_site, category_id, visual_y);
        self.push_recruited_rolling_grain_at_y(
            support_site,
            category_id,
            direction,
            visual_y,
        );
        self.record_front_recruit_move(false);
        self.enqueue_neighborhood(uphill);
        self.enqueue_neighborhood(support_site);
        true
    }

    /// Advance every currently mobile grain by at most one lattice site.
    ///
    /// Momentum-v1 remains available unchanged as a control. In the front model
    /// an already-moving layer additionally exchanges mass with the static bed:
    /// steep passages entrain one grain and sufficient loss of support can make
    /// an uphill neighbour join the flow. Flat runout is shorter than v1, but a
    /// locally thicker rolling layer receives a small bounded coast extension.
    pub(super) fn advance_rolling_grains(&mut self) -> bool {
        if !self.momentum_enabled || self.rolling_grains.is_empty() {
            return false;
        }

        let active_at_start = self.rolling_grains.len();
        let mut eroded_this_tick = vec![false; self.columns.len()];
        let mut changed = false;
        for _ in 0..active_at_start {
            let Some(mut rolling) = self.rolling_grains.pop_front() else {
                break;
            };
            let source = rolling.site;
            let direction = rolling.direction;
            let Some(next_site) = self.next_visible_site(rolling) else {
                self.settle_rolling_grain(rolling);
                changed = true;
                continue;
            };

            let current_height = self.columns[source].len();
            let next_height = self.columns[next_site].len();
            if next_height < current_height {
                rolling.flat_coast_remaining = self.flat_coast_budget(next_site);
                self.move_rolling_grain(&mut rolling, next_site);
                self.rolling_grains.push_back(rolling);
                changed = true;
                changed |= self.front_erode_after_hop(
                    source,
                    next_site,
                    direction,
                    &mut eroded_this_tick,
                );
            } else if next_height == current_height && rolling.flat_coast_remaining > 0 {
                rolling.flat_coast_remaining -= 1;
                self.move_rolling_grain(&mut rolling, next_site);
                self.rolling_grains.push_back(rolling);
                changed = true;
            } else {
                self.settle_rolling_grain(rolling);
                changed = true;
            }
        }
        self.record_rolling_peak();
        changed
    }

    pub(super) fn finalize_momentum_event(&mut self) {
        if self.momentum_event_seeds > 0 || self.momentum_event_hops > 0 {
            self.momentum_last_seeds = self.momentum_event_seeds;
            self.momentum_last_hops = self.momentum_event_hops;
            self.momentum_last_peak_active = self.momentum_event_peak_active;
            self.front_last_erosions = self.front_event_erosions;
            self.front_last_support_recruits = self.front_event_support_recruits;
        }
        self.momentum_event_seeds = 0;
        self.momentum_event_hops = 0;
        self.momentum_event_peak_active = 0;
        self.front_event_erosions = 0;
        self.front_event_support_recruits = 0;
        self.finalize_fluid_event();
    }
}
