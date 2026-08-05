from pathlib import Path
import re


def sub_once(text: str, pattern: str, replacement: str, label: str) -> str:
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return result


# All application persistence routes require the one SQLite authority.
path = Path("src/app/category_state.rs")
text = path.read_text()
text = text.replace("    sqlite, storage,\n", "    sqlite,\n")
text = sub_once(
    text,
    r"impl App \{\n    pub\(super\) fn persist_categories\(.*?\n    pub\(super\) fn sync_modal_description_from_selection",
    '''impl App {
    pub(super) fn persist_categories(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CategorySync,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let categories = self.time_tracker.categories_for_storage();
        let result = sqlite::sync_tui_categories(
            &database_path,
            &categories,
            self.time_tracker.active_category_id(),
            self.session.active_session_stable_id.as_deref(),
        );
        if let Some(archived) = self.record_storage_result_for(
            PersistenceOperation::CategorySync,
            RecoveryAction::FlushCurrentState,
            result,
        ) {
            self.archived_categories = archived;
        }
    }

    pub(super) fn persist_sessions(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::SessionSync,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let result = sqlite::sync_tui_sessions(&database_path, &self.time_tracker.sessions);
        self.record_storage_result_for(
            PersistenceOperation::SessionSync,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn persist_sand_state(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let state = self.sand_engine.snapshot_state();
        let result = sqlite::save_tui_sand_state(&database_path, &state);
        self.record_storage_result_for(
            PersistenceOperation::SandStateSave,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn persist_daily_sand_snapshot(&mut self) {
        self.reconcile_all_daily_contributions();
    }

    pub(super) fn persist_category_tags(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CategoryTagsSync,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id)
            .collect::<Vec<_>>();
        let result =
            sqlite::sync_tui_category_tags(&database_path, &self.category_tags, &category_ids);
        self.record_storage_result_for(
            PersistenceOperation::CategoryTagsSync,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn restore_sand_state(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::StateReload,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let state = match sqlite::load_tui_sand_state(&database_path) {
            Ok(value) => value,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::StateReload,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        };
        let Some(state) = state else {
            return;
        };
        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<std::collections::HashSet<_>>();
        if let Err(error) = self.sand_engine.restore_state(&state, &valid_category_ids) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::StateReload,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
        }
    }

    pub(super) fn load_daily_sediment_snapshot(
        &mut self,
        day: NaiveDate,
    ) -> Option<crate::sand::SedimentSnapshot> {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::StateReload,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return None;
        };
        let day = day.format("%Y-%m-%d").to_string();
        match sqlite::load_tui_daily_snapshot(&database_path, &day) {
            Ok(value) => value,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::StateReload,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                None
            }
        }
    }

    pub(super) fn save_daily_sediment_snapshot(
        &mut self,
        day: NaiveDate,
        snapshot: &crate::sand::SedimentSnapshot,
    ) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::DailySnapshotSave,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let day = day.format("%Y-%m-%d").to_string();
        let result = sqlite::save_tui_daily_snapshot(&database_path, &day, snapshot);
        self.record_storage_result_for(
            PersistenceOperation::DailySnapshotSave,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn delete_daily_sediment_snapshot(&mut self, day: NaiveDate) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::DailySnapshotDelete,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let day = day.format("%Y-%m-%d").to_string();
        let result = sqlite::delete_tui_daily_snapshot(&database_path, &day);
        self.record_storage_result_for(
            PersistenceOperation::DailySnapshotDelete,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn sync_modal_description_from_selection''',
    "SQLite category state",
)
path.write_text(text)

