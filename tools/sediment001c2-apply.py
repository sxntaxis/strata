from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected one match in {path}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


# Bounded detached-state derivation.
Path("src/sand/recovery.rs").write_text(r'''use std::{collections::HashSet, time::Duration};

use crate::domain::CategoryId;

use super::{SandEngine, SandState};

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
    elapsed: Duration,
    spawn_accumulator: Duration,
    physics_accumulator: Duration,
    spawn_period: Duration,
    physics_period: Duration,
) -> Result<RecoveredSediment, String> {
    validate_checkpoint_state(base_state, valid_category_ids)?;
    if !valid_category_ids.contains(&active_category_id) {
        return Err(format!(
            "recovery active category {} does not exist",
            active_category_id.0
        ));
    }

    let spawn = advance_periodic(spawn_accumulator, elapsed, spawn_period)?;
    let physics = advance_periodic(physics_accumulator, elapsed, physics_period)?;

    let mut engine = SandEngine::new(1, 1);
    engine.restore_state(base_state, valid_category_ids);
    engine.add_logical_grains(active_category_id, spawn.due_events)?;

    Ok(RecoveredSediment {
        state: engine.snapshot_state(),
        spawn_remainder: spawn.remainder,
        physics_remainder: physics.remainder,
        added_grains: spawn.due_events,
        skipped_physics_events: physics.due_events,
    })
}

fn validate_checkpoint_state(
    state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
) -> Result<(), String> {
    if state.version != SandState::VERSION && state.version != SandState::LEGACY_VERSION {
        return Err(format!("unsupported sediment state schema {}", state.version));
    }
    if state.grid_width == 0 || state.grid_height == 0 {
        return Err("recovery sediment canvas must be non-empty".to_string());
    }
    if state.version == SandState::VERSION && !state.pending_grains.is_empty() {
        return Err("version 2 sediment state contains legacy pending grains".to_string());
    }
    if state.version == SandState::LEGACY_VERSION && !state.pending_runs.is_empty() {
        return Err("version 1 sediment state contains version 2 pending runs".to_string());
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
    use super::{PeriodicAdvance, advance_periodic, recover_detached_sediment};
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
                SandStateGrain { x: 0, y: 3, category_id: 1 },
                SandStateGrain { x: 2, y: 2, category_id: 2 },
            ],
            frame_count: 19,
            sweep_left_to_right: false,
            rng_state: 77,
            pending_grains: Vec::new(),
            pending_runs: vec![PendingGrainRun { category_id: 1, count: 3 }],
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
            Duration::from_millis(2_500),
            Duration::from_millis(750),
            Duration::from_millis(20),
            Duration::from_secs(1),
            Duration::from_millis(50),
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
        assert_eq!(recovered.state.sweep_left_to_right, base.sweep_left_to_right);
        assert_eq!(recovered.state.rng_state, base.rng_state);
        let before_mass = base.grains.len() + 3;
        let after_mass = recovered.state.grains.len()
            + recovered.state.pending_runs.iter().map(|run| run.count).sum::<usize>();
        assert_eq!(after_mass, before_mass + 3);
    }

    #[test]
    fn extreme_gap_is_one_compressed_run_when_ingress_is_full() {
        let mut base = base_state();
        base.grains = (0..4)
            .map(|x| SandStateGrain { x, y: 0, category_id: 1 })
            .collect();
        base.pending_runs.clear();
        let recovered = recover_detached_sediment(
            &base,
            &categories(),
            CategoryId::new(2),
            Duration::from_secs(1_000_000_000),
            Duration::ZERO,
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
        .unwrap();
        assert_eq!(recovered.state.pending_runs.len(), 1);
        assert_eq!(recovered.state.pending_runs[0].category_id, 2);
        assert_eq!(recovered.state.pending_runs[0].count, 1_000_000_000);
        assert_eq!(recovered.state.grains, base.grains);
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
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                Duration::from_secs(1),
                Duration::from_millis(50),
            )
            .is_err()
        );
    }
}
''')
replace_once(
    "src/sand/mod.rs",
    "mod engine;\n#[allow(dead_code)]\nmod recovery;\n\n#[allow(unused_imports)]\npub use engine::{PendingGrainRun, SandEngine, SandState, SandStateGrain};",
    "mod engine;\nmod recovery;\n\npub use engine::{PendingGrainRun, SandEngine, SandState, SandStateGrain};\npub(crate) use recovery::{RecoveredSediment, advance_periodic, recover_detached_sediment};",
)

