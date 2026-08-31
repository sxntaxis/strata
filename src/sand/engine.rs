use std::collections::{HashMap, HashSet, VecDeque};

use ratatui::{
    prelude::{Line, Span},
    style::{Color, Stylize},
};
use serde::{Deserialize, Serialize};

use crate::{
    constants::SAND_ENGINE,
    domain::{Category, CategoryId, DRIFT_CATEGORY_ID},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandStateGrain {
    pub x: usize,
    pub y: usize,
    pub category_id: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandStateCoordinate {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryReleaseDirection {
    Left,
    Right,
}

impl BoundaryReleaseDirection {
    fn outward_target(self, x: usize) -> Option<usize> {
        match self {
            Self::Left => x.checked_sub(1),
            Self::Right => x.checked_add(1),
        }
    }

    fn inward_source(self, x: usize) -> Option<usize> {
        match self {
            Self::Left => x.checked_add(1),
            Self::Right => x.checked_sub(1),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SandStateBoundaryReleaseFront {
    pub direction: BoundaryReleaseDirection,
    /// Canonical column that was the visible wall when confinement was removed.
    pub wall_x: usize,
    /// Deepest inward canonical column admitted into this release lineage.
    pub front_x: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingGrainRun {
    pub category_id: u64,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandState {
    pub version: u8,
    pub grid_width: usize,
    pub grid_height: usize,
    pub grains: Vec<SandStateGrain>,
    #[serde(default)]
    pub frame_count: usize,
    #[serde(default = "default_sweep_left_to_right")]
    pub sweep_left_to_right: bool,
    #[serde(default = "default_rng_state")]
    pub rng_state: u64,
    /// Canonical dot column for the slowly wandering visible-top rain focus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingress_focus_x: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_grains: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_runs: Vec<PendingGrainRun>,
    /// Legacy v4 regional-avalanche evidence. Current v6 snapshots always emit
    /// this empty; it remains readable only so pre-v5 semantic migration can
    /// reject malformed historical state without reviving regional runtime causality.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_avalanche_columns: Vec<usize>,
    /// Exact transient dynamic state for the grain-causal H4 model. Coordinates
    /// are canonical, row-major sorted, unique, and must reference placed grains.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mobilized_grains: Vec<SandStateCoordinate>,
    /// Exact transient release fronts created only when resize removes a visible
    /// lateral wall. Each front owns one canonical column and a fixed outward
    /// direction until the newly exposed face reaches dynamic repose.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundary_release_fronts: Vec<SandStateBoundaryReleaseFront>,
    /// At most one grain may be in flight because of boundary release itself.
    /// This is distinct from H4 mobility: it preserves exact restart custody
    /// without granting the released grain artificial slip momentum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_release_in_flight: Option<SandStateCoordinate>,
}

impl SandState {
    pub const VERSION: u8 = 6;
    pub const GRAIN_CAUSAL_VERSION: u8 = 5;
    pub const REGIONAL_AVALANCHE_VERSION: u8 = 4;
    pub const ORGANIC_VERSION: u8 = 3;
    pub const COMPRESSED_PENDING_VERSION: u8 = 2;
    pub const LEGACY_VERSION: u8 = 1;
}

pub(crate) fn recolor_state_category_mass(
    state: &mut SandState,
    from_category_id: CategoryId,
    to_category_id: CategoryId,
    count: usize,
) -> usize {
    if count == 0 || from_category_id == to_category_id {
        return 0;
    }

    let from = from_category_id.0;
    let to = to_category_id.0;
    let mut remaining = count;
    let mut recolored = 0usize;

    // `SandEngine::snapshot_state` serializes placed grains in canonical
    // row-major order. Recoloring in that same order is deterministic while
    // leaving every coordinate and all topology untouched.
    for grain in &mut state.grains {
        if remaining == 0 {
            break;
        }
        if grain.category_id == from {
            grain.category_id = to;
            remaining -= 1;
            recolored += 1;
        }
    }

    // Legacy/uncompressed pending grains remain FIFO; only category identity
    // changes.
    for category_id in &mut state.pending_grains {
        if remaining == 0 {
            break;
        }
        if *category_id == from {
            *category_id = to;
            remaining -= 1;
            recolored += 1;
        }
    }

    if remaining == 0 || state.pending_runs.is_empty() {
        return recolored;
    }

    let mut rewritten = Vec::with_capacity(state.pending_runs.len().saturating_add(1));
    let append = |runs: &mut Vec<PendingGrainRun>, category_id: u64, count: usize| {
        if count == 0 {
            return;
        }
        if let Some(last) = runs.last_mut()
            && last.category_id == category_id
        {
            last.count = last.count.saturating_add(count);
        } else {
            runs.push(PendingGrainRun { category_id, count });
        }
    };

    for run in state.pending_runs.drain(..) {
        if remaining == 0 || run.category_id != from {
            append(&mut rewritten, run.category_id, run.count);
            continue;
        }

        let take = run.count.min(remaining);
        append(&mut rewritten, to, take);
        append(&mut rewritten, from, run.count - take);
        remaining -= take;
        recolored += take;
    }
    state.pending_runs = rewritten;
    recolored
}

fn default_sweep_left_to_right() -> bool {
    true
}

fn default_rng_state() -> u64 {
    0x9E37_79B9_7F4A_7C15
}

const INGRESS_FOCUS_MOVE_ONE_IN: usize = 4;
const INGRESS_FOCUS_BIAS_ONE_IN: usize = 4;
const DYNAMIC_REPOSE_RELIEF: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingRun {
    category_id: CategoryId,
    count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ViewportBounds {
    x_start: usize,
    x_end: usize,
    y_start: usize,
    y_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundaryReleaseRouteState {
    Reposed,
    Paused,
    Ready {
        source_y: usize,
        target_height: usize,
    },
}

pub struct SandEngine {
    pub(crate) grid: Vec<Vec<Option<CategoryId>>>,
    pub cell_width: u16,
    pub cell_height: u16,
    pub grid_width_dots: usize,
    pub grid_height_dots: usize,
    frame_count: usize,
    sweep_left_to_right: bool,
    rng_state: u64,
    ingress_focus_x: Option<usize>,
    pending_runs: VecDeque<PendingRun>,
    mobilized: Vec<Vec<bool>>,
    boundary_release_fronts: Vec<SandStateBoundaryReleaseFront>,
    boundary_release_in_flight: Option<SandStateCoordinate>,
    supported_heights: Vec<usize>,
    #[cfg(test)]
    last_avalanche_motion: bool,
    #[cfg(test)]
    last_diagonal_topple: bool,
    #[cfg(test)]
    last_avalanche_span: usize,
    #[cfg(test)]
    last_landing_failure_mobilizations: usize,
    #[cfg(test)]
    last_slip_lineage_mobilizations: usize,
    #[cfg(test)]
    last_support_loss_mobilizations: usize,
    #[cfg(test)]
    last_ordinary_vertical_moves: usize,
    #[cfg(test)]
    last_mobilized_vertical_moves: usize,
    #[cfg(test)]
    last_mobilized_diagonal_moves: usize,
    pub grain_count: usize,
}

impl SandEngine {
    pub fn new(width: u16, height: u16) -> Self {
        let grid_width_dots = width as usize * SAND_ENGINE.dot_width;
        let grid_height_dots = height as usize * SAND_ENGINE.dot_height;

        Self {
            grid: vec![vec![None; grid_width_dots]; grid_height_dots],
            cell_width: width,
            cell_height: height,
            grid_width_dots,
            grid_height_dots,
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: rand::random::<u64>() | 1,
            ingress_focus_x: None,
            pending_runs: VecDeque::new(),
            mobilized: vec![vec![false; grid_width_dots]; grid_height_dots],
            boundary_release_fronts: Vec::new(),
            boundary_release_in_flight: None,
            supported_heights: vec![0; grid_width_dots],
            #[cfg(test)]
            last_avalanche_motion: false,
            #[cfg(test)]
            last_diagonal_topple: false,
            #[cfg(test)]
            last_avalanche_span: 0,
            #[cfg(test)]
            last_landing_failure_mobilizations: 0,
            #[cfg(test)]
            last_slip_lineage_mobilizations: 0,
            #[cfg(test)]
            last_support_loss_mobilizations: 0,
            #[cfg(test)]
            last_ordinary_vertical_moves: 0,
            #[cfg(test)]
            last_mobilized_vertical_moves: 0,
            #[cfg(test)]
            last_mobilized_diagonal_moves: 0,
            grain_count: 0,
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        let previous_bounds = self.viewport_bounds();
        self.cell_width = width;
        self.cell_height = height;
        let (horizontal_offset, vertical_offset) = self.expand_logical_canvas_to_viewport();

        let Some(previous_bounds) = previous_bounds else {
            return;
        };
        let shifted_previous_bounds = ViewportBounds {
            x_start: previous_bounds.x_start.saturating_add(horizontal_offset),
            x_end: previous_bounds.x_end.saturating_add(horizontal_offset),
            y_start: previous_bounds.y_start.saturating_add(vertical_offset),
            y_end: previous_bounds.y_end.saturating_add(vertical_offset),
        };
        if let Some(current_bounds) = self.viewport_bounds() {
            self.release_lateral_confinement(shifted_previous_bounds, current_bounds);
        }
    }

    fn expand_logical_canvas_to_viewport(&mut self) -> (usize, usize) {
        let viewport_width = self.cell_width as usize * SAND_ENGINE.dot_width;
        let viewport_height = self.cell_height as usize * SAND_ENGINE.dot_height;
        let target_width = self.grid_width_dots.max(viewport_width);
        let target_height = self.grid_height_dots.max(viewport_height);
        if target_width == self.grid_width_dots && target_height == self.grid_height_dots {
            return (0, 0);
        }

        let horizontal_offset = target_width.saturating_sub(self.grid_width_dots) / 2;
        let vertical_offset = target_height.saturating_sub(self.grid_height_dots);
        let mut expanded = vec![vec![None; target_width]; target_height];
        for (y, row) in self.grid.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                expanded[y + vertical_offset][x + horizontal_offset] = *cell;
            }
        }
        self.grid = expanded;
        let mut expanded_mobilized = vec![vec![false; target_width]; target_height];
        for (y, row) in self.mobilized.iter().enumerate() {
            for (x, mobilized) in row.iter().copied().enumerate() {
                if mobilized {
                    expanded_mobilized[y + vertical_offset][x + horizontal_offset] = true;
                }
            }
        }
        self.mobilized = expanded_mobilized;
        self.ingress_focus_x = self
            .ingress_focus_x
            .map(|x| x.saturating_add(horizontal_offset));
        for front in &mut self.boundary_release_fronts {
            front.wall_x = front.wall_x.saturating_add(horizontal_offset);
            front.front_x = front.front_x.saturating_add(horizontal_offset);
        }
        if let Some(coordinate) = &mut self.boundary_release_in_flight {
            coordinate.x = coordinate.x.saturating_add(horizontal_offset);
            coordinate.y = coordinate.y.saturating_add(vertical_offset);
        }
        self.grid_width_dots = target_width;
        self.grid_height_dots = target_height;
        self.supported_heights.resize(target_width, 0);
        (horizontal_offset, vertical_offset)
    }

    fn release_lateral_confinement(
        &mut self,
        previous_bounds: ViewportBounds,
        current_bounds: ViewportBounds,
    ) {
        let released_left = current_bounds.x_start < previous_bounds.x_start;
        let released_right = current_bounds.x_end > previous_bounds.x_end;
        if !released_left && !released_right {
            return;
        }

        self.derive_supported_heights(current_bounds);
        if released_left {
            self.start_boundary_release_front(
                current_bounds,
                previous_bounds.x_start,
                BoundaryReleaseDirection::Left,
            );
        }
        if released_right && previous_bounds.x_end > 0 {
            self.start_boundary_release_front(
                current_bounds,
                previous_bounds.x_end - 1,
                BoundaryReleaseDirection::Right,
            );
        }
    }

    fn start_boundary_release_front(
        &mut self,
        bounds: ViewportBounds,
        source_x: usize,
        direction: BoundaryReleaseDirection,
    ) {
        let Some(target_x) = direction.outward_target(source_x) else {
            return;
        };
        if source_x < bounds.x_start
            || source_x >= bounds.x_end
            || target_x < bounds.x_start
            || target_x >= bounds.x_end
        {
            return;
        }
        if !matches!(
            self.boundary_release_route_state(bounds, source_x, target_x),
            BoundaryReleaseRouteState::Ready { .. } | BoundaryReleaseRouteState::Paused
        ) {
            return;
        }

        // A resize event that falls inside an already admitted release lineage
        // does not create a second obligation. Distinct former walls remain
        // separate until their causal domains actually meet.
        if self.boundary_release_fronts.iter().any(|front| {
            front.direction == direction && Self::boundary_release_front_contains(*front, source_x)
        }) {
            return;
        }

        self.boundary_release_fronts.push(SandStateBoundaryReleaseFront {
            direction,
            wall_x: source_x,
            front_x: source_x,
        });
        self.normalize_boundary_release_fronts();
    }

    fn boundary_release_front_contains(front: SandStateBoundaryReleaseFront, x: usize) -> bool {
        let low = front.wall_x.min(front.front_x);
        let high = front.wall_x.max(front.front_x);
        (low..=high).contains(&x)
    }

    fn boundary_release_front_interval(
        front: SandStateBoundaryReleaseFront,
    ) -> (usize, usize) {
        (front.wall_x.min(front.front_x), front.wall_x.max(front.front_x))
    }

    fn normalize_boundary_release_fronts(&mut self) {
        self.boundary_release_fronts.sort_unstable_by_key(|front| {
            let (low, high) = Self::boundary_release_front_interval(*front);
            (front.direction, low, high)
        });

        let mut normalized: Vec<SandStateBoundaryReleaseFront> = Vec::new();
        for front in self.boundary_release_fronts.drain(..) {
            let Some(last) = normalized.last_mut() else {
                normalized.push(front);
                continue;
            };
            if last.direction != front.direction {
                normalized.push(front);
                continue;
            }

            let (last_low, last_high) = Self::boundary_release_front_interval(*last);
            let (front_low, front_high) = Self::boundary_release_front_interval(front);
            if front_low > last_high.saturating_add(1) {
                normalized.push(front);
                continue;
            }

            let merged_low = last_low.min(front_low);
            let merged_high = last_high.max(front_high);
            match last.direction {
                BoundaryReleaseDirection::Left => {
                    last.wall_x = merged_low;
                    last.front_x = merged_high;
                }
                BoundaryReleaseDirection::Right => {
                    last.wall_x = merged_high;
                    last.front_x = merged_low;
                }
            }
        }
        self.boundary_release_fronts = normalized;
    }

    fn boundary_release_route_state(
        &self,
        bounds: ViewportBounds,
        source_x: usize,
        target_x: usize,
    ) -> BoundaryReleaseRouteState {
        if source_x < bounds.x_start
            || source_x >= bounds.x_end
            || target_x < bounds.x_start
            || target_x >= bounds.x_end
        {
            return BoundaryReleaseRouteState::Paused;
        }

        let source_height = self.supported_heights[source_x];
        if source_height == 0 {
            return BoundaryReleaseRouteState::Reposed;
        }
        let target_height = self.supported_heights[target_x];
        if source_height.saturating_sub(target_height) <= DYNAMIC_REPOSE_RELIEF {
            return BoundaryReleaseRouteState::Reposed;
        }

        let source_y = bounds.y_end - source_height;
        let target_y = source_y.saturating_add(1);
        if target_y >= bounds.y_end
            || self.grid[source_y][source_x].is_none()
            || self.grid[target_y][target_x].is_some()
        {
            return BoundaryReleaseRouteState::Paused;
        }

        BoundaryReleaseRouteState::Ready {
            source_y,
            target_height,
        }
    }

    fn capacity(&self) -> usize {
        if self.grid.is_empty() || self.grid[0].is_empty() {
            0
        } else {
            self.grid.len() * self.grid[0].len()
        }
    }

    fn viewport_bounds(&self) -> Option<ViewportBounds> {
        let grid_height = self.grid.len();
        let grid_width = self.grid.first().map_or(0, Vec::len);
        let visible_width =
            grid_width.min((self.cell_width as usize).saturating_mul(SAND_ENGINE.dot_width));
        let visible_height =
            grid_height.min((self.cell_height as usize).saturating_mul(SAND_ENGINE.dot_height));
        if visible_width == 0 || visible_height == 0 {
            return None;
        }
        let x_start = grid_width.saturating_sub(visible_width) / 2;
        let y_start = grid_height.saturating_sub(visible_height);
        Some(ViewportBounds {
            x_start,
            x_end: x_start + visible_width,
            y_start,
            y_end: y_start + visible_height,
        })
    }

    fn canonical_bounds(&self) -> Option<ViewportBounds> {
        let grid_height = self.grid.len();
        let grid_width = self.grid.first().map_or(0, Vec::len);
        if grid_width == 0 || grid_height == 0 {
            return None;
        }
        Some(ViewportBounds {
            x_start: 0,
            x_end: grid_width,
            y_start: 0,
            y_end: grid_height,
        })
    }

    pub fn spawn(&mut self, category_id: CategoryId) {
        self.add_logical_grains(category_id, 1)
            .expect("single-grain logical count must fit usize");
    }

    pub fn add_logical_grains(
        &mut self,
        category_id: CategoryId,
        count: usize,
    ) -> Result<(), String> {
        if count == 0 {
            return Ok(());
        }

        let next_total = self
            .grain_count
            .checked_add(count)
            .ok_or_else(|| "logical sediment count exceeds the supported range".to_string())?;
        Self::append_pending_run(&mut self.pending_runs, category_id, count)?;
        self.grain_count = next_total;
        self.flush_pending_grains();
        Ok(())
    }

    pub fn pending_grain_count(&self) -> usize {
        self.pending_runs.iter().map(|run| run.count).sum()
    }

    #[cfg(test)]
    fn pending_run_count(&self) -> usize {
        self.pending_runs.len()
    }

    pub fn physical_grain_count(&self) -> usize {
        self.grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_some())
            .count()
    }

    fn refresh_logical_grain_count(&mut self) {
        self.grain_count = self
            .physical_grain_count()
            .checked_add(self.pending_grain_count())
            .expect("validated logical sediment count must fit usize");
    }

    fn append_pending_run(
        runs: &mut VecDeque<PendingRun>,
        category_id: CategoryId,
        count: usize,
    ) -> Result<(), String> {
        if count == 0 {
            return Ok(());
        }

        if let Some(last) = runs.back_mut()
            && last.category_id == category_id
        {
            last.count = last
                .count
                .checked_add(count)
                .ok_or_else(|| "pending sediment run exceeds the supported range".to_string())?;
            return Ok(());
        }

        runs.push_back(PendingRun { category_id, count });
        Ok(())
    }

    fn flush_pending_grains(&mut self) {
        if self.capacity() == 0 || self.pending_runs.is_empty() {
            return;
        }

        let Some(bounds) = self.viewport_bounds() else {
            return;
        };
        let ingress_y = bounds.y_start;
        let mut free_columns = (bounds.x_start..bounds.x_end)
            .filter(|x| self.grid[ingress_y][*x].is_none())
            .collect::<Vec<_>>();

        while !free_columns.is_empty() && !self.pending_runs.is_empty() {
            let free_index = self.choose_ingress_free_index(bounds, &free_columns);
            let x = free_columns.swap_remove(free_index);
            let category_id = self
                .pending_runs
                .front()
                .expect("pending run exists")
                .category_id;
            self.grid[ingress_y][x] = Some(category_id);
            self.mobilized[ingress_y][x] = false;
            if (ingress_y + 1 >= bounds.y_end || self.grid[ingress_y + 1][x].is_some())
                && !self.grain_has_static_support(bounds, x, ingress_y)
            {
                self.mobilized[ingress_y][x] = true;
            }

            let exhausted = {
                let run = self.pending_runs.front_mut().expect("pending run exists");
                run.count -= 1;
                run.count == 0
            };
            if exhausted {
                self.pending_runs.pop_front();
            }
        }
    }

    fn choose_ingress_free_index(
        &mut self,
        bounds: ViewportBounds,
        free_columns: &[usize],
    ) -> usize {
        debug_assert!(!free_columns.is_empty());
        let visible_width = bounds.x_end.saturating_sub(bounds.x_start);
        debug_assert!(visible_width > 0);

        // The slow focus is persistent engine state, but it must not read as a
        // visible nozzle. Every grain starts as a full-width rain sample. A small
        // fraction gets one second full-width candidate and softly favors whichever
        // candidate is closer to the focus. That keeps short-term fall broad while
        // allowing a long-lived focus to accumulate a statistical hill over time.
        // Occupancy may force actual placement to the nearest free top cell, but
        // never outside the visible basin and never by dropping pending mass.
        let focus = self.advance_ingress_focus(bounds);
        let target = self.sample_ingress_target(bounds, focus);

        let mut best_index = 0usize;
        let mut best_distance = free_columns[0].abs_diff(target);
        for (index, x) in free_columns.iter().copied().enumerate().skip(1) {
            let distance = x.abs_diff(target);
            if distance < best_distance || (distance == best_distance && self.random_bool()) {
                best_index = index;
                best_distance = distance;
            }
        }
        best_index
    }

    fn advance_ingress_focus(&mut self, bounds: ViewportBounds) -> usize {
        let visible_width = bounds.x_end.saturating_sub(bounds.x_start);
        debug_assert!(visible_width > 0);

        let mut focus = match self.ingress_focus_x {
            Some(focus) => focus.clamp(bounds.x_start, bounds.x_end - 1),
            None => bounds.x_start + self.random_index(visible_width),
        };

        if self.ingress_focus_x.is_some() && self.random_index(INGRESS_FOCUS_MOVE_ONE_IN) == 0 {
            let step = if self.random_bool() { 1isize } else { -1isize };
            focus = focus
                .saturating_add_signed(step)
                .clamp(bounds.x_start, bounds.x_end - 1);
        }

        self.ingress_focus_x = Some(focus);
        focus
    }

    fn sample_ingress_target(&mut self, bounds: ViewportBounds, focus: usize) -> usize {
        let visible_width = bounds.x_end.saturating_sub(bounds.x_start);
        debug_assert!(visible_width > 0);

        let first = bounds.x_start + self.random_index(visible_width);
        if visible_width == 1 || self.random_index(INGRESS_FOCUS_BIAS_ONE_IN) != 0 {
            return first;
        }

        let second = bounds.x_start + self.random_index(visible_width);
        match first.abs_diff(focus).cmp(&second.abs_diff(focus)) {
            std::cmp::Ordering::Less => first,
            std::cmp::Ordering::Greater => second,
            std::cmp::Ordering::Equal => {
                if self.random_bool() {
                    second
                } else {
                    first
                }
            }
        }
    }

    fn derive_supported_heights(&mut self, bounds: ViewportBounds) {
        self.supported_heights.fill(0);
        for x in bounds.x_start..bounds.x_end {
            let mut height = 0;
            for y in (bounds.y_start..bounds.y_end).rev() {
                if self.grid[y][x].is_none() {
                    break;
                }
                height += 1;
            }
            self.supported_heights[x] = height;
        }
    }

    fn grain_has_static_support(&self, bounds: ViewportBounds, x: usize, y: usize) -> bool {
        if y + 1 >= bounds.y_end {
            return true;
        }
        if self.grid[y + 1][x].is_none() {
            return false;
        }

        let left_brace = x == bounds.x_start || self.grid[y + 1][x.saturating_sub(1)].is_some();
        let right_brace = x + 1 == bounds.x_end || self.grid[y + 1][x + 1].is_some();
        left_brace || right_brace
    }

    fn mobilize_dependents_after_vacancy(
        &mut self,
        bounds: ViewportBounds,
        source_x: usize,
        source_y: usize,
    ) {
        if source_y <= bounds.y_start {
            return;
        }
        let dependent_y = source_y - 1;

        if self.grid[dependent_y][source_x].is_some() {
            self.mobilized[dependent_y][source_x] = true;
            #[cfg(test)]
            {
                self.last_support_loss_mobilizations += 1;
            }
        }

        for dependent_x in [source_x.checked_sub(1), source_x.checked_add(1)]
            .into_iter()
            .flatten()
        {
            if dependent_x < bounds.x_start || dependent_x >= bounds.x_end {
                continue;
            }
            if self.grid[dependent_y][dependent_x].is_none() {
                continue;
            }
            let exposed =
                dependent_y == bounds.y_start || self.grid[dependent_y - 1][dependent_x].is_none();
            if exposed && !self.grain_has_static_support(bounds, dependent_x, dependent_y) {
                self.mobilized[dependent_y][dependent_x] = true;
                #[cfg(test)]
                {
                    self.last_support_loss_mobilizations += 1;
                }
            }
        }
    }

    fn move_vertical_grain(
        &mut self,
        bounds: ViewportBounds,
        x: usize,
        y: usize,
        category: CategoryId,
    ) {
        let was_mobilized = self.mobilized[y][x];
        self.grid[y][x] = None;
        self.mobilized[y][x] = false;
        self.grid[y + 1][x] = Some(category);
        self.mobilized[y + 1][x] = was_mobilized;
        if self.boundary_release_in_flight == Some(SandStateCoordinate { x, y }) {
            self.boundary_release_in_flight = Some(SandStateCoordinate { x, y: y + 1 });
        }

        if was_mobilized {
            #[cfg(test)]
            {
                self.last_mobilized_vertical_moves += 1;
            }
            self.mobilize_dependents_after_vacancy(bounds, x, y);
            #[cfg(test)]
            {
                self.last_avalanche_motion = true;
            }
        } else {
            #[cfg(test)]
            {
                self.last_ordinary_vertical_moves += 1;
            }
            let landed = y + 2 >= bounds.y_end || self.grid[y + 2][x].is_some();
            if landed && !self.grain_has_static_support(bounds, x, y + 1) {
                self.mobilized[y + 1][x] = true;
                #[cfg(test)]
                {
                    self.last_landing_failure_mobilizations += 1;
                }
            }
        }
    }

    fn surface_has_reachable_relief(
        &self,
        bounds: ViewportBounds,
        x: usize,
        threshold: usize,
    ) -> bool {
        let source_height = self.supported_heights[x];
        if source_height == 0 {
            return false;
        }
        let source_y = bounds.y_end - source_height;
        if source_y + 1 >= bounds.y_end || self.grid[source_y][x].is_none() {
            return false;
        }

        [x.checked_sub(1), x.checked_add(1)]
            .into_iter()
            .flatten()
            .any(|target_x| {
                if target_x < bounds.x_start || target_x >= bounds.x_end {
                    return false;
                }
                if self.grid[source_y + 1][target_x].is_some() {
                    return false;
                }
                source_height.saturating_sub(self.supported_heights[target_x]) > threshold
            })
    }

    fn mobilize_exposed_slip_surface(&mut self, bounds: ViewportBounds, x: usize) {
        let source_height = self.supported_heights[x];
        if source_height == 0 {
            return;
        }
        let source_y = bounds.y_end - source_height;
        if self.grid[source_y][x].is_some()
            && self.surface_has_reachable_relief(bounds, x, DYNAMIC_REPOSE_RELIEF)
        {
            self.mobilized[source_y][x] = true;
            #[cfg(test)]
            {
                self.last_slip_lineage_mobilizations += 1;
            }
        }
    }

    fn diagonal_target_for_relief(
        &mut self,
        bounds: ViewportBounds,
        x: usize,
        threshold: usize,
    ) -> Option<(usize, usize)> {
        let source_height = self.supported_heights[x];
        if source_height == 0 {
            return None;
        }
        let source_y = bounds.y_end - source_height;
        if source_y + 1 >= bounds.y_end || self.grid[source_y][x].is_none() {
            return None;
        }

        let mut greatest_relief = 0;
        let mut preferred_target = None;
        let mut equal_relief = false;
        for target_x in [x.checked_sub(1), x.checked_add(1)].into_iter().flatten() {
            if target_x < bounds.x_start || target_x >= bounds.x_end {
                continue;
            }
            if self.grid[source_y + 1][target_x].is_some() {
                continue;
            }
            let target_height = self.supported_heights[target_x];
            if source_height <= target_height {
                continue;
            }
            let relief = source_height - target_height;
            if relief <= threshold {
                continue;
            }
            if relief > greatest_relief {
                greatest_relief = relief;
                preferred_target = Some(target_x);
                equal_relief = false;
            } else if relief == greatest_relief {
                equal_relief = true;
            }
        }
        let preferred_target = preferred_target?;
        let target_x = if equal_relief {
            if self.random_bool() {
                if Some(preferred_target) == x.checked_sub(1) {
                    x.checked_add(1).unwrap()
                } else {
                    x.checked_sub(1).unwrap()
                }
            } else {
                preferred_target
            }
        } else {
            preferred_target
        };
        Some((source_y, target_x))
    }

    fn refresh_boundary_release_in_flight(&mut self, bounds: ViewportBounds) {
        let Some(coordinate) = self.boundary_release_in_flight else {
            return;
        };
        if coordinate.x < bounds.x_start
            || coordinate.x >= bounds.x_end
            || coordinate.y < bounds.y_start
            || coordinate.y >= bounds.y_end
        {
            return;
        }

        let supported_height = self.supported_heights[coordinate.x];
        if supported_height == 0 {
            return;
        }
        let supported_top = bounds.y_end - supported_height;
        if coordinate.y >= supported_top {
            self.boundary_release_in_flight = None;
        }
    }

    fn process_one_boundary_release_front(&mut self, bounds: ViewportBounds) -> bool {
        // Boundary release intentionally serializes its own grains. A released
        // grain must settle into the bottom-connected face before another
        // boundary-release topple can occur; ordinary H4 motion may continue.
        if self.boundary_release_in_flight.is_some() || self.boundary_release_fronts.is_empty() {
            return false;
        }

        let directions = if self.sweep_left_to_right {
            [BoundaryReleaseDirection::Left, BoundaryReleaseDirection::Right]
        } else {
            [BoundaryReleaseDirection::Right, BoundaryReleaseDirection::Left]
        };

        for direction in directions {
            let mut index = 0usize;
            while index < self.boundary_release_fronts.len() {
                if self.boundary_release_fronts[index].direction != direction {
                    index += 1;
                    continue;
                }
                let before_len = self.boundary_release_fronts.len();
                if self.process_boundary_release_front(bounds, index) {
                    return true;
                }
                if self.boundary_release_fronts.len() == before_len {
                    index += 1;
                }
            }
        }
        false
    }

    fn process_boundary_release_front(
        &mut self,
        bounds: ViewportBounds,
        index: usize,
    ) -> bool {
        let front = self.boundary_release_fronts[index];

        match front.direction {
            BoundaryReleaseDirection::Left => {
                for source_x in front.wall_x..=front.front_x {
                    let Some(target_x) = source_x.checked_sub(1) else {
                        self.boundary_release_fronts.remove(index);
                        return false;
                    };
                    match self.boundary_release_route_state(bounds, source_x, target_x) {
                        BoundaryReleaseRouteState::Ready {
                            source_y,
                            target_height,
                        } => {
                            self.topple_boundary_release_grain(
                                bounds,
                                source_x,
                                source_y,
                                target_x,
                                target_height,
                            );
                            return true;
                        }
                        BoundaryReleaseRouteState::Paused => return false,
                        BoundaryReleaseRouteState::Reposed => {}
                    }
                }
            }
            BoundaryReleaseDirection::Right => {
                for source_x in (front.front_x..=front.wall_x).rev() {
                    let Some(target_x) = source_x.checked_add(1) else {
                        self.boundary_release_fronts.remove(index);
                        return false;
                    };
                    match self.boundary_release_route_state(bounds, source_x, target_x) {
                        BoundaryReleaseRouteState::Ready {
                            source_y,
                            target_height,
                        } => {
                            self.topple_boundary_release_grain(
                                bounds,
                                source_x,
                                source_y,
                                target_x,
                                target_height,
                            );
                            return true;
                        }
                        BoundaryReleaseRouteState::Paused => return false,
                        BoundaryReleaseRouteState::Reposed => {}
                    }
                }
            }
        }

        let Some(inward_x) = front.direction.inward_source(front.front_x) else {
            self.boundary_release_fronts.remove(index);
            return false;
        };
        match self.boundary_release_route_state(bounds, inward_x, front.front_x) {
            BoundaryReleaseRouteState::Ready {
                source_y,
                target_height,
            } => {
                self.boundary_release_fronts[index].front_x = inward_x;
                self.topple_boundary_release_grain(
                    bounds,
                    inward_x,
                    source_y,
                    front.front_x,
                    target_height,
                );
                self.normalize_boundary_release_fronts();
                true
            }
            BoundaryReleaseRouteState::Paused => false,
            BoundaryReleaseRouteState::Reposed => {
                self.boundary_release_fronts.remove(index);
                false
            }
        }
    }

    fn topple_boundary_release_grain(
        &mut self,
        bounds: ViewportBounds,
        source_x: usize,
        source_y: usize,
        target_x: usize,
        target_height_before: usize,
    ) {
        debug_assert!(self.boundary_release_in_flight.is_none());
        let source_height = self.supported_heights[source_x];
        let relief_before = source_height.saturating_sub(target_height_before);
        let target_y = source_y + 1;
        let category = self.grid[source_y][source_x]
            .take()
            .expect("boundary-release surface grain exists");
        self.mobilized[source_y][source_x] = false;
        self.grid[target_y][target_x] = Some(category);
        // Boundary release is the causal carrier. The moved grain does not inherit
        // mobility merely because the removed wall let it topple; after this one
        // diagonal release step it returns to ordinary H4 fall/support semantics.
        self.mobilized[target_y][target_x] = false;
        self.mobilize_dependents_after_vacancy(bounds, source_x, source_y);

        self.supported_heights[source_x] = source_height - 1;
        if relief_before == 2 {
            self.supported_heights[target_x] = target_height_before + 1;
        } else {
            // Relief >2 places the grain above the bottom-connected target stack.
            // Keep exact custody until ordinary vertical gravity lands it; no
            // second boundary topple may outrun that visible fall.
            self.boundary_release_in_flight = Some(SandStateCoordinate {
                x: target_x,
                y: target_y,
            });
        }

        #[cfg(test)]
        {
            self.last_diagonal_topple = true;
            self.last_avalanche_motion = true;
            self.last_avalanche_span = source_x.abs_diff(target_x) + 1;
        }
    }

    fn topple_one_mobilized_surface(&mut self, bounds: ViewportBounds) -> bool {
        let left_to_right = !self.sweep_left_to_right;
        if left_to_right {
            for x in bounds.x_start..bounds.x_end {
                if self.topple_mobilized_column(bounds, x) {
                    return true;
                }
            }
        } else {
            for x in (bounds.x_start..bounds.x_end).rev() {
                if self.topple_mobilized_column(bounds, x) {
                    return true;
                }
            }
        }
        false
    }

    fn topple_mobilized_column(&mut self, bounds: ViewportBounds, x: usize) -> bool {
        let source_height = self.supported_heights[x];
        if source_height == 0 {
            return false;
        }
        let source_y = bounds.y_end - source_height;
        if !self.mobilized[source_y][x] {
            return false;
        }

        let Some((_, target_x)) = self.diagonal_target_for_relief(bounds, x, DYNAMIC_REPOSE_RELIEF)
        else {
            self.mobilized[source_y][x] = false;
            return false;
        };

        let target_y = source_y + 1;
        let target_height_before = self.supported_heights[target_x];
        let category = self.grid[source_y][x]
            .take()
            .expect("mobilized surface grain exists");
        self.mobilized[source_y][x] = false;
        self.grid[target_y][target_x] = Some(category);
        self.mobilized[target_y][target_x] = true;
        self.mobilize_dependents_after_vacancy(bounds, x, source_y);

        // A diagonal topple exposes the next grain on the same slip face.
        // Transfer dynamic mobility only down that exact source column, and only
        // while the newly exposed surface still has a reachable dynamic route.
        // This preserves multi-grain avalanche lineage without reviving a broad
        // active radius or allowing unrelated rain to inherit mobility.
        self.supported_heights[x] = source_height - 1;
        if source_height.saturating_sub(target_height_before) == 2 {
            self.supported_heights[target_x] = target_height_before + 1;
        }
        self.mobilize_exposed_slip_surface(bounds, x);

        #[cfg(test)]
        {
            self.last_diagonal_topple = true;
            self.last_avalanche_motion = true;
            self.last_avalanche_span = x.abs_diff(target_x) + 1;
            self.last_mobilized_diagonal_moves += 1;
        }
        true
    }

    fn apply_gravity(&mut self) {
        #[cfg(test)]
        {
            self.last_avalanche_motion = false;
            self.last_diagonal_topple = false;
            self.last_avalanche_span = 0;
            self.last_landing_failure_mobilizations = 0;
            self.last_slip_lineage_mobilizations = 0;
            self.last_support_loss_mobilizations = 0;
            self.last_ordinary_vertical_moves = 0;
            self.last_mobilized_vertical_moves = 0;
            self.last_mobilized_diagonal_moves = 0;
        }
        let Some(bounds) = self.viewport_bounds() else {
            return;
        };
        if bounds.y_end.saturating_sub(bounds.y_start) < 2 {
            return;
        }

        let base_left_to_right = self.sweep_left_to_right;
        self.sweep_left_to_right = !self.sweep_left_to_right;

        for y in (bounds.y_start..bounds.y_end - 1).rev() {
            let left_to_right = if y.is_multiple_of(2) {
                base_left_to_right
            } else {
                !base_left_to_right
            };

            if left_to_right {
                for x in bounds.x_start..bounds.x_end {
                    if let Some(category) = self.grid[y][x] {
                        if self.grid[y + 1][x].is_none() {
                            self.move_vertical_grain(bounds, x, y, category);
                        } else if self.mobilized[y][x] {
                            let buried = y > bounds.y_start && self.grid[y - 1][x].is_some();
                            if buried {
                                self.mobilized[y][x] = false;
                            }
                        }
                    }
                }
            } else {
                for x in (bounds.x_start..bounds.x_end).rev() {
                    if let Some(category) = self.grid[y][x] {
                        if self.grid[y + 1][x].is_none() {
                            self.move_vertical_grain(bounds, x, y, category);
                        } else if self.mobilized[y][x] {
                            let buried = y > bounds.y_start && self.grid[y - 1][x].is_some();
                            if buried {
                                self.mobilized[y][x] = false;
                            }
                        }
                    }
                }
            }
        }

        let floor_y = bounds.y_end - 1;
        for x in bounds.x_start..bounds.x_end {
            self.mobilized[floor_y][x] = false;
        }

        self.derive_supported_heights(bounds);
        self.refresh_boundary_release_in_flight(bounds);
        if !self.process_one_boundary_release_front(bounds) {
            let _ = self.topple_one_mobilized_surface(bounds);
        }
    }

    pub fn update(&mut self) {
        self.frame_count += 1;
        if self.frame_count.is_multiple_of(2) {
            self.apply_gravity();
        }
        self.flush_pending_grains();
    }

    pub fn render(&self, categories: &[Category]) -> Vec<Line<'static>> {
        let cell_w = self.cell_width as usize;
        let cell_h = self.cell_height as usize;
        let viewport_width_dots = cell_w.saturating_mul(SAND_ENGINE.dot_width);
        let viewport_height_dots = cell_h.saturating_mul(SAND_ENGINE.dot_height);
        let grid_h = self.grid.len();
        let grid_w = self.grid.first().map_or(0, |row| row.len());
        let visible_width = grid_w.min(viewport_width_dots);
        let visible_height = grid_h.min(viewport_height_dots);
        let source_x = grid_w.saturating_sub(visible_width) / 2;
        let source_y = grid_h.saturating_sub(visible_height);
        let destination_x = viewport_width_dots.saturating_sub(visible_width) / 2;
        let destination_y = viewport_height_dots.saturating_sub(visible_height);
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(cell_h);

        let category_colors: HashMap<CategoryId, Color> = categories
            .iter()
            .map(|category| (category.id, category.color))
            .collect();
        let none_id = DRIFT_CATEGORY_ID;

        for cy in 0..cell_h {
            let mut spans: Vec<Span<'static>> = Vec::with_capacity(cell_w);

            for cx in 0..cell_w {
                let mut dots = 0u8;
                let mut counts: HashMap<CategoryId, usize> = HashMap::new();

                for dy in 0..SAND_ENGINE.dot_height {
                    for dx in 0..SAND_ENGINE.dot_width {
                        let viewport_x = cx * SAND_ENGINE.dot_width + dx;
                        let viewport_y = cy * SAND_ENGINE.dot_height + dy;

                        if viewport_x < destination_x
                            || viewport_x >= destination_x + visible_width
                            || viewport_y < destination_y
                            || viewport_y >= destination_y + visible_height
                        {
                            continue;
                        }

                        let grid_x = source_x + viewport_x - destination_x;
                        let grid_y = source_y + viewport_y - destination_y;

                        if let Some(cat_id) = self.grid[grid_y][grid_x] {
                            let dot_index = match (dx, dy) {
                                (0, 0) => 0,
                                (0, 1) => 1,
                                (0, 2) => 2,
                                (0, 3) => 6,
                                (1, 0) => 3,
                                (1, 1) => 4,
                                (1, 2) => 5,
                                (1, 3) => 7,
                                _ => 0,
                            };
                            dots |= 1 << dot_index;
                            *counts.entry(cat_id).or_insert(0) += 1;
                        }
                    }
                }

                let total_colored_dots: usize = counts.values().sum();
                let color = if total_colored_dots > 0 {
                    let mut blended_r = 0f32;
                    let mut blended_g = 0f32;
                    let mut blended_b = 0f32;

                    for (category_id, count) in &counts {
                        let (r, g, b) = if *category_id == none_id {
                            (255u8, 255u8, 255u8)
                        } else {
                            match category_colors
                                .get(category_id)
                                .copied()
                                .unwrap_or(Color::White)
                            {
                                Color::Rgb(r, g, b) => (r, g, b),
                                _ => (255, 255, 255),
                            }
                        };

                        let weight = *count as f32 / total_colored_dots as f32;
                        blended_r += r as f32 * weight;
                        blended_g += g as f32 * weight;
                        blended_b += b as f32 * weight;
                    }

                    Color::Rgb(blended_r as u8, blended_g as u8, blended_b as u8)
                } else {
                    Color::White
                };

                let ch = char::from_u32(SAND_ENGINE.braille_base + dots as u32).unwrap_or(' ');
                spans.push(Span::raw(ch.to_string()).fg(color));
            }

            lines.push(Line::from(spans));
        }

        lines
    }

    pub fn clear(&mut self) {
        self.grid_width_dots = (self.cell_width as usize).saturating_mul(SAND_ENGINE.dot_width);
        self.grid_height_dots = (self.cell_height as usize).saturating_mul(SAND_ENGINE.dot_height);
        self.grid = vec![vec![None; self.grid_width_dots]; self.grid_height_dots];
        self.mobilized = vec![vec![false; self.grid_width_dots]; self.grid_height_dots];
        self.boundary_release_fronts.clear();
        self.boundary_release_in_flight = None;
        self.pending_runs.clear();
        self.ingress_focus_x = None;
        self.grain_count = 0;
    }

    pub fn clear_category(&mut self, category_id: CategoryId) {
        let mut removed = Vec::new();
        for (y, row) in self.grid.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                if *cell == Some(category_id) {
                    *cell = None;
                    self.mobilized[y][x] = false;
                    if self.boundary_release_in_flight == Some(SandStateCoordinate { x, y }) {
                        self.boundary_release_in_flight = None;
                    }
                    removed.push((x, y));
                }
            }
        }

        if let Some(bounds) = self.canonical_bounds() {
            for (x, y) in removed {
                self.mobilize_dependents_after_vacancy(bounds, x, y);
            }
        }

        self.pending_runs
            .retain(|run| run.category_id != category_id);
        self.refresh_logical_grain_count();
    }

    pub fn remove_category_grains(&mut self, category_id: CategoryId, count: usize) -> usize {
        if count == 0 || self.grain_count == 0 {
            return 0;
        }

        let mut removed = 0usize;
        let mut removed_coordinates = Vec::new();
        for y in (0..self.grid.len()).rev() {
            for x in 0..self.grid[y].len() {
                if removed >= count {
                    break;
                }

                if self.grid[y][x] == Some(category_id) {
                    self.grid[y][x] = None;
                    self.mobilized[y][x] = false;
                    if self.boundary_release_in_flight == Some(SandStateCoordinate { x, y }) {
                        self.boundary_release_in_flight = None;
                    }
                    removed_coordinates.push((x, y));
                    removed += 1;
                }
            }

            if removed >= count {
                break;
            }
        }

        if let Some(bounds) = self.canonical_bounds() {
            for &(x, y) in &removed_coordinates {
                self.mobilize_dependents_after_vacancy(bounds, x, y);
            }
        }

        let mut pending_removed = 0usize;
        if removed < count {
            let mut remaining = count - removed;
            for run in &mut self.pending_runs {
                if remaining == 0 {
                    break;
                }
                if run.category_id != category_id {
                    continue;
                }
                let take = run.count.min(remaining);
                run.count -= take;
                remaining -= take;
                pending_removed += take;
            }
            self.pending_runs.retain(|run| run.count > 0);
        }

        if removed > 0 {
            self.apply_gravity();
        }
        self.flush_pending_grains();
        self.refresh_logical_grain_count();

        removed + pending_removed
    }

    pub fn snapshot_state(&self) -> SandState {
        let grid_height = self.grid.len();
        let grid_width = self.grid.first().map_or(0, |row| row.len());
        let mut grains = Vec::with_capacity(self.physical_grain_count());
        let mut mobilized_grains = Vec::new();

        for (y, row) in self.grid.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if let Some(category_id) = cell {
                    grains.push(SandStateGrain {
                        x,
                        y,
                        category_id: category_id.0,
                    });
                    if self.mobilized[y][x] {
                        mobilized_grains.push(SandStateCoordinate { x, y });
                    }
                } else {
                    debug_assert!(
                        !self.mobilized[y][x],
                        "empty sand cell cannot carry mobilized state"
                    );
                }
            }
        }

        SandState {
            version: SandState::VERSION,
            grid_width,
            grid_height,
            grains,
            frame_count: self.frame_count,
            sweep_left_to_right: self.sweep_left_to_right,
            rng_state: self.rng_state,
            ingress_focus_x: self.ingress_focus_x,
            pending_grains: Vec::new(),
            pending_runs: self
                .pending_runs
                .iter()
                .map(|run| PendingGrainRun {
                    category_id: run.category_id.0,
                    count: run.count,
                })
                .collect(),
            active_avalanche_columns: Vec::new(),
            mobilized_grains,
            boundary_release_fronts: self.boundary_release_fronts.clone(),
            boundary_release_in_flight: self.boundary_release_in_flight,
        }
    }

    fn seed_pre_v5_mobility(&mut self) {
        let Some(bounds) = self.canonical_bounds() else {
            return;
        };
        if bounds.y_end.saturating_sub(bounds.y_start) < 2 {
            return;
        }

        // v1-v4 do not contain exact grain-causal dynamic state. Do not attempt
        // to translate v4 regional activity into per-grain causality: that would
        // preserve the obsolete approximation H4 deliberately retired. Instead,
        // preserve topology exactly and deterministically seed only currently
        // unsupported bottom-connected surface grains. The first ordinary
        // grain-causal gravity passes relax those grains through H4 mechanics.
        self.derive_supported_heights(bounds);
        for x in bounds.x_start..bounds.x_end {
            let height = self.supported_heights[x];
            if height == 0 {
                continue;
            }
            let y = bounds.y_end - height;
            if !self.grain_has_static_support(bounds, x, y) {
                self.mobilized[y][x] = true;
            }
        }
    }

    pub fn restore_state(
        &mut self,
        state: &SandState,
        valid_category_ids: &HashSet<CategoryId>,
    ) -> Result<(), String> {
        if state.version != SandState::VERSION
            && state.version != SandState::GRAIN_CAUSAL_VERSION
            && state.version != SandState::REGIONAL_AVALANCHE_VERSION
            && state.version != SandState::COMPRESSED_PENDING_VERSION
            && state.version != SandState::ORGANIC_VERSION
            && state.version != SandState::LEGACY_VERSION
        {
            return Err(format!("unsupported sand state version {}", state.version));
        }
        if (state.grid_width == 0 || state.grid_height == 0) && !state.grains.is_empty() {
            return Err("zero-sized sand state cannot contain placed grains".to_string());
        }
        if state.version != SandState::VERSION
            && state.version != SandState::GRAIN_CAUSAL_VERSION
            && state.version != SandState::REGIONAL_AVALANCHE_VERSION
            && state.version != SandState::ORGANIC_VERSION
            && state.ingress_focus_x.is_some()
        {
            return Err("pre-organic sand state contains an ingress focus".to_string());
        }
        if state.version != SandState::REGIONAL_AVALANCHE_VERSION
            && !state.active_avalanche_columns.is_empty()
        {
            return Err(
                "only v4 sand state may contain legacy active avalanche columns".to_string(),
            );
        }
        if state.version != SandState::VERSION
            && state.version != SandState::GRAIN_CAUSAL_VERSION
            && !state.mobilized_grains.is_empty()
        {
            return Err("pre-v5 sand state contains mobilized grain coordinates".to_string());
        }
        if state.version != SandState::VERSION && !state.boundary_release_fronts.is_empty() {
            return Err("pre-v6 sand state contains boundary-release fronts".to_string());
        }
        if state
            .active_avalanche_columns
            .windows(2)
            .any(|columns| columns[0] >= columns[1])
        {
            return Err("sand active avalanche columns must be strictly sorted".to_string());
        }
        if state
            .active_avalanche_columns
            .iter()
            .any(|&x| x >= state.grid_width)
        {
            return Err("sand active avalanche column is outside the canonical grid".to_string());
        }
        if state.mobilized_grains.windows(2).any(|coordinates| {
            (coordinates[0].y, coordinates[0].x) >= (coordinates[1].y, coordinates[1].x)
        }) {
            return Err(
                "sand mobilized grain coordinates must be strictly row-major sorted".to_string(),
            );
        }
        if state
            .mobilized_grains
            .iter()
            .any(|coordinate| coordinate.x >= state.grid_width || coordinate.y >= state.grid_height)
        {
            return Err(
                "sand mobilized grain coordinate is outside the canonical grid".to_string(),
            );
        }
        let release_sort_key = |front: &SandStateBoundaryReleaseFront| {
            let low = front.wall_x.min(front.front_x);
            let high = front.wall_x.max(front.front_x);
            (front.direction, low, high)
        };
        if state
            .boundary_release_fronts
            .windows(2)
            .any(|fronts| release_sort_key(&fronts[0]) >= release_sort_key(&fronts[1]))
        {
            return Err("sand boundary-release fronts must be strictly sorted".to_string());
        }
        for front in &state.boundary_release_fronts {
            if front.wall_x >= state.grid_width || front.front_x >= state.grid_width {
                return Err("sand boundary-release front is outside the canonical grid".to_string());
            }
            let direction_shape_valid = match front.direction {
                BoundaryReleaseDirection::Left => front.wall_x <= front.front_x && front.wall_x > 0,
                BoundaryReleaseDirection::Right => {
                    front.front_x <= front.wall_x
                        && front
                            .wall_x
                            .checked_add(1)
                            .is_some_and(|target_x| target_x < state.grid_width)
                }
            };
            if !direction_shape_valid {
                return Err(
                    "sand boundary-release front has invalid direction or outward target"
                        .to_string(),
                );
            }
        }
        for fronts in state.boundary_release_fronts.windows(2) {
            if fronts[0].direction != fronts[1].direction {
                continue;
            }
            let (_, first_high) = Self::boundary_release_front_interval(fronts[0]);
            let (second_low, _) = Self::boundary_release_front_interval(fronts[1]);
            if second_low <= first_high.saturating_add(1) {
                return Err(
                    "sand boundary-release fronts of one direction must be normalized".to_string(),
                );
            }
        }
        if state.version != SandState::VERSION && state.boundary_release_in_flight.is_some() {
            return Err("pre-v6 sand state contains a boundary-release in-flight grain".to_string());
        }
        if state.boundary_release_in_flight.is_some() && state.boundary_release_fronts.is_empty() {
            return Err(
                "sand boundary-release in-flight grain requires a release front".to_string(),
            );
        }

        if let Some(focus_x) = state.ingress_focus_x
            && (state.grid_width == 0 || focus_x >= state.grid_width)
        {
            return Err(format!(
                "sand ingress focus {focus_x} is outside the {}-column canonical grid",
                state.grid_width
            ));
        }

        let mut restored = vec![vec![None; state.grid_width]; state.grid_height];
        let mut occupied = HashSet::with_capacity(state.grains.len());
        for grain in &state.grains {
            if grain.x >= state.grid_width || grain.y >= state.grid_height {
                return Err(format!(
                    "sand grain ({}, {}) is outside the {}x{} canonical grid",
                    grain.x, grain.y, state.grid_width, state.grid_height
                ));
            }
            let category_id = CategoryId::new(grain.category_id);
            if !valid_category_ids.contains(&category_id) {
                return Err(format!(
                    "sand state references unknown category ID {}",
                    grain.category_id
                ));
            }
            if !occupied.insert((grain.x, grain.y)) {
                return Err(format!(
                    "sand state contains duplicate grain coordinate ({}, {})",
                    grain.x, grain.y
                ));
            }
            restored[grain.y][grain.x] = Some(category_id);
        }
        if state
            .mobilized_grains
            .iter()
            .any(|coordinate| !occupied.contains(&(coordinate.x, coordinate.y)))
        {
            return Err(
                "sand mobilized grain coordinate does not reference a placed grain".to_string(),
            );
        }

        if let Some(coordinate) = state.boundary_release_in_flight {
            if coordinate.x >= state.grid_width || coordinate.y >= state.grid_height {
                return Err(
                    "sand boundary-release in-flight grain is outside the canonical grid"
                        .to_string(),
                );
            }
            if !occupied.contains(&(coordinate.x, coordinate.y)) {
                return Err(
                    "sand boundary-release in-flight coordinate does not reference a placed grain"
                        .to_string(),
                );
            }
            if state.mobilized_grains.contains(&coordinate) {
                return Err(
                    "sand boundary-release in-flight grain cannot simultaneously carry H4 mobility"
                        .to_string(),
                );
            }
            let belongs_to_release = state.boundary_release_fronts.iter().any(|front| {
                let source_x = match front.direction {
                    BoundaryReleaseDirection::Left => coordinate.x.checked_add(1),
                    BoundaryReleaseDirection::Right => coordinate.x.checked_sub(1),
                };
                source_x.is_some_and(|x| Self::boundary_release_front_contains(*front, x))
            });
            if !belongs_to_release {
                return Err(
                    "sand boundary-release in-flight grain is unrelated to every persisted release front"
                        .to_string(),
                );
            }
        }

        let mut pending_runs = VecDeque::new();
        let mut append_serialized_run = |category_id: u64, count: usize| -> Result<(), String> {
            if count == 0 {
                return Err(format!(
                    "sand state contains a zero-count pending run for category {category_id}"
                ));
            }
            let category_id = CategoryId::new(category_id);
            if !valid_category_ids.contains(&category_id) {
                return Err(format!(
                    "sand state references unknown pending category ID {}",
                    category_id.0
                ));
            }
            Self::append_pending_run(&mut pending_runs, category_id, count)
        };

        if state.version == SandState::LEGACY_VERSION {
            if !state.pending_runs.is_empty() {
                return Err("legacy sand state cannot contain compressed pending runs".to_string());
            }
            for category_id in &state.pending_grains {
                append_serialized_run(*category_id, 1)?;
            }
        } else {
            if !state.pending_runs.is_empty() && !state.pending_grains.is_empty() {
                return Err(
                    "sand state contains both legacy pending grains and compressed pending runs"
                        .to_string(),
                );
            }
            if state.pending_runs.is_empty() {
                for category_id in &state.pending_grains {
                    append_serialized_run(*category_id, 1)?;
                }
            } else {
                for run in &state.pending_runs {
                    append_serialized_run(run.category_id, run.count)?;
                }
            }
        }

        let physical_count = state.grains.len();
        let pending_count = pending_runs.iter().try_fold(0usize, |total, run| {
            total
                .checked_add(run.count)
                .ok_or_else(|| "pending sediment count exceeds the supported range".to_string())
        })?;
        let logical_count = physical_count
            .checked_add(pending_count)
            .ok_or_else(|| "logical sediment count exceeds the supported range".to_string())?;

        self.grid = restored;
        self.mobilized = vec![vec![false; state.grid_width]; state.grid_height];
        if state.version == SandState::VERSION || state.version == SandState::GRAIN_CAUSAL_VERSION {
            for coordinate in &state.mobilized_grains {
                self.mobilized[coordinate.y][coordinate.x] = true;
            }
        }
        self.boundary_release_fronts = if state.version == SandState::VERSION {
            state.boundary_release_fronts.clone()
        } else {
            Vec::new()
        };
        self.boundary_release_in_flight = if state.version == SandState::VERSION {
            state.boundary_release_in_flight
        } else {
            None
        };
        self.grid_width_dots = state.grid_width;
        self.grid_height_dots = state.grid_height;
        self.pending_runs = pending_runs;
        self.grain_count = logical_count;
        self.frame_count = state.frame_count;
        self.sweep_left_to_right = state.sweep_left_to_right;
        self.rng_state = if state.rng_state == 0 {
            default_rng_state()
        } else {
            state.rng_state
        };
        self.ingress_focus_x = if state.version == SandState::VERSION
            || state.version == SandState::GRAIN_CAUSAL_VERSION
            || state.version == SandState::REGIONAL_AVALANCHE_VERSION
            || state.version == SandState::ORGANIC_VERSION
        {
            state.ingress_focus_x
        } else {
            None
        };
        self.supported_heights = vec![0; state.grid_width];
        if state.version != SandState::VERSION && state.version != SandState::GRAIN_CAUSAL_VERSION {
            self.seed_pre_v5_mobility();
        }
        self.expand_logical_canvas_to_viewport();
        Ok(())
    }

    fn next_random_u64(&mut self) -> u64 {
        if self.rng_state == 0 {
            self.rng_state = default_rng_state();
        }

        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    fn random_bool(&mut self) -> bool {
        self.next_random_u64() & 1 == 1
    }

    fn random_index(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_random_u64() % upper as u64) as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{
        domain::CategoryId,
        sand::{
            BoundaryReleaseDirection, PendingGrainRun, SandEngine, SandState,
            SandStateBoundaryReleaseFront, SandStateGrain, recolor_state_category_mass,
        },
    };

    fn category_mass(engine: &SandEngine, category_id: CategoryId) -> usize {
        let placed = engine
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| **cell == Some(category_id))
            .count();
        let pending = engine
            .pending_runs
            .iter()
            .filter(|run| run.category_id == category_id)
            .map(|run| run.count)
            .sum::<usize>();
        placed + pending
    }

    #[test]
    fn state_recolor_preserves_topology_mass_metadata_and_pending_order() {
        let mut state = SandState {
            version: SandState::VERSION,
            grid_width: 4,
            grid_height: 3,
            grains: vec![
                SandStateGrain {
                    x: 0,
                    y: 2,
                    category_id: 1,
                },
                SandStateGrain {
                    x: 1,
                    y: 2,
                    category_id: 2,
                },
                SandStateGrain {
                    x: 2,
                    y: 2,
                    category_id: 1,
                },
            ],
            frame_count: 77,
            sweep_left_to_right: false,
            rng_state: 12345,
            ingress_focus_x: Some(2),
            pending_grains: Vec::new(),
            pending_runs: vec![
                PendingGrainRun {
                    category_id: 1,
                    count: 3,
                },
                PendingGrainRun {
                    category_id: 2,
                    count: 2,
                },
            ],
            active_avalanche_columns: Vec::new(),
            mobilized_grains: Vec::new(),
            boundary_release_fronts: Vec::new(),
            boundary_release_in_flight: None,
        };
        let before_coordinates = state
            .grains
            .iter()
            .map(|grain| (grain.x, grain.y))
            .collect::<Vec<_>>();
        let before_mass = state.grains.len()
            + state
                .pending_runs
                .iter()
                .map(|run| run.count)
                .sum::<usize>();

        let recolored =
            recolor_state_category_mass(&mut state, CategoryId::new(1), CategoryId::new(3), 4);

        assert_eq!(recolored, 4);
        assert_eq!(
            state
                .grains
                .iter()
                .map(|grain| (grain.x, grain.y))
                .collect::<Vec<_>>(),
            before_coordinates
        );
        assert_eq!(state.grains[0].category_id, 3);
        assert_eq!(state.grains[1].category_id, 2);
        assert_eq!(state.grains[2].category_id, 3);
        assert_eq!(
            state.pending_runs,
            vec![
                PendingGrainRun {
                    category_id: 3,
                    count: 2,
                },
                PendingGrainRun {
                    category_id: 1,
                    count: 1,
                },
                PendingGrainRun {
                    category_id: 2,
                    count: 2,
                },
            ]
        );
        assert_eq!(
            state.grains.len()
                + state
                    .pending_runs
                    .iter()
                    .map(|run| run.count)
                    .sum::<usize>(),
            before_mass
        );
        assert_eq!(state.frame_count, 77);
        assert!(!state.sweep_left_to_right);
        assert_eq!(state.rng_state, 12345);
        assert_eq!(state.ingress_focus_x, Some(2));
        assert_eq!(state.grid_width, 4);
        assert_eq!(state.grid_height, 3);
    }

    #[test]
    fn state_recolor_caps_at_retained_source_mass_without_fabrication() {
        let mut state = SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: vec![SandStateGrain {
                x: 0,
                y: 1,
                category_id: 1,
            }],
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 7,
            ingress_focus_x: None,
            pending_grains: Vec::new(),
            pending_runs: vec![PendingGrainRun {
                category_id: 2,
                count: 4,
            }],
            active_avalanche_columns: Vec::new(),
            mobilized_grains: Vec::new(),
            boundary_release_fronts: Vec::new(),
            boundary_release_in_flight: None,
        };

        let recolored =
            recolor_state_category_mass(&mut state, CategoryId::new(1), CategoryId::new(3), 60);

        assert_eq!(recolored, 1);
        assert_eq!(state.grains[0].category_id, 3);
        assert_eq!(state.pending_runs[0].category_id, 2);
        assert_eq!(state.pending_runs[0].count, 4);
    }

    #[test]
    fn resize_expands_logical_canvas_monotonically_and_conserves_mass() {
        let mut engine = SandEngine::new(12, 8);
        engine.clear();
        engine.grid[0][0] = Some(CategoryId::new(1));
        engine.grid[31][23] = Some(CategoryId::new(2));
        SandEngine::append_pending_run(&mut engine.pending_runs, CategoryId::new(2), 1).unwrap();
        engine.grain_count = 3;
        let mass_1 = category_mass(&engine, CategoryId::new(1));
        let mass_2 = category_mass(&engine, CategoryId::new(2));

        engine.resize(30, 20);
        let expanded_width = engine.grid_width_dots;
        let expanded_height = engine.grid_height_dots;
        assert_eq!(expanded_width, 30 * crate::constants::SAND_ENGINE.dot_width);
        assert_eq!(
            expanded_height,
            20 * crate::constants::SAND_ENGINE.dot_height
        );
        assert_eq!(category_mass(&engine, CategoryId::new(1)), mass_1);
        assert_eq!(category_mass(&engine, CategoryId::new(2)), mass_2);
        assert_eq!(engine.grain_count, 3);

        let after_growth = engine.snapshot_state();
        engine.resize(3, 2);
        engine.resize(30, 20);
        assert_eq!(engine.grid_width_dots, expanded_width);
        assert_eq!(engine.grid_height_dots, expanded_height);
        assert_eq!(engine.snapshot_state(), after_growth);
    }

    #[test]
    fn growth_preserves_horizontal_center_and_bottom_baseline() {
        let mut engine = SandEngine::new(4, 2);
        engine.clear();
        let old_width = engine.grid_width_dots;
        let old_height = engine.grid_height_dots;
        engine.grid[old_height - 1][0] = Some(CategoryId::new(1));
        engine.grid[old_height - 1][old_width - 1] = Some(CategoryId::new(2));
        engine.grain_count = 2;

        engine.resize(8, 4);
        let x_offset = (engine.grid_width_dots - old_width) / 2;
        let y_offset = engine.grid_height_dots - old_height;
        assert_eq!(
            engine.grid[y_offset + old_height - 1][x_offset],
            Some(CategoryId::new(1))
        );
        assert_eq!(
            engine.grid[y_offset + old_height - 1][x_offset + old_width - 1],
            Some(CategoryId::new(2))
        );
    }

    #[test]
    fn narrowing_remains_projection_only_and_does_not_create_mobility() {
        let mut engine = SandEngine::new(12, 3);
        engine.clear();
        let bottom = engine.grid_height_dots;
        let x = engine.grid_width_dots / 2;
        for y in bottom - 4..bottom {
            engine.grid[y][x] = Some(CategoryId::new(1));
        }
        engine.grain_count = 4;
        let before = engine.snapshot_state();

        engine.resize(4, 2);

        assert_eq!(engine.snapshot_state(), before);
    }

    fn fill_supported_column(
        engine: &mut SandEngine,
        x: usize,
        height: usize,
        category_id: CategoryId,
    ) {
        let bottom = engine.grid_height_dots;
        for y in bottom - height..bottom {
            engine.grid[y][x] = Some(category_id);
        }
    }

    #[test]
    fn reexpansion_starts_boundary_release_without_resize_reflow_or_invented_mobility() {
        let mut engine = SandEngine::new(12, 3);
        engine.clear();
        engine.resize(4, 3);
        let previous_bounds = engine.viewport_bounds().expect("visible viewport");
        let wall_x = previous_bounds.x_start;
        let bottom = previous_bounds.y_end;

        fill_supported_column(&mut engine, wall_x, 6, CategoryId::new(1));
        fill_supported_column(&mut engine, wall_x + 1, 5, CategoryId::new(1));
        engine.grain_count = 11;
        let surface_y = bottom - 6;
        let before_grid = engine.grid.clone();
        let before_rng = engine.rng_state;

        engine.resize(12, 3);

        assert_eq!(engine.grid, before_grid, "resize must not reflow grains");
        assert_eq!(engine.grain_count, 11);
        assert_eq!(engine.rng_state, before_rng, "resize must not consume RNG");
        assert!(engine.mobilized.iter().flatten().all(|mobile| !*mobile));
        assert_eq!(
            engine.boundary_release_fronts,
            vec![SandStateBoundaryReleaseFront {
                direction: BoundaryReleaseDirection::Left,
                wall_x,
                front_x: wall_x,
            }]
        );
        assert!(engine.boundary_release_in_flight.is_none());

        engine.apply_gravity();

        assert_eq!(engine.grid[surface_y][wall_x], None);
        assert_eq!(
            engine.grid[surface_y + 1][wall_x - 1],
            Some(CategoryId::new(1))
        );
        assert_eq!(engine.grain_count, 11);
        assert_eq!(
            engine.boundary_release_in_flight,
            Some(SandStateCoordinate {
                x: wall_x - 1,
                y: surface_y + 1,
            })
        );
    }

    fn assert_boundary_release_staircase(direction: BoundaryReleaseDirection) {
        let mut engine = SandEngine::new(12, 6);
        engine.clear();
        engine.resize(4, 6);
        let previous_bounds = engine.viewport_bounds().expect("visible viewport");
        let wall_x = match direction {
            BoundaryReleaseDirection::Left => previous_bounds.x_start,
            BoundaryReleaseDirection::Right => previous_bounds.x_end - 1,
        };
        let outward_x = direction.outward_target(wall_x).expect("canonical outward column");
        let inward_1 = direction.inward_source(wall_x).expect("first inward column");
        let inward_2 = direction.inward_source(inward_1).expect("second inward column");

        fill_supported_column(&mut engine, wall_x, 2, CategoryId::new(1));
        fill_supported_column(&mut engine, inward_1, 4, CategoryId::new(1));
        fill_supported_column(&mut engine, inward_2, 4, CategoryId::new(1));
        engine.grain_count = 10;
        let before_grid = engine.grid.clone();

        engine.resize(12, 6);
        assert_eq!(engine.grid, before_grid, "resize must remain movement-free");
        assert_eq!(
            engine.boundary_release_fronts,
            vec![SandStateBoundaryReleaseFront {
                direction,
                wall_x,
                front_x: wall_x,
            }]
        );

        engine.apply_gravity();
        assert_eq!(engine.boundary_release_fronts[0].front_x, wall_x);

        engine.apply_gravity();
        assert_eq!(
            engine.boundary_release_fronts[0].front_x, inward_1,
            "the release must admit the next inward column after the former wall reaches repose"
        );
        assert!(engine.boundary_release_in_flight.is_some());

        engine.apply_gravity();
        assert!(engine.boundary_release_in_flight.is_none());
        assert!(
            engine.boundary_release_fronts.is_empty(),
            "the release front must stop only after its released grain lands and the whole admitted face is stable"
        );
        let bounds = engine.viewport_bounds().expect("expanded viewport");
        engine.derive_supported_heights(bounds);
        assert_eq!(engine.supported_heights[outward_x], 1);
        assert_eq!(engine.supported_heights[wall_x], 2);
        assert_eq!(engine.supported_heights[inward_1], 3);
        assert_eq!(engine.supported_heights[inward_2], 4);
        assert_eq!(engine.grain_count, 10);
    }

    #[test]
    fn left_boundary_release_propagates_inward_until_the_face_reaches_repose() {
        assert_boundary_release_staircase(BoundaryReleaseDirection::Left);
    }

    #[test]
    fn right_boundary_release_propagates_inward_until_the_face_reaches_repose() {
        assert_boundary_release_staircase(BoundaryReleaseDirection::Right);
    }

    fn assert_tall_boundary_face_releases_without_leaving_a_new_frozen_cliff(
        direction: BoundaryReleaseDirection,
    ) {
        let mut engine = SandEngine::new(14, 8);
        engine.clear();
        engine.resize(5, 8);
        let previous_bounds = engine.viewport_bounds().expect("visible viewport");
        let wall_x = match direction {
            BoundaryReleaseDirection::Left => previous_bounds.x_start,
            BoundaryReleaseDirection::Right => previous_bounds.x_end - 1,
        };
        let outward_x = direction.outward_target(wall_x).expect("outward column");
        let inward_1 = direction.inward_source(wall_x).expect("inward 1");
        let inward_2 = direction.inward_source(inward_1).expect("inward 2");
        let inward_3 = direction.inward_source(inward_2).expect("inward 3");

        for (x, height) in [(wall_x, 2), (inward_1, 4), (inward_2, 6), (inward_3, 8)] {
            fill_supported_column(&mut engine, x, height, CategoryId::new(1));
        }
        engine.grain_count = 20;
        engine.resize(14, 8);

        let mut passes = 0usize;
        while (!engine.boundary_release_fronts.is_empty()
            || engine.boundary_release_in_flight.is_some())
            && passes < 128
        {
            engine.apply_gravity();
            passes += 1;
        }
        assert!(passes < 128, "boundary release must converge in bounded gravity passes");
        assert!(engine.boundary_release_fronts.is_empty());
        assert!(engine.boundary_release_in_flight.is_none());
        assert_eq!(engine.grain_count, 20);

        let bounds = engine.viewport_bounds().expect("expanded viewport");
        engine.derive_supported_heights(bounds);
        let columns = vec![outward_x, wall_x, inward_1, inward_2, inward_3];
        for pair in columns.windows(2) {
            let outward_height = engine.supported_heights[pair[0]];
            let inward_height = engine.supported_heights[pair[1]];
            assert!(
                inward_height.saturating_sub(outward_height) <= DYNAMIC_REPOSE_RELIEF,
                "released face retained a cliff between columns {} and {}: {} -> {}",
                pair[0],
                pair[1],
                outward_height,
                inward_height
            );
        }
    }

    #[test]
    fn tall_left_boundary_face_revisits_outward_steps_until_stable() {
        assert_tall_boundary_face_releases_without_leaving_a_new_frozen_cliff(
            BoundaryReleaseDirection::Left,
        );
    }

    #[test]
    fn tall_right_boundary_face_revisits_outward_steps_until_stable() {
        assert_tall_boundary_face_releases_without_leaving_a_new_frozen_cliff(
            BoundaryReleaseDirection::Right,
        );
    }

    #[test]
    fn boundary_release_in_flight_grain_survives_exact_v6_restart() {
        let mut source = SandEngine::new(12, 6);
        source.clear();
        source.resize(4, 6);
        let bounds = source.viewport_bounds().expect("visible viewport");
        let wall_x = bounds.x_start;
        fill_supported_column(&mut source, wall_x, 4, CategoryId::new(1));
        source.grain_count = 4;
        source.resize(12, 6);
        source.apply_gravity();
        assert!(source.boundary_release_in_flight.is_some());

        let checkpoint = source.snapshot_state();
        assert_eq!(checkpoint.version, SandState::VERSION);
        assert_eq!(
            checkpoint.boundary_release_fronts,
            source.boundary_release_fronts
        );
        assert_eq!(
            checkpoint.boundary_release_in_flight,
            source.boundary_release_in_flight
        );

        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut restored = SandEngine::new(12, 6);
        restored.restore_state(&checkpoint, &valid).unwrap();
        assert_eq!(restored.snapshot_state(), checkpoint);

        for _ in 0..8 {
            source.apply_gravity();
            restored.apply_gravity();
            assert_eq!(restored.snapshot_state(), source.snapshot_state());
            if source.boundary_release_in_flight.is_none() {
                break;
            }
        }
        assert!(source.boundary_release_in_flight.is_none());
    }

    #[test]
    fn canonical_growth_tracks_shifted_release_state_without_mass_change() {
        let mut engine = SandEngine::new(4, 3);
        engine.clear();
        let old_width = engine.grid_width_dots;
        let bottom = engine.grid_height_dots;
        let left_x = 0;
        let right_x = old_width - 1;

        for y in bottom - 4..bottom {
            engine.grid[y][left_x] = Some(CategoryId::new(1));
            engine.grid[y][right_x] = Some(CategoryId::new(2));
        }
        for y in bottom - 3..bottom {
            engine.grid[y][left_x + 1] = Some(CategoryId::new(1));
            engine.grid[y][right_x - 1] = Some(CategoryId::new(2));
        }
        engine.grain_count = 14;

        engine.resize(8, 3);

        let offset = (engine.grid_width_dots - old_width) / 2;
        let shifted_left = left_x + offset;
        let shifted_right = right_x + offset;
        let surface_y = bottom - 4;
        assert_eq!(engine.grain_count, 14);
        assert_eq!(
            engine.grid[surface_y][shifted_left],
            Some(CategoryId::new(1))
        );
        assert_eq!(
            engine.grid[surface_y][shifted_right],
            Some(CategoryId::new(2))
        );
        assert!(engine.mobilized.iter().flatten().all(|mobile| !*mobile));
        assert_eq!(
            engine.boundary_release_fronts,
            vec![
                SandStateBoundaryReleaseFront {
                    direction: BoundaryReleaseDirection::Left,
                    wall_x: shifted_left,
                    front_x: shifted_left,
                },
                SandStateBoundaryReleaseFront {
                    direction: BoundaryReleaseDirection::Right,
                    wall_x: shifted_right,
                    front_x: shifted_right,
                },
            ]
        );
        assert!(engine.boundary_release_in_flight.is_none());
    }

    #[test]
    fn reexpansion_can_start_a_visible_release_while_an_older_same_side_front_is_hidden() {
        let mut engine = SandEngine::new(12, 6);
        engine.clear();
        engine.resize(4, 6);
        let first_bounds = engine.viewport_bounds().expect("first narrow viewport");
        let first_wall = first_bounds.x_start;
        fill_supported_column(&mut engine, first_wall, 4, CategoryId::new(1));
        engine.grain_count = 4;

        engine.resize(6, 6);
        assert_eq!(engine.boundary_release_fronts.len(), 1);
        assert_eq!(engine.boundary_release_fronts[0].wall_x, first_wall);

        engine.resize(2, 6);
        let second_bounds = engine.viewport_bounds().expect("second narrow viewport");
        let second_wall = second_bounds.x_start;
        fill_supported_column(&mut engine, second_wall, 4, CategoryId::new(2));
        engine.grain_count = 8;

        engine.resize(4, 6);
        assert_eq!(engine.boundary_release_fronts.len(), 2);
        assert_eq!(engine.boundary_release_fronts[0].wall_x, first_wall);
        assert_eq!(engine.boundary_release_fronts[1].wall_x, second_wall);
        let second_surface_y = engine.grid_height_dots - 4;

        engine.apply_gravity();

        assert_eq!(
            engine.grid[second_surface_y][second_wall],
            None,
            "a hidden older release must not block the newly visible former wall"
        );
        assert_eq!(
            engine.grid[second_surface_y + 1][second_wall - 1],
            Some(CategoryId::new(2))
        );
        assert_eq!(engine.grain_count, 8);
    }

    #[test]
    fn incremental_reexpansion_normalizes_overlapping_same_side_release_obligations() {
        let mut engine = SandEngine::new(12, 6);
        engine.clear();
        engine.resize(4, 6);
        let first_bounds = engine.viewport_bounds().expect("first narrow viewport");
        let first_wall = first_bounds.x_start;
        fill_supported_column(&mut engine, first_wall, 4, CategoryId::new(1));
        fill_supported_column(&mut engine, first_wall + 1, 5, CategoryId::new(1));
        engine.grain_count = 9;

        engine.resize(5, 6);
        assert_eq!(engine.boundary_release_fronts.len(), 1);
        let first_front = engine.boundary_release_fronts[0];

        engine.resize(4, 6);
        let second_bounds = engine.viewport_bounds().expect("second narrow viewport");
        let second_wall = second_bounds.x_start;
        assert!(second_wall >= first_front.wall_x);
        fill_supported_column(&mut engine, second_wall, 6, CategoryId::new(2));
        engine.grain_count = engine.physical_grain_count();

        engine.resize(6, 6);

        let left_fronts = engine
            .boundary_release_fronts
            .iter()
            .filter(|front| front.direction == BoundaryReleaseDirection::Left)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(left_fronts.len(), 1, "overlapping incremental releases must normalize");
        assert!(left_fronts[0].wall_x <= left_fronts[0].front_x);
        assert_eq!(engine.grain_count, engine.physical_grain_count());
    }

    #[test]
    fn later_canonical_growth_shifts_active_release_front_and_in_flight_grain_exactly() {
        let mut engine = SandEngine::new(4, 4);
        engine.clear();
        fill_supported_column(&mut engine, 0, 4, CategoryId::new(1));
        engine.grain_count = 4;

        engine.resize(8, 4);
        engine.apply_gravity();
        let before_front = engine.boundary_release_fronts[0];
        let before_in_flight = engine
            .boundary_release_in_flight
            .expect("released grain must still be in flight");
        let width_before_second_growth = engine.grid_width_dots;

        engine.resize(12, 4);
        let second_offset = (engine.grid_width_dots - width_before_second_growth) / 2;
        assert_eq!(engine.grain_count, 4);
        assert_eq!(engine.boundary_release_fronts.len(), 1);
        assert_eq!(
            engine.boundary_release_fronts[0],
            SandStateBoundaryReleaseFront {
                direction: before_front.direction,
                wall_x: before_front.wall_x + second_offset,
                front_x: before_front.front_x + second_offset,
            }
        );
        assert_eq!(
            engine.boundary_release_in_flight,
            Some(SandStateCoordinate {
                x: before_in_flight.x + second_offset,
                y: before_in_flight.y,
            })
        );
    }

    #[test]
    fn viewport_render_size_tracks_terminal_while_canvas_keeps_maximum_extent() {
        let mut engine = SandEngine::new(4, 2);
        engine.resize(9, 5);
        let expanded = engine.render(&[]);
        assert_eq!(expanded.len(), 5);
        assert!(expanded.iter().all(|line| line.spans.len() == 9));
        let logical = (engine.grid_width_dots, engine.grid_height_dots);

        engine.resize(1, 1);
        let shrunk = engine.render(&[]);
        assert_eq!(shrunk.len(), 1);
        assert_eq!(shrunk[0].spans.len(), 1);
        assert_eq!((engine.grid_width_dots, engine.grid_height_dots), logical);
    }

    #[test]
    fn pending_spawn_uses_only_visible_top_ingress_after_shrink() {
        let mut engine = SandEngine::new(8, 4);
        engine.resize(12, 6);
        engine.resize(4, 2);
        let bounds = engine.viewport_bounds().expect("visible viewport");

        for _ in 0..bounds.x_end.saturating_sub(bounds.x_start) {
            engine.spawn(CategoryId::new(1));
        }

        assert!(
            engine.grid[..bounds.y_start]
                .iter()
                .flatten()
                .all(Option::is_none)
        );
        assert!(
            engine.grid[bounds.y_start][bounds.x_start..bounds.x_end]
                .iter()
                .all(|cell| *cell == Some(CategoryId::new(1)))
        );
    }

    #[test]
    fn visible_side_walls_block_diagonal_leakage() {
        let mut engine = SandEngine::new(8, 4);
        engine.resize(12, 6);
        engine.resize(4, 2);
        let bounds = engine.viewport_bounds().expect("visible viewport");
        let y = bounds.y_end - 2;
        engine.grid[y][bounds.x_start] = Some(CategoryId::new(1));
        engine.grid[y + 1][bounds.x_start] = Some(CategoryId::new(2));
        engine.rng_state = 2;
        engine.apply_gravity();

        assert_eq!(engine.grid[y][bounds.x_start], Some(CategoryId::new(1)));
        assert!(
            engine.grid[y + 1][..bounds.x_start]
                .iter()
                .all(Option::is_none)
        );
    }

    #[test]
    fn hidden_grains_freeze_and_fall_after_reexpansion() {
        let mut engine = SandEngine::new(8, 4);
        engine.resize(12, 6);
        let x = 4;
        engine.grid[5][x] = Some(CategoryId::new(1));
        engine.grain_count = 1;

        engine.resize(4, 2);
        for _ in 0..6 {
            engine.update();
        }
        assert_eq!(engine.grid[5][x], Some(CategoryId::new(1)));

        engine.resize(12, 6);
        engine.update();
        engine.update();
        assert_eq!(engine.grid[5][x], None);
        assert_eq!(engine.grid[6][x], Some(CategoryId::new(1)));
    }

    #[test]
    fn clear_resets_empty_canvas_to_current_viewport() {
        let mut engine = SandEngine::new(4, 2);
        engine.resize(12, 6);
        engine.resize(3, 1);
        engine.spawn(CategoryId::new(1));

        engine.clear();

        assert_eq!(engine.grid_width_dots, 6);
        assert_eq!(engine.grid_height_dots, 4);
        assert_eq!(engine.grain_count, 0);
    }

    #[test]
    fn test_sand_state_snapshot_restore_round_trip() {
        let mut se = SandEngine::new(20, 20);
        se.clear();
        se.grid[3][2] = Some(CategoryId::new(1));
        se.grid[10][7] = Some(CategoryId::new(2));
        se.grain_count = 2;

        let state = se.snapshot_state();

        let mut restored = SandEngine::new(20, 20);
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)]);
        restored.restore_state(&state, &valid).unwrap();

        assert_eq!(restored.grid[3][2], Some(CategoryId::new(1)));
        assert_eq!(restored.grid[10][7], Some(CategoryId::new(2)));
        assert_eq!(restored.grain_count, 2);
    }

    #[test]
    fn test_sand_state_restore_rejects_unknown_category_without_mutation() {
        let state = super::SandState {
            version: super::SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: vec![super::SandStateGrain {
                x: 1,
                y: 1,
                category_id: 99,
            }],
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 1,
            ingress_focus_x: None,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
            active_avalanche_columns: Vec::new(),
            mobilized_grains: Vec::new(),
            boundary_release_fronts: Vec::new(),
            boundary_release_in_flight: None,
        };

        let mut restored = SandEngine::new(20, 20);
        let before = restored.snapshot_state();
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let error = restored.restore_state(&state, &valid).unwrap_err();

        assert!(error.contains("unknown category ID 99"));
        assert_eq!(restored.snapshot_state(), before);
    }

    #[test]
    fn test_sand_state_restore_preserves_canonical_grid_on_different_viewport() {
        let mut source = SandEngine::new(20, 20);
        source.clear();
        source.grid[2][2] = Some(CategoryId::new(1));
        source.grid[20][20] = Some(CategoryId::new(2));
        source.grain_count = 2;
        source.ingress_focus_x = Some(3);
        let state = source.snapshot_state();

        let mut restored = SandEngine::new(40, 40);
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)]);
        restored.restore_state(&state, &valid).unwrap();

        assert_eq!(restored.cell_width, 40);
        assert_eq!(restored.cell_height, 40);
        assert_eq!(
            restored.grid_width_dots,
            40 * crate::constants::SAND_ENGINE.dot_width
        );
        assert_eq!(
            restored.grid_height_dots,
            40 * crate::constants::SAND_ENGINE.dot_height
        );
        assert_eq!(restored.grain_count, 2);
        let horizontal_offset = (restored.grid_width_dots - state.grid_width) / 2;
        assert_eq!(restored.ingress_focus_x, Some(3 + horizontal_offset));
        assert_eq!(category_mass(&restored, CategoryId::new(1)), 1);
        assert_eq!(category_mass(&restored, CategoryId::new(2)), 1);
    }

    #[test]
    fn zero_viewport_restore_preserves_persisted_canonical_extent() {
        let mut source = SandEngine::new(7, 3);
        source.clear();
        source.grid[2][3] = Some(CategoryId::new(1));
        source.grain_count = 1;
        let state = source.snapshot_state();

        let mut restored = SandEngine::new(0, 0);
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        restored.restore_state(&state, &valid).unwrap();

        assert_eq!(restored.grid_width_dots, state.grid_width);
        assert_eq!(restored.grid_height_dots, state.grid_height);
        assert_eq!(restored.snapshot_state(), state);
    }

    #[test]
    fn test_clear_category_removes_only_requested_id() {
        let mut se = SandEngine::new(20, 20);
        se.clear();
        se.resize(40, 40);
        se.resize(5, 5);
        let canonical_extent = (se.grid_width_dots, se.grid_height_dots);
        se.grid[1][1] = Some(CategoryId::new(0));
        se.grid[2][2] = Some(CategoryId::new(0));
        se.grid[3][3] = Some(CategoryId::new(1));
        se.grain_count = 3;

        se.clear_category(CategoryId::new(0));

        assert_eq!(se.grid[1][1], None);
        assert_eq!(se.grid[2][2], None);
        assert_eq!(se.grid[3][3], Some(CategoryId::new(1)));
        assert_eq!(se.grain_count, 1);
        assert_eq!((se.grid_width_dots, se.grid_height_dots), canonical_extent);
    }

    #[test]
    fn test_remove_category_grains_respects_count_and_category() {
        let mut se = SandEngine::new(20, 20);
        se.clear();
        se.grid[0][0] = Some(CategoryId::new(1));
        se.grid[0][1] = Some(CategoryId::new(1));
        se.grid[0][2] = Some(CategoryId::new(1));
        se.grid[0][3] = Some(CategoryId::new(2));
        se.grain_count = 4;

        let removed = se.remove_category_grains(CategoryId::new(1), 2);

        assert_eq!(removed, 2);
        assert_eq!(
            se.grid
                .iter()
                .flat_map(|row| row.iter())
                .filter(|cell| **cell == Some(CategoryId::new(1)))
                .count(),
            1
        );
        assert_eq!(
            se.grid
                .iter()
                .flat_map(|row| row.iter())
                .filter(|cell| **cell == Some(CategoryId::new(2)))
                .count(),
            1
        );
        assert_eq!(se.grain_count, 2);
    }

    #[test]
    fn test_sand_state_snapshot_round_trips_engine_metadata() {
        let mut se = SandEngine::new(10, 10);
        se.clear();
        se.grid[0][0] = Some(CategoryId::new(1));
        se.grain_count = 1;
        se.frame_count = 9;
        se.sweep_left_to_right = false;
        se.rng_state = 0xABCD_1234;

        let state = se.snapshot_state();
        assert_eq!(state.frame_count, 9);
        assert!(!state.sweep_left_to_right);
        assert_eq!(state.rng_state, 0xABCD_1234);

        let mut restored = SandEngine::new(10, 10);
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        restored.restore_state(&state, &valid).unwrap();

        assert_eq!(restored.frame_count, 9);
        assert!(!restored.sweep_left_to_right);
        assert_eq!(restored.rng_state, 0xABCD_1234);
    }

    #[test]
    fn test_apply_gravity_alternates_horizontal_sweep_direction() {
        let mut se = SandEngine::new(10, 10);
        let initial = se.sweep_left_to_right;

        se.apply_gravity();
        let after_first = se.sweep_left_to_right;

        se.apply_gravity();
        let after_second = se.sweep_left_to_right;

        assert_ne!(after_first, initial);
        assert_eq!(after_second, initial);
    }
}

