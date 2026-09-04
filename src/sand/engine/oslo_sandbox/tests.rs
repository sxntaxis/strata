use super::*;

const TEST_SEED: u64 = 0x51A7_A51A_0B10_5EED;

fn grain() -> CategoryId {
    CategoryId::new(1)
}

fn set_column_height(engine: &mut OsloSandboxEngine, site: usize, height: usize) {
    engine.columns[site] = vec![grain(); height];
}

fn direct_drive_and_relax(engine: &mut OsloSandboxEngine, site: usize) -> usize {
    engine.columns[site].push(grain());
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);
    let mut moves = 0usize;
    while engine.topple_one_active_site() {
        moves = moves.saturating_add(1);
    }
    moves
}

fn p95(values: &mut [usize]) -> usize {
    values.sort_unstable();
    values[(values.len() - 1) * 95 / 100]
}

#[test]
fn thresholds_are_exactly_one_or_two() {
    let engine = OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::ClosedBox);
    assert!(
        engine
            .critical_slopes
            .iter()
            .all(|threshold| matches!(*threshold, OSLO_THRESHOLD_LOW | OSLO_THRESHOLD_HIGH))
    );
}

#[test]
fn zero_outside_control_discharges_at_the_visible_edge() {
    let mut engine = OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::ZeroOutside);
    set_column_height(&mut engine, 0, 3);
    set_column_height(&mut engine, 1, 2);
    engine.critical_slopes[0] = 1;

    assert!(engine.topple_site_if_unstable(0));
    assert_eq!(engine.columns[0].len(), 2);
    assert_eq!(engine.columns[1].len(), 2);
    assert_eq!(engine.discharged_count(), 1);
}

#[test]
fn closed_box_removes_the_missing_neighbor_from_edge_relief() {
    let mut engine = OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::ClosedBox);
    set_column_height(&mut engine, 0, 3);
    set_column_height(&mut engine, 1, 2);
    engine.critical_slopes[0] = 1;

    assert!(!engine.topple_site_if_unstable(0));
    assert_eq!(engine.columns[0].len(), 3);
    assert_eq!(engine.discharged_count(), 0);
}

#[test]
fn canonical_vessel_is_closed_below_the_wall_top() {
    let mut engine =
        OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::CanonicalWallOverflow);
    let wall = engine.canonical_wall_height();
    set_column_height(&mut engine, 0, wall.saturating_sub(1));
    set_column_height(&mut engine, 1, wall.saturating_sub(1));
    engine.critical_slopes[0] = 1;

    assert!(!engine.topple_site_if_unstable(0));
    assert_eq!(engine.discharged_count(), 0);
}

#[test]
fn canonical_vessel_overflows_only_above_the_wall_top() {
    let mut engine =
        OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::CanonicalWallOverflow);
    let wall = engine.canonical_wall_height();
    set_column_height(&mut engine, 0, wall + 3);
    set_column_height(&mut engine, 1, wall + 3);
    engine.critical_slopes[0] = 1;

    assert!(engine.topple_site_if_unstable(0));
    assert_eq!(engine.columns[0].len(), wall + 2);
    assert_eq!(engine.columns[1].len(), wall + 3);
    assert_eq!(engine.discharged_count(), 1);
}

#[test]
fn canonical_wall_height_does_not_shrink_with_the_viewport() {
    let mut engine =
        OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::CanonicalWallOverflow);
    let wall = engine.canonical_wall_height();
    engine.resize(20, 5);

    assert_eq!(engine.canonical_wall_height(), wall);
    assert_eq!(engine.canonical_dimensions().1, wall);
}

#[test]
fn temporary_visible_wall_blocks_hidden_neighbor_until_reexpansion() {
    let mut engine = OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::ClosedBox);
    let canonical_width = engine.lattice_size();
    engine.resize(10, 10);
    let (visible_start, visible_end) = engine.visible_lattice_bounds();
    assert!(visible_start > 0);
    assert!(visible_end < canonical_width);

    let source = visible_start;
    let hidden_left = source - 1;
    set_column_height(&mut engine, source, 4);
    set_column_height(&mut engine, source + 1, 4);
    set_column_height(&mut engine, hidden_left, 0);
    engine.critical_slopes[source] = 1;
    engine.active_sites.clear();
    engine.queued_sites.fill(false);

    assert!(!engine.topple_site_if_unstable(source));
    assert_eq!(engine.columns[hidden_left].len(), 0);

    engine.resize(20, 10);
    engine.active_sites.clear();
    engine.queued_sites.fill(false);
    engine.critical_slopes[source] = 1;
    assert!(engine.topple_site_if_unstable(source));
    assert_eq!(engine.columns[hidden_left].len(), 1);
}

