use super::*;

const TEST_SEED: u64 = 0x51A7_A51A_0B10_5EED;

fn grain() -> CategoryId {
    CategoryId::new(1)
}

fn set_column(engine: &mut OsloSandboxEngine, site: usize, categories: Vec<CategoryId>) {
    let height = categories.len();
    engine.columns[site] = categories.clone();
    engine.column_visual_y[site] = vec![None; height];
    if engine.flowviz_conservative || engine.flowviz_unit {
        engine.flowviz_shadow_columns[site] = categories;
    }
    if engine.flowviz_unit {
        engine.flowviz_unit_custody[site] = vec![None; height];
    }
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

fn seed_overlapping_micro_carriers(
    engine: &mut OsloSandboxEngine,
    count: usize,
) -> Vec<flowviz_unit::FlowVizMotionId> {
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(
        engine,
        source,
        (0..count)
            .map(|index| CategoryId::new(index as u64 + 1))
            .collect(),
    );
    set_column(engine, destination, Vec::new());
    engine.reset_unit_flowviz_from_physics();

    let mut ids = Vec::new();
    for _ in 0..count {
        let (category_id, visual_y, custody) =
            engine.pop_settled_grain_with_custody(source).unwrap();
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
        ids.push(id);
    }

    let y = (engine.surface.grid_height_dots / 2) as f32;
    for id in &ids {
        let carrier = engine.flowviz_unit_carriers.get_mut(id).unwrap();
        carrier.x = source as f32;
        carrier.y = y;
        carrier.ideal_x = source as f32;
        carrier.ideal_y = y;
    }
    ids
}

#[test]
fn unit_micro_flowviz_is_enabled_only_for_the_015b_constructor() {
    let baseline = OsloSandboxEngine::new_front_unit_flowviz_vessel(20, 10, TEST_SEED);
    let micro = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);

    assert!(baseline.flowviz_unit_enabled());
    assert!(!baseline.flowviz_unit_micro_enabled());
    assert!(micro.flowviz_unit_enabled());
    assert!(micro.flowviz_unit_micro_enabled());
    assert_eq!(baseline.columns, micro.columns);
    assert_eq!(baseline.critical_slopes, micro.critical_slopes);
    assert_eq!(baseline.threshold_rng_state, micro.threshold_rng_state);
    assert_eq!(baseline.relax_rng_state, micro.relax_rng_state);
}

#[test]
fn unit_micro_flowviz_preserves_frozen_front_physics_and_exact_stack() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut micro = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);
    let width = frozen.columns.len();

    for drive in 0..750usize {
        let site = (drive.wrapping_mul(37).wrapping_add(11)) % width;
        direct_front_drive_and_quiesce(&mut frozen, site);
        direct_front_drive_and_quiesce(&mut micro, site);

        assert_eq!(micro.columns, frozen.columns);
        assert_eq!(micro.critical_slopes, frozen.critical_slopes);
        assert_eq!(micro.threshold_rng_state, frozen.threshold_rng_state);
        assert_eq!(micro.relax_rng_state, frozen.relax_rng_state);
        assert_eq!(micro.discharged_count(), frozen.discharged_count());
        assert_eq!(micro.front_erosions(), frozen.front_erosions());
        assert_eq!(
            micro.front_support_recruits(),
            frozen.front_support_recruits()
        );
        assert_eq!(micro.avalanche_moves, frozen.avalanche_moves);
        assert!(micro.flowviz_unit_mass_matches_physics());

        let mut guard = 0usize;
        while micro.rolling_visual_motion_active() {
            guard = guard.saturating_add(1);
            assert!(guard < 25_000, "015B unit presentation failed to drain");
            let _ = micro.advance_flowviz_tracers();
            assert!(micro.flowviz_unit_mass_matches_physics());
        }

        assert_eq!(micro.flowviz_shadow_columns, micro.columns);
        assert_eq!(micro.flowviz_unit_misses(), 0);
    }
}

