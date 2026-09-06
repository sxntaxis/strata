use std::collections::VecDeque;

use ratatui::prelude::Line;

use crate::domain::{Category, CategoryId};

use super::{SandEngine, ViewportBounds, centered_half_open_interval};

mod grounded_index;
use grounded_index::GroundedColumnIndex;

const CLASSIC_PHYSICS_RNG_XOR: u64 = 0xC6BC_2796_92B5_CC83;
const CLASSIC_RAIN_RNG_XOR: u64 = 0xD1B5_4A32_D192_ED03;
const CLASSIC_REPOSE_RNG_XOR: u64 = 0xA5A5_5A5A_D3C1_B7E9;
const CLASSIC_REPOSE_LOW: u8 = 1;
const CLASSIC_REPOSE_MID: u8 = 2;
const CLASSIC_REPOSE_HIGH: u8 = 3;
const CLASSIC_REPOSE_ANCHOR: u8 = 4;
const GOLDEN_RATIO: f64 = 1.618_033_988_749_895;
const RAIN_FOCUS_BIAS_ONE_IN: usize = 10;
const CLASSIC_MOMENTUM_MIN_DROP_DEPTH: usize = 2;
const CLASSIC_MOMENTUM_MAX_DROP_DEPTH: usize = 3;
// At one ingress per second, a full focus traverse takes about twelve hours.
const RAIN_FOCUS_EDGE_TO_EDGE_INGRESSES: usize = 43_200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClassicRainMode {
    Uniform,
    WanderingFocus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassicTextureProfile {
    Baseline,
    Textured,
    Rugged,
    Terraced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassicExperimentProfile {
    Rugged,
    Memory,
    Slope,
    MemorySlope,
    Anchored,
    Momentum,
    MomentumRepose,
    MomentumTangent,
    MomentumSoft,
    MomentumContact,
    MomentumReposeContact,
    MomentumSurface,
    MomentumGroundedContact,
}

impl ClassicExperimentProfile {
    fn name(self) -> &'static str {
        match self {
            Self::Rugged => "rugged",
            Self::Memory => "memory",
            Self::Slope => "slope",
            Self::MemorySlope => "memory-slope",
            Self::Anchored => "anchored",
            Self::Momentum => "momentum",
            Self::MomentumRepose => "momentum-repose",
            Self::MomentumTangent => "momentum-tangent",
            Self::MomentumSoft => "momentum-soft",
            Self::MomentumContact => "momentum-contact",
            Self::MomentumReposeContact => "momentum-repose-contact",
            Self::MomentumSurface => "momentum-surface",
            Self::MomentumGroundedContact => "momentum-grounded-contact",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "rugged" => Some(Self::Rugged),
            "memory" => Some(Self::Memory),
            "slope" => Some(Self::Slope),
            "memory-slope" => Some(Self::MemorySlope),
            "anchored" => Some(Self::Anchored),
            "momentum" => Some(Self::Momentum),
            "momentum-repose" => Some(Self::MomentumRepose),
            "momentum-tangent" => Some(Self::MomentumTangent),
            "momentum-soft" => Some(Self::MomentumSoft),
            "momentum-contact" => Some(Self::MomentumContact),
            "momentum-repose-contact" => Some(Self::MomentumReposeContact),
            "momentum-surface" => Some(Self::MomentumSurface),
            "momentum-grounded-contact" => Some(Self::MomentumGroundedContact),
            _ => None,
        }
    }

    fn uses_memory(self) -> bool {
        matches!(
            self,
            Self::Memory
                | Self::MemorySlope
                | Self::Anchored
                | Self::Momentum
                | Self::MomentumRepose
                | Self::MomentumTangent
                | Self::MomentumSoft
                | Self::MomentumContact
                | Self::MomentumReposeContact
                | Self::MomentumSurface
                | Self::MomentumGroundedContact
        )
    }

    fn uses_slope_bias(self) -> bool {
        matches!(
            self,
            Self::Slope
                | Self::MemorySlope
                | Self::Anchored
                | Self::Momentum
                | Self::MomentumRepose
                | Self::MomentumTangent
                | Self::MomentumSoft
                | Self::MomentumContact
                | Self::MomentumReposeContact
                | Self::MomentumSurface
                | Self::MomentumGroundedContact
        )
    }

    fn uses_anchors(self) -> bool {
        matches!(
            self,
            Self::Anchored
                | Self::Momentum
                | Self::MomentumRepose
                | Self::MomentumTangent
                | Self::MomentumSoft
                | Self::MomentumContact
                | Self::MomentumReposeContact
                | Self::MomentumSurface
                | Self::MomentumGroundedContact
        )
    }

    fn uses_momentum(self) -> bool {
        matches!(
            self,
            Self::Momentum
                | Self::MomentumRepose
                | Self::MomentumTangent
                | Self::MomentumSoft
                | Self::MomentumContact
                | Self::MomentumReposeContact
                | Self::MomentumSurface
                | Self::MomentumGroundedContact
        )
    }

    fn momentum_requires_grounded_blocker(self) -> bool {
        matches!(
            self,
            Self::MomentumContact
                | Self::MomentumReposeContact
                | Self::MomentumSurface
                | Self::MomentumGroundedContact
        )
    }

    fn ordinary_diagonal_requires_grounded_blocker(self) -> bool {
        matches!(self, Self::MomentumGroundedContact)
    }

    fn momentum_requires_grounded_receiving_support(self) -> bool {
        matches!(self, Self::MomentumSurface)
    }
}

impl ClassicTextureProfile {
    fn name(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Textured => "textured",
            Self::Rugged => "rugged",
            Self::Terraced => "terraced",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "baseline" => Some(Self::Baseline),
            "textured" => Some(Self::Textured),
            "rugged" => Some(Self::Rugged),
            "terraced" => Some(Self::Terraced),
            _ => None,
        }
    }

    fn continuation_percent(self) -> u8 {
        match self {
            Self::Baseline => 0,
            Self::Textured => 45,
            Self::Rugged => 60,
            Self::Terraced => 78,
        }
    }
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
    repose_memory_remaining: Vec<u8>,
    texture_profile: ClassicTextureProfile,
    experiment_profile: ClassicExperimentProfile,
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
        let repose_memory_remaining = vec![0; surface.grid_width_dots];
        let mut engine = Self {
            surface,
            mode,
            physics_rng_state,
            rain_rng_state,
            initial_repose_rng_state: repose_rng_state,
            repose_rng_state,
            local_repose,
            repose_memory_remaining,
            texture_profile: ClassicTextureProfile::Rugged,
            experiment_profile: ClassicExperimentProfile::MomentumGroundedContact,
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
        let old_memory = self.repose_memory_remaining.clone();
        self.surface.resize(width, height);
        let new_width = self.surface.grid_width_dots;
        let horizontal_offset = new_width.saturating_sub(old_width) / 2;
        if new_width > old_width {
            let mut expanded_repose = Vec::with_capacity(new_width);
            for _ in 0..new_width {
                expanded_repose.push(self.sample_base_local_repose());
            }
            for (x, repose) in old_repose.into_iter().enumerate() {
                expanded_repose[x + horizontal_offset] = repose;
            }
            self.local_repose = expanded_repose;

            let mut expanded_memory = vec![self.experiment_memory_refreshes(); new_width];
            for (x, remaining) in old_memory.into_iter().enumerate() {
                expanded_memory[x + horizontal_offset] = remaining;
            }
            self.repose_memory_remaining = expanded_memory;
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

    pub(crate) fn texture_profile_name(&self) -> &'static str {
        self.texture_profile.name()
    }

    pub(crate) fn set_texture_profile_name(&mut self, profile: &str) -> Result<(), String> {
        let Some(profile) = ClassicTextureProfile::parse(profile) else {
            return Err(
                "classic texture profile must be baseline, textured, rugged, or terraced"
                    .to_string(),
            );
        };
        // Deliberately do not rewrite the live repose field. The profile becomes
        // authoritative for future resamples; `testingcheats fill` starts a clean
        // comparison from the selected preset.
        self.texture_profile = profile;
        Ok(())
    }

    pub(crate) fn experiment_profile_name(&self) -> &'static str {
        self.experiment_profile.name()
    }

    pub(crate) fn set_experiment_profile_name(&mut self, profile: &str) -> Result<(), String> {
        let Some(profile) = ClassicExperimentProfile::parse(profile) else {
            return Err(
                "classic experiment profile must be rugged, memory, slope, memory-slope, anchored, momentum, momentum-repose, momentum-tangent, momentum-soft, momentum-contact, momentum-repose-contact, momentum-surface, or momentum-grounded-contact"
                    .to_string(),
            );
        };
        // Every experiment intentionally starts from the owner-selected rugged
        // texture baseline. Selection remains non-retroactive; `fill` performs
        // the clean field resample for comparison.
        self.texture_profile = ClassicTextureProfile::Rugged;
        self.experiment_profile = profile;
        Ok(())
    }

    pub(crate) fn debug_fill_rainbow_80(
        &mut self,
        category_ids: &[CategoryId],
    ) -> Result<usize, String> {
        self.debug_fill_rainbow_80_with_span(category_ids, false)
    }

    pub(crate) fn debug_fill_rainbow_80_centered_half(
        &mut self,
        category_ids: &[CategoryId],
    ) -> Result<usize, String> {
        self.debug_fill_rainbow_80_with_span(category_ids, true)
    }

    fn debug_fill_rainbow_80_with_span(
        &mut self,
        category_ids: &[CategoryId],
        centered_half_width: bool,
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

        let (fill_x_start, fill_x_end) = if centered_half_width {
            centered_half_open_interval(bounds.x_start, bounds.x_end)
        } else {
            (bounds.x_start, bounds.x_end)
        };

        for x in fill_x_start..fill_x_end {
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
        self.apply_gravity_with_grounded_index(true);
    }

    fn apply_gravity_with_grounded_index(&mut self, use_grounded_index: bool) {
        let Some(bounds) = self.surface.viewport_bounds() else {
            return;
        };
        if bounds.y_end.saturating_sub(bounds.y_start) < 2 {
            return;
        }

        // CLASSIC-012 performance path: the accepted grounded-contact baseline
        // otherwise rescans from nearly every direct blocker to the visible floor.
        // Build one exact sweep-local bottom-connectivity index and update it after
        // each in-place move. Existing profiles do not pay for or consult it.
        let mut grounded_index = (use_grounded_index
            && self
                .experiment_profile
                .ordinary_diagonal_requires_grounded_blocker())
        .then(|| GroundedColumnIndex::from_grid(&self.surface.grid, bounds));

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
                    self.move_grain_once_with_grounded_index(
                        bounds,
                        x,
                        y,
                        grounded_index.as_mut(),
                    );
                }
            } else {
                for x in (bounds.x_start..bounds.x_end).rev() {
                    self.move_grain_once_with_grounded_index(
                        bounds,
                        x,
                        y,
                        grounded_index.as_mut(),
                    );
                }
            }
        }
        self.sync_surface_metadata();
    }

    #[cfg(test)]
    fn apply_gravity_uncached_for_test(&mut self) {
        self.apply_gravity_with_grounded_index(false);
    }

    fn move_grain_once(&mut self, bounds: ViewportBounds, x: usize, y: usize) {
        self.move_grain_once_with_grounded_index(bounds, x, y, None);
    }

    fn move_grain_once_with_grounded_index(
        &mut self,
        bounds: ViewportBounds,
        x: usize,
        y: usize,
        mut grounded_index: Option<&mut GroundedColumnIndex>,
    ) {
        let Some(category_id) = self.surface.grid[y][x] else {
            return;
        };
        if self.surface.grid[y + 1][x].is_none() {
            self.surface.grid[y][x] = None;
            self.surface.grid[y + 1][x] = Some(category_id);
            if let Some(index) = grounded_index.as_deref_mut() {
                index.record_move(&self.surface.grid, x, y, x, y + 1);
            }
            self.vertical_moves = self.vertical_moves.saturating_add(1);
            return;
        }

        // CLASSIC-011 diagnostic: for the grounded-contact profile, catching
        // another grain in free fall is not surface contact. Wait one gravity
        // sweep instead of converting that transient collision into a lateral
        // Classic step. Existing profiles preserve their ordinary diagonal law.
        let ordinary_blocker_grounded = if self
            .experiment_profile
            .ordinary_diagonal_requires_grounded_blocker()
        {
            let grounded = grounded_index.as_deref().map_or_else(
                || self.direct_blocker_is_grounded(bounds, x, y + 1),
                |index| index.is_grounded(&self.surface.grid, x, y + 1),
            );
            if !grounded {
                return;
            }
            true
        } else {
            false
        };

        // Classic still commits at most one ordinary diagonal choice per grain.
        // Experimental profiles may bias that choice toward the steeper side,
        // but never introduce a second-side retry when the chosen side is blocked.
        let step = self.choose_diagonal_step(bounds, x, y);
        let Some(target_x) = x.checked_add_signed(step) else {
            return;
        };
        if !self.classic_diagonal_is_available(bounds, x, target_x, y) {
            return;
        }

        // Contact-gated momentum distinguishes a grain arriving at the actual
        // supported pile from one merely catching another grain in flight.
        // Ordinary Classic diagonal behavior remains unchanged either way.
        let momentum_blocker_grounded =
            if self.experiment_profile.momentum_requires_grounded_blocker() {
                if ordinary_blocker_grounded && grounded_index.is_some() {
                    true
                } else {
                    self.direct_blocker_is_grounded(bounds, x, y + 1)
                }
            } else {
                true
            };

        self.surface.grid[y][x] = None;
        self.surface.grid[y + 1][target_x] = Some(category_id);
        if let Some(index) = grounded_index.as_deref_mut() {
            index.record_move(&self.surface.grid, x, y, target_x, y + 1);
        }
        self.diagonal_moves = self.diagonal_moves.saturating_add(1);

        if self.experiment_profile.uses_momentum() {
            self.try_one_bonus_diagonal(
                bounds,
                target_x,
                y + 1,
                step,
                category_id,
                momentum_blocker_grounded,
                grounded_index.as_deref_mut(),
            );
        }

        self.refresh_local_repose(x);
        self.refresh_local_repose(target_x);
    }

    fn direct_blocker_is_grounded(
        &self,
        bounds: ViewportBounds,
        x: usize,
        blocker_y: usize,
    ) -> bool {
        if blocker_y >= bounds.y_end || self.surface.grid[blocker_y][x].is_none() {
            return false;
        }
        // Classic terrain settles into bottom-connected vertical columns. A
        // blocker with any air gap beneath it is still airborne for the narrow
        // purpose of deciding whether momentum may add a second diagonal hop.
        // This does not change whether the ordinary Classic diagonal happens.
        (blocker_y..bounds.y_end).all(|row| self.surface.grid[row][x].is_some())
    }

    // CLASSIC-010: the bonus may follow relief only when the first occupied
    // support below its proposed destination is itself bottom-connected pile.
    // This is a bonus-only surface gate; ordinary Classic motion is untouched.
    fn first_support_below_is_grounded(
        &self,
        bounds: ViewportBounds,
        x: usize,
        destination_y: usize,
    ) -> bool {
        let Some(support_y) = (destination_y.saturating_add(1)..bounds.y_end)
            .find(|row| self.surface.grid[*row][x].is_some())
        else {
            return false;
        };
        self.direct_blocker_is_grounded(bounds, x, support_y)
    }

    fn choose_diagonal_step(&mut self, bounds: ViewportBounds, x: usize, y: usize) -> isize {
        let random_step = if self.physics_random_bool() {
            1isize
        } else {
            -1isize
        };
        if !self.experiment_profile.uses_slope_bias() {
            return random_step;
        }

        let left = x.checked_sub(1).filter(|target| *target >= bounds.x_start);
        let right = (x + 1 < bounds.x_end).then_some(x + 1);
        let left_open = left.is_some_and(|target| self.surface.grid[y + 1][target].is_none());
        let right_open = right.is_some_and(|target| self.surface.grid[y + 1][target].is_none());
        // Preserve Classic's one-side behavior exactly. Relief only informs the
        // choice when both ordinary diagonals are actually available.
        if !left_open || !right_open {
            return random_step;
        }
        let left_drop = self.diagonal_drop_depth(bounds, left.expect("left open"), y);
        let right_drop = self.diagonal_drop_depth(bounds, right.expect("right open"), y);
        if left_drop == right_drop {
            return random_step;
        }

        // Mild 3:1 preference. The remaining quarter preserves Classic's
        // stochastic character and prevents relief from becoming deterministic.
        let prefer_steeper = !self.next_physics_random_u64().is_multiple_of(4);
        if prefer_steeper {
            if left_drop > right_drop { -1 } else { 1 }
        } else {
            random_step
        }
    }

    fn diagonal_drop_depth(
        &self,
        bounds: ViewportBounds,
        target_x: usize,
        source_y: usize,
    ) -> usize {
        let mut depth = 0usize;
        for y in source_y + 1..bounds.y_end {
            if self.surface.grid[y][target_x].is_some() {
                break;
            }
            depth += 1;
            if depth >= 8 {
                break;
            }
        }
        depth
    }

    fn classic_diagonal_is_available(
        &self,
        bounds: ViewportBounds,
        source_x: usize,
        target_x: usize,
        source_y: usize,
    ) -> bool {
        target_x >= bounds.x_start
            && target_x < bounds.x_end
            && self.surface.grid[source_y + 1][target_x].is_none()
            && self.local_repose_allows_diagonal(bounds, source_x, target_x, source_y)
    }

    fn try_one_bonus_diagonal(
        &mut self,
        bounds: ViewportBounds,
        x: usize,
        y: usize,
        step: isize,
        category_id: CategoryId,
        blocker_grounded: bool,
        grounded_index: Option<&mut GroundedColumnIndex>,
    ) {
        if !blocker_grounded {
            return;
        }
        let Some(next_x) = x.checked_add_signed(step) else {
            return;
        };
        if y + 1 >= bounds.y_end || !self.classic_diagonal_is_available(bounds, x, next_x, y) {
            return;
        }
        let drop_depth = self.diagonal_drop_depth(bounds, next_x, y);
        if !self.momentum_bonus_is_eligible(bounds, x, next_x, y, step, drop_depth) {
            return;
        }
        // The owner-observed residual lane artifact can occur after the original
        // grounded-contact check: a legal bonus may still chase an airborne
        // receiving support. `momentum-surface` rejects only that second hop.
        if self
            .experiment_profile
            .momentum_requires_grounded_receiving_support()
            && !self.first_support_below_is_grounded(bounds, next_x, y + 1)
        {
            return;
        }
        self.surface.grid[y][x] = None;
        self.surface.grid[y + 1][next_x] = Some(category_id);
        if let Some(index) = grounded_index {
            index.record_move(&self.surface.grid, x, y, next_x, y + 1);
        }
        self.diagonal_moves = self.diagonal_moves.saturating_add(1);
        self.refresh_local_repose(next_x);
    }

    fn momentum_bonus_is_eligible(
        &mut self,
        bounds: ViewportBounds,
        source_x: usize,
        target_x: usize,
        source_y: usize,
        step: isize,
        drop_depth: usize,
    ) -> bool {
        match self.experiment_profile {
            ClassicExperimentProfile::Momentum | ClassicExperimentProfile::MomentumContact => {
                // CLASSIC-007 control: a fixed local slope band removes the
                // deep early-fill cliff artifact while preserving the accepted
                // mature one-hop continuation.
                (CLASSIC_MOMENTUM_MIN_DROP_DEPTH..=CLASSIC_MOMENTUM_MAX_DROP_DEPTH)
                    .contains(&drop_depth)
            }
            ClassicExperimentProfile::MomentumRepose
            | ClassicExperimentProfile::MomentumReposeContact
            | ClassicExperimentProfile::MomentumSurface
            | ClassicExperimentProfile::MomentumGroundedContact => {
                // A: reuse Classic's own local stability threshold. Momentum is
                // eligible only on relief at, or one dot beyond, the source
                // column's local repose instead of using a globally fixed band.
                let repose = usize::from(self.local_repose[source_x]);
                (repose..=repose.saturating_add(1)).contains(&drop_depth)
            }
            ClassicExperimentProfile::MomentumTangent => {
                // B: require the receiving surface and one forward sample to
                // form a bounded, locally continuous diagonal. This is only a
                // one-column lookahead gate; it never creates another hop.
                if drop_depth == 0 || drop_depth > CLASSIC_MOMENTUM_MAX_DROP_DEPTH {
                    return false;
                }
                let Some(forward_x) = target_x.checked_add_signed(step) else {
                    return false;
                };
                if forward_x < bounds.x_start || forward_x >= bounds.x_end {
                    return false;
                }
                let forward_drop =
                    self.diagonal_drop_depth(bounds, forward_x, source_y.saturating_add(1));
                forward_drop > 0
                    && forward_drop <= CLASSIC_MOMENTUM_MAX_DROP_DEPTH
                    && drop_depth.abs_diff(forward_drop) <= 1
            }
            ClassicExperimentProfile::MomentumSoft => {
                // C: turn the repose-relative upper edge into a soft transition.
                // Exact local repose always continues, one extra dot continues
                // 60% of the time, and anything steeper is treated as a cliff.
                let repose = usize::from(self.local_repose[source_x]);
                match drop_depth.checked_sub(repose) {
                    Some(0) => true,
                    Some(1) => self.next_physics_random_u64() % 5 < 3,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn local_repose_allows_diagonal(
        &self,
        bounds: ViewportBounds,
        source_x: usize,
        target_x: usize,
        source_y: usize,
    ) -> bool {
        let repose = usize::from(self.local_repose[source_x]);
        if repose <= usize::from(CLASSIC_REPOSE_LOW) {
            return true;
        }

        // Classic's original diagonal vacancy test is equivalent to repose=1.
        // Higher presets remain the same one-diagonal Classic law: they merely
        // require one or two extra empty cells below the ordinary target before
        // release. No rolling layer, second-side retry, or Oslo toppling exists.
        for extra_depth in 2..=repose {
            let Some(deeper_y) = source_y.checked_add(extra_depth) else {
                return false;
            };
            if deeper_y >= bounds.y_end || self.surface.grid[deeper_y][target_x].is_some() {
                return false;
            }
        }
        true
    }

    fn resample_all_local_repose(&mut self) {
        let mut previous = CLASSIC_REPOSE_LOW;
        let memory = self.experiment_memory_refreshes();
        for x in 0..self.local_repose.len() {
            let repose = if x > 0 && self.texture_patch_continues() {
                previous
            } else {
                self.sample_base_local_repose()
            };
            self.local_repose[x] = repose;
            self.repose_memory_remaining[x] = memory;
            previous = repose;
        }
    }

    fn refresh_local_repose(&mut self, x: usize) {
        if x >= self.local_repose.len() {
            return;
        }
        if self.experiment_profile.uses_memory() && self.repose_memory_remaining[x] > 0 {
            self.repose_memory_remaining[x] -= 1;
            return;
        }

        let repose = if self.texture_patch_continues() {
            let left = x.checked_sub(1).map(|index| self.local_repose[index]);
            let right = (x + 1 < self.local_repose.len()).then(|| self.local_repose[x + 1]);
            match (left, right) {
                (Some(left), Some(right)) => {
                    if self.next_repose_random_u64() & 1 == 0 {
                        left
                    } else {
                        right
                    }
                }
                (Some(value), None) | (None, Some(value)) => value,
                (None, None) => self.sample_base_local_repose(),
            }
        } else {
            self.sample_base_local_repose()
        };
        self.local_repose[x] = repose;
        self.repose_memory_remaining[x] = self.experiment_memory_refreshes();
    }

    fn experiment_memory_refreshes(&self) -> u8 {
        if self.experiment_profile.uses_memory() {
            3
        } else {
            0
        }
    }

    fn texture_patch_continues(&mut self) -> bool {
        let percent = self.texture_profile.continuation_percent();
        percent > 0 && self.next_repose_random_u64() % 100 < u64::from(percent)
    }

    fn sample_base_local_repose(&mut self) -> u8 {
        #[cfg(test)]
        if self.force_uniform_repose {
            return CLASSIC_REPOSE_LOW;
        }

        let random = self.next_repose_random_u64();
        if self.experiment_profile.uses_anchors() {
            return match random % 1000 {
                0..=899 => CLASSIC_REPOSE_LOW,
                900..=979 => CLASSIC_REPOSE_MID,
                980..=994 => CLASSIC_REPOSE_HIGH,
                _ => CLASSIC_REPOSE_ANCHOR,
            };
        }
        match self.texture_profile {
            // Preserve CLASSIC-002's exact accepted mapping byte-for-byte in RNG
            // consumption and 1-in-20 threshold selection.
            ClassicTextureProfile::Baseline => {
                if random.is_multiple_of(20) {
                    CLASSIC_REPOSE_MID
                } else {
                    CLASSIC_REPOSE_LOW
                }
            }
            ClassicTextureProfile::Textured => match random % 100 {
                0..=92 => CLASSIC_REPOSE_LOW,
                93..=98 => CLASSIC_REPOSE_MID,
                _ => CLASSIC_REPOSE_HIGH,
            },
            ClassicTextureProfile::Rugged | ClassicTextureProfile::Terraced => match random % 100 {
                0..=89 => CLASSIC_REPOSE_LOW,
                90..=97 => CLASSIC_REPOSE_MID,
                _ => CLASSIC_REPOSE_HIGH,
            },
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
    fn classic_fillhalf_uses_centered_half_width_with_identical_vertical_layers() {
        let mut engine = ClassicSandboxEngine::new(11, 10, 13, ClassicRainMode::Uniform);
        let categories = [CategoryId(1), CategoryId(2), CategoryId(3)];
        let bounds = engine.surface.viewport_bounds().expect("visible bounds");
        let visible_width = bounds.x_end - bounds.x_start;
        let fill_width = (visible_width / 2).max(1);
        let fill_start = bounds.x_start + (visible_width - fill_width) / 2;
        let fill_end = fill_start + fill_width;
        let fill_height = (bounds.y_end - bounds.y_start) * 4 / 5;

        let grains = engine
            .debug_fill_rainbow_80_centered_half(&categories)
            .expect("classic centered half fixture fill");

        assert_eq!(grains, fill_width * fill_height);
        assert_eq!(engine.grain_count(), grains);
        assert_eq!(engine.pending_count(), 0);
        assert_eq!(engine.movement_counts(), (0, 0));

        for x in bounds.x_start..bounds.x_end {
            if !(fill_start..fill_end).contains(&x) {
                assert!(
                    engine.surface.grid[bounds.y_start..bounds.y_end]
                        .iter()
                        .all(|row| row[x].is_none()),
                    "outside half-fill span must remain empty at x={x}"
                );
                continue;
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