#[cfg(test)]
mod organic_formation_tests {
    use std::collections::HashSet;

    use crate::{
        domain::CategoryId,
        sand::{SandEngine, SandState, SandStateCoordinate},
    };

    fn set_supported_profile(engine: &mut SandEngine, x_start: usize, heights: &[usize]) {
        engine.clear();
        let bottom = engine.grid_height_dots;
        let mut mass = 0usize;
        for (offset, &height) in heights.iter().enumerate() {
            let x = x_start + offset;
            for y in bottom - height..bottom {
                engine.grid[y][x] = Some(CategoryId::new(1));
                mass += 1;
            }
        }
        engine.grain_count = mass;
    }

    #[test]
    fn gravity_down_open_remains_unconditional_and_consumes_no_rng() {
        let mut engine = SandEngine::new(3, 2);
        engine.clear();
        let y = engine.grid_height_dots - 3;
        let x = engine.grid_width_dots / 2;
        engine.grid[y][x] = Some(CategoryId::new(1));
        engine.grain_count = 1;
        engine.rng_state = 17;

        engine.apply_gravity();

        assert_eq!(engine.grid[y][x], None);
        assert_eq!(engine.grid[y + 1][x], Some(CategoryId::new(1)));
        assert_eq!(engine.rng_state, 17);
        assert!(!engine.mobilized[y + 1][x]);
    }

