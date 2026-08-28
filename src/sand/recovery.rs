use std::{collections::HashSet, time::Duration};

use crate::domain::CategoryId;

use super::{PendingGrainRun, SandEngine, SandState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PeriodicAdvance {
    pub due_events: usize,
    pub remainder: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveredSediment {
    pub state: SandState,
    pub spawn_remainder: Duration,
    pub physics_remainder: Duration,
    pub added_grains: usize,
    pub skipped_physics_events: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveryTiming {
    pub elapsed: Duration,
    pub spawn_accumulator: Duration,
    pub physics_accumulator: Duration,
    pub spawn_period: Duration,
    pub physics_period: Duration,
}

pub(crate) fn advance_periodic(
    accumulator: Duration,
    elapsed: Duration,
    period: Duration,
) -> Result<PeriodicAdvance, String> {
    let period_nanos = period.as_nanos();
    if period_nanos == 0 {
        return Err("periodic recovery interval must be non-zero".to_string());
    }
    if accumulator >= period {
        return Err("periodic recovery accumulator must be smaller than its period".to_string());
    }

    let total_nanos = accumulator
        .as_nanos()
        .checked_add(elapsed.as_nanos())
        .ok_or_else(|| "periodic recovery duration overflow".to_string())?;
    let due_events = usize::try_from(total_nanos / period_nanos)
        .map_err(|_| "periodic recovery event count exceeds the supported range".to_string())?;
    let remainder_nanos = total_nanos % period_nanos;
    let remainder_seconds = u64::try_from(remainder_nanos / 1_000_000_000)
        .map_err(|_| "periodic recovery remainder exceeds Duration".to_string())?;
    let remainder_subsec = u32::try_from(remainder_nanos % 1_000_000_000)
        .map_err(|_| "periodic recovery remainder is invalid".to_string())?;

    Ok(PeriodicAdvance {
        due_events,
        remainder: Duration::new(remainder_seconds, remainder_subsec),
    })
}

pub(crate) fn recover_detached_sediment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    timing: RecoveryTiming,
) -> Result<RecoveredSediment, String> {
    recover_sediment(
        base_state,
        valid_category_ids,
        active_category_id,
        timing,
        CanvasPolicy::RequireInitialized,
    )
}

pub(crate) fn settle_transition_sediment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    timing: RecoveryTiming,
) -> Result<RecoveredSediment, String> {
    recover_sediment(
        base_state,
        valid_category_ids,
        active_category_id,
        timing,
        CanvasPolicy::AllowUninitialized,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanvasPolicy {
    RequireInitialized,
    AllowUninitialized,
}

fn recover_sediment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    timing: RecoveryTiming,
    canvas_policy: CanvasPolicy,
) -> Result<RecoveredSediment, String> {
    validate_sediment_state(base_state, valid_category_ids, canvas_policy)?;
    if !valid_category_ids.contains(&active_category_id) {
        return Err(format!(
            "recovery active category {} does not exist",
            active_category_id.0
        ));
    }

    let spawn = advance_periodic(
        timing.spawn_accumulator,
        timing.elapsed,
        timing.spawn_period,
    )?;
    let physics = advance_periodic(
        timing.physics_accumulator,
        timing.elapsed,
        timing.physics_period,
    )?;

    let mut state = if base_state.grid_width == 0 && base_state.grid_height == 0 {
        migrate_uninitialized_state(base_state)?
    } else {
        // Recovery must restore canonical coordinates without projecting them into a
        // display viewport. The live renderer expands to its terminal separately.
        let mut engine = SandEngine::new(0, 0);
        engine.restore_state(base_state, valid_category_ids)?;
        engine.snapshot_state()
    };
    let pending_mass = state
        .pending_runs
        .iter()
        .try_fold(0usize, |total, run| total.checked_add(run.count));
    let Some(existing_mass) =
        pending_mass.and_then(|pending| state.grains.len().checked_add(pending))
    else {
        return Err("recovery sediment mass exceeds the supported range".to_string());
    };
    if existing_mass.checked_add(spawn.due_events).is_none() {
        return Err("recovered sediment mass exceeds the supported range".to_string());
    }
    if spawn.due_events > 0 {
        if let Some(last) = state.pending_runs.last_mut()
            && last.category_id == active_category_id.0
        {
            last.count = last
                .count
                .checked_add(spawn.due_events)
                .ok_or_else(|| "recovered pending run exceeds the supported range".to_string())?;
        } else {
            state.pending_runs.push(PendingGrainRun {
                category_id: active_category_id.0,
                count: spawn.due_events,
            });
        }
    }

    Ok(RecoveredSediment {
        state,
        spawn_remainder: spawn.remainder,
        physics_remainder: physics.remainder,
        added_grains: spawn.due_events,
        skipped_physics_events: physics.due_events,
    })
}

fn migrate_uninitialized_state(base_state: &SandState) -> Result<SandState, String> {
    let mut state = base_state.clone();
    if state.version == SandState::LEGACY_VERSION {
        let mut runs: Vec<PendingGrainRun> = Vec::new();
        for category_id in state.pending_grains.drain(..) {
            if let Some(last) = runs.last_mut()
                && last.category_id == category_id
            {
                last.count = last
                    .count
                    .checked_add(1)
                    .ok_or_else(|| "legacy pending run exceeds the supported range".to_string())?;
            } else {
                runs.push(PendingGrainRun {
                    category_id,
                    count: 1,
                });
            }
        }
        state.pending_runs = runs;
    }
    if state.version == SandState::LEGACY_VERSION
        || state.version == SandState::COMPRESSED_PENDING_VERSION
        || state.version == SandState::ORGANIC_VERSION
        || state.version == SandState::REGIONAL_AVALANCHE_VERSION
    {
        state.version = SandState::VERSION;
        if base_state.version != SandState::ORGANIC_VERSION
            && base_state.version != SandState::REGIONAL_AVALANCHE_VERSION
        {
            state.ingress_focus_x = None;
        }
        state.active_avalanche_columns.clear();
        state.mobilized_grains.clear();
    }
    Ok(state)
}

fn validate_sediment_state(
    state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    canvas_policy: CanvasPolicy,
) -> Result<(), String> {
    if state.version != SandState::VERSION
        && state.version != SandState::REGIONAL_AVALANCHE_VERSION
        && state.version != SandState::COMPRESSED_PENDING_VERSION
        && state.version != SandState::ORGANIC_VERSION
        && state.version != SandState::LEGACY_VERSION
    {
        return Err(format!(
            "unsupported sediment state schema {}",
            state.version
        ));
    }
    if (state.grid_width == 0) != (state.grid_height == 0) {
        return Err("recovery sediment canvas has only one zero dimension".to_string());
    }
    if state.grid_width == 0 && state.grid_height == 0 {
        if canvas_policy == CanvasPolicy::RequireInitialized {
            return Err("recovery sediment canvas must be non-empty".to_string());
        }
        if !state.grains.is_empty() {
            return Err("uninitialized sediment canvas contains placed grains".to_string());
        }
    }
    if (state.version == SandState::VERSION
        || state.version == SandState::REGIONAL_AVALANCHE_VERSION
        || state.version == SandState::COMPRESSED_PENDING_VERSION)
        && !state.pending_grains.is_empty()
    {
        return Err("compressed sediment state contains legacy pending grains".to_string());
    }
    if state.version == SandState::LEGACY_VERSION && !state.pending_runs.is_empty() {
        return Err("version 1 sediment state contains compressed pending runs".to_string());
    }
    if state.version != SandState::VERSION
        && state.version != SandState::REGIONAL_AVALANCHE_VERSION
        && state.version != SandState::ORGANIC_VERSION
        && state.ingress_focus_x.is_some()
    {
        return Err("pre-organic sediment state contains an ingress focus".to_string());
    }
    if let Some(focus_x) = state.ingress_focus_x
        && (state.grid_width == 0 || focus_x >= state.grid_width)
    {
        return Err(format!(
            "recovery ingress focus {focus_x} is outside {}-column canvas",
            state.grid_width
        ));
    }
    if state.version != SandState::REGIONAL_AVALANCHE_VERSION
        && !state.active_avalanche_columns.is_empty()
    {
        return Err("only v4 recovery state may contain active avalanche columns".to_string());
    }
    if state.version != SandState::VERSION && !state.mobilized_grains.is_empty() {
        return Err("pre-v5 recovery state contains mobilized grain coordinates".to_string());
    }
    if state
        .active_avalanche_columns
        .windows(2)
        .any(|columns| columns[0] >= columns[1])
        || state
            .active_avalanche_columns
            .iter()
            .any(|&x| x >= state.grid_width)
    {
        return Err("invalid active avalanche columns in recovery state".to_string());
    }

    if state.mobilized_grains.windows(2).any(|coordinates| {
        (coordinates[0].y, coordinates[0].x) >= (coordinates[1].y, coordinates[1].x)
    }) {
        return Err(
            "recovery mobilized grain coordinates must be strictly row-major sorted".to_string(),
        );
    }
    if state
        .mobilized_grains
        .iter()
        .any(|coordinate| coordinate.x >= state.grid_width || coordinate.y >= state.grid_height)
    {
        return Err(
            "recovery mobilized grain coordinate is outside the canonical grid".to_string(),
        );
    }

    let mut occupied = HashSet::with_capacity(state.grains.len());
    for grain in &state.grains {
        if grain.x >= state.grid_width || grain.y >= state.grid_height {
            return Err(format!(
                "recovery grain ({}, {}) is outside {}x{} canvas",
                grain.x, grain.y, state.grid_width, state.grid_height
            ));
        }
        if !occupied.insert((grain.x, grain.y)) {
            return Err(format!(
                "recovery sediment contains duplicate coordinate ({}, {})",
                grain.x, grain.y
            ));
        }
        let category_id = CategoryId::new(grain.category_id);
        if !valid_category_ids.contains(&category_id) {
            return Err(format!(
                "recovery grain references unavailable category {}",
                grain.category_id
            ));
        }
    }

    if state
        .mobilized_grains
        .iter()
        .any(|coordinate| !occupied.contains(&(coordinate.x, coordinate.y)))
    {
        return Err(
            "recovery mobilized grain coordinate does not reference a placed grain".to_string(),
        );
    }

    for category_id in &state.pending_grains {
        if !valid_category_ids.contains(&CategoryId::new(*category_id)) {
            return Err(format!(
                "recovery pending grain references unavailable category {category_id}"
            ));
        }
    }
    for run in &state.pending_runs {
        if !valid_category_ids.contains(&CategoryId::new(run.category_id)) {
            return Err(format!(
                "recovery pending run references unavailable category {}",
                run.category_id
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CanvasPolicy, PeriodicAdvance, RecoveryTiming, advance_periodic, recover_detached_sediment,
        settle_transition_sediment, validate_sediment_state,
    };
    use crate::{
        domain::CategoryId,
        sand::{PendingGrainRun, SandState, SandStateGrain},
    };
    use std::{collections::HashSet, time::Duration};

    fn categories() -> HashSet<CategoryId> {
        HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)])
    }

    fn base_state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 4,
            grid_height: 4,
            grains: vec![
                SandStateGrain {
                    x: 0,
                    y: 3,
                    category_id: 1,
                },
                SandStateGrain {
                    x: 2,
                    y: 2,
                    category_id: 2,
                },
            ],
            frame_count: 19,
            sweep_left_to_right: false,
            rng_state: 77,
            ingress_focus_x: Some(1),
            pending_grains: Vec::new(),
            pending_runs: vec![PendingGrainRun {
                category_id: 1,
                count: 3,
            }],
            active_avalanche_columns: Vec::new(),
            mobilized_grains: Vec::new(),
        }
    }

    #[test]
    fn long_gap_is_counted_without_iterative_replay() {
        assert_eq!(
            advance_periodic(
                Duration::ZERO,
                Duration::from_secs(1_000_000_000),
                Duration::from_secs(1),
            )
            .unwrap(),
            PeriodicAdvance {
                due_events: 1_000_000_000,
                remainder: Duration::ZERO,
            }
        );
    }

    #[test]
    fn accumulator_and_remainder_are_exact() {
        assert_eq!(
            advance_periodic(
                Duration::from_millis(750),
                Duration::from_millis(2_500),
                Duration::from_secs(1),
            )
            .unwrap(),
            PeriodicAdvance {
                due_events: 3,
                remainder: Duration::from_millis(250),
            }
        );
    }

    #[test]
    fn zero_period_and_full_accumulator_are_rejected() {
        assert!(advance_periodic(Duration::ZERO, Duration::from_secs(1), Duration::ZERO).is_err());
        assert!(
            advance_periodic(
                Duration::from_secs(1),
                Duration::ZERO,
                Duration::from_secs(1),
            )
            .is_err()
        );
    }

    #[test]
    fn recovery_preserves_existing_topology_and_adds_exact_mass() {
        let base = base_state();
        let recovered = recover_detached_sediment(
            &base,
            &categories(),
            CategoryId::new(2),
            RecoveryTiming {
                elapsed: Duration::from_millis(2_500),
                spawn_accumulator: Duration::from_millis(750),
                physics_accumulator: Duration::from_millis(20),
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();

        assert_eq!(recovered.added_grains, 3);
        assert_eq!(recovered.spawn_remainder, Duration::from_millis(250));
        assert_eq!(recovered.physics_remainder, Duration::from_millis(20));
        assert_eq!(recovered.skipped_physics_events, 50);
        for original in &base.grains {
            assert!(recovered.state.grains.contains(original));
        }
        assert_eq!(recovered.state.frame_count, base.frame_count);
        assert_eq!(
            recovered.state.sweep_left_to_right,
            base.sweep_left_to_right
        );
        assert_eq!(recovered.state.rng_state, base.rng_state);
        assert_eq!(recovered.state.ingress_focus_x, base.ingress_focus_x);
        let before_mass = base.grains.len() + 3;
        let after_mass = recovered.state.grains.len()
            + recovered
                .state
                .pending_runs
                .iter()
                .map(|run| run.count)
                .sum::<usize>();
        assert_eq!(after_mass, before_mass + 3);
    }

    #[test]
    fn version_four_checkpoint_recovers_into_v5_without_regional_activity() {
        let mut base = base_state();
        base.version = SandState::REGIONAL_AVALANCHE_VERSION;
        base.active_avalanche_columns = vec![1];

        let recovered = recover_detached_sediment(
            &base,
            &categories(),
            CategoryId::new(1),
            RecoveryTiming {
                elapsed: Duration::ZERO,
                spawn_accumulator: Duration::ZERO,
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();

        assert_eq!(recovered.state.version, SandState::VERSION);
        assert!(recovered.state.active_avalanche_columns.is_empty());
        assert_eq!(recovered.state.ingress_focus_x, base.ingress_focus_x);
        let mut expected_grains = base.grains.clone();
        expected_grains.sort_by_key(|grain| (grain.y, grain.x));
        assert_eq!(recovered.state.grains, expected_grains);
    }

    #[test]
    fn malformed_mobility_is_rejected_before_recovery_installation() {
        let base = base_state();
        let cases = [
            {
                let mut state = base.clone();
                state.mobilized_grains = vec![
                    super::super::SandStateCoordinate { x: 0, y: 3 },
                    super::super::SandStateCoordinate { x: 2, y: 2 },
                ];
                state
            },
            {
                let mut state = base.clone();
                state.mobilized_grains = vec![
                    super::super::SandStateCoordinate { x: 0, y: 3 },
                    super::super::SandStateCoordinate { x: 0, y: 3 },
                ];
                state
            },
            {
                let mut state = base.clone();
                state.mobilized_grains = vec![super::super::SandStateCoordinate { x: 4, y: 0 }];
                state
            },
            {
                let mut state = base.clone();
                state.mobilized_grains = vec![super::super::SandStateCoordinate { x: 1, y: 1 }];
                state
            },
        ];
        for (index, state) in cases.into_iter().enumerate() {
            assert!(
                validate_sediment_state(&state, &categories(), CanvasPolicy::RequireInitialized)
                    .is_err(),
                "malformed mobility case {index} was accepted"
            );
        }
    }

    #[test]
    fn version_two_checkpoint_recovers_into_current_organic_state() {
        let mut base = base_state();
        base.version = SandState::COMPRESSED_PENDING_VERSION;
        base.ingress_focus_x = None;

        let recovered = recover_detached_sediment(
            &base,
            &categories(),
            CategoryId::new(1),
            RecoveryTiming {
                elapsed: Duration::ZERO,
                spawn_accumulator: Duration::ZERO,
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();

        assert_eq!(recovered.state.version, SandState::VERSION);
        assert_eq!(recovered.state.ingress_focus_x, None);
        let mut expected_grains = base.grains.clone();
        expected_grains.sort_by_key(|grain| (grain.y, grain.x));
        assert_eq!(recovered.state.grains, expected_grains);
        assert_eq!(recovered.state.pending_runs, base.pending_runs);
    }

    #[test]
    fn extreme_gap_is_one_compressed_run_when_ingress_is_full() {
        let mut base = base_state();
        base.grains = (0..4)
            .map(|x| SandStateGrain {
                x,
                y: 0,
                category_id: 1,
            })
            .collect();
        base.pending_runs.clear();
        let recovered = recover_detached_sediment(
            &base,
            &categories(),
            CategoryId::new(2),
            RecoveryTiming {
                elapsed: Duration::from_secs(1_000_000_000),
                spawn_accumulator: Duration::ZERO,
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();
        assert_eq!(recovered.state.pending_runs.len(), 1);
        assert_eq!(recovered.state.pending_runs[0].category_id, 2);
        assert_eq!(recovered.state.pending_runs[0].count, 1_000_000_000);
        assert_eq!(recovered.state.grains, base.grains);
    }

    #[test]
    fn transition_settlement_preserves_mass_on_uninitialized_canvas() {
        let state = SandState {
            version: SandState::VERSION,
            grid_width: 0,
            grid_height: 0,
            grains: Vec::new(),
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 1,
            ingress_focus_x: None,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
            active_avalanche_columns: Vec::new(),
            mobilized_grains: Vec::new(),
        };
        let timing = RecoveryTiming {
            elapsed: Duration::from_millis(100),
            spawn_accumulator: Duration::from_millis(900),
            physics_accumulator: Duration::ZERO,
            spawn_period: Duration::from_secs(1),
            physics_period: Duration::from_millis(50),
        };
        assert!(
            recover_detached_sediment(&state, &categories(), CategoryId::new(1), timing).is_err()
        );
        let settled =
            settle_transition_sediment(&state, &categories(), CategoryId::new(1), timing).unwrap();
        assert_eq!(settled.state.grid_width, 0);
        assert_eq!(settled.state.grid_height, 0);
        assert!(settled.state.grains.is_empty());
        assert_eq!(settled.state.pending_runs.len(), 1);
        assert_eq!(settled.state.pending_runs[0].category_id, 1);
        assert_eq!(settled.state.pending_runs[0].count, 1);
        assert_eq!(settled.spawn_remainder, Duration::ZERO);
    }

    #[test]
    fn malformed_checkpoint_state_fails_closed() {
        let mut invalid = base_state();
        invalid.grains.push(invalid.grains[0].clone());
        assert!(
            recover_detached_sediment(
                &invalid,
                &categories(),
                CategoryId::new(1),
                RecoveryTiming {
                    elapsed: Duration::ZERO,
                    spawn_accumulator: Duration::ZERO,
                    physics_accumulator: Duration::ZERO,
                    spawn_period: Duration::from_secs(1),
                    physics_period: Duration::from_millis(50),
                },
            )
            .is_err()
        );
    }
}