#[test]
fn closed_box_never_discharges_under_long_conservative_drive() {
    let mut engine = OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::ClosedBox);
    let width = engine.lattice_size();
    for drive in 0..6_000usize {
        direct_drive_and_relax(&mut engine, drive % width);
    }

    assert_eq!(engine.discharged_count(), 0);
    assert_eq!(engine.settled_count(), 6_000);
    assert_eq!(engine.generated_count(), engine.grain_count());
}

#[test]
fn canonical_vessel_recovers_a_broad_avalanche_tail_after_filling() {
    let mut engine =
        OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::CanonicalWallOverflow);
    let width = engine.lattice_size();
    let mut tail = Vec::new();

    for drive in 0..6_000usize {
        let moves = direct_drive_and_relax(&mut engine, drive % width);
        if drive >= 2_000 && moves > 0 {
            tail.push(moves);
        }
    }

    assert!(
        engine.discharged_count() > 0,
        "vessel never reached overflow"
    );
    assert_eq!(
        engine.generated_count(),
        engine
            .grain_count()
            .saturating_add(engine.discharged_count())
    );
    assert!(tail.len() > 100, "too few post-fill avalanche events");
    let percentile = p95(&mut tail);
    let maximum = tail.iter().copied().max().unwrap_or(0);
    assert!(
        percentile >= 3,
        "expected a non-microscopic tail, got p95={percentile}"
    );
    assert!(
        maximum >= percentile.saturating_mul(3),
        "expected a broad tail, got p95={percentile}, max={maximum}"
    );
}

#[test]
fn deferred_updates_match_eager_oslo_state_after_surface_sync() {
    let mut eager = OsloSandboxEngine::new(40, 20, TEST_SEED, OsloBoundaryMode::ZeroOutside);
    let mut deferred = OsloSandboxEngine::new(40, 20, TEST_SEED, OsloBoundaryMode::ZeroOutside);
    for _ in 0..128 {
        eager.spawn(grain());
        deferred.spawn(grain());
    }

    let surface_before = deferred.surface.grain_count;
    for _ in 0..2_000 {
        eager.update();
        deferred.update_deferred();
    }

    assert_eq!(deferred.surface.grain_count, surface_before);
    assert_eq!(deferred.columns, eager.columns);
    assert_eq!(deferred.critical_slopes, eager.critical_slopes);
    assert_eq!(deferred.threshold_rng_state, eager.threshold_rng_state);
    assert_eq!(deferred.rain_rng_state, eager.rain_rng_state);
    assert_eq!(deferred.relax_rng_state, eager.relax_rng_state);
    assert_eq!(deferred.rain_focus_site, eager.rain_focus_site);
    assert_eq!(
        deferred.rain_focus_target_site,
        eager.rain_focus_target_site
    );
    assert_eq!(
        deferred.rain_focus_move_counter,
        eager.rain_focus_move_counter
    );
    assert_eq!(deferred.rain_region_counts(), eager.rain_region_counts());
    assert_eq!(deferred.falling_drives, eager.falling_drives);
    assert_eq!(deferred.active_sites, eager.active_sites);
    assert_eq!(deferred.queued_sites, eager.queued_sites);
    assert_eq!(deferred.discharged, eager.discharged);
    assert_eq!(deferred.total_generated, eager.total_generated);
    assert_eq!(deferred.avalanche_moves, eager.avalanche_moves);
    assert_eq!(deferred.avalanche_peak_moves, eager.avalanche_peak_moves);
    assert_eq!(deferred.recent_avalanches, eager.recent_avalanches);

    deferred.sync_for_render();
    assert_eq!(deferred.surface.grain_count, eager.surface.grain_count);
    assert_eq!(deferred.surface.grid, eager.surface.grid);
}

#[test]
fn zero_outside_control_reproduces_the_low_edge_wedge_pressure() {
    let mut engine = OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::ZeroOutside);
    let width = engine.lattice_size();
    for drive in 0..6_000usize {
        direct_drive_and_relax(&mut engine, drive % width);
    }

    let center = engine.columns[width / 2].len();
    let left = engine.columns[0].len();
    let right = engine.columns[width - 1].len();
    assert!(
        center > left + 10,
        "left edge no longer exhibits wedge pressure"
    );
    assert!(
        center > right + 10,
        "right edge no longer exhibits wedge pressure"
    );
    assert!(engine.discharged_count() > 0);
    assert_eq!(
        engine.generated_count(),
        engine
            .grain_count()
            .saturating_add(engine.discharged_count())
    );
}

