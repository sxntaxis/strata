use super::*;

const TEST_SEED: u64 = 0x51A7_0156_5A90_7701;

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

fn drain_support_drive(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 500_000, "015G support drive failed to drain");
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
        assert!(guard < 250_000, "015G frozen control failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
            break;
        }
    }
}

#[test]
fn unit_visual_support_constructor_extends_015f_without_physics_delta() {
    let direct =
        OsloSandboxEngine::new_front_unit_direct_geometry_flowviz_vessel(20, 10, TEST_SEED);
    let support =
        OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 10, TEST_SEED);

    assert!(direct.flowviz_unit_direct_geometry_enabled());
    assert!(!direct.flowviz_unit_visual_support_enabled());
    assert!(support.flowviz_unit_direct_geometry_enabled());
    assert!(support.flowviz_unit_visual_support_enabled());
    assert_eq!(support.columns, direct.columns);
    assert_eq!(support.critical_slopes, direct.critical_slopes);
    assert_eq!(support.threshold_rng_state, direct.threshold_rng_state);
    assert_eq!(support.rain_rng_state, direct.rain_rng_state);
    assert_eq!(support.relax_rng_state, direct.relax_rng_state);
}

#[test]
fn unit_visual_support_lane_uses_visible_shadow_not_hidden_physical_settlement() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(11)]);
    set_column(
        &mut engine,
        destination,
        vec![grain(21), grain(22), grain(23), grain(24)],
    );
    // Model three physically settled units that are still presentation-pending.
    engine.flowviz_shadow_columns[destination] = vec![grain(21)];
    engine.reset_unit_flowviz_from_physics();
    // reset_unit_flowviz_from_physics intentionally mirrors physics, so restore
    // the delayed visible support after the reset.
    engine.flowviz_shadow_columns[destination] = vec![grain(21)];

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

    assert_eq!(
        engine
            .flowviz_unit_carriers
            .get(&id)
            .unwrap()
            .queued
            .back()
            .unwrap()
            .observed_target_y,
        None,
        "015G lane must remain pending until the quantum occupancy is final"
    );
    engine.finalize_direct_rolling_batch_geometry();

    let target_y = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .back()
        .unwrap()
        .observed_target_y
        .unwrap();
    let expected_visible = engine.surface.grid_height_dots as f32 - 1.0 - 1.0;
    let old_hidden_physics = engine.surface.grid_height_dots as f32 - 4.0 - 1.0;
    assert_eq!(target_y, expected_visible);
    assert_ne!(target_y, old_hidden_physics);
}

#[test]
fn unit_visual_support_final_batch_lanes_are_contiguous_above_visible_bed() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 10, TEST_SEED);
    let destination = engine.columns.len() / 2;
    let source_a = destination - 1;
    let source_b = destination + 1;
    assert_eq!(source_a.abs_diff(destination), 1);
    assert_eq!(source_b.abs_diff(destination), 1);
    set_column(&mut engine, source_a, vec![grain(31)]);
    set_column(&mut engine, source_b, vec![grain(32)]);
    set_column(&mut engine, destination, vec![grain(41), grain(42)]);
    engine.reset_unit_flowviz_from_physics();

    let mut ids = Vec::new();
    for (source, direction) in [
        (source_a, ToppleDirection::Right),
        (source_b, ToppleDirection::Left),
    ] {
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
            direction,
            source_y,
            Some(id),
        );
        ids.push(id);
    }

    engine.finalize_direct_rolling_batch_geometry();
    let base = engine.surface.grid_height_dots as f32
        - engine.flowviz_shadow_columns[destination].len() as f32
        - 1.0;
    let targets = ids
        .into_iter()
        .map(|id| {
            engine
                .flowviz_unit_carriers
                .get(&id)
                .unwrap()
                .queued
                .back()
                .unwrap()
                .observed_target_y
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(targets, vec![base, base - 1.0]);
}

#[test]
fn unit_visual_support_playback_uses_finalized_target_after_shadow_changes() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(51)]);
    set_column(&mut engine, destination, vec![grain(61)]);
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
    engine.finalize_direct_rolling_batch_geometry();
    let target_y = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .back()
        .unwrap()
        .observed_target_y
        .unwrap();

    // A later visible-bed mutation must not bend the already-observed segment.
    engine.flowviz_shadow_columns[destination].extend([
        grain(71),
        grain(72),
        grain(73),
        grain(74),
    ]);
    assert!(engine.advance_unit_flowviz());
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    let active = carrier.active.unwrap();
    assert_eq!(active.target_y, target_y);
    let expected_y = active.start_y + (target_y - active.start_y) * flowviz_unit::UNIT_SEGMENT_STEP;
    assert!((carrier.y - expected_y).abs() < 0.0001);
}

#[test]
fn unit_visual_support_preserves_frozen_front_physics_and_exact_final_stack() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut support =
        OsloSandboxEngine::new_front_unit_visual_support_flowviz_vessel(20, 10, TEST_SEED);
    let sites = [9usize, 10, 9, 8, 10, 11, 9, 10];

    for drive in 0..24 {
        let site = sites[drive % sites.len()];
        frozen_drive_and_quiesce(&mut frozen, site);
        drain_support_drive(&mut support, site);

        assert_eq!(support.columns, frozen.columns);
        assert_eq!(support.critical_slopes, frozen.critical_slopes);
        assert_eq!(support.threshold_rng_state, frozen.threshold_rng_state);
        assert_eq!(support.discharged, frozen.discharged);
        assert_eq!(support.avalanche_moves, frozen.avalanche_moves);
        assert_eq!(support.flowviz_shadow_columns, support.columns);
        assert_eq!(support.flowviz_unit_misses(), 0);
    }
}
