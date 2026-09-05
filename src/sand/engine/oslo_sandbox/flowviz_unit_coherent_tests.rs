use super::*;

const TEST_SEED: u64 = 0x51A7_015E_C0DE_5EED;

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

fn seed_one_observed_rolling_hop(
    engine: &mut OsloSandboxEngine,
) -> (flowviz_unit::FlowVizMotionId, usize, usize) {
    let source = engine.columns.len() / 2;
    let destination = source + 1;
    set_column(engine, source, vec![grain(11)]);
    set_column(engine, destination, Vec::new());
    set_column(engine, destination + 1, Vec::new());
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
    (id, source, destination)
}

#[test]
fn unit_coherent_constructor_extends_015d_without_physics_delta() {
    let truthful = OsloSandboxEngine::new_front_unit_truthful_flowviz_vessel(20, 10, TEST_SEED);
    let coherent = OsloSandboxEngine::new_front_unit_coherent_flowviz_vessel(20, 10, TEST_SEED);

    assert!(truthful.flowviz_unit_truthful_enabled());
    assert!(!truthful.flowviz_unit_coherent_enabled());
    assert!(coherent.flowviz_unit_truthful_enabled());
    assert!(coherent.flowviz_unit_coherent_enabled());
    assert_eq!(coherent.columns, truthful.columns);
    assert_eq!(coherent.critical_slopes, truthful.critical_slopes);
    assert_eq!(coherent.threshold_rng_state, truthful.threshold_rng_state);
    assert_eq!(coherent.rain_rng_state, truthful.rain_rng_state);
    assert_eq!(coherent.relax_rng_state, truthful.relax_rng_state);
}

#[test]
fn unit_coherent_barrier_spends_visual_debt_before_next_physics_hop() {
    let mut engine = OsloSandboxEngine::new_front_unit_coherent_flowviz_vessel(20, 10, TEST_SEED);
    let (id, _, destination) = seed_one_observed_rolling_hop(&mut engine);

    assert!(engine.flowviz_unit_coherent_batch_pending());
    let moves_before = engine.avalanche_moves;
    let rolling_site_before = engine.rolling_grains.front().unwrap().site;
    let columns_before = engine.columns.clone();

    assert!(engine.advance_testing_coherent_flow_frame());

    assert_eq!(engine.avalanche_moves, moves_before);
    assert_eq!(engine.rolling_grains.front().unwrap().site, rolling_site_before);
    assert_eq!(engine.columns, columns_before);
    assert_eq!(rolling_site_before, destination);
    assert!(engine.flowviz_unit_carriers.contains_key(&id));
}

#[test]
fn unit_coherent_caught_up_rolling_carrier_releases_the_barrier() {
    let mut engine = OsloSandboxEngine::new_front_unit_coherent_flowviz_vessel(20, 10, TEST_SEED);
    let (id, _, destination) = seed_one_observed_rolling_hop(&mut engine);

    let mut guard = 0usize;
    while engine.flowviz_unit_coherent_batch_pending() {
        guard = guard.saturating_add(1);
        assert!(guard < 100, "015E first observed hop failed to drain");
        let _ = engine.advance_testing_truthful_visual_frame();
    }

    let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
    assert!(carrier.active.is_none());
    assert!(carrier.queued.is_empty());
    assert_eq!(carrier.physical, flowviz_unit::UnitPhysicalState::Rolling);
    assert_eq!(engine.rolling_grains.front().unwrap().site, destination);
    assert_eq!(engine.flowviz_unit_presentation_backlog(), 0);

    let moves_before = engine.avalanche_moves;
    assert!(engine.advance_testing_coherent_flow_frame());
    assert!(engine.avalanche_moves > moves_before);
    assert!(engine.flowviz_unit_coherent_batch_pending());
}

#[test]
fn unit_coherent_never_accumulates_a_second_physics_epoch_while_visual_debt_exists() {
    let mut engine = OsloSandboxEngine::new_front_unit_coherent_flowviz_vessel(20, 10, TEST_SEED);
    let (id, _, _) = seed_one_observed_rolling_hop(&mut engine);

    let initial_segments = engine.flowviz_unit_segments();
    let moves_before = engine.avalanche_moves;
    for _ in 0..3 {
        let _ = engine.advance_testing_coherent_flow_frame();
        if engine.flowviz_unit_coherent_batch_pending() {
            assert_eq!(engine.avalanche_moves, moves_before);
            assert_eq!(engine.flowviz_unit_segments(), initial_segments);
            let carrier = engine.flowviz_unit_carriers.get(&id).unwrap();
            assert!(carrier.queued.len() <= 1);
        } else {
            break;
        }
    }
}

