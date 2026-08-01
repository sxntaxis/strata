from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


# TUI repository adapter.
path = Path("src/sqlite/tui_runtime.rs")
text = path.read_text()
text = replace_once(
    text,
    '''use super::{\n    NewActiveSession, SessionCompletion,\n    authority::open_cli_repository,\n    repository::{CheckpointRecord, CheckpointStatus, SandStateRecord},\n};''',
    '''use super::{\n    NewActiveSession, SessionCompletion,\n    authority::open_cli_repository,\n    repository::SandStateRecord,\n    runtime_coordination,\n};''',
    "TUI runtime imports",
)
text = replace_once(
    text,
    '''pub(crate) struct SqliteTuiActiveSession {\n    pub category_id: CategoryId,''',
    '''pub(crate) struct SqliteTuiActiveSession {\n    pub stable_id: String,\n    pub category_id: CategoryId,''',
    "TUI active stable ID",
)
text = replace_once(
    text,
    '''            Ok::<SqliteTuiActiveSession, String>(SqliteTuiActiveSession {\n                category_id: CategoryId::new(category_id),''',
    '''            Ok::<SqliteTuiActiveSession, String>(SqliteTuiActiveSession {\n                stable_id: row.stable_id,\n                category_id: CategoryId::new(category_id),''',
    "load TUI active stable ID",
)
start = text.index("pub(crate) fn ensure_active_session(")
end = text.index("\npub(crate) fn sync_categories(", start)
new_transitions = r'''pub(crate) fn ensure_active_session(
    database_path: &Path,
    category_id: CategoryId,
    description: &str,
    started_at_utc: DateTime<Utc>,
) -> Result<String, String> {
    let mut repository = open_cli_repository(database_path)?;
    let stable_id = stable_id("tui", started_at_utc);
    let started = timestamp(started_at_utc);
    runtime_coordination::start_active_session(
        &mut repository,
        &NewActiveSession {
            stable_id: &stable_id,
            project: "",
            category_id: as_i64(category_id.0, "category ID")?,
            description,
            started_at_utc: &started,
            recovery_kind: "live",
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(stable_id)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn switch_active_session(
    database_path: &Path,
    expected_active_stable_id: &str,
    operation_id: &str,
    next_stable_id: &str,
    next_category_id: CategoryId,
    next_description: &str,
    switched_at_utc: DateTime<Utc>,
    operational_day: &str,
    elapsed_seconds: usize,
) -> Result<runtime_coordination::RuntimeTransitionReceipt, String> {
    let mut repository = open_cli_repository(database_path)?;
    let switched = timestamp(switched_at_utc);
    runtime_coordination::switch_active_session(
        &mut repository,
        expected_active_stable_id,
        operation_id,
        &SessionCompletion {
            ended_at_utc: &switched,
            operational_day,
            elapsed_seconds: as_i64(elapsed_seconds as u64, "elapsed seconds")?,
            source: "tui-runtime",
        },
        &NewActiveSession {
            stable_id: next_stable_id,
            project: "",
            category_id: as_i64(next_category_id.0, "category ID")?,
            description: next_description,
            started_at_utc: &switched,
            recovery_kind: "live",
        },
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn finish_active_session(
    database_path: &Path,
    expected_active_stable_id: &str,
    operation_id: &str,
    ended_at_utc: DateTime<Utc>,
    operational_day: &str,
    elapsed_seconds: usize,
) -> Result<runtime_coordination::RuntimeTransitionReceipt, String> {
    let mut repository = open_cli_repository(database_path)?;
    let ended = timestamp(ended_at_utc);
    runtime_coordination::finish_active_session(
        &mut repository,
        expected_active_stable_id,
        operation_id,
        &SessionCompletion {
            ended_at_utc: &ended,
            operational_day,
            elapsed_seconds: as_i64(elapsed_seconds as u64, "elapsed seconds")?,
            source: "tui-runtime",
        },
        true,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn reset_active_session(
    database_path: &Path,
    expected_active_stable_id: &str,
    operation_id: &str,
    next_stable_id: &str,
    started_at_utc: DateTime<Utc>,
) -> Result<runtime_coordination::RuntimeTransitionReceipt, String> {
    let mut repository = open_cli_repository(database_path)?;
    let active = repository
        .active_session()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "there is no active TUI session to reset".to_string())?;
    let started = timestamp(started_at_utc);
    runtime_coordination::reset_active_session(
        &mut repository,
        expected_active_stable_id,
        operation_id,
        &NewActiveSession {
            stable_id: next_stable_id,
            project: &active.project,
            category_id: active.category_id,
            description: &active.description,
            started_at_utc: &started,
            recovery_kind: "live",
        },
        &started,
        "tui-runtime",
    )
    .map_err(|error| error.to_string())
}
'''
text = text[:start] + new_transitions + text[end:]
text = replace_once(
    text,
    '''pub(crate) fn sync_categories(\n    database_path: &Path,\n    categories: &[Category],\n    active_category_id: CategoryId,\n) -> Result<Vec<Category>, String> {''',
    '''pub(crate) fn sync_categories(\n    database_path: &Path,\n    categories: &[Category],\n    active_category_id: CategoryId,\n    expected_active_stable_id: Option<&str>,\n) -> Result<Vec<Category>, String> {''',
    "sync category fence signature",
)
old_update = '''    let active_description = categories\n        .iter()\n        .find(|category| category.id == active_category_id)\n        .map(|category| category.description.as_str())\n        .unwrap_or_default();\n    transaction\n        .execute(\n            "UPDATE active_session SET description = ?1 WHERE singleton = 1 AND category_id = ?2",\n            params![active_description, active_id],\n        )\n        .map_err(|error| error.to_string())?;\n    transaction.commit().map_err(|error| error.to_string())?;'''
new_update = '''    if let Some(expected_active_stable_id) = expected_active_stable_id {\n        let active_description = categories\n            .iter()\n            .find(|category| category.id == active_category_id)\n            .map(|category| category.description.as_str())\n            .unwrap_or_default();\n        let changed = transaction\n            .execute(\n                "UPDATE active_session SET description = ?1\n                 WHERE singleton = 1 AND category_id = ?2 AND stable_id = ?3",\n                params![active_description, active_id, expected_active_stable_id],\n            )\n            .map_err(|error| error.to_string())?;\n        if changed != 1 {\n            let actual: Option<String> = transaction\n                .query_row(\n                    "SELECT stable_id FROM active_session WHERE singleton = 1",\n                    [],\n                    |row| row.get(0),\n                )\n                .optional()\n                .map_err(|error| error.to_string())?;\n            return Err(format!(\n                "active session changed concurrently; expected {}, found {}",\n                expected_active_stable_id,\n                actual.unwrap_or_else(|| "no active session".to_string())\n            ));\n        }\n    }\n    transaction.commit().map_err(|error| error.to_string())?;'''
text = replace_once(text, old_update, new_update, "sync category active update")

