use super::*;
const TEST_SEED: u64 = 0x51A7_0158_D157_A11E;
fn grain(id: u64) -> CategoryId {
    CategoryId::new(id)
}
fn set_column(engine: &mut OsloSandboxEngine, site: usize, categories: Vec<CategoryId>) {
    let height = categories.len();
    engine.columns[site] = categories.clone();
    engine.column_visual_y[site] = vec![None; height];
    engine.flowviz_shadow_columns[site] = categories;
    engine.flowviz_unit_custody[site] = vec![None; height];
}
fn seed_large_drop(engine: &mut OsloSandboxEngine) -> flowviz_unit::FlowVizMotionId {
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(engine, source, (0..20).map(|_| grain(51)).collect());
    set_column(engine, destination, vec![grain(61)]);
    engine.reset_unit_flowviz_from_physics();
    let (category_id, source_y, custody) = engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .record_flowviz_mobile_entry_with_custody(
            source,
            destination,
            category_id,
            source_y,
            custody,
        )
        .unwrap();
    engine.seed_rolling_grain_at_y_with_motion(
        destination,
        category_id,
        ToppleDirection::Right,
        source_y,
        Some(id),
    );
    engine.finalize_direct_rolling_batch_geometry();
    id
}
#[test]
fn unit_distance_aware_constructor_extends_015g_without_physics_delta() {
    let support = OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 10, TEST_SEED);
    let distance = OsloSandboxEngine::new_front_unit_distance_aware_flowviz_vessel(20, 10, TEST_SEED);
    assert!(support.flowviz_unit_visual_support_enabled());
    assert!(!support.flowviz_unit_distance_aware_enabled());
    assert!(distance.flowviz_unit_visual_support_enabled());
    assert!(distance.flowviz_unit_distance_aware_enabled());
    assert_eq!(distance.columns, support.columns);
    assert_eq!(distance.critical_slopes, support.critical_slopes);
    assert_eq!(distance.threshold_rng_state, support.threshold_rng_state);
    assert_eq!(distance.rain_rng_state, support.rain_rng_state);
    assert_eq!(distance.relax_rng_state, support.relax_rng_state);
}
#[test]
fn unit_distance_aware_short_hop_keeps_015g_cadence_and_linear_y() {
    let start_y = 20.0;
    let target_y = 23.0;
    let step = flowviz_unit_distance::unit_segment_progress_step(start_y, target_y, true);
    assert_eq!(step, flowviz_unit::UNIT_SEGMENT_STEP);
    assert_eq!(
        flowviz_unit_distance::unit_segment_vertical_fraction(step, start_y, target_y, true),
        step
    );
}
#[test]
fn unit_distance_aware_large_drop_is_longer_bounded_and_gravity_shaped() {
    let start_y = 10.0;
    let target_y = 160.0;
    let step = flowviz_unit_distance::unit_segment_progress_step(start_y, target_y, true);
    let frames = (1.0 / step).round();
    assert!(step < flowviz_unit::UNIT_SEGMENT_STEP);
    assert!((5.0..=18.0).contains(&frames));
    let halfway_y = flowviz_unit_distance::unit_segment_vertical_fraction(0.5, start_y, target_y, true);
    assert!((halfway_y - 0.25).abs() < 0.0001);
    assert_eq!(
        flowviz_unit_distance::unit_segment_vertical_fraction(1.0, start_y, target_y, true),
        1.0
    );
}
#[test]
fn unit_distance_aware_large_drop_preserves_015g_target_but_replays_it_slower() {
    let mut support =
        OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 12, TEST_SEED);
    let mut distance =
        OsloSandboxEngine::new_front_unit_distance_aware_flowviz_vessel(20, 12, TEST_SEED);
    let support_id = seed_large_drop(&mut support);
    let distance_id = seed_large_drop(&mut distance);
    let support_target = support
        .flowviz_unit_carriers
        .get(&support_id)
        .unwrap()
        .queued
        .front()
        .unwrap()
        .observed_target_y
        .unwrap();
    let distance_target = distance
        .flowviz_unit_carriers
        .get(&distance_id)
        .unwrap()
        .queued
        .front()
        .unwrap()
        .observed_target_y
        .unwrap();
    assert_eq!(support_target, distance_target);
    assert!(support.advance_unit_flowviz());
    assert!(distance.advance_unit_flowviz());
    let support_carrier = support.flowviz_unit_carriers.get(&support_id).unwrap();
    let distance_carrier = distance.flowviz_unit_carriers.get(&distance_id).unwrap();
    let support_active = support_carrier.active.unwrap();
    let distance_active = distance_carrier.active.unwrap();
    assert_eq!(support_active.target_y, distance_active.target_y);
    assert!(distance_active.progress < support_active.progress);
    assert!(distance_carrier.y < support_carrier.y);
}
#[test]
fn unit_distance_aware_large_drop_arrives_exactly_at_finalized_target() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_distance_aware_flowviz_vessel(20, 12, TEST_SEED);
    let id = seed_large_drop(&mut engine);
    let target_y = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap()
        .observed_target_y
        .unwrap();
    let mut frames = 0usize;
    while engine
        .flowviz_unit_carriers
        .get(&id)
        .is_some_and(|carrier| carrier.active.is_some() || !carrier.queued.is_empty())
    {
        frames += 1;
        assert!(frames <= 18, "distance-aware long drop exceeded its frame cap");
        assert!(engine.advance_unit_flowviz());
    }
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    assert!((carrier.y - target_y).abs() < 0.0001);
    assert!((5..=18).contains(&frames));
}
fn drain_distance_drive(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);
    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 500_000, "015H distance-aware drive failed to drain");
        let changed = if engine.visible_flow_active() {
            engine.advance_testing_coherent_flow_frame()
        } else if engine.rolling_visual_motion_active() {
            engine.advance_testing_truthful_visual_frame()
        } else {
            false
        };
        if !changed
            && engine.active_sites.is_empty()
            && !engine.explicit_flow_active()
            && !engine.rolling_visual_motion_active()
        {
            break;
        }
    }
}
fn frozen_drive_and_quiesce(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);
    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 250_000, "015H frozen control failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
            break;
        }
    }
}
#[test]
fn unit_distance_aware_preserves_frozen_front_physics_and_exact_final_stack() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut distance =
        OsloSandboxEngine::new_front_unit_distance_aware_flowviz_vessel(20, 10, TEST_SEED);
    let sites = [9usize, 10, 9, 8, 10, 11, 9, 10];
    for drive in 0..24 {
        let site = sites[drive % sites.len()];
        frozen_drive_and_quiesce(&mut frozen, site);
        drain_distance_drive(&mut distance, site);
        assert_eq!(distance.columns, frozen.columns);
        assert_eq!(distance.critical_slopes, frozen.critical_slopes);
        assert_eq!(distance.threshold_rng_state, frozen.threshold_rng_state);
        assert_eq!(distance.discharged, frozen.discharged);
        assert_eq!(distance.avalanche_moves, frozen.avalanche_moves);
        assert_eq!(distance.flowviz_shadow_columns, distance.columns);
        assert_eq!(distance.flowviz_unit_misses(), 0);
    }
}