    #[test]
    fn ordinary_ingress_starts_non_mobilized_while_airborne() {
        let mut engine = SandEngine::new(8, 2);
        engine.clear();
        let bounds = engine.viewport_bounds().unwrap();

        engine.spawn(CategoryId::new(1));

        let x = (bounds.x_start..bounds.x_end)
            .find(|&x| engine.grid[bounds.y_start][x].is_some())
            .unwrap();
        assert!(!engine.mobilized[bounds.y_start][x]);
    }

    #[test]
    fn newly_landed_two_dot_spire_cannot_remain_settled() {
        let mut engine = SandEngine::new(4, 2);
        engine.clear();
        let x = engine.grid_width_dots / 2;
        let bottom = engine.grid_height_dots - 1;
        engine.grid[bottom][x] = Some(CategoryId::new(1));
        engine.grid[bottom - 2][x] = Some(CategoryId::new(1));
        engine.grain_count = 2;

        engine.apply_gravity();

        assert_eq!(engine.grid[bottom - 1][x], None);
        assert!(engine.last_diagonal_topple);
        assert_eq!(engine.physical_grain_count(), 2);
    }

    #[test]
    fn newly_landed_zero_six_five_profile_is_contact_supported() {
        let mut engine = SandEngine::new(5, 2);
        engine.clear();
        let x = engine.grid_width_dots / 2;
        let bottom = engine.grid_height_dots - 1;
        for y in bottom - 4..=bottom {
            engine.grid[y][x] = Some(CategoryId::new(1));
            engine.grid[y][x + 1] = Some(CategoryId::new(2));
        }
        engine.grid[bottom - 6][x] = Some(CategoryId::new(1));
        engine.grain_count = 11;

        engine.apply_gravity();

        assert_eq!(engine.grid[bottom - 5][x], Some(CategoryId::new(1)));
        assert!(!engine.mobilized[bottom - 5][x]);
        assert!(!engine.last_diagonal_topple);
        assert_eq!(engine.physical_grain_count(), 11);
    }