# Replace checkpoint functions with claimed/committed state machine.
start = text.index("pub(crate) fn save_checkpoint<T: Serialize>(")
end = text.index("\nfn category_from_row(", start)
new_checkpoint = r'''#[derive(Debug)]
pub(crate) struct SqliteClaimedCheckpoint<T> {
    pub active_session_stable_id: Option<String>,
    pub payload: T,
}

pub(crate) fn save_checkpoint<T: Serialize>(
    database_path: &Path,
    expected_active_stable_id: &str,
    detached_at_utc: DateTime<Utc>,
    simulation_time_utc: DateTime<Utc>,
    payload: &T,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let payload_json = serde_json::to_string(payload).map_err(|error| error.to_string())?;
    runtime_coordination::save_checkpoint(
        &mut repository,
        expected_active_stable_id,
        &timestamp(detached_at_utc),
        &timestamp(simulation_time_utc),
        &payload_json,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn load_checkpoint<T: DeserializeOwned>(
    database_path: &Path,
) -> Result<Option<SqliteClaimedCheckpoint<T>>, String> {
    let mut repository = open_cli_repository(database_path)?;
    let Some(claimed) = runtime_coordination::claim_checkpoint(&mut repository)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    match serde_json::from_str(&claimed.payload_json) {
        Ok(payload) => Ok(Some(SqliteClaimedCheckpoint {
            active_session_stable_id: claimed.active_session_stable_id,
            payload,
        })),
        Err(error) => {
            runtime_coordination::quarantine_checkpoint(&mut repository)
                .map_err(|quarantine_error| quarantine_error.to_string())?;
            Err(format!("Invalid runtime checkpoint payload: {error}"))
        }
    }
}

pub(crate) fn quarantine_checkpoint(database_path: &Path) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    runtime_coordination::quarantine_checkpoint(&mut repository)
        .map_err(|error| error.to_string())
}

pub(crate) fn commit_checkpoint_recovery(
    database_path: &Path,
    expected_active_stable_id: &str,
    operational_day: &str,
    state: &SandState,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let existing = repository.sand_state().map_err(|error| error.to_string())?;
    let formation_id = existing
        .as_ref()
        .map(|record| record.formation_id.as_str())
        .unwrap_or("default");
    let quantum_seconds = existing
        .as_ref()
        .map(|record| record.quantum_seconds)
        .unwrap_or(1);
    let now = timestamp(Utc::now());
    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;
    runtime_coordination::commit_checkpoint_recovery(
        &mut repository,
        expected_active_stable_id,
        operational_day,
        &SandStateRecord {
            formation_id: formation_id.to_string(),
            quantum_seconds,
            grid_width: i64::try_from(state.grid_width)
                .map_err(|_| "sand width is too large".to_string())?,
            grid_height: i64::try_from(state.grid_height)
                .map_err(|_| "sand height is too large".to_string())?,
            payload_json,
            updated_at_utc: now.clone(),
        },
        &now,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn clear_checkpoint(database_path: &Path) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    runtime_coordination::clear_committed_checkpoint(&mut repository)
        .map_err(|error| error.to_string())
}
'''
text = text[:start] + new_checkpoint + text[end:]