#[test]
fn unit_micro_settled_targets_preserve_exact_pending_stack_depth() {
    let mut engine = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);
    let destination = engine.columns.len() / 2;
    let lower_source = destination - 1;
    let upper_source = destination + 1;
    let base = CategoryId::new(31);
    let lower = CategoryId::new(41);
    let upper = CategoryId::new(51);

    set_column(&mut engine, destination, vec![base]);
    set_column(&mut engine, lower_source, vec![lower]);
    set_column(&mut engine, upper_source, vec![upper]);
    engine.reset_unit_flowviz_from_physics();

    let (lower_category, lower_y, lower_custody) =
        engine.pop_settled_grain_with_custody(lower_source).unwrap();
    let lower_id = engine
        .begin_unit_motion(
            lower_source,
            Some(destination),
            lower_category,
            lower_y,
            lower_custody,
        )
        .unwrap();
    engine.push_settled_grain_at_visual_y_with_custody(
        destination,
        lower_category,
        Some(lower_y),
        Some(lower_id),
    );
    engine.mark_unit_settled(lower_id, destination);

    let (upper_category, upper_y, upper_custody) =
        engine.pop_settled_grain_with_custody(upper_source).unwrap();
    let upper_id = engine
        .begin_unit_motion(
            upper_source,
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

    let lower_target = engine.flowviz_unit_settled_target_y(lower_id).unwrap();
    let upper_target = engine.flowviz_unit_settled_target_y(upper_id).unwrap();
    assert_eq!(lower_target - upper_target, 1.0);
    assert_eq!(engine.columns[destination], vec![base, lower, upper]);
}

#[test]
fn unit_micro_local_projection_separates_coincident_carriers_without_route_escape() {
    let mut engine = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);
    let ids = seed_overlapping_micro_carriers(&mut engine, 4);
    let source = engine.columns.len() / 2;
    let destination = source + 1;

    assert!(engine.relax_unit_micro_positions());
    let minimum = engine.flowviz_unit_micro_min_pair_distance().unwrap();
    assert!(
        minimum > 0.45,
        "coincident carriers remained visually collapsed: {minimum}"
    );

    for id in ids {
        let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
        assert!(carrier.x >= source as f32 - 0.34 - f32::EPSILON);
        assert!(carrier.x <= destination as f32 + 0.34 + f32::EPSILON);
        assert_eq!(carrier.queued.len(), 1);
        assert!(carrier.active.is_none());
    }
    assert_eq!(engine.flowviz_unit_segments(), 4);
    assert_eq!(engine.flowviz_unit_misses(), 0);
}

#[test]
fn unit_micro_rasterizer_keeps_dense_unit_carriers_individually_visible() {
    let mut engine = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);
    let ids = seed_overlapping_micro_carriers(&mut engine, 6);

    assert_eq!(engine.flowviz_unit_carrier_count(), ids.len());
    assert_eq!(engine.flowviz_unit_micro_render_cell_count(), ids.len());
    let placements = engine.collect_unit_micro_render_cells();
    let mut rendered_categories = placements
        .iter()
        .map(|(_, category_id, _, _)| *category_id)
        .collect::<Vec<_>>();
    let mut carrier_categories = engine
        .flowviz_unit_carriers
        .values()
        .map(|carrier| carrier.category_id)
        .collect::<Vec<_>>();
    rendered_categories.sort_unstable_by_key(|category_id| category_id.0);
    carrier_categories.sort_unstable_by_key(|category_id| category_id.0);
    assert_eq!(rendered_categories, carrier_categories);
    assert_eq!(engine.flowviz_unit_misses(), 0);
}