    #[test]
    fn unrelated_vertical_rain_near_mobilized_grain_never_inherits_mobility() {
        let mut engine = SandEngine::new(5, 3);
        engine.clear();
        let bounds = engine.viewport_bounds().unwrap();
        let x = bounds.x_start + 3;
        let y = bounds.y_start + 1;
        engine.grid[y][x] = Some(CategoryId::new(1));
        engine.grid[y][x + 1] = Some(CategoryId::new(2));
        engine.mobilized[y][x] = true;
        engine.grain_count = 2;

        engine.apply_gravity();

        assert!(engine.mobilized[y + 1][x]);
        assert!(!engine.mobilized[y + 1][x + 1]);
    }

    #[test]
    fn support_loss_wakes_only_exact_dependents_not_a_radius() {
        let mut engine = SandEngine::new(6, 3);
        engine.clear();
        let bounds = engine.viewport_bounds().unwrap();
        let x = bounds.x_start + 4;
        let y = bounds.y_start + 5;

        engine.grid[y - 1][x] = Some(CategoryId::new(1));
        engine.grid[y - 1][x + 2] = Some(CategoryId::new(2));
        engine.grain_count = 2;

        engine.mobilize_dependents_after_vacancy(bounds, x, y);

        assert!(engine.mobilized[y - 1][x]);
        assert!(!engine.mobilized[y - 1][x + 2]);
    }