# Adapt existing adapter tests.
text = text.replace(
    '''            CategoryId::new(0),\n        )''',
    '''            CategoryId::new(0),\n            None,\n        )''',
)
old_test = '''        drop(repository);\n        let state = SandState {'''
new_test = '''        repository\n            .start_session(&NewActiveSession {\n                stable_id: "checkpoint-active",\n                project: "",\n                category_id: 0,\n                description: "",\n                started_at_utc: "2026-08-01T12:00:00Z",\n                recovery_kind: "live",\n            })\n            .unwrap();\n        drop(repository);\n        let state = SandState {'''
text = replace_once(text, old_test, new_test, "checkpoint test active seed")
text = replace_once(
    text,
    '''        save_checkpoint(\n            &path,\n            Utc::now(),''',
    '''        save_checkpoint(\n            &path,\n            "checkpoint-active",\n            Utc::now(),''',
    "checkpoint test save fence",
)
text = replace_once(
    text,
    '''        let checkpoint: Option<BTreeMap<String, String>> = load_checkpoint(&path).unwrap();\n        assert_eq!(checkpoint.unwrap().get("status").unwrap(), "detached");\n        clear_checkpoint(&path).unwrap();''',
    '''        let checkpoint: Option<SqliteClaimedCheckpoint<BTreeMap<String, String>>> =\n            load_checkpoint(&path).unwrap();\n        assert_eq!(checkpoint.unwrap().payload.get("status").unwrap(), "detached");\n        commit_checkpoint_recovery(&path, "checkpoint-active", "2026-08-01", &state)\n            .unwrap();\n        clear_checkpoint(&path).unwrap();''',
    "checkpoint test claim and commit",
)
path.write_text(text)