# SQLite claim status and recoverable committed checkpoints.
replace_once(
    "src/sqlite/runtime_coordination.rs",
    "pub(crate) struct ClaimedCheckpoint {\n    pub active_session_stable_id: Option<String>,\n    pub payload_json: String,\n}",
    "pub(crate) struct ClaimedCheckpoint {\n    pub active_session_stable_id: Option<String>,\n    pub payload_json: String,\n    pub was_committed: bool,\n}",
)
text = Path("src/sqlite/runtime_coordination.rs").read_text()
text = text.replace(
    "                payload_json,\n            }))",
    "                payload_json,\n                was_committed: false,\n            }))",
    2,
)
old_committed = '''        "committed" => {
            transaction.execute(
                "DELETE FROM runtime_checkpoint WHERE singleton = 1 AND status = 'committed'",
                [],
            )?;
            maybe_inject_test_fault("checkpoint-claim", "commit")?;
            transaction.commit()?;
            Ok(None)
        }
'''
new_committed = '''        "committed" => {
            transaction.execute(
                "UPDATE runtime_checkpoint SET status = 'recovering'
                 WHERE singleton = 1 AND status = 'committed'",
                [],
            )?;
            maybe_inject_test_fault("checkpoint-claim", "commit")?;
            transaction.commit()?;
            Ok(Some(ClaimedCheckpoint {
                active_session_stable_id,
                payload_json,
                was_committed: true,
            }))
        }
'''
if text.count(old_committed) != 1:
    raise SystemExit("committed claim branch did not match")
text = text.replace(old_committed, new_committed, 1)
insert_before = "pub(crate) fn quarantine_checkpoint(\n"
index = text.index(insert_before)
replace_fn = r'''pub(crate) fn replace_recovering_checkpoint_payload(
    repository: &mut SqliteRepository,
    expected_active_stable_id: &str,
    payload_json: &str,
) -> Result<(), CoordinationError> {
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let record: Option<(String, Option<String>)> = transaction
        .query_row(
            "SELECT status, active_session_stable_id
             FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((status, active_session_stable_id)) = record else {
        return Err(CoordinationError::CheckpointConflict {
            expected: "recovering".to_string(),
            actual: "missing".to_string(),
        });
    };
    if status != "recovering" {
        return Err(CoordinationError::CheckpointConflict {
            expected: "recovering".to_string(),
            actual: status,
        });
    }
    if active_session_stable_id.as_deref() != Some(expected_active_stable_id) {
        return Err(CoordinationError::ActiveSessionConflict {
            expected: expected_active_stable_id.to_string(),
            actual: active_session_stable_id.unwrap_or_else(|| "missing".to_string()),
        });
    }
    transaction.execute(
        "UPDATE runtime_checkpoint SET payload_json = ?1
         WHERE singleton = 1 AND status = 'recovering'",
        params![payload_json],
    )?;
    maybe_inject_test_fault("checkpoint-recovery-target", "commit")?;
    transaction.commit()?;
    Ok(())
}

'''
text = text[:index] + replace_fn + text[index:]
Path("src/sqlite/runtime_coordination.rs").write_text(text)

replace_once(
    "src/sqlite/tui_runtime.rs",
    "pub(crate) struct SqliteClaimedCheckpoint<T> {\n    pub active_session_stable_id: Option<String>,\n    pub payload: T,\n}",
    "pub(crate) struct SqliteClaimedCheckpoint<T> {\n    pub active_session_stable_id: Option<String>,\n    pub payload: T,\n    pub was_committed: bool,\n}",
)
replace_once(
    "src/sqlite/tui_runtime.rs",
    "            active_session_stable_id: claimed.active_session_stable_id,\n            payload,",
    "            active_session_stable_id: claimed.active_session_stable_id,\n            payload,\n            was_committed: claimed.was_committed,",
)
anchor = "pub(crate) fn quarantine_checkpoint(database_path: &Path) -> Result<(), String> {\n"
text = Path("src/sqlite/tui_runtime.rs").read_text()
index = text.index(anchor)
wrapper = r'''pub(crate) fn replace_recovering_checkpoint<T: Serialize>(
    database_path: &Path,
    expected_active_stable_id: &str,
    payload: &T,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let payload_json = serde_json::to_string(payload).map_err(|error| error.to_string())?;
    runtime_coordination::replace_recovering_checkpoint_payload(
        &mut repository,
        expected_active_stable_id,
        &payload_json,
    )
    .map_err(|error| error.to_string())
}

'''
text = text[:index] + wrapper + text[index:]
Path("src/sqlite/tui_runtime.rs").write_text(text)
replace_once(
    "src/sqlite.rs",
    "    reset_active_session as reset_tui_active_session, save_checkpoint as save_tui_checkpoint,",
    "    replace_recovering_checkpoint as replace_tui_recovering_checkpoint,\n    reset_active_session as reset_tui_active_session, save_checkpoint as save_tui_checkpoint,",
)

