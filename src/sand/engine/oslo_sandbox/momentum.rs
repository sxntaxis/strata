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

    fn rolling_depth_at(&self, site: usize) -> usize {
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

    pub(super) fn seed_rolling_grain(
        &mut self,
        site: usize,
        category_id: CategoryId,
        direction: ToppleDirection,
    ) {
        let flat_coast_remaining = self.flat_coast_budget(site);
        self.rolling_grains.push_back(RollingGrain {
            site,
            category_id,
            direction,
            flat_coast_remaining,
        });
        self.momentum_seeds = self.momentum_seeds.saturating_add(1);
        self.momentum_event_seeds = self.momentum_event_seeds.saturating_add(1);
        self.record_rolling_peak();
    }

    fn push_recruited_rolling_grain(
        &mut self,
        site: usize,
        category_id: CategoryId,
        direction: ToppleDirection,
    ) {
        let flat_coast_remaining = self.flat_coast_budget(site);
        self.rolling_grains.push_back(RollingGrain {
            site,
            category_id,
            direction,
            flat_coast_remaining,
        });
        self.record_rolling_peak();
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
        self.columns[rolling.site].push(rolling.category_id);
        self.momentum_settles = self.momentum_settles.saturating_add(1);
        self.enqueue_neighborhood(rolling.site);
    }

    fn move_rolling_grain(&mut self, rolling: &mut RollingGrain, next_site: usize) {
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

        let category_id = self.columns[source]
            .pop()
            .expect("front erosion source was checked non-empty");
        eroded_this_tick[source] = true;
        self.push_recruited_rolling_grain(destination, category_id, direction);
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

        let category_id = self.columns[uphill]
            .pop()
            .expect("support-loss source was checked non-empty");
        self.push_recruited_rolling_grain(support_site, category_id, direction);
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
    }
}
