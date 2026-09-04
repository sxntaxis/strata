use super::*;

impl OsloSandboxEngine {
    /// Seed a bounded spatial fluidity field at a severe Oslo failure.
    ///
    /// This is a discrete product analogue of the order parameter used by
    /// Aranson-Tsimring: zero means static, larger values mean a locally
    /// fluidized contact network. The field is transient and debug-only.
    pub(super) fn seed_fluidization_failure(&mut self, source: usize, destination: usize) {
        if !self.fluid_enabled {
            return;
        }
        for site in [source, destination] {
            if site >= self.fluidity.len() || !self.visible_site(site) {
                continue;
            }
            let was_static = self.fluidity[site] == 0;
            self.fluidity[site] = self.fluidity[site].max(FLUIDITY_SEED);
            if was_static {
                self.fluid_activations = self.fluid_activations.saturating_add(1);
                self.fluid_event_activations = self.fluid_event_activations.saturating_add(1);
            }
        }
        self.record_fluid_peak();
    }

    fn record_fluid_peak(&mut self) {
        self.fluid_peak_active_sites = self
            .fluid_peak_active_sites
            .max(self.fluidity.iter().filter(|value| **value > 0).count());
    }

    fn fluid_downhill(&self, site: usize) -> Option<(ToppleDirection, usize, usize)> {
        if !self.visible_site(site) || self.columns.get(site).is_none_or(Vec::is_empty) {
            return None;
        }
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let height = self.columns[site].len();
        let left = site
            .checked_sub(1)
            .filter(|left| *left >= visible_start)
            .map(|left| (ToppleDirection::Left, left, height.saturating_sub(self.columns[left].len())));
        let right = site
            .checked_add(1)
            .filter(|right| *right < visible_end)
            .map(|right| (ToppleDirection::Right, right, height.saturating_sub(self.columns[right].len())));

        match (left, right) {
            (None, None) => None,
            (Some(left), None) => (left.2 > 0).then_some(left),
            (None, Some(right)) => (right.2 > 0).then_some(right),
            (Some(left), Some(right)) => {
                if left.2 == 0 && right.2 == 0 {
                    None
                } else if left.2 > right.2 {
                    Some(left)
                } else if right.2 > left.2 {
                    Some(right)
                } else if (site + self.frame_count).is_multiple_of(2) {
                    Some(left)
                } else {
                    Some(right)
                }
            }
        }
    }

    fn local_downhill_relief(&self, site: usize) -> usize {
        self.fluid_downhill(site).map_or(0, |(_, _, relief)| relief)
    }

    fn record_fluid_release_move(&mut self) {
        self.fluid_releases = self.fluid_releases.saturating_add(1);
        self.fluid_event_releases = self.fluid_event_releases.saturating_add(1);
        self.momentum_hops = self.momentum_hops.saturating_add(1);
        self.momentum_event_hops = self.momentum_event_hops.saturating_add(1);
        self.avalanche_moves = self.avalanche_moves.saturating_add(1);
    }

    /// Advance the partial-fluidization order parameter by one physical tick.
    ///
    /// The static-start threshold is intentionally higher than the dynamic-stop
    /// threshold. Fluidity diffuses one site per tick, while actual grain mass
    /// always travels downhill through the existing rolling-grain substrate.
    /// This gives a region-level flowing/static transition without replacing
    /// Oslo's ordinary metastable preparation law.
    pub(super) fn advance_fluidization_field(&mut self) -> bool {
        if !self.fluid_enabled || self.fluidity.iter().all(|value| *value == 0) {
            return false;
        }

        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let previous = self.fluidity.clone();
        let mut next = previous.clone();
        let mut changed = false;
        let mut spread_activations = 0usize;
        let mut rolling_depths = vec![0usize; self.columns.len()];
        for rolling in &self.rolling_grains {
            rolling_depths[rolling.site] = rolling_depths[rolling.site].saturating_add(1);
        }

        for site in visible_start..visible_end {
            let relief = self.local_downhill_relief(site);
            let current = previous[site];
            let rolling_here = rolling_depths[site] > 0;

            let mut value = if current == 0 {
                0
            } else if relief >= FLUIDITY_DYNAMIC_STOP_RELIEF || rolling_here {
                current.saturating_sub(1)
            } else {
                current.saturating_sub(2)
            };

            let neighbour_fluidity = [site.checked_sub(1), site.checked_add(1)]
                .into_iter()
                .flatten()
                .filter(|neighbour| *neighbour >= visible_start && *neighbour < visible_end)
                .map(|neighbour| previous[neighbour])
                .max()
                .unwrap_or(0);
            if neighbour_fluidity >= FLUIDITY_SPREAD_MIN
                && (relief >= FLUIDITY_DYNAMIC_STOP_RELIEF || rolling_here)
            {
                value = value.max(neighbour_fluidity.saturating_sub(1));
            }

            // A severe local cliff can sustain an already-existing fluid region,
            // but cannot nucleate one by itself; nucleation remains tied to the
            // actual Oslo failure hook so ordinary stable terrain stays exact.
            if current > 0 && relief >= FLUIDITY_STATIC_START_RELIEF {
                value = value.max(FLUIDITY_MAX.saturating_sub(1));
            }

            value = value.min(FLUIDITY_MAX);
            if current == 0 && value > 0 {
                spread_activations = spread_activations.saturating_add(1);
            }
            changed |= value != current;
            next[site] = value;
        }

        for (site, value) in next.iter_mut().enumerate() {
            if site < visible_start || site >= visible_end {
                *value = 0;
            }
        }
        self.fluidity = next;
        self.fluid_activations = self.fluid_activations.saturating_add(spread_activations);
        self.fluid_event_activations = self
            .fluid_event_activations
            .saturating_add(spread_activations);

        // Exchange at most one settled grain per fluidized site per tick. Mass
        // becomes an explicit rolling grain and therefore remains category- and
        // dot-preserving rather than disappearing into a continuum field.
        let release_sites = (visible_start..visible_end)
            .filter(|site| self.fluidity[*site] >= FLUIDITY_RELEASE_MIN)
            .filter_map(|site| {
                let (direction, destination, relief) = self.fluid_downhill(site)?;
                (relief >= FLUIDITY_RELEASE_RELIEF).then_some((site, direction, destination))
            })
            .collect::<Vec<_>>();

        for (site, direction, destination) in release_sites {
            let Some(visual_y) = self.top_grain_y(site) else {
                continue;
            };
            let Some(category_id) = self.columns[site].pop() else {
                continue;
            };
            self.push_recruited_rolling_grain_at_y(
                destination,
                category_id,
                direction,
                visual_y,
            );
            self.record_fluid_release_move();
            self.enqueue_neighborhood(site);
            self.enqueue_neighborhood(destination);
            changed = true;
        }

        self.record_fluid_peak();
        changed
    }

    pub(super) fn finalize_fluid_event(&mut self) {
        if self.fluid_event_activations > 0 || self.fluid_event_releases > 0 {
            self.fluid_last_activations = self.fluid_event_activations;
            self.fluid_last_releases = self.fluid_event_releases;
        }
        self.fluid_event_activations = 0;
        self.fluid_event_releases = 0;
    }
}