# Checkpoint schema and app lifecycle.
replace_once(
    "src/app.rs",
    "    sand::{SandEngine, SandState, SandStateGrain},",
    "    sand::{SandEngine, SandState, SandStateGrain, recover_detached_sediment},",
)
replace_once(
    "src/app.rs",
    "#[derive(Clone, Debug, Serialize, Deserialize)]\nstruct QueuedMutationRecord",
    "#[derive(Clone, Debug, Serialize, Deserialize)]\nstruct QueuedMutationRecord",
)
replace_once(
    "src/app.rs",
    "    pending_mutations: Vec<QueuedMutationEventRecord>,\n}\n\nstruct SessionState",
    "    pending_mutations: Vec<QueuedMutationEventRecord>,\n    #[serde(default)]\n    recovery_target_utc: Option<DateTime<Utc>>,\n    #[serde(default)]\n    legacy_recovery_committed: bool,\n}\n\nimpl DetachedRuntimeCheckpoint {\n    const VERSION: u8 = 2;\n    const LEGACY_VERSION: u8 = 1;\n}\n\nstruct SessionState",
)
replace_once(
    "src/app.rs",
    "    checkpoint_recovery_active: bool,\n    persistence_recovery: Option<PersistenceRecoveryState>,",
    "    checkpoint_recovery_active: bool,\n    checkpoint_recovery_payload: Option<DetachedRuntimeCheckpoint>,\n    persistence_recovery: Option<PersistenceRecoveryState>,",
)
replace_once(
    "src/app.rs",
    "            checkpoint_recovery_active: false,\n            persistence_recovery: None,",
    "            checkpoint_recovery_active: false,\n            checkpoint_recovery_payload: None,\n            persistence_recovery: None,",
)
replace_once(
    "src/app.rs",
    "        if !app.restore_from_detached_checkpoint() {\n            if let Some(active) = sqlite_active_session {",
    "        if !app.restore_from_detached_checkpoint() && !app.has_persistence_recovery() {\n            if let Some(active) = sqlite_active_session {",
)
replace_once(
    "src/app.rs",
    "        app.sync_drift_idle_state();\n        app.commit_checkpoint_recovery_if_ready();\n        if let Some(recovery) = app.persistence_recovery.take() {",
    "        app.sync_drift_idle_state();\n        app.commit_checkpoint_recovery_if_ready();\n        if !app.has_persistence_recovery() {\n            app.persist_runtime_checkpoint();\n        }\n        if let Some(recovery) = app.persistence_recovery.take() {",
)

# Recovery no longer installs a relaxed topology.
replace_between(
    "src/app.rs",
    "    fn finalize_catchup_transition(&mut self)",
    "    fn catchup_visual_lines(",
    '''    fn finalize_catchup_transition(&mut self) {
        self.simulation.catchup_visual_engine = None;
        self.simulation.catchup_progress_anchor = None;
        self.simulation.catchup_visual_last_refresh = Instant::now();
        self.simulation.catchup_cadence_accumulator = Duration::ZERO;
        self.render_needed = true;
    }

''',
)
replace_once(
    "src/app.rs",
    "            pending_runs: Vec::new(),",
    "            pending_runs: state.pending_runs.clone(),",
)

# Runtime checkpoints replace detach-only checkpoints.
text = Path("src/app.rs").read_text()
text = text.replace("persist_detached_checkpoint", "persist_runtime_checkpoint")
Path("src/app.rs").write_text(text)
text = Path("src/app/persistence_recovery.rs").read_text().replace(
    "persist_detached_checkpoint", "persist_runtime_checkpoint"
)
Path("src/app/persistence_recovery.rs").write_text(text)

