use super::*;

impl ToppleDirection {
    fn step(self, site: usize) -> Option<usize> {
        match self {
            Self::Left => site.checked_sub(1),
            Self::Right => site.checked_add(1),
        }
    }
}

impl OsloSandboxEngine {
    #[cfg(test)]
    pub(super) fn momentum_trigger_relief(&self) -> usize {
        MOMENTUM_TRIGGER_RELIEF
    }

    pub(super) fn should_seed_momentum(&self, relief: usize) -> bool {
        self.momentum_enabled && relief >= MOMENTUM_TRIGGER_RELIEF
    }

    pub(super) fn seed_rolling_grain(
        &mut self,
        site: usize,
        category_id: CategoryId,
        direction: ToppleDirection,
    ) {
        self.rolling_grains.push_back(RollingGrain {
            site,
            category_id,
            direction,
            flat_coast_remaining: MOMENTUM_FLAT_COAST_STEPS,
        });
        self.momentum_seeds = self.momentum_seeds.saturating_add(1);
        self.momentum_event_seeds = self.momentum_event_seeds.saturating_add(1);
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

    /// Advance every currently mobile grain by at most one lattice site.
    ///
    /// This is intentionally a distinct moving phase rather than a larger Oslo
    /// toppling budget. A steep static failure can therefore release many grains
    /// that remain concurrently mobile while the static heightfield continues to
    /// relax behind them.
    pub(super) fn advance_rolling_grains(&mut self) -> bool {
        if !self.momentum_enabled || self.rolling_grains.is_empty() {
            return false;
        }

        let active_at_start = self.rolling_grains.len();
        let mut changed = false;
        for _ in 0..active_at_start {
            let Some(mut rolling) = self.rolling_grains.pop_front() else {
                break;
            };
            let Some(next_site) = self.next_visible_site(rolling) else {
                self.settle_rolling_grain(rolling);
                changed = true;
                continue;
            };

            let current_height = self.columns[rolling.site].len();
            let next_height = self.columns[next_site].len();
            if next_height < current_height {
                rolling.flat_coast_remaining = MOMENTUM_FLAT_COAST_STEPS;
                self.move_rolling_grain(&mut rolling, next_site);
                self.rolling_grains.push_back(rolling);
                changed = true;
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
        }
        self.momentum_event_seeds = 0;
        self.momentum_event_hops = 0;
        self.momentum_event_peak_active = 0;
    }
}
