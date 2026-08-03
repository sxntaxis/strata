from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label} anchor missing")
    return text.replace(old, new, 1)

# Fix and complete the generated application patch.
app_path = Path("src/app.rs")
app = app_path.read_text()
app = replace_once(
    app,
    "use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};",
    "use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, SecondsFormat, Utc};",
    "chrono import",
)
app = replace_once(
    app,
    "        Category, CategoryId, DRIFT_CATEGORY_DISPLAY_NAME, DRIFT_CATEGORY_ID, FirstDayOfWeek,\n        ReportPeriod, RuntimeSettings, TimeTracker, civil_time_for_utc, is_drift_category_id,",
    "        Category, CategoryId, DRIFT_CATEGORY_DISPLAY_NAME, DRIFT_CATEGORY_ID, FirstDayOfWeek,\n        OperationalDayPolicy, ReportPeriod, RuntimeSettings, TimeTracker, civil_time_for_utc,\n        is_drift_category_id,",
    "operational-day import",
)
app = replace_once(
    app,
    "                interval.started_at_utc,\n                interval.ended_at_utc,",
    "                self.session\n                    .active_session_started_at_utc\n                    .ok_or_else(|| \"active session is missing its UTC start timestamp\".to_string())?,\n                interval.ended_at_utc,",
    "affected-day interval start",
)

old_sqlite_placeholder = r'''        if self.sqlite_database_path.is_some() {
            // The second B2C pass replaces this fail-closed placeholder with one
            // SQLite transaction. Until then, do not publish split authority.
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.sand_engine.restore_state(
                &previous_sand,
                &self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect(),
            );
            self.record_storage_result_for::<()>(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                Err("SQLite clear-all transaction is not yet installed; no state changed".to_string()),
            );
            return;
        }
'''
new_sqlite_implementation = r'''        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = previous_session.active_session_stable_id.clone() else {
                self.time_tracker = previous_tracker;
                self.session = previous_session;
                self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );
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
                self.time_tracker = previous_tracker;
                self.session = previous_session;
                self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );
                return;
            }
            self.session.active_session_stable_id = Some(resulting_stable_id);
            self.sync_drift_idle_state();
            return;
        }
'''
app = replace_once(
    app,
    "        checkpoint.clear_all = Some(receipt);",
    "        checkpoint.clear_all = Some(receipt.clone());",
    "clear-all receipt clone",
)

app = replace_once(
    app,
    old_sqlite_placeholder,
    new_sqlite_implementation,
    "SQLite clear-all placeholder",
)

old_recovery_call = r'''        } else if let Some(database_path) = self.sqlite_database_path.clone() {
            checkpoint.clear_all = None;
            sqlite::replace_tui_recovering_checkpoint(&database_path, checkpoint)?;
        }
'''
new_recovery_call = r'''        } else if let Some(database_path) = self.sqlite_database_path.clone() {
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
        }
'''
app = replace_once(app, old_recovery_call, new_recovery_call, "SQLite receipt replay")
# Remove the obsolete standalone reset helper; clear-all now owns its reset atomically.
reset_start = app.find("    fn reset_active_session_at(")
if reset_start != -1:
    reset_end = app.find("    fn open_modal(", reset_start)
    if reset_end == -1:
        raise SystemExit("reset helper end anchor missing")
    app = app[:reset_start] + app[reset_end:]

# Keep strict Clippy clean without suppressions.
old_idle_reset = """        if idle_reset {
            if let Err(error) = self.begin_transition_session(applied_at_utc, clock_mode) {
                self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveReset,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        }
"""
new_idle_reset = """        if idle_reset
            && let Err(error) = self.begin_transition_session(applied_at_utc, clock_mode)
        {
            self.sand_engine.restore_state(
                &previous_sand,
                &self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect(),
            );
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }
"""
app = replace_once(app, old_idle_reset, new_idle_reset, "idle reset collapse")
app_path.write_text(app)

recovery_path = Path("src/app/persistence_recovery.rs")
recovery = recovery_path.read_text()
recovery = replace_once(
    recovery,
    "    DriftSessionDelete,\n",
    "",
    "obsolete idle-delete operation",
)
recovery = replace_once(
    recovery,
    '            Self::DriftSessionDelete => "idle-session deletion",\n',
    "",
    "obsolete idle-delete label",
)
recovery_path.write_text(recovery)