#[test]
fn momentum_vessel_is_exact_oslo_below_the_local_relief_trigger() {
    let mut standard =
        OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::CanonicalWallOverflow);
    let mut momentum = OsloSandboxEngine::new_momentum_vessel(20, 10, TEST_SEED);
    let site = standard.lattice_size() / 2;

    for engine in [&mut standard, &mut momentum] {
        set_column_height(engine, site - 1, 5);
        set_column_height(engine, site, 5);
        set_column_height(engine, site + 1, 2);
        engine.critical_slopes[site] = 2;
    }

    assert_eq!(momentum.momentum_trigger_relief(), 4);
    assert!(standard.topple_site_if_unstable(site));
    assert!(momentum.topple_site_if_unstable(site));
    assert_eq!(momentum.columns, standard.columns);
    assert_eq!(momentum.critical_slopes, standard.critical_slopes);
    assert_eq!(momentum.threshold_rng_state, standard.threshold_rng_state);
    assert_eq!(momentum.relax_rng_state, standard.relax_rng_state);
    assert!(momentum.rolling_grains.is_empty());
    assert_eq!(momentum.momentum_seeds(), 0);
}

#[test]
fn ordinary_quiescent_driving_never_enters_momentum_and_matches_vessel_exactly() {
    let mut standard =
        OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::CanonicalWallOverflow);
    let mut momentum = OsloSandboxEngine::new_momentum_vessel(20, 10, TEST_SEED);
    let width = standard.lattice_size();

    for drive in 0..2_000usize {
        let site = drive % width;
        assert_eq!(
            direct_drive_and_relax(&mut momentum, site),
            direct_drive_and_relax(&mut standard, site)
        );
    }

    assert_eq!(momentum.momentum_seeds(), 0);
    assert!(momentum.rolling_grains.is_empty());
    assert_eq!(momentum.columns, standard.columns);
    assert_eq!(momentum.critical_slopes, standard.critical_slopes);
    assert_eq!(momentum.threshold_rng_state, standard.threshold_rng_state);
    assert_eq!(momentum.relax_rng_state, standard.relax_rng_state);
    assert_eq!(momentum.discharged_count(), standard.discharged_count());
}

#[test]
fn steep_local_oslo_failure_immediately_seeds_a_moving_grain() {
    let mut engine = OsloSandboxEngine::new_momentum_vessel(20, 10, TEST_SEED);
    let site = engine.lattice_size() / 2;
    set_column_height(&mut engine, site - 1, 6);
    set_column_height(&mut engine, site, 6);
    set_column_height(&mut engine, site + 1, 2);
    engine.critical_slopes[site] = 2;
    let mass_before = engine.grain_count();

    assert!(engine.topple_site_if_unstable(site));
    assert_eq!(engine.columns[site].len(), 5);
    assert_eq!(engine.columns[site + 1].len(), 2);
    assert_eq!(engine.rolling_grains.len(), 1);
    let rolling = engine.rolling_grains.front().copied().unwrap();
    assert_eq!(rolling.site, site + 1);
    assert_eq!(rolling.direction, ToppleDirection::Right);
    assert_eq!(rolling.flat_coast_remaining, MOMENTUM_FLAT_COAST_STEPS);
    assert_eq!(engine.momentum_seeds(), 1);
    assert_eq!(engine.grain_count(), mass_before);
}

#[test]
fn moving_grain_runs_downhill_coasts_briefly_and_then_settles() {
    let mut engine = OsloSandboxEngine::new_momentum_vessel(20, 10, TEST_SEED);
    for (site, height) in [4usize, 4, 3, 2, 2, 2, 3].into_iter().enumerate() {
        set_column_height(&mut engine, site, height);
    }
    let settled_before = engine.settled_count();
    engine.seed_rolling_grain(1, grain(), ToppleDirection::Right);

    assert!(engine.advance_rolling_grains());
    assert_eq!(engine.rolling_grains.front().unwrap().site, 2);
    assert!(engine.advance_rolling_grains());
    assert_eq!(engine.rolling_grains.front().unwrap().site, 3);
    assert!(engine.advance_rolling_grains());
    assert_eq!(engine.rolling_grains.front().unwrap().site, 4);
    assert!(engine.advance_rolling_grains());
    assert_eq!(engine.rolling_grains.front().unwrap().site, 5);
    assert!(engine.advance_rolling_grains());

    assert!(engine.rolling_grains.is_empty());
    assert_eq!(engine.columns[5].len(), 3);
    assert_eq!(engine.momentum_hops(), 4);
    assert_eq!(engine.momentum_settles(), 1);
    assert_eq!(engine.settled_count(), settled_before + 1);
}

