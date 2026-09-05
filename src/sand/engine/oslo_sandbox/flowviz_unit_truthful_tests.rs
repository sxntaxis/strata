use super::*;

const TEST_SEED: u64 = 0x51A7_015D_7A17_F00D;

fn grain(id: u64) -> CategoryId {
    CategoryId::new(id)
}

fn direct_front_drive_and_quiesce(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 250_000, "015D direct drive failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
            break;
        }
    }
}

fn seed_overlapping_truthful_carriers(engine: &mut OsloSandboxEngine, count: usize) {
    use std::collections::VecDeque;

    engine.flowviz_unit_carriers.clear();
    let x = (engine.surface.grid_width_dots / 2) as f32;
    let y = (engine.surface.grid_height_dots / 2) as f32;
    for index in 0..count {
        let id = flowviz_unit::FlowVizMotionId(index as u64 + 1);
        engine.flowviz_unit_carriers.insert(
            id,
            flowviz_unit::FlowVizUnitCarrier {
                id,
                category_id: grain((index % 7) as u64 + 500),
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
}

#[test]
fn unit_truthful_constructor_extends_015c_without_physics_delta() {
    let perceptual = OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    let truthful = OsloSandboxEngine::new_front_unit_truthful_flowviz_vessel(20, 10, TEST_SEED);

    assert!(perceptual.flowviz_unit_perceptual_enabled());
    assert!(!perceptual.flowviz_unit_truthful_enabled());
    assert!(truthful.flowviz_unit_perceptual_enabled());
    assert!(truthful.flowviz_unit_truthful_enabled());
    assert_eq!(truthful.columns, perceptual.columns);
    assert_eq!(truthful.critical_slopes, perceptual.critical_slopes);
    assert_eq!(truthful.threshold_rng_state, perceptual.threshold_rng_state);
    assert_eq!(truthful.rain_rng_state, perceptual.rain_rng_state);
    assert_eq!(truthful.relax_rng_state, perceptual.relax_rng_state);
}

#[test]
fn unit_truthful_raster_never_relocates_a_carrier_beyond_one_dot() {
    let mut engine = OsloSandboxEngine::new_front_unit_truthful_flowviz_vessel(20, 10, TEST_SEED);
    seed_overlapping_truthful_carriers(&mut engine, 24);

    let placements = engine.collect_unit_micro_render_cells();
    assert_eq!(placements.len(), 24);
    for (id, _, x, y) in placements {
        let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
        let dx = (x as isize - carrier.x.round() as isize).abs();
        let dy = (y as isize - carrier.y.round() as isize).abs();
        assert!(dx <= 1, "truthful raster moved x by {dx}");
        assert!(dy <= 1, "truthful raster moved y by {dy}");
    }
}

#[test]
fn unit_truthful_dense_overlap_prefers_occlusion_over_spatial_fabrication() {
    let mut engine = OsloSandboxEngine::new_front_unit_truthful_flowviz_vessel(20, 10, TEST_SEED);
    seed_overlapping_truthful_carriers(&mut engine, 40);

    let placements = engine.collect_unit_micro_render_cells();
    let unique = placements
        .iter()
        .map(|(_, _, x, y)| (*x, *y))
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(placements.len(), 40, "unit accounting may not be dropped");
    assert!(
        unique.len() <= 9,
        "truthful raster must not search outside its fixed 3x3 stencil"
    );
}

#[test]
fn unit_truthful_disables_pbd_relocation_and_keeps_observed_positions_exact() {
    let mut engine = OsloSandboxEngine::new_front_unit_truthful_flowviz_vessel(20, 10, TEST_SEED);
    seed_overlapping_truthful_carriers(&mut engine, 12);
    let before = engine
        .flowviz_unit_carriers
        .values()
        .map(|carrier| {
            (
                carrier.id,
                carrier.x,
                carrier.y,
                carrier.ideal_x,
                carrier.ideal_y,
            )
        })
        .collect::<Vec<_>>();

    assert!(!engine.relax_unit_micro_positions());

    let after = engine
        .flowviz_unit_carriers
        .values()
        .map(|carrier| {
            (
                carrier.id,
                carrier.x,
                carrier.y,
                carrier.ideal_x,
                carrier.ideal_y,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(after, before);
}

#[test]
fn unit_truthful_flow_frame_advances_rain_without_committing_during_active_flow() {
    let mut engine = OsloSandboxEngine::new_front_unit_truthful_flowviz_vessel(20, 10, TEST_SEED);
    engine.spawn(grain(777));
    assert_eq!(engine.falling_drives.len(), 1);
    let y_before = engine.falling_drives.front().unwrap().y;
    let columns_before = engine.columns.clone();

    // A latent marker is enough to make the authoritative flow gate active;
    // this profile does not enable fluid physics, so it cannot mutate columns.
    let marker_site = engine.columns.len() / 2;
    engine.fluidity[marker_site] = 1;
    assert!(engine.explicit_flow_active());

    assert!(engine.advance_testing_truthful_flow_frame());
    let y_after = engine.falling_drives.front().unwrap().y;
    assert!(y_after >= y_before);
    assert_eq!(engine.columns, columns_before);
    assert_eq!(engine.falling_drives.len(), 1);
}

#[test]
fn unit_truthful_preserves_frozen_front_physics_and_exact_stack() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut truthful = OsloSandboxEngine::new_front_unit_truthful_flowviz_vessel(20, 10, TEST_SEED);
    let sites = [9usize, 10, 9, 8, 10, 11, 9, 10];

    for drive in 0..64 {
        let site = sites[drive % sites.len()];
        direct_front_drive_and_quiesce(&mut frozen, site);
        direct_front_drive_and_quiesce(&mut truthful, site);
        assert_eq!(truthful.columns, frozen.columns);
        assert_eq!(truthful.critical_slopes, frozen.critical_slopes);
        assert_eq!(truthful.threshold_rng_state, frozen.threshold_rng_state);
        assert_eq!(truthful.relax_rng_state, frozen.relax_rng_state);
        assert_eq!(truthful.discharged_count(), frozen.discharged_count());
        assert_eq!(truthful.avalanche_moves, frozen.avalanche_moves);

        let mut guard = 0usize;
        while truthful.rolling_visual_motion_active() {
            guard = guard.saturating_add(1);
            assert!(guard < 50_000, "015D presentation failed to drain");
            let _ = truthful.advance_flowviz_tracers();
        }
        assert_eq!(truthful.flowviz_shadow_columns, truthful.columns);
        assert_eq!(truthful.flowviz_unit_misses(), 0);
    }
}

#[test]
fn unit_truthful_015c_control_remains_spatially_distinct() {
    let mut perceptual =
        OsloSandboxEngine::new_front_unit_perceptual_flowviz_vessel(20, 10, TEST_SEED);
    seed_overlapping_truthful_carriers(&mut perceptual, 24);
    let placements = perceptual.collect_unit_micro_render_cells();
    let base_y = perceptual.surface.grid_height_dots / 2;
    assert!(
        placements.iter().any(|(_, _, _, y)| y.abs_diff(base_y) > 1),
        "015C control must retain the wider capacity search for regression separation"
    );
}

#[test]
#[ignore = "native truthful raster scaling evidence; run explicitly with --ignored --nocapture"]
fn unit_truthful_native_scaling_probe() {
    use std::collections::VecDeque;
    use std::time::Instant;

    for count in [5_000usize, 10_000, 20_000, 40_000] {
        let mut engine =
            OsloSandboxEngine::new_front_unit_truthful_flowviz_vessel(160, 50, TEST_SEED);
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
                    category_id: grain((index % 7) as u64 + 600),
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
        let relaxed = engine.relax_unit_micro_positions();
        let relax_elapsed = relax_started.elapsed();
        let raster_started = Instant::now();
        let rendered = engine.collect_unit_micro_render_cells().len();
        let raster_elapsed = raster_started.elapsed();
        eprintln!(
            "015D_SCALE carriers={count} rendered={rendered} relaxed={relaxed} relax_us={} raster_us={}",
            relax_elapsed.as_micros(),
            raster_elapsed.as_micros()
        );
        assert_eq!(rendered, count);
        assert!(!relaxed);
    }
}
