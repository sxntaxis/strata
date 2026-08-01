from pathlib import Path

def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}")
    target.write_text(text.replace(old, new, 1))

# ---------------------------------------------------------------------------
# Thread-local deterministic fault injection for unit certification.
# ---------------------------------------------------------------------------
replace_once(
    "src/sqlite/runtime_coordination.rs",
    '''#[cfg(debug_assertions)]
use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};''',
    '''#[cfg(debug_assertions)]
use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

#[cfg(test)]
use std::cell::RefCell;''',
)

replace_once(
    "src/sqlite/runtime_coordination.rs",
    '''pub(crate) fn maybe_inject_test_fault(
    operation: &str,
    phase: &str,
) -> Result<(), CoordinationError> {
    #[cfg(debug_assertions)]
    {
        if let Ok(specification) = std::env::var("STRATA_TEST_SQLITE_FAULT") {''',
    '''#[cfg(test)]
#[derive(Clone)]
struct ScopedTestFault {
    operation: String,
    phase: String,
    class: String,
    remaining: usize,
}

#[cfg(test)]
thread_local! {
    static SCOPED_TEST_FAULT: RefCell<Option<ScopedTestFault>> = const { RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn with_test_fault<T>(
    operation: &str,
    phase: &str,
    class: &str,
    action: impl FnOnce() -> T,
) -> T {
    struct Reset(Option<ScopedTestFault>);

    impl Drop for Reset {
        fn drop(&mut self) {
            SCOPED_TEST_FAULT.with(|slot| {
                slot.replace(self.0.take());
            });
        }
    }

    let previous = SCOPED_TEST_FAULT.with(|slot| {
        slot.replace(Some(ScopedTestFault {
            operation: operation.to_string(),
            phase: phase.to_string(),
            class: class.to_string(),
            remaining: 1,
        }))
    });
    let _reset = Reset(previous);
    action()
}

pub(crate) fn maybe_inject_test_fault(
    operation: &str,
    phase: &str,
) -> Result<(), CoordinationError> {
    #[cfg(test)]
    {
        let injected = SCOPED_TEST_FAULT.with(|slot| {
            let mut slot = slot.borrow_mut();
            let specification = slot.as_mut()?;
            if specification.operation != operation
                || specification.phase != phase
                || specification.remaining == 0
            {
                return None;
            }
            specification.remaining -= 1;
            Some(specification.class.clone())
        });
        if let Some(class) = injected {
            return Err(CoordinationError::InjectedFailure {
                operation: operation.to_string(),
                phase: phase.to_string(),
                class,
            });
        }
    }

    #[cfg(debug_assertions)]
    {
        if let Ok(specification) = std::env::var("STRATA_TEST_SQLITE_FAULT") {''',
)

# Runtime checkpoint mutation commit phases.
replace_once(
    "src/sqlite/runtime_coordination.rs",
    '''            transaction.execute(
                "UPDATE runtime_checkpoint SET status = 'recovering'
                 WHERE singleton = 1 AND status = 'pending'",
                [],
            )?;
            transaction.commit()?;''',
    '''            transaction.execute(
                "UPDATE runtime_checkpoint SET status = 'recovering'
                 WHERE singleton = 1 AND status = 'pending'",
                [],
            )?;
            maybe_inject_test_fault("checkpoint-claim", "commit")?;
            transaction.commit()?;''',
)

replace_once(
    "src/sqlite/runtime_coordination.rs",
    '''        "recovering" => {
            transaction.commit()?;
            Ok(Some(ClaimedCheckpoint {''',
    '''        "recovering" => {
            maybe_inject_test_fault("checkpoint-claim", "commit")?;
            transaction.commit()?;
            Ok(Some(ClaimedCheckpoint {''',
)

replace_once(
    "src/sqlite/runtime_coordination.rs",
    '''        "committed" => {
            transaction.execute(
                "DELETE FROM runtime_checkpoint WHERE singleton = 1 AND status = 'committed'",
                [],
            )?;
            transaction.commit()?;''',
    '''        "committed" => {
            transaction.execute(
                "DELETE FROM runtime_checkpoint WHERE singleton = 1 AND status = 'committed'",
                [],
            )?;
            maybe_inject_test_fault("checkpoint-claim", "commit")?;
            transaction.commit()?;''',
)

