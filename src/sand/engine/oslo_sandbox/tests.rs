use super::*;

const TEST_SEED: u64 = 0x51A7_A51A_0B10_5EED;

fn grain() -> CategoryId {
    CategoryId::new(1)
}

fn set_column_height(engine: &mut OsloSandboxEngine, site: usize, height: usize) {
    engine.columns[site] = vec![grain(); height];
    engine.column_visual_y[site] = vec![None; height];
    if engine.flowviz_conservative || engine.flowviz_unit {
        engine.flowviz_shadow_columns[site] = vec![grain(); height];
    }
    if engine.flowviz_unit {
        engine.flowviz_unit_custody[site] = vec![None; height];
    }
}

fn direct_drive_and_relax(engine: &mut OsloSandboxEngine, site: usize) -> usize {
    engine.push_settled_grain_at_visual_y(site, grain(), None);
    engine.mirror_flowviz_settled_push(site, grain());
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);
    let mut moves = 0usize;
    while engine.topple_one_active_site() {
        moves = moves.saturating_add(1);
    }
    moves
}

fn direct_front_drive_and_quiesce(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(), None);
    engine.mirror_flowviz_settled_push(site, grain());
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 250_000, "front direct drive failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
            break;
        }
    }
}

fn category_counts<I>(categories: I) -> std::collections::HashMap<CategoryId, usize>
where
    I: IntoIterator<Item = CategoryId>,
{
    let mut counts = std::collections::HashMap::new();
    for category_id in categories {
        *counts.entry(category_id).or_insert(0) += 1;
    }
    counts
}

fn physical_category_counts(
    engine: &OsloSandboxEngine,
) -> std::collections::HashMap<CategoryId, usize> {
    category_counts(
        engine
            .columns
            .iter()
            .flatten()
            .copied()
            .chain(engine.rolling_grains.iter().map(|grain| grain.category_id)),
    )
}

fn conservative_visual_category_counts(
    engine: &OsloSandboxEngine,
) -> std::collections::HashMap<CategoryId, usize> {
    category_counts(
        engine
            .flowviz_shadow_columns
            .iter()
            .flatten()
            .copied()
            .chain(
                engine
                    .flowviz_parcels
                    .iter()
                    .flat_map(|parcel| std::iter::repeat_n(parcel.category_id, parcel.mass)),
            ),
    )
}

fn unit_visual_category_counts(
    engine: &OsloSandboxEngine,
) -> std::collections::HashMap<CategoryId, usize> {
    category_counts(
        engine
            .flowviz_shadow_columns
            .iter()
            .flatten()
            .copied()
            .chain(
                engine
                    .flowviz_unit_carriers
                    .values()
                    .map(|carrier| carrier.category_id),
            ),
    )
}

fn unit_authoritative_category_counts(
    engine: &OsloSandboxEngine,
) -> std::collections::HashMap<CategoryId, usize> {
    category_counts(
        engine
            .columns
            .iter()
            .flatten()
            .copied()
            .chain(engine.rolling_grains.iter().map(|grain| grain.category_id))
            .chain(
                engine
                    .flowviz_unit_carriers
                    .values()
                    .filter(|carrier| {
                        carrier.physical == flowviz_unit::UnitPhysicalState::Discharged
                    })
                    .map(|carrier| carrier.category_id),
            ),
    )
}

fn per_site_category_counts(
    columns: &[Vec<CategoryId>],
) -> Vec<std::collections::HashMap<CategoryId, usize>> {
    columns
        .iter()
        .map(|column| category_counts(column.iter().copied()))
        .collect()
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
        assert!(
            guard < canonical_width * 4,
            "reconnected seam never reached the active queue"
        );
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
        assert!(
            guard < 100_000,
            "momentum wall relaxation failed to quiesce"
        );
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
    assert_eq!(
        engine.columns[1].len(),
        4,
        "one static grain should be entrained"
    );
    assert_eq!(engine.front_erosions(), 1);
    assert_eq!(engine.rolling_grains.len(), 2);
    assert!(
        engine
            .rolling_grains
            .iter()
            .all(|rolling| rolling.site == 2)
    );
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
    assert!(
        engine
            .rolling_grains
            .iter()
            .any(|rolling| rolling.site == 2 && rolling.direction == ToppleDirection::Right)
    );
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
    assert!(
        engine.front_erosions() > 0,
        "moving layer never eroded the bed"
    );
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
    let categories = [
        CategoryId(11),
        CategoryId(22),
        CategoryId(33),
        CategoryId(44),
    ];
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

#[test]
fn rainbow_fillhalf_populates_centered_half_of_current_visible_width() {
    let mut engine = OsloSandboxEngine::new_front_vessel(21, 10, TEST_SEED);
    let categories = [CategoryId(11), CategoryId(22), CategoryId(33), CategoryId(44)];
    let (visible_start, visible_end) = engine.visible_lattice_bounds();
    let visible_width = visible_end - visible_start;
    let expected_width = (visible_width / 2).max(1);
    let expected_start = visible_start + (visible_width - expected_width) / 2;
    let expected_end = expected_start + expected_width;
    let expected_height = engine.visible_height() * 4 / 5;

    let grains = engine
        .debug_fill_rainbow_80_centered_half(&categories)
        .unwrap();

    assert_eq!(grains, expected_width * expected_height);
    for site in visible_start..visible_end {
        if (expected_start..expected_end).contains(&site) {
            assert_eq!(engine.columns[site].len(), expected_height);
            assert_eq!(engine.columns[site].first().copied(), Some(categories[0]));
            assert_eq!(engine.columns[site].last().copied(), Some(categories[3]));
        } else {
            assert!(engine.columns[site].is_empty());
        }
    }
}

#[test]
fn severe_front_release_preserves_source_elevation_then_descends_one_dot_per_visual_step() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let site = engine.lattice_size() / 2;
    set_column_height(&mut engine, site - 1, 8);
    set_column_height(&mut engine, site, 8);
    set_column_height(&mut engine, site + 1, 2);
    let _ = engine.pop_settled_grain(site).unwrap();
    engine.push_settled_grain_at_visual_y(site, CategoryId(77), None);
    engine.critical_slopes[site] = OSLO_THRESHOLD_HIGH;
    let source_y = engine.top_grain_y(site).unwrap();

    assert!(engine.topple_site_if_unstable(site));
    let rolling = engine.rolling_grains.front().copied().unwrap();
    assert_eq!(rolling.site, site + 1);
    assert_eq!(rolling.category_id, CategoryId(77));
    assert_eq!(rolling.visual_y, source_y);
    assert!(engine.rolling_visual_motion_active());

    let first_y = rolling.visual_y;
    assert!(engine.advance_rolling_visual_motion());
    assert_eq!(engine.rolling_grains.front().unwrap().visual_y, first_y + 1);

    let mut guard = 0usize;
    while engine.rolling_visual_motion_active() {
        guard += 1;
        assert!(guard < 100, "rolling visual transport did not converge");
        assert!(engine.advance_rolling_visual_motion());
    }
    let target_y = engine.surface.grid_height_dots - engine.columns[site + 1].len() - 1;
    assert_eq!(engine.rolling_grains.front().unwrap().visual_y, target_y);
    assert_eq!(engine.grain_count(), 18);
}

