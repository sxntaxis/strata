use std::collections::VecDeque;
use std::sync::OnceLock;

use ratatui::prelude::Line;

use crate::domain::{Category, CategoryId};

use super::{
    BrailleColorBackground, BrailleColorBlend, SandEngine, ViewportBounds,
    centered_half_open_interval,
};

mod grounded_index;
mod occupancy_index;
mod stratigraphy;
use grounded_index::GroundedColumnIndex;
use occupancy_index::RowOccupancyIndex;
use stratigraphy::ClassicRainbowFillDescriptor;

const CLASSIC_PHYSICS_RNG_XOR: u64 = 0xC6BC_2796_92B5_CC83;
const CLASSIC_RAIN_RNG_XOR: u64 = 0xD1B5_4A32_D192_ED03;
const CLASSIC_REPOSE_RNG_XOR: u64 = 0xA5A5_5A5A_D3C1_B7E9;
const CLASSIC_REPOSE_LOW: u8 = 1;
const CLASSIC_REPOSE_MID: u8 = 2;
const CLASSIC_REPOSE_HIGH: u8 = 3;
const CLASSIC_REPOSE_ANCHOR: u8 = 4;
const GOLDEN_RATIO: f64 = 1.618_033_988_749_895;
const RAIN_FOCUS_BIAS_ONE_IN: usize = 4;
// RAIN-003 keeps the human-approved broad 75/25 rain envelope but replaces
// deterministic cross-corridor waypoints and RAIN-002 dwell/avulsion with an
// aperiodic correlated meander. A hypothetical uninterrupted traverse takes
// roughly four simulated hours: fast enough to vary successive strata while
// remaining a low-frequency morphology signal rather than a visible nozzle.
const RAIN_FOCUS_MEANDER_EDGE_TO_EDGE_INGRESSES: usize = 14_400;
const RAIN_FOCUS_HEADING_INGRESSES: usize = 900;
const RAIN_FOCUS_REPHASE_HEADING_INGRESSES: usize = 180;
const RAIN_FOCUS_REPHASE_INGRESSES: usize = 900;
const CLASSIC_MOMENTUM_MIN_DROP_DEPTH: usize = 2;
const CLASSIC_MOMENTUM_MAX_DROP_DEPTH: usize = 3;

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
/// `Uniform` uses full-width random rain. `WanderingFocus` changes only ingress
/// sampling: 75% remains uniform over the full visible width and 25% uses a broad
/// golden-corridor focus. RAIN-003 drives that focus with a continuous correlated
/// meander: directional persistence is perturbed stochastically, broad terrain
/// relief provides only weak steering, and corridor edges provide only soft inward
/// steering. A category transition transiently decorrelates the heading without
/// teleporting the focus, so successive strata can develop different envelopes.
/// New grains are sampled directly from free visible-top sites rather than sampling
/// an occupied target and relocating it.
/// Nothing from this sandbox is persisted or becomes production authority.
struct ClassicRuntimeIndex {
    occupancy: RowOccupancyIndex,
    grounded: Option<GroundedColumnIndex>,
}

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
    color_blend_profile: BrailleColorBlend,
    color_background_policy: BrailleColorBackground,
    last_rainbow_fill: Option<ClassicRainbowFillDescriptor>,
    rain_focus_x: Option<usize>,
    rain_focus_direction: i8,
    rain_focus_move_counter: usize,
    rain_focus_heading_counter: usize,
    rain_focus_rephase_remaining: usize,
    rain_last_category_id: Option<CategoryId>,
    rain_left_padding_targets: usize,
    rain_corridor_targets: usize,
    rain_right_padding_targets: usize,
    pending_drive: VecDeque<CategoryId>,
    frame_count: usize,
    total_generated: usize,
    vertical_moves: usize,
    diagonal_moves: usize,
    runtime_index: Option<ClassicRuntimeIndex>,
    #[cfg(test)]
    force_reference_gravity: bool,
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
            color_blend_profile: BrailleColorBlend::RgbLumaSafe,
            color_background_policy: BrailleColorBackground::Neutral,
            last_rainbow_fill: None,
            rain_focus_x: None,
            rain_focus_direction: 0,
            rain_focus_move_counter: 0,
            rain_focus_heading_counter: 0,
            rain_focus_rephase_remaining: 0,
            rain_last_category_id: None,
            rain_left_padding_targets: 0,
            rain_corridor_targets: 0,
            rain_right_padding_targets: 0,
            pending_drive: VecDeque::new(),
            frame_count: 0,
            total_generated: 0,
            vertical_moves: 0,
            diagonal_moves: 0,
            runtime_index: None,
            #[cfg(test)]
            force_reference_gravity: true,
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
        self.runtime_index = None;
        let old_width = self.surface.grid_width_dots;
        let old_repose = self.local_repose.clone();
        let old_memory = self.repose_memory_remaining.clone();
        self.surface.resize(width, height);
        self.last_rainbow_fill = None;
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
        }
        self.flush_pending_drive();
    }

    pub(crate) fn clear(&mut self) {
        self.runtime_index = None;
        self.surface.clear();
        self.last_rainbow_fill = None;
        self.pending_drive.clear();
        self.rain_focus_x = None;
        self.rain_focus_direction = 0;
        self.rain_focus_move_counter = 0;
        self.rain_focus_heading_counter = 0;
        self.rain_focus_rephase_remaining = 0;
        self.rain_last_category_id = None;
        self.rain_left_padding_targets = 0;
        self.rain_corridor_targets = 0;
        self.rain_right_padding_targets = 0;
        self.frame_count = 0;
        self.total_generated = 0;
        self.vertical_moves = 0;
        self.diagonal_moves = 0;
        self.repose_rng_state = self.initial_repose_rng_state;
        self.resample_all_local_repose();
    }

    pub(crate) fn render(&self, categories: &[Category]) -> Vec<Line<'static>> {
        self.surface.render_with_color_blend_and_background(
            categories,
            self.color_blend_profile,
            self.color_background_policy,
        )
    }

    pub(crate) fn color_blend_profile_name(&self) -> &'static str {
        self.color_blend_profile.name()
    }

    pub(crate) fn set_color_blend_profile_name(&mut self, profile: &str) -> Result<(), String> {
        let Some(profile) = BrailleColorBlend::parse(profile) else {
            return Err(
                "classic colorblend profile must be rgb, rgb-additive, rgb-luma, rgb-luma-safe, rgb-mid, rgb-contrast, linear, oklab, dominant, or dominant-soft"
                    .to_string(),
            );
        };
        self.color_blend_profile = profile;
        Ok(())
    }

    pub(crate) fn color_background_policy_name(&self) -> &'static str {
        self.color_background_policy.name()
    }

    pub(crate) fn set_color_background_policy_name(&mut self, policy: &str) -> Result<(), String> {
        let Some(policy) = BrailleColorBackground::parse(policy) else {
            return Err(
                "classic colorbackground policy must be neutral, dark, or light".to_string(),
            );
        };
        self.color_background_policy = policy;
        Ok(())
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

    pub(crate) fn rain_region_counts(&self) -> (usize, usize, usize) {
        (
            self.rain_left_padding_targets,
            self.rain_corridor_targets,
            self.rain_right_padding_targets,
        )
    }

    pub(crate) fn rain_profile_name(&self) -> &'static str {
        match self.mode {
            ClassicRainMode::Uniform => "uniform",
            ClassicRainMode::WanderingFocus => "75/25-correlated",
        }
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
        self.runtime_index = None;
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

        self.runtime_index = None;

        self.last_rainbow_fill = Some(ClassicRainbowFillDescriptor::from_fill(
            category_ids,
            fill_x_start,
            fill_x_end,
            bounds.y_end,
            fill_height,
        ));

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
        let mut runtime = self.runtime_index.take();

        while !free_columns.is_empty() && !self.pending_drive.is_empty() {
            // RAIN-002's free-site ingress remains authoritative: an occupied
            // top-row target is never sampled and then relocated sideways.
            // RAIN-003 observes the actual FIFO ingress category before choosing
            // its position so a real stratum boundary can rephase focus motion.
            let category_id = *self
                .pending_drive
                .front()
                .expect("pending drive exists");
            self.note_ingress_category(category_id);
            let free_index = self.choose_free_rain_index(bounds, &free_columns);
            let x = free_columns.swap_remove(free_index);
            let category_id = self
                .pending_drive
                .pop_front()
                .expect("pending drive exists");
            self.surface.grid[ingress_y][x] = Some(category_id);
            if let Some(index) = runtime.as_mut() {
                index.occupancy.set(x, ingress_y, true);
                if let Some(grounded) = index.grounded.as_mut() {
                    grounded.record_new_fill(&self.surface.grid, x, ingress_y);
                }
            }
        }
        self.runtime_index = runtime;
        self.sync_surface_metadata();
    }

    fn choose_free_rain_index(
        &mut self,
        bounds: ViewportBounds,
        free_columns: &[usize],
    ) -> usize {
        debug_assert!(!free_columns.is_empty());
        if self.mode == ClassicRainMode::Uniform {
            return self.rain_random_index(free_columns.len());
        }

        let focus = self.advance_rain_focus(bounds);
        let uniform_index = self.rain_random_index(free_columns.len());
        let chosen_index = if free_columns.len() == 1
            || self.rain_random_index(RAIN_FOCUS_BIAS_ONE_IN) != 0
        {
            uniform_index
        } else {
            self.sample_focus_biased_free_index(bounds, focus, free_columns)
                .unwrap_or(uniform_index)
        };
        self.record_rain_region(free_columns[chosen_index], bounds);
        chosen_index
    }

    #[cfg(test)]
    fn choose_rain_target(&mut self, bounds: ViewportBounds) -> usize {
        let free_columns = (bounds.x_start..bounds.x_end).collect::<Vec<_>>();
        let index = self.choose_free_rain_index(bounds, &free_columns);
        free_columns[index]
    }

    fn record_rain_region(&mut self, target: usize, bounds: ViewportBounds) {
        let (start, end) = Self::golden_focus_bounds(bounds);
        if target < start {
            self.rain_left_padding_targets = self.rain_left_padding_targets.saturating_add(1);
        } else if target >= end {
            self.rain_right_padding_targets = self.rain_right_padding_targets.saturating_add(1);
        } else {
            self.rain_corridor_targets = self.rain_corridor_targets.saturating_add(1);
        }
    }

    fn golden_focus_bounds(bounds: ViewportBounds) -> (usize, usize) {
        let width = bounds.x_end.saturating_sub(bounds.x_start);
        debug_assert!(width > 0);
        if width <= 2 {
            return (bounds.x_start, bounds.x_end);
        }
        // One more recursive golden subdivision than RAIN-001/002: roughly
        // 7.3% focus-only padding on each side, leaving ~85.4% for the meander.
        // Physical terrain and ordinary rain still use the complete visible width.
        let edge_fraction = (((1.0 - 1.0 / GOLDEN_RATIO) / 2.0) / GOLDEN_RATIO) / GOLDEN_RATIO;
        let padding = ((width as f64) * edge_fraction)
            .round()
            .min(((width - 1) / 2) as f64) as usize;
        (bounds.x_start + padding, bounds.x_end - padding)
    }

    fn note_ingress_category(&mut self, category_id: CategoryId) {
        if self.mode != ClassicRainMode::WanderingFocus {
            return;
        }
        if self
            .rain_last_category_id
            .is_some_and(|previous| previous != category_id)
        {
            // A category boundary is a depositional rephase, not a positional
            // reset. Preserve the exact focus and its current heading, but make
            // the next heading decision immediate and temporarily less inertial.
            self.rain_focus_rephase_remaining = RAIN_FOCUS_REPHASE_INGRESSES;
            self.rain_focus_heading_counter = RAIN_FOCUS_REPHASE_HEADING_INGRESSES;
        }
        self.rain_last_category_id = Some(category_id);
    }

    fn advance_rain_focus(&mut self, bounds: ViewportBounds) -> usize {
        let (start, end) = Self::golden_focus_bounds(bounds);
        let focus_width = end.saturating_sub(start);
        debug_assert!(focus_width > 0);

        let mut focus = self.rain_focus_x.map_or_else(
            || start + self.rain_random_index(focus_width),
            |focus| focus.clamp(start, end - 1),
        );

        if self.rain_focus_x.is_none() {
            self.rain_focus_direction = self.random_focus_direction();
            self.rain_focus_move_counter = 0;
            self.rain_focus_heading_counter = 0;
            self.rain_focus_x = Some(focus);
            return focus;
        }

        let in_rephase = self.rain_focus_rephase_remaining > 0;
        if in_rephase {
            self.rain_focus_rephase_remaining -= 1;
        }

        let heading_interval = if in_rephase {
            RAIN_FOCUS_REPHASE_HEADING_INGRESSES
        } else {
            RAIN_FOCUS_HEADING_INGRESSES
        };
        self.rain_focus_heading_counter = self.rain_focus_heading_counter.saturating_add(1);
        if self.rain_focus_direction == 0
            || self.rain_focus_heading_counter >= heading_interval
        {
            self.rain_focus_heading_counter = 0;
            self.rain_focus_direction =
                self.choose_meander_direction(focus, start, end, bounds, in_rephase);
        }

        let traversable_steps = focus_width.saturating_sub(1).max(1);
        let ingresses_per_step =
            (RAIN_FOCUS_MEANDER_EDGE_TO_EDGE_INGRESSES / traversable_steps).max(1);
        self.rain_focus_move_counter = self.rain_focus_move_counter.saturating_add(1);
        if self.rain_focus_move_counter >= ingresses_per_step {
            self.rain_focus_move_counter = 0;
            match self.rain_focus_direction.cmp(&0) {
                std::cmp::Ordering::Less if focus > start => focus -= 1,
                std::cmp::Ordering::Greater if focus + 1 < end => focus += 1,
                std::cmp::Ordering::Less => self.rain_focus_direction = 1,
                std::cmp::Ordering::Greater => self.rain_focus_direction = -1,
                std::cmp::Ordering::Equal => {}
            }
        }

        self.rain_focus_x = Some(focus);
        focus
    }

    fn choose_meander_direction(
        &mut self,
        focus: usize,
        start: usize,
        end: usize,
        bounds: ViewportBounds,
        in_rephase: bool,
    ) -> i8 {
        let focus_width = end.saturating_sub(start);
        if focus_width <= 1 {
            return 0;
        }

        let current = if self.rain_focus_direction == 0 {
            self.random_focus_direction()
        } else {
            self.rain_focus_direction
        };
        let terrain = self.terrain_steering_direction(focus, start, end, bounds);
        let edge = Self::edge_steering_direction(focus, start, end);
        let roll = self.rain_random_index(8);

        let proposed = if in_rephase {
            match roll {
                0 => current,
                1..=3 if terrain != 0 => terrain,
                1..=3 => self.random_focus_direction(),
                4..=6 => self.random_focus_direction(),
                _ if edge != 0 => edge,
                _ => self.random_focus_direction(),
            }
        } else {
            match roll {
                0..=4 => current,
                5 if terrain != 0 => terrain,
                5 => current,
                6 => self.random_focus_direction(),
                _ if edge != 0 => edge,
                _ => self.random_focus_direction(),
            }
        };

        if proposed == 0 { current } else { proposed }
    }

    fn random_focus_direction(&mut self) -> i8 {
        if self.rain_random_index(2) == 0 {
            -1
        } else {
            1
        }
    }

    fn terrain_steering_direction(
        &self,
        focus: usize,
        start: usize,
        end: usize,
        bounds: ViewportBounds,
    ) -> i8 {
        let focus_width = end.saturating_sub(start);
        if focus_width <= 2 {
            return 0;
        }
        let offset = (focus_width / 6).max(1);
        let left = focus.saturating_sub(offset).max(start);
        let right = focus.saturating_add(offset).min(end - 1);
        if left == right {
            return 0;
        }
        let left_score = self.smoothed_grounded_height(left, bounds, focus_width);
        let right_score = self.smoothed_grounded_height(right, bounds, focus_width);
        let left_cross = (left_score.0 as u128) * (right_score.1 as u128);
        let right_cross = (right_score.0 as u128) * (left_score.1 as u128);
        match left_cross.cmp(&right_cross) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => 0,
        }
    }

    fn edge_steering_direction(focus: usize, start: usize, end: usize) -> i8 {
        let focus_width = end.saturating_sub(start);
        if focus_width <= 2 {
            return 0;
        }
        let guard = (focus_width / 12).max(1);
        if focus < start.saturating_add(guard) {
            1
        } else if focus >= end.saturating_sub(guard) {
            -1
        } else {
            0
        }
    }

    fn smoothed_grounded_height(
        &self,
        candidate: usize,
        bounds: ViewportBounds,
        focus_width: usize,
    ) -> (usize, usize) {
        // One broad sample spans roughly one sixth of the focus corridor. This
        // intentionally ignores grain-scale ruggedness. RAIN-003 uses the result
        // only as weak directional steering; it never selects a destination or
        // globally searches for the lowest accommodation.
        let radius = (focus_width / 12).max(1);
        let sample_start = candidate.saturating_sub(radius).max(bounds.x_start);
        let sample_end = candidate
            .saturating_add(radius)
            .saturating_add(1)
            .min(bounds.x_end);
        let mut sum = 0usize;
        let mut count = 0usize;
        for x in sample_start..sample_end {
            let mut top = bounds.y_end;
            while top > bounds.y_start && self.surface.grid[top - 1][x].is_some() {
                top -= 1;
            }
            sum = sum.saturating_add(bounds.y_end.saturating_sub(top));
            count = count.saturating_add(1);
        }
        (sum, count.max(1))
    }

    fn sample_focus_biased_free_index(
        &mut self,
        bounds: ViewportBounds,
        focus: usize,
        free_columns: &[usize],
    ) -> Option<usize> {
        let (start, end) = Self::golden_focus_bounds(bounds);
        let first = self.random_free_index_in_range(free_columns, start, end)?;
        let second = self.random_free_index_in_range(free_columns, start, end)?;
        match free_columns[first]
            .abs_diff(focus)
            .cmp(&free_columns[second].abs_diff(focus))
        {
            std::cmp::Ordering::Less => Some(first),
            std::cmp::Ordering::Greater => Some(second),
            std::cmp::Ordering::Equal => {
                if self.rain_random_bool() {
                    Some(second)
                } else {
                    Some(first)
                }
            }
        }
    }

    fn random_free_index_in_range(
        &mut self,
        free_columns: &[usize],
        start: usize,
        end: usize,
    ) -> Option<usize> {
        let count = free_columns
            .iter()
            .filter(|x| (start..end).contains(*x))
            .count();
        if count == 0 {
            return None;
        }
        let mut ordinal = self.rain_random_index(count);
        for (index, x) in free_columns.iter().copied().enumerate() {
            if (start..end).contains(&x) {
                if ordinal == 0 {
                    return Some(index);
                }
                ordinal -= 1;
            }
        }
        unreachable!("counted free corridor site must be found")
    }

    fn apply_gravity(&mut self) {
        #[cfg(test)]
        if self.force_reference_gravity {
            self.apply_gravity_reference(true);
            return;
        }
        self.apply_gravity_optimized();
    }

    fn apply_gravity_optimized(&mut self) {
        let Some(bounds) = self.surface.viewport_bounds() else {
            return;
        };
        if bounds.y_end.saturating_sub(bounds.y_start) < 2 {
            return;
        }

        // PERF-001 keeps both indexes across ordinary Classic sweeps. The grid
        // remains authority; every production mutation updates the mirrors and
        // resize/clear/fill/profile changes invalidate them. This removes the
        // repeated dense occupancy scan and the CLASSIC-012 grounded rebuild.
        let mut runtime = match self.runtime_index.take() {
            Some(runtime) => runtime,
            None => ClassicRuntimeIndex {
                occupancy: RowOccupancyIndex::from_grid(&self.surface.grid),
                grounded: self
                    .experiment_profile
                    .ordinary_diagonal_requires_grounded_blocker()
                    .then(|| GroundedColumnIndex::from_grid(&self.surface.grid, bounds)),
            },
        };

        let base_left_to_right = self.surface.sweep_left_to_right;
        self.surface.sweep_left_to_right = !self.surface.sweep_left_to_right;

        for y in (bounds.y_start..bounds.y_end - 1).rev() {
            let left_to_right = if y.is_multiple_of(2) {
                base_left_to_right
            } else {
                !base_left_to_right
            };
            if left_to_right {
                let mut cursor = bounds.x_start;
                let mut stable_rng_draws = 0usize;
                while let Some(x) = runtime.occupancy.next_occupied(y, cursor, bounds.x_end) {
                    if self.stable_blocked_grain_consumes_one_rng_only(
                        bounds,
                        x,
                        y,
                        runtime.grounded.as_ref(),
                    ) {
                        stable_rng_draws = stable_rng_draws.saturating_add(1);
                    } else {
                        self.advance_physics_rng_draws(stable_rng_draws);
                        stable_rng_draws = 0;
                        self.move_grain_once_with_indexes(
                            bounds,
                            x,
                            y,
                            runtime.grounded.as_mut(),
                            Some(&mut runtime.occupancy),
                        );
                    }
                    cursor = x.saturating_add(1);
                }
                self.advance_physics_rng_draws(stable_rng_draws);
            } else {
                let mut cursor = bounds.x_end;
                let mut stable_rng_draws = 0usize;
                while let Some(x) = runtime
                    .occupancy
                    .previous_occupied(y, bounds.x_start, cursor)
                {
                    if self.stable_blocked_grain_consumes_one_rng_only(
                        bounds,
                        x,
                        y,
                        runtime.grounded.as_ref(),
                    ) {
                        stable_rng_draws = stable_rng_draws.saturating_add(1);
                    } else {
                        self.advance_physics_rng_draws(stable_rng_draws);
                        stable_rng_draws = 0;
                        self.move_grain_once_with_indexes(
                            bounds,
                            x,
                            y,
                            runtime.grounded.as_mut(),
                            Some(&mut runtime.occupancy),
                        );
                    }
                    cursor = x;
                }
                self.advance_physics_rng_draws(stable_rng_draws);
            }
        }
        self.runtime_index = Some(runtime);
        self.sync_surface_metadata();
    }

    #[cfg(test)]
    fn apply_gravity_reference(&mut self, use_grounded_index: bool) {
        let Some(bounds) = self.surface.viewport_bounds() else {
            return;
        };
        if bounds.y_end.saturating_sub(bounds.y_start) < 2 {
            return;
        }

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
                    self.move_grain_once_with_indexes(bounds, x, y, grounded_index.as_mut(), None);
                }
            } else {
                for x in (bounds.x_start..bounds.x_end).rev() {
                    self.move_grain_once_with_indexes(bounds, x, y, grounded_index.as_mut(), None);
                }
            }
        }
        self.sync_surface_metadata();
    }

    #[cfg(test)]
    fn apply_gravity_uncached_for_test(&mut self) {
        self.apply_gravity_reference(false);
    }

    #[cfg(test)]
    fn move_grain_once(&mut self, bounds: ViewportBounds, x: usize, y: usize) {
        self.move_grain_once_with_indexes(bounds, x, y, None, None);
    }

    fn move_grain_once_with_indexes(
        &mut self,
        bounds: ViewportBounds,
        x: usize,
        y: usize,
        mut grounded_index: Option<&mut GroundedColumnIndex>,
        mut occupancy_index: Option<&mut RowOccupancyIndex>,
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
            if let Some(index) = occupancy_index.as_deref_mut() {
                index.record_move(x, y, x, y + 1);
            }
            self.vertical_moves = self.vertical_moves.saturating_add(1);
            return;
        }

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

        let step = self.choose_diagonal_step(bounds, x, y);
        let Some(target_x) = x.checked_add_signed(step) else {
            return;
        };
        if !self.classic_diagonal_is_available(bounds, x, target_x, y) {
            return;
        }

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
        if let Some(index) = grounded_index.as_mut() {
            index.record_move(&self.surface.grid, x, y, target_x, y + 1);
        }
        if let Some(index) = occupancy_index.as_mut() {
            index.record_move(x, y, target_x, y + 1);
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
                grounded_index,
                occupancy_index,
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

    #[allow(clippy::too_many_arguments)]
    fn try_one_bonus_diagonal(
        &mut self,
        bounds: ViewportBounds,
        x: usize,
        y: usize,
        step: isize,
        category_id: CategoryId,
        blocker_grounded: bool,
        mut grounded_index: Option<&mut GroundedColumnIndex>,
        mut occupancy_index: Option<&mut RowOccupancyIndex>,
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
        if let Some(index) = grounded_index.as_mut() {
            index.record_move(&self.surface.grid, x, y, next_x, y + 1);
        }
        if let Some(index) = occupancy_index.as_mut() {
            index.record_move(x, y, next_x, y + 1);
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
        // Classic never marks SandEngine mobility and never removes mass. The
        // previous implementation cleared an already-false mobility matrix and
        // rescanned the complete grid after every spawn/gravity sweep. Exact
        // Classic mass authority is `total_generated = physical + pending`, so
        // the same observable metadata is maintained in O(1).
        #[cfg(test)]
        debug_assert!(self.surface.mobilized.iter().flatten().all(|value| !*value));
        self.surface.pending_runs.clear();
        self.surface.ingress_focus_x = None;
        self.surface.grain_count = self.total_generated;
    }

    fn stable_blocked_grain_consumes_one_rng_only(
        &self,
        bounds: ViewportBounds,
        x: usize,
        y: usize,
        grounded: Option<&GroundedColumnIndex>,
    ) -> bool {
        if self.surface.grid[y][x].is_none() || self.surface.grid[y + 1][x].is_none() {
            return false;
        }
        if self
            .experiment_profile
            .ordinary_diagonal_requires_grounded_blocker()
            && grounded.is_none_or(|index| !index.is_grounded(&self.surface.grid, x, y + 1))
        {
            return false;
        }
        let left_blocked = x == bounds.x_start || self.surface.grid[y + 1][x - 1].is_some();
        let right_blocked = x + 1 >= bounds.x_end || self.surface.grid[y + 1][x + 1].is_some();
        left_blocked && right_blocked
    }

    fn advance_physics_rng_draws(&mut self, draws: usize) {
        if draws == 0 {
            return;
        }

        fn step(mut x: u64) -> u64 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        }
        fn apply(transform: &[u64; 64], mut value: u64) -> u64 {
            let mut result = 0u64;
            while value != 0 {
                let bit = value.trailing_zeros() as usize;
                result ^= transform[bit];
                value &= value - 1;
            }
            result
        }
        fn powers() -> &'static Vec<[u64; 64]> {
            static POWERS: OnceLock<Vec<[u64; 64]>> = OnceLock::new();
            POWERS.get_or_init(|| {
                let mut base = [0u64; 64];
                for (bit, image) in base.iter_mut().enumerate() {
                    *image = step(1u64 << bit);
                }
                let mut powers = Vec::with_capacity(64);
                powers.push(base);
                for exponent in 1..64 {
                    let previous = powers[exponent - 1];
                    let mut squared = [0u64; 64];
                    for (bit, image) in squared.iter_mut().enumerate() {
                        *image = apply(&previous, previous[bit]);
                    }
                    powers.push(squared);
                }
                powers
            })
        }

        let mut state = if self.physics_rng_state == 0 {
            CLASSIC_PHYSICS_RNG_XOR
        } else {
            self.physics_rng_state
        };
        let mut remaining = draws as u64;
        let mut bit = 0usize;
        while remaining != 0 {
            if remaining & 1 != 0 {
                state = apply(&powers()[bit], state);
            }
            remaining >>= 1;
            bit += 1;
        }
        self.physics_rng_state = state;
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
    use ratatui::style::Color;

    use super::*;

    fn mass(engine: &ClassicSandboxEngine) -> usize {
        engine.physical_grain_count() + engine.pending_count()
    }

    fn fixture_categories(ids: &[CategoryId]) -> Vec<Category> {
        ids.iter()
            .copied()
            .enumerate()
            .map(|(index, id)| Category {
                id,
                name: format!("Fixture {index}"),
                color: Color::Rgb(
                    (30 + index * 25) as u8,
                    (60 + index * 20) as u8,
                    (90 + index * 15) as u8,
                ),
                description: String::new(),
                balance_effect: 0,
            })
            .collect()
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
    fn colorblend_selection_is_render_only_and_nonretroactive() {
        let mut engine = ClassicSandboxEngine::new(12, 10, 701, ClassicRainMode::Uniform);
        assert_eq!(engine.color_blend_profile_name(), "rgb-luma-safe");
        assert_eq!(engine.color_background_policy_name(), "neutral");
        let ids = [
            CategoryId(1),
            CategoryId(2),
            CategoryId(3),
            CategoryId(4),
            CategoryId(5),
            CategoryId(6),
        ];
        engine
            .debug_fill_rainbow_80_centered_half(&ids)
            .expect("fillhalf");
        let grid = engine.surface.grid.clone();
        let physics_rng = engine.physics_rng_state;
        let repose_rng = engine.repose_rng_state;
        let local_repose = engine.local_repose.clone();
        let memory = engine.repose_memory_remaining.clone();
        let moves = engine.movement_counts();

        for profile in [
            "rgb",
            "rgb-additive",
            "rgb-luma",
            "rgb-luma-safe",
            "rgb-mid",
            "rgb-contrast",
            "linear",
            "oklab",
            "dominant",
            "dominant-soft",
        ] {
            engine
                .set_color_blend_profile_name(profile)
                .expect("valid blend");
            assert_eq!(engine.color_blend_profile_name(), profile);
            assert_eq!(engine.surface.grid, grid, "profile={profile}");
            assert_eq!(engine.physics_rng_state, physics_rng, "profile={profile}");
            assert_eq!(engine.repose_rng_state, repose_rng, "profile={profile}");
            assert_eq!(engine.local_repose, local_repose, "profile={profile}");
            assert_eq!(engine.repose_memory_remaining, memory, "profile={profile}");
            assert_eq!(engine.movement_counts(), moves, "profile={profile}");
        }
        assert!(engine.set_color_blend_profile_name("neon").is_err());

        for policy in ["neutral", "dark", "light"] {
            engine
                .set_color_background_policy_name(policy)
                .expect("valid background policy");
            assert_eq!(engine.color_background_policy_name(), policy);
            assert_eq!(engine.surface.grid, grid, "background={policy}");
            assert_eq!(engine.physics_rng_state, physics_rng, "background={policy}");
            assert_eq!(engine.repose_rng_state, repose_rng, "background={policy}");
            assert_eq!(engine.local_repose, local_repose, "background={policy}");
            assert_eq!(
                engine.repose_memory_remaining, memory,
                "background={policy}"
            );
            assert_eq!(engine.movement_counts(), moves, "background={policy}");
        }
        assert!(engine.set_color_background_policy_name("auto").is_err());
    }

    #[test]
    fn stratigraphy_report_describes_fill_distribution_without_mutating_physics() {
        let mut engine = ClassicSandboxEngine::new(12, 10, 702, ClassicRainMode::Uniform);
        let ids = [
            CategoryId(1),
            CategoryId(2),
            CategoryId(3),
            CategoryId(4),
            CategoryId(5),
            CategoryId(6),
        ];
        let categories = fixture_categories(&ids);
        engine
            .debug_fill_rainbow_80_centered_half(&ids)
            .expect("fillhalf");
        let grid_before = engine.surface.grid.clone();
        let rng_before = (engine.physics_rng_state, engine.repose_rng_state);
        let repose_before = engine.local_repose.clone();
        let memory_before = engine.repose_memory_remaining.clone();
        let moves_before = engine.movement_counts();

        let report = engine
            .rainbow_stratigraphy_report(&categories)
            .expect("stratigraphy report");
        assert!(report.contains("CLASSIC_STRATIGRAPHY_REPORT"));
        assert!(report.contains("stratigraphic_rank_drops=0"));
        assert!(report.contains("outside_fill_span=0"));
        assert_eq!(engine.surface.grid, grid_before);
        assert_eq!(
            (engine.physics_rng_state, engine.repose_rng_state),
            rng_before
        );
        assert_eq!(engine.local_repose, repose_before);
        assert_eq!(engine.repose_memory_remaining, memory_before);
        assert_eq!(engine.movement_counts(), moves_before);
    }

    #[test]
    fn stratigraphy_report_surfaces_cross_band_and_lateral_outliers() {
        let mut engine = ClassicSandboxEngine::new(12, 10, 703, ClassicRainMode::Uniform);
        let ids = [
            CategoryId(1),
            CategoryId(2),
            CategoryId(3),
            CategoryId(4),
            CategoryId(5),
            CategoryId(6),
        ];
        let categories = fixture_categories(&ids);
        engine
            .debug_fill_rainbow_80_centered_half(&ids)
            .expect("fillhalf");
        let fill = engine.last_rainbow_fill.clone().expect("fill descriptor");
        assert!(fill.x_start > 0);
        let top_category = *ids.last().expect("top category");
        let source = engine
            .surface
            .grid
            .iter()
            .enumerate()
            .find_map(|(y, row)| {
                row.iter()
                    .position(|cell| *cell == Some(top_category))
                    .map(|x| (x, y))
            })
            .expect("top category grain");
        engine.surface.grid[source.1][source.0] = None;
        engine.surface.grid[fill.y_end - 1][fill.x_start - 1] = Some(top_category);

        let report = engine
            .rainbow_stratigraphy_report(&categories)
            .expect("stratigraphy report");
        let line = report
            .lines()
            .find(|line| line.contains("id=6 initial_band="))
            .expect("top category report line");
        assert!(line.contains("below=1"), "{line}");
        assert!(line.contains("outside_fill_span=1"), "{line}");
        assert!(report.contains("category=\"Fixture 5\" id=6"));
    }

    #[test]
    fn resize_or_clear_invalidates_fill_relative_stratigraphy_reference() {
        let ids = [CategoryId(1), CategoryId(2)];
        let categories = fixture_categories(&ids);
        let mut resized = ClassicSandboxEngine::new(12, 10, 704, ClassicRainMode::Uniform);
        resized
            .debug_fill_rainbow_80_centered_half(&ids)
            .expect("fillhalf");
        resized.resize(13, 10);
        assert!(resized.rainbow_stratigraphy_report(&categories).is_err());

        let mut cleared = ClassicSandboxEngine::new(12, 10, 705, ClassicRainMode::Uniform);
        cleared
            .debug_fill_rainbow_80_centered_half(&ids)
            .expect("fillhalf");
        cleared.clear();
        assert!(cleared.rainbow_stratigraphy_report(&categories).is_err());
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
    fn hybrid_focus_is_a_quarter_ingress_bias_inside_the_smaller_focus_corridor() {
        assert_eq!(RAIN_FOCUS_BIAS_ONE_IN, 4);
        let mut engine = ClassicSandboxEngine::new(100, 20, 17, ClassicRainMode::WanderingFocus);
        let bounds = engine.surface.viewport_bounds().expect("visible basin");
        let width = bounds.x_end - bounds.x_start;
        let (start, end) = ClassicSandboxEngine::golden_focus_bounds(bounds);
        let left_padding = start - bounds.x_start;
        let right_padding = bounds.x_end - end;
        assert_eq!(left_padding, right_padding);
        let observed = left_padding as f64 / width as f64;
        assert!((observed - 0.073).abs() < 0.01, "observed={observed}");
        for _ in 0..20_000 {
            let focus = engine.advance_rain_focus(bounds);
            assert!((start..end).contains(&focus));
        }
    }

    #[test]
    fn correlated_focus_meanders_aperiodically_in_both_directions() {
        let mut engine = ClassicSandboxEngine::new(100, 20, 23, ClassicRainMode::WanderingFocus);
        let bounds = engine.surface.viewport_bounds().expect("visible basin");
        let (start, end) = ClassicSandboxEngine::golden_focus_bounds(bounds);
        let mut previous = engine.advance_rain_focus(bounds);
        let mut saw_left = false;
        let mut saw_right = false;
        let mut reversals = 0usize;
        let mut previous_direction = 0i8;
        let mut min_focus = previous;
        let mut max_focus = previous;
        for _ in 0..60_000 {
            let focus = engine.advance_rain_focus(bounds);
            min_focus = min_focus.min(focus);
            max_focus = max_focus.max(focus);
            let direction = focus.cmp(&previous);
            let signed = match direction {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Equal => 0,
            };
            saw_left |= signed < 0;
            saw_right |= signed > 0;
            if signed != 0 && previous_direction != 0 && signed != previous_direction {
                reversals += 1;
            }
            if signed != 0 {
                previous_direction = signed;
            }
            previous = focus;
        }
        assert!(saw_left && saw_right);
        assert!(reversals >= 2, "reversals={reversals}");
        assert!(max_focus - min_focus > (end - start) / 4);
    }

    #[test]
    fn broad_terrain_is_weak_steering_not_a_forced_destination() {
        let mut saw_leftward_persistence = false;
        let mut saw_rightward_terrain = false;
        for seed in 1..=96 {
            let mut engine =
                ClassicSandboxEngine::new(80, 20, seed, ClassicRainMode::WanderingFocus);
            let bounds = engine.surface.viewport_bounds().expect("visible basin");
            let (start, end) = ClassicSandboxEngine::golden_focus_bounds(bounds);
            let focus = start + (end - start) / 2;
            let focus_width = end - start;
            let floor = bounds.y_end - 1;
            let offset = (focus_width / 6).max(1);
            let left_center = focus.saturating_sub(offset).max(start);
            let radius = (focus_width / 12).max(1);
            for x in left_center.saturating_sub(radius)..=(left_center + radius).min(bounds.x_end - 1) {
                for depth in 0..8usize {
                    engine.surface.grid[floor - depth][x] = Some(CategoryId(1));
                }
            }
            engine.rain_focus_direction = -1;
            assert_eq!(
                engine.terrain_steering_direction(focus, start, end, bounds),
                1
            );
            let chosen = engine.choose_meander_direction(focus, start, end, bounds, false);
            saw_leftward_persistence |= chosen < 0;
            saw_rightward_terrain |= chosen > 0;
        }
        assert!(saw_leftward_persistence && saw_rightward_terrain);
    }

    #[test]
    fn category_change_rephases_heading_without_teleporting_focus() {
        let mut engine = ClassicSandboxEngine::new(80, 20, 31, ClassicRainMode::WanderingFocus);
        let bounds = engine.surface.viewport_bounds().expect("visible basin");
        for _ in 0..2_000 {
            engine.advance_rain_focus(bounds);
        }
        engine.note_ingress_category(CategoryId(1));
        let focus_before = engine.rain_focus_x;
        let direction_before = engine.rain_focus_direction;
        engine.note_ingress_category(CategoryId(2));
        assert_eq!(engine.rain_focus_x, focus_before);
        assert_eq!(engine.rain_focus_direction, direction_before);
        assert_eq!(engine.rain_focus_rephase_remaining, RAIN_FOCUS_REPHASE_INGRESSES);
        assert_eq!(
            engine.rain_focus_heading_counter,
            RAIN_FOCUS_REPHASE_HEADING_INGRESSES
        );

        engine.advance_rain_focus(bounds);
        assert_eq!(engine.rain_focus_rephase_remaining, RAIN_FOCUS_REPHASE_INGRESSES - 1);
        assert!(engine.rain_focus_x.is_some());
    }

    #[test]
    fn repeated_same_category_does_not_rephase_focus() {
        let mut engine = ClassicSandboxEngine::new(40, 15, 37, ClassicRainMode::WanderingFocus);
        engine.note_ingress_category(CategoryId(3));
        assert_eq!(engine.rain_focus_rephase_remaining, 0);
        engine.note_ingress_category(CategoryId(3));
        assert_eq!(engine.rain_focus_rephase_remaining, 0);
    }

    #[test]
    fn actual_fifo_ingress_category_change_arms_rephase_only_when_it_can_enter() {
        let mut engine = ClassicSandboxEngine::new(8, 6, 41, ClassicRainMode::WanderingFocus);
        let bounds = engine.surface.viewport_bounds().expect("visible basin");
        let ingress_y = bounds.y_start;

        engine.spawn(CategoryId(1));
        assert_eq!(engine.rain_last_category_id, Some(CategoryId(1)));
        assert_eq!(engine.rain_focus_rephase_remaining, 0);

        for x in bounds.x_start..bounds.x_end {
            engine.surface.grid[ingress_y][x] = Some(CategoryId(1));
        }
        engine.runtime_index = None;
        engine.spawn(CategoryId(2));
        assert_eq!(engine.pending_count(), 1);
        assert_eq!(engine.rain_last_category_id, Some(CategoryId(1)));
        assert_eq!(engine.rain_focus_rephase_remaining, 0);

        let gap = bounds.x_start + 2;
        engine.surface.grid[ingress_y][gap] = None;
        engine.runtime_index = None;
        engine.flush_pending_drive();
        assert_eq!(engine.pending_count(), 0);
        assert_eq!(engine.rain_last_category_id, Some(CategoryId(2)));
        assert!(engine.rain_focus_rephase_remaining > 0);
    }

    #[test]
    fn correlated_rain_stays_broad_and_reaches_both_focus_paddings() {
        let mut engine = ClassicSandboxEngine::new(60, 20, 29, ClassicRainMode::WanderingFocus);
        let bounds = engine.surface.viewport_bounds().expect("visible basin");
        let width = bounds.x_end - bounds.x_start;
        let bin_count = 12usize;
        let samples = 24_000usize;
        let mut bins = vec![0usize; bin_count];
        for _ in 0..samples {
            let target = engine.choose_rain_target(bounds);
            let local = target - bounds.x_start;
            let bin = (local * bin_count / width).min(bin_count - 1);
            bins[bin] += 1;
        }
        let (left, corridor, right) = engine.rain_region_counts();
        assert_eq!(left + corridor + right, samples);
        assert!(left > 0 && right > 0, "left={left} right={right}");
        assert!(bins.iter().all(|count| *count > samples / 40), "{bins:?}");
        let max_bin = *bins.iter().max().expect("bins");
        assert!(max_bin < samples / 5, "anti-nozzle bins={bins:?}");
        println!(
            "RAIN_003_METRICS samples={samples} left={left} corridor={corridor} right={right} max_bin={max_bin} bins={bins:?}"
        );
    }

    #[test]
    fn ingress_samples_only_currently_free_top_sites_and_waits_when_full() {
        for mode in [ClassicRainMode::Uniform, ClassicRainMode::WanderingFocus] {
            let mut engine = ClassicSandboxEngine::new(8, 6, 31, mode);
            let bounds = engine.surface.viewport_bounds().expect("visible basin");
            let ingress_y = bounds.y_start;
            let first_gap = bounds.x_start + 3;
            for x in bounds.x_start..bounds.x_end {
                if x != first_gap {
                    engine.surface.grid[ingress_y][x] = Some(CategoryId(1));
                }
            }
            engine.runtime_index = None;
            engine.total_generated = engine.surface.physical_grain_count();
            engine.sync_surface_metadata();

            engine.spawn(CategoryId(2));
            assert_eq!(engine.surface.grid[ingress_y][first_gap], Some(CategoryId(2)));
            assert_eq!(engine.pending_count(), 0);

            engine.spawn(CategoryId(3));
            assert_eq!(engine.pending_count(), 1, "mode={mode:?}");

            let second_gap = bounds.x_start + 1;
            engine.surface.grid[ingress_y][second_gap] = None;
            engine.runtime_index = None;
            engine.flush_pending_drive();
            assert_eq!(engine.surface.grid[ingress_y][second_gap], Some(CategoryId(3)));
            assert_eq!(engine.pending_count(), 0);
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
mod perf_001_tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn advance_exact(engine: &mut ClassicSandboxEngine, simulated: Duration) {
        advance_exact_with_category(engine, simulated, CategoryId::new(1));
    }

    fn advance_exact_with_category(
        engine: &mut ClassicSandboxEngine,
        simulated: Duration,
        category: CategoryId,
    ) {
        let tick = Duration::from_millis(crate::constants::TIME_SETTINGS.tick_ms);
        let physics = Duration::from_millis(crate::constants::TIME_SETTINGS.physics_ms);
        let mut spawn_accumulator = Duration::ZERO;
        let mut physics_accumulator = Duration::ZERO;
        let mut remaining = simulated;

        while !remaining.is_zero() {
            let spawn_left = tick.saturating_sub(spawn_accumulator);
            let physics_left = physics.saturating_sub(physics_accumulator);
            let step = remaining.min(spawn_left.min(physics_left));
            spawn_accumulator += step;
            physics_accumulator += step;
            remaining = remaining.saturating_sub(step);

            let spawn_due = spawn_accumulator >= tick;
            let physics_due = physics_accumulator >= physics;
            if spawn_due {
                spawn_accumulator = spawn_accumulator.saturating_sub(tick);
                engine.spawn(category);
            }
            if physics_due {
                physics_accumulator = physics_accumulator.saturating_sub(physics);
                engine.update();
            }
            assert!(step > Duration::ZERO || spawn_due || physics_due);
        }
    }

    fn assert_exact(left: &ClassicSandboxEngine, right: &ClassicSandboxEngine) {
        assert_eq!(left.surface.grid, right.surface.grid);
        assert_eq!(left.surface.mobilized, right.surface.mobilized);
        assert_eq!(left.surface.grain_count, right.surface.grain_count);
        assert_eq!(left.surface.pending_runs, right.surface.pending_runs);
        assert_eq!(left.surface.ingress_focus_x, right.surface.ingress_focus_x);
        assert_eq!(
            left.surface.sweep_left_to_right,
            right.surface.sweep_left_to_right
        );
        assert_eq!(left.physics_rng_state, right.physics_rng_state);
        assert_eq!(left.rain_rng_state, right.rain_rng_state);
        assert_eq!(left.repose_rng_state, right.repose_rng_state);
        assert_eq!(left.local_repose, right.local_repose);
        assert_eq!(left.repose_memory_remaining, right.repose_memory_remaining);
        assert_eq!(left.rain_focus_x, right.rain_focus_x);
        assert_eq!(left.rain_focus_direction, right.rain_focus_direction);
        assert_eq!(left.rain_focus_move_counter, right.rain_focus_move_counter);
        assert_eq!(
            left.rain_focus_heading_counter,
            right.rain_focus_heading_counter
        );
        assert_eq!(
            left.rain_focus_rephase_remaining,
            right.rain_focus_rephase_remaining
        );
        assert_eq!(left.rain_last_category_id, right.rain_last_category_id);
        assert_eq!(
            left.rain_left_padding_targets,
            right.rain_left_padding_targets
        );
        assert_eq!(left.rain_corridor_targets, right.rain_corridor_targets);
        assert_eq!(
            left.rain_right_padding_targets,
            right.rain_right_padding_targets
        );
        assert_eq!(left.pending_drive, right.pending_drive);
        assert_eq!(left.frame_count, right.frame_count);
        assert_eq!(left.total_generated, right.total_generated);
        assert_eq!(left.vertical_moves, right.vertical_moves);
        assert_eq!(left.diagonal_moves, right.diagonal_moves);
    }

    #[test]
    fn physics_rng_jump_matches_individual_draws_exactly() {
        for draws in [1usize, 2, 3, 17, 64, 255, 1_024, 65_537] {
            let mut individual =
                ClassicSandboxEngine::new(8, 6, 0xA11C_E155, ClassicRainMode::Uniform);
            let mut jumped = ClassicSandboxEngine::new(8, 6, 0xA11C_E155, ClassicRainMode::Uniform);
            for _ in 0..draws {
                let _ = individual.next_physics_random_u64();
            }
            jumped.advance_physics_rng_draws(draws);
            assert_eq!(individual.physics_rng_state, jumped.physics_rng_state);
        }
    }

    #[test]
    fn optimized_classic_is_exact_against_dense_reference_for_uniform_and_rain_003() {
        for mode in [ClassicRainMode::Uniform, ClassicRainMode::WanderingFocus] {
            for seed in [0xA11C_E201, 0xA11C_E202, 0xA11C_E203] {
                let mut reference = ClassicSandboxEngine::new(48, 18, seed, mode);
                let mut optimized = ClassicSandboxEngine::new(48, 18, seed, mode);
                optimized.force_reference_gravity = false;

                advance_exact(&mut reference, Duration::from_secs(180));
                advance_exact(&mut optimized, Duration::from_secs(180));
                assert_exact(&reference, &optimized);

                // The optimized cache must not hide a divergence in subsequent
                // canonical reference execution.
                optimized.force_reference_gravity = true;
                optimized.runtime_index = None;
                advance_exact(&mut reference, Duration::from_secs(60));
                advance_exact(&mut optimized, Duration::from_secs(60));
                assert_exact(&reference, &optimized);
            }
        }
    }

    #[test]
    fn optimized_hybrid_category_rephase_is_exact_against_dense_reference() {
        let mut reference = ClassicSandboxEngine::new(64, 20, 0xA11C_E2C1, ClassicRainMode::WanderingFocus);
        let mut optimized = ClassicSandboxEngine::new(64, 20, 0xA11C_E2C1, ClassicRainMode::WanderingFocus);
        optimized.force_reference_gravity = false;

        for category in [CategoryId::new(1), CategoryId::new(2), CategoryId::new(3)] {
            advance_exact_with_category(&mut reference, Duration::from_secs(90), category);
            advance_exact_with_category(&mut optimized, Duration::from_secs(90), category);
            assert_exact(&reference, &optimized);
        }

        optimized.force_reference_gravity = true;
        optimized.runtime_index = None;
        advance_exact_with_category(&mut reference, Duration::from_secs(60), CategoryId::new(4));
        advance_exact_with_category(&mut optimized, Duration::from_secs(60), CategoryId::new(4));
        assert_exact(&reference, &optimized);
    }

    #[test]
    fn o1_metadata_matches_exact_classic_mass_authority() {
        let mut engine =
            ClassicSandboxEngine::new(32, 12, 0xA11C_E204, ClassicRainMode::WanderingFocus);
        engine.force_reference_gravity = false;
        advance_exact(&mut engine, Duration::from_secs(240));
        assert_eq!(engine.surface.grain_count, engine.total_generated);
        assert_eq!(
            engine.total_generated,
            engine.surface.physical_grain_count() + engine.pending_drive.len()
        );
        assert!(
            engine
                .surface
                .mobilized
                .iter()
                .flatten()
                .all(|value| !*value)
        );
    }

    #[test]
    #[ignore = "native PERF-001 exact Classic high-speed probe"]
    fn perf_001_hybrid_rate_probe() {
        let simulated = Duration::from_secs(900);
        let mut reference =
            ClassicSandboxEngine::new(190, 48, 0xA11C_E2F0, ClassicRainMode::WanderingFocus);
        let mut optimized =
            ClassicSandboxEngine::new(190, 48, 0xA11C_E2F0, ClassicRainMode::WanderingFocus);
        optimized.force_reference_gravity = false;

        // Keep the reference sample bounded while still measuring the dense
        // path on the same viewport and rain law.
        let reference_simulated = Duration::from_secs(120);
        let reference_started = Instant::now();
        advance_exact(&mut reference, reference_simulated);
        let reference_elapsed = reference_started.elapsed();

        let optimized_started = Instant::now();
        advance_exact(&mut optimized, simulated);
        let optimized_elapsed = optimized_started.elapsed();
        let optimized_x =
            simulated.as_secs_f64() / optimized_elapsed.as_secs_f64().max(f64::EPSILON);
        let reference_x =
            reference_simulated.as_secs_f64() / reference_elapsed.as_secs_f64().max(f64::EPSILON);
        let speedup = optimized_x / reference_x.max(f64::EPSILON);

        println!(
            "PERF_001_HYBRID_RATE simulated_secs={} optimized_wall_us={} optimized_x={optimized_x:.2} reference_secs={} reference_wall_us={} reference_x={reference_x:.2} speedup={speedup:.2}x",
            simulated.as_secs(),
            optimized_elapsed.as_micros(),
            reference_simulated.as_secs(),
            reference_elapsed.as_micros(),
        );
        assert_eq!(optimized.total_generated, simulated.as_secs() as usize);
    }
}

#[cfg(test)]
#[path = "classic/repose_tests.rs"]
mod repose_tests;
