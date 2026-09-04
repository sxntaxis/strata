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