# Export the new checkpoint operations.
path = Path("src/sqlite.rs")
text = path.read_text()
text = replace_once(
    text,
    '''    archive_category as archive_tui_category, clear_checkpoint as clear_tui_checkpoint,\n    delete_daily_snapshot as delete_tui_daily_snapshot,''',
    '''    archive_category as archive_tui_category, clear_checkpoint as clear_tui_checkpoint,\n    commit_checkpoint_recovery as commit_tui_checkpoint_recovery,\n    delete_daily_snapshot as delete_tui_daily_snapshot,''',
    "checkpoint commit export",
)
text = replace_once(
    text,
    '''    load_state as load_tui_state, reset_active_session as reset_tui_active_session,\n    save_checkpoint as save_tui_checkpoint,''',
    '''    load_state as load_tui_state, quarantine_checkpoint as quarantine_tui_checkpoint,\n    reset_active_session as reset_tui_active_session, save_checkpoint as save_tui_checkpoint,''',
    "checkpoint quarantine export",
)
path.write_text(text)


# Category persistence supplies the active stable-ID fence.
path = Path("src/app/category_state.rs")
text = path.read_text()
text = replace_once(
    text,
    '''                &categories,\n                self.time_tracker.active_category_id(),\n            );''',
    '''                &categories,\n                self.time_tracker.active_category_id(),\n                self.session.active_session_stable_id.as_deref(),\n            );''',
    "category persistence fence",
)
path.write_text(text)


# App coordination and recovery lifecycle.
path = Path("src/app.rs")
text = path.read_text()
text = replace_once(
    text,
    "use chrono::{DateTime, Duration as ChronoDuration, Local, Utc};",
    "use chrono::{DateTime, Duration as ChronoDuration, Local, SecondsFormat, Utc};",
    "app timestamp import",
)
text = replace_once(
    text,
    '''struct SessionState {\n    blink_state: i32,\n    active_session_started_at_utc: Option<DateTime<Utc>>,''',
    '''struct SessionState {\n    blink_state: i32,\n    active_session_stable_id: Option<String>,\n    active_session_started_at_utc: Option<DateTime<Utc>>,''',
    "app stable identity field",
)
text = replace_once(
    text,
    '''    archived_categories: Vec<Category>,\n    storage_error: Option<String>,''',
    '''    archived_categories: Vec<Category>,\n    checkpoint_recovery_active: bool,\n    storage_error: Option<String>,''',
    "app recovery field",
)
text = replace_once(
    text,
    '''            session: SessionState {\n                blink_state: 0,\n                active_session_started_at_utc: None,''',
    '''            session: SessionState {\n                blink_state: 0,\n                active_session_stable_id: None,\n                active_session_started_at_utc: None,''',
    "app session initializer",
)
text = replace_once(
    text,
    '''            archived_categories,\n            storage_error: None,''',
    '''            archived_categories,\n            checkpoint_recovery_active: false,\n            storage_error: None,''',
    "app recovery initializer",
)
text = replace_once(
    text,
    '''                let _ = app\n                    .time_tracker\n                    .set_category_description_by_id(active.category_id, active.description);\n                app.begin_active_session_at(active.started_at_utc);''',
    '''                let _ = app\n                    .time_tracker\n                    .set_category_description_by_id(active.category_id, active.description);\n                app.session.active_session_stable_id = Some(active.stable_id);\n                app.begin_active_session_at(active.started_at_utc);''',
    "loaded active stable identity",
)
text = replace_once(
    text,
    '''        app.sync_drift_idle_state();\n        if let Some(error) = app.storage_error.take() {''',
    '''        app.sync_drift_idle_state();\n        app.commit_checkpoint_recovery_if_ready();\n        if let Some(error) = app.storage_error.take() {''',
    "initial recovery commit",
)
old_persist_start = '''        let result = sqlite::ensure_tui_active_session(\n            &database_path,\n            category_id,\n            &description,\n            started_at,\n        );\n        self.record_storage_result(result);'''
new_persist_start = '''        let result = sqlite::ensure_tui_active_session(\n            &database_path,\n            category_id,\n            &description,\n            started_at,\n        );\n        if let Some(stable_id) = self.record_storage_result(result) {\n            self.session.active_session_stable_id = Some(stable_id);\n        }'''
text = replace_once(text, old_persist_start, new_persist_start, "persist active stable ID")

