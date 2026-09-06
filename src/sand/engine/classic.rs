use std::collections::VecDeque;

use ratatui::prelude::Line;

use crate::domain::{Category, CategoryId};

use super::{SandEngine, ViewportBounds};

const CLASSIC_PHYSICS_RNG_XOR: u64 = 0xC6BC_2796_92B5_CC83;
const CLASSIC_RAIN_RNG_XOR: u64 = 0xD1B5_4A32_D192_ED03;
const CLASSIC_REPOSE_RNG_XOR: u64 = 0xA5A5_5A5A_D3C1_B7E9;
const CLASSIC_REPOSE_LOW: u8 = 1;
const CLASSIC_REPOSE_HIGH: u8 = 2;
// Weak heterogeneity only: one column in twenty temporarily tolerates one
// additional unit of local relief before a diagonal release.
const CLASSIC_HIGH_REPOSE_ONE_IN: usize = 20;
const GOLDEN_RATIO: f64 = 1.618_033_988_749_895;
const RAIN_FOCUS_BIAS_ONE_IN: usize = 10;
// At one ingress per second, a full focus traverse takes about twelve hours.
const RAIN_FOCUS_EDGE_TO_EDGE_INGRESSES: usize = 43_200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClassicRainMode {
    Uniform,
    WanderingFocus,
}

/// Debug-only Strata sediment experiment.
///
/// Physics deliberately returns to the pre-pause grain law while retaining the
/// post-pause spatial architecture:
/// - each grain is a physical cell, not a height-only particle;
/// - gravity first tries straight down;
/// - when blocked, the grain chooses one random diagonal and moves only if that
///   destination is free and the source column's weak local repose allows release;
/// - the current visible window is the complete active basin, so its side edges
///   are real closed walls and grains never cross them;
/// - the canonical canvas grows monotonically to the largest viewport seen;
/// - canonical terrain outside the current visible window is dormant/frozen;
/// - expanding the visible window reconnects the preserved terrain and may cause
///   a large ordinary avalanche;
/// - no grain is discharged, buried, compressed, or deleted.
///
/// `Uniform` uses the original pre-pause full-width random rain. `WanderingFocus`
/// changes only ingress sampling: 90% remains uniform over the full visible width
/// and 10% uses the accepted slow golden-corridor focus from the Oslo experiments.
/// Nothing from this sandbox is persisted or becomes production authority.
pub(crate) struct ClassicSandboxEngine {
    surface: SandEngine,
    mode: ClassicRainMode,
    physics_rng_state: u64,
    rain_rng_state: u64,
    initial_repose_rng_state: u64,
    repose_rng_state: u64,
    local_repose: Vec<u8>,
    rain_focus_x: Option<usize>,
    rain_focus_target_x: Option<usize>,
    rain_focus_move_counter: usize,
    pending_drive: VecDeque<CategoryId>,
    frame_count: usize,
    total_generated: usize,
    vertical_moves: usize,
    diagonal_moves: usize,
    #[cfg(test)]
    force_uniform_repose: bool,
}

impl ClassicSandboxEngine {
    pub(crate) fn new(width: u16, height: u16, seed: u64, mode: ClassicRainMode) -> Self {
        let mut physics_rng_state = seed ^ CLASSIC_PHYSICS_RNG_XOR;
        if physics_rng_state == 0 {
            physics_rng_state = CLASSIC_PHYSICS_RNG_XOR;
        }
        let mut rain_rng_state = seed ^ CLASSIC_RAIN_RNG_XOR;
        if rain_rng_state == 0 {
            rain_rng_state = CLASSIC_RAIN_RNG_XOR;
        }
        let mut repose_rng_state = seed ^ CLASSIC_REPOSE_RNG_XOR;
        if repose_rng_state == 0 {
            repose_rng_state = CLASSIC_REPOSE_RNG_XOR;
        }
        let surface = SandEngine::new(width, height);
        let local_repose = vec![CLASSIC_REPOSE_LOW; surface.grid_width_dots];
        let mut engine = Self {
            surface,
            mode,
            physics_rng_state,
            rain_rng_state,
            initial_repose_rng_state: repose_rng_state,
            repose_rng_state,
            local_repose,
            rain_focus_x: None,
            rain_focus_target_x: None,
            rain_focus_move_counter: 0,
            pending_drive: VecDeque::new(),
            frame_count: 0,
            total_generated: 0,
            vertical_moves: 0,
            diagonal_moves: 0,
            #[cfg(test)]
            force_uniform_repose: false,
        };
        engine.resample_all_local_repose();
        engine
    }

