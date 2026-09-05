use std::collections::{BTreeMap, VecDeque};

use crate::domain::{Category, CategoryId};
use ratatui::prelude::Line;

use super::{PendingGrainRun, SandEngine};

mod flowviz;
mod flowviz_conservative;
mod flowviz_conservative_render;
mod flowviz_unit;
mod flowviz_unit_micro;
#[cfg(test)]
mod flowviz_unit_micro_tests;
mod flowviz_unit_perceptual;
#[cfg(test)]
mod flowviz_unit_perceptual_tests;
#[cfg(test)]
mod flowviz_unit_direct_tests;
#[cfg(test)]
mod flowviz_unit_support_tests;
mod flowviz_unit_render;
#[cfg(test)]
mod flowviz_unit_truthful_tests;
#[cfg(test)]
mod flowviz_unit_coherent_tests;

const OSLO_THRESHOLD_LOW: u8 = 1;
const OSLO_THRESHOLD_HIGH: u8 = 2;
const OSLO_THRESHOLD_RNG_XOR: u64 = 0xA24B_AED4_963E_E407;
const OSLO_RAIN_RNG_XOR: u64 = 0xD1B5_4A32_D192_ED03;
const OSLO_RELAX_RNG_XOR: u64 = 0x94D0_49BB_1331_11EB;
const GOLDEN_RATIO: f64 = 1.618_033_988_749_895;
const RAIN_FOCUS_BIAS_ONE_IN: usize = 10;
// At the normal one-grain-per-second cadence, a full golden-corridor traverse
// takes about twelve simulated hours. Waypoints live in opposite outer sixths
// so every completed left/right leg spans at least two thirds of the corridor
// while the exact destination remains stochastic instead of diffusing in place.
const RAIN_FOCUS_EDGE_TO_EDGE_INGRESSES: usize = 43_200;
const RECENT_AVALANCHE_WINDOW: usize = 512;
// Catastrophic local relief enters a transient moving-grain phase. Ordinary
// Oslo slips remain unchanged below this threshold. A rolling grain keeps
// moving down-slope and may coast briefly across level terrain, modelling the
// lower dynamic friction of material that is already in motion.
const MOMENTUM_TRIGGER_RELIEF: usize = 4;
const MOMENTUM_FLAT_COAST_STEPS: u8 = 2;
// SEDIMENT-008 adds a BCRE/Daerr-Douady-inspired exchange layer on top of
// causal rolling grains. Static material needs a larger loss-of-support slope
// to join the flow than already-moving material needs to continue. A moving
// layer erodes at relief >= 2, while uphill support failure recruits at relief
// >= 3. Flat runout is deliberately shorter than momentum-v1 and grows only
// slightly with local moving-layer thickness.
const FRONT_EROSION_RELIEF: usize = 2;
const FRONT_SUPPORT_LOSS_RELIEF: usize = 3;
const FRONT_BASE_FLAT_COAST_STEPS: u8 = 1;
const FRONT_MAX_FLAT_COAST_STEPS: u8 = 3;
// SEDIMENT-009 curiosity model: a bounded discrete analogue of BCRE /
// Aranson-Tsimring partial fluidization. The static Oslo bed remains the
// preparation mechanism, but a severe failure can create a spatial fluidity
// field. Static material needs relief 4 to nucleate fluidity; already-fluidized
// material can remain active down to relief 1, giving start/stop hysteresis.
const FLUIDITY_MAX: u8 = 8;
const FLUIDITY_SEED: u8 = 8;
const FLUIDITY_SPREAD_MIN: u8 = 3;
const FLUIDITY_RELEASE_MIN: u8 = 4;
const FLUIDITY_STATIC_START_RELIEF: usize = 4;
const FLUIDITY_DYNAMIC_STOP_RELIEF: usize = 1;
const FLUIDITY_RELEASE_RELIEF: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToppleDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OsloBoundaryMode {
    /// Diagnostic control matching the Oslo experiment that visually formed
    /// wedges: the missing neighbour is height zero and an outward topple
    /// discharges the top grain.
    ZeroOutside,
    /// A real closed wall: the outward direction has zero relief and can never
    /// receive or discharge a grain.
    ClosedBox,
    /// A full-height vessel wall. Below the canonical wall top this is identical
    /// to ClosedBox; only grains standing above the canonical wall height can
    /// topple outward and overflow.
    CanonicalWallOverflow,
}