# Fenced reset.
start = text.index("    fn reset_active_session_at(")
end = text.index("\n    fn open_modal", start)
new_reset = r'''    fn reset_active_session_at(&mut self, started_at_utc: DateTime<Utc>) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result::<()>(Err(
                    "SQLite runtime has no active stable identity to reset".to_string(),
                ));
                return;
            };
            let operation_id = self.transition_operation_id(
                "reset",
                &expected_stable_id,
                started_at_utc,
                "active",
            );
            let next_stable_id = format!("tui-active:{operation_id}");
            let result = sqlite::reset_tui_active_session(
                &database_path,
                &expected_stable_id,
                &operation_id,
                &next_stable_id,
                started_at_utc,
            );
            let Some(receipt) = self.record_storage_result(result) else {
                return;
            };
            self.session.active_session_stable_id = receipt.resulting_active_stable_id;
        }
        self.begin_active_session_at(started_at_utc);
    }
'''
text = text[:start] + new_reset + text[end:]

# Fenced finish and switch methods.
start = text.index("    fn end_active_session_at(")
end = text.index("\n    fn simulation_backlog_duration_at", start)
new_session_methods = r'''    fn end_active_session_at(&mut self, ended_at_utc: DateTime<Utc>) -> Option<usize> {
        let start_utc = self
            .session
            .active_session_started_at_utc
            .or_else(|| {
                self.time_tracker.current_session_start.map(|start| {
                    Utc::now()
                        - ChronoDuration::from_std(start.elapsed())
                            .unwrap_or(ChronoDuration::zero())
                })
            })
            .unwrap_or(ended_at_utc);

        let clamped_end = if ended_at_utc < start_utc {
            start_utc
        } else {
            ended_at_utc
        };

        let elapsed = (clamped_end - start_utc)
            .to_std()
            .unwrap_or(Duration::ZERO)
            .as_secs() as usize;
        let ended_local = clamped_end.with_timezone(&Local);

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result::<()>(Err(
                    "SQLite runtime has no active stable identity to finish".to_string(),
                ));
                return None;
            };
            let operational_day = operational_day_key_for_local(&ended_local)
                .format("%Y-%m-%d")
                .to_string();
            let operation_id = format!("finish:{expected_stable_id}");
            self.record_storage_result(sqlite::finish_tui_active_session(
                &database_path,
                &expected_stable_id,
                &operation_id,
                clamped_end,
                &operational_day,
                elapsed,
            ))?;
            let active_category_id = self.time_tracker.active_category_id();
            let _ = self
                .time_tracker
                .set_category_description_by_id(active_category_id, String::new());
            self.time_tracker.current_session_start = None;
            self.session.active_session_stable_id = None;
            self.session.active_session_started_at_utc = None;
            self.reload_sqlite_sessions();
            self.persist_categories();
            return Some(elapsed);
        }

        let result = self
            .time_tracker
            .end_session_with_elapsed_at_local(elapsed, ended_local);
        self.session.active_session_started_at_utc = None;
        result
    }

    fn switch_active_category_at(
        &mut self,
        category_id: CategoryId,
        switched_at_utc: DateTime<Utc>,
    ) -> bool {
        if self.time_tracker.active_category_id() == category_id {
            return false;
        }

        if self.time_tracker.category_by_id(category_id).is_none() {
            return false;
        }

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result::<()>(Err(
                    "SQLite runtime has no active stable identity to switch".to_string(),
                ));
                return false;
            };
            let start_utc = self
                .session
                .active_session_started_at_utc
                .unwrap_or(switched_at_utc);
            let elapsed = (switched_at_utc - start_utc)
                .to_std()
                .unwrap_or(Duration::ZERO)
                .as_secs() as usize;
            let switched_local = switched_at_utc.with_timezone(&Local);
            let operational_day = operational_day_key_for_local(&switched_local)
                .format("%Y-%m-%d")
                .to_string();
            let next_description = self
                .time_tracker
                .category_description_by_id(category_id)
                .unwrap_or_default()
                .to_string();
            let operation_id = self.transition_operation_id(
                "switch",
                &expected_stable_id,
                switched_at_utc,
                &category_id.0.to_string(),
            );
            let next_stable_id = format!("tui-active:{operation_id}");
            let result = sqlite::switch_tui_active_session(
                &database_path,
                &expected_stable_id,
                &operation_id,
                &next_stable_id,
                category_id,
                &next_description,
                switched_at_utc,
                &operational_day,
                elapsed,
            );
            let Some(receipt) = self.record_storage_result(result) else {
                return false;
            };
            let previous_category_id = self.time_tracker.active_category_id();
            let _ = self
                .time_tracker
                .set_category_description_by_id(previous_category_id, String::new());
            if !self.time_tracker.set_active_category_by_id(category_id) {
                return false;
            }
            self.session.active_session_stable_id = receipt.resulting_active_stable_id;
            self.begin_active_session_at(switched_at_utc);
            self.reload_sqlite_sessions();
            self.persist_categories();
            self.sync_drift_idle_state();
            return true;
        }

        self.end_active_session_at(switched_at_utc);
        self.persist_sessions();

        if !self.time_tracker.set_active_category_by_id(category_id) {
            return false;
        }

        self.begin_active_session_at(switched_at_utc);
        self.sync_drift_idle_state();

        true
    }

    fn transition_operation_id(
        &self,
        kind: &str,
        expected_stable_id: &str,
        at_utc: DateTime<Utc>,
        discriminator: &str,
    ) -> String {
        format!(
            "{}:{}:{}:{}",
            kind,
            expected_stable_id,
            at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true),
            discriminator
        )
    }
'''
text = text[:start] + new_session_methods + text[end:]