#[test]
fn rolling_physics_hop_changes_one_lattice_site_without_vertical_teleport() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    for (site, height) in [6usize, 4, 2, 1].into_iter().enumerate() {
        set_column_height(&mut engine, site, height);
    }
    engine.seed_rolling_grain(1, CategoryId(88), ToppleDirection::Right);
    let start_y = engine.rolling_grains.front().unwrap().visual_y;

    assert!(engine.advance_rolling_grains());
    let rolling = engine.rolling_grains.front().unwrap();
    assert_eq!(rolling.site, 2);
    assert_eq!(rolling.visual_y, start_y);
    assert_eq!(rolling.category_id, CategoryId(88));
    assert!(engine.rolling_visual_motion_active());
}

#[test]
fn rolling_visual_interpolation_is_semantically_inert_for_front_relaxation() {
    fn run(mut engine: OsloSandboxEngine, interpolate: bool) -> OsloSandboxEngine {
        let width = engine.lattice_size();
        for site in 0..(width / 2) {
            set_column_height(&mut engine, site, 12);
        }
        for site in (width / 2)..width {
            set_column_height(&mut engine, site, 2);
        }
        engine.critical_slopes.fill(OSLO_THRESHOLD_HIGH);
        engine.enqueue_neighborhood(width / 2 - 1);
        engine.enqueue_neighborhood(width / 2);

        let mut guard = 0usize;
        loop {
            guard = guard.saturating_add(1);
            assert!(guard < 200_000, "front relaxation did not quiesce");
            let mut changed = engine.advance_rolling_grains();
            changed |= engine.topple_one_active_site();
            // SEDIMENT-011 deliberately lets presentation move concurrently
            // instead of draining every colored dot before the next physics
            // event. Neither choice may influence the authoritative columns.
            if interpolate {
                let _ = engine.advance_rolling_visual_motion();
            }
            if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
                break;
            }
        }
        engine
    }

    let direct = run(
        OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED),
        false,
    );
    let interpolated = run(OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED), true);

    assert_eq!(interpolated.columns, direct.columns);
    assert_eq!(interpolated.critical_slopes, direct.critical_slopes);
    assert_eq!(interpolated.threshold_rng_state, direct.threshold_rng_state);
    assert_eq!(interpolated.relax_rng_state, direct.relax_rng_state);
    assert_eq!(interpolated.discharged_count(), direct.discharged_count());
    assert_eq!(interpolated.momentum_hops(), direct.momentum_hops());
    assert_eq!(interpolated.front_erosions(), direct.front_erosions());
    assert_eq!(
        interpolated.front_support_recruits(),
        direct.front_support_recruits()
    );
}

#[test]
fn visual_transit_never_globally_blocks_front_physics() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let site = engine.lattice_size() / 2;
    set_column_height(&mut engine, site - 1, 10);
    set_column_height(&mut engine, site, 10);
    set_column_height(&mut engine, site + 1, 2);
    engine.critical_slopes[site] = OSLO_THRESHOLD_HIGH;

    assert!(engine.topple_site_if_unstable(site));
    assert!(engine.rolling_visual_motion_active());
    let before_hops = engine.momentum_hops();

    // A single full frame must advance authoritative rolling physics even while
    // the released CategoryId is still many visual rows above its target.
    let _ = engine.advance_testing_flow_frame();
    assert!(engine.momentum_hops() > before_hops);
}

#[test]
fn settled_category_transport_moves_in_parallel_without_adding_mass() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let (start, _) = engine.visible_lattice_bounds();
    let top = engine.visible_vertical_bounds().0;
    engine.push_settled_grain_at_visual_y(start, CategoryId(71), Some(top));
    engine.push_settled_grain_at_visual_y(start + 1, CategoryId(72), Some(top));
    let mass_before = engine.grain_count();
    let before_a = engine.column_visual_y[start][0].unwrap();
    let before_b = engine.column_visual_y[start + 1][0].unwrap();

    assert!(engine.advance_rolling_visual_motion());

    assert_eq!(engine.grain_count(), mass_before);
    assert_eq!(engine.column_visual_y[start][0], Some(before_a + 1));
    assert_eq!(engine.column_visual_y[start + 1][0], Some(before_b + 1));
}

#[test]
fn moving_a_settled_category_again_uses_its_current_visible_elevation() {
    let mut engine = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let site = engine.visible_lattice_bounds().0;
    let top = engine.visible_vertical_bounds().0;
    engine.push_settled_grain_at_visual_y(site, CategoryId(73), Some(top));
    assert!(engine.advance_rolling_visual_motion());
    let current_y = engine.column_visual_y[site][0].unwrap();

    let (category_id, visual_y) = engine.pop_settled_grain(site).unwrap();

    assert_eq!(category_id, CategoryId(73));
    assert_eq!(visual_y, current_y);
    assert_eq!(engine.grain_count(), 0);
}

#[test]
fn falling_rain_does_not_turn_a_latent_fluid_field_into_visible_flow() {
    let mut engine = OsloSandboxEngine::new_fluid_vessel(20, 10, TEST_SEED);
    let site = engine.visible_lattice_bounds().0;
    engine.fluidity[site] = 1;
    engine.spawn(CategoryId(74));

    assert!(engine.explicit_flow_active());
    assert!(engine.latent_flow_active());
    assert!(!engine.visible_flow_active());
}