replace_once(
    "src/sqlite/runtime_coordination.rs",
    '''    if changed != 1 {
        let actual: Option<String> = transaction
            .query_row(
                "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        return Err(CoordinationError::CheckpointConflict {
            expected: "pending or recovering".to_string(),
            actual: actual.unwrap_or_else(|| "missing".to_string()),
        });
    }
    transaction.commit()?;
    Ok(())
}''',
    '''    if changed != 1 {
        let actual: Option<String> = transaction
            .query_row(
                "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        return Err(CoordinationError::CheckpointConflict {
            expected: "pending or recovering".to_string(),
            actual: actual.unwrap_or_else(|| "missing".to_string()),
        });
    }
    maybe_inject_test_fault("checkpoint-quarantine", "commit")?;
    transaction.commit()?;
    Ok(())
}''',
)

replace_once(
    "src/sqlite/runtime_coordination.rs",
    '''        Some("committed") => {
            transaction.execute(
                "DELETE FROM runtime_checkpoint WHERE singleton = 1 AND status = 'committed'",
                [],
            )?;
            transaction.commit()?;
            Ok(())
        }''',
    '''        Some("committed") => {
            transaction.execute(
                "DELETE FROM runtime_checkpoint WHERE singleton = 1 AND status = 'committed'",
                [],
            )?;
            maybe_inject_test_fault("checkpoint-clear", "commit")?;
            transaction.commit()?;
            Ok(())
        }''',
)