impl OsloBoundaryMode {
    pub(crate) fn model_name(self) -> &'static str {
        match self {
            Self::ZeroOutside => "oslo-zero",
            Self::ClosedBox => "oslo-box",
            Self::CanonicalWallOverflow => "oslo-vessel",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FallingDrive {
    x: usize,
    y: usize,
    category_id: CategoryId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RollingGrain {
    site: usize,
    visual_y: usize,
    unit_observed_y: Option<f32>,
    category_id: CategoryId,
    direction: ToppleDirection,
    flat_coast_remaining: u8,
    motion_id: Option<flowviz_unit::FlowVizMotionId>,
}

/// Debug-only Strata-rain-driven Oslo boundary laboratory.
///
/// The relaxation law keeps the Oslo model's local stochastic-slope character:
/// - each column carries a quenched critical slope in `{1,2}`;
/// - a column topples when its steepest local relief exceeds that threshold;
/// - one top grain moves toward the lower neighbour with greater relief, with an
///   independent random tie-break only when both downhill reliefs are equal;
/// - the toppled column redraws only its own threshold;
/// - relaxation remains nearest-neighbour/local and runs to quiescence before
///   the next drive grain is committed;
/// - the relaxation law is identical across the three boundary controls; only
///   the missing-neighbour boundary condition changes (`oslo-zero`, `oslo-box`,
///   `oslo-vessel`).
///
/// `oslo-vessel-momentum` keeps that exact static Oslo law until a single local
/// topple crosses a deliberately high relief. The released top grain then enters
/// a transient rolling phase: it preserves its downhill direction, continues on
/// descending terrain, can coast a bounded distance across flat terrain, and
/// settles back into the Oslo heightfield when dynamic motion can no longer
/// continue. Many such grains may roll concurrently. This is a physical sandbox
/// change, not presentation batching and not a global avalanche-size gate.
///
/// `oslo-vessel-front` keeps the same severe-failure seed, then adds a discrete
/// static/rolling exchange layer inspired by BCRE and granular-avalanche
/// experiments. Moving material can entrain one settled grain while crossing a
/// relief >= 2, and loss of support can recruit the immediately uphill column at
/// relief >= 3. The moving layer therefore has a lower stopping threshold than
/// the static start threshold: thin events can die downhill, while a sufficiently
/// developed wall failure can propagate uphill as support is removed. Exchange is
/// bounded to one erosion per source site per rolling tick; this remains a debug
/// product experiment rather than a claim of numerical fidelity to a paper.
///
/// `oslo-vessel-fluid` freezes those front rules and adds a separate transient
/// fluidity/order field. A severe Oslo failure nucleates the field; it spreads
/// locally, survives at a lower dynamic-stop relief than the static-start relief,
/// and converts settled grains into the existing explicit rolling layer. This is
/// a dot-preserving discrete visualization of partial fluidization, not a direct
/// discretization of the continuum BCRE or Aranson-Tsimring equations.
///
/// Strata's rain remains full-width. Only the slowly wandering statistical focus
/// is constrained to a centered corridor whose side padding is one additional
/// golden-ratio subdivision of the earlier ~19.1% margin: about 11.8% on each
/// side. The outer regions are focus padding, not physical padding: ordinary rain may land there
/// and Oslo avalanches may freely enter or leave them. Constraining only the
/// focus keeps the long-lived rainfall bias away from the frame while preserving
/// the full-width physical pile.
///
/// CategoryId rides with each grain and incoming grains remain visibly animated.
/// This file is debug-only and deliberately has no persistence or production cutover.
pub(crate) struct OsloSandboxEngine {
    surface: SandEngine,
    boundary: OsloBoundaryMode,
    momentum_enabled: bool,
    front_enabled: bool,
    fluid_enabled: bool,
    flowviz_enabled: bool,
    flowviz_conservative: bool,
    flowviz_unit: bool,
    flowviz_unit_micro: bool,
    flowviz_unit_perceptual: bool,
    flowviz_unit_truthful: bool,
    flowviz_unit_coherent: bool,
    flowviz_unit_direct_geometry: bool,
    flowviz_unit_visual_support: bool,
    columns: Vec<Vec<CategoryId>>,
    // Presentation-only y overrides aligned one-for-one with `columns`.
    // A settled grain can already participate in authoritative physics while
    // its CategoryId continues travelling visibly toward the settled row.
    // This keeps avalanche physics exact without globally blocking on render
    // interpolation or visually teleporting colored grains down tall cliffs.
    column_visual_y: Vec<Vec<Option<usize>>>,
    flowviz_edge_flux: Vec<i16>,
    flowviz_tracers: VecDeque<flowviz::FlowVizTracer>,
    flowviz_parcels: VecDeque<flowviz_conservative::FlowVizParcel>,
    flowviz_unit_carriers:
        BTreeMap<flowviz_unit::FlowVizMotionId, flowviz_unit::FlowVizUnitCarrier>,
    flowviz_unit_custody: Vec<Vec<Option<flowviz_unit::FlowVizMotionId>>>,
    flowviz_unit_next_id: u64,
    flowviz_unit_next_sequence: u64,
    flowviz_unit_peak: usize,
    flowviz_unit_segments: usize,
    flowviz_unit_reveals: usize,
    flowviz_unit_misses: usize,
    flowviz_shadow_columns: Vec<Vec<CategoryId>>,
    flowviz_transport_left: Vec<Vec<(CategoryId, usize)>>,
    flowviz_transport_right: Vec<Vec<(CategoryId, usize)>>,
    flowviz_rng_state: u64,
    flowviz_spawned: usize,
    flowviz_peak_tracers: usize,
    flowviz_dropped_samples: usize,
    flowviz_peak_parcels: usize,
    flowviz_mobile_mass: usize,
    flowviz_peak_mobile_mass: usize,
    flowviz_visual_withdrawals: usize,
    flowviz_visual_deposits: usize,
    flowviz_reused_deposits: usize,
    flowviz_coalesced_mass: usize,
    flowviz_shadow_misses: usize,
    critical_slopes: Vec<u8>,
    threshold_rng_state: u64,
    rain_rng_state: u64,
    relax_rng_state: u64,
    rain_focus_site: Option<usize>,
    rain_focus_target_site: Option<usize>,
    rain_focus_move_counter: usize,
    rain_left_padding_targets: usize,
    rain_corridor_targets: usize,
    rain_right_padding_targets: usize,
    falling_drives: VecDeque<FallingDrive>,
    rolling_grains: VecDeque<RollingGrain>,
    pending_runs: VecDeque<PendingGrainRun>,
    active_sites: VecDeque<usize>,
    queued_sites: Vec<bool>,
    frame_count: usize,
    discharged: usize,
    total_generated: usize,
    avalanche_moves: usize,
    avalanche_peak_moves: usize,
    avalanche_last_moves: usize,
    avalanche_completed: usize,
    recent_avalanches: VecDeque<usize>,
    momentum_seeds: usize,
    momentum_hops: usize,
    momentum_settles: usize,
    momentum_peak_active: usize,
    momentum_event_seeds: usize,
    momentum_event_hops: usize,
    momentum_event_peak_active: usize,
    momentum_last_seeds: usize,
    momentum_last_hops: usize,
    momentum_last_peak_active: usize,
    front_erosions: usize,
    front_support_recruits: usize,
    front_event_erosions: usize,
    front_event_support_recruits: usize,
    front_last_erosions: usize,
    front_last_support_recruits: usize,
    fluidity: Vec<u8>,
    fluid_activations: usize,
    fluid_releases: usize,
    fluid_peak_active_sites: usize,
    fluid_event_activations: usize,
    fluid_event_releases: usize,
    fluid_last_activations: usize,
    fluid_last_releases: usize,
}
impl OsloSandboxEngine {
    pub(crate) fn new(width: u16, height: u16, seed: u64, boundary: OsloBoundaryMode) -> Self {
        Self::new_with_flow(width, height, seed, boundary, false, false, false)
    }

    pub(crate) fn new_momentum_vessel(width: u16, height: u16, seed: u64) -> Self {
        Self::new_with_flow(
            width,
            height,
            seed,
            OsloBoundaryMode::CanonicalWallOverflow,
            true,
            false,
            false,
        )
    }

    pub(crate) fn new_front_vessel(width: u16, height: u16, seed: u64) -> Self {
        Self::new_with_flow(
            width,
            height,
            seed,
            OsloBoundaryMode::CanonicalWallOverflow,
            true,
            true,
            false,
        )
    }

    pub(crate) fn new_front_flowviz_vessel(width: u16, height: u16, seed: u64) -> Self {
        let mut sandbox = Self::new_front_vessel(width, height, seed);
        sandbox.enable_flowviz(seed);
        sandbox.sync_surface();
        sandbox
    }

    pub(crate) fn new_front_conservative_flowviz_vessel(
        width: u16,
        height: u16,
        seed: u64,
    ) -> Self {
        let mut sandbox = Self::new_front_vessel(width, height, seed);
        sandbox.enable_conservative_flowviz(seed);
        sandbox.sync_surface();
        sandbox
    }

    #[cfg(test)]
    pub(crate) fn new_front_unit_flowviz_vessel(width: u16, height: u16, seed: u64) -> Self {
        let mut sandbox = Self::new_front_vessel(width, height, seed);
        sandbox.enable_unit_flowviz();
        sandbox.sync_surface();
        sandbox
    }

    #[cfg(test)]
    pub(crate) fn new_front_unit_micro_flowviz_vessel(width: u16, height: u16, seed: u64) -> Self {
        let mut sandbox = Self::new_front_vessel(width, height, seed);
        sandbox.enable_unit_flowviz();
        sandbox.flowviz_unit_micro = true;
        sandbox.sync_surface();
        sandbox
    }

    pub(crate) fn new_front_unit_perceptual_flowviz_vessel(
        width: u16,
        height: u16,
        seed: u64,
    ) -> Self {
        let mut sandbox = Self::new_front_vessel(width, height, seed);
        sandbox.enable_unit_flowviz();
        sandbox.flowviz_unit_micro = true;
        sandbox.flowviz_unit_perceptual = true;
        sandbox.sync_surface();
        sandbox
    }

    pub(crate) fn new_front_unit_truthful_flowviz_vessel(
        width: u16,
        height: u16,
        seed: u64,
    ) -> Self {
        let mut sandbox = Self::new_front_unit_perceptual_flowviz_vessel(width, height, seed);
        sandbox.flowviz_unit_truthful = true;
        sandbox.sync_surface();
        sandbox
    }

    pub(crate) fn new_front_unit_coherent_flowviz_vessel(
        width: u16,
        height: u16,
        seed: u64,
    ) -> Self {
        let mut sandbox = Self::new_front_unit_truthful_flowviz_vessel(width, height, seed);
        sandbox.flowviz_unit_coherent = true;
        sandbox.sync_surface();
        sandbox
    }

    pub(crate) fn new_front_unit_direct_geometry_flowviz_vessel(
        width: u16,
        height: u16,
        seed: u64,
    ) -> Self {
        let mut sandbox = Self::new_front_unit_coherent_flowviz_vessel(width, height, seed);
        sandbox.flowviz_unit_direct_geometry = true;
        sandbox.sync_surface();
        sandbox
    }

    pub(crate) fn new_front_unit_visual_support_flowviz_vessel(
        width: u16,
        height: u16,
        seed: u64,
    ) -> Self {
        let mut sandbox = Self::new_front_unit_direct_geometry_flowviz_vessel(width, height, seed);
        sandbox.flowviz_unit_visual_support = true;
        sandbox.sync_surface();
        sandbox
    }

    pub(crate) fn new_fluid_vessel(width: u16, height: u16, seed: u64) -> Self {
        Self::new_with_flow(
            width,
            height,
            seed,
            OsloBoundaryMode::CanonicalWallOverflow,
            true,
            true,
            true,
        )
    }

    fn new_with_flow(
        width: u16,
        height: u16,
        seed: u64,
        boundary: OsloBoundaryMode,
        momentum_enabled: bool,
        front_enabled: bool,
        fluid_enabled: bool,
    ) -> Self {
        debug_assert!(
            !momentum_enabled || boundary == OsloBoundaryMode::CanonicalWallOverflow,
            "moving-phase Oslo is defined only for the canonical vessel sandbox"
        );
        debug_assert!(
            !front_enabled || momentum_enabled,
            "front exchange requires the causal rolling phase"
        );
        debug_assert!(
            !fluid_enabled || front_enabled,
            "partial fluidization requires the rolling/static front substrate"
        );
        let surface = SandEngine::new(width, height);
        let lattice_size = Self::lattice_size_for(&surface);
        let mut threshold_rng_state = seed ^ OSLO_THRESHOLD_RNG_XOR;
        if threshold_rng_state == 0 {
            threshold_rng_state = OSLO_THRESHOLD_RNG_XOR;
        }
        let mut rain_rng_state = seed ^ OSLO_RAIN_RNG_XOR;
        if rain_rng_state == 0 {
            rain_rng_state = OSLO_RAIN_RNG_XOR;
        }
        let mut relax_rng_state = seed ^ OSLO_RELAX_RNG_XOR;
        if relax_rng_state == 0 {
            relax_rng_state = OSLO_RELAX_RNG_XOR;
        }
        let mut sandbox = Self {
            surface,
            boundary,
            momentum_enabled,
            front_enabled,
            fluid_enabled,
            flowviz_enabled: false,
            flowviz_conservative: false,
            flowviz_unit: false,
            flowviz_unit_micro: false,
            flowviz_unit_perceptual: false,
            flowviz_unit_truthful: false,
            flowviz_unit_coherent: false,
            flowviz_unit_direct_geometry: false,
            flowviz_unit_visual_support: false,
            columns: vec![Vec::new(); lattice_size],
            column_visual_y: vec![Vec::new(); lattice_size],
            flowviz_edge_flux: vec![0; lattice_size],
            flowviz_tracers: VecDeque::new(),
            flowviz_parcels: VecDeque::new(),
            flowviz_unit_carriers: BTreeMap::new(),
            flowviz_unit_custody: vec![Vec::new(); lattice_size],
            flowviz_unit_next_id: 1,
            flowviz_unit_next_sequence: 1,
            flowviz_unit_peak: 0,
            flowviz_unit_segments: 0,
            flowviz_unit_reveals: 0,
            flowviz_unit_misses: 0,
            flowviz_shadow_columns: vec![Vec::new(); lattice_size],
            flowviz_transport_left: vec![Vec::new(); lattice_size],
            flowviz_transport_right: vec![Vec::new(); lattice_size],
            flowviz_rng_state: seed ^ 0xB529_7A4D_1C68_E9D7,
            flowviz_spawned: 0,
            flowviz_peak_tracers: 0,
            flowviz_dropped_samples: 0,
            flowviz_peak_parcels: 0,
            flowviz_mobile_mass: 0,
            flowviz_peak_mobile_mass: 0,
            flowviz_visual_withdrawals: 0,
            flowviz_visual_deposits: 0,
            flowviz_reused_deposits: 0,
            flowviz_coalesced_mass: 0,
            flowviz_shadow_misses: 0,
            critical_slopes: vec![OSLO_THRESHOLD_LOW; lattice_size],
            threshold_rng_state,
            rain_rng_state,
            relax_rng_state,
            rain_focus_site: None,
            rain_focus_target_site: None,
            rain_focus_move_counter: 0,
            rain_left_padding_targets: 0,
            rain_corridor_targets: 0,
            rain_right_padding_targets: 0,
            falling_drives: VecDeque::new(),
            rolling_grains: VecDeque::new(),
            pending_runs: VecDeque::new(),
            active_sites: VecDeque::new(),
            queued_sites: vec![false; lattice_size],
            frame_count: 0,
            discharged: 0,
            total_generated: 0,
            avalanche_moves: 0,
            avalanche_peak_moves: 0,
            avalanche_last_moves: 0,
            avalanche_completed: 0,
            recent_avalanches: VecDeque::with_capacity(RECENT_AVALANCHE_WINDOW),
            momentum_seeds: 0,
            momentum_hops: 0,
            momentum_settles: 0,
            momentum_peak_active: 0,
            momentum_event_seeds: 0,
            momentum_event_hops: 0,
            momentum_event_peak_active: 0,
            momentum_last_seeds: 0,
            momentum_last_hops: 0,
            momentum_last_peak_active: 0,
            front_erosions: 0,
            front_support_recruits: 0,
            front_event_erosions: 0,
            front_event_support_recruits: 0,
            front_last_erosions: 0,
            front_last_support_recruits: 0,
            fluidity: vec![0; lattice_size],
            fluid_activations: 0,
            fluid_releases: 0,
            fluid_peak_active_sites: 0,
            fluid_event_activations: 0,
            fluid_event_releases: 0,
            fluid_last_activations: 0,
            fluid_last_releases: 0,
        };
        sandbox.randomize_all_thresholds();
        sandbox.sync_surface();
        sandbox
    }

    pub(crate) fn spawn(&mut self, category_id: CategoryId) {
        if let Some(x) = self.choose_drive_site() {
            self.falling_drives.push_back(FallingDrive {
                x,
                y: self.visible_vertical_bounds().0,
                category_id,
            });
        } else {
            self.append_pending_run(category_id, 1);
        }
        self.total_generated = self.total_generated.saturating_add(1);
    }

    #[allow(dead_code)]
    pub(crate) fn update(&mut self) {
        if self.update_deferred() {
            self.sync_surface();
        }
    }

    /// Advance one Oslo physics tick without rebuilding the presentation grid.
    ///
    /// Testing-cheat acceleration can execute thousands of simulated ticks in a
    /// short wall-clock interval. Keeping the lattice authoritative during that
    /// batch and synchronizing the Ratatui surface only once per cooperative
    /// chunk avoids O(grid) redraw preparation on every 32 ms simulated tick.
    pub(crate) fn update_deferred(&mut self) -> bool {
        self.frame_count = self.frame_count.wrapping_add(1);
        if !self.frame_count.is_multiple_of(2) {
            return false;
        }
        self.advance_full_physics_frame()
    }

    /// One *effective* 64 ms Oslo frame used by testingcheats while visible flow
    /// is playing on the wall clock. This bypasses the historical every-other
    /// 32 ms wrapper without changing the order or content of the actual frame.
    pub(crate) fn advance_testing_flow_frame(&mut self) -> bool {
        self.frame_count = if self.frame_count.is_multiple_of(2) {
            self.frame_count.wrapping_add(2)
        } else {
            self.frame_count.wrapping_add(1)
        };
        self.advance_full_physics_frame()
    }

    fn advance_full_physics_frame(&mut self) -> bool {
        let mut changed = self.launch_one_pending();
        changed |= self.advance_falling_drives();
        changed |= self.advance_fluidization_field();
        changed |= self.advance_rolling_grains();
        // Presentation follows every physical frame concurrently. Crucially,
        // physics never waits for all colored grains to finish their vertical
        // interpolation; all active transports move together.
        changed |= self.advance_rolling_visual_motion();
        if self.topple_one_active_site() {
            return true;
        }

        changed |= self.commit_next_drive_if_quiescent();
        changed
    }

    /// Advance only visible presentation while authoritative physics is already
    /// quiescent. Used to let settled CategoryId transport finish at a steady
    /// grain clock without turning that animation into new sediment events.
    pub(crate) fn advance_testing_visual_frame(&mut self) -> bool {
        let mut changed = self.advance_falling_drives();
        changed |= self.advance_rolling_visual_motion();
        changed
    }

    /// Drain an invisible partial-fluidization order field cooperatively only
    /// when no rolling grain and no queued Oslo site can interleave with it.
    /// Falling dots are presentation/drive custody at this point and cannot be
    /// committed while the fluid field remains explicit flow.
    pub(crate) fn advance_testing_latent_fluid_once(&mut self) -> bool {
        debug_assert!(self.rolling_grains.is_empty());
        debug_assert!(self.active_sites.is_empty());
        self.advance_fluidization_field()
    }

    pub(crate) fn sync_for_render(&mut self) {
        self.sync_surface();
    }

    pub(crate) fn resize(&mut self, width: u16, height: u16) {
        let old_dimensions = self.dimensions();
        let old_visible_bounds = self.visible_lattice_bounds();
        let old_visible_vertical = self.visible_vertical_bounds();
        let old_width = self.columns.len();
        let old_height = self.surface.grid_height_dots;
        self.surface.resize(width, height);
        let new_width = self.surface.grid_width_dots;
        let new_height = self.surface.grid_height_dots;
        let vertical_added = new_height.saturating_sub(old_height);
        let perceptual_vertical_shift = new_height as f32 - old_height as f32;
        let mut left_added = 0usize;

        if new_width > old_width {
            let added = new_width - old_width;
            left_added = added / 2;
            let mut columns = vec![Vec::new(); new_width];
            for (index, column) in self.columns.drain(..).enumerate() {
                columns[left_added + index] = column;
            }
            self.columns = columns;

            let mut column_visual_y = vec![Vec::new(); new_width];
            for (index, visual) in self.column_visual_y.drain(..).enumerate() {
                column_visual_y[left_added + index] = visual;
            }
            self.column_visual_y = column_visual_y;
            self.resize_flowviz_for_growth(left_added, new_width);

            let old_fluidity = std::mem::take(&mut self.fluidity);
            self.fluidity = vec![0; new_width];
            for (index, fluidity) in old_fluidity.into_iter().enumerate() {
                self.fluidity[left_added + index] = fluidity;
            }

            let old_slopes = std::mem::take(&mut self.critical_slopes);
            self.critical_slopes = vec![OSLO_THRESHOLD_LOW; new_width];
            for (index, slope) in old_slopes.into_iter().enumerate() {
                self.critical_slopes[left_added + index] = slope;
            }
            for index in 0..left_added {
                self.critical_slopes[index] = self.sample_threshold();
            }
            for index in (left_added + old_width)..new_width {
                self.critical_slopes[index] = self.sample_threshold();
            }

            for falling in &mut self.falling_drives {
                falling.x = falling.x.saturating_add(left_added);
            }
            for rolling in &mut self.rolling_grains {
                rolling.site = rolling.site.saturating_add(left_added);
            }
            for site in &mut self.active_sites {
                *site = site.saturating_add(left_added);
            }
            self.rain_focus_site = self.rain_focus_site.map(|site| site + left_added);
            self.rain_focus_target_site = self.rain_focus_target_site.map(|site| site + left_added);
        }

        let shifted_old_visible = (
            old_visible_bounds.0.saturating_add(left_added),
            old_visible_bounds.1.saturating_add(left_added),
        );
        let new_visible = self.visible_lattice_bounds();
        let old_visible_width = shifted_old_visible.1.saturating_sub(shifted_old_visible.0);
        let new_visible_width = new_visible.1.saturating_sub(new_visible.0);

        // Falling drives are transient presentation state, not settled topology.
        // If the terminal shrinks, project their lateral position into the new
        // visible window so an off-screen in-flight grain cannot block FIFO Oslo
        // deposition. Canonical settled columns never move on shrink.
        if new_visible_width > 0 && new_visible_width < old_visible_width {
            for falling in &mut self.falling_drives {
                falling.x =
                    Self::project_site_between_bounds(falling.x, shifted_old_visible, new_visible);
            }
            for rolling in &mut self.rolling_grains {
                rolling.site = Self::project_site_between_bounds(
                    rolling.site,
                    shifted_old_visible,
                    new_visible,
                );
            }
            if self.flowviz_enabled {
                for tracer in &mut self.flowviz_tracers {
                    let projected = Self::project_site_between_bounds(
                        tracer.x.max(0.0).floor() as usize,
                        shifted_old_visible,
                        new_visible,
                    );
                    tracer.x = projected as f32 + 0.5;
                }
                for parcel in &mut self.flowviz_parcels {
                    let projected = Self::project_site_between_bounds(
                        parcel.x.max(0.0).floor() as usize,
                        shifted_old_visible,
                        new_visible,
                    );
                    parcel.x = projected as f32 + 0.5;
                }
                if self.flowviz_unit {
                    for carrier in self.flowviz_unit_carriers.values_mut() {
                        if carrier.physical != flowviz_unit::UnitPhysicalState::Rolling {
                            continue;
                        }
                        let projected = Self::project_site_between_bounds(
                            carrier.x.max(0.0).floor() as usize,
                            shifted_old_visible,
                            new_visible,
                        );
                        carrier.x = projected as f32 + 0.5;
                        let ideal_projected = Self::project_site_between_bounds(
                            carrier.ideal_x.max(0.0).floor() as usize,
                            shifted_old_visible,
                            new_visible,
                        );
                        carrier.ideal_x = ideal_projected as f32 + 0.5;
                        if let Some(active) = &mut carrier.active {
                            active.start_x = carrier.ideal_x;
                            let target_projected = Self::project_site_between_bounds(
                                active.target_x.max(0.0).floor() as usize,
                                shifted_old_visible,
                                new_visible,
                            );
                            active.target_x = target_projected as f32 + 0.5;
                            active.segment.source = Self::project_site_between_bounds(
                                active.segment.source,
                                shifted_old_visible,
                                new_visible,
                            );
                            active.segment.destination = active.segment.destination.map(|site| {
                                Self::project_site_between_bounds(
                                    site,
                                    shifted_old_visible,
                                    new_visible,
                                )
                            });
                        }
                        for segment in &mut carrier.queued {
                            segment.source = Self::project_site_between_bounds(
                                segment.source,
                                shifted_old_visible,
                                new_visible,
                            );
                            segment.destination = segment.destination.map(|site| {
                                Self::project_site_between_bounds(
                                    site,
                                    shifted_old_visible,
                                    new_visible,
                                )
                            });
                        }
                    }
                }
            }
        }

        let shifted_old_vertical = (
            old_visible_vertical.0.saturating_add(vertical_added),
            old_visible_vertical.1.saturating_add(vertical_added),
        );
        let new_visible_vertical = self.visible_vertical_bounds();
        let old_visible_height = shifted_old_vertical
            .1
            .saturating_sub(shifted_old_vertical.0);
        let new_visible_height = new_visible_vertical
            .1
            .saturating_sub(new_visible_vertical.0);
        for falling in &mut self.falling_drives {
            falling.y = falling.y.saturating_add(vertical_added);
            if new_visible_height == 0 {
                falling.y = 0;
            } else if new_visible_height < old_visible_height {
                falling.y = Self::project_site_between_bounds(
                    falling.y,
                    shifted_old_vertical,
                    new_visible_vertical,
                );
            } else {
                falling.y = falling
                    .y
                    .clamp(new_visible_vertical.0, new_visible_vertical.1 - 1);
            }
        }
        for rolling in &mut self.rolling_grains {
            rolling.visual_y = rolling.visual_y.saturating_add(vertical_added);
            if new_visible_height == 0 {
                rolling.visual_y = 0;
            } else if new_visible_height < old_visible_height {
                rolling.visual_y = Self::project_site_between_bounds(
                    rolling.visual_y,
                    shifted_old_vertical,
                    new_visible_vertical,
                );
            } else {
                rolling.visual_y = rolling
                    .visual_y
                    .clamp(new_visible_vertical.0, new_visible_vertical.1 - 1);
            }
        }
        if self.flowviz_unit_direct_geometry && perceptual_vertical_shift != 0.0 {
            for rolling in &mut self.rolling_grains {
                if let Some(observed_y) = &mut rolling.unit_observed_y {
                    *observed_y += perceptual_vertical_shift;
                }
            }
        }
        if self.flowviz_enabled {
            for tracer in &mut self.flowviz_tracers {
                let mut y = tracer.y.max(0.0).floor() as usize;
                y = y.saturating_add(vertical_added);
                if new_visible_height == 0 {
                    tracer.y = 0.0;
                } else if new_visible_height < old_visible_height {
                    tracer.y = Self::project_site_between_bounds(
                        y,
                        shifted_old_vertical,
                        new_visible_vertical,
                    ) as f32;
                } else {
                    tracer.y = y.clamp(new_visible_vertical.0, new_visible_vertical.1 - 1) as f32;
                }
            }
            for parcel in &mut self.flowviz_parcels {
                let mut y = parcel.y.max(0.0).floor() as usize;
                y = y.saturating_add(vertical_added);
                if new_visible_height == 0 {
                    parcel.y = 0.0;
                } else if new_visible_height < old_visible_height {
                    parcel.y = Self::project_site_between_bounds(
                        y,
                        shifted_old_vertical,
                        new_visible_vertical,
                    ) as f32;
                } else {
                    parcel.y = y.clamp(new_visible_vertical.0, new_visible_vertical.1 - 1) as f32;
                }
            }
            if self.flowviz_unit {
                if self.flowviz_unit_perceptual {
                    // 015C y geometry is recorded relative to the bottom-anchored
                    // physical stack. Resize therefore applies one exact signed
                    // bottom shift and preserves offscreen coordinates instead of
                    // clamping them onto the visible top/bottom rows.
                    for carrier in self.flowviz_unit_carriers.values_mut() {
                        carrier.y += perceptual_vertical_shift;
                        carrier.ideal_y += perceptual_vertical_shift;
                        if let Some(active) = &mut carrier.active {
                            active.start_y += perceptual_vertical_shift;
                            active.target_y += perceptual_vertical_shift;
                            if let Some(source_y) = &mut active.segment.observed_source_y {
                                *source_y += perceptual_vertical_shift;
                            }
                            if let Some(target_y) = &mut active.segment.observed_target_y {
                                *target_y += perceptual_vertical_shift;
                            }
                        }
                        for segment in &mut carrier.queued {
                            if let Some(source_y) = &mut segment.observed_source_y {
                                *source_y += perceptual_vertical_shift;
                            }
                            if let Some(target_y) = &mut segment.observed_target_y {
                                *target_y += perceptual_vertical_shift;
                            }
                        }
                    }
                } else {
                    for carrier in self.flowviz_unit_carriers.values_mut() {
                        let mut y = carrier.y.max(0.0).floor() as usize;
                        y = y.saturating_add(vertical_added);
                        carrier.y = if new_visible_height == 0 {
                            0.0
                        } else if new_visible_height < old_visible_height {
                            Self::project_site_between_bounds(
                                y,
                                shifted_old_vertical,
                                new_visible_vertical,
                            ) as f32
                        } else {
                            y.clamp(new_visible_vertical.0, new_visible_vertical.1 - 1) as f32
                        };
                        let mut ideal_y = carrier.ideal_y.max(0.0).floor() as usize;
                        ideal_y = ideal_y.saturating_add(vertical_added);
                        carrier.ideal_y = if new_visible_height == 0 {
                            0.0
                        } else if new_visible_height < old_visible_height {
                            Self::project_site_between_bounds(
                                ideal_y,
                                shifted_old_vertical,
                                new_visible_vertical,
                            ) as f32
                        } else {
                            ideal_y.clamp(new_visible_vertical.0, new_visible_vertical.1 - 1) as f32
                        };
                        if let Some(active) = &mut carrier.active {
                            let mut start_y = active.start_y.max(0.0).floor() as usize;
                            start_y = start_y.saturating_add(vertical_added);
                            active.start_y = if new_visible_height == 0 {
                                0.0
                            } else if new_visible_height < old_visible_height {
                                Self::project_site_between_bounds(
                                    start_y,
                                    shifted_old_vertical,
                                    new_visible_vertical,
                                ) as f32
                            } else {
                                start_y.clamp(new_visible_vertical.0, new_visible_vertical.1 - 1)
                                    as f32
                            };
                            let mut target_y = active.target_y.max(0.0).floor() as usize;
                            target_y = target_y.saturating_add(vertical_added);
                            active.target_y = if new_visible_height == 0 {
                                0.0
                            } else if new_visible_height < old_visible_height {
                                Self::project_site_between_bounds(
                                    target_y,
                                    shifted_old_vertical,
                                    new_visible_vertical,
                                ) as f32
                            } else {
                                target_y.clamp(new_visible_vertical.0, new_visible_vertical.1 - 1)
                                    as f32
                            };
                        }
                    }
                }
            }
        }
        for column in &mut self.column_visual_y {
            for visual_y in column.iter_mut().flatten() {
                *visual_y = visual_y.saturating_add(vertical_added);
                if new_visible_height == 0 {
                    *visual_y = 0;
                } else if new_visible_height < old_visible_height {
                    *visual_y = Self::project_site_between_bounds(
                        *visual_y,
                        shifted_old_vertical,
                        new_visible_vertical,
                    );
                } else {
                    *visual_y =
                        (*visual_y).clamp(new_visible_vertical.0, new_visible_vertical.1 - 1);
                }
            }
        }

        // Hidden canonical columns are custody only while the viewport is smaller.
        // Freeze their Oslo activity and rebuild the exact visible queue. On
        // re-expansion, only newly visible sites plus the former visible edges are
        // scheduled so a changed local boundary can settle through ordinary Oslo.
        self.active_sites
            .retain(|site| *site >= new_visible.0 && *site < new_visible.1);
        self.queued_sites = vec![false; self.columns.len()];
        for &site in &self.active_sites {
            self.queued_sites[site] = true;
        }
        if self.fluid_enabled {
            for (site, value) in self.fluidity.iter_mut().enumerate() {
                if site < new_visible.0 || site >= new_visible.1 {
                    *value = 0;
                }
            }
        }
        if old_dimensions != (width, height) || new_width > old_width {
            for site in new_visible.0..new_visible.1 {
                if site < shifted_old_visible.0 || site >= shifted_old_visible.1 {
                    self.enqueue_site(site);
                }
            }
            for site in [
                shifted_old_visible.0,
                shifted_old_visible.0.saturating_sub(1),
                shifted_old_visible.1.saturating_sub(1),
                shifted_old_visible.1,
            ] {
                self.enqueue_site(site);
            }
        }

        self.clamp_focus_to_visible_corridor();
        self.sync_surface();
    }

    fn project_site_between_bounds(
        site: usize,
        old_bounds: (usize, usize),
        new_bounds: (usize, usize),
    ) -> usize {
        let old_width = old_bounds.1.saturating_sub(old_bounds.0);
        let new_width = new_bounds.1.saturating_sub(new_bounds.0);
        debug_assert!(new_width > 0);
        if new_width == 1 || old_width <= 1 {
            return new_bounds.0 + new_width.saturating_sub(1) / 2;
        }

        let old_site = site.clamp(old_bounds.0, old_bounds.1 - 1) - old_bounds.0;
        let old_span = old_width - 1;
        let new_span = new_width - 1;
        let projected = old_site
            .saturating_mul(new_span)
            .saturating_add(old_span / 2)
            / old_span;
        new_bounds.0 + projected.min(new_span)
    }

    pub(crate) fn clear(&mut self) {
        self.surface.clear();
        let lattice_size = Self::lattice_size_for(&self.surface);
        self.columns = vec![Vec::new(); lattice_size];
        self.column_visual_y = vec![Vec::new(); lattice_size];
        self.critical_slopes = vec![OSLO_THRESHOLD_LOW; lattice_size];
        self.falling_drives.clear();
        self.rolling_grains.clear();
        self.clear_flowviz();
        self.pending_runs.clear();
        self.active_sites.clear();
        self.queued_sites = vec![false; lattice_size];
        self.discharged = 0;
        self.total_generated = 0;
        self.avalanche_moves = 0;
        self.avalanche_peak_moves = 0;
        self.avalanche_last_moves = 0;
        self.avalanche_completed = 0;
        self.recent_avalanches.clear();
        self.momentum_seeds = 0;
        self.momentum_hops = 0;
        self.momentum_settles = 0;
        self.momentum_peak_active = 0;
        self.momentum_event_seeds = 0;
        self.momentum_event_hops = 0;
        self.momentum_event_peak_active = 0;
        self.momentum_last_seeds = 0;
        self.momentum_last_hops = 0;
        self.momentum_last_peak_active = 0;
        self.front_erosions = 0;
        self.front_support_recruits = 0;
        self.front_event_erosions = 0;
        self.front_event_support_recruits = 0;
        self.front_last_erosions = 0;
        self.front_last_support_recruits = 0;
        self.fluidity = vec![0; lattice_size];
        self.fluid_activations = 0;
        self.fluid_releases = 0;
        self.fluid_peak_active_sites = 0;
        self.fluid_event_activations = 0;
        self.fluid_event_releases = 0;
        self.fluid_last_activations = 0;
        self.fluid_last_releases = 0;
        self.rain_focus_site = None;
        self.rain_focus_target_site = None;
        self.rain_focus_move_counter = 0;
        self.rain_left_padding_targets = 0;
        self.rain_corridor_targets = 0;
        self.rain_right_padding_targets = 0;
        self.randomize_all_thresholds();
        self.sync_surface();
    }

    pub(crate) fn debug_fill_rainbow_80(
        &mut self,
        category_ids: &[CategoryId],
    ) -> Result<usize, String> {
        if category_ids.is_empty() {
            return Err("testingcheats fill requires at least one configured layer".to_string());
        }
        self.clear();
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        let fill_height = self.visible_height().saturating_mul(4) / 5;
        if fill_height == 0 || visible_start >= visible_end {
            return Ok(0);
        }
        for site in visible_start..visible_end {
            let mut filled = Vec::with_capacity(fill_height);
            for depth in 0..fill_height {
                let layer = depth.saturating_mul(category_ids.len()) / fill_height;
                filled.push(category_ids[layer.min(category_ids.len() - 1)]);
            }
            self.columns[site] = filled;
            self.column_visual_y[site] = vec![None; fill_height];
        }
        self.total_generated = self.settled_count();
        self.reset_conservative_flowviz_from_physics();
        self.reset_unit_flowviz_from_physics();
        for site in visible_start..visible_end {
            self.enqueue_neighborhood(site);
        }
        self.sync_surface();
        Ok(self.settled_count())
    }

    pub(crate) fn render(&self, categories: &[Category]) -> Vec<Line<'static>> {
        self.surface.render(categories)
    }

    pub(crate) fn dimensions(&self) -> (u16, u16) {
        (self.surface.cell_width, self.surface.cell_height)
    }

    pub(crate) fn grain_count(&self) -> usize {
        self.settled_count().saturating_add(self.pending_count())
    }

    pub(crate) fn generated_count(&self) -> usize {
        self.total_generated
    }

    pub(crate) fn settled_count(&self) -> usize {
        self.columns.iter().map(Vec::len).sum()
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.falling_drives
            .len()
            .saturating_add(self.rolling_grains.len())
            .saturating_add(self.pending_runs.iter().map(|run| run.count).sum::<usize>())
    }

    pub(crate) fn discharged_count(&self) -> usize {
        self.discharged
    }

    #[cfg(test)]
    pub(crate) fn lattice_size(&self) -> usize {
        self.columns.len()
    }

    #[cfg(test)]
    pub(crate) fn test_seed_visible_flow(&mut self) {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        if visible_start >= visible_end {
            return;
        }
        self.seed_rolling_grain(visible_start, CategoryId::new(1), ToppleDirection::Right);
    }

    #[cfg(test)]
    pub(crate) fn test_seed_visual_transit(&mut self) {
        let (visible_start, visible_end) = self.visible_lattice_bounds();
        if visible_start >= visible_end || self.surface.grid_height_dots == 0 {
            return;
        }
        let visual_y = self.visible_vertical_bounds().0;
        self.push_settled_grain_at_visual_y(visible_start, CategoryId::new(1), Some(visual_y));
    }

    pub(crate) fn avalanche_peak_moves(&self) -> usize {
        self.avalanche_peak_moves
    }

    pub(crate) fn avalanche_last_moves(&self) -> usize {
        self.avalanche_last_moves
    }

    pub(crate) fn avalanche_completed(&self) -> usize {
        self.avalanche_completed
    }

    pub(crate) fn avalanche_recent_p95(&self) -> usize {
        if self.recent_avalanches.is_empty() {
            return 0;
        }
        let mut values = self.recent_avalanches.iter().copied().collect::<Vec<_>>();
        values.sort_unstable();
        values[(values.len() - 1) * 95 / 100]
    }

    pub(crate) fn visible_height(&self) -> usize {
        let (top, bottom) = self.visible_vertical_bounds();
        bottom.saturating_sub(top)
    }

    pub(crate) fn model_name(&self) -> &'static str {
        if self.flowviz_unit {
            "oslo-vessel-front-grains"
        } else if self.flowviz_conservative {
            "oslo-vessel-front-parcels"
        } else if self.flowviz_enabled {
            "oslo-vessel-front-flowviz"
        } else if self.fluid_enabled {
            "oslo-vessel-fluid"
        } else if self.front_enabled {
            "oslo-vessel-front"
        } else if self.momentum_enabled {
            "oslo-vessel-momentum"
        } else {
            self.boundary.model_name()
        }
    }

    pub(crate) fn momentum_enabled(&self) -> bool {
        self.momentum_enabled
    }

    pub(crate) fn front_enabled(&self) -> bool {
        self.front_enabled
    }

    pub(crate) fn fluid_enabled(&self) -> bool {
        self.fluid_enabled
    }

    pub(crate) fn explicit_flow_active(&self) -> bool {
        !self.rolling_grains.is_empty() || self.fluidity.iter().any(|value| *value > 0)
    }

    /// Visible avalanche physics, distinct from ordinary falling rain. A latent
    /// fluidity field with no rolling mass is not a reason to wall-clock-throttle
    /// every invisible order-field step.
    pub(crate) fn visible_flow_active(&self) -> bool {
        !self.rolling_grains.is_empty() || !self.active_sites.is_empty()
    }

    pub(crate) fn latent_flow_active(&self) -> bool {
        self.fluidity.iter().any(|value| *value > 0)
            && self.rolling_grains.is_empty()
            && self.active_sites.is_empty()
    }

    pub(crate) fn fluid_active_sites(&self) -> usize {
        self.fluidity.iter().filter(|value| **value > 0).count()
    }

    pub(crate) fn fluid_activations(&self) -> usize {
        self.fluid_activations
    }

    pub(crate) fn fluid_releases(&self) -> usize {
        self.fluid_releases
    }

    pub(crate) fn fluid_peak_active_sites(&self) -> usize {
        self.fluid_peak_active_sites
    }

    pub(crate) fn fluid_last_activations(&self) -> usize {
        self.fluid_last_activations
    }

    pub(crate) fn fluid_last_releases(&self) -> usize {
        self.fluid_last_releases
    }

    pub(crate) fn rolling_count(&self) -> usize {
        self.rolling_grains.len()
    }

    pub(crate) fn rolling_in_transit_count(&self) -> usize {
        self.total_visual_in_transit_count()
    }

    pub(crate) fn momentum_seeds(&self) -> usize {
        self.momentum_seeds
    }

    pub(crate) fn momentum_hops(&self) -> usize {
        self.momentum_hops
    }

    pub(crate) fn momentum_settles(&self) -> usize {
        self.momentum_settles
    }

    pub(crate) fn momentum_peak_active(&self) -> usize {
        self.momentum_peak_active
    }

    pub(crate) fn momentum_last_seeds(&self) -> usize {
        self.momentum_last_seeds
    }

    pub(crate) fn momentum_last_hops(&self) -> usize {
        self.momentum_last_hops
    }

    pub(crate) fn momentum_last_peak_active(&self) -> usize {
        self.momentum_last_peak_active
    }

    pub(crate) fn front_erosions(&self) -> usize {
        self.front_erosions
    }

    pub(crate) fn front_support_recruits(&self) -> usize {
        self.front_support_recruits
    }

    pub(crate) fn front_last_erosions(&self) -> usize {
        self.front_last_erosions
    }

    pub(crate) fn front_last_support_recruits(&self) -> usize {
        self.front_last_support_recruits
    }

    pub(crate) fn canonical_wall_height(&self) -> usize {
        self.surface.grid_height_dots
    }

    pub(crate) fn visible_profile(&self) -> (usize, usize, usize, usize) {
        let (start, end) = self.visible_lattice_bounds();
        if start >= end {
            return (0, 0, 0, 0);
        }
        let heights = &self.columns[start..end];
        let min = heights.iter().map(Vec::len).min().unwrap_or(0);
        let max = heights.iter().map(Vec::len).max().unwrap_or(0);
        let left = heights.first().map_or(0, Vec::len);
        let right = heights.last().map_or(0, Vec::len);
        (min, max, left, right)
    }

    pub(crate) fn rain_region_counts(&self) -> (usize, usize, usize) {
        (
            self.rain_left_padding_targets,
            self.rain_corridor_targets,
            self.rain_right_padding_targets,
        )
    }

    pub(crate) fn canonical_dimensions(&self) -> (usize, usize) {
        (self.columns.len(), self.surface.grid_height_dots)
    }

    fn lattice_size_for(surface: &SandEngine) -> usize {
        surface.grid_width_dots
    }
}

mod fluidization;
mod momentum;
mod physics;
mod rain;
#[cfg(test)]
mod tests;
