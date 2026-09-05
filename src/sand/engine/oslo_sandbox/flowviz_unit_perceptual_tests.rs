use super::*;

const TEST_SEED: u64 = 0x51A7_015C_0B10_5EED;

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

fn direct_front_drive_and_quiesce(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 250_000, "015C direct drive failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
            break;
        }
    }
}

#[test]
fn unit_perceptual_constructor_is_a_presentation_only_extension_of_015b() {
    let micro = OsloSandboxEngine::new_front_unit_micro_flowviz_vessel(20, 10, TEST_SEED);
    let perceptual =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);

    assert!(micro.flowviz_unit_micro_enabled());
    assert!(!micro.flowviz_unit_perceptual_enabled());
    assert!(perceptual.flowviz_unit_micro_enabled());
    assert!(perceptual.flowviz_unit_perceptual_enabled());
    assert_eq!(micro.columns, perceptual.columns);
    assert_eq!(micro.critical_slopes, perceptual.critical_slopes);
    assert_eq!(micro.threshold_rng_state, perceptual.threshold_rng_state);
    assert_eq!(micro.relax_rng_state, perceptual.relax_rng_state);
}

#[test]
fn unit_perceptual_freezes_observed_segment_geometry_before_shadow_changes() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(11)]);
    set_column(&mut engine, destination, vec![grain(21), grain(22), grain(23)]);
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
    let expected_target = engine.unit_observed_settled_target_y(destination);
    let id = engine
        .begin_unit_motion(source, Some(destination), category_id, visual_y, custody)
        .unwrap();
    let segment = *engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap();
    assert_eq!(segment.observed_source_y, Some(visual_y as f32));
    assert_eq!(segment.observed_target_y, Some(expected_target));

    engine.flowviz_shadow_columns[destination].clear();
    let _ = engine.advance_flowviz_tracers();
    let active = engine.flowviz_unit_carriers.get(&id).unwrap().active.unwrap();
    assert_eq!(active.target_y, expected_target);
}

#[test]
fn unit_perceptual_rolling_entry_records_distinct_observed_lanes() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(31), grain(32)]);
    set_column(&mut engine, destination, vec![grain(41), grain(42)]);
    engine.reset_unit_flowviz_from_physics();

    let mut targets = Vec::new();
    for _ in 0..2 {
        let (category_id, visual_y, custody) =
            engine.pop_settled_grain_with_custody(source).unwrap();
        let id = engine
            .record_flowviz_mobile_entry_with_custody(
                source,
                destination,
                category_id,
                visual_y,
                custody,
            )
            .unwrap();
        let target = engine
            .flowviz_unit_carriers
            .get(&id)
            .unwrap()
            .queued
            .front()
            .unwrap()
            .observed_target_y
            .unwrap();
        targets.push(target);
        engine.seed_rolling_grain_at_y_with_motion(
            destination,
            category_id,
            ToppleDirection::Right,
            visual_y,
            Some(id),
        );
    }

    assert_eq!(targets[0] - targets[1], 1.0);
    assert!(engine.flowviz_unit_observed_geometry_complete());
}

#[test]
fn unit_perceptual_second_rolling_hop_starts_from_the_previous_observed_lane() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let middle = source + 1;
    let destination = middle + 1;
    set_column(&mut engine, source, vec![grain(71)]);
    set_column(&mut engine, middle, vec![grain(72), grain(73)]);
    set_column(&mut engine, destination, vec![grain(74)]);
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .record_flowviz_mobile_entry_with_custody(
            source,
            middle,
            category_id,
            visual_y,
            custody,
        )
        .unwrap();
    let first_target = engine
        .flowviz_unit_carriers
        .get(&id)
        .unwrap()
        .queued
        .front()
        .unwrap()
        .observed_target_y
        .unwrap();
    engine.append_unit_rolling_segment(id, middle, destination);
    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    let second = carrier.queued.back().unwrap();

    assert_eq!(second.source, middle);
    assert_eq!(second.destination, Some(destination));
    assert_eq!(second.observed_source_y, Some(first_target));
    assert_eq!(
        second.observed_target_y,
        Some(engine.unit_observed_rolling_target_y(destination))
    );
}