    #[test]
    fn preexisting_unsupported_shape_is_metastable_without_causal_touch() {
        let mut engine = SandEngine::new(4, 2);
        let x = engine.grid_width_dots / 2;
        set_supported_profile(&mut engine, x, &[2]);
        let before = engine.snapshot_state();

        for _ in 0..10_000 {
            engine.apply_gravity();
        }

        assert_eq!(engine.snapshot_state(), before);
        assert!(engine.mobilized.iter().flatten().all(|mobile| !*mobile));
    }

    #[test]
    fn dynamic_relief_one_stops_and_relief_two_yields() {
        let mut stable = SandEngine::new(5, 2);
        let x = stable.grid_width_dots / 2;
        set_supported_profile(&mut stable, x - 1, &[1, 2, 1]);
        stable.mobilized[stable.grid_height_dots - 2][x] = true;
        stable.apply_gravity();
        assert!(!stable.last_diagonal_topple);
        assert!(!stable.mobilized[stable.grid_height_dots - 2][x]);

        let mut yielding = SandEngine::new(5, 2);
        let x = yielding.grid_width_dots / 2;
        set_supported_profile(&mut yielding, x - 1, &[0, 2, 0]);
        yielding.mobilized[yielding.grid_height_dots - 2][x] = true;
        yielding.apply_gravity();
        assert!(yielding.last_diagonal_topple);
    }