# Report edits publish once through SQLite and then update the in-memory projection.
path = Path("src/app/report_state.rs")
text = path.read_text()
text = sub_once(
    text,
    r"    pub\(super\) fn commit_report_log_edit\(&mut self\) -> bool \{.*?\n    fn sync_report_selection_for_interval",
    '''    pub(super) fn commit_report_log_edit(&mut self) -> bool {
        let Some(edit) = self.report_log_edit.clone() else {
            return false;
        };
        if !self
            .time_tracker
            .sessions
            .iter()
            .any(|session| session.id == edit.session_id)
        {
            return false;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            retain_report_edit_after_commit(&mut self.report_log_edit, false);
            self.render_needed = true;
            return false;
        };
        let result = crate::sqlite::update_tui_session_description(
            &database_path,
            edit.session_id,
            &edit.draft,
        );
        if self
            .record_storage_result_for(
                PersistenceOperation::SessionEdit,
                RecoveryAction::ReloadAuthority,
                result,
            )
            .is_none()
        {
            retain_report_edit_after_commit(&mut self.report_log_edit, false);
            self.render_needed = true;
            return false;
        }
        if !self
            .time_tracker
            .set_session_description_by_id(edit.session_id, edit.draft)
        {
            return false;
        }
        retain_report_edit_after_commit(&mut self.report_log_edit, true);
        self.render_needed = true;
        true
    }

    fn sync_report_selection_for_interval''',
    "SQLite report edit",
)
text = text.replace(
    '''        if self.sqlite_database_path.is_none() {
            self.persist_sessions();
        }
''',
    "",
)
path.write_text(text)

path = Path("src/app.rs")
text = path.read_text()

# Clear-all replay only updates the claimed SQLite checkpoint; the transaction already owns state.
text = sub_once(
    text,
    r"    fn reconcile_clear_all_receipt\(\n        &mut self,\n        checkpoint: &mut DetachedRuntimeCheckpoint,\n    \) -> Result<\(\), String> \{.*?\n    fn apply_clear_all_at",
    '''    fn reconcile_clear_all_receipt(
        &mut self,
        checkpoint: &mut DetachedRuntimeCheckpoint,
    ) -> Result<(), String> {
        let Some(receipt) = checkpoint.clear_all.clone() else {
            return Ok(());
        };
        validate_clear_all_checkpoint(checkpoint, &receipt)?;
        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
        let expected_stable_id = self
            .session
            .active_session_stable_id
            .as_deref()
            .ok_or_else(|| "SQLite clear-all recovery has no active stable identity".to_string())?;
        checkpoint.clear_all = None;
        sqlite::replace_tui_recovering_checkpoint(
            &database_path,
            expected_stable_id,
            checkpoint,
        )?;
        Ok(())
    }

    fn apply_clear_all_at''',
    "SQLite clear receipt replay",
)

text = sub_once(
    text,
    r"    fn apply_clear_all_at\(&mut self, applied_at_utc: DateTime<Utc>, clock_mode: SessionClockMode\) \{.*?\n    fn settle_simulation_segment_to",
    '''    fn apply_clear_all_at(&mut self, applied_at_utc: DateTime<Utc>, clock_mode: SessionClockMode) {
        let (affected_days, previous_elapsed_seconds) =
            match self.clear_all_effect(applied_at_utc, clock_mode) {
                Ok(effect) => effect,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::SandStateSave,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return;
                }
            };
        let previous_tracker = self.time_tracker.clone();
        let previous_session = self.session.clone();
        let previous_sand = self.sand_engine.snapshot_state();
        let rollback = |app: &mut Self| {
            app.time_tracker = previous_tracker.clone();
            app.session = previous_session.clone();
            app.sand_engine
                .restore_state(
                    &previous_sand,
                    &app
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(app.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                )
                .expect("captured rollback sediment must remain valid");
        };
        let previous_active = ActiveIntervalReceipt {
            category_id: self.time_tracker.active_category_id().0,
            description: self.time_tracker.active_description().to_string(),
            started_at_utc: match self.session.active_session_started_at_utc {
                Some(value) => value,
                None => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::ActiveReset,
                        RecoveryAction::ReloadAuthority,
                        Err("runtime has no active UTC start timestamp to clear".to_string()),
                    );
                    return;
                }
            },
        };
        let idle_reset = is_drift_category_id(self.time_tracker.active_category_id());
        self.sand_engine.clear();
        if idle_reset && let Err(error) = self.begin_transition_session(applied_at_utc, clock_mode) {
            rollback(self);
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }
        let resulting_active = ActiveIntervalReceipt {
            category_id: previous_active.category_id,
            description: previous_active.description.clone(),
            started_at_utc: if idle_reset {
                applied_at_utc
            } else {
                previous_active.started_at_utc
            },
        };
        let affected_operational_days = affected_days
            .iter()
            .map(|day| day.format("%Y-%m-%d").to_string())
            .collect::<Vec<_>>();
        let operation_id = clear_all_operation_id(
            &previous_active,
            applied_at_utc,
            idle_reset,
            previous_elapsed_seconds,
            &affected_operational_days,
        );
        let receipt = ClearAllReceipt {
            operation_id,
            applied_at_utc,
            previous_active,
            resulting_active,
            idle_reset,
            previous_elapsed_seconds,
            affected_operational_days,
        };
        let mut checkpoint = match self.build_runtime_checkpoint() {
            Ok(value) => value,
            Err(error) => {
                rollback(self);
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        };
        checkpoint.clear_all = Some(receipt.clone());
        let Some(database_path) = self.sqlite_database_path.clone() else {
            rollback(self);
            self.record_storage_result_for::<()>(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let Some(expected_stable_id) = previous_session.active_session_stable_id.clone() else {
            rollback(self);
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                Err("SQLite clear-all has no active stable identity".to_string()),
            );
            return;
        };
        let resulting_stable_id = if idle_reset {
            format!("tui-active:{}", receipt.operation_id)
        } else {
            expected_stable_id.clone()
        };
        let daily_updates = affected_days
            .iter()
            .map(|day| {
                (
                    day.format("%Y-%m-%d").to_string(),
                    self.daily_contribution_from_time_log(*day),
                )
            })
            .collect::<Vec<_>>();
        let result = sqlite::clear_tui_state(
            &database_path,
            sqlite::TuiClearAllStateRequest {
                expected_active_stable_id: &expected_stable_id,
                resulting_active_stable_id: &resulting_stable_id,
                resulting_started_at_utc: receipt.resulting_active.started_at_utc,
                state: &checkpoint.sand_state,
                daily_updates: &daily_updates,
                detached_at_utc: checkpoint.detached_at_utc,
                simulation_time_utc: checkpoint.simulation_time_utc,
                checkpoint: &checkpoint,
            },
        );
        if self
            .record_storage_result_for(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                result,
            )
            .is_none()
        {
            rollback(self);
            return;
        }
        self.session.active_session_stable_id = Some(resulting_stable_id);
        self.sync_drift_idle_state();
    }

    fn settle_simulation_segment_to''',
    "SQLite clear transaction",
)