#[test]
fn unit_perceptual_dense_raster_uses_full_local_vertical_capacity() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    let count = 20usize;
    set_column(
        &mut engine,
        source,
        (0..count).map(|index| grain(index as u64 + 100)).collect(),
    );
    set_column(&mut engine, destination, Vec::new());
    engine.reset_unit_flowviz_from_physics();

    let mut ids = Vec::new();
    for _ in 0..count {
        let (category_id, visual_y, custody) =
            engine.pop_settled_grain_with_custody(source).unwrap();
        let id = engine
            .record_flowviz_mobile_entry_with_custody(
                source,
                destination,
                category_id,
                visual_y,
                custody,
            )
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

    let placements = engine.collect_unit_micro_render_cells();
    assert_eq!(placements.len(), ids.len());
    let unique = placements
        .iter()
        .map(|(_, _, x, y)| (*x, *y))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), ids.len());
}

#[test]
fn unit_perceptual_resize_preserves_bottom_relative_observed_geometry() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(181)]);
    set_column(&mut engine, destination, vec![grain(182), grain(183)]);
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .begin_unit_motion(source, Some(destination), category_id, visual_y, custody)
        .unwrap();
    let _ = engine.advance_flowviz_tracers();
    let before_height = engine.surface.grid_height_dots as f32;
    let before_top = engine.visible_vertical_bounds().0 as f32;
    let before = engine.flowviz_unit_carriers.get(&id).unwrap().active.unwrap();

    engine.resize(20, 14);
    let expanded_height = engine.surface.grid_height_dots as f32;
    let expanded_top = engine.visible_vertical_bounds().0 as f32;
    let shift = expanded_height - before_height;
    let expanded = engine.flowviz_unit_carriers.get(&id).unwrap().active.unwrap();
    assert_eq!(expanded.start_y, before.start_y + shift);
    assert_eq!(expanded.target_y, before.target_y + shift);
    assert_eq!(
        expanded.segment.observed_source_y,
        before.segment.observed_source_y.map(|value| value + shift)
    );
    assert_eq!(
        expanded.segment.observed_target_y,
        before.segment.observed_target_y.map(|value| value + shift)
    );

    // The logical canvas is grow-only. Bottom-relative geometry therefore
    // preserves canonical depth when the canvas expands, while viewport-local
    // coordinates change only with the visible top.
    assert_eq!(
        expanded_height - 1.0 - expanded.start_y,
        before_height - 1.0 - before.start_y
    );
    assert_eq!(
        expanded_height - 1.0 - expanded.target_y,
        before_height - 1.0 - before.target_y
    );
    assert_eq!(expanded.start_y - expanded_top, expanded.start_y);

    engine.resize(20, 10);
    let restored_height = engine.surface.grid_height_dots as f32;
    let restored_top = engine.visible_vertical_bounds().0 as f32;
    let restored = engine.flowviz_unit_carriers.get(&id).unwrap().active.unwrap();

    // Shrinking the terminal does not shrink the canonical canvas, so the raw
    // canonical y values must remain at the expanded positions. What returns
    // to the original location is the coordinate inside the bottom-anchored
    // viewport.
    assert_eq!(restored_height, expanded_height);
    assert_eq!(restored.start_y, expanded.start_y);
    assert_eq!(restored.target_y, expanded.target_y);
    assert_eq!(
        restored.segment.observed_source_y,
        expanded.segment.observed_source_y
    );
    assert_eq!(
        restored.segment.observed_target_y,
        expanded.segment.observed_target_y
    );
    assert_eq!(restored.start_y - restored_top, before.start_y - before_top);
    assert_eq!(restored.target_y - restored_top, before.target_y - before_top);
    assert_eq!(
        restored.segment.observed_source_y.map(|value| value - restored_top),
        before.segment.observed_source_y.map(|value| value - before_top)
    );
    assert_eq!(
        restored.segment.observed_target_y.map(|value| value - restored_top),
        before.segment.observed_target_y.map(|value| value - before_top)
    );

    // Re-expanding merely exposes the existing canonical canvas again; it must
    // not apply the vertical growth shift a second time.
    engine.resize(20, 14);
    let reexpanded = engine.flowviz_unit_carriers.get(&id).unwrap().active.unwrap();
    assert_eq!(reexpanded.start_y, expanded.start_y);
    assert_eq!(reexpanded.target_y, expanded.target_y);
    assert_eq!(
        reexpanded.segment.observed_source_y,
        expanded.segment.observed_source_y
    );
    assert_eq!(
        reexpanded.segment.observed_target_y,
        expanded.segment.observed_target_y
    );
}