#[test]
fn moving_grain_cannot_cross_a_visible_vessel_wall() {
    let mut engine = OsloSandboxEngine::new_momentum_vessel(20, 10, TEST_SEED);
    let (_, visible_end) = engine.visible_lattice_bounds();
    let edge = visible_end - 1;
    set_column_height(&mut engine, edge, 3);
    let mass_before = engine.grain_count();
    engine.seed_rolling_grain(edge, grain(), ToppleDirection::Right);

    assert!(engine.advance_rolling_grains());
    assert!(engine.rolling_grains.is_empty());
    assert_eq!(engine.columns[edge].len(), 4);
    assert_eq!(engine.discharged_count(), 0);
    assert_eq!(engine.grain_count(), mass_before + 1);
}

#[test]
fn horizontal_reconnection_can_seed_momentum_on_the_first_steep_seam_topple() {
    let mut engine = OsloSandboxEngine::new_momentum_vessel(20, 10, TEST_SEED);
    let canonical_width = engine.lattice_size();
    engine.resize(10, 10);
    let (visible_start, visible_end) = engine.visible_lattice_bounds();
    assert!(visible_start > 0);
    assert!(visible_end < canonical_width);

    let source = visible_start;
    let hidden_left = source - 1;
    set_column_height(&mut engine, source, 8);
    set_column_height(&mut engine, source + 1, 8);
    set_column_height(&mut engine, hidden_left, 0);
    engine.critical_slopes[source] = 2;
    engine.active_sites.clear();
    engine.queued_sites.fill(false);
    assert!(!engine.topple_site_if_unstable(source));

    let mass_before = engine.grain_count();
    engine.resize(20, 10);
    let mut guard = 0usize;
    while engine.momentum_seeds() == 0 {
        guard = guard.saturating_add(1);
        assert!(guard < canonical_width * 4, "reconnected seam never reached the active queue");
        let _ = engine.topple_one_active_site();
    }

    assert_eq!(engine.momentum_seeds(), 1);
    assert_eq!(engine.rolling_grains.len(), 1);
    assert_eq!(engine.rolling_grains.front().unwrap().site, hidden_left);
    assert_eq!(engine.grain_count(), mass_before);
}

#[test]
fn steep_wall_relaxation_creates_a_concurrent_mobile_front_and_conserves_mass() {
    let mut engine = OsloSandboxEngine::new_momentum_vessel(20, 10, TEST_SEED);
    let width = engine.lattice_size();
    for site in 0..(width / 2) {
        set_column_height(&mut engine, site, 12);
    }
    for site in (width / 2)..width {
        set_column_height(&mut engine, site, 2);
    }
    engine.critical_slopes.fill(OSLO_THRESHOLD_HIGH);
    let mass_before = engine.settled_count();
    engine.enqueue_neighborhood(width / 2 - 1);
    engine.enqueue_neighborhood(width / 2);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 100_000, "momentum wall relaxation failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && engine.rolling_grains.is_empty() {
            break;
        }
    }

    assert!(engine.momentum_seeds() > 0);
    assert!(engine.momentum_hops() > 0);
    assert!(engine.momentum_peak_active() >= 2);
    assert_eq!(engine.discharged_count(), 0);
    assert_eq!(engine.settled_count(), mass_before);
    assert_eq!(engine.grain_count(), mass_before);
    assert!(engine.avalanche_last_moves() > 0);
    assert!(engine.momentum_last_seeds() > 0);
    assert!(engine.momentum_last_hops() > 0);
}