#[test]
fn flowviz_front_preserves_frozen_front_physics_on_ordinary_quiescent_drive() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut flowviz = OsloSandboxEngine::new_front_flowviz_vessel(20, 10, TEST_SEED);
    let width = frozen.lattice_size();

    for drive in 0..2_000usize {
        let site = (drive.wrapping_mul(37).wrapping_add(11)) % width;
        let frozen_moves = direct_drive_and_relax(&mut frozen, site);
        let flowviz_moves = direct_drive_and_relax(&mut flowviz, site);
        assert_eq!(flowviz_moves, frozen_moves);
    }

    assert_eq!(flowviz.columns, frozen.columns);
    assert_eq!(flowviz.critical_slopes, frozen.critical_slopes);
    assert_eq!(flowviz.threshold_rng_state, frozen.threshold_rng_state);
    assert_eq!(flowviz.relax_rng_state, frozen.relax_rng_state);
    assert_eq!(flowviz.discharged_count(), frozen.discharged_count());
    assert_eq!(flowviz.front_erosions(), frozen.front_erosions());
    assert_eq!(
        flowviz.front_support_recruits(),
        frozen.front_support_recruits()
    );
}

#[test]
fn flowviz_transfer_creates_visual_flux_without_changing_physical_mass() {
    let mut engine = OsloSandboxEngine::new_front_flowviz_vessel(20, 10, TEST_SEED);
    let site = engine.lattice_size() / 2;
    set_column_height(&mut engine, site, 8);
    set_column_height(&mut engine, site + 1, 2);
    let mass_before = engine.grain_count();
    let source_y = engine.top_grain_y(site).unwrap();

    engine.record_flowviz_transfer(site, site + 1, CategoryId(91), source_y);

    assert_eq!(engine.grain_count(), mass_before);
    assert_eq!(engine.flowviz_tracer_count(), 1);
    assert_eq!(engine.flowviz_active_flux_edges(), 1);
    assert_eq!(engine.flowviz_spawned(), 1);
}

#[test]
fn flowviz_tracers_move_as_bounded_local_samples_not_long_destination_paths() {
    let mut engine = OsloSandboxEngine::new_front_flowviz_vessel(20, 10, TEST_SEED);
    let site = engine.lattice_size() / 2;
    set_column_height(&mut engine, site, 10);
    set_column_height(&mut engine, site + 1, 2);
    let source_y = engine.top_grain_y(site).unwrap();
    engine.record_flowviz_transfer(site, site + 1, CategoryId(92), source_y);

    let before = *engine.flowviz_tracers.front().unwrap();
    assert!(engine.advance_flowviz_tracers());
    let after = *engine.flowviz_tracers.front().unwrap();

    assert!((after.x - before.x).abs() <= 1.0);
    assert!((after.y - before.y).abs() <= 1.0);
    assert_eq!(after.category_id, before.category_id);
}

#[test]
fn flowviz_wall_failure_builds_a_concurrent_tracer_cloud_and_conserves_physics() {
    fn prepare(mut engine: OsloSandboxEngine) -> OsloSandboxEngine {
        let width = engine.lattice_size();
        for site in 0..(width / 2) {
            set_column_height(&mut engine, site, 12);
        }
        for site in (width / 2)..width {
            set_column_height(&mut engine, site, 2);
        }
        engine.critical_slopes.fill(OSLO_THRESHOLD_HIGH);
        engine.enqueue_neighborhood(width / 2 - 1);
        engine.enqueue_neighborhood(width / 2);
        engine
    }

    fn relax(mut engine: OsloSandboxEngine) -> OsloSandboxEngine {
        let mut guard = 0usize;
        loop {
            guard = guard.saturating_add(1);
            assert!(guard < 250_000, "flowviz wall relaxation did not quiesce");
            let mut changed = engine.advance_rolling_grains();
            changed |= engine.topple_one_active_site();
            if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
                break;
            }
        }
        engine
    }

    let frozen = relax(prepare(OsloSandboxEngine::new_front_vessel(
        20, 10, TEST_SEED,
    )));
    let mut flowviz = relax(prepare(OsloSandboxEngine::new_front_flowviz_vessel(
        20, 10, TEST_SEED,
    )));

    assert_eq!(flowviz.columns, frozen.columns);
    assert_eq!(flowviz.critical_slopes, frozen.critical_slopes);
    assert_eq!(flowviz.threshold_rng_state, frozen.threshold_rng_state);
    assert_eq!(flowviz.relax_rng_state, frozen.relax_rng_state);
    assert_eq!(flowviz.discharged_count(), frozen.discharged_count());
    assert!(flowviz.flowviz_spawned() > 16);
    assert!(flowviz.flowviz_peak_tracers() > 8);

    let mut visual_guard = 0usize;
    while flowviz.rolling_visual_motion_active() {
        visual_guard = visual_guard.saturating_add(1);
        assert!(visual_guard < 2_000, "flowviz tracer cloud failed to drain");
        let _ = flowviz.advance_flowviz_tracers();
    }
    assert_eq!(flowviz.grain_count(), frozen.grain_count());
}

#[test]
fn conservative_flowviz_preserves_frozen_front_physics_on_ordinary_quiescent_drive() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut parcels = OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED);
    let width = frozen.lattice_size();

    for drive in 0..2_000usize {
        let site = (drive.wrapping_mul(37).wrapping_add(11)) % width;
        let frozen_moves = direct_drive_and_relax(&mut frozen, site);
        let parcel_moves = direct_drive_and_relax(&mut parcels, site);
        assert_eq!(parcel_moves, frozen_moves);
    }

    assert_eq!(parcels.columns, frozen.columns);
    assert_eq!(parcels.critical_slopes, frozen.critical_slopes);
    assert_eq!(parcels.threshold_rng_state, frozen.threshold_rng_state);
    assert_eq!(parcels.relax_rng_state, frozen.relax_rng_state);
    assert_eq!(parcels.discharged_count(), frozen.discharged_count());
    assert_eq!(parcels.front_erosions(), frozen.front_erosions());
    assert_eq!(
        parcels.front_support_recruits(),
        frozen.front_support_recruits()
    );
    assert_eq!(parcels.flowviz_shadow_misses(), 0);
    assert!(parcels.flowviz_visual_mass_matches_physical_mobile_system());
    assert_eq!(
        conservative_visual_category_counts(&parcels),
        physical_category_counts(&parcels)
    );
}

#[test]
fn conservative_flowviz_fungible_discharge_consumes_nearest_shadow_surplus() {
    let mut engine = OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let surplus_site = source.saturating_add(2);

    for column in &mut engine.columns {
        column.clear();
    }
    for column in &mut engine.flowviz_shadow_columns {
        column.clear();
    }
    engine.flowviz_parcels.clear();
    engine.flowviz_mobile_mass = 0;
    engine.flowviz_shadow_misses = 0;

    // Model the state *after* an anonymous same-category physical unit has
    // discharged at `source`: no exact-source visual unit remains, but the
    // fungible shadow still owns one same-category surplus elsewhere.
    engine.flowviz_shadow_columns[surplus_site].push(grain());

    engine.mirror_flowviz_settled_discharge(source, grain());

    assert_eq!(engine.flowviz_shadow_mass(), 0);
    assert_eq!(engine.flowviz_parcel_mass(), 0);
    assert_eq!(engine.flowviz_shadow_misses(), 0);
    assert_eq!(engine.flowviz_reused_deposits(), 1);
    assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());
    assert_eq!(
        conservative_visual_category_counts(&engine),
        physical_category_counts(&engine)
    );
}