#[test]
fn unit_coherent_shadow_plus_carrier_mass_stays_exact_during_batch_interpolation() {
    let mut engine = OsloSandboxEngine::new_front_unit_coherent_flowviz_vessel(20, 10, TEST_SEED);
    let (_id, _, _) = seed_one_observed_rolling_hop(&mut engine);

    assert!(engine.flowviz_unit_mass_matches_physics());
    for _ in 0..8 {
        let _ = engine.advance_testing_truthful_visual_frame();
        assert!(engine.flowviz_unit_mass_matches_physics());
        if !engine.flowviz_unit_coherent_batch_pending() {
            break;
        }
    }
}

fn direct_frozen_drive_and_quiesce(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 250_000, "015E frozen control failed to quiesce");
        let mut changed = engine.advance_rolling_grains();
        changed |= engine.topple_one_active_site();
        if !changed && engine.active_sites.is_empty() && !engine.explicit_flow_active() {
            break;
        }
    }
}

fn direct_coherent_drive_and_drain(engine: &mut OsloSandboxEngine, site: usize) {
    engine.push_settled_grain_at_visual_y(site, grain(1), None);
    engine.mirror_flowviz_settled_push(site, grain(1));
    engine.total_generated = engine.total_generated.saturating_add(1);
    engine.enqueue_neighborhood(site);

    let mut guard = 0usize;
    loop {
        guard = guard.saturating_add(1);
        assert!(guard < 500_000, "015E coherent drive failed to drain");
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
fn unit_coherent_lockstep_preserves_frozen_front_physics_and_exact_final_stack() {
    let mut frozen = OsloSandboxEngine::new_front_vessel(20, 10, TEST_SEED);
    let mut coherent = OsloSandboxEngine::new_front_unit_coherent_flowviz_vessel(20, 10, TEST_SEED);
    let sites = [9usize, 10, 9, 8, 10, 11, 9, 10];

    for drive in 0..32 {
        let site = sites[drive % sites.len()];
        direct_frozen_drive_and_quiesce(&mut frozen, site);
        direct_coherent_drive_and_drain(&mut coherent, site);

        assert_eq!(coherent.columns, frozen.columns);
        assert_eq!(coherent.critical_slopes, frozen.critical_slopes);
        assert_eq!(coherent.threshold_rng_state, frozen.threshold_rng_state);
        assert_eq!(coherent.relax_rng_state, frozen.relax_rng_state);
        assert_eq!(coherent.discharged_count(), frozen.discharged_count());
        assert_eq!(coherent.avalanche_moves, frozen.avalanche_moves);
        assert_eq!(coherent.flowviz_shadow_columns, coherent.columns);
        assert_eq!(coherent.flowviz_unit_misses(), 0);
    }
}

#[test]
#[ignore = "native coherent one-hop batch scaling evidence; run explicitly with --ignored --nocapture"]
fn unit_coherent_native_one_hop_batch_probe() {
    use std::collections::VecDeque;
    use std::time::Instant;

    for count in [5_000usize, 10_000, 20_000, 40_000] {
        let mut engine =
            OsloSandboxEngine::new_front_unit_coherent_flowviz_vessel(160, 50, TEST_SEED);
        let width = engine.surface.grid_width_dots.max(2);
        let height = engine.surface.grid_height_dots.max(1);
        engine.flowviz_unit_carriers.clear();
        engine.flowviz_unit_next_sequence = count as u64 + 2;

        for index in 0..count {
            let id = flowviz_unit::FlowVizMotionId(index as u64 + 1);
            let source = index % (width - 1);
            let destination = source + 1;
            let y = ((index / width) % height) as f32;
            let sequence = index as u64 + 1;
            engine.flowviz_unit_carriers.insert(
                id,
                flowviz_unit::FlowVizUnitCarrier {
                    id,
                    category_id: grain((index % 7) as u64 + 700),
                    x: source as f32,
                    y,
                    ideal_x: source as f32,
                    ideal_y: y,
                    queued: VecDeque::from([flowviz_unit::UnitSegment {
                        sequence,
                        source,
                        destination: Some(destination),
                        observed_source_y: Some(y),
                        observed_target_y: Some(y),
                    }]),
                    active: None,
                    physical: flowviz_unit::UnitPhysicalState::Rolling,
                    arrived: false,
                },
            );
        }

        let started = Instant::now();
        let mut steps = 0usize;
        while engine.flowviz_unit_coherent_batch_pending() {
            steps = steps.saturating_add(1);
            assert!(steps <= 6, "015E one-hop batch exceeded six presentation steps");
            let _ = engine.advance_flowviz_tracers();
        }
        let elapsed = started.elapsed();
        eprintln!(
            "015E_BATCH carriers={count} steps={steps} elapsed_us={}",
            elapsed.as_micros()
        );
        assert_eq!(steps, 5);
    }
}