#[test]
fn front_vessel_preserves_ordinary_oslo_and_never_fluidizes_normal_drive() {
    let mut standard =
        OsloSandboxEngine::new(20, 10, TEST_SEED, OsloBoundaryMode::CanonicalWallOverflow);
    let mut front = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let width = standard.lattice_size();

    for drive in 0..2_000usize {
        let site = drive % width;
        assert_eq!(
            direct_drive_and_relax(&mut front, site),
            direct_drive_and_relax(&mut standard, site)
        );
    }

    assert_eq!(front.momentum_seeds(), 0);
    assert_eq!(front.front_erosions(), 0);
    assert_eq!(front.front_support_recruits(), 0);
    assert!(front.rolling_grains.is_empty());
    assert_eq!(front.columns, standard.columns);
    assert_eq!(front.critical_slopes, standard.critical_slopes);
    assert_eq!(front.threshold_rng_state, standard.threshold_rng_state);
    assert_eq!(front.relax_rng_state, standard.relax_rng_state);
    assert_eq!(front.discharged_count(), standard.discharged_count());
}

#[test]
fn front_moving_layer_erodes_a_steep_static_bed_one_grain_per_site_per_tick() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    for (site, height) in [4usize, 5, 3, 2, 2].into_iter().enumerate() {
        set_column_height(&mut engine, site, height);
    }
    let settled_before = engine.settled_count();
    engine.seed_rolling_grain(1, grain(), ToppleDirection::Right);
    let mass_before = engine.grain_count();

    assert_eq!(engine.front_erosion_relief(), 2);
    assert!(engine.advance_rolling_grains());
    assert_eq!(engine.columns[1].len(), 4, "one static grain should be entrained");
    assert_eq!(engine.front_erosions(), 1);
    assert_eq!(engine.rolling_grains.len(), 2);
    assert!(engine.rolling_grains.iter().all(|rolling| rolling.site == 2));
    assert_eq!(engine.settled_count(), settled_before - 1);
    assert_eq!(engine.grain_count(), mass_before);
}

#[test]
fn front_loss_of_support_recruits_the_immediately_uphill_column() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    set_column_height(&mut engine, 1, 7);
    set_column_height(&mut engine, 2, 4);
    set_column_height(&mut engine, 3, 2);
    engine.seed_rolling_grain(3, grain(), ToppleDirection::Right);
    let mass_before = engine.grain_count();

    assert_eq!(engine.front_support_loss_relief(), 3);
    assert!(engine.front_recruit_support_after_loss(2, ToppleDirection::Right));
    assert_eq!(engine.columns[1].len(), 6);
    assert_eq!(engine.front_support_recruits(), 1);
    assert!(engine
        .rolling_grains
        .iter()
        .any(|rolling| rolling.site == 2 && rolling.direction == ToppleDirection::Right));
    assert_eq!(engine.grain_count(), mass_before);
}

#[test]
fn front_flat_runout_is_shorter_than_momentum_v1_but_thick_flow_can_coast_more() {
    let mut front = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    for site in 0..8 {
        set_column_height(&mut front, site, 3);
    }
    front.seed_rolling_grain(1, grain(), ToppleDirection::Right);
    assert_eq!(
        front.rolling_grains.front().unwrap().flat_coast_remaining,
        FRONT_BASE_FLAT_COAST_STEPS
    );

    let mut thick = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    for site in 0..8 {
        set_column_height(&mut thick, site, 3);
    }
    for _ in 0..7 {
        thick.seed_rolling_grain(1, grain(), ToppleDirection::Right);
    }
    let budget = thick.flat_coast_budget(1);
    assert!(budget > FRONT_BASE_FLAT_COAST_STEPS);
    assert!(budget <= FRONT_MAX_FLAT_COAST_STEPS);
}

#[test]
fn front_wall_failure_erodes_propagates_uphill_and_quiesces_with_mass_conserved() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let width = engine.lattice_size();
    for site in 0..(width / 2) {
        set_column_height(&mut engine, site, 12);
    }
    for site in (width / 2)..width {
        set_column_height(&mut engine, site, 2);
    }
    engine.critical_slopes.fill(OSLO_THRESHOLD_HIGH);
    let mass_before = engine.grain_count();
    engine.enqueue_neighborhood(width / 2 - 1);
    engine.enqueue_neighborhood(width / 2);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 200_000, "front wall relaxation failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && engine.rolling_grains.is_empty() {
            break;
        }
    }

    assert!(engine.momentum_seeds() > 0);
    assert!(engine.front_erosions() > 0, "moving layer never eroded the bed");
    assert!(
        engine.front_support_recruits() > 0,
        "wall failure never propagated uphill by support loss"
    );
    assert!(engine.momentum_peak_active() >= 3);
    assert_eq!(engine.discharged_count(), 0);
    assert_eq!(engine.grain_count(), mass_before);
    assert_eq!(engine.settled_count(), mass_before);
    assert!(engine.avalanche_last_moves() > 0);
    assert!(engine.front_last_erosions() > 0);
    assert!(engine.front_last_support_recruits() > 0);
}