# Every checkpoint publication and recovery commit is SQLite-owned.
text = sub_once(
    text,
    r"    pub\(super\) fn try_write_runtime_checkpoint\(&self\) -> Result<\(\), String> \{.*?\n    fn try_emergency_runtime_checkpoint",
    '''    pub(super) fn try_write_runtime_checkpoint(&self) -> Result<(), String> {
        let checkpoint = self.build_runtime_checkpoint()?;
        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
        let expected_stable_id = self
            .session
            .active_session_stable_id
            .as_deref()
            .ok_or_else(|| "SQLite runtime has no active stable identity to checkpoint".to_string())?;
        sqlite::save_tui_checkpoint(
            &database_path,
            expected_stable_id,
            checkpoint.detached_at_utc,
            checkpoint.simulation_time_utc,
            &checkpoint,
        )
    }

    fn try_emergency_runtime_checkpoint''',
    "SQLite checkpoint write",
)

text = sub_once(
    text,
    r"        let claim_persisted = if let Some\(database_path\) = self\.sqlite_database_path\.clone\(\) \{.*?\n        \};\n        if !claim_persisted",
    '''        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result::<()>(Err("SQLite authority is unavailable".to_string()));
            return false;
        };
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result::<()>(Err(
                "SQLite recovery checkpoint has no stable identity".to_string(),
            ));
            return false;
        };
        let claim_persisted = self
            .record_storage_result_for(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                sqlite::replace_tui_recovering_checkpoint(
                    &database_path,
                    &expected_stable_id,
                    &checkpoint,
                ),
            )
            .is_some();
        if !claim_persisted''',
    "SQLite recovery claim",
)

