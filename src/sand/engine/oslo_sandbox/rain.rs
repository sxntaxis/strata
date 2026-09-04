use crate::constants::SAND_ENGINE;

use super::*;

impl OsloSandboxEngine {
    pub(super) fn visible_lattice_bounds(&self) -> (usize, usize) {
        let viewport_width =
            (self.surface.cell_width as usize).saturating_mul(SAND_ENGINE.dot_width);
        let visible_width = viewport_width.min(self.columns.len());
        let start = self.columns.len().saturating_sub(visible_width) / 2;
        (start, start + visible_width)
    }

    pub(super) fn visible_vertical_bounds(&self) -> (usize, usize) {
        let canonical_height = self.surface.grid_height_dots;
        let viewport_height =
            (self.surface.cell_height as usize).saturating_mul(SAND_ENGINE.dot_height);
        let visible_height = viewport_height.min(canonical_height);
        (
            canonical_height.saturating_sub(visible_height),
            canonical_height,
        )
    }

    pub(super) fn clamp_focus_to_visible_corridor(&mut self) {
        let (start, end) = self.golden_focus_bounds();
        if start >= end {
            self.rain_focus_site = None;
            self.rain_focus_target_site = None;
            self.rain_focus_move_counter = 0;
            return;
        }
        self.rain_focus_site = self.rain_focus_site.map(|site| site.clamp(start, end - 1));
        self.rain_focus_target_site = self
            .rain_focus_target_site
            .map(|site| site.clamp(start, end - 1));
    }

    fn golden_focus_bounds_for_width(width: usize) -> (usize, usize) {
        debug_assert!(width > 0);
        if width <= 2 {
            return (0, width);
        }

        let edge_fraction = ((1.0 - 1.0 / GOLDEN_RATIO) / 2.0) / GOLDEN_RATIO;
        let padding = ((width as f64) * edge_fraction)
            .round()
            .min(((width - 1) / 2) as f64) as usize;
        (padding, width - padding)
    }

    fn golden_focus_bounds(&self) -> (usize, usize) {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let visible_width = visible_end.saturating_sub(visible_start);
        if visible_width == 0 {
            return (visible_start, visible_start);
        }
        let (local_start, local_end) = Self::golden_focus_bounds_for_width(visible_width);
        (visible_start + local_start, visible_start + local_end)
    }

    pub(super) fn drive_site_accepts(&self, site: usize) -> bool {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        if site < visible_start || site >= visible_end {
            return false;
        }
        let (visible_top, visible_bottom) = self.visible_vertical_bounds();
        let visible_height = visible_bottom.saturating_sub(visible_top);
        if visible_height == 0 {
            return false;
        }
        let at_full_canonical_height = visible_height == self.surface.grid_height_dots;
        if at_full_canonical_height
            && matches!(
                self.boundary,
                OsloBoundaryMode::ZeroOutside | OsloBoundaryMode::CanonicalWallOverflow
            )
        {
            // At the full canonical height these two modes can receive a grain
            // above the wall/top and let ordinary relaxation decide whether it
            // remains supported or eventually leaves through an outward topple.
            return true;
        }
        self.columns[site].len() < visible_height
    }

    pub(super) fn nearest_accepting_site(&mut self, target: usize) -> Option<usize> {
        let (start, end) = self.visible_lattice_bounds();
        let mut best = None;
        let mut best_distance = usize::MAX;
        for site in start..end {
            if !self.drive_site_accepts(site) {
                continue;
            }
            let distance = site.abs_diff(target);
            if distance < best_distance || (distance == best_distance && self.rain_random_bool()) {
                best = Some(site);
                best_distance = distance;
            }
        }
        best
    }

    pub(super) fn choose_drive_site(&mut self) -> Option<usize> {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let (visible_top, visible_bottom) = self.visible_vertical_bounds();
        if visible_start >= visible_end || visible_top >= visible_bottom {
            return None;
        }
        let focus = self.advance_rain_focus();
        let sampled_target = self.sample_rain_target(focus);
        let target = self.nearest_accepting_site(sampled_target)?;
        let (start, end) = self.golden_focus_bounds();
        if target < start {
            self.rain_left_padding_targets = self.rain_left_padding_targets.saturating_add(1);
        } else if target >= end {
            self.rain_right_padding_targets = self.rain_right_padding_targets.saturating_add(1);
        } else {
            self.rain_corridor_targets = self.rain_corridor_targets.saturating_add(1);
        }
        Some(target)
    }