replace_between(
    "src/app.rs",
    "    fn persist_runtime_checkpoint(&mut self)",
    "    fn clear_detached_checkpoint(&mut self)",
    '''    fn persist_runtime_checkpoint(&mut self) {
        if self.checkpoint_recovery_active {
            return;
        }
        if !self.simulation.pending_mutations.is_empty() {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::FlushCurrentState,
                Err("runtime checkpoint cannot be written while mutations are pending".to_string()),
            );
            return;
        }
        let active_category_id = self.time_tracker.active_category_id();
        let active_description = self
            .time_tracker
            .category_description_by_id(active_category_id)
            .unwrap_or_default()
            .to_string();

        let checkpoint = DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: Utc::now(),
            simulation_time_utc: self.simulation.simulation_time_utc,
            spawn_accumulator_nanos: self
                .simulation
                .spawn_accumulator
                .as_nanos()
                .min(u64::MAX as u128) as u64,
            physics_accumulator_nanos: self
                .simulation
                .physics_accumulator
                .as_nanos()
                .min(u64::MAX as u128) as u64,
            active_category_id: active_category_id.0,
            active_description,
            active_session_started_at_utc: self.session.active_session_started_at_utc,
            sand_state: self.sand_engine.snapshot_state(),
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
        };

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result::<()>(Err(
                    "SQLite runtime has no active stable identity to checkpoint".to_string(),
                ));
                return;
            };
            let result = sqlite::save_tui_checkpoint(
                &database_path,
                &expected_stable_id,
                checkpoint.detached_at_utc,
                checkpoint.simulation_time_utc,
                &checkpoint,
            );
            self.record_storage_result_for(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::DetachAndExit,
                result,
            );
        } else {
            let path = storage::get_detached_runtime_path();
            if let Err(error) = storage::write_json_atomic(&path, &checkpoint) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

''',
)