text = sub_once(
    text,
    r"    fn commit_checkpoint_recovery_if_ready\(&mut self\) \{.*?\n    fn next_blink_interval",
    '''    fn commit_checkpoint_recovery_if_ready(&mut self) {
        if !self.checkpoint_recovery_active {
            return;
        }
        let Some(_checkpoint) = self.checkpoint_recovery_payload.clone() else {
            self.record_storage_result::<()>(Err(
                "checkpoint recovery payload is unavailable for commit".to_string(),
            ));
            return;
        };
        let state = self.sand_engine.snapshot_state();
        let operational_day_date = operational_day_key_for_utc(self.simulation.simulation_time_utc);
        let operational_day = operational_day_date.format("%Y-%m-%d").to_string();
        let Some(daily_contribution) = self.daily_contribution_from_time_log(operational_day_date)
        else {
            self.record_storage_result::<()>(Err(
                "checkpoint recovery produced no daily contribution for its active session"
                    .to_string(),
            ));
            return;
        };
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result::<()>(Err("SQLite authority is unavailable".to_string()));
            return;
        };
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
                    &daily_contribution,
                ),
            )
            .is_none()
        {
            return;
        }
        self.checkpoint_recovery_active = false;
        self.checkpoint_recovery_payload = None;
        self.reconcile_all_daily_contributions();
    }

    fn next_blink_interval''',
    "SQLite recovery commit",
)
path.write_text(text)

# TUI state has a dedicated archived projection; the compact loaded catalog does not duplicate it.
path = Path("src/sqlite/tui_runtime.rs")
text = path.read_text().replace(
    "            archived_categories: archived_categories.clone(),\n",
    "",
)
path.write_text(text)

# Retain pure domain session builders only for domain tests.
path = Path("src/domain.rs")
text = path.read_text()
if "end_session_with_elapsed_at_local" not in text:
    marker = "    pub fn get_todays_time(&self) -> usize {"
    helper = '''    #[cfg(test)]
    pub fn end_session_with_elapsed_at_local<Tz>(
        &mut self,
        elapsed: usize,
        end_local: DateTime<Tz>,
    ) -> Option<usize>
    where
        Tz: chrono::TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        self.current_session_start?;
        let cat_id = self.active_category_id;
        let active_description = self.active_description.clone();
        if elapsed > 0 {
            self.record_session_at(cat_id, &active_description, elapsed, end_local);
        }
        self.active_description.clear();
        self.current_session_start = None;
        Some(elapsed)
    }

    #[cfg(test)]
    pub fn record_session_at<Tz>(
        &mut self,
        cat_id: CategoryId,
        cat_description: &str,
        elapsed: usize,
        end_local: DateTime<Tz>,
    ) where
        Tz: chrono::TimeZone,
        Tz::Offset: std::fmt::Display,
    {
        if elapsed == 0 {
            return;
        }
        let end_utc = end_local.with_timezone(&Utc);
        let start_utc = end_utc - ChronoDuration::seconds(elapsed as i64);
        let start_time = end_local.clone() - ChronoDuration::seconds(elapsed as i64);
        let today = operational_day_key_for_utc(end_utc)
            .format("%Y-%m-%d")
            .to_string();
        self.sessions.push(Session {
            id: self.session_id_counter,
            date: today,
            category_id: cat_id,
            project: String::new(),
            description: cat_description.to_string(),
            start_time: start_time.format("%H:%M:%S").to_string(),
            end_time: end_local.format("%H:%M:%S").to_string(),
            elapsed_seconds: elapsed,
            started_at_utc: Some(start_utc),
            ended_at_utc: Some(end_utc),
            operational_day_policy: Some(OperationalDayPolicy::from_config(day_boundary_config())),
        });
        self.session_id_counter += 1;
    }

'''
    if marker not in text:
        raise SystemExit("domain helper insertion marker missing")
    text = text.replace(marker, helper + marker, 1)
path.write_text(text)

# Production code must not retain any file-backed runtime vocabulary.
forbidden = [
    "get_categories_path",
    "get_time_log_path",
    "get_sand_state_path",
    "get_category_tags_path",
    "get_sand_contribution_path_for_day",
    "get_detached_runtime_path",
    "save_sessions_to_csv",
    "save_category_catalog_to_csv",
    "try_load_sand_state",
    "LegacyTransitionReceipt",
    "LegacyFinishReceipt",
    "legacy_recovery_committed",
    "legacy_transition",
    "legacy_finish",
]
violations = []
for source in Path("src").rglob("*.rs"):
    content = source.read_text()
    for token in forbidden:
        if token in content:
            violations.append(f"{source}:{token}")
if violations:
    raise SystemExit("file runtime residue remains:\n" + "\n".join(violations))

print("final SQLite-only application cleanup applied")