    #[test]
    fn mobilized_topple_passes_mobility_to_exposed_dynamic_slip_surface() {
        let mut engine = SandEngine::new(5, 2);
        let x = engine.grid_width_dots / 2;
        set_supported_profile(&mut engine, x - 1, &[0, 4, 0]);
        let source_y = engine.grid_height_dots - 4;
        engine.mobilized[source_y][x] = true;

        engine.apply_gravity();

        assert!(engine.last_diagonal_topple);
        let exposed_y = source_y + 1;
        assert_eq!(engine.grid[exposed_y][x], Some(CategoryId::new(1)));
        assert!(engine.mobilized[exposed_y][x]);
        engine.apply_gravity();
        assert!(engine.last_diagonal_topple);
        assert_eq!(engine.physical_grain_count(), 4);
    }

    #[test]
    fn two_dot_peak_does_not_overpropagate_slip_lineage() {
        let mut engine = SandEngine::new(5, 2);
        let x = engine.grid_width_dots / 2;
        set_supported_profile(&mut engine, x - 1, &[0, 2, 0]);
        let source_y = engine.grid_height_dots - 2;
        engine.mobilized[source_y][x] = true;

        engine.apply_gravity();

        assert!(engine.last_diagonal_topple);
        let exposed_y = source_y + 1;
        assert_eq!(engine.grid[exposed_y][x], Some(CategoryId::new(1)));
        assert!(!engine.mobilized[exposed_y][x]);
        assert_eq!(engine.physical_grain_count(), 2);
    }

