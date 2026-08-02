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
}

impl SandState {
    pub const VERSION: u8 = 1;
}

fn default_sweep_left_to_right() -> bool {
    true
}

fn default_rng_state() -> u64 {
    0x9E37_79B9_7F4A_7C15
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
    pending_grains: VecDeque<CategoryId>,
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
            pending_grains: VecDeque::new(),
            grain_count: 0,
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.cell_width = width;
        self.cell_height = height;
    }

    fn capacity(&self) -> usize {
        if self.grid.is_empty() || self.grid[0].is_empty() {
            0
        } else {
            self.grid.len() * self.grid[0].len()
        }
    }

    pub fn spawn(&mut self, category_id: CategoryId) {
        self.pending_grains.push_back(category_id);
        self.grain_count += 1;
        self.flush_pending_grains();
    }

    #[cfg(test)]
    fn pending_grain_count(&self) -> usize {
        self.pending_grains.len()
    }

    pub fn physical_grain_count(&self) -> usize {
        self.grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_some())
            .count()
    }

    fn refresh_logical_grain_count(&mut self) {
        self.grain_count = self.physical_grain_count() + self.pending_grains.len();
    }

    fn flush_pending_grains(&mut self) {
        if self.capacity() == 0 {
            return;
        }

        let width = self.grid[0].len();
        while let Some(category_id) = self.pending_grains.front().copied() {
            let start = self.random_index(width);
            let insertion = (0..width)
                .map(|offset| (start + offset) % width)
                .find(|&x| self.grid[0][x].is_none());

            let Some(x) = insertion else {
                break;
            };

            self.grid[0][x] = Some(category_id);
            self.pending_grains.pop_front();
        }
    }

    fn apply_gravity(&mut self) {
        let h = self.grid.len();
        if h < 2 {
            return;
        }
        let w = self.grid[0].len();
        if w == 0 {
            return;
        }

        let base_left_to_right = self.sweep_left_to_right;
        self.sweep_left_to_right = !self.sweep_left_to_right;

        for y in (0..h - 1).rev() {
            let left_to_right = if y.is_multiple_of(2) {
                base_left_to_right
            } else {
                !base_left_to_right
            };

            if left_to_right {
                for x in 0..w {
                    if let Some(cat) = self.grid[y][x] {
                        if self.grid[y + 1][x].is_none() {
                            self.grid[y + 1][x] = Some(cat);
                            self.grid[y][x] = None;
                        } else {
                            let dir: isize = if self.random_bool() { 1 } else { -1 };
                            let nx = (x as isize) + dir;

                            if nx >= 0
                                && (nx as usize) < w
                                && self.grid[y + 1][nx as usize].is_none()
                            {
                                self.grid[y + 1][nx as usize] = Some(cat);
                                self.grid[y][x] = None;
                            }
                        }
                    }
                }
            } else {
                for x in (0..w).rev() {
                    if let Some(cat) = self.grid[y][x] {
                        if self.grid[y + 1][x].is_none() {
                            self.grid[y + 1][x] = Some(cat);
                            self.grid[y][x] = None;
                        } else {
                            let dir: isize = if self.random_bool() { 1 } else { -1 };
                            let nx = (x as isize) + dir;

                            if nx >= 0
                                && (nx as usize) < w
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
        for row in &mut self.grid {
            for cell in row {
                *cell = None;
            }
        }
        self.pending_grains.clear();
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

        self.pending_grains
            .retain(|pending| *pending != category_id);
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
            let mut retained = VecDeque::with_capacity(self.pending_grains.len());
            while let Some(category) = self.pending_grains.pop_front() {
                if category == category_id && removed + pending_removed < count {
                    pending_removed += 1;
                } else {
                    retained.push_back(category);
                }
            }
            self.pending_grains = retained;
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
        let mut grains = Vec::with_capacity(self.grain_count);

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
            pending_grains: self
                .pending_grains
                .iter()
                .map(|category_id| category_id.0)
                .collect(),
        }
    }

    pub fn restore_state(&mut self, state: &SandState, valid_category_ids: &HashSet<CategoryId>) {
        if state.version != SandState::VERSION {
            return;
        }

        if state.grid_width == 0 || state.grid_height == 0 {
            self.clear();
            return;
        }

        let mut restored = vec![vec![None; state.grid_width]; state.grid_height];
        let none_id = DRIFT_CATEGORY_ID;

        for grain in &state.grains {
            if grain.x >= state.grid_width || grain.y >= state.grid_height {
                continue;
            }

            let category_id = CategoryId::new(grain.category_id);
            let normalized_id = if valid_category_ids.contains(&category_id) {
                category_id
            } else {
                none_id
            };

            restored[grain.y][grain.x] = Some(normalized_id);
        }

        self.grid = restored;
        self.grid_width_dots = state.grid_width;
        self.grid_height_dots = state.grid_height;
        self.pending_grains = state
            .pending_grains
            .iter()
            .map(|category_id| {
                let category_id = CategoryId::new(*category_id);
                if valid_category_ids.contains(&category_id) {
                    category_id
                } else {
                    none_id
                }
            })
            .collect();
        self.refresh_logical_grain_count();
        self.frame_count = state.frame_count;
        self.sweep_left_to_right = state.sweep_left_to_right;
        self.rng_state = if state.rng_state == 0 {
            default_rng_state()
        } else {
            state.rng_state
        };
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

    use crate::{domain::CategoryId, sand::SandEngine};

    #[test]
    fn viewport_resize_preserves_exact_logical_state() {
        let mut engine = SandEngine::new(12, 8);
        engine.clear();
        engine.grid[0][0] = Some(CategoryId::new(1));
        engine.grid[10][12] = Some(CategoryId::new(2));
        engine.grid[31][23] = Some(CategoryId::new(1));
        engine.pending_grains.push_back(CategoryId::new(2));
        engine.grain_count = 4;
        engine.frame_count = 17;
        engine.sweep_left_to_right = false;
        engine.rng_state = 0xCAFE_BABE;
        let before = engine.snapshot_state();

        engine.resize(3, 2);
        assert_eq!(engine.snapshot_state(), before);

        engine.resize(30, 20);
        assert_eq!(engine.snapshot_state(), before);
    }

    #[test]
    fn oscillating_viewport_resizes_are_logically_idempotent() {
        let mut engine = SandEngine::new(10, 6);
        engine.clear();
        for (x, y, category_id) in [(0, 0, 1), (7, 8, 2), (19, 23, 1)] {
            engine.grid[y][x] = Some(CategoryId::new(category_id));
        }
        engine.grain_count = 3;
        let baseline = engine.snapshot_state();

        for (width, height) in [(2, 1), (40, 20), (5, 3), (10, 6), (1, 1)] {
            engine.resize(width, height);
            assert_eq!(engine.snapshot_state(), baseline);
        }
    }

    #[test]
    fn hidden_grains_reappear_when_viewport_expands() {
        let mut engine = SandEngine::new(4, 2);
        engine.clear();
        engine.grid[0][0] = Some(CategoryId::new(0));
        engine.grain_count = 1;

        engine.resize(2, 1);
        let cropped = engine.render(&[]);
        assert!(
            cropped
                .iter()
                .flat_map(|line| line.spans.iter())
                .all(|span| { span.content.as_ref() == "⠀" })
        );

        engine.resize(4, 2);
        let expanded = engine.render(&[]);
        assert!(
            expanded
                .iter()
                .flat_map(|line| line.spans.iter())
                .any(|span| { span.content.as_ref() != "⠀" })
        );
    }

    #[test]
    fn viewport_render_size_is_independent_of_logical_canvas_size() {
        let mut engine = SandEngine::new(4, 2);
        engine.resize(9, 5);
        let expanded = engine.render(&[]);
        assert_eq!(expanded.len(), 5);
        assert!(expanded.iter().all(|line| line.spans.len() == 9));

        engine.resize(1, 1);
        let shrunk = engine.render(&[]);
        assert_eq!(shrunk.len(), 1);
        assert_eq!(shrunk[0].spans.len(), 1);
        assert_eq!(engine.grid_width_dots, 8);
        assert_eq!(engine.grid_height_dots, 8);
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
        restored.restore_state(&state, &valid);

        assert_eq!(restored.grid[3][2], Some(CategoryId::new(1)));
        assert_eq!(restored.grid[10][7], Some(CategoryId::new(2)));
        assert_eq!(restored.grain_count, 2);
    }

    #[test]
    fn test_sand_state_restore_maps_unknown_category_to_none() {
        let mut se = SandEngine::new(20, 20);
        se.clear();
        se.grid[2][2] = Some(CategoryId::new(99));
        se.grain_count = 1;

        let state = se.snapshot_state();

        let mut restored = SandEngine::new(20, 20);
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1)]);
        restored.restore_state(&state, &valid);

        assert_eq!(restored.grid[2][2], Some(CategoryId::new(0)));
        assert_eq!(restored.grain_count, 1);
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
        restored.restore_state(&state, &valid);

        assert_eq!(restored.snapshot_state(), state);
        assert_eq!(restored.cell_width, 40);
        assert_eq!(restored.cell_height, 40);
        assert_eq!(restored.grid_width_dots, state.grid_width);
        assert_eq!(restored.grid_height_dots, state.grid_height);
    }

    #[test]
    fn test_clear_category_removes_only_requested_id() {
        let mut se = SandEngine::new(20, 20);
        se.clear();
        se.grid[1][1] = Some(CategoryId::new(0));
        se.grid[2][2] = Some(CategoryId::new(0));
        se.grid[3][3] = Some(CategoryId::new(1));
        se.grain_count = 3;

        se.clear_category(CategoryId::new(0));

        assert_eq!(se.grid[1][1], None);
        assert_eq!(se.grid[2][2], None);
        assert_eq!(se.grid[3][3], Some(CategoryId::new(1)));
        assert_eq!(se.grain_count, 1);
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
        restored.restore_state(&state, &valid);

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
        assert_eq!(state.pending_grains, vec![2]);

        let mut restored = SandEngine::new(2, 1);
        restored.restore_state(
            &state,
            &HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)]),
        );

        assert_eq!(restored.pending_grain_count(), 1);
        assert_eq!(restored.grain_count, state.grains.len() + 1);
        assert_eq!(restored.snapshot_state().pending_grains, vec![2]);
    }
}