# ---------------------------------------------------------------------------
# Make every TUI persistence adapter expose deterministic pre-write/commit
# phases and preserve transaction atomicity.
# ---------------------------------------------------------------------------
replace_once(
    "src/sqlite/tui_runtime.rs",
    '''    }
    transaction.commit().map_err(|error| error.to_string())?;

    Ok(load_state(database_path)?.archived_categories)
}''',
    '''    }
    runtime_coordination::maybe_inject_test_fault("category-sync", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;

    Ok(load_state(database_path)?.archived_categories)
}''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''pub(crate) fn archive_category(
    database_path: &Path,
    category_id: CategoryId,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let category_id = as_i64(category_id.0, "category ID")?;
    let active_category_id = repository
        .active_session()
        .map_err(|error| error.to_string())?
        .map(|active| active.category_id);
    if active_category_id == Some(category_id) {
        return Err("the active category cannot be archived".to_string());
    }
    repository
        .archive_category(category_id, &timestamp(Utc::now()))
        .map_err(|error| error.to_string())?;
    Ok(())
}''',
    '''pub(crate) fn archive_category(
    database_path: &Path,
    category_id: CategoryId,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("category-archive", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let category_id = as_i64(category_id.0, "category ID")?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let active_category_id: Option<i64> = transaction
        .query_row(
            "SELECT category_id FROM active_session WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if active_category_id == Some(category_id) {
        return Err("the active category cannot be archived".to_string());
    }
    let changed = transaction
        .execute(
            "UPDATE categories
             SET archived_at_utc = ?1
             WHERE id = ?2 AND archived_at_utc IS NULL",
            params![timestamp(Utc::now()), category_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("active category {category_id} does not exist"));
    }
    runtime_coordination::maybe_inject_test_fault("category-archive", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''pub(crate) fn sync_category_tags(
    database_path: &Path,
    tags: &CategoryTagsState,
    category_ids: &[CategoryId],
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    for category_id in category_ids {
        let category_id = as_i64(category_id.0, "category ID")?;
        let category_id_u64 = u64::try_from(category_id)
            .map_err(|_| format!("Category ID {category_id} is invalid"))?;
        let values = tags
            .tags_by_category
            .get(&category_id_u64)
            .cloned()
            .unwrap_or_default();
        repository
            .replace_category_tags(category_id, &values)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}''',
    '''pub(crate) fn sync_category_tags(
    database_path: &Path,
    tags: &CategoryTagsState,
    category_ids: &[CategoryId],
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("category-tags", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    for category_id in category_ids {
        let category_id = as_i64(category_id.0, "category ID")?;
        let category_id_u64 = u64::try_from(category_id)
            .map_err(|_| format!("Category ID {category_id} is invalid"))?;
        let values = tags
            .tags_by_category
            .get(&category_id_u64)
            .cloned()
            .unwrap_or_default();
        let mut normalized = Vec::with_capacity(values.len());
        let mut seen = BTreeSet::new();
        for value in values {
            let value = value.trim();
            if value.is_empty() {
                return Err("category tag is empty".to_string());
            }
            if !seen.insert(value.to_string()) {
                return Err(format!("duplicate category tag '{value}'"));
            }
            normalized.push(value.to_string());
        }
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id = ?1)",
                params![category_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !exists {
            return Err(format!("category {category_id} does not exist"));
        }
        transaction
            .execute(
                "DELETE FROM category_tags WHERE category_id = ?1",
                params![category_id],
            )
            .map_err(|error| error.to_string())?;
        for (ordinal, tag) in normalized.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO category_tags(category_id, ordinal, tag)
                     VALUES (?1, ?2, ?3)",
                    params![
                        category_id,
                        i64::try_from(ordinal).map_err(|_| "too many category tags".to_string())?,
                        tag,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    runtime_coordination::maybe_inject_test_fault("category-tags", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''pub(crate) fn sync_sessions(database_path: &Path, sessions: &[Session]) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;''',
    '''pub(crate) fn sync_sessions(database_path: &Path, sessions: &[Session]) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("session-sync", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''        transaction
            .execute(
                "UPDATE sessions SET description = ?1 WHERE id = ?2",
                params![session.description, id],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}''',
    '''        transaction
            .execute(
                "UPDATE sessions SET description = ?1 WHERE id = ?2",
                params![session.description, id],
            )
            .map_err(|error| error.to_string())?;
    }
    runtime_coordination::maybe_inject_test_fault("session-sync", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''    let repository = open_cli_repository(database_path)?;
    let session_id =
        i64::try_from(session_id).map_err(|_| "session ID is too large".to_string())?;
    let changed = repository
        .connection
        .execute(
            "UPDATE sessions SET description = ?1 WHERE id = ?2",
            params![description, session_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    Ok(())
}''',
    '''    let mut repository = open_cli_repository(database_path)?;
    let session_id =
        i64::try_from(session_id).map_err(|_| "session ID is too large".to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let changed = transaction
        .execute(
            "UPDATE sessions SET description = ?1 WHERE id = ?2",
            params![description, session_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    runtime_coordination::maybe_inject_test_fault("session-edit", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''    let repository = open_cli_repository(database_path)?;
    let session_id =
        i64::try_from(session_id).map_err(|_| "session ID is too large".to_string())?;
    let changed = repository
        .connection
        .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    Ok(())
}''',
    '''    let mut repository = open_cli_repository(database_path)?;
    let session_id =
        i64::try_from(session_id).map_err(|_| "session ID is too large".to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let changed = transaction
        .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    runtime_coordination::maybe_inject_test_fault("session-delete", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''pub(crate) fn delete_drift_sessions_for_day(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    repository
        .connection
        .execute(
            "DELETE FROM sessions WHERE category_id = 0 AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}''',
    '''pub(crate) fn delete_drift_sessions_for_day(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("drift-session-delete", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM sessions WHERE category_id = 0 AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("drift-session-delete", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''    repository
        .save_sand_state(&SandStateRecord {
            formation_id: formation_id.to_string(),
            quantum_seconds,
            grid_width: i64::try_from(state.grid_width)
                .map_err(|_| "sand width is too large".to_string())?,
            grid_height: i64::try_from(state.grid_height)
                .map_err(|_| "sand height is too large".to_string())?,
            payload_json,
            updated_at_utc: timestamp(Utc::now()),
        })
        .map_err(|error| error.to_string())
}''',
    '''    let grid_width = i64::try_from(state.grid_width)
        .map_err(|_| "sand width is too large".to_string())?;
    let grid_height = i64::try_from(state.grid_height)
        .map_err(|_| "sand height is too large".to_string())?;
    let updated_at_utc = timestamp(Utc::now());
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO sand_state (
                singleton, formation_id, quantum_seconds, grid_width, grid_height,
                payload_json, updated_at_utc, legacy_import_id
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, NULL)
             ON CONFLICT(singleton) DO UPDATE SET
                formation_id = excluded.formation_id,
                quantum_seconds = excluded.quantum_seconds,
                grid_width = excluded.grid_width,
                grid_height = excluded.grid_height,
                payload_json = excluded.payload_json,
                updated_at_utc = excluded.updated_at_utc,
                legacy_import_id = NULL",
            params![
                formation_id,
                quantum_seconds,
                grid_width,
                grid_height,
                payload_json,
                updated_at_utc,
            ],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("sand-state", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn load_daily_snapshot(''',
    '''        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("daily-snapshot", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn load_daily_snapshot(''',
)