    #[test]
    fn diagonal_topple_updates_supported_heights_without_airborne_inflation() {
        let mut relief_two = SandEngine::new(5, 2);
        let x = relief_two.grid_width_dots / 2;
        set_supported_profile(&mut relief_two, x - 1, &[0, 2, 0]);
        relief_two.derive_supported_heights(relief_two.viewport_bounds().unwrap());
        relief_two.mobilized[relief_two.grid_height_dots - 2][x] = true;
        relief_two.apply_gravity();
        assert_eq!(relief_two.supported_heights[x], 1);
        assert_eq!(
            relief_two.supported_heights[x - 1].max(relief_two.supported_heights[x + 1]),
            1
        );

        let mut relief_three = SandEngine::new(5, 2);
        let x = relief_three.grid_width_dots / 2;
        set_supported_profile(&mut relief_three, x - 1, &[0, 3, 0]);
        relief_three.derive_supported_heights(relief_three.viewport_bounds().unwrap());
        relief_three.mobilized[relief_three.grid_height_dots - 3][x] = true;
        relief_three.apply_gravity();
        assert_eq!(relief_three.supported_heights[x], 2);
        assert_eq!(relief_three.supported_heights[x - 1], 0);
    }

    #[test]
    fn airborne_grains_do_not_inflate_supported_surface_height() {
        let mut engine = SandEngine::new(4, 2);
        engine.clear();
        let bounds = engine.viewport_bounds().unwrap();
        let x = bounds.x_start + 2;
        engine.grid[bounds.y_end - 1][x] = Some(CategoryId::new(1));
        engine.grid[bounds.y_start][x] = Some(CategoryId::new(2));

        engine.derive_supported_heights(bounds);

        assert_eq!(engine.supported_heights[x], 1);
    }

    #[test]
    fn v6_snapshot_restore_preserves_exact_mobilized_grain_state() {
        let mut source = SandEngine::new(5, 2);
        let x = source.grid_width_dots / 2;
        set_supported_profile(&mut source, x - 1, &[0, 4, 0]);
        let source_y = source.grid_height_dots - 4;
        source.mobilized[source_y][x] = true;
        source.rng_state = 0x1234_5678;

        let state = source.snapshot_state();
        assert_eq!(state.version, SandState::VERSION);
        assert_eq!(
            state.mobilized_grains,
            vec![SandStateCoordinate { x, y: source_y }]
        );
        assert!(state.active_avalanche_columns.is_empty());

        let valid = HashSet::from([CategoryId::new(1)]);
        let mut restored = SandEngine::new(5, 2);
        restored.restore_state(&state, &valid).unwrap();
        assert_eq!(restored.snapshot_state(), state);

        for _ in 0..8 {
            source.apply_gravity();
            restored.apply_gravity();
            assert_eq!(restored.snapshot_state(), source.snapshot_state());
        }
    }

    #[test]
    fn v5_restore_upgrades_exact_mobility_without_inventing_boundary_release() {
        let mut source = SandEngine::new(5, 2);
        let x = source.grid_width_dots / 2;
        set_supported_profile(&mut source, x - 1, &[0, 4, 0]);
        let source_y = source.grid_height_dots - 4;
        source.mobilized[source_y][x] = true;
        source.rng_state = 0x1234_5678;

        let mut v5 = source.snapshot_state();
        v5.version = SandState::GRAIN_CAUSAL_VERSION;
        v5.boundary_release_fronts.clear();
        v5.boundary_release_in_flight = None;

        let valid = HashSet::from([CategoryId::new(1)]);
        let mut restored = SandEngine::new(5, 2);
        restored.restore_state(&v5, &valid).unwrap();
        let migrated = restored.snapshot_state();

        assert_eq!(migrated.version, SandState::VERSION);
        assert_eq!(migrated.mobilized_grains, v5.mobilized_grains);
        assert!(migrated.boundary_release_fronts.is_empty());
        assert!(migrated.boundary_release_in_flight.is_none());
        assert_eq!(migrated.rng_state, v5.rng_state);
    }

    #[test]
    fn v4_migration_discards_regional_activity_and_seeds_only_unsupported_surface() {
        let mut legacy = SandEngine::new(8, 2);
        legacy.clear();
        let bottom = legacy.grid_height_dots;
        let stable_x = legacy.grid_width_dots / 2;
        let unstable_x = stable_x - 4;

        for y in bottom - 6..bottom {
            legacy.grid[y][stable_x] = Some(CategoryId::new(1));
        }
        for y in bottom - 5..bottom {
            legacy.grid[y][stable_x + 1] = Some(CategoryId::new(1));
        }
        for y in bottom - 2..bottom {
            legacy.grid[y][unstable_x] = Some(CategoryId::new(1));
        }
        legacy.grain_count = 13;

        let mut state = legacy.snapshot_state();
        state.version = SandState::REGIONAL_AVALANCHE_VERSION;
        state.active_avalanche_columns = vec![stable_x];
        state.mobilized_grains.clear();

        let valid = HashSet::from([CategoryId::new(1)]);
        let mut restored = SandEngine::new(8, 2);
        restored.restore_state(&state, &valid).unwrap();

        assert!(restored.mobilized[bottom - 2][unstable_x]);
        assert!(
            !restored.mobilized[bottom - 6][stable_x],
            "obsolete v4 regional activity must not become false grain causality"
        );

        let migrated = restored.snapshot_state();
        assert_eq!(migrated.version, SandState::VERSION);
        assert!(migrated.active_avalanche_columns.is_empty());
        assert_eq!(
            migrated.mobilized_grains,
            vec![SandStateCoordinate {
                x: unstable_x,
                y: bottom - 2,
            }]
        );
    }

    #[test]
    fn v6_restore_rejects_malformed_mobilized_coordinates_without_mutation() {
        let mut source = SandEngine::new(5, 2);
        let x = source.grid_width_dots / 2;
        set_supported_profile(&mut source, x, &[2]);
        let mut state = source.snapshot_state();
        state.mobilized_grains = vec![
            SandStateCoordinate {
                x,
                y: source.grid_height_dots - 1,
            },
            SandStateCoordinate {
                x,
                y: source.grid_height_dots - 2,
            },
        ];

        let valid = HashSet::from([CategoryId::new(1)]);
        let mut restored = SandEngine::new(5, 2);
        let before = restored.snapshot_state();
        assert!(restored.restore_state(&state, &valid).is_err());
        assert_eq!(restored.snapshot_state(), before);
    }

    #[test]
    fn mobilized_coordinates_shift_with_canonical_growth() {
        let mut engine = SandEngine::new(4, 2);
        engine.clear();
        let old_width = engine.grid_width_dots;
        engine.grid[3][2] = Some(CategoryId::new(1));
        engine.mobilized[3][2] = true;
        engine.grain_count = 1;

        engine.resize(8, 4);

        let x_offset = (engine.grid_width_dots - old_width) / 2;
        let y_offset = engine.grid_height_dots - 8;
        assert_eq!(
            engine.grid[3 + y_offset][2 + x_offset],
            Some(CategoryId::new(1))
        );
        assert!(engine.mobilized[3 + y_offset][2 + x_offset]);
    }

    #[test]
    fn category_clear_propagates_only_from_removed_support_cells() {
        let mut engine = SandEngine::new(5, 2);
        engine.clear();
        let bottom = engine.grid_height_dots - 1;
        let x = engine.grid_width_dots / 2;
        engine.grid[bottom][x] = Some(CategoryId::new(1));
        engine.grid[bottom - 1][x] = Some(CategoryId::new(2));
        engine.grid[bottom][x + 2] = Some(CategoryId::new(2));
        engine.grain_count = 3;

        engine.clear_category(CategoryId::new(1));

        assert!(engine.mobilized[bottom - 1][x]);
        assert!(!engine.mobilized[bottom][x + 2]);
        assert_eq!(engine.grain_count, 2);
    }

    #[test]
    fn v6_restart_preserves_hidden_mobility_and_continuation_exactly() {
        let mut source = SandEngine::new(8, 4);
        source.clear();
        let x = source.grid_width_dots / 2;
        let y = source.grid_height_dots - 3;
        source.grid[y][x] = Some(CategoryId::new(1));
        source.grid[y + 1][x] = Some(CategoryId::new(1));
        source.mobilized[y][x] = true;
        source.grain_count = 2;
        source.add_logical_grains(CategoryId::new(2), 3).unwrap();
        source.frame_count = 7;
        source.sweep_left_to_right = false;
        source.rng_state = 0x1234_5678;
        source.ingress_focus_x = Some(x);

        let state = source.snapshot_state();
        let valid = HashSet::from([CategoryId::new(1), CategoryId::new(2)]);
        let mut restored = SandEngine::new(2, 2);
        restored.restore_state(&state, &valid).unwrap();
        assert_eq!(restored.snapshot_state(), state);

        source.resize(2, 2);
        restored.resize(2, 2);
        assert_eq!(restored.snapshot_state(), source.snapshot_state());
        source.resize(8, 4);
        restored.resize(8, 4);
        for _ in 0..8 {
            source.apply_gravity();
            restored.apply_gravity();
            assert_eq!(restored.snapshot_state(), source.snapshot_state());
        }
    }

    #[test]
    fn v6_restore_rejects_each_dynamic_state_invariant_without_mutation() {
        let mut source = SandEngine::new(4, 4);
        source.clear();
        source.grid[1][1] = Some(CategoryId::new(1));
        source.grid[2][2] = Some(CategoryId::new(1));
        source.mobilized[1][1] = true;
        source.grain_count = 2;
        let valid = HashSet::from([CategoryId::new(1)]);
        let base = source.snapshot_state();

        let mut malformed_states = Vec::new();
        let mut unsorted = base.clone();
        unsorted.mobilized_grains = vec![
            SandStateCoordinate { x: 1, y: 2 },
            SandStateCoordinate { x: 1, y: 1 },
        ];
        malformed_states.push(unsorted);
        let mut duplicate = base.clone();
        duplicate.mobilized_grains = vec![
            SandStateCoordinate { x: 1, y: 1 },
            SandStateCoordinate { x: 1, y: 1 },
        ];
        malformed_states.push(duplicate);
        let mut out_of_bounds = base.clone();
        out_of_bounds.mobilized_grains = vec![SandStateCoordinate { x: 4, y: 1 }];
        malformed_states.push(out_of_bounds);
        let mut empty = base.clone();
        empty.mobilized_grains = vec![SandStateCoordinate { x: 0, y: 0 }];
        malformed_states.push(empty);
        let mut legacy_conflict = base.clone();
        legacy_conflict.active_avalanche_columns = vec![1];
        malformed_states.push(legacy_conflict);
        let mut pre_v5_conflict = base.clone();
        pre_v5_conflict.version = SandState::REGIONAL_AVALANCHE_VERSION;
        malformed_states.push(pre_v5_conflict);

        let mut overlapping_release_domains = base.clone();
        overlapping_release_domains.boundary_release_fronts = vec![
            SandStateBoundaryReleaseFront {
                direction: BoundaryReleaseDirection::Left,
                wall_x: 1,
                front_x: 2,
            },
            SandStateBoundaryReleaseFront {
                direction: BoundaryReleaseDirection::Left,
                wall_x: 3,
                front_x: 3,
            },
        ];
        malformed_states.push(overlapping_release_domains);

        let mut unsorted_release_fronts = base.clone();
        unsorted_release_fronts.boundary_release_fronts = vec![
            SandStateBoundaryReleaseFront {
                direction: BoundaryReleaseDirection::Right,
                wall_x: 2,
                front_x: 2,
            },
            SandStateBoundaryReleaseFront {
                direction: BoundaryReleaseDirection::Left,
                wall_x: 1,
                front_x: 1,
            },
        ];
        malformed_states.push(unsorted_release_fronts);

        let mut reversed_release_front = base.clone();
        reversed_release_front.boundary_release_fronts = vec![SandStateBoundaryReleaseFront {
            direction: BoundaryReleaseDirection::Left,
            wall_x: 2,
            front_x: 1,
        }];
        malformed_states.push(reversed_release_front);

        let mut release_without_target = base.clone();
        release_without_target.boundary_release_fronts = vec![SandStateBoundaryReleaseFront {
            direction: BoundaryReleaseDirection::Left,
            wall_x: 0,
            front_x: 0,
        }];
        malformed_states.push(release_without_target);

        let mut pre_v6_release = base.clone();
        pre_v6_release.version = SandState::GRAIN_CAUSAL_VERSION;
        pre_v6_release.boundary_release_fronts = vec![SandStateBoundaryReleaseFront {
            direction: BoundaryReleaseDirection::Left,
            wall_x: 1,
            front_x: 1,
        }];
        malformed_states.push(pre_v6_release);

        let mut in_flight_without_front = base.clone();
        in_flight_without_front.boundary_release_in_flight = Some(SandStateCoordinate { x: 1, y: 1 });
        malformed_states.push(in_flight_without_front);

        let mut in_flight_empty_coordinate = base.clone();
        in_flight_empty_coordinate.boundary_release_fronts = vec![SandStateBoundaryReleaseFront {
            direction: BoundaryReleaseDirection::Left,
            wall_x: 1,
            front_x: 1,
        }];
        in_flight_empty_coordinate.boundary_release_in_flight =
            Some(SandStateCoordinate { x: 0, y: 0 });
        malformed_states.push(in_flight_empty_coordinate);

        let mut pre_v6_in_flight = base.clone();
        pre_v6_in_flight.version = SandState::GRAIN_CAUSAL_VERSION;
        pre_v6_in_flight.boundary_release_fronts.clear();
        pre_v6_in_flight.boundary_release_in_flight =
            Some(SandStateCoordinate { x: 1, y: 1 });
        malformed_states.push(pre_v6_in_flight);

        let mut mobile_in_flight = base.clone();
        mobile_in_flight.boundary_release_fronts = vec![SandStateBoundaryReleaseFront {
            direction: BoundaryReleaseDirection::Right,
            wall_x: 0,
            front_x: 0,
        }];
        mobile_in_flight.boundary_release_in_flight = Some(SandStateCoordinate { x: 1, y: 1 });
        malformed_states.push(mobile_in_flight);

        let mut unrelated_in_flight = base.clone();
        unrelated_in_flight.boundary_release_fronts = vec![SandStateBoundaryReleaseFront {
            direction: BoundaryReleaseDirection::Left,
            wall_x: 1,
            front_x: 1,
        }];
        unrelated_in_flight.boundary_release_in_flight = Some(SandStateCoordinate { x: 2, y: 2 });
        malformed_states.push(unrelated_in_flight);

        for malformed in malformed_states {
            let mut target = SandEngine::new(4, 4);
            let before = target.snapshot_state();
            assert!(target.restore_state(&malformed, &valid).is_err());
            assert_eq!(target.snapshot_state(), before);
        }
    }

