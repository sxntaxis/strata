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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_avalanche_columns: Vec<usize>,
}

impl SandState {
    pub const VERSION: u8 = 4;
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
const STATIC_REPOSE_RELIEF: usize = 3;
const DYNAMIC_REPOSE_RELIEF: usize = 1;
const AVALANCHE_ACTIVITY_RADIUS: usize = 1;
const MAX_ISOLATED_SPIRE_HEIGHT: usize = 2;

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
    avalanche_active: Vec<bool>,
    supported_heights: Vec<usize>,
    #[cfg(test)]
    last_avalanche_motion: bool,
    #[cfg(test)]
    last_diagonal_topple: bool,
    #[cfg(test)]
    last_avalanche_span: usize,
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
            avalanche_active: vec![false; grid_width_dots],
            supported_heights: vec![0; grid_width_dots],
            #[cfg(test)]
            last_avalanche_motion: false,
            #[cfg(test)]
            last_diagonal_topple: false,
            #[cfg(test)]
            last_avalanche_span: 0,
            grain_count: 0,
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.cell_width = width;
        self.cell_height = height;
        self.expand_logical_canvas_to_viewport();
    }

    fn expand_logical_canvas_to_viewport(&mut self) {
        let viewport_width = self.cell_width as usize * SAND_ENGINE.dot_width;
        let viewport_height = self.cell_height as usize * SAND_ENGINE.dot_height;
        let target_width = self.grid_width_dots.max(viewport_width);
        let target_height = self.grid_height_dots.max(viewport_height);
        if target_width == self.grid_width_dots && target_height == self.grid_height_dots {
            return;
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
        self.ingress_focus_x = self
            .ingress_focus_x
            .map(|x| x.saturating_add(horizontal_offset));
        self.grid_width_dots = target_width;
        self.grid_height_dots = target_height;
        let mut shifted_active = vec![false; target_width];
        for (old_x, is_active) in self.avalanche_active.iter().copied().enumerate() {
            if is_active {
                shifted_active[old_x + horizontal_offset] = true;
            }
        }
        self.avalanche_active = shifted_active;
        self.supported_heights.resize(target_width, 0);
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

    fn refresh_avalanche_activity(&mut self, bounds: ViewportBounds, x: usize) {
        let start = x
            .saturating_sub(AVALANCHE_ACTIVITY_RADIUS)
            .max(bounds.x_start);
        let end = x
            .saturating_add(AVALANCHE_ACTIVITY_RADIUS + 1)
            .min(bounds.x_end);
        for column in start..end {
            self.avalanche_active[column] = true;
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

    fn diagonal_topple(
        &mut self,
        bounds: ViewportBounds,
        threshold: usize,
        active_only: bool,
    ) -> bool {
        if !self.sweep_left_to_right {
            for x in bounds.x_start..bounds.x_end {
                if active_only && !self.avalanche_active[x] {
                    continue;
                }
                if self.topple_column_if_unstable(bounds, x, threshold) {
                    return true;
                }
            }
        } else {
            for x in (bounds.x_start..bounds.x_end).rev() {
                if active_only && !self.avalanche_active[x] {
                    continue;
                }
                if self.topple_column_if_unstable(bounds, x, threshold) {
                    return true;
                }
            }
        }
        false
    }

    fn is_isolated_supported_spire(&self, bounds: ViewportBounds, x: usize) -> bool {
        if x <= bounds.x_start || x + 1 >= bounds.x_end {
            return false;
        }
        self.supported_heights[x] > MAX_ISOLATED_SPIRE_HEIGHT
            && self.supported_heights[x - 1] == 0
            && self.supported_heights[x + 1] == 0
    }

    fn topple_column_if_unstable(
        &mut self,
        bounds: ViewportBounds,
        x: usize,
        threshold: usize,
    ) -> bool {
        // H2's ordinary static repose deliberately allows relief three. Daily-use
        // evidence found one narrower artifact worth excluding: a bottom-supported
        // single-column needle three dots high with completely empty immediate
        // neighbors. Keep two-dot needles and all non-isolated shoulders on the
        // normal 3/1 repose model; only the isolated static needle uses a cap of two.
        let effective_threshold =
            if threshold == STATIC_REPOSE_RELIEF && self.is_isolated_supported_spire(bounds, x) {
                MAX_ISOLATED_SPIRE_HEIGHT
            } else {
                threshold
            };
        let Some((source_y, target_x)) =
            self.diagonal_target_for_relief(bounds, x, effective_threshold)
        else {
            return false;
        };
        let target_y = source_y + 1;
        let category = self.grid[source_y][x].take().expect("surface grain exists");
        self.grid[target_y][target_x] = Some(category);
        self.refresh_avalanche_activity(bounds, x);
        self.refresh_avalanche_activity(bounds, target_x);
        #[cfg(test)]
        {
            self.last_diagonal_topple = true;
            self.last_avalanche_motion = true;
            self.last_avalanche_span = x.abs_diff(target_x) + 1;
        }
        true
    }

    fn apply_gravity(&mut self) {
        #[cfg(test)]
        {
            self.last_avalanche_motion = false;
            self.last_diagonal_topple = false;
            self.last_avalanche_span = 0;
        }
        let Some(bounds) = self.viewport_bounds() else {
            return;
        };
        if bounds.y_end.saturating_sub(bounds.y_start) < 2 {
            return;
        }

        let base_left_to_right = self.sweep_left_to_right;
        self.sweep_left_to_right = !self.sweep_left_to_right;

        let mut active_vertical = false;
        for y in (bounds.y_start..bounds.y_end - 1).rev() {
            let left_to_right = if y.is_multiple_of(2) {
                base_left_to_right
            } else {
                !base_left_to_right
            };

            if left_to_right {
                for x in bounds.x_start..bounds.x_end {
                    if let Some(cat) = self.grid[y][x]
                        && self.grid[y + 1][x].is_none()
                    {
                        self.grid[y + 1][x] = Some(cat);
                        self.grid[y][x] = None;
                        active_vertical |= self.avalanche_active[x]
                            || self
                                .avalanche_active
                                .get(x.saturating_sub(1))
                                .copied()
                                .unwrap_or(false)
                            || self.avalanche_active.get(x + 1).copied().unwrap_or(false);
                        #[cfg(test)]
                        if active_vertical {
                            self.last_avalanche_motion = true;
                        }
                    }
                }
            } else {
                for x in (bounds.x_start..bounds.x_end).rev() {
                    if let Some(cat) = self.grid[y][x]
                        && self.grid[y + 1][x].is_none()
                    {
                        self.grid[y + 1][x] = Some(cat);
                        self.grid[y][x] = None;
                        active_vertical |= self.avalanche_active[x]
                            || self
                                .avalanche_active
                                .get(x.saturating_sub(1))
                                .copied()
                                .unwrap_or(false)
                            || self.avalanche_active.get(x + 1).copied().unwrap_or(false);
                        #[cfg(test)]
                        if active_vertical {
                            self.last_avalanche_motion = true;
                        }
                    }
                }
            }
        }

        self.derive_supported_heights(bounds);
        let active_visible = (bounds.x_start..bounds.x_end).any(|x| self.avalanche_active[x]);
        if active_visible {
            if self.diagonal_topple(bounds, DYNAMIC_REPOSE_RELIEF, true) {
                return;
            }
            if active_vertical {
                return;
            }
            for x in bounds.x_start..bounds.x_end {
                self.avalanche_active[x] = false;
            }
        }
        let _ = self.diagonal_topple(bounds, STATIC_REPOSE_RELIEF, false);
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
        self.pending_runs.clear();
        self.ingress_focus_x = None;
        self.avalanche_active.fill(false);
        self.grain_count = 0;
    }

    pub fn clear_category(&mut self, category_id: CategoryId) {
        for row in &mut self.grid {
            for cell in row {
                if *cell == Some(category_id) {
                    *cell = None;
                }
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
        for row in self.grid.iter_mut().rev() {
            for cell in row.iter_mut() {
                if removed >= count {
                    break;
                }

                if *cell == Some(category_id) {
                    *cell = None;
                    removed += 1;
                }
            }

            if removed >= count {
                break;
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

        for (y, row) in self.grid.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if let Some(category_id) = cell {
                    grains.push(SandStateGrain {
                        x,
                        y,
                        category_id: category_id.0,
                    });
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
            active_avalanche_columns: self
                .avalanche_active
                .iter()
                .enumerate()
                .filter_map(|(x, active)| active.then_some(x))
                .collect(),
        }
    }

    pub fn restore_state(
        &mut self,
        state: &SandState,
        valid_category_ids: &HashSet<CategoryId>,
    ) -> Result<(), String> {
        if state.version != SandState::VERSION
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
            && state.version != SandState::ORGANIC_VERSION
            && state.ingress_focus_x.is_some()
        {
            return Err("pre-organic sand state contains an ingress focus".to_string());
        }
        if state.version != SandState::VERSION && !state.active_avalanche_columns.is_empty() {
            return Err("pre-v4 sand state contains active avalanche columns".to_string());
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
        self.ingress_focus_x =
            if state.version == SandState::VERSION || state.version == SandState::ORGANIC_VERSION {
                state.ingress_focus_x
            } else {
                None
            };
        self.avalanche_active = vec![false; state.grid_width];
        if state.version == SandState::VERSION {
            for &x in &state.active_avalanche_columns {
                self.avalanche_active[x] = true;
            }
        }
        self.supported_heights = vec![0; state.grid_width];
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
            PendingGrainRun, SandEngine, SandState, SandStateGrain, recolor_state_category_mass,
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
        sand::{SandEngine, SandState, SandStateGrain},
    };

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
    }

    fn supported_relief_fixture(relief: usize) -> (SandEngine, usize) {
        let mut engine = SandEngine::new(4, 1);
        engine.clear();
        let source_x = 2;
        for y in (engine.grid_height_dots - relief)..engine.grid_height_dots {
            for x in source_x..engine.grid_width_dots {
                engine.grid[y][x] = Some(CategoryId::new(if x == source_x { 1 } else { 2 }));
            }
        }
        engine.grain_count = relief * (engine.grid_width_dots - source_x);
        (engine, source_x)
    }

    #[test]
    fn static_metastability_is_deterministic_for_ten_thousand_passes() {
        let (mut engine, source_x) = supported_relief_fixture(3);
        let before = engine.snapshot_state();

        for _ in 0..10_000 {
            engine.apply_gravity();
        }

        assert_eq!(engine.snapshot_state(), before);
        assert!(engine.avalanche_active.iter().all(|active| !active));
        assert_eq!(engine.rng_state, before.rng_state);
        assert!(
            engine.grid[engine.grid_height_dots - 3..]
                .iter()
                .all(|row| row[source_x].is_some())
        );
    }

    #[test]
    fn isolated_two_dot_spire_remains_allowed() {
        let mut engine = SandEngine::new(4, 1);
        engine.clear();
        let x = engine.grid_width_dots / 2;
        for y in engine.grid_height_dots - 2..engine.grid_height_dots {
            engine.grid[y][x] = Some(CategoryId::new(1));
        }
        engine.grain_count = 2;
        let before = engine.snapshot_state();

        for _ in 0..10_000 {
            engine.apply_gravity();
        }

        assert_eq!(engine.snapshot_state(), before);
        assert!(engine.avalanche_active.iter().all(|active| !active));
    }

    #[test]
    fn isolated_three_dot_spire_yields_on_next_static_pass() {
        let mut engine = SandEngine::new(4, 1);
        engine.clear();
        let x = engine.grid_width_dots / 2;
        for y in engine.grid_height_dots - 3..engine.grid_height_dots {
            engine.grid[y][x] = Some(CategoryId::new(1));
        }
        engine.grain_count = 3;

        engine.apply_gravity();

        assert_eq!(engine.grid[engine.grid_height_dots - 3][x], None);
        assert!(engine.avalanche_active.iter().any(|active| *active));
        assert_eq!(engine.physical_grain_count(), 3);
    }

    #[test]
    fn three_dot_peak_with_neighbor_support_keeps_normal_static_repose() {
        let mut engine = SandEngine::new(4, 1);
        engine.clear();
        let x = engine.grid_width_dots / 2;
        for y in engine.grid_height_dots - 3..engine.grid_height_dots {
            engine.grid[y][x] = Some(CategoryId::new(1));
        }
        engine.grid[engine.grid_height_dots - 1][x - 1] = Some(CategoryId::new(2));
        engine.grain_count = 4;
        let before = engine.snapshot_state();

        for _ in 0..10_000 {
            engine.apply_gravity();
        }

        assert_eq!(engine.snapshot_state(), before);
        assert!(engine.avalanche_active.iter().all(|active| !active));
    }

    #[test]
    fn supported_neighbors_keep_three_dot_peak_on_normal_h2_repose() {
        for (left, right) in [(0, 1), (1, 1), (2, 2)] {
            let mut engine = SandEngine::new(4, 1);
            engine.clear();
            let x = engine.grid_width_dots / 2;
            for y in engine.grid_height_dots - 3..engine.grid_height_dots {
                engine.grid[y][x] = Some(CategoryId::new(1));
            }
            for (neighbor_x, height) in [(x - 1, left), (x + 1, right)] {
                for y in engine.grid_height_dots - height..engine.grid_height_dots {
                    engine.grid[y][neighbor_x] = Some(CategoryId::new(2));
                }
            }
            engine.grain_count = 3 + left + right;
            let before = engine.snapshot_state();

            for _ in 0..10_000 {
                engine.apply_gravity();
            }

            assert_eq!(engine.snapshot_state(), before);
            assert!(engine.avalanche_active.iter().all(|active| !active));
        }
    }

    #[test]
    fn three_dot_spire_at_visible_wall_keeps_normal_static_repose() {
        let mut engine = SandEngine::new(4, 1);
        engine.clear();
        let bounds = engine.viewport_bounds().unwrap();
        let x = bounds.x_start;
        for y in engine.grid_height_dots - 3..engine.grid_height_dots {
            engine.grid[y][x] = Some(CategoryId::new(1));
        }
        engine.grain_count = 3;
        let before = engine.snapshot_state();

        for _ in 0..10_000 {
            engine.apply_gravity();
        }

        assert_eq!(engine.snapshot_state(), before);
        assert!(engine.avalanche_active.iter().all(|active| !active));
    }

    #[test]
    fn relief_four_yields_without_random_waiting() {
        let (mut engine, source_x) = supported_relief_fixture(4);
        let before_rng = engine.rng_state;

        engine.apply_gravity();

        assert_eq!(engine.rng_state, before_rng);
        assert_eq!(engine.grid[engine.grid_height_dots - 4][source_x], None);
        assert!(engine.avalanche_active.iter().any(|active| *active));
    }

    #[test]
    fn dynamic_threshold_two_yields_but_static_threshold_three_holds() {
        let (mut engine, source_x) = supported_relief_fixture(2);
        engine.apply_gravity();
        assert_eq!(
            engine.grid[engine.grid_height_dots - 2][source_x],
            Some(CategoryId::new(1))
        );

        engine.avalanche_active[source_x] = true;
        engine.apply_gravity();
        assert_eq!(engine.grid[engine.grid_height_dots - 2][source_x], None);
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
    fn active_avalanche_columns_round_trip_sorted_and_shift_on_growth() {
        let mut engine = SandEngine::new(4, 2);
        engine.avalanche_active[1] = true;
        engine.avalanche_active[5] = true;
        let state = engine.snapshot_state();
        assert_eq!(state.active_avalanche_columns, vec![1, 5]);

        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut restored = SandEngine::new(4, 2);
        restored.restore_state(&state, &valid).unwrap();
        assert_eq!(restored.snapshot_state(), state);

        let old_width = restored.grid_width_dots;
        restored.resize(8, 2);
        let offset = (restored.grid_width_dots - old_width) / 2;
        assert!(restored.avalanche_active[1 + offset]);
        assert!(restored.avalanche_active[5 + offset]);
    }

    #[test]
    fn malformed_active_avalanche_columns_fail_closed() {
        let mut state = SandEngine::new(4, 2).snapshot_state();
        state.active_avalanche_columns = vec![3, 3];
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut engine = SandEngine::new(4, 2);
        assert!(engine.restore_state(&state, &valid).is_err());

        state.active_avalanche_columns = vec![state.grid_width];
        assert!(engine.restore_state(&state, &valid).is_err());
    }

    #[test]
    fn ingress_focus_moves_slowly_and_never_more_than_one_dot() {
        let mut engine = SandEngine::new(40, 2);
        engine.clear();
        let bounds = engine.viewport_bounds().expect("visible viewport");
        engine.ingress_focus_x = Some((bounds.x_start + bounds.x_end) / 2);
        engine.rng_state = 0x0A11_CE55;

        let mut previous = engine.ingress_focus_x.unwrap();
        let mut moved = 0usize;
        let mut stayed = 0usize;
        for _ in 0..64 {
            let focus = engine.advance_ingress_focus(bounds);
            assert!(focus.abs_diff(previous) <= 1);
            if focus == previous {
                stayed += 1;
            } else {
                moved += 1;
            }
            previous = focus;
        }

        assert!(moved > 0);
        assert!(stayed > moved);
    }

    #[test]
    fn ingress_rain_is_full_width_with_only_a_soft_focus_bias() {
        let mut engine = SandEngine::new(40, 2);
        engine.clear();
        let bounds = engine.viewport_bounds().expect("visible viewport");
        let visible_width = bounds.x_end - bounds.x_start;
        let focus = (bounds.x_start + bounds.x_end) / 2;
        engine.ingress_focus_x = Some(focus);
        engine.rng_state = 0x0A11_CE55;

        let samples = (0..1024)
            .map(|_| engine.sample_ingress_target(bounds, focus))
            .collect::<Vec<_>>();
        let distinct = samples.iter().copied().collect::<HashSet<_>>().len();
        let far = samples
            .iter()
            .filter(|&&x| x.abs_diff(focus) >= visible_width / 4)
            .count();
        let mean_distance =
            samples.iter().map(|&x| x.abs_diff(focus)).sum::<usize>() as f64 / samples.len() as f64;
        let uniform_mean_distance = visible_width as f64 / 4.0;

        assert!(
            distinct >= visible_width * 3 / 4,
            "rain must cover most visible columns rather than expose a nozzle"
        );
        assert!(
            far >= samples.len() / 3,
            "far-away rain must remain common, not exceptional"
        );
        assert!(
            mean_distance < uniform_mean_distance,
            "the wandering focus must still create a measurable long-run bias"
        );
        assert!(
            mean_distance > uniform_mean_distance * 0.75,
            "the short-run bias must stay weak enough to preserve a rain-like fall"
        );
    }

    #[test]
    fn occupied_fallback_does_not_drag_the_slow_focus_to_the_placement() {
        let mut engine = SandEngine::new(8, 2);
        engine.clear();
        let bounds = engine.viewport_bounds().expect("visible viewport");
        let initial_focus = (bounds.x_start + bounds.x_end) / 2;
        let only_free = bounds.x_end - 1;
        engine.ingress_focus_x = Some(initial_focus);
        engine.rng_state = 0x55AA_1234;

        for x in bounds.x_start..bounds.x_end {
            if x != only_free {
                engine.grid[bounds.y_start][x] = Some(CategoryId::new(2));
                engine.grain_count += 1;
            }
        }

        engine.spawn(CategoryId::new(1));

        assert_eq!(
            engine.grid[bounds.y_start][only_free],
            Some(CategoryId::new(1))
        );
        let focus = engine.ingress_focus_x.expect("persisted ingress focus");
        assert!(focus.abs_diff(initial_focus) <= 1);
        assert_ne!(focus, only_free);
    }

    #[test]
    fn hidden_ingress_focus_is_clamped_back_into_the_visible_basin() {
        let mut engine = SandEngine::new(8, 4);
        engine.resize(12, 6);
        engine.ingress_focus_x = Some(0);
        engine.resize(4, 2);
        let bounds = engine.viewport_bounds().expect("visible viewport");
        assert!(engine.ingress_focus_x.unwrap() < bounds.x_start);

        engine.spawn(CategoryId::new(1));

        let focus = engine.ingress_focus_x.expect("spawned ingress focus");
        assert!(bounds.x_start <= focus && focus < bounds.x_end);
        let visible_spawn_count = (bounds.x_start..bounds.x_end)
            .filter(|x| engine.grid[bounds.y_start][*x] == Some(CategoryId::new(1)))
            .count();
        assert_eq!(visible_spawn_count, 1);
        assert!(
            engine.grid[bounds.y_start][..bounds.x_start]
                .iter()
                .all(Option::is_none)
        );
        assert!(
            engine.grid[bounds.y_start][bounds.x_end..]
                .iter()
                .all(Option::is_none)
        );
    }

    #[test]
    fn canvas_growth_shifts_ingress_focus_with_centered_topology() {
        let mut engine = SandEngine::new(4, 2);
        engine.clear();
        let old_width = engine.grid_width_dots;
        engine.ingress_focus_x = Some(2);

        engine.resize(8, 2);

        let horizontal_offset = (engine.grid_width_dots - old_width) / 2;
        assert_eq!(engine.ingress_focus_x, Some(2 + horizontal_offset));
    }

    #[test]
    fn full_clear_resets_ingress_focus_but_category_clear_preserves_it() {
        let mut engine = SandEngine::new(8, 2);
        engine.clear();
        engine.ingress_focus_x = Some(5);
        let bottom = engine.grid_height_dots - 1;
        engine.grid[bottom][1] = Some(CategoryId::new(1));
        engine.grain_count = 1;

        engine.clear_category(CategoryId::new(1));
        assert_eq!(engine.ingress_focus_x, Some(5));

        engine.clear();
        assert_eq!(engine.ingress_focus_x, None);
    }

    #[test]
    fn snapshot_restore_continues_the_same_biased_rain_stream() {
        let mut source = SandEngine::new(8, 2);
        source.clear();
        let bounds = source.viewport_bounds().expect("visible viewport");
        source.ingress_focus_x = Some(bounds.x_start + 3);
        source.rng_state = 0x55AA_1234_9876;
        let state = source.snapshot_state();

        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut restored = SandEngine::new(8, 2);
        restored.restore_state(&state, &valid).unwrap();

        source.add_logical_grains(CategoryId::new(1), 5).unwrap();
        restored.add_logical_grains(CategoryId::new(1), 5).unwrap();

        assert_eq!(restored.snapshot_state(), source.snapshot_state());
    }

    #[test]
    fn serialized_version_two_state_defaults_missing_ingress_focus() {
        let json = r#"{
            "version":2,
            "grid_width":2,
            "grid_height":2,
            "grains":[],
            "frame_count":4,
            "sweep_left_to_right":true,
            "rng_state":9,
            "pending_runs":[]
        }"#;

        let state: SandState = serde_json::from_str(json).unwrap();

        assert_eq!(state.version, SandState::COMPRESSED_PENDING_VERSION);
        assert_eq!(state.ingress_focus_x, None);
    }

    #[test]
    fn version_two_state_restores_without_inventing_an_ingress_focus() {
        let state = SandState {
            version: SandState::COMPRESSED_PENDING_VERSION,
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
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
            active_avalanche_columns: Vec::new(),
        };
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut engine = SandEngine::new(1, 1);

        engine.restore_state(&state, &valid).unwrap();
        let upgraded = engine.snapshot_state();

        assert_eq!(upgraded.version, SandState::VERSION);
        assert_eq!(upgraded.ingress_focus_x, None);
    }

    #[test]
    fn version_three_state_migrates_focus_and_empty_activity_to_v4() {
        let mut state = SandEngine::new(4, 2).snapshot_state();
        state.version = SandState::ORGANIC_VERSION;
        state.ingress_focus_x = Some(3);
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut engine = SandEngine::new(1, 1);

        engine.restore_state(&state, &valid).unwrap();
        let upgraded = engine.snapshot_state();

        assert_eq!(upgraded.version, SandState::VERSION);
        assert_eq!(upgraded.ingress_focus_x, Some(3));
        assert!(upgraded.active_avalanche_columns.is_empty());
    }

    #[test]
    fn snapshot_restore_continues_an_in_progress_avalanche_exactly() {
        let mut source = SandEngine::new(8, 4);
        source.avalanche_active[3] = true;
        source.avalanche_active[4] = true;
        source.rng_state = 0x1234_5678;
        source.sweep_left_to_right = false;
        let state = source.snapshot_state();
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut restored = SandEngine::new(8, 4);
        restored.restore_state(&state, &valid).unwrap();

        for _ in 0..8 {
            source.apply_gravity();
            restored.apply_gravity();
            assert_eq!(restored.snapshot_state(), source.snapshot_state());
        }
    }

    #[test]
    fn native_h2_statistics_cover_representative_viewports() {
        use crate::constants::TIME_SETTINGS;

        fn run(width: u16, height: u16) -> (Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>, usize) {
            let mut engine = SandEngine::new(width, height);
            engine.rng_state = 0xD15E_A5ED;
            let mut sizes = Vec::new();
            let mut quiet = Vec::new();
            let mut spans = Vec::new();
            let mut passes = Vec::new();
            let mut event_size: usize = 0;
            let mut quiet_since_event: usize = 0;
            let mut event_quiet: usize = 0;
            let mut event_span: usize = 0;
            let mut event_passes: usize = 0;
            let mut active_event = false;
            let mut spawned = 0usize;
            let mut next_spawn_ms = TIME_SETTINGS.tick_ms;
            let mut next_physics_ms = TIME_SETTINGS.physics_ms;
            while spawned < 10_000 || active_event {
                let now_ms = next_spawn_ms.min(next_physics_ms);
                if now_ms == next_spawn_ms && spawned < 10_000 {
                    engine.spawn(CategoryId::new(1));
                    spawned += 1;
                    next_spawn_ms += TIME_SETTINGS.tick_ms;
                    if !active_event {
                        quiet_since_event += 1;
                    }
                }
                if now_ms == next_physics_ms {
                    engine.update();
                    next_physics_ms += TIME_SETTINGS.physics_ms;
                    if engine.last_avalanche_motion {
                        if !active_event {
                            active_event = true;
                            event_size = 0;
                            event_quiet = quiet_since_event;
                            quiet_since_event = 0;
                            event_span = 0;
                            event_passes = 0;
                        }
                        event_size += usize::from(engine.last_diagonal_topple);
                        event_passes += 1;
                        event_span = event_span.max(engine.last_avalanche_span);
                    } else if active_event {
                        sizes.push(event_size);
                        quiet.push(event_quiet);
                        spans.push(event_span);
                        passes.push(event_passes);
                        active_event = false;
                    }
                }
            }
            if active_event {
                sizes.push(event_size);
                quiet.push(event_quiet);
                spans.push(event_span);
                passes.push(event_passes);
            }
            for x in 1..engine.grid_width_dots - 1 {
                assert!(
                    !(engine.supported_heights[x] >= 3
                        && engine.supported_heights[x - 1] == 0
                        && engine.supported_heights[x + 1] == 0)
                );
            }
            sizes.sort_unstable();
            quiet.sort_unstable();
            spans.sort_unstable();
            passes.sort_unstable();
            (sizes, quiet, spans, passes, engine.grain_count)
        }

        for (width, height) in [(40, 20), (80, 30)] {
            let (sizes, quiet, spans, passes, mass) = run(width, height);
            assert_eq!(mass, 10_000);
            assert!(!sizes.is_empty());
            let median = sizes[sizes.len() / 2];
            let p95 = sizes[(sizes.len() * 95 / 100).min(sizes.len() - 1)];
            let max = sizes.iter().copied().max().unwrap();
            let one_move = sizes.iter().filter(|&&size| size == 1).count() * 100 / sizes.len();
            eprintln!(
                "H2 live {width}x{height}: events={} median={} p95={} max={} quiet_median={} one_move_pct={} passes_median={} duration_ms={} span_median={}",
                sizes.len(),
                median,
                p95,
                max,
                quiet[quiet.len() / 2],
                one_move,
                passes[passes.len() / 2],
                passes[passes.len() / 2] * 64,
                spans[spans.len() / 2]
            );
        }
    }

    #[test]
    fn continuous_ingress_overload_stress_is_non_acceptance_only() {
        let mut engine = SandEngine::new(40, 20);
        engine.rng_state = 0xD15E_A5ED;
        for _ in 0..2_000 {
            engine.spawn(CategoryId::new(1));
            engine.apply_gravity();
        }
        assert_eq!(engine.grain_count, 2_000);
        eprintln!(
            "H2 overload: active_columns={} active_motion={}",
            engine
                .avalanche_active
                .iter()
                .filter(|active| **active)
                .count(),
            engine.last_avalanche_motion
        );
    }

    #[test]
    fn version_two_state_cannot_smuggle_an_ingress_focus() {
        let state = SandState {
            version: SandState::COMPRESSED_PENDING_VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: Vec::new(),
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 9,
            ingress_focus_x: Some(0),
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
            active_avalanche_columns: Vec::new(),
        };
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut engine = SandEngine::new(1, 1);

        let error = engine.restore_state(&state, &valid).unwrap_err();

        assert!(error.contains("pre-organic"));
    }

    #[test]
    fn invalid_persisted_ingress_focus_is_rejected_without_mutation() {
        let state = SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: Vec::new(),
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 9,
            ingress_focus_x: Some(2),
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
            active_avalanche_columns: Vec::new(),
        };
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        let mut engine = SandEngine::new(4, 2);
        let before = engine.snapshot_state();

        let error = engine.restore_state(&state, &valid).unwrap_err();

        assert!(error.contains("ingress focus"));
        assert_eq!(engine.snapshot_state(), before);
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
