use super::*;

const TEST_SEED: u64 = 0x51A7_015F_D1CE_7EED;

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

fn drain_coherent_batch(engine: &mut OsloSandboxEngine) {
    let mut guard = 0usize;
    while engine.flowviz_unit_coherent_batch_pending() {
        guard = guard.saturating_add(1);
        assert!(guard < 100, "015F coherent batch failed to drain");
        let _ = engine.advance_testing_truthful_visual_frame();
    }
}

#[test]
fn unit_direct_constructor_extends_015e_without_physics_delta() {
    let coherent = OsloSandboxEngine::new_front_unit_coherent_flowviz_vessel(20, 10, TEST_SEED);
    let direct =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);

    assert!(coherent.flowviz_unit_coherent_enabled());
    assert!(!coherent.flowviz_unit_direct_geometry_enabled());
    assert!(direct.flowviz_unit_coherent_enabled());
    assert!(direct.flowviz_unit_direct_geometry_enabled());
    assert_eq!(direct.columns, coherent.columns);
    assert_eq!(direct.critical_slopes, coherent.critical_slopes);
    assert_eq!(direct.threshold_rng_state, coherent.threshold_rng_state);
    assert_eq!(direct.rain_rng_state, coherent.rain_rng_state);
    assert_eq!(direct.relax_rng_state, coherent.relax_rng_state);
}

#[test]
fn unit_direct_initial_recruit_segment_matches_assigned_rolling_lane() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(11)]);
    set_column(
        &mut engine,
        destination,
        vec![grain(21), grain(22), grain(23)],
    );
    engine.reset_unit_flowviz_from_physics();

    let (category_id, source_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
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

    let rolling = engine.rolling_grains.back().unwrap();
    let segment = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap();
    assert_eq!(rolling.motion_id, Some(id));
    assert_eq!(segment.observed_source_y, Some(source_y as f32));
    assert_eq!(segment.observed_target_y, rolling.unit_observed_y);
}

#[test]
fn unit_direct_same_destination_recruits_use_distinct_assigned_lanes() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(31), grain(32)]);
    set_column(&mut engine, destination, vec![grain(41), grain(42)]);
    engine.reset_unit_flowviz_from_physics();

    let mut assigned = Vec::new();
    for _ in 0..2 {
        let (category_id, source_y, custody) =
            engine.pop_settled_grain_with_custody(source).unwrap();
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
        let rolling_y = engine
            .rolling_grains
            .back()
            .unwrap()
            .unit_observed_y
            .unwrap();
        let target_y = engine
            .flowviz_unit_carriers
            .get(&id)
            .unwrap()
            .queued
            .front()
            .unwrap()
            .observed_target_y
            .unwrap();
        assert_eq!(target_y, rolling_y);
        assigned.push(rolling_y);
    }

    assert_eq!((assigned[0] - assigned[1]).abs(), 1.0);
}

#[test]
fn unit_direct_offscreen_lane_remains_bottom_relative_and_unclamped() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(45)]);
    let overheight = engine.surface.grid_height_dots.saturating_add(3);
    set_column(
        &mut engine,
        destination,
        (0..overheight)
            .map(|index| grain(index as u64 + 500))
            .collect(),
    );
    engine.reset_unit_flowviz_from_physics();

    let (category_id, source_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
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

    let rolling_y = engine
        .rolling_grains
        .back()
        .unwrap()
        .unit_observed_y
        .unwrap();
    let target_y = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap()
        .observed_target_y
        .unwrap();
    assert!(rolling_y < 0.0);
    assert_eq!(target_y, rolling_y);
}

#[test]
fn unit_direct_second_hop_uses_direct_y_before_and_after_physics_move() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let middle = source + 1;
    let destination = middle + 1;
    set_column(&mut engine, source, vec![grain(51)]);
    set_column(&mut engine, middle, vec![grain(61)]);
    set_column(&mut engine, destination, Vec::new());
    engine.reset_unit_flowviz_from_physics();

    let (category_id, source_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .record_flowviz_mobile_entry_with_custody(
            source,
            middle,
            category_id,
            source_y,
            custody,
        )
        .unwrap();
    engine.seed_rolling_grain_at_y_with_motion(
        middle,
        category_id,
        ToppleDirection::Right,
        source_y,
        Some(id),
    );
    drain_coherent_batch(&mut engine);

    let before_y = engine
        .rolling_grains
        .front()
        .unwrap()
        .unit_observed_y
        .unwrap();
    assert_eq!(engine.rolling_grains.front().unwrap().site, middle);
    let moves_before = engine.avalanche_moves;
    assert!(engine.advance_testing_coherent_flow_frame());
    assert!(engine.avalanche_moves > moves_before);

    let rolling = engine
        .rolling_grains
        .iter()
        .find(|rolling| rolling.motion_id == Some(id))
        .unwrap();
    assert_eq!(rolling.site, destination);
    let after_y = rolling.unit_observed_y.unwrap();
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    let segment = carrier
        .active
        .map(|active| active.segment)
        .or_else(|| carrier.queued.back().copied())
        .expect("second direct hop must remain observable");
    assert_eq!(segment.source, middle);
    assert_eq!(segment.destination, Some(destination));
    assert_eq!(segment.observed_source_y, Some(before_y));
    assert_eq!(segment.observed_target_y, Some(after_y));
}