# Recovery commit is checked after every simulation advance.
text = replace_once(
    text,
    '''        self.simulation.catchup_was_active = now_catching;\n    }''',
    '''        self.simulation.catchup_was_active = now_catching;\n        self.commit_checkpoint_recovery_if_ready();\n    }''',
    "runtime recovery completion hook",
)

# Checkpoint persistence and restoration.
text = replace_once(
    text,
    '''    fn persist_detached_checkpoint(&mut self) {\n        let active_category_id''',
    '''    fn persist_detached_checkpoint(&mut self) {\n        if self.checkpoint_recovery_active {\n            return;\n        }\n        let active_category_id''',
    "do not overwrite recovering checkpoint",
)
old_save_call = '''            let result = sqlite::save_tui_checkpoint(\n                &database_path,\n                checkpoint.detached_at_utc,\n                checkpoint.simulation_time_utc,\n                &checkpoint,\n            );\n            self.record_storage_result(result);'''
new_save_call = '''            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {\n                self.record_storage_result::<()>(Err(\n                    "SQLite runtime has no active stable identity to checkpoint".to_string(),\n                ));\n                return;\n            };\n            let result = sqlite::save_tui_checkpoint(\n                &database_path,\n                &expected_stable_id,\n                checkpoint.detached_at_utc,\n                checkpoint.simulation_time_utc,\n                &checkpoint,\n            );\n            self.record_storage_result(result);'''
text = replace_once(text, old_save_call, new_save_call, "fenced checkpoint save")