replace_once(
    "src/sqlite/tui_runtime.rs",
    '''pub(crate) fn delete_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    repository
        .connection
        .execute(
            "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily' AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}''',
    '''pub(crate) fn delete_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("daily-snapshot-delete", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily' AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("daily-snapshot-delete", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}''',
)

# Register exhaustive certification module.
replace_once(
    "src/sqlite.rs",
    '''mod authority;
mod cli_runtime;
mod legacy_import;''',
    '''mod authority;
mod cli_runtime;
#[cfg(test)]
mod fault_certification;
mod legacy_import;''',
)

# Expand failure-class certification.
replace_once(
    "src/app/persistence_recovery.rs",
    '''        assert_eq!(
            classify_failure("active session changed concurrently; expected a, found b"),
            PersistenceFailureClass::Conflict
        );
    }
}''',
    '''        assert_eq!(
            classify_failure("active session changed concurrently; expected a, found b"),
            PersistenceFailureClass::Conflict
        );
        assert_eq!(
            classify_failure("FOREIGN KEY constraint failed"),
            PersistenceFailureClass::Constraint
        );
        assert_eq!(
            classify_failure("database or disk is full"),
            PersistenceFailureClass::Io
        );
        assert_eq!(
            classify_failure("invalid runtime transition"),
            PersistenceFailureClass::InvalidData
        );
        assert_eq!(
            classify_failure("unrecognized persistence response"),
            PersistenceFailureClass::Unknown
        );
    }
}''',
)