    fn advance_rain_focus(&mut self) -> usize {
        let (start, end) = self.golden_focus_bounds();
        let focus_width = end - start;
        debug_assert!(focus_width > 0);

        let was_initialized = self.rain_focus_site.is_some();
        let mut focus = self.rain_focus_site.map_or_else(
            || start + self.rain_random_index(focus_width),
            |focus| focus.clamp(start, end - 1),
        );

        if !was_initialized {
            self.rain_focus_target_site = Some(self.choose_focus_waypoint(focus, start, end));
            self.rain_focus_move_counter = 0;
            self.rain_focus_site = Some(focus);
            return focus;
        }

        let mut target = self
            .rain_focus_target_site
            .filter(|target| *target >= start && *target < end)
            .unwrap_or_else(|| self.choose_focus_waypoint(focus, start, end));
        if target == focus && focus_width > 1 {
            target = self.choose_focus_waypoint(focus, start, end);
        }
        self.rain_focus_target_site = Some(target);

        let traversable_steps = focus_width.saturating_sub(1).max(1);
        let ingresses_per_step = RAIN_FOCUS_EDGE_TO_EDGE_INGRESSES
            .div_ceil(traversable_steps)
            .max(1);
        self.rain_focus_move_counter = self.rain_focus_move_counter.saturating_add(1);
        if self.rain_focus_move_counter >= ingresses_per_step {
            self.rain_focus_move_counter = 0;
            focus = match focus.cmp(&target) {
                std::cmp::Ordering::Less => focus + 1,
                std::cmp::Ordering::Greater => focus - 1,
                std::cmp::Ordering::Equal => focus,
            };
            focus = focus.clamp(start, end - 1);
            if focus == target && focus_width > 1 {
                self.rain_focus_target_site = Some(self.choose_focus_waypoint(focus, start, end));
            }
        }

        self.rain_focus_site = Some(focus);
        focus
    }

    fn choose_focus_waypoint(&mut self, focus: usize, start: usize, end: usize) -> usize {
        let focus_width = end - start;
        if focus_width <= 1 {
            return start;
        }

        // Pick the next target from the opposite outer sixth. Keeping the target
        // band narrow guarantees that successive left/right waypoints are separated
        // by at least about two thirds of the corridor, while preserving stochastic
        // placement within each edge band. This makes the daily wander contract a
        // geometric property rather than a lucky outcome of random waypoint draws.
        let sixth = (focus_width / 6).max(1);
        let midpoint = start + focus_width / 2;
        let choose_right = if focus < midpoint {
            true
        } else if focus > midpoint {
            false
        } else {
            self.rain_random_bool()
        };
        let (target_start, target_end) = if choose_right {
            (end.saturating_sub(sixth), end)
        } else {
            (start, (start + sixth).min(end))
        };
        target_start + self.rain_random_index((target_end - target_start).max(1))
    }

    fn sample_rain_target(&mut self, focus: usize) -> usize {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let width = visible_end.saturating_sub(visible_start);
        debug_assert!(width > 0);

        // Most ingress is genuinely uniform over the entire visible physical width.
        // The focus affects only the minority bias stream, and that stream is sampled
        // wholly inside the golden corridor. Canonical columns hidden by a temporary
        // terminal shrink retain custody but do not attract new rain.
        let full_width = visible_start + self.rain_random_index(width);
        if width == 1 || self.rain_random_index(RAIN_FOCUS_BIAS_ONE_IN) != 0 {
            return full_width;
        }

        self.sample_focus_biased_target(focus)
    }

    fn sample_focus_biased_target(&mut self, focus: usize) -> usize {
        let (start, end) = self.golden_focus_bounds();
        let focus_width = end - start;
        let first = start + self.rain_random_index(focus_width);
        let second = start + self.rain_random_index(focus_width);
        match first.abs_diff(focus).cmp(&second.abs_diff(focus)) {
            std::cmp::Ordering::Less => first,
            std::cmp::Ordering::Greater => second,
            std::cmp::Ordering::Equal => {
                if self.rain_random_bool() {
                    second
                } else {
                    first
                }
            }
        }
    }
}
