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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_grains: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_runs: Vec<PendingGrainRun>,
}

impl SandState {
    pub const VERSION: u8 = 2;
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
    pending_runs: VecDeque<PendingRun>,
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
            pending_runs: VecDeque::new(),
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
        self.grid_width_dots = target_width;
        self.grid_height_dots = target_height;
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
            let free_index = self.random_index(free_columns.len());
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

    fn apply_gravity(&mut self) {
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
                    if let Some(cat) = self.grid[y][x] {
                        if self.grid[y + 1][x].is_none() {
                            self.grid[y + 1][x] = Some(cat);
                            self.grid[y][x] = None;
                        } else {
                            let dir: isize = if self.random_bool() { 1 } else { -1 };
                            let nx = (x as isize) + dir;

                            if nx >= bounds.x_start as isize
                                && (nx as usize) < bounds.x_end
                                && self.grid[y + 1][nx as usize].is_none()
                            {
                                self.grid[y + 1][nx as usize] = Some(cat);
                                self.grid[y][x] = None;
                            }
                        }
                    }
                }
            } else {
                for x in (bounds.x_start..bounds.x_end).rev() {
                    if let Some(cat) = self.grid[y][x] {
                        if self.grid[y + 1][x].is_none() {
                            self.grid[y + 1][x] = Some(cat);
                            self.grid[y][x] = None;
                        } else {
                            let dir: isize = if self.random_bool() { 1 } else { -1 };
                            let nx = (x as isize) + dir;

                            if nx >= bounds.x_start as isize
                                && (nx as usize) < bounds.x_end
                                && self.grid[y + 1][nx as usize].is_none()
                            {
                                self.grid[y + 1][nx as usize] = Some(cat);
                                self.grid[y][x] = None;
                            }
                        }
                    }
                }
            }
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
        self.pending_runs.clear();
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
            pending_grains: Vec::new(),
            pending_runs: self
                .pending_runs
                .iter()
                .map(|run| PendingGrainRun {
                    category_id: run.category_id.0,
                    count: run.count,
                })
                .collect(),
        }
    }

    pub fn restore_state(
        &mut self,
        state: &SandState,
        valid_category_ids: &HashSet<CategoryId>,
    ) -> Result<(), String> {
        if state.version != SandState::VERSION && state.version != SandState::LEGACY_VERSION {
            return Err(format!("unsupported sand state version {}", state.version));
        }
        if (state.grid_width == 0 || state.grid_height == 0) && !state.grains.is_empty() {
            return Err("zero-sized sand state cannot contain placed grains".to_string());
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
                return Err("legacy sand state cannot contain version-two pending runs".to_string());
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
            pending_grains: Vec::new(),
            pending_runs: vec![PendingGrainRun {
                category_id: 2,
                count: 4,
            }],
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
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
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
                pending_grains: vec![2, 2, 1],
                pending_runs: Vec::new(),
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
