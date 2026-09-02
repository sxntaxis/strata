use super::*;

impl OsloSandboxEngine {
    fn next_threshold_random_u64(&mut self) -> u64 {
        let mut x = self.threshold_rng_state;
        if x == 0 {
            x = OSLO_THRESHOLD_RNG_XOR;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.threshold_rng_state = x;
        x
    }

    pub(super) fn next_rain_random_u64(&mut self) -> u64 {
        let mut x = self.rain_rng_state;
        if x == 0 {
            x = OSLO_RAIN_RNG_XOR;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rain_rng_state = x;
        x
    }

    pub(super) fn rain_random_index(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next_rain_random_u64() as usize) % upper
    }

    pub(super) fn rain_random_bool(&mut self) -> bool {
        self.next_rain_random_u64() & 1 == 0
    }

    fn next_relax_random_u64(&mut self) -> u64 {
        let mut x = self.relax_rng_state;
        if x == 0 {
            x = OSLO_RELAX_RNG_XOR;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.relax_rng_state = x;
        x
    }

    fn relax_random_bool(&mut self) -> bool {
        self.next_relax_random_u64() & 1 == 0
    }

    fn sample_threshold(&mut self) -> u8 {
        if self.next_threshold_random_u64() & 1 == 0 {
            OSLO_THRESHOLD_LOW
        } else {
            OSLO_THRESHOLD_HIGH
        }
    }

    pub(super) fn randomize_all_thresholds(&mut self) {
        for site in 0..self.critical_slopes.len() {
            self.critical_slopes[site] = self.sample_threshold();
        }
    }

    fn downhill_reliefs(&self, site: usize) -> Option<(usize, usize)> {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        if site < visible_start || site >= visible_end {
            return None;
        }
        let height = self.columns[site].len();
        let outside_height = match self.boundary {
            OsloBoundaryMode::ZeroOutside => 0,
            OsloBoundaryMode::ClosedBox => height,
            OsloBoundaryMode::CanonicalWallOverflow => self.canonical_wall_height(),
        };
        let left_height = if site > visible_start {
            self.columns[site - 1].len()
        } else {
            outside_height
        };
        let right_height = if site + 1 < visible_end {
            self.columns[site + 1].len()
        } else {
            outside_height
        };
        Some((
            height.saturating_sub(left_height),
            height.saturating_sub(right_height),
        ))
    }

    fn unstable_direction(&mut self, site: usize) -> Option<ToppleDirection> {
        let (left_relief, right_relief) = self.downhill_reliefs(site)?;
        let threshold = usize::from(self.critical_slopes[site]);
        let left_unstable = left_relief > threshold;
        let right_unstable = right_relief > threshold;

        match (left_unstable, right_unstable) {
            (false, false) => None,
            (true, false) => Some(ToppleDirection::Left),
            (false, true) => Some(ToppleDirection::Right),
            (true, true) => match left_relief.cmp(&right_relief) {
                std::cmp::Ordering::Greater => Some(ToppleDirection::Left),
                std::cmp::Ordering::Less => Some(ToppleDirection::Right),
                std::cmp::Ordering::Equal => Some(if self.relax_random_bool() {
                    ToppleDirection::Left
                } else {
                    ToppleDirection::Right
                }),
            },
        }
    }

    fn enqueue_site(&mut self, site: usize) {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        if site < visible_start
            || site >= visible_end
            || site >= self.queued_sites.len()
            || self.queued_sites[site]
        {
            return;
        }
        self.queued_sites[site] = true;
        self.active_sites.push_back(site);
    }

    pub(super) fn enqueue_neighborhood(&mut self, site: usize) {
        if let Some(left) = site.checked_sub(1) {
            self.enqueue_site(left);
        }
        self.enqueue_site(site);
        if let Some(right) = site.checked_add(1) {
            self.enqueue_site(right);
        }
    }

    fn enqueue_after_topple(&mut self, source: usize, destination: Option<usize>) {
        self.enqueue_neighborhood(source);
        if let Some(destination) = destination {
            self.enqueue_neighborhood(destination);
        }
    }

    pub(super) fn topple_one_active_site(&mut self) -> bool {
        let queued_at_start = self.active_sites.len();
        for _ in 0..queued_at_start {
            let Some(site) = self.active_sites.pop_front() else {
                break;
            };
            self.queued_sites[site] = false;
            if self.topple_site_if_unstable(site) {
                return true;
            }
        }

        if self.active_sites.is_empty() && self.avalanche_moves > 0 {
            self.avalanche_last_moves = self.avalanche_moves;
            self.avalanche_peak_moves = self.avalanche_peak_moves.max(self.avalanche_moves);
            self.avalanche_completed = self.avalanche_completed.saturating_add(1);
            self.recent_avalanches.push_back(self.avalanche_moves);
            if self.recent_avalanches.len() > RECENT_AVALANCHE_WINDOW {
                self.recent_avalanches.pop_front();
            }
            self.avalanche_moves = 0;
        }
        false
    }

    pub(super) fn topple_site_if_unstable(&mut self, site: usize) -> bool {
        let Some(direction) = self.unstable_direction(site) else {
            return false;
        };

        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let destination = match direction {
            ToppleDirection::Left => (site > visible_start).then_some(site - 1),
            ToppleDirection::Right => (site + 1 < visible_end).then_some(site + 1),
        };

        if destination.is_none() && self.boundary == OsloBoundaryMode::ClosedBox {
            // ClosedBox contributes zero outward relief, so this is defensive only.
            return false;
        }

        let category_id = self.columns[site]
            .pop()
            .expect("over-critical Oslo site contains a grain");
        if let Some(destination) = destination {
            self.columns[destination].push(category_id);
        } else {
            self.discharged = self.discharged.saturating_add(1);
        }
        self.critical_slopes[site] = self.sample_threshold();
        self.enqueue_after_topple(site, destination);
        self.avalanche_moves = self.avalanche_moves.saturating_add(1);
        true
    }

    fn append_pending_run(&mut self, category_id: CategoryId, count: usize) {
        if count == 0 {
            return;
        }
        if let Some(last) = self.pending_runs.back_mut()
            && last.category_id == category_id.0
        {
            last.count = last.count.saturating_add(count);
        } else {
            self.pending_runs.push_back(PendingGrainRun {
                category_id: category_id.0,
                count,
            });
        }
    }

    pub(super) fn launch_one_pending(&mut self) -> bool {
        let Some(run) = self.pending_runs.front().cloned() else {
            return false;
        };
        let Some(x) = self.choose_drive_site() else {
            return false;
        };
        self.falling_drives.push_back(FallingDrive {
            x,
            y: self.visible_vertical_bounds().0,
            category_id: CategoryId::new(run.category_id),
        });
        if let Some(front) = self.pending_runs.front_mut() {
            front.count -= 1;
        }
        if self.pending_runs.front().is_some_and(|front| front.count == 0) {
            self.pending_runs.pop_front();
        }
        true
    }

    pub(super) fn advance_falling_drives(&mut self) -> bool {
        if self.columns.is_empty() || self.falling_drives.is_empty() {
            return false;
        }

        let grid_height = self.surface.grid_height_dots;
        let columns = &self.columns;
        let mut changed = false;
        for falling in &mut self.falling_drives {
            let target_height = columns[falling.x].len();
            let landing_y = grid_height.saturating_sub(target_height.saturating_add(1));
            let next_y = if falling.y < landing_y {
                falling.y + 1
            } else {
                landing_y
            };
            changed |= next_y != falling.y;
            falling.y = next_y;
        }
        changed
    }

    pub(super) fn commit_next_drive_if_quiescent(&mut self) -> bool {
        if !self.active_sites.is_empty() || self.columns.is_empty() {
            return false;
        }
        let Some(mut falling) = self.falling_drives.front().copied() else {
            return false;
        };

        // Several visible drives may be in flight at once. An earlier drive can
        // fill the column selected by a later one before that later grain lands.
        // Do not let the FIFO front permanently block the whole ingress stream:
        // re-home it to the nearest currently accepting visible site, preserving
        // category and queue order. This is a harness/runtime robustness rule,
        // not a change to Oslo relaxation.
        if !self.drive_site_accepts(falling.x) {
            let Some(target_x) = self.nearest_accepting_site(falling.x) else {
                return false;
            };
            if let Some(front) = self.falling_drives.front_mut() {
                front.x = target_x;
            }
            falling.x = target_x;
        }

        let target_height = self.columns[falling.x].len();
        let landing_y = self
            .surface
            .grid_height_dots
            .saturating_sub(target_height.saturating_add(1));
        if falling.y < landing_y {
            return false;
        }

        let falling = self
            .falling_drives
            .pop_front()
            .expect("front falling Oslo drive exists");
        self.columns[falling.x].push(falling.category_id);
        self.enqueue_neighborhood(falling.x);
        true
    }

    pub(super) fn sync_surface(&mut self) {
        for row in &mut self.surface.grid {
            row.fill(None);
        }
        for row in &mut self.surface.mobilized {
            row.fill(false);
        }
        self.surface.pending_runs.clear();

        let grid_height = self.surface.grid_height_dots;
        for (site, column) in self.columns.iter().enumerate() {
            let x = site;
            if x >= self.surface.grid_width_dots {
                break;
            }
            for (depth, category_id) in column.iter().copied().enumerate() {
                let Some(y) = grid_height.checked_sub(depth + 1) else {
                    break;
                };
                self.surface.grid[y][x] = Some(category_id);
            }
        }

        for falling in &self.falling_drives {
            let x = falling.x;
            if x < self.surface.grid_width_dots && falling.y < grid_height {
                self.surface.grid[falling.y][x] = Some(falling.category_id);
            }
        }
        self.surface.grain_count = self
            .settled_count()
            .saturating_add(self.falling_drives.len())
            .saturating_add(self.pending_runs.iter().map(|run| run.count).sum::<usize>());
    }
}