    pub(crate) fn model_name(&self) -> &'static str {
        match self.mode {
            ClassicRainMode::Uniform => "classic",
            ClassicRainMode::WanderingFocus => "hybrid",
        }
    }

    pub(crate) fn spawn(&mut self, category_id: CategoryId) {
        self.pending_drive.push_back(category_id);
        self.total_generated = self.total_generated.saturating_add(1);
        self.flush_pending_drive();
    }

    pub(crate) fn update(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
        if self.frame_count.is_multiple_of(2) {
            self.apply_gravity();
        }
        self.flush_pending_drive();
    }

    pub(crate) fn resize(&mut self, width: u16, height: u16) {
        let old_width = self.surface.grid_width_dots;
        let old_repose = self.local_repose.clone();
        self.surface.resize(width, height);
        let new_width = self.surface.grid_width_dots;
        let horizontal_offset = new_width.saturating_sub(old_width) / 2;
        if new_width > old_width {
            let mut expanded_repose = Vec::with_capacity(new_width);
            for _ in 0..new_width {
                expanded_repose.push(self.sample_local_repose());
            }
            for (x, repose) in old_repose.into_iter().enumerate() {
                expanded_repose[x + horizontal_offset] = repose;
            }
            self.local_repose = expanded_repose;
        }
        if horizontal_offset > 0 {
            self.rain_focus_x = self
                .rain_focus_x
                .map(|x| x.saturating_add(horizontal_offset));
            self.rain_focus_target_x = self
                .rain_focus_target_x
                .map(|x| x.saturating_add(horizontal_offset));
        }
        self.flush_pending_drive();
    }

    pub(crate) fn clear(&mut self) {
        self.surface.clear();
        self.pending_drive.clear();
        self.rain_focus_x = None;
        self.rain_focus_target_x = None;
        self.rain_focus_move_counter = 0;
        self.frame_count = 0;
        self.total_generated = 0;
        self.vertical_moves = 0;
        self.diagonal_moves = 0;
        self.repose_rng_state = self.initial_repose_rng_state;
        self.resample_all_local_repose();
    }

    pub(crate) fn render(&self, categories: &[Category]) -> Vec<Line<'static>> {
        self.surface.render(categories)
    }

    pub(crate) fn dimensions(&self) -> (u16, u16) {
        (self.surface.cell_width, self.surface.cell_height)
    }

    pub(crate) fn grain_count(&self) -> usize {
        self.total_generated
    }

    pub(crate) fn physical_grain_count(&self) -> usize {
        self.surface.physical_grain_count()
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.pending_drive.len()
    }

    pub(crate) fn canonical_dimensions(&self) -> (usize, usize) {
        (self.surface.grid_width_dots, self.surface.grid_height_dots)
    }

    pub(crate) fn movement_counts(&self) -> (usize, usize) {
        (self.vertical_moves, self.diagonal_moves)
    }

    pub(crate) fn debug_fill_rainbow_80(
        &mut self,
        category_ids: &[CategoryId],
    ) -> Result<usize, String> {
        if category_ids.is_empty() {
            return Err("testingcheats fill requires at least one configured layer".to_string());
        }

        self.clear();
        let Some(bounds) = self.surface.viewport_bounds() else {
            return Ok(0);
        };
        let visible_height = bounds.y_end.saturating_sub(bounds.y_start);
        let fill_height = visible_height.saturating_mul(4) / 5;
        if fill_height == 0 || bounds.x_start >= bounds.x_end {
            return Ok(0);
        }

        for x in bounds.x_start..bounds.x_end {
            for depth in 0..fill_height {
                let layer = depth.saturating_mul(category_ids.len()) / fill_height;
                let category_id = category_ids[layer.min(category_ids.len() - 1)];
                let y = bounds.y_end - 1 - depth;
                self.surface.grid[y][x] = Some(category_id);
            }
        }

        self.pending_drive.clear();
        self.frame_count = 0;
        self.total_generated = self.surface.physical_grain_count();
        self.vertical_moves = 0;
        self.diagonal_moves = 0;
        self.sync_surface_metadata();
        Ok(self.total_generated)
    }

    fn flush_pending_drive(&mut self) {
        if self.pending_drive.is_empty() {
            return;
        }
        let Some(bounds) = self.surface.viewport_bounds() else {
            return;
        };
        let ingress_y = bounds.y_start;
        let mut free_columns = (bounds.x_start..bounds.x_end)
            .filter(|x| self.surface.grid[ingress_y][*x].is_none())
            .collect::<Vec<_>>();

        while !free_columns.is_empty() && !self.pending_drive.is_empty() {
            let target = self.choose_rain_target(bounds);
            let free_index = self.nearest_free_index(target, &free_columns);
            let x = free_columns.swap_remove(free_index);
            let category_id = self
                .pending_drive
                .pop_front()
                .expect("pending drive exists");
            self.surface.grid[ingress_y][x] = Some(category_id);
        }
        self.sync_surface_metadata();
    }

    fn nearest_free_index(&mut self, target: usize, free_columns: &[usize]) -> usize {
        debug_assert!(!free_columns.is_empty());
        let mut best_index = 0usize;
        let mut best_distance = free_columns[0].abs_diff(target);
        for (index, x) in free_columns.iter().copied().enumerate().skip(1) {
            let distance = x.abs_diff(target);
            if distance < best_distance || (distance == best_distance && self.rain_random_bool()) {
                best_index = index;
                best_distance = distance;
            }
        }
        best_index
    }

    fn choose_rain_target(&mut self, bounds: ViewportBounds) -> usize {
        let width = bounds.x_end.saturating_sub(bounds.x_start);
        debug_assert!(width > 0);
        if self.mode == ClassicRainMode::Uniform {
            return bounds.x_start + self.rain_random_index(width);
        }

        let focus = self.advance_rain_focus(bounds);
        let full_width = bounds.x_start + self.rain_random_index(width);
        if width == 1 || self.rain_random_index(RAIN_FOCUS_BIAS_ONE_IN) != 0 {
            return full_width;
        }
        self.sample_focus_biased_target(bounds, focus)
    }

    fn golden_focus_bounds(bounds: ViewportBounds) -> (usize, usize) {
        let width = bounds.x_end.saturating_sub(bounds.x_start);
        debug_assert!(width > 0);
        if width <= 2 {
            return (bounds.x_start, bounds.x_end);
        }
        let edge_fraction = ((1.0 - 1.0 / GOLDEN_RATIO) / 2.0) / GOLDEN_RATIO;
        let padding = ((width as f64) * edge_fraction)
            .round()
            .min(((width - 1) / 2) as f64) as usize;
        (bounds.x_start + padding, bounds.x_end - padding)
    }

    fn advance_rain_focus(&mut self, bounds: ViewportBounds) -> usize {
        let (start, end) = Self::golden_focus_bounds(bounds);
        let focus_width = end.saturating_sub(start);
        debug_assert!(focus_width > 0);

        let was_initialized = self.rain_focus_x.is_some();
        let mut focus = self.rain_focus_x.map_or_else(
            || start + self.rain_random_index(focus_width),
            |focus| focus.clamp(start, end - 1),
        );

        if !was_initialized {
            self.rain_focus_target_x = Some(self.choose_focus_waypoint(focus, start, end));
            self.rain_focus_move_counter = 0;
            self.rain_focus_x = Some(focus);
            return focus;
        }

        let mut target = self
            .rain_focus_target_x
            .filter(|target| *target >= start && *target < end)
            .unwrap_or_else(|| self.choose_focus_waypoint(focus, start, end));
        if target == focus && focus_width > 1 {
            target = self.choose_focus_waypoint(focus, start, end);
        }
        self.rain_focus_target_x = Some(target);

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
                self.rain_focus_target_x = Some(self.choose_focus_waypoint(focus, start, end));
            }
        }

        self.rain_focus_x = Some(focus);
        focus
    }

    fn choose_focus_waypoint(&mut self, focus: usize, start: usize, end: usize) -> usize {
        let focus_width = end.saturating_sub(start);
        if focus_width <= 1 {
            return start;
        }
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

    fn sample_focus_biased_target(&mut self, bounds: ViewportBounds, focus: usize) -> usize {
        let (start, end) = Self::golden_focus_bounds(bounds);
        let focus_width = end.saturating_sub(start);
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

    fn apply_gravity(&mut self) {
        let Some(bounds) = self.surface.viewport_bounds() else {
            return;
        };
        if bounds.y_end.saturating_sub(bounds.y_start) < 2 {
            return;
        }

        let base_left_to_right = self.surface.sweep_left_to_right;
        self.surface.sweep_left_to_right = !self.surface.sweep_left_to_right;

        for y in (bounds.y_start..bounds.y_end - 1).rev() {
            let left_to_right = if y.is_multiple_of(2) {
                base_left_to_right
            } else {
                !base_left_to_right
            };
            if left_to_right {
                for x in bounds.x_start..bounds.x_end {
                    self.move_grain_once(bounds, x, y);
                }
            } else {
                for x in (bounds.x_start..bounds.x_end).rev() {
                    self.move_grain_once(bounds, x, y);
                }
            }
        }
        self.sync_surface_metadata();
    }

    fn move_grain_once(&mut self, bounds: ViewportBounds, x: usize, y: usize) {
        let Some(category_id) = self.surface.grid[y][x] else {
            return;
        };
        if self.surface.grid[y + 1][x].is_none() {
            self.surface.grid[y][x] = None;
            self.surface.grid[y + 1][x] = Some(category_id);
            self.vertical_moves = self.vertical_moves.saturating_add(1);
            return;
        }

        // This intentionally reproduces the pre-pause rule: choose exactly one
        // diagonal. If that side is blocked, the grain waits for a later tick
        // rather than trying the opposite side in the same update.
        let step = if self.physics_random_bool() {
            1isize
        } else {
            -1isize
        };
        let Some(target_x) = x.checked_add_signed(step) else {
            return;
        };
        if target_x < bounds.x_start || target_x >= bounds.x_end {
            return;
        }
        if self.surface.grid[y + 1][target_x].is_some() {
            return;
        }
        if !self.local_repose_allows_diagonal(bounds, x, target_x, y) {
            return;
        }
        self.surface.grid[y][x] = None;
        self.surface.grid[y + 1][target_x] = Some(category_id);
        self.diagonal_moves = self.diagonal_moves.saturating_add(1);
        self.refresh_local_repose(x);
        self.refresh_local_repose(target_x);
    }

    fn local_repose_allows_diagonal(
        &self,
        bounds: ViewportBounds,
        source_x: usize,
        target_x: usize,
        source_y: usize,
    ) -> bool {
        if self.local_repose[source_x] <= CLASSIC_REPOSE_LOW {
            return true;
        }

        // Classic's original diagonal vacancy test is equivalent to allowing a
        // compact surface to relax when the local height relief is at least two.
        // A rare repose=2 column tolerates exactly one extra relief unit, so the
        // cell one step below the ordinary diagonal target must also be empty.
        let deeper_y = source_y + 2;
        deeper_y < bounds.y_end && self.surface.grid[deeper_y][target_x].is_none()
    }

    fn resample_all_local_repose(&mut self) {
        for x in 0..self.local_repose.len() {
            let repose = self.sample_local_repose();
            self.local_repose[x] = repose;
        }
    }

    fn refresh_local_repose(&mut self, x: usize) {
        if x >= self.local_repose.len() {
            return;
        }
        let repose = self.sample_local_repose();
        self.local_repose[x] = repose;
    }

    fn sample_local_repose(&mut self) -> u8 {
        #[cfg(test)]
        if self.force_uniform_repose {
            return CLASSIC_REPOSE_LOW;
        }

        if self.next_repose_random_u64() as usize % CLASSIC_HIGH_REPOSE_ONE_IN == 0 {
            CLASSIC_REPOSE_HIGH
        } else {
            CLASSIC_REPOSE_LOW
        }
    }

    fn sync_surface_metadata(&mut self) {
        for row in &mut self.surface.mobilized {
            row.fill(false);
        }
        self.surface.pending_runs.clear();
        self.surface.ingress_focus_x = None;
        self.surface.grain_count = self
            .surface
            .physical_grain_count()
            .saturating_add(self.pending_drive.len());
    }

    fn next_physics_random_u64(&mut self) -> u64 {
        let mut x = self.physics_rng_state;
        if x == 0 {
            x = CLASSIC_PHYSICS_RNG_XOR;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.physics_rng_state = x;
        x
    }

    fn physics_random_bool(&mut self) -> bool {
        self.next_physics_random_u64() & 1 == 0
    }

    fn next_repose_random_u64(&mut self) -> u64 {
        let mut x = self.repose_rng_state;
        if x == 0 {
            x = CLASSIC_REPOSE_RNG_XOR;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.repose_rng_state = x;
        x
    }

    fn next_rain_random_u64(&mut self) -> u64 {
        let mut x = self.rain_rng_state;
        if x == 0 {
            x = CLASSIC_RAIN_RNG_XOR;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rain_rng_state = x;
        x
    }

    fn rain_random_index(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next_rain_random_u64() as usize) % upper
    }

    fn rain_random_bool(&mut self) -> bool {
        self.next_rain_random_u64() & 1 == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mass(engine: &ClassicSandboxEngine) -> usize {
        engine.physical_grain_count() + engine.pending_count()
    }

    #[test]
    fn classic_fill_rainbow_80_builds_exact_bottom_anchored_layers() {
        let mut engine = ClassicSandboxEngine::new(10, 10, 7, ClassicRainMode::Uniform);
        let categories = [
            CategoryId(1),
            CategoryId(2),
            CategoryId(3),
            CategoryId(4),
            CategoryId(5),
            CategoryId(6),
        ];

        let grains = engine
            .debug_fill_rainbow_80(&categories)
            .expect("classic fixture fill");
        let bounds = engine.surface.viewport_bounds().expect("visible bounds");
        let visible_height = bounds.y_end - bounds.y_start;
        let fill_height = visible_height * 4 / 5;
        let visible_width = bounds.x_end - bounds.x_start;

        assert_eq!(grains, visible_width * fill_height);
        assert_eq!(engine.grain_count(), grains);
        assert_eq!(engine.physical_grain_count(), grains);
        assert_eq!(engine.pending_count(), 0);
        assert_eq!(engine.movement_counts(), (0, 0));

        for x in bounds.x_start..bounds.x_end {
            for y in bounds.y_start..bounds.y_end - fill_height {
                assert_eq!(engine.surface.grid[y][x], None);
            }
            for depth in 0..fill_height {
                let layer = depth * categories.len() / fill_height;
                let expected = categories[layer.min(categories.len() - 1)];
                let y = bounds.y_end - 1 - depth;
                assert_eq!(engine.surface.grid[y][x], Some(expected));
            }
        }
    }

    #[test]
    fn classic_fill_rainbow_80_is_repeatable_and_rejects_empty_layers() {
        let mut engine = ClassicSandboxEngine::new(8, 6, 11, ClassicRainMode::Uniform);
        let categories = [CategoryId(11), CategoryId(12)];

        let first = engine
            .debug_fill_rainbow_80(&categories)
            .expect("first classic fixture fill");
        for _ in 0..8 {
            engine.update();
        }
        let second = engine
            .debug_fill_rainbow_80(&categories)
            .expect("second classic fixture fill");

        assert_eq!(second, first);
        assert_eq!(engine.grain_count(), first);
        assert_eq!(engine.movement_counts(), (0, 0));
        assert!(engine.debug_fill_rainbow_80(&[]).is_err());
    }

    #[test]
    fn classic_closed_visible_walls_never_discharge_mass() {
        let mut engine = ClassicSandboxEngine::new(12, 8, 7, ClassicRainMode::Uniform);
        let category = CategoryId(1);
        for _ in 0..2_000 {
            engine.spawn(category);
            for _ in 0..4 {
                engine.update();
            }
        }
        assert_eq!(mass(&engine), engine.grain_count());
    }

    #[test]
    fn classic_shrink_freezes_hidden_canonical_terrain_and_reexpand_restores_it() {
        let mut engine = ClassicSandboxEngine::new(20, 8, 7, ClassicRainMode::Uniform);
        let bounds = engine.surface.viewport_bounds().expect("visible basin");
        let hidden_x = bounds.x_start;
        let floor = bounds.y_end - 1;
        engine.surface.grid[floor][hidden_x] = Some(CategoryId(1));
        let original_width = engine.surface.grid_width_dots;

        engine.resize(10, 8);
        assert_eq!(engine.surface.grid_width_dots, original_width);
        let shrunk = engine.surface.viewport_bounds().expect("shrunk basin");
        assert!(hidden_x < shrunk.x_start || hidden_x >= shrunk.x_end);
        for _ in 0..20 {
            engine.update();
        }
        assert_eq!(engine.surface.grid[floor][hidden_x], Some(CategoryId(1)));

        engine.resize(20, 8);
        assert_eq!(engine.surface.grid[floor][hidden_x], Some(CategoryId(1)));
    }

    #[test]
    fn shrink_wall_blocks_hidden_diagonal_until_reexpansion() {
        let mut engine = ClassicSandboxEngine::new(20, 8, 13, ClassicRainMode::Uniform);
        engine.resize(10, 8);
        let shrunk = engine.surface.viewport_bounds().expect("shrunk basin");
        let wall_x = shrunk.x_start;
        let hidden_x = wall_x - 1;
        let floor = shrunk.y_end - 1;
        let category = CategoryId(1);

        engine.surface.grid[floor][wall_x] = Some(category);
        engine.surface.grid[floor - 1][wall_x] = Some(category);
        engine.surface.grid[floor][wall_x + 1] = Some(category);
        assert_eq!(engine.surface.grid[floor][hidden_x], None);

        for _ in 0..64 {
            engine.apply_gravity();
        }
        assert_eq!(engine.surface.grid[floor][hidden_x], None);
        assert_eq!(engine.surface.grid[floor - 1][wall_x], Some(category));

        engine.resize(20, 8);
        for _ in 0..128 {
            engine.apply_gravity();
            if engine.surface.grid[floor][hidden_x] == Some(category) {
                break;
            }
        }
        assert_eq!(engine.surface.grid[floor][hidden_x], Some(category));
    }

    #[test]
    fn classic_side_wall_is_not_an_outside_height_zero_sink() {
        let mut engine = ClassicSandboxEngine::new(8, 6, 3, ClassicRainMode::Uniform);
        let bounds = engine.surface.viewport_bounds().expect("visible basin");
        let x = bounds.x_start;
        let floor = bounds.y_end - 1;
        engine.surface.grid[floor][x] = Some(CategoryId(1));
        engine.surface.grid[floor - 1][x] = Some(CategoryId(1));
        let before = mass(&engine);
        for _ in 0..20 {
            engine.apply_gravity();
        }
        assert_eq!(mass(&engine), before);
        assert!(engine.surface.grid.iter().any(|row| row[x].is_some()));
    }

    #[test]
    fn hybrid_focus_is_only_a_ten_percent_ingress_bias() {
        assert_eq!(RAIN_FOCUS_BIAS_ONE_IN, 10);
        let mut engine = ClassicSandboxEngine::new(40, 20, 11, ClassicRainMode::WanderingFocus);
        let bounds = engine.surface.viewport_bounds().expect("visible basin");
        let (start, end) = ClassicSandboxEngine::golden_focus_bounds(bounds);
        for _ in 0..10_000 {
            let focus = engine.advance_rain_focus(bounds);
            assert!(focus >= start && focus < end);
        }
    }

    #[test]
    fn uniform_and_hybrid_share_identical_gravity_law() {
        let mut uniform = ClassicSandboxEngine::new(10, 8, 19, ClassicRainMode::Uniform);
        let mut hybrid = ClassicSandboxEngine::new(10, 8, 19, ClassicRainMode::WanderingFocus);
        let bounds = uniform.surface.viewport_bounds().expect("visible basin");
        let floor = bounds.y_end - 1;
        for (x, y) in [
            (bounds.x_start + 2, floor),
            (bounds.x_start + 2, floor - 1),
            (bounds.x_start + 3, floor),
            (bounds.x_start + 4, floor),
            (bounds.x_start + 4, floor - 1),
            (bounds.x_start + 4, floor - 2),
        ] {
            uniform.surface.grid[y][x] = Some(CategoryId(1));
            hybrid.surface.grid[y][x] = Some(CategoryId(1));
        }
        for _ in 0..20 {
            uniform.apply_gravity();
            hybrid.apply_gravity();
        }
        assert_eq!(uniform.surface.grid, hybrid.surface.grid);
    }
}

#[cfg(test)]
#[path = "classic/repose_tests.rs"]
mod repose_tests;