#[test]
fn conservative_flowviz_offscreen_discharge_withdrawal_is_still_custody_success() {
    let mut engine = OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED);
    let source = 0usize;
    let viewport_height = engine.surface.grid_height_dots;

    for column in &mut engine.columns {
        column.clear();
    }
    for column in &mut engine.flowviz_shadow_columns {
        column.clear();
    }
    engine.flowviz_parcels.clear();
    engine.flowviz_mobile_mass = 0;
    engine.flowviz_shadow_misses = 0;
    engine.flowviz_reused_deposits = 0;

    // Model the state immediately after a physical discharge from a column
    // whose visual stack extends above the current viewport. Conservative
    // custody still owns exactly one extra same-category shadow unit. Its
    // depth has no representable positive y, but withdrawal must still report
    // success rather than mutating the shadow and then returning `None`.
    engine.columns[source] = vec![grain(); viewport_height.saturating_add(1)];
    engine.flowviz_shadow_columns[source] = vec![grain(); viewport_height.saturating_add(2)];

    engine.mirror_flowviz_settled_discharge(source, grain());

    assert_eq!(
        engine.flowviz_shadow_columns[source].len(),
        engine.columns[source].len()
    );
    assert_eq!(engine.flowviz_parcel_mass(), 0);
    assert_eq!(engine.flowviz_shadow_misses(), 0);
    assert_eq!(engine.flowviz_reused_deposits(), 0);
    assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());
    assert_eq!(
        conservative_visual_category_counts(&engine),
        physical_category_counts(&engine)
    );
}

#[test]
fn conservative_flowviz_offscreen_mobile_entry_creates_real_parcel_custody() {
    let mut engine = OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED);
    let source = 1usize;
    let destination = source + 1;
    let viewport_height = engine.surface.grid_height_dots;

    for column in &mut engine.columns {
        column.clear();
    }
    for column in &mut engine.flowviz_shadow_columns {
        column.clear();
    }
    engine.flowviz_parcels.clear();
    engine.flowviz_mobile_mass = 0;
    engine.flowviz_shadow_misses = 0;
    engine.flowviz_reused_deposits = 0;
    engine.flowviz_visual_withdrawals = 0;

    // Model state immediately after physics popped the top source grain but
    // before it is inserted into rolling custody. The shadow unit is above the
    // viewport; that affects only its projected source y, not whether custody
    // exists and must become parcel mass.
    engine.columns[source] = vec![grain(); viewport_height.saturating_add(1)];
    engine.flowviz_shadow_columns[source] = vec![grain(); viewport_height.saturating_add(2)];

    engine.record_flowviz_mobile_entry(source, destination, grain(), 0);
    engine.seed_rolling_grain_at_y(destination, grain(), ToppleDirection::Right, 0);

    assert_eq!(engine.flowviz_parcel_mass(), 1);
    assert_eq!(engine.flowviz_visual_withdrawals(), 1);
    assert_eq!(engine.flowviz_reused_deposits(), 0);
    assert_eq!(engine.flowviz_shadow_misses(), 0);
    assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());
    assert_eq!(
        conservative_visual_category_counts(&engine),
        physical_category_counts(&engine)
    );
}

#[test]
fn conservative_flowviz_offscreen_settled_transfer_mirrors_custody() {
    let mut engine = OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED);
    let source = 1usize;
    let destination = source + 1;
    let viewport_height = engine.surface.grid_height_dots;

    for column in &mut engine.columns {
        column.clear();
    }
    for column in &mut engine.flowviz_shadow_columns {
        column.clear();
    }
    engine.flowviz_parcels.clear();
    engine.flowviz_mobile_mass = 0;

    // Model state after the authoritative ordinary settled transfer has already
    // moved one unit from source to destination. The corresponding shadow source
    // unit lives above the viewport and must still be recognized as present.
    engine.columns[source] = vec![grain(); viewport_height.saturating_add(1)];
    engine.columns[destination].push(grain());
    engine.flowviz_shadow_columns[source] = vec![grain(); viewport_height.saturating_add(2)];

    engine.mirror_flowviz_settled_transfer(source, destination, grain());

    assert_eq!(
        engine.flowviz_shadow_columns[source].len(),
        engine.columns[source].len()
    );
    assert_eq!(
        engine.flowviz_shadow_columns[destination],
        engine.columns[destination]
    );
    assert_eq!(engine.flowviz_total_transport_due_status(), 0);
    assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());
    assert_eq!(
        conservative_visual_category_counts(&engine),
        physical_category_counts(&engine)
    );
}

#[test]
fn conservative_flowviz_delays_visible_deposit_until_parcel_arrival() {
    let mut engine = OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let destination = source + 1;
    set_column_height(&mut engine, source, 3);
    set_column_height(&mut engine, destination, 0);
    engine.reset_conservative_flowviz_from_physics();

    let (category_id, visual_y) = engine.pop_settled_grain(source).unwrap();
    engine.record_flowviz_mobile_entry(source, destination, category_id, visual_y);
    engine.seed_rolling_grain_at_y(destination, category_id, ToppleDirection::Right, visual_y);
    assert_eq!(engine.flowviz_shadow_columns[destination].len(), 0);
    assert_eq!(engine.flowviz_parcel_mass(), 1);
    assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());

    let rolling = engine.rolling_grains.pop_front().unwrap();
    engine.push_settled_grain_at_visual_y(destination, rolling.category_id, Some(rolling.visual_y));
    engine.record_flowviz_settlement(destination, rolling.category_id);

    // Physics already owns the destination grain, but the visual surface does
    // not gain it until the conservative parcel reaches a real deposit credit.
    assert_eq!(engine.columns[destination].len(), 1);
    assert_eq!(engine.flowviz_shadow_columns[destination].len(), 0);
    assert_eq!(engine.flowviz_total_deposit_due(), 1);
    assert_eq!(engine.flowviz_parcel_mass(), 1);
    assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());

    let mut guard = 0usize;
    while engine.flowviz_parcel_mass() > 0 {
        guard = guard.saturating_add(1);
        assert!(guard < 2_000, "conservative parcel failed to deposit");
        let _ = engine.advance_flowviz_tracers();
    }

    assert_eq!(engine.flowviz_shadow_columns[destination].len(), 1);
    assert_eq!(engine.flowviz_total_deposit_due(), 0);
    assert_eq!(engine.flowviz_visual_deposits(), 1);
    assert_eq!(engine.flowviz_shadow_misses(), 0);
    assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());
    assert_eq!(
        conservative_visual_category_counts(&engine),
        physical_category_counts(&engine)
    );
}