    #[test]
    fn v6_restore_does_not_normalize_empty_mobility() {
        let mut source = SandEngine::new(4, 2);
        let x = source.grid_width_dots / 2;
        set_supported_profile(&mut source, x, &[2]);
        let state = source.snapshot_state();
        let mut restored = SandEngine::new(4, 2);
        restored
            .restore_state(&state, &HashSet::from([CategoryId::new(1)]))
            .unwrap();
        assert!(restored.mobilized.iter().flatten().all(|mobile| !*mobile));
    }

    #[test]
    fn recolor_preserves_non_empty_mobility_coordinates() {
        let mut engine = SandEngine::new(4, 2);
        engine.clear();
        engine.grid[2][2] = Some(CategoryId::new(1));
        engine.mobilized[2][2] = true;
        engine.grain_count = 1;
        let before = engine.snapshot_state();
        let mut recolored = before.clone();
        super::recolor_state_category_mass(
            &mut recolored,
            CategoryId::new(1),
            CategoryId::new(2),
            1,
        );
        assert_eq!(recolored.mobilized_grains, before.mobilized_grains);
        assert_eq!(
            recolored.mobilized_grains,
            vec![SandStateCoordinate { x: 2, y: 2 }]
        );
    }

    #[test]
    #[ignore = "explicit H4R2C real-cadence behavior bench"]
    fn h4r2c_real_cadence_bench_40x20_and_80x30() {
        use std::time::Instant;

        #[derive(Default)]
        struct Episode {
            diagonal_moves: usize,
            lineage_mobilizations: usize,
            support_loss_mobilizations: usize,
        }

        fn percentile(values: &mut [usize], numerator: usize, denominator: usize) -> usize {
            if values.is_empty() {
                return 0;
            }
            values.sort_unstable();
            values[(values.len() - 1) * numerator / denominator]
        }

        fn run(width: u16, height: u16, ingress_count: usize) {
            let started = Instant::now();
            let mut engine = SandEngine::new(width, height);
            engine.rng_state = 0xC0FF_EE00_0000_0001 ^ u64::from(width);
            let mut episodes = Vec::new();
            let mut current = None;
            let mut quiet_between = Vec::new();
            let mut structural_ingresses = Vec::new();
            let mut quiet = 0;

            for ingress in 0..ingress_count {
                engine.spawn(CategoryId::new(1));
                let mut structural_this_ingress = false;
                let gravity_passes = if ingress % 8 < 5 { 15 } else { 16 };
                for _ in 0..gravity_passes {
                    engine.apply_gravity();
                    structural_this_ingress |= engine.last_slip_lineage_mobilizations > 0
                        || engine.last_support_loss_mobilizations > 0;
                    if engine.last_avalanche_motion {
                        let episode = current.get_or_insert_with(Episode::default);
                        episode.diagonal_moves += engine.last_mobilized_diagonal_moves;
                        episode.lineage_mobilizations += engine.last_slip_lineage_mobilizations;
                        episode.support_loss_mobilizations +=
                            engine.last_support_loss_mobilizations;
                        quiet = 0;
                    } else if let Some(episode) = current.take() {
                        episodes.push(episode);
                        quiet_between.push(quiet);
                        quiet = 1;
                    } else {
                        quiet += 1;
                    }
                }
                if structural_this_ingress {
                    structural_ingresses.push(ingress);
                }
            }
            if let Some(episode) = current.take() {
                episodes.push(episode);
            }

            let settlement = episodes
                .iter()
                .filter(|episode| {
                    episode.lineage_mobilizations == 0 && episode.support_loss_mobilizations == 0
                })
                .collect::<Vec<_>>();
            let slip = episodes
                .iter()
                .filter(|episode| episode.lineage_mobilizations > 0)
                .collect::<Vec<_>>();
            let multi_lineage = slip
                .iter()
                .filter(|episode| episode.lineage_mobilizations >= 2)
                .count();
            let support = episodes
                .iter()
                .filter(|episode| episode.support_loss_mobilizations > 0)
                .collect::<Vec<_>>();
            let mut slip_lineage = slip
                .iter()
                .map(|episode| episode.lineage_mobilizations)
                .collect::<Vec<_>>();
            let mut slip_moves = slip
                .iter()
                .map(|episode| episode.diagonal_moves)
                .collect::<Vec<_>>();
            let mut settlement_moves = settlement
                .iter()
                .map(|episode| episode.diagonal_moves)
                .collect::<Vec<_>>();
            let mass = engine.physical_grain_count() + engine.pending_grain_count();
            let total_lineage = slip
                .iter()
                .map(|episode| episode.lineage_mobilizations)
                .sum::<usize>();
            let total_support_loss = episodes
                .iter()
                .map(|episode| episode.support_loss_mobilizations)
                .sum::<usize>();
            let mut structural_quiet = structural_ingresses
                .windows(2)
                .map(|pair| pair[1].saturating_sub(pair[0]))
                .collect::<Vec<_>>();
            println!(
                "H4R2C {width}x{height}: elapsed={:?} episodes={} settlement={} slip={} support={} slip_per_1000={:.3} total_lineage={} multi_lineage={} slip_lineage_median={} slip_lineage_p95={} slip_lineage_max={} slip_moves_median={} slip_moves_p95={} slip_moves_max={} settlement_moves_median={} settlement_moves_p95={} quiet_median={} structural_quiet_median={} support_loss_total={} mass={mass}/{ingress_count}",
                started.elapsed(),
                episodes.len(),
                settlement.len(),
                slip.len(),
                support.len(),
                slip.len() as f64 * 1000.0 / ingress_count.max(1) as f64,
                total_lineage,
                multi_lineage,
                percentile(&mut slip_lineage, 1, 2),
                percentile(&mut slip_lineage, 19, 20),
                slip_lineage.iter().copied().max().unwrap_or(0),
                percentile(&mut slip_moves, 1, 2),
                percentile(&mut slip_moves, 19, 20),
                slip_moves.iter().copied().max().unwrap_or(0),
                percentile(&mut settlement_moves, 1, 2),
                percentile(&mut settlement_moves, 19, 20),
                percentile(&mut quiet_between, 1, 2),
                percentile(&mut structural_quiet, 1, 2),
                total_support_loss,
            );
            assert_eq!(mass, ingress_count);
        }

        let ingress_count = std::env::var("H4R2C_INGRESS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(10_000);
        run(40, 20, ingress_count);
        run(80, 30, ingress_count);
    }

    #[test]
    fn drained_surface_preserves_supported_one_step_prominence() {
        for (width, height) in [(40, 20), (80, 30)] {
            let mut engine = SandEngine::new(width, height);
            engine.rng_state = 0xC0FF_EE00_0000_0001 ^ u64::from(width);
            for _ in 0..2_000 {
                engine.spawn(CategoryId::new(1));
                for _ in 0..15 {
                    engine.apply_gravity();
                }
            }
            for _ in 0..10_000 {
                engine.apply_gravity();
                if !engine.mobilized.iter().flatten().any(|mobile| *mobile) {
                    break;
                }
            }
            let bounds = engine.viewport_bounds().unwrap();
            engine.derive_supported_heights(bounds);
            for x in bounds.x_start + 1..bounds.x_end - 1 {
                assert!(
                    engine.supported_heights[x]
                        <= engine.supported_heights[x - 1].max(engine.supported_heights[x + 1]) + 1
                );
            }
            let excerpt = engine.supported_heights[bounds.x_start..bounds.x_end]
                .iter()
                .copied()
                .take(24)
                .collect::<Vec<_>>();
            println!("H4R2C settled {width}x{height} supported-height excerpt: {excerpt:?}");
        }
    }
}

#[cfg(test)]
mod conservation_tests {
    use std::collections::HashSet;

    use crate::{domain::CategoryId, sand::SandEngine};

    #[test]
    fn spawn_scans_every_ingress_column_before_blocking() {
        let mut engine = SandEngine::new(4, 2);
        let ingress_width = engine.grid[0].len();
        for x in 0..ingress_width - 1 {
            engine.grid[0][x] = Some(CategoryId::new(1));
        }
        engine.grain_count = ingress_width - 1;

        engine.spawn(CategoryId::new(2));

        assert_eq!(engine.grid[0][ingress_width - 1], Some(CategoryId::new(2)));
        assert_eq!(engine.pending_grain_count(), 0);
        assert_eq!(engine.grain_count, ingress_width);
    }

    #[test]
    fn blocked_spawn_remains_logical_until_ingress_reopens() {
        let mut engine = SandEngine::new(3, 2);
        let ingress_width = engine.grid[0].len();
        for x in 0..ingress_width {
            engine.grid[0][x] = Some(CategoryId::new(1));
        }
        engine.grain_count = ingress_width;

        engine.spawn(CategoryId::new(2));
        assert_eq!(engine.pending_grain_count(), 1);
        assert_eq!(engine.grain_count, ingress_width + 1);

        let displaced = engine.grid[0][0].take().expect("occupied ingress");
        engine.grid[1][0] = Some(displaced);
        engine.update();

        assert_eq!(engine.pending_grain_count(), 0);
        assert_eq!(engine.physical_grain_count(), ingress_width + 1);
        assert_eq!(engine.grain_count, ingress_width + 1);
    }

    #[test]
    fn render_uses_terminal_cell_dimensions_exactly() {
        let engine = SandEngine::new(3, 2);
        let lines = engine.render(&[]);

        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.spans.len() == 3));
        assert_eq!(engine.grid_width_dots, 3 * 2);
        assert_eq!(engine.grid_height_dots, 2 * 4);
    }

    #[test]
    fn pending_grains_round_trip_with_category_identity() {
        let mut engine = SandEngine::new(2, 1);
        for cell in &mut engine.grid[0] {
            *cell = Some(CategoryId::new(1));
        }
        engine.grain_count = engine.grid[0].len();
        engine.spawn(CategoryId::new(2));

        let state = engine.snapshot_state();
        assert!(state.pending_grains.is_empty());
        assert_eq!(
            state.pending_runs,
            vec![super::PendingGrainRun {
                category_id: 2,
                count: 1,
            }]
        );

        let mut restored = SandEngine::new(2, 1);
        restored
            .restore_state(
                &state,
                &HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)]),
            )
            .unwrap();

        assert_eq!(restored.pending_grain_count(), 1);
        assert_eq!(restored.grain_count, state.grains.len() + 1);
        assert_eq!(restored.snapshot_state().pending_runs, state.pending_runs);
    }

    #[cfg(test)]
    mod compressed_mass_tests {
        use std::collections::HashSet;

        use crate::domain::CategoryId;
        use crate::sand::{PendingGrainRun, SandEngine, SandState, SandStateGrain};

        #[test]
        fn billion_grains_use_one_pending_run() {
            let mut engine = SandEngine::new(1, 1);
            for cell in &mut engine.grid[0] {
                *cell = Some(CategoryId::new(0));
            }
            engine.grain_count = engine.grid[0].len();

            engine
                .add_logical_grains(CategoryId::new(7), 1_000_000_000)
                .unwrap();

            assert_eq!(engine.pending_run_count(), 1);
            assert_eq!(engine.pending_grain_count(), 1_000_000_000);
            assert_eq!(engine.grain_count, engine.grid[0].len() + 1_000_000_000);
            assert_eq!(engine.snapshot_state().pending_runs[0].count, 1_000_000_000);
        }

        #[test]
        fn adjacent_categories_merge_without_losing_fifo_transitions() {
            let mut engine = SandEngine::new(1, 1);
            for cell in &mut engine.grid[0] {
                *cell = Some(CategoryId::new(0));
            }
            engine.grain_count = engine.grid[0].len();

            engine.add_logical_grains(CategoryId::new(1), 5).unwrap();
            engine.add_logical_grains(CategoryId::new(1), 7).unwrap();
            engine.add_logical_grains(CategoryId::new(2), 3).unwrap();
            engine.add_logical_grains(CategoryId::new(1), 2).unwrap();

            assert_eq!(
                engine.snapshot_state().pending_runs,
                vec![
                    PendingGrainRun {
                        category_id: 1,
                        count: 12
                    },
                    PendingGrainRun {
                        category_id: 2,
                        count: 3
                    },
                    PendingGrainRun {
                        category_id: 1,
                        count: 2
                    },
                ]
            );
        }

        #[test]
        fn legacy_pending_vector_migrates_to_version_two_runs() {
            let legacy = SandState {
                version: SandState::LEGACY_VERSION,
                grid_width: 2,
                grid_height: 2,
                grains: vec![SandStateGrain {
                    x: 0,
                    y: 1,
                    category_id: 1,
                }],
                frame_count: 4,
                sweep_left_to_right: true,
                rng_state: 9,
                ingress_focus_x: None,
                pending_grains: vec![2, 2, 1],
                pending_runs: Vec::new(),
                active_avalanche_columns: Vec::new(),
                mobilized_grains: Vec::new(),
                boundary_release_fronts: Vec::new(),
                boundary_release_in_flight: None,
            };
            let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)]);
            let mut engine = SandEngine::new(1, 1);
            engine.restore_state(&legacy, &valid).unwrap();
            let migrated = engine.snapshot_state();

            assert_eq!(migrated.version, SandState::VERSION);
            assert!(migrated.pending_grains.is_empty());
            assert_eq!(
                migrated.pending_runs,
                vec![
                    PendingGrainRun {
                        category_id: 2,
                        count: 2
                    },
                    PendingGrainRun {
                        category_id: 1,
                        count: 1
                    },
                ]
            );
            assert_eq!(engine.grain_count, 4);
        }

        #[test]
        fn category_removal_is_exact_across_compressed_runs() {
            let mut engine = SandEngine::new(1, 1);
            for cell in &mut engine.grid[0] {
                *cell = Some(CategoryId::new(0));
            }
            engine.grain_count = engine.grid[0].len();
            engine.add_logical_grains(CategoryId::new(3), 10).unwrap();
            engine.add_logical_grains(CategoryId::new(4), 2).unwrap();
            engine.add_logical_grains(CategoryId::new(3), 5).unwrap();

            assert_eq!(engine.remove_category_grains(CategoryId::new(3), 12), 12);
            assert_eq!(
                engine.snapshot_state().pending_runs,
                vec![
                    PendingGrainRun {
                        category_id: 4,
                        count: 2
                    },
                    PendingGrainRun {
                        category_id: 3,
                        count: 3
                    },
                ]
            );
        }
    }
}