replace_between(
    "src/app.rs",
    "    fn restore_from_detached_checkpoint(&mut self)",
    "    fn commit_checkpoint_recovery_if_ready(&mut self)",
    '''    fn restore_from_detached_checkpoint(&mut self) -> bool {
        let (mut checkpoint, was_committed): (DetachedRuntimeCheckpoint, bool) =
            if let Some(database_path) = self.sqlite_database_path.clone() {
                match sqlite::load_tui_checkpoint(&database_path) {
                    Ok(Some(claimed)) => {
                        let Some(active_stable_id) = claimed.active_session_stable_id else {
                            let _ = sqlite::quarantine_tui_checkpoint(&database_path);
                            self.record_storage_result::<()>(Err(
                                "SQLite recovery checkpoint has no active stable identity".to_string(),
                            ));
                            return false;
                        };
                        self.session.active_session_stable_id = Some(active_stable_id);
                        (claimed.payload, claimed.was_committed)
                    }
                    Ok(None) => return false,
                    Err(error) => {
                        self.record_storage_result::<()>(Err(error));
                        return false;
                    }
                }
            } else {
                let path = storage::get_detached_runtime_path();
                if !storage::file_exists(&path) {
                    return false;
                }
                match storage::read_json::<DetachedRuntimeCheckpoint>(&path) {
                    Ok(value) => {
                        let committed = value.legacy_recovery_committed;
                        (value, committed)
                    }
                    Err(error) => {
                        self.record_storage_result::<()>(Err(error));
                        return false;
                    }
                }
            };

        self.checkpoint_recovery_active = true;

        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION
            && checkpoint.schema_version != DetachedRuntimeCheckpoint::LEGACY_VERSION
        {
            if let Some(database_path) = self.sqlite_database_path.clone() {
                let _ = sqlite::quarantine_tui_checkpoint(&database_path);
            }
            self.record_storage_result::<()>(Err(format!(
                "unsupported detached checkpoint schema {}",
                checkpoint.schema_version
            )));
            return false;
        }
        if !checkpoint.pending_mutations.is_empty() {
            self.record_storage_result::<()>(Err(
                "detached checkpoint contains queued mutations that cannot be recovered without a stable legacy receipt identity; evidence retained"
                    .to_string(),
            ));
            return false;
        }

        let now_utc = Utc::now();
        let target_utc = if was_committed || checkpoint.legacy_recovery_committed {
            now_utc
        } else {
            checkpoint.recovery_target_utc.unwrap_or(now_utc)
        };
        if target_utc > now_utc {
            self.record_storage_result::<()>(Err(format!(
                "detached recovery target {target_utc} is in the future"
            )));
            return false;
        }
        if checkpoint.simulation_time_utc > checkpoint.detached_at_utc
            || checkpoint.detached_at_utc > target_utc
        {
            self.record_storage_result::<()>(Err(
                "detached checkpoint timestamps are not monotonic".to_string(),
            ));
            return false;
        }

        checkpoint.schema_version = DetachedRuntimeCheckpoint::VERSION;
        checkpoint.recovery_target_utc = Some(target_utc);
        checkpoint.legacy_recovery_committed = false;

        let claim_persisted = if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result::<()>(Err(
                    "SQLite recovery checkpoint has no stable identity".to_string(),
                ));
                return false;
            };
            self.record_storage_result_for(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                sqlite::replace_tui_recovering_checkpoint(
                    &database_path,
                    &expected_stable_id,
                    &checkpoint,
                ),
            )
            .is_some()
        } else {
            let path = storage::get_detached_runtime_path();
            self.record_storage_result_for(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                storage::write_json_atomic(&path, &checkpoint),
            )
            .is_some()
        };
        if !claim_persisted {
            return false;
        }

        let active_category_id = CategoryId::new(checkpoint.active_category_id);
        if self.time_tracker.category_by_id(active_category_id).is_none() {
            self.record_storage_result::<()>(Err(format!(
                "detached checkpoint references unavailable active category {}",
                checkpoint.active_category_id
            )));
            return false;
        }
        let Some(started_at_utc) = checkpoint.active_session_started_at_utc else {
            self.record_storage_result::<()>(Err(
                "detached checkpoint has no active-session start timestamp".to_string(),
            ));
            return false;
        };
        if started_at_utc > target_utc {
            self.record_storage_result::<()>(Err(
                "detached checkpoint active session starts after its recovery target".to_string(),
            ));
            return false;
        }

        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        let elapsed = match (target_utc - checkpoint.simulation_time_utc).to_std() {
            Ok(elapsed) => elapsed,
            Err(error) => {
                self.record_storage_result::<()>(Err(format!(
                    "invalid detached recovery interval: {error}"
                )));
                return false;
            }
        };
        let recovered = match recover_detached_sediment(
            &checkpoint.sand_state,
            &valid_category_ids,
            active_category_id,
            elapsed,
            Duration::from_nanos(checkpoint.spawn_accumulator_nanos),
            Duration::from_nanos(checkpoint.physics_accumulator_nanos),
            Duration::from_millis(TIME_SETTINGS.tick_ms),
            Duration::from_millis(TIME_SETTINGS.physics_ms),
        ) {
            Ok(recovered) => recovered,
            Err(error) => {
                self.record_storage_result::<()>(Err(error));
                return false;
            }
        };

        self.sand_engine
            .restore_state(&recovered.state, &valid_category_ids);
        if !self
            .time_tracker
            .set_active_category_by_id(active_category_id)
        {
            self.record_storage_result::<()>(Err(
                "detached recovery could not select its active category".to_string(),
            ));
            return false;
        }
        let _ = self.time_tracker.set_category_description_by_id(
            active_category_id,
            checkpoint.active_description.clone(),
        );
        if let Err(error) = self.begin_active_session_at(started_at_utc, true) {
            self.record_storage_result::<()>(Err(error));
            return false;
        }

        self.simulation.simulation_time_utc = target_utc;
        self.simulation.spawn_accumulator = recovered.spawn_remainder;
        self.simulation.physics_accumulator = recovered.physics_remainder;
        self.simulation.pending_mutations.clear();
        self.simulation.catchup_cadence_accumulator = Duration::ZERO;
        self.simulation.catchup_visual_engine = None;
        self.simulation.catchup_progress_anchor = None;
        self.simulation.catchup_was_active = false;
        self.checkpoint_recovery_payload = Some(checkpoint);
        true
    }

''',
)

replace_between(
    "src/app.rs",
    "    fn commit_checkpoint_recovery_if_ready(&mut self)",
    "    fn next_blink_interval(&self)",
    '''    fn commit_checkpoint_recovery_if_ready(&mut self) {
        if !self.checkpoint_recovery_active {
            return;
        }
        let Some(mut checkpoint) = self.checkpoint_recovery_payload.clone() else {
            self.record_storage_result::<()>(Err(
                "checkpoint recovery payload is unavailable for commit".to_string(),
            ));
            return;
        };
        let state = self.sand_engine.snapshot_state();
        let operational_day = operational_day_key_for_utc(self.simulation.simulation_time_utc)
            .format("%Y-%m-%d")
            .to_string();

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result::<()>(Err(
                    "SQLite recovery has no active stable identity to commit".to_string(),
                ));
                return;
            };
            if self
                .record_storage_result_for(
                    PersistenceOperation::CheckpointRecovery,
                    RecoveryAction::CommitCheckpointRecovery,
                    sqlite::commit_tui_checkpoint_recovery(
                        &database_path,
                        &expected_stable_id,
                        &operational_day,
                        &state,
                    ),
                )
                .is_none()
            {
                return;
            }
        } else {
            if let Err(error) = self.try_flush_current_state() {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointRecovery,
                    RecoveryAction::CommitCheckpointRecovery,
                    Err(error),
                );
                return;
            }
            checkpoint.legacy_recovery_committed = true;
            let path = storage::get_detached_runtime_path();
            if self
                .record_storage_result_for(
                    PersistenceOperation::CheckpointRecovery,
                    RecoveryAction::CommitCheckpointRecovery,
                    storage::write_json_atomic(&path, &checkpoint),
                )
                .is_none()
            {
                return;
            }
        }

        self.checkpoint_recovery_active = false;
        self.checkpoint_recovery_payload = None;
    }

''',
)