#[test]
fn conservative_flowviz_reuses_pending_deposit_when_mass_reenters_motion() {
    let mut engine = OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let middle = source + 1;
    let destination = source + 2;
    set_column_height(&mut engine, source, 3);
    set_column_height(&mut engine, middle, 0);
    set_column_height(&mut engine, destination, 0);
    engine.reset_conservative_flowviz_from_physics();

    let (category_id, visual_y) = engine.pop_settled_grain(source).unwrap();
    engine.record_flowviz_mobile_entry(source, middle, category_id, visual_y);
    engine.seed_rolling_grain_at_y(middle, category_id, ToppleDirection::Right, visual_y);
    let rolling = engine.rolling_grains.pop_front().unwrap();
    engine.push_settled_grain_at_visual_y(middle, category_id, Some(rolling.visual_y));
    engine.record_flowviz_settlement(middle, category_id);

    let (category_id, visual_y) = engine.pop_settled_grain(middle).unwrap();
    engine.record_flowviz_mobile_entry(middle, destination, category_id, visual_y);
    engine.seed_rolling_grain_at_y(destination, category_id, ToppleDirection::Right, visual_y);

    assert_eq!(engine.flowviz_parcel_mass(), 1);
    assert_eq!(engine.flowviz_total_deposit_due(), 0);
    assert_eq!(engine.flowviz_reused_deposits(), 1);
    assert_eq!(engine.flowviz_visual_withdrawals(), 1);
    assert_eq!(engine.flowviz_shadow_misses(), 0);
    assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());
    assert_eq!(
        conservative_visual_category_counts(&engine),
        physical_category_counts(&engine)
    );
}

#[test]
fn conservative_flowviz_fungible_reentry_drains_through_real_adjacent_transport() {
    let mut engine = OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let middle = source + 1;
    let destination = source + 2;
    set_column_height(&mut engine, source, 3);
    set_column_height(&mut engine, middle, 0);
    set_column_height(&mut engine, destination, 0);
    engine.reset_conservative_flowviz_from_physics();

    let (category_id, visual_y) = engine.pop_settled_grain(source).unwrap();
    engine.record_flowviz_mobile_entry(source, middle, category_id, visual_y);
    engine.seed_rolling_grain_at_y(middle, category_id, ToppleDirection::Right, visual_y);

    let rolling = engine.rolling_grains.pop_front().unwrap();
    engine.push_settled_grain_at_visual_y(middle, category_id, Some(rolling.visual_y));
    engine.record_flowviz_settlement(middle, category_id);

    let (category_id, visual_y) = engine.pop_settled_grain(middle).unwrap();
    engine.record_flowviz_mobile_entry(middle, destination, category_id, visual_y);
    engine.seed_rolling_grain_at_y(destination, category_id, ToppleDirection::Right, visual_y);

    let rolling = engine.rolling_grains.pop_front().unwrap();
    engine.push_settled_grain_at_visual_y(destination, rolling.category_id, Some(rolling.visual_y));
    engine.record_flowviz_settlement(destination, rolling.category_id);

    assert_eq!(engine.flowviz_shadow_misses(), 0);
    assert_eq!(engine.flowviz_parcel_mass(), 1);
    assert_eq!(engine.flowviz_total_transport_due_status(), 2);
    assert_eq!(engine.flowviz_total_deposit_due(), 1);

    let mut guard = 0usize;
    while engine.flowviz_parcel_mass() > 0 {
        guard = guard.saturating_add(1);
        assert!(guard < 2_000, "fungible two-edge parcel failed to drain");
        let _ = engine.advance_flowviz_tracers();
        assert!(engine.flowviz_visual_mass_matches_physical_mobile_system());
        assert_eq!(
            conservative_visual_category_counts(&engine),
            physical_category_counts(&engine)
        );
    }

    assert_eq!(engine.flowviz_total_transport_due_status(), 0);
    assert_eq!(engine.flowviz_total_deposit_due(), 0);
    assert_eq!(engine.flowviz_shadow_misses(), 0);
    assert_eq!(
        per_site_category_counts(&engine.flowviz_shadow_columns),
        per_site_category_counts(&engine.columns)
    );
}

#[test]
fn conservative_flowviz_wall_failure_conserves_visual_mass_and_drains_to_real_credits() {
    fn prepare(mut engine: OsloSandboxEngine) -> OsloSandboxEngine {
        let width = engine.lattice_size();
        for site in 0..(width / 2) {
            set_column_height(&mut engine, site, 12);
        }
        for site in (width / 2)..width {
            set_column_height(&mut engine, site, 2);
        }
        engine.reset_conservative_flowviz_from_physics();
        engine.critical_slopes.fill(OSLO_THRESHOLD_HIGH);
        engine.enqueue_neighborhood(width / 2 - 1);
        engine.enqueue_neighborhood(width / 2);
        engine
    }

    fn relax(mut engine: OsloSandboxEngine) -> OsloSandboxEngine {
        let mut guard = 0usize;
        loop {
            guard = guard.saturating_add(1);
            assert!(guard < 250_000, "parcel wall relaxation did not quiesce");
            let mut changed = engine.advance_rolling_grains();
            changed |= engine.topple_one_active_site();
            if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
                break;
            }
        }
        engine
    }

    let frozen = relax(prepare(OsloSandboxEngine::new_front_vessel(
        20, 10, TEST_SEED,
    )));
    let mut parcels = relax(prepare(
        OsloSandboxEngine::new_front_conservative_flowviz_vessel(20, 10, TEST_SEED),
    ));

    assert_eq!(parcels.columns, frozen.columns);
    assert_eq!(parcels.critical_slopes, frozen.critical_slopes);
    assert_eq!(parcels.threshold_rng_state, frozen.threshold_rng_state);
    assert_eq!(parcels.relax_rng_state, frozen.relax_rng_state);
    assert_eq!(parcels.discharged_count(), frozen.discharged_count());
    assert!(parcels.flowviz_parcel_mass() > 8);
    assert!(parcels.flowviz_peak_mobile_mass() > 8);
    assert_eq!(parcels.flowviz_shadow_misses(), 0);
    assert!(parcels.flowviz_visual_mass_matches_physical_mobile_system());

    let mut visual_guard = 0usize;
    while parcels.flowviz_parcel_mass() > 0 {
        visual_guard = visual_guard.saturating_add(1);
        assert!(
            visual_guard < 20_000,
            "conservative parcel cloud failed to drain"
        );
        let _ = parcels.advance_flowviz_tracers();
        assert!(parcels.flowviz_visual_mass_matches_physical_mobile_system());
        assert_eq!(
            conservative_visual_category_counts(&parcels),
            physical_category_counts(&parcels)
        );
    }

    assert_eq!(parcels.flowviz_total_deposit_due(), 0);
    assert_eq!(parcels.flowviz_total_transport_due_status(), 0);
    assert_eq!(parcels.flowviz_shadow_mass(), frozen.settled_count());
    assert_eq!(parcels.flowviz_shadow_misses(), 0);
    assert!(parcels.flowviz_visual_mass_matches_physical_mobile_system());
    assert_eq!(
        conservative_visual_category_counts(&parcels),
        physical_category_counts(&parcels)
    );
    assert_eq!(
        per_site_category_counts(&parcels.flowviz_shadow_columns),
        per_site_category_counts(&parcels.columns)
    );
}