Path("src/sqlite/fault_certification.rs").write_text(r'''use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use chrono::{TimeZone, Utc};
use rusqlite::{Connection, params};

use crate::{
    constants::COLORS,
    domain::{Category, CategoryId, Session},
    sand::SandState,
    storage::CategoryTagsState,
};

use super::{
    NewActiveSession, SqliteRepository,
    repository::{NewCategoryRecord, SandStateRecord},
    runtime_coordination,
    tui_runtime,
};

fn database_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "strata-sqlite010-{name}-{}-{}.sqlite3",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ))
}

fn remove_database(path: &Path) {
    fs::remove_file(path).ok();
    fs::remove_file(format!("{}-wal", path.display())).ok();
    fs::remove_file(format!("{}-shm", path.display())).ok();
}

fn seed(path: &Path) {
    let mut repository = SqliteRepository::open(path).unwrap();
    repository
        .transition_storage_authority(
            "sqlite-candidate",
            "sqlite-cli",
            "2026-08-01T12:00:00Z",
        )
        .unwrap();
    repository
        .create_category(&NewCategoryRecord {
            name: "Work",
            description: "original-work",
            color_index: 0,
            balance_effect: 1,
        })
        .unwrap();
    repository
        .create_category(&NewCategoryRecord {
            name: "Rest",
            description: "original-rest",
            color_index: 1,
            balance_effect: -1,
        })
        .unwrap();
}

fn start_active(path: &Path, stable_id: &str, category_id: i64) {
    let mut repository = SqliteRepository::open(path).unwrap();
    runtime_coordination::start_active_session(
        &mut repository,
        &NewActiveSession {
            stable_id,
            project: "",
            category_id,
            description: "",
            started_at_utc: "2026-08-01T12:00:00Z",
            recovery_kind: "live",
        },
    )
    .unwrap();
}

fn insert_session(path: &Path, id: i64, category_id: i64, description: &str) {
    let repository = SqliteRepository::open(path).unwrap();
    repository
        .connection
        .execute(
            "INSERT INTO sessions (
                id, stable_id, project, category_id, description, started_at_utc,
                ended_at_utc, operational_day, elapsed_seconds, source
             ) VALUES (?1, ?2, '', ?3, ?4, '2026-08-01T12:00:00Z',
                '2026-08-01T13:00:00Z', '2026-08-01', 3600, 'tui-runtime')",
            params![id, format!("session-{id}"), category_id, description],
        )
        .unwrap();
}

fn category(id: u64, name: &str, description: &str, color_index: usize, balance: i8) -> Category {
    Category {
        id: CategoryId::new(id),
        name: name.to_string(),
        color: COLORS[color_index % COLORS.len()],
        description: description.to_string(),
        karma_effect: balance,
    }
}

fn session_description(path: &Path, id: i64) -> String {
    let repository = SqliteRepository::open(path).unwrap();
    repository
        .connection
        .query_row(
            "SELECT description FROM sessions WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .unwrap()
}

fn active_id(path: &Path) -> Option<String> {
    SqliteRepository::open(path)
        .unwrap()
        .active_session()
        .unwrap()
        .map(|active| active.stable_id)
}

fn count(path: &Path, table: &str) -> i64 {
    let repository = SqliteRepository::open(path).unwrap();
    repository
        .connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn sand_state(frame_count: u64) -> SandState {
    SandState {
        version: SandState::VERSION,
        grid_width: 2,
        grid_height: 2,
        grains: Vec::new(),
        frame_count,
        sweep_left_to_right: true,
        rng_state: frame_count,
    }
}

fn with_database(name: &str, action: impl FnOnce(&Path)) {
    let path = database_path(name);
    seed(&path);
    action(&path);
    remove_database(&path);
}

#[test]
fn every_authoritative_persistence_family_rolls_back_or_remains_recoverable() {
    with_database("active-start", |path| {
        let started = Utc.with_ymd_and_hms(2026, 8, 1, 12, 30, 0).unwrap();
        let error = runtime_coordination::with_test_fault(
            "active-start",
            "commit",
            "commit",
            || tui_runtime::ensure_active_session(path, CategoryId::new(1), "", started),
        )
        .unwrap_err();
        assert!(error.contains("active-start commit"));
        assert_eq!(active_id(path), None);
    });

    with_database("active-finish", |path| {
        start_active(path, "active-a", 1);
        let ended = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        runtime_coordination::with_test_fault("finish", "commit", "commit", || {
            tui_runtime::finish_active_session(
                path,
                "active-a",
                "finish:active-a",
                ended,
                "2026-08-01",
                3600,
            )
        })
        .unwrap_err();
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
        assert_eq!(count(path, "sessions"), 0);
        assert_eq!(count(path, "runtime_transitions"), 0);
    });

    with_database("active-switch", |path| {
        start_active(path, "active-a", 1);
        let switched = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        runtime_coordination::with_test_fault("switch", "commit", "commit", || {
            tui_runtime::switch_active_session(
                path,
                "active-a",
                "switch:active-a",
                "active-b",
                CategoryId::new(2),
                "",
                switched,
                "2026-08-01",
                3600,
            )
        })
        .unwrap_err();
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
        assert_eq!(count(path, "sessions"), 0);
        assert_eq!(count(path, "runtime_transitions"), 0);
    });

    with_database("active-reset", |path| {
        start_active(path, "active-a", 1);
        let reset_at = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        runtime_coordination::with_test_fault("reset", "commit", "commit", || {
            tui_runtime::reset_active_session(
                path,
                "active-a",
                "reset:active-a",
                "active-b",
                reset_at,
            )
        })
        .unwrap_err();
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
        assert_eq!(count(path, "runtime_transitions"), 0);
    });

    with_database("category-sync", |path| {
        start_active(path, "active-a", 1);
        let categories = vec![
            category(0, "None", "", 0, 0),
            category(1, "Work", "changed", 0, 1),
            category(2, "Rest", "changed", 1, -1),
        ];
        runtime_coordination::with_test_fault("category-sync", "commit", "commit", || {
            tui_runtime::sync_categories(
                path,
                &categories,
                CategoryId::new(1),
                Some("active-a"),
            )
        })
        .unwrap_err();
        let state = tui_runtime::load_state(path).unwrap();
        assert_eq!(state.loaded_categories.categories[1].description, "original-work");
        assert_eq!(state.loaded_categories.categories[2].description, "original-rest");
    });

    with_database("category-archive", |path| {
        runtime_coordination::with_test_fault("category-archive", "commit", "commit", || {
            tui_runtime::archive_category(path, CategoryId::new(1))
        })
        .unwrap_err();
        let state = tui_runtime::load_state(path).unwrap();
        assert!(state.archived_categories.is_empty());
        assert!(state.loaded_categories.categories.iter().any(|entry| entry.id.0 == 1));
    });

    with_database("category-tags", |path| {
        let mut repository = SqliteRepository::open(path).unwrap();
        repository
            .replace_category_tags(1, &["old-work".to_string()])
            .unwrap();
        repository
            .replace_category_tags(2, &["old-rest".to_string()])
            .unwrap();
        drop(repository);
        let mut tags = CategoryTagsState::default();
        tags.tags_by_category.insert(1, vec!["new-work".to_string()]);
        tags.tags_by_category.insert(2, vec!["new-rest".to_string()]);
        runtime_coordination::with_test_fault("category-tags", "commit", "commit", || {
            tui_runtime::sync_category_tags(
                path,
                &tags,
                &[CategoryId::new(1), CategoryId::new(2)],
            )
        })
        .unwrap_err();
        let stored = SqliteRepository::open(path).unwrap().category_tags().unwrap();
        assert_eq!(stored.get(&1).unwrap(), &vec!["old-work".to_string()]);
        assert_eq!(stored.get(&2).unwrap(), &vec!["old-rest".to_string()]);
    });

    with_database("session-sync", |path| {
        insert_session(path, 7, 1, "old");
        let sessions = vec![Session {
            id: 7,
            date: "2026-08-01".to_string(),
            category_id: CategoryId::new(1),
            description: "new".to_string(),
            start_time: "12:00:00".to_string(),
            end_time: "13:00:00".to_string(),
            elapsed_seconds: 3600,
        }];
        runtime_coordination::with_test_fault("session-sync", "commit", "commit", || {
            tui_runtime::sync_sessions(path, &sessions)
        })
        .unwrap_err();
        assert_eq!(session_description(path, 7), "old");
    });

    with_database("session-edit", |path| {
        insert_session(path, 7, 1, "old");
        runtime_coordination::with_test_fault("session-edit", "commit", "commit", || {
            tui_runtime::update_session_description(path, 7, "new")
        })
        .unwrap_err();
        assert_eq!(session_description(path, 7), "old");
    });

    with_database("session-delete", |path| {
        insert_session(path, 7, 1, "old");
        runtime_coordination::with_test_fault("session-delete", "commit", "commit", || {
            tui_runtime::delete_session(path, 7)
        })
        .unwrap_err();
        assert_eq!(count(path, "sessions"), 1);
    });

    with_database("drift-delete", |path| {
        insert_session(path, 7, 0, "drift");
        runtime_coordination::with_test_fault(
            "drift-session-delete",
            "commit",
            "commit",
            || tui_runtime::delete_drift_sessions_for_day(path, "2026-08-01"),
        )
        .unwrap_err();
        assert_eq!(count(path, "sessions"), 1);
    });

    with_database("sand-state", |path| {
        tui_runtime::save_sand_state(path, &sand_state(1)).unwrap();
        runtime_coordination::with_test_fault("sand-state", "commit", "commit", || {
            tui_runtime::save_sand_state(path, &sand_state(2))
        })
        .unwrap_err();
        assert_eq!(tui_runtime::load_sand_state(path).unwrap().unwrap().frame_count, 1);
    });

    with_database("daily-snapshot", |path| {
        tui_runtime::save_daily_snapshot(path, "2026-08-01", &sand_state(1)).unwrap();
        runtime_coordination::with_test_fault("daily-snapshot", "commit", "commit", || {
            tui_runtime::save_daily_snapshot(path, "2026-08-01", &sand_state(2))
        })
        .unwrap_err();
        assert_eq!(
            tui_runtime::load_daily_snapshot(path, "2026-08-01")
                .unwrap()
                .unwrap()
                .frame_count,
            1
        );
    });

    with_database("daily-snapshot-delete", |path| {
        tui_runtime::save_daily_snapshot(path, "2026-08-01", &sand_state(1)).unwrap();
        runtime_coordination::with_test_fault(
            "daily-snapshot-delete",
            "commit",
            "commit",
            || tui_runtime::delete_daily_snapshot(path, "2026-08-01"),
        )
        .unwrap_err();
        assert!(
            tui_runtime::load_daily_snapshot(path, "2026-08-01")
                .unwrap()
                .is_some()
        );
    });

    with_database("checkpoint-save", |path| {
        start_active(path, "active-a", 1);
        let at = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        runtime_coordination::with_test_fault("checkpoint-save", "commit", "commit", || {
            tui_runtime::save_checkpoint(path, "active-a", at, at, &sand_state(1))
        })
        .unwrap_err();
        assert!(SqliteRepository::open(path).unwrap().checkpoint().unwrap().is_none());
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
    });

    with_database("checkpoint-claim", |path| {
        start_active(path, "active-a", 1);
        let at = Utc.with_ymd_and_hms(2026, 8, 1, 13, 0, 0).unwrap();
        tui_runtime::save_checkpoint(path, "active-a", at, at, &sand_state(1)).unwrap();
        runtime_coordination::with_test_fault("checkpoint-claim", "commit", "commit", || {
            tui_runtime::load_checkpoint::<SandState>(path)
        })
        .unwrap_err();
        assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status
                .as_str(),
            "pending"
        );
    });

    with_database("checkpoint-quarantine", |path| {
        start_active(path, "active-a", 1);
        let mut repository = SqliteRepository::open(path).unwrap();
        runtime_coordination::save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T13:00:00Z",
            "2026-08-01T13:00:00Z",
            "{}",
        )
        .unwrap();
        runtime_coordination::claim_checkpoint(&mut repository)
            .unwrap()
            .unwrap();
        drop(repository);
        runtime_coordination::with_test_fault(
            "checkpoint-quarantine",
            "commit",
            "commit",
            || tui_runtime::quarantine_checkpoint(path),
        )
        .unwrap_err();
        assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status
                .as_str(),
            "recovering"
        );
    });

    with_database("checkpoint-recovery", |path| {
        start_active(path, "active-a", 1);
        tui_runtime::save_sand_state(path, &sand_state(1)).unwrap();
        let mut repository = SqliteRepository::open(path).unwrap();
        runtime_coordination::save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T13:00:00Z",
            "2026-08-01T13:00:00Z",
            "{}",
        )
        .unwrap();
        runtime_coordination::claim_checkpoint(&mut repository)
            .unwrap()
            .unwrap();
        drop(repository);
        runtime_coordination::with_test_fault(
            "checkpoint-recovery",
            "commit",
            "commit",
            || {
                tui_runtime::commit_checkpoint_recovery(
                    path,
                    "active-a",
                    "2026-08-01",
                    &sand_state(2),
                )
            },
        )
        .unwrap_err();
        assert_eq!(tui_runtime::load_sand_state(path).unwrap().unwrap().frame_count, 1);
        assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status
                .as_str(),
            "recovering"
        );
    });

    with_database("checkpoint-clear", |path| {
        start_active(path, "active-a", 1);
        let mut repository = SqliteRepository::open(path).unwrap();
        runtime_coordination::save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T13:00:00Z",
            "2026-08-01T13:00:00Z",
            "{}",
        )
        .unwrap();
        runtime_coordination::claim_checkpoint(&mut repository)
            .unwrap()
            .unwrap();
        runtime_coordination::commit_checkpoint_recovery(
            &mut repository,
            "active-a",
            "2026-08-01",
            &SandStateRecord {
                formation_id: "default".to_string(),
                quantum_seconds: 1,
                grid_width: 2,
                grid_height: 2,
                payload_json: serde_json::to_string(&sand_state(1)).unwrap(),
                updated_at_utc: "2026-08-01T13:00:00Z".to_string(),
            },
            "2026-08-01T13:00:00Z",
        )
        .unwrap();
        drop(repository);
        runtime_coordination::with_test_fault("checkpoint-clear", "commit", "commit", || {
            tui_runtime::clear_checkpoint(path)
        })
        .unwrap_err();
        assert_eq!(
            SqliteRepository::open(path)
                .unwrap()
                .checkpoint()
                .unwrap()
                .unwrap()
                .status
                .as_str(),
            "committed"
        );
    });

    with_database("state-load", |path| {
        runtime_coordination::with_test_fault("state-load", "before-read", "corrupt", || {
            tui_runtime::load_state(path)
        })
        .unwrap_err();
        assert_eq!(count(path, "categories"), 3);
    });
}

#[test]
fn real_sqlite_busy_full_constraint_and_corruption_fail_without_false_success() {
    with_database("real-busy", |path| {
        start_active(path, "active-a", 1);
        let lock = Connection::open(path).unwrap();
        lock.execute_batch("BEGIN IMMEDIATE").unwrap();
        let started = Instant::now();
        let error = tui_runtime::update_session_description(path, 999, "blocked").unwrap_err();
        assert!(error.to_ascii_lowercase().contains("locked") || error.to_ascii_lowercase().contains("busy"));
        assert!(started.elapsed().as_secs() >= 4);
        lock.execute_batch("ROLLBACK").unwrap();
        assert_eq!(active_id(path).as_deref(), Some("active-a"));
    });

    with_database("real-constraint", |path| {
        insert_session(path, 7, 1, "old");
        let repository = SqliteRepository::open(path).unwrap();
        repository
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_session_edit
                 BEFORE UPDATE OF description ON sessions
                 BEGIN SELECT RAISE(ABORT, 'injected constraint'); END;",
            )
            .unwrap();
        drop(repository);
        let error = tui_runtime::update_session_description(path, 7, "new").unwrap_err();
        assert!(error.to_ascii_lowercase().contains("constraint"));
        assert_eq!(session_description(path, 7), "old");
    });

    with_database("real-full", |path| {
        let repository = SqliteRepository::open(path).unwrap();
        repository
            .connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        let page_count: i64 = repository
            .connection
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap();
        repository
            .connection
            .pragma_update(None, "max_page_count", page_count)
            .unwrap();
        drop(repository);
        let categories = vec![
            category(0, "None", "", 0, 0),
            category(1, "Work", &"x".repeat(2 * 1024 * 1024), 0, 1),
            category(2, "Rest", "original-rest", 1, -1),
        ];
        let error = tui_runtime::sync_categories(path, &categories, CategoryId::new(0), None)
            .unwrap_err();
        assert!(error.to_ascii_lowercase().contains("full"));
        let state = tui_runtime::load_state(path).unwrap();
        assert_eq!(state.loaded_categories.categories[1].description, "original-work");
    });

    let path = database_path("real-corrupt");
    seed(&path);
    fs::write(&path, b"not a sqlite database").unwrap();
    let error = tui_runtime::load_state(&path).unwrap_err();
    let normalized = error.to_ascii_lowercase();
    assert!(normalized.contains("not a database") || normalized.contains("malformed"));
    remove_database(&path);
}

#[cfg(unix)]
#[test]
fn real_read_only_authority_rejects_writes_and_preserves_rows() {
    use std::os::unix::fs::PermissionsExt;

    let root = std::env::temp_dir().join(format!(
        "strata-sqlite010-readonly-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("strata.sqlite3");
    seed(&path);
    insert_session(&path, 7, 1, "old");
    fs::remove_file(format!("{}-wal", path.display())).ok();
    fs::remove_file(format!("{}-shm", path.display())).ok();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).unwrap();

    let error = tui_runtime::update_session_description(&path, 7, "new").unwrap_err();

    fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        error.to_ascii_lowercase().contains("readonly")
            || error.to_ascii_lowercase().contains("read-only")
    );
    assert_eq!(session_description(&path, 7), "old");
    remove_database(&path);
    fs::remove_dir_all(root).ok();
}
''')