start = text.index("    fn restore_from_detached_checkpoint(")
end = text.index("\n    fn next_blink_interval", start)
new_restore = r'''    fn restore_from_detached_checkpoint(&mut self) -> bool {
        let checkpoint: DetachedRuntimeCheckpoint =
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
                        self.checkpoint_recovery_active = true;
                        claimed.payload
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
                match storage::read_json(&path) {
                    Ok(value) => value,
                    Err(error) => {
                        self.record_storage_result::<()>(Err(error));
                        return false;
                    }
                }
            };

        if checkpoint.schema_version != 1 {
            if let Some(database_path) = self.sqlite_database_path.clone() {
                let _ = sqlite::quarantine_tui_checkpoint(&database_path);
                self.checkpoint_recovery_active = false;
            }
            self.record_storage_result::<()>(Err(format!(
                "unsupported detached checkpoint schema {}",
                checkpoint.schema_version
            )));
            return false;
        }

        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        self.sand_engine
            .restore_state(&checkpoint.sand_state, &valid_category_ids);

        let active_category_id = CategoryId::new(checkpoint.active_category_id);
        if !self
            .time_tracker
            .set_active_category_by_id(active_category_id)
        {
            let _ = self
                .time_tracker
                .set_active_category_by_id(DRIFT_CATEGORY_ID);
        }
        let active_id = self.time_tracker.active_category_id();
        let _ = self
            .time_tracker
            .set_category_description_by_id(active_id, checkpoint.active_description);

        if let Some(started_at) = checkpoint.active_session_started_at_utc {
            self.begin_active_session_at(started_at);
        } else {
            self.begin_active_session_now();
        }

        self.simulation.simulation_time_utc = checkpoint.simulation_time_utc;
        self.simulation.spawn_accumulator =
            Duration::from_nanos(checkpoint.spawn_accumulator_nanos);
        self.simulation.physics_accumulator =
            Duration::from_nanos(checkpoint.physics_accumulator_nanos);
        self.simulation.pending_mutations = checkpoint
            .pending_mutations
            .into_iter()
            .map(|event| QueuedMutationEvent {
                execute_at_utc: event.execute_at_utc,
                mutation: match event.mutation {
                    QueuedMutationRecord::SwitchLayer { category_id } => {
                        QueuedMutation::SwitchLayer(CategoryId::new(category_id))
                    }
                    QueuedMutationRecord::ClearAllSand => QueuedMutation::ClearAllSand,
                    QueuedMutationRecord::ClearDriftSand => QueuedMutation::ClearDriftSand,
                },
            })
            .collect();

        self.simulation
            .pending_mutations
            .make_contiguous()
            .sort_by(|a, b| a.execute_at_utc.cmp(&b.execute_at_utc));

        if self.sqlite_database_path.is_none() {
            self.clear_detached_checkpoint();
        }
        true
    }

    fn commit_checkpoint_recovery_if_ready(&mut self) {
        if !self.checkpoint_recovery_active
            || self.is_catching_up()
            || !self.simulation.pending_mutations.is_empty()
        {
            return;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.checkpoint_recovery_active = false;
            return;
        };
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result::<()>(Err(
                "SQLite recovery has no active stable identity to commit".to_string(),
            ));
            return;
        };
        let state = self.sand_engine.snapshot_state();
        let operational_day = crate::domain::operational_day_key_now()
            .format("%Y-%m-%d")
            .to_string();
        if self
            .record_storage_result(sqlite::commit_tui_checkpoint_recovery(
                &database_path,
                &expected_stable_id,
                &operational_day,
                &state,
            ))
            .is_none()
        {
            return;
        }
        if self
            .record_storage_result(sqlite::clear_tui_checkpoint(&database_path))
            .is_some()
        {
            self.checkpoint_recovery_active = false;
        }
    }
'''
text = text[:start] + new_restore + text[end:]

# Cleanup must never consume an uncommitted recovery checkpoint.
old_cleanup = '''    if runtime_error.is_none() {\n        if app.detach_requested {\n            app.persist_sessions();\n            app.persist_sand_state();\n            app.persist_daily_sand_snapshot();\n            app.persist_detached_checkpoint();\n        } else {\n            app.end_active_session_now();\n            app.persist_sessions();\n            app.persist_sand_state();\n            app.persist_daily_sand_snapshot();\n            app.clear_detached_checkpoint();\n        }\n        runtime_error = app.storage_error.take();\n    }'''
new_cleanup = '''    if runtime_error.is_none() {\n        if app.checkpoint_recovery_active {\n            if !app.detach_requested {\n                runtime_error = Some(\n                    "recovery catch-up is not durably committed; checkpoint retained".to_string(),\n                );\n            }\n        } else if app.detach_requested {\n            app.persist_sessions();\n            app.persist_sand_state();\n            app.persist_daily_sand_snapshot();\n            app.persist_detached_checkpoint();\n        } else {\n            app.end_active_session_now();\n            app.persist_sessions();\n            app.persist_sand_state();\n            app.persist_daily_sand_snapshot();\n            app.clear_detached_checkpoint();\n        }\n        if runtime_error.is_none() {\n            runtime_error = app.storage_error.take();\n        }\n    }'''
text = replace_once(text, old_cleanup, new_cleanup, "recovery-safe UI cleanup")
path.write_text(text)