#[test]
fn unit_direct_resize_shifts_rolling_and_segment_geometry_once_with_canvas_growth() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(65)]);
    set_column(&mut engine, destination, vec![grain(66), grain(67)]);
    engine.reset_unit_flowviz_from_physics();

    let (category_id, source_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
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

    let before_height = engine.surface.grid_height_dots as f32;
    let before_rolling_y = engine.rolling_grains.back().unwrap().unit_observed_y.unwrap();
    let before_segment_y = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap()
        .observed_target_y
        .unwrap();
    assert_eq!(before_rolling_y, before_segment_y);

    engine.resize(20, 14);
    let expanded_height = engine.surface.grid_height_dots as f32;
    let shift = expanded_height - before_height;
    let expanded_rolling_y = engine.rolling_grains.back().unwrap().unit_observed_y.unwrap();
    let expanded_segment_y = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap()
        .observed_target_y
        .unwrap();
    assert_eq!(expanded_rolling_y, before_rolling_y + shift);
    assert_eq!(expanded_segment_y, before_segment_y + shift);

    engine.resize(20, 10);
    let restored_rolling_y = engine.rolling_grains.back().unwrap().unit_observed_y.unwrap();
    let restored_segment_y = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap()
        .observed_target_y
        .unwrap();
    assert_eq!(restored_rolling_y, expanded_rolling_y);
    assert_eq!(restored_segment_y, expanded_segment_y);

    engine.resize(20, 14);
    assert_eq!(
        engine.rolling_grains.back().unwrap().unit_observed_y,
        Some(expanded_rolling_y)
    );
    assert_eq!(
        engine
            .flowviz_unit_carriers
            .get(&id)
            .unwrap()
            .queued
            .front()
            .unwrap()
            .observed_target_y,
        Some(expanded_segment_y)
    );
}

#[test]
fn unit_direct_recorded_hop_geometry_does_not_follow_later_column_mutation() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let middle = source + 1;
    let destination = middle + 1;
    set_column(&mut engine, source, vec![grain(71)]);
    set_column(&mut engine, middle, vec![grain(72)]);
    set_column(&mut engine, destination, Vec::new());
    engine.reset_unit_flowviz_from_physics();

    let (category_id, source_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .record_flowviz_mobile_entry_with_custody(
            source,
            middle,
            category_id,
            source_y,
            custody,
        )
        .unwrap();
    engine.seed_rolling_grain_at_y_with_motion(
        middle,
        category_id,
        ToppleDirection::Right,
        source_y,
        Some(id),
    );
    drain_coherent_batch(&mut engine);
    let _ = engine.advance_testing_coherent_flow_frame();

    let before = {
        let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
        carrier
            .active
            .map(|active| active.segment)
            .or_else(|| carrier.queued.back().copied())
            .unwrap()
    };
    engine.columns[destination].extend([grain(81), grain(82), grain(83), grain(84)]);
    let after = {
        let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
        carrier
            .active
            .map(|active| active.segment)
            .or_else(|| carrier.queued.back().copied())
            .unwrap()
    };
    assert_eq!(after.observed_source_y, before.observed_source_y);
    assert_eq!(after.observed_target_y, before.observed_target_y);
}

fn direct_frozen_drive_and_quiesce(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 250_000, "015F frozen control failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
            break;
        }
    }
}

fn direct_geometry_drive_and_drain(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 500_000, "015F direct-geometry drive failed to drain");
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

#[test]
fn unit_direct_geometry_preserves_frozen_front_physics_and_exact_final_stack() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut direct =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);
    let sites = [9usize, 10, 9, 8, 10, 11, 9, 10];

    for drive in 0..32 {
        let site = sites[drive % sites.len()];
        direct_frozen_drive_and_quiesce(&mut frozen, site);
        direct_geometry_drive_and_drain(&mut direct, site);

        assert_eq!(direct.columns, frozen.columns);
        assert_eq!(direct.critical_slopes, frozen.critical_slopes);
        assert_eq!(direct.threshold_rng_state, frozen.threshold_rng_state);
        assert_eq!(direct.relax_rng_state, frozen.relax_rng_state);
        assert_eq!(direct.discharged_count(), frozen.discharged_count());
        assert_eq!(direct.avalanche_moves, frozen.avalanche_moves);
        assert_eq!(direct.flowviz_shadow_columns, direct.columns);
        assert_eq!(direct.flowviz_unit_misses(), 0);
    }
}