#[test]
fn unit_perceptual_flow_frame_freezes_waiting_ingress_presentation() {
    let mut engine =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(&mut engine, source, vec![grain(201)]);
    set_column(&mut engine, destination, Vec::new());
    engine.reset_unit_flowviz_from_physics();

    let (category_id, visual_y, custody) =
        engine.pop_settled_grain_with_custody(source).unwrap();
    let id = engine
        .record_flowviz_mobile_entry_with_custody(
            source,
            destination,
            category_id,
            visual_y,
            custody,
        )
        .unwrap();
    engine.seed_rolling_grain_at_y_with_motion(
        destination,
        category_id,
        ToppleDirection::Right,
        visual_y,
        Some(id),
    );
    engine.spawn(grain(999));
    let before = engine.falling_drives.front().copied().unwrap();

    let _ = engine.advance_testing_perceptual_flow_frame();

    let after = engine.falling_drives.front().copied().unwrap();
    assert_eq!(after, before);
}

#[test]
fn unit_perceptual_preserves_frozen_front_physics_and_exact_stack() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut perceptual =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    let width = frozen.columns.len();

    for drive in 0..500usize {
        let site = (drive.wrapping_mul(37).wrapping_add(11)) % width;
        direct_front_drive_and_quiesce(&mut frozen, site);
        direct_front_drive_and_quiesce(&mut perceptual, site);
        assert_eq!(perceptual.columns, frozen.columns);
        assert_eq!(perceptual.critical_slopes, frozen.critical_slopes);
        assert_eq!(perceptual.threshold_rng_state, frozen.threshold_rng_state);
        assert_eq!(perceptual.relax_rng_state, frozen.relax_rng_state);
        assert_eq!(perceptual.discharged_count(), frozen.discharged_count());
        assert_eq!(perceptual.avalanche_moves, frozen.avalanche_moves);
        assert!(perceptual.flowviz_unit_mass_matches_physics());

        let mut guard = 0usize;
        while perceptual.rolling_visual_motion_active() {
            guard = guard.saturating_add(1);
            assert!(guard < 25_000, "015C presentation failed to drain");
            let _ = perceptual.advance_flowviz_tracers();
        }
        assert_eq!(perceptual.flowviz_shadow_columns, perceptual.columns);
        assert_eq!(perceptual.flowviz_unit_misses(), 0);
    }
}

#[test]
#[ignore = "native presentation scaling evidence; run explicitly with --ignored --nocapture"]
fn unit_perceptual_native_scaling_probe() {
    use std::collections::VecDeque;
    use std::time::Instant;

    for count in [5_000usize, 10_000, 20_000, 40_000] {
        let mut engine =
            OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(160, 50, TEST_SEED);
        let width = engine.surface.grid_width_dots.max(1);
        let height = engine.surface.grid_height_dots.max(1);
        engine.flowviz_unit_carriers.clear();

        for index in 0..count {
            let id = flowviz_unit::FlowVizMotionId(index as u64 + 1);
            let x = (index % width) as f32;
            let y = ((index / width) % height) as f32;
            engine.flowviz_unit_carriers.insert(
                id,
                flowviz_unit::FlowVizUnitCarrier {
                    id,
                    category_id: grain((index % 7) as u64 + 300),
                    x,
                    y,
                    ideal_x: x,
                    ideal_y: y,
                    queued: VecDeque::new(),
                    active: None,
                    physical: flowviz_unit::UnitPhysicalState::Rolling,
                    arrived: false,
                },
            );
        }

        let relax_started = Instant::now();
        let _ = engine.relax_unit_micro_positions();
        let relax_elapsed = relax_started.elapsed();
        let raster_started = Instant::now();
        let rendered = engine.collect_unit_micro_render_cells().len();
        let raster_elapsed = raster_started.elapsed();
        eprintln!(
            "015C_SCALE carriers={count} rendered={rendered} relax_us={} raster_us={}",
            relax_elapsed.as_micros(),
            raster_elapsed.as_micros()
        );
    }
}
