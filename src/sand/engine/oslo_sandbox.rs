use std::collections::VecDeque;

use crate::domain::{Category, CategoryId};
use ratatui::prelude::Line;

use super::{PendingGrainRun, SandEngine};

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
    category_id: CategoryId,
    direction: ToppleDirection,
    flat_coast_remaining: u8,
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
    columns: Vec<Vec<CategoryId>>,
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
}
impl OsloSandboxEngine {
    pub(crate) fn new(width: u16, height: u16, seed: u64, boundary: OsloBoundaryMode) -> Self {
        Self::new_with_flow(width, height, seed, boundary, false, false)
    }

    pub(crate) fn new_momentum_vessel(width: u16, height: u16, seed: u64) -> Self {
        Self::new_with_flow(
            width,
            height,
            seed,
            OsloBoundaryMode::CanonicalWallOverflow,
            true,
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
        )
    }

    fn new_with_flow(
        width: u16,
        height: u16,
        seed: u64,
        boundary: OsloBoundaryMode,
        momentum_enabled: bool,
        front_enabled: bool,
    ) -> Self {
        debug_assert!(
            !momentum_enabled || boundary == OsloBoundaryMode::CanonicalWallOverflow,
            "moving-phase Oslo is defined only for the canonical vessel sandbox"
        );
        debug_assert!(
            !front_enabled || momentum_enabled,
            "front exchange requires the causal rolling phase"
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
            columns: vec![Vec::new(); lattice_size],
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

        let mut changed = self.launch_one_pending();
        changed |= self.advance_falling_drives();
        changed |= self.advance_rolling_grains();
        if self.topple_one_active_site() {
            return true;
        }

        changed |= self.commit_next_drive_if_quiescent();
        changed
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
        let vertical_added = self.surface.grid_height_dots.saturating_sub(old_height);
        let mut left_added = 0usize;

        if new_width > old_width {
            let added = new_width - old_width;
            left_added = added / 2;
            let mut columns = vec![Vec::new(); new_width];
            for (index, column) in self.columns.drain(..).enumerate() {
                columns[left_added + index] = column;
            }
            self.columns = columns;

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
        self.critical_slopes = vec![OSLO_THRESHOLD_LOW; lattice_size];
        self.falling_drives.clear();
        self.rolling_grains.clear();
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
        self.rain_focus_site = None;
        self.rain_focus_target_site = None;
        self.rain_focus_move_counter = 0;
        self.rain_left_padding_targets = 0;
        self.rain_corridor_targets = 0;
        self.rain_right_padding_targets = 0;
        self.randomize_all_thresholds();
        self.sync_surface();
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
        if self.front_enabled {
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

    pub(crate) fn rolling_count(&self) -> usize {
        self.rolling_grains.len()
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

mod momentum;
mod physics;
mod rain;
#[cfg(test)]
mod tests;