#[test]
fn unit_flowviz_preserves_frozen_front_physics_and_exact_stack_on_ordinary_drive() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut grains = OsloSandboxEngine::new_front_unit_flowviz_vessel(20, 10, TEST_SEED);
    let width = frozen.lattice_size();

    for drive in 0..2_000usize {
        let site = (drive.wrapping_mul(37).wrapping_add(11)) % width;
        direct_front_drive_and_quiesce(&mut frozen, site);
        direct_front_drive_and_quiesce(&mut grains, site);

        // This gate is intentionally stronger than the earlier direct-topple
        // helper: the frozen front and unit presentation model must first reach
        // the same *authoritative physical quiescence*, including any rolling
        // grains spawned by a severe local failure, before presentation alone
        // is asked to drain its already-observed history.
        assert!(!frozen.explicit_flow_active());
        assert!(!grains.explicit_flow_active());
        assert_eq!(grains.columns, frozen.columns);
        assert_eq!(grains.critical_slopes, frozen.critical_slopes);
        assert_eq!(grains.threshold_rng_state, frozen.threshold_rng_state);
        assert_eq!(grains.relax_rng_state, frozen.relax_rng_state);
        assert_eq!(grains.discharged_count(), frozen.discharged_count());
        assert_eq!(grains.front_erosions(), frozen.front_erosions());
        assert_eq!(
            grains.front_support_recruits(),
            frozen.front_support_recruits()
        );
        assert_eq!(grains.avalanche_moves, frozen.avalanche_moves);
        assert!(grains.flowviz_unit_mass_matches_physics());
        assert_eq!(
            unit_visual_category_counts(&grains),
            unit_authoritative_category_counts(&grains)
        );

        let mut visual_guard = 0usize;
        while grains.rolling_visual_motion_active() {
            visual_guard = visual_guard.saturating_add(1);
            assert!(
                visual_guard < 20_000,
                "unit visual transport failed to drain"
            );
            let _ = grains.advance_flowviz_tracers();
            assert!(grains.flowviz_unit_mass_matches_physics());
            assert_eq!(
                unit_visual_category_counts(&grains),
                unit_authoritative_category_counts(&grains)
            );
        }
        assert_eq!(grains.flowviz_shadow_columns, grains.columns);
        assert!(
            grains
                .flowviz_unit_custody
                .iter()
                .flatten()
                .all(Option::is_none)
        );
        assert_eq!(grains.flowviz_unit_misses(), 0);
    }
}

#[test]
fn unit_flowviz_visual_clock_does_not_invent_rolling_completion() {
    let mut engine = OsloSandboxEngine::new_front_unit_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let destination = source + 1;
    set_column_height(&mut engine, source, 4);
    set_column_height(&mut engine, destination, 1);
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) = engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .begin_unit_motion(source, Some(destination), category_id, visual_y, custody)
        .unwrap();
    engine.seed_rolling_grain_at_y_with_motion(
        destination,
        category_id,
        ToppleDirection::Right,
        visual_y,
        Some(id),
    );

    for _ in 0..32 {
        let _ = engine.advance_flowviz_tracers();
    }

    // Presentation may replay the observed source->destination edge, but it may
    // not infer a settlement/egress event that authoritative physics has not
    // performed. The live carrier therefore remains paired with the live
    // RollingGrain until a physical update supplies the next fact.
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    assert_eq!(carrier.physical, flowviz_unit::UnitPhysicalState::Rolling);
    assert!(carrier.active.is_none());
    assert!(carrier.queued.is_empty());
    assert_eq!(engine.rolling_grains.len(), 1);
    assert_eq!(engine.flowviz_unit_carrier_count(), 1);
    assert!(engine.rolling_visual_motion_active());
    assert!(engine.flowviz_unit_mass_matches_physics());
    assert_eq!(
        unit_visual_category_counts(&engine),
        unit_authoritative_category_counts(&engine)
    );
    assert_eq!(engine.flowviz_unit_misses(), 0);
}

#[test]
fn unit_flowviz_reentry_keeps_one_motion_id_and_reveals_exact_stack() {
    let mut engine = OsloSandboxEngine::new_front_unit_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let middle = source + 1;
    let destination = source + 2;
    set_column_height(&mut engine, source, 3);
    set_column_height(&mut engine, middle, 0);
    set_column_height(&mut engine, destination, 0);
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) = engine.pop_settled_grain_with_custody(source).unwrap();
    assert!(custody.is_none());
    let motion_id = engine
        .begin_unit_motion(source, Some(middle), category_id, visual_y, custody)
        .unwrap();
    engine.push_settled_grain_at_visual_y_with_custody(
        middle,
        category_id,
        Some(visual_y),
        Some(motion_id),
    );
    engine.mark_unit_settled(motion_id, middle);

    let (category_id, visual_y, custody) = engine.pop_settled_grain_with_custody(middle).unwrap();
    assert_eq!(custody, Some(motion_id));
    let reused = engine
        .begin_unit_motion(middle, Some(destination), category_id, visual_y, custody)
        .unwrap();
    assert_eq!(reused, motion_id);
    engine.push_settled_grain_at_visual_y_with_custody(
        destination,
        category_id,
        Some(visual_y),
        Some(motion_id),
    );
    engine.mark_unit_settled(motion_id, destination);

    assert_eq!(engine.flowviz_unit_carrier_count(), 1);
    assert_eq!(engine.flowviz_unit_segments(), 2);
    assert!(engine.flowviz_unit_mass_matches_physics());

    let mut guard = 0usize;
    while engine.rolling_visual_motion_active() {
        guard = guard.saturating_add(1);
        assert!(guard < 1_000, "unit re-entry carrier failed to drain");
        let _ = engine.advance_flowviz_tracers();
        assert!(engine.flowviz_unit_mass_matches_physics());
        assert_eq!(
            unit_visual_category_counts(&engine),
            unit_authoritative_category_counts(&engine)
        );
    }

    assert_eq!(engine.flowviz_shadow_columns, engine.columns);
    assert_eq!(engine.flowviz_unit_reveals(), 1);
    assert_eq!(engine.flowviz_unit_misses(), 0);
}