#[test]
fn fluid_vessel_preserves_frozen_front_for_ordinary_oslo_drive() {
    let mut front = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut fluid = OsloSandboxEngine::new_fluid_vessel(20, 10, TEST_SEED);
    let width = front.lattice_size();

    for drive in 0..2_000usize {
        let site = drive % width;
        assert_eq!(
            direct_drive_and_relax(&mut fluid, site),
            direct_drive_and_relax(&mut front, site)
        );
    }

    assert_eq!(fluid.fluid_activations(), 0);
    assert_eq!(fluid.fluid_releases(), 0);
    assert_eq!(fluid.fluid_active_sites(), 0);
    assert_eq!(fluid.columns, front.columns);
    assert_eq!(fluid.critical_slopes, front.critical_slopes);
    assert_eq!(fluid.threshold_rng_state, front.threshold_rng_state);
    assert_eq!(fluid.relax_rng_state, front.relax_rng_state);
    assert_eq!(fluid.discharged_count(), front.discharged_count());
}

#[test]
fn fluidization_nucleates_on_first_severe_failure_and_spreads_as_a_region() {
    let mut engine = OsloSandboxEngine::new_fluid_vessel(20, 10, TEST_SEED);
    let width = engine.lattice_size();
    let source = width / 2 - 2;
    for (offset, height) in [14usize, 14, 10, 7, 4, 2].into_iter().enumerate() {
        set_column_height(&mut engine, source - 1 + offset, height);
    }
    engine.critical_slopes[source] = OSLO_THRESHOLD_HIGH;
    engine.enqueue_neighborhood(source);

    assert!(engine.topple_one_active_site());
    assert!(engine.fluid_activations() >= 2);
    assert!(engine.fluid_active_sites() >= 2);
    assert!(engine.rolling_count() >= 1);

    for _ in 0..8 {
        let _ = engine.advance_fluidization_field();
        let _ = engine.advance_rolling_grains();
        let _ = engine.topple_one_active_site();
    }

    assert!(engine.fluid_peak_active_sites() >= 3);
    assert!(engine.fluid_releases() > 0);
}

#[test]
fn fluid_wall_failure_releases_mass_collectively_then_reaches_quiescence() {
    let mut engine = OsloSandboxEngine::new_fluid_vessel(20, 10, TEST_SEED);
    let width = engine.lattice_size();
    for site in 0..(width / 2) {
        set_column_height(&mut engine, site, 14);
    }
    for site in (width / 2)..width {
        set_column_height(&mut engine, site, 2);
    }
    engine.critical_slopes.fill(OSLO_THRESHOLD_HIGH);
    let mass_before = engine.grain_count();
    engine.enqueue_neighborhood(width / 2 - 1);
    engine.enqueue_neighborhood(width / 2);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 300_000, "fluid wall relaxation failed to quiesce");
        let mut changed = engine.advance_fluidization_field();
        changed |= engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
            break;
        }
    }

    assert!(engine.fluid_activations() > 0);
    assert!(engine.fluid_releases() > 0);
    assert!(engine.fluid_peak_active_sites() >= 3);
    assert!(engine.momentum_peak_active() >= 3);
    assert_eq!(engine.discharged_count(), 0);
    assert_eq!(engine.grain_count(), mass_before);
    assert_eq!(engine.settled_count(), mass_before);
    assert_eq!(engine.fluid_active_sites(), 0);
}

#[test]
fn rainbow_fill_populates_exactly_eighty_percent_of_current_visible_window() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let categories = [CategoryId(11), CategoryId(22), CategoryId(33), CategoryId(44)];
    let (start, end) = engine.visible_lattice_bounds();
    let expected_height = engine.visible_height() * 4 / 5;
    let grains = engine.debug_fill_rainbow_80(&categories).unwrap();

    assert_eq!(grains, (end - start) * expected_height);
    for site in start..end {
        assert_eq!(engine.columns[site].len(), expected_height);
        assert_eq!(engine.columns[site].first().copied(), Some(categories[0]));
        assert_eq!(engine.columns[site].last().copied(), Some(categories[3]));
    }
    assert!(engine.columns[..start].iter().all(Vec::is_empty));
    assert!(engine.columns[end..].iter().all(Vec::is_empty));
}