# Autosave always refreshes a crash-recovery checkpoint after state publication.
old_autosave = '''                    if !app.has_persistence_recovery() {
                        app.persist_daily_sand_snapshot();
                    }
                    last_save = Instant::now();
'''
new_autosave = '''                    if !app.has_persistence_recovery() {
                        app.persist_daily_sand_snapshot();
                    }
                    if !app.has_persistence_recovery() {
                        app.persist_runtime_checkpoint();
                    }
                    last_save = Instant::now();
'''
replace_once("src/app.rs", old_autosave, new_autosave)

# Legacy recovery needs the existing full-state flush as an explicit callable boundary.
replace_once(
    "src/app/persistence_recovery.rs",
    "    fn try_flush_current_state(&mut self) -> Result<(), String> {",
    "    pub(super) fn try_flush_current_state(&mut self) -> Result<(), String> {",
)

# Remove the post-runtime assertion that recovery must still be replaying.
old_guard = '''        if app.checkpoint_recovery_active {
            app.begin_manual_persistence_failure(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                "recovery catch-up is not durably committed; checkpoint retained",
            );
            app.detach_requested = false;
            continue 'runtime;
        }

'''
replace_once("src/app.rs", old_guard, "")

# Self-contained tests for checkpoint phase selection and serialized compatibility.
append = r'''

#[cfg(test)]
mod bounded_checkpoint_tests {
    use super::DetachedRuntimeCheckpoint;
    use crate::sand::SandState;
    use chrono::{TimeZone, Utc};

    fn checkpoint() -> DetachedRuntimeCheckpoint {
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).unwrap(),
            simulation_time_utc: Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).unwrap(),
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: 0,
            active_description: String::new(),
            active_session_started_at_utc: Some(
                Utc.with_ymd_and_hms(2026, 8, 2, 11, 0, 0).unwrap(),
            ),
            sand_state: SandState {
                version: SandState::VERSION,
                grid_width: 2,
                grid_height: 4,
                grains: Vec::new(),
                frame_count: 0,
                sweep_left_to_right: true,
                rng_state: 1,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
        }
    }

    #[test]
    fn new_checkpoint_fields_are_backward_compatible() {
        let value = serde_json::json!({
            "schema_version": 1,
            "detached_at_utc": "2026-08-02T12:00:00Z",
            "simulation_time_utc": "2026-08-02T12:00:00Z",
            "spawn_accumulator_nanos": 0,
            "physics_accumulator_nanos": 0,
            "active_category_id": 0,
            "active_description": "",
            "active_session_started_at_utc": "2026-08-02T11:00:00Z",
            "sand_state": checkpoint().sand_state,
            "pending_mutations": []
        });
        let decoded: DetachedRuntimeCheckpoint = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.recovery_target_utc, None);
        assert!(!decoded.legacy_recovery_committed);
    }

    #[test]
    fn committed_legacy_evidence_remains_explicit_in_payload() {
        let mut value = checkpoint();
        value.recovery_target_utc = Some(Utc.with_ymd_and_hms(2026, 8, 2, 13, 0, 0).unwrap());
        value.legacy_recovery_committed = true;
        let encoded = serde_json::to_string(&value).unwrap();
        let decoded: DetachedRuntimeCheckpoint = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.legacy_recovery_committed);
        assert_eq!(decoded.recovery_target_utc, value.recovery_target_utc);
    }
}
'''
path = Path("src/app.rs")
path.write_text(path.read_text() + append)

for temporary in [
    ".github/workflows/sediment001c2-apply.yml",
    "tools/sediment001c2-apply.py",
    "tools/sediment001c2.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