#[test]
fn unit_flowviz_wall_failure_replays_unit_transport_and_drains_to_exact_stack() {
    fn prepare(mut engine: OsloSandboxEngine) -> OsloSandboxEngine {
        let width = engine.lattice_size();
        for site in 0..(width / 2) {
            set_column_height(&mut engine, site, 12);
        }
        for site in (width / 2)..width {
            set_column_height(&mut engine, site, 2);
        }
        engine.reset_unit_flowviz_from_physics();
        engine.critical_slopes.fill(OSLO_THRESHOLD_HIGH);
        engine.enqueue_neighborhood(width / 2 - 1);
        engine.enqueue_neighborhood(width / 2);
        engine
    }

    fn relax(mut engine: OsloSandboxEngine) -> OsloSandboxEngine {
        let mut guard = 0usize;
        loop {
            guard = guard.saturating_add(1);
            assert!(guard < 250_000, "unit wall relaxation did not quiesce");
            let mut changed = engine.advance_rolling_grains();
            changed |= engine.topple_one_active_site();
            if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
                break;
            }
        }
        engine
    }

    let frozen = relax(prepare(OsloSandboxEngine::new_front_vessel(
        20, 10, TEST_SEED,
    )));
    let mut grains = relax(prepare(OsloSandboxEngine::new_front_unit_flowviz_vessel(
        20, 10, TEST_SEED,
    )));

    assert_eq!(grains.columns, frozen.columns);
    assert_eq!(grains.critical_slopes, frozen.critical_slopes);
    assert_eq!(grains.threshold_rng_state, frozen.threshold_rng_state);
    assert_eq!(grains.relax_rng_state, frozen.relax_rng_state);
    assert_eq!(grains.discharged_count(), frozen.discharged_count());
    assert!(grains.flowviz_unit_peak() > 8);
    assert!(grains.flowviz_unit_segments() > grains.flowviz_unit_peak());
    assert_eq!(grains.flowviz_unit_misses(), 0);
    assert!(grains.flowviz_unit_mass_matches_physics());

    let mut visual_guard = 0usize;
    while grains.rolling_visual_motion_active() {
        visual_guard = visual_guard.saturating_add(1);
        assert!(
            visual_guard < 100_000,
            "unit wall transport failed to drain"
        );
        let _ = grains.advance_flowviz_tracers();
        assert!(grains.flowviz_unit_mass_matches_physics());
        assert_eq!(
            unit_visual_category_counts(&grains),
            unit_authoritative_category_counts(&grains)
        );
    }

    assert_eq!(grains.flowviz_unit_carrier_count(), 0);
    assert_eq!(grains.flowviz_unit_egress_pending(), 0);
    assert_eq!(grains.flowviz_shadow_columns, grains.columns);
    assert!(
        grains
            .flowviz_unit_custody
            .iter()
            .flatten()
            .all(Option::is_none)
    );
    assert_eq!(grains.flowviz_unit_misses(), 0);
}

#[test]
fn unit_flowviz_never_predicts_an_unobserved_edge() {
    let mut engine = OsloSandboxEngine::new_front_unit_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let middle = source + 1;
    let destination = source + 2;
    set_column_height(&mut engine, source, 3);
    set_column_height(&mut engine, middle, 0);
    set_column_height(&mut engine, destination, 0);
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) = engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .begin_unit_motion(source, Some(middle), category_id, visual_y, custody)
        .unwrap();
    engine.seed_rolling_grain_at_y_with_motion(
        middle,
        category_id,
        ToppleDirection::Right,
        visual_y,
        Some(id),
    );

    assert_eq!(engine.flowviz_unit_segments(), 1);
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    assert_eq!(carrier.queued.len(), 1);
    assert!(carrier.active.is_none());
    assert_eq!(carrier.queued.front().unwrap().sequence, 1);
    assert_eq!(carrier.queued.front().unwrap().source, source);
    assert_eq!(carrier.queued.front().unwrap().destination, Some(middle));

    for _ in 0..8 {
        let _ = engine.advance_flowviz_tracers();
    }
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    assert_eq!(engine.flowviz_unit_segments(), 1);
    assert!(carrier.queued.is_empty());
    assert!(carrier.active.is_none());
    assert_eq!(carrier.physical, flowviz_unit::UnitPhysicalState::Rolling);

    engine.append_unit_segment(id, middle, Some(destination));
    assert_eq!(engine.flowviz_unit_segments(), 2);
    let second = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap();
    assert_eq!(second.sequence, 2);
    assert_eq!(second.destination, Some(destination));
    assert_eq!(engine.flowviz_unit_misses(), 0);
}