#[test]
fn unit_micro_relaxation_never_changes_observed_route_or_physical_state() {
    let mut engine = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);
    let ids = seed_overlapping_micro_carriers(&mut engine, 4);
    let before_columns = engine.columns.clone();
    let before_slopes = engine.critical_slopes.clone();
    let before_threshold_rng = engine.threshold_rng_state;
    let before_relax_rng = engine.relax_rng_state;
    let before_segments = ids
        .iter()
        .map(|id| {
            let carrier = engine.flowviz_unit_carriers.get(id).unwrap();
            carrier.queued.iter().copied().collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    for _ in 0..16 {
        let _ = engine.relax_unit_micro_positions();
    }

    assert_eq!(engine.columns, before_columns);
    assert_eq!(engine.critical_slopes, before_slopes);
    assert_eq!(engine.threshold_rng_state, before_threshold_rng);
    assert_eq!(engine.relax_rng_state, before_relax_rng);
    for (id, expected) in ids.iter().zip(before_segments) {
        let carrier = engine.flowviz_unit_carriers.get(id).unwrap();
        assert_eq!(carrier.queued.len(), expected.len());
        for (actual, expected) in carrier.queued.iter().zip(expected) {
            assert_eq!(actual.sequence, expected.sequence);
            assert_eq!(actual.source, expected.source);
            assert_eq!(actual.destination, expected.destination);
        }
    }
    assert_eq!(engine.flowviz_unit_segments(), ids.len());
    assert_eq!(engine.flowviz_unit_misses(), 0);
}

#[test]
fn unit_micro_resize_round_trip_preserves_active_geometry_custody_and_exact_stack() {
    let mut engine = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.lattice_size() / 2;
    let destination = source + 1;
    set_column(
        &mut engine,
        source,
        vec![
            CategoryId::new(11),
            CategoryId::new(12),
            CategoryId::new(13),
            CategoryId::new(14),
        ],
    );
    set_column(&mut engine, destination, vec![CategoryId::new(21)]);
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

    let _ = engine.advance_flowviz_tracers();
    let active_before = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .active
        .unwrap();
    assert!(active_before.progress > 0.0);
    assert!(engine.flowviz_unit_mass_matches_physics());

    engine.resize(10, 8);
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    let active_narrow = carrier.active.unwrap();
    assert!(carrier.x.is_finite());
    assert!(carrier.y.is_finite());
    assert!(carrier.ideal_x.is_finite());
    assert!(carrier.ideal_y.is_finite());
    assert!(active_narrow.start_x.is_finite());
    assert!(active_narrow.start_y.is_finite());
    assert!(active_narrow.target_x.is_finite());
    assert!(active_narrow.target_y.is_finite());
    assert!(engine.flowviz_unit_mass_matches_physics());

    engine.resize(20, 10);
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    let active_wide = carrier.active.unwrap();
    assert!(carrier.x.is_finite());
    assert!(carrier.y.is_finite());
    assert!(carrier.ideal_x.is_finite());
    assert!(carrier.ideal_y.is_finite());
    assert!(active_wide.start_x.is_finite());
    assert!(active_wide.start_y.is_finite());
    assert!(active_wide.target_x.is_finite());
    assert!(active_wide.target_y.is_finite());
    assert!(engine.flowviz_unit_mass_matches_physics());

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
        assert!(guard < 2_000, "015B resize carrier failed to drain");
        let _ = engine.advance_flowviz_tracers();
        assert!(engine.flowviz_unit_mass_matches_physics());
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
fn unit_micro_offscreen_pending_depths_remain_distinct_instead_of_clamping_to_top_row() {
    let mut engine = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);
    let destination = engine.columns.len() / 2;
    let lower_source = destination - 1;
    let upper_source = destination + 1;
    let visible_height = engine.surface.grid_height_dots;
    let base = CategoryId::new(61);
    let lower = CategoryId::new(62);
    let upper = CategoryId::new(63);

    set_column(&mut engine, destination, vec![base; visible_height + 2]);
    set_column(&mut engine, lower_source, vec![lower]);
    set_column(&mut engine, upper_source, vec![upper]);
    engine.reset_unit_flowviz_from_physics();

    let (lower_category, lower_y, lower_custody) =
        engine.pop_settled_grain_with_custody(lower_source).unwrap();
    let lower_id = engine
        .begin_unit_motion(
            lower_source,
            Some(destination),
            lower_category,
            lower_y,
            lower_custody,
        )
        .unwrap();
    engine.push_settled_grain_at_visual_y_with_custody(
        destination,
        lower_category,
        Some(lower_y),
        Some(lower_id),
    );
    engine.mark_unit_settled(lower_id, destination);

    let (upper_category, upper_y, upper_custody) =
        engine.pop_settled_grain_with_custody(upper_source).unwrap();
    let upper_id = engine
        .begin_unit_motion(
            upper_source,
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

    let lower_target = engine.flowviz_unit_settled_target_y(lower_id).unwrap();
    let upper_target = engine.flowviz_unit_settled_target_y(upper_id).unwrap();
    assert!(lower_target < 0.0);
    assert!(upper_target < lower_target);
    assert_eq!(lower_target - upper_target, 1.0);
}
