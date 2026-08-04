from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text()


def write(path: str, content: str) -> None:
    Path(path).write_text(content)


def insert_before(path: str, marker: str, insertion: str) -> None:
    content = read(path)
    if insertion.strip() in content:
        return
    if marker not in content:
        raise SystemExit(f"marker missing in {path}: {marker[:100]!r}")
    write(path, content.replace(marker, insertion + marker, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    content = read(path)
    start_index = content.find(start)
    if start_index < 0:
        raise SystemExit(f"start marker missing in {path}: {start[:100]!r}")
    end_index = content.find(end, start_index)
    if end_index < 0:
        raise SystemExit(f"end marker missing in {path}: {end[:100]!r}")
    write(path, content[:start_index] + replacement + content[end_index:])


settlement_helpers = r'''#[derive(Debug, Clone, PartialEq, Eq)]
struct TransitionSedimentSettlement {
    state: SandState,
    spawn_remainder: Duration,
    physics_remainder: Duration,
    added_grains: usize,
    skipped_physics_events: usize,
}

fn settle_transition_sediment_segment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    elapsed: Duration,
    spawn_accumulator: Duration,
    physics_accumulator: Duration,
    spawn_period: Duration,
    physics_period: Duration,
) -> Result<TransitionSedimentSettlement, String> {
    let recovered = recover_detached_sediment(
        base_state,
        valid_category_ids,
        active_category_id,
        RecoveryTiming {
            elapsed,
            spawn_accumulator,
            physics_accumulator,
            spawn_period,
            physics_period,
        },
    )?;
    Ok(TransitionSedimentSettlement {
        state: recovered.state,
        spawn_remainder: recovered.spawn_remainder,
        physics_remainder: recovered.physics_remainder,
        added_grains: recovered.added_grains,
        skipped_physics_events: recovered.skipped_physics_events,
    })
}

'''
insert_before("src/app.rs", "fn valid_category_ids_for_catalog(", settlement_helpers)

finish_methods = r'''    fn prepare_active_finish_for_exit(&mut self) -> Option<usize> {
        let finished_at_utc = Utc::now();
        if let Err(error) = self.settle_transition_boundary(finished_at_utc) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::FinishAndExit,
                Err(error),
            );
            return None;
        }
        if self.sqlite_database_path.is_some() {
            return self.end_active_session_at(
                finished_at_utc,
                SessionClockMode::LiveMonotonic,
            );
        }
        let interval = match self
            .reconciled_active_interval(finished_at_utc, SessionClockMode::LiveMonotonic)
        {
            Ok(interval) => interval,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveFinish,
                    RecoveryAction::FinishAndExit,
                    Err(error),
                );
                return None;
            }
        };
        let previous_tracker = self.time_tracker.clone();
        let previous_session = self.session.clone();
        let previous_category_id = self.time_tracker.active_category_id();
        let previous_description = self
            .time_tracker
            .category_description_by_id(previous_category_id)
            .unwrap_or_default()
            .to_string();
        let Some(previous_started_at_utc) = self.session.active_session_started_at_utc else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::FinishAndExit,
                Err("legacy runtime has no active UTC start timestamp to finish".to_string()),
            );
            return None;
        };
        let mut prepared_checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::FinishAndExit,
                    Err(error),
                );
                return None;
            }
        };
        let previous_session_count = self.time_tracker.sessions.len();
        let ended_civil = civil_time_for_utc(interval.ended_at_utc);
        let result = self
            .time_tracker
            .end_session_with_elapsed_at_local(interval.elapsed_seconds, ended_civil);
        self.session.active_session_started_at_utc = None;
        let completed_session = self
            .time_tracker
            .sessions
            .get(previous_session_count)
            .map(LegacySessionReceipt::from_session);
        let expected_identity = format!(
            "legacy:{}:{}",
            previous_category_id.0,
            previous_started_at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true)
        );
        let receipt = LegacyFinishReceipt {
            version: LegacyFinishReceipt::VERSION,
            operation_id: transition_operation_id(
                "legacy-finish",
                &expected_identity,
                interval.ended_at_utc,
                "complete",
            ),
            expected_previous_category_id: previous_category_id.0,
            expected_previous_description: previous_description,
            expected_previous_started_at_utc: previous_started_at_utc,
            finished_at_utc: interval.ended_at_utc,
            completed_session,
        };
        prepared_checkpoint.legacy_finish = Some(receipt);
        if let Err(error) =
            storage::write_json_atomic(&storage::get_detached_runtime_path(), &prepared_checkpoint)
        {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::FinishAndExit,
                Err(error),
            );
            return None;
        }
        result
    }

'''
replace_between(
    "src/app.rs",
    "    fn end_active_session_now(&mut self) -> Option<usize> {",
    "    fn end_active_session_at(",
    finish_methods,
)

mutation_methods = r'''    fn settle_simulation_segment_to(
        &mut self,
        target_utc: DateTime<Utc>,
    ) -> Result<(), String> {
        if target_utc <= self.simulation.simulation_time_utc {
            return Ok(());
        }
        let elapsed = (target_utc - self.simulation.simulation_time_utc)
            .to_std()
            .map_err(|error| error.to_string())?;
        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        let settlement = settle_transition_sediment_segment(
            &self.sand_engine.snapshot_state(),
            &valid_category_ids,
            self.time_tracker.active_category_id(),
            elapsed,
            self.simulation.spawn_accumulator,
            self.simulation.physics_accumulator,
            Duration::from_millis(TIME_SETTINGS.tick_ms),
            Duration::from_millis(TIME_SETTINGS.physics_ms),
        )?;
        self.sand_engine
            .restore_state(&settlement.state, &valid_category_ids);
        self.simulation.spawn_accumulator = settlement.spawn_remainder;
        self.simulation.physics_accumulator = settlement.physics_remainder;
        self.simulation.simulation_time_utc = target_utc;
        if settlement.added_grains > 0 || settlement.skipped_physics_events > 0 {
            self.render_needed = true;
        }
        Ok(())
    }

    fn settle_transition_boundary(
        &mut self,
        boundary_utc: DateTime<Utc>,
    ) -> Result<(), String> {
        loop {
            let Some(next) = self.simulation.pending_mutations.front().cloned() else {
                break;
            };
            if next.execute_at_utc > boundary_utc {
                break;
            }
            self.settle_simulation_segment_to(next.execute_at_utc)?;
            self.simulation.pending_mutations.pop_front();
            self.apply_mutation_at(
                next.mutation,
                next.execute_at_utc,
                SessionClockMode::HistoricalWall,
            );
            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
        }
        self.settle_simulation_segment_to(boundary_utc)
    }

    fn queue_or_apply_mutation(&mut self, mutation: QueuedMutation) {
        let scheduled_at_utc = Utc::now();
        if self.is_catching_up() || !self.simulation.pending_mutations.is_empty() {
            self.simulation
                .pending_mutations
                .push_back(QueuedMutationEvent {
                    execute_at_utc: scheduled_at_utc,
                    mutation,
                });
        } else if let Err(error) = self.settle_transition_boundary(scheduled_at_utc) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
        } else {
            self.apply_mutation_at(
                mutation,
                scheduled_at_utc,
                SessionClockMode::LiveMonotonic,
            );
        }
        self.render_needed = true;
    }

'''
replace_between(
    "src/app.rs",
    "    fn queue_or_apply_mutation(&mut self, mutation: QueuedMutation) {",
    "    fn apply_mutation_at(",
    mutation_methods,
)

transition_tests = r'''#[cfg(test)]
mod transition_edge_tests {
    use std::{collections::HashSet, time::Duration};

    use super::settle_transition_sediment_segment;
    use crate::{
        domain::CategoryId,
        sand::{SandState, SandStateGrain},
    };

    fn categories() -> HashSet<CategoryId> {
        HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)])
    }

    fn empty_state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: Vec::new(),
            frame_count: 7,
            sweep_left_to_right: true,
            rng_state: 11,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        }
    }

    #[test]
    fn exact_boundary_grain_belongs_to_outgoing_category() {
        let outgoing = settle_transition_sediment_segment(
            &empty_state(),
            &categories(),
            CategoryId::new(1),
            Duration::from_millis(100),
            Duration::from_millis(900),
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
        .unwrap();
        assert_eq!(outgoing.added_grains, 1);
        assert_eq!(outgoing.spawn_remainder, Duration::ZERO);
        assert_eq!(outgoing.state.pending_runs.len(), 1);
        assert_eq!(outgoing.state.pending_runs[0].category_id, 1);
        assert_eq!(outgoing.state.pending_runs[0].count, 1);

        let resulting = settle_transition_sediment_segment(
            &outgoing.state,
            &categories(),
            CategoryId::new(2),
            Duration::from_secs(1),
            outgoing.spawn_remainder,
            outgoing.physics_remainder,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
        .unwrap();
        assert_eq!(resulting.state.pending_runs.len(), 2);
        assert_eq!(resulting.state.pending_runs[0].category_id, 1);
        assert_eq!(resulting.state.pending_runs[0].count, 1);
        assert_eq!(resulting.state.pending_runs[1].category_id, 2);
        assert_eq!(resulting.state.pending_runs[1].count, 1);
    }

    #[test]
    fn cleared_pre_boundary_mass_cannot_reappear_after_clear() {
        let settled = settle_transition_sediment_segment(
            &empty_state(),
            &categories(),
            CategoryId::new(1),
            Duration::from_millis(100),
            Duration::from_millis(900),
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
        .unwrap();
        assert_eq!(settled.added_grains, 1);

        let mut cleared = settled.state;
        cleared.grains.clear();
        cleared.pending_runs.clear();
        let before_next_tick = settle_transition_sediment_segment(
            &cleared,
            &categories(),
            CategoryId::new(2),
            Duration::from_millis(999),
            settled.spawn_remainder,
            settled.physics_remainder,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
        .unwrap();
        assert_eq!(before_next_tick.added_grains, 0);
        assert!(before_next_tick.state.grains.is_empty());
        assert!(before_next_tick.state.pending_runs.is_empty());
    }

    #[test]
    fn large_transition_gap_is_bounded_and_preserves_topology() {
        let mut state = empty_state();
        state.grains.push(SandStateGrain {
            x: 0,
            y: 1,
            category_id: 1,
        });
        let settled = settle_transition_sediment_segment(
            &state,
            &categories(),
            CategoryId::new(2),
            Duration::from_secs(1_000_000_000),
            Duration::ZERO,
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
        .unwrap();
        assert_eq!(settled.added_grains, 1_000_000_000);
        assert_eq!(settled.state.grains, state.grains);
        assert_eq!(settled.state.pending_runs.len(), 1);
        assert_eq!(settled.state.pending_runs[0].category_id, 2);
        assert_eq!(settled.state.pending_runs[0].count, 1_000_000_000);
    }
}

'''
insert_before("src/app.rs", "#[cfg(test)]\nmod clear_all_replay_tests {", transition_tests)