#[test]
fn unit_flowviz_resize_round_trip_preserves_custody_and_exact_stack() {
    let mut engine = OsloSandboxEngine::new_front_unit_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let destination = source + 1;
    set_column_height(&mut engine, source, 4);
    set_column_height(&mut engine, destination, 1);
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) = engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .begin_unit_motion(source, Some(destination), category_id, visual_y, custody)
        .unwrap();
    engine.seed_rolling_grain_at_y_with_motion(
        destination,
        category_id,
        ToppleDirection::Right,
        visual_y,
        Some(id),
    );
    assert!(engine.flowviz_unit_mass_matches_physics());
    assert_eq!(
        unit_visual_category_counts(&engine),
        unit_authoritative_category_counts(&engine)
    );

    engine.resize(10, 8);
    assert!(engine.flowviz_unit_carriers.contains_key(&id));
    assert!(engine.flowviz_unit_mass_matches_physics());
    assert_eq!(
        unit_visual_category_counts(&engine),
        unit_authoritative_category_counts(&engine)
    );

    engine.resize(20, 10);
    assert!(engine.flowviz_unit_carriers.contains_key(&id));
    assert!(engine.flowviz_unit_mass_matches_physics());
    assert_eq!(
        unit_visual_category_counts(&engine),
        unit_authoritative_category_counts(&engine)
    );

    let rolling = engine.rolling_grains.pop_front().unwrap();
    engine.push_settled_grain_at_visual_y_with_custody(
        rolling.site,
        rolling.category_id,
        Some(rolling.visual_y),
        rolling.motion_id,
    );
    engine.mark_unit_settled(id, rolling.site);

    let mut guard = 0usize;
    while engine.rolling_visual_motion_active() {
        guard = guard.saturating_add(1);
        assert!(guard < 1_000, "unit resize carrier failed to drain");
        let _ = engine.advance_flowviz_tracers();
        assert!(engine.flowviz_unit_mass_matches_physics());
        assert_eq!(
            unit_visual_category_counts(&engine),
            unit_authoritative_category_counts(&engine)
        );
    }

    assert_eq!(engine.flowviz_shadow_columns, engine.columns);
    assert!(
        engine
            .flowviz_unit_custody
            .iter()
            .flatten()
            .all(Option::is_none)
    );
    assert_eq!(engine.flowviz_unit_misses(), 0);
}

#[test]
fn unit_flowviz_colored_settlement_waits_for_exact_stack_order() {
    let mut engine = OsloSandboxEngine::new_front_unit_flowviz_vessel(20, 10, TEST_SEED);
    let destination = engine.lattice_size() / 2;
    let first_source = destination - 2;
    let middle = destination - 1;
    let second_source = destination + 1;
    let base = CategoryId::new(31);
    let lower = CategoryId::new(41);
    let upper = CategoryId::new(51);

    engine.columns[first_source] = vec![lower];
    engine.column_visual_y[first_source] = vec![None];
    engine.columns[second_source] = vec![upper];
    engine.column_visual_y[second_source] = vec![None];
    engine.columns[destination] = vec![base];
    engine.column_visual_y[destination] = vec![None];
    engine.reset_unit_flowviz_from_physics();

    let (lower_category, lower_y, lower_custody) =
        engine.pop_settled_grain_with_custody(first_source).unwrap();
    let lower_id = engine
        .begin_unit_motion(
            first_source,
            Some(middle),
            lower_category,
            lower_y,
            lower_custody,
        )
        .unwrap();
    engine.append_unit_segment(lower_id, middle, Some(destination));
    engine.push_settled_grain_at_visual_y_with_custody(
        destination,
        lower_category,
        Some(lower_y),
        Some(lower_id),
    );
    engine.mark_unit_settled(lower_id, destination);

    let (upper_category, upper_y, upper_custody) = engine
        .pop_settled_grain_with_custody(second_source)
        .unwrap();
    let upper_id = engine
        .begin_unit_motion(
            second_source,
            Some(destination),
            upper_category,
            upper_y,
            upper_custody,
        )
        .unwrap();
    engine.push_settled_grain_at_visual_y_with_custody(
        destination,
        upper_category,
        Some(upper_y),
        Some(upper_id),
    );
    engine.mark_unit_settled(upper_id, destination);

    assert_eq!(engine.columns[destination], vec![base, lower, upper]);
    assert_eq!(engine.flowviz_shadow_columns[destination], vec![base]);
    assert_eq!(engine.flowviz_unit_carrier_count(), 2);

    for _ in 0..5 {
        let _ = engine.advance_flowviz_tracers();
    }
    assert!(engine.flowviz_unit_carriers.get(&upper_id).unwrap().arrived);
    assert!(!engine.flowviz_unit_carriers.get(&lower_id).unwrap().arrived);
    assert_eq!(
        engine.flowviz_shadow_columns[destination],
        vec![base],
        "upper CategoryId must not reveal through a lower pending stack depth"
    );
    assert_eq!(
        unit_visual_category_counts(&engine),
        unit_authoritative_category_counts(&engine)
    );

    let mut guard = 0usize;
    while engine.rolling_visual_motion_active() {
        guard = guard.saturating_add(1);
        assert!(guard < 1_000, "colored ordered settlement failed to drain");
        let _ = engine.advance_flowviz_tracers();
        assert!(engine.flowviz_unit_mass_matches_physics());
        assert_eq!(
            unit_visual_category_counts(&engine),
            unit_authoritative_category_counts(&engine)
        );
    }

    assert_eq!(engine.flowviz_shadow_columns, engine.columns);
    assert_eq!(engine.flowviz_unit_reveals(), 2);
    assert_eq!(engine.flowviz_unit_misses(), 0);
}

#[test]
fn unit_flowviz_discharge_keeps_one_visible_unit_until_observed_egress_finishes() {
    let mut engine = OsloSandboxEngine::new_front_unit_flowviz_vessel(20, 10, TEST_SEED);
    let (visible_start, _) = engine.visible_lattice_bounds();
    let category_id = CategoryId::new(61);
    engine.columns[visible_start] = vec![category_id];
    engine.column_visual_y[visible_start] = vec![None];
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) = engine
        .pop_settled_grain_with_custody(visible_start)
        .unwrap();
    let id = engine
        .begin_unit_motion(visible_start, None, category_id, visual_y, custody)
        .unwrap();
    engine.mark_unit_discharged(id);

    assert_eq!(engine.flowviz_unit_carrier_count(), 1);
    assert_eq!(engine.flowviz_unit_egress_pending(), 1);
    assert_eq!(engine.flowviz_unit_segments(), 1);
    assert!(engine.flowviz_unit_mass_matches_physics());
    assert_eq!(
        unit_visual_category_counts(&engine),
        unit_authoritative_category_counts(&engine)
    );

    let mut guard = 0usize;
    while engine.rolling_visual_motion_active() {
        guard = guard.saturating_add(1);
        assert!(
            guard < 100,
            "unit egress carrier failed to leave the viewport"
        );
        let _ = engine.advance_flowviz_tracers();
        assert!(engine.flowviz_unit_mass_matches_physics());
        assert_eq!(
            unit_visual_category_counts(&engine),
            unit_authoritative_category_counts(&engine)
        );
    }

    assert_eq!(engine.flowviz_unit_carrier_count(), 0);
    assert_eq!(engine.flowviz_unit_egress_pending(), 0);
    assert_eq!(engine.flowviz_shadow_columns, engine.columns);
    assert_eq!(engine.flowviz_unit_misses(), 0);
}
