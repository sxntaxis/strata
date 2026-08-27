use std::{path::Path, time::Duration};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use thiserror::Error;

mod cli_runtime;
#[cfg(test)]
mod fault_certification;
mod maintenance;
mod repository;
mod runtime;
mod runtime_coordination;
mod tui_runtime;

pub(crate) use cli_runtime::{
    read_snapshot as read_cli_snapshot, start_session as start_cli_session,
    stop_session as stop_cli_session,
};
pub(crate) use maintenance::{
    BackupOptions, BundleExportOptions, BundleImportOptions, DoctorOptions, RestoreOptions,
    SqliteMaintenanceReport,
};
pub(crate) use runtime::resolve_runtime_database;
pub(crate) use tui_runtime::{
    ClearAllStateRequest as TuiClearAllStateRequest,
    HistoricalActivePreview as TuiHistoricalActivePreview,
    HistoricalMissedActivityRequest as TuiHistoricalMissedActivityRequest,
    InitialActiveGenerationRequest as TuiInitialActiveGenerationRequest,
    archive_category as archive_tui_category, clear_all_state as clear_tui_state,
    clear_checkpoint as clear_tui_checkpoint,
    commit_checkpoint_recovery as commit_tui_checkpoint_recovery,
    delete_daily_snapshot as delete_tui_daily_snapshot, delete_session as delete_tui_session,
    ensure_active_session as ensure_tui_active_session,
    finish_active_session as finish_tui_active_session,
    initial_active_stable_id as initial_tui_active_stable_id,
    load_checkpoint as load_tui_checkpoint, load_daily_snapshot as load_tui_daily_snapshot,
    load_day_end_snapshot as load_tui_day_end_snapshot, load_sand_state as load_tui_sand_state,
    load_state as load_tui_state, log_missed_activity as log_tui_missed_activity,
    quarantine_checkpoint as quarantine_tui_checkpoint,
    replace_recovering_checkpoint as replace_tui_recovering_checkpoint,
    save_checkpoint as save_tui_checkpoint, save_daily_snapshot as save_tui_daily_snapshot,
    save_day_end_snapshot as save_tui_day_end_snapshot, save_sand_state as save_tui_sand_state,
    start_active_session_with_checkpoint as start_tui_active_session_with_checkpoint,
    switch_active_session as switch_tui_active_session, sync_categories as sync_tui_categories,
    sync_category_tags as sync_tui_category_tags, sync_sessions as sync_tui_sessions,
    update_active_description as update_tui_active_description,
    update_session_description as update_tui_session_description,
};

pub(crate) fn inject_tui_test_fault(operation: &str, phase: &str) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault(operation, phase)
        .map_err(|error| error.to_string())
}

pub(crate) fn run_bundle_export(
    options: BundleExportOptions,
) -> Result<SqliteMaintenanceReport, String> {
    maintenance::export_bundle(options).map_err(|error| error.to_string())
}

pub(crate) fn run_bundle_import(
    options: BundleImportOptions,
) -> Result<SqliteMaintenanceReport, String> {
    maintenance::import_bundle(options).map_err(|error| error.to_string())
}

pub(crate) fn run_doctor(options: DoctorOptions) -> Result<SqliteMaintenanceReport, String> {
    maintenance::doctor(options).map_err(|error| error.to_string())
}

pub(crate) fn run_backup(options: BackupOptions) -> Result<SqliteMaintenanceReport, String> {
    maintenance::backup(options).map_err(|error| error.to_string())
}

pub(crate) fn run_restore(options: RestoreOptions) -> Result<SqliteMaintenanceReport, String> {
    maintenance::restore(options).map_err(|error| error.to_string())
}

const CURRENT_SCHEMA_VERSION: i64 = 1;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

const CURRENT_SCHEMA: &str = r#"
CREATE TABLE database_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE,
    description TEXT NOT NULL DEFAULT '',
    color_index INTEGER NOT NULL CHECK (color_index >= 0),
    balance_effect INTEGER NOT NULL DEFAULT 0 CHECK (balance_effect BETWEEN -1 AND 1),
    archived_at_utc TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0)
) STRICT;

CREATE UNIQUE INDEX categories_active_name_unique
    ON categories(name)
    WHERE archived_at_utc IS NULL;
CREATE INDEX categories_active_order_index
    ON categories(archived_at_utc, sort_order, id);

INSERT INTO categories (
    id, name, description, color_index, balance_effect, archived_at_utc, sort_order
) VALUES (0, 'idle', '', 0, 0, NULL, 0);

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    stable_id TEXT NOT NULL UNIQUE,
    category_id INTEGER NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    started_at_utc TEXT NOT NULL,
    ended_at_utc TEXT NOT NULL,
    operational_day TEXT NOT NULL,
    elapsed_seconds INTEGER NOT NULL CHECK (elapsed_seconds > 0),
    source TEXT NOT NULL DEFAULT 'runtime',
    boundary_utc_offset_seconds INTEGER
        CHECK (boundary_utc_offset_seconds BETWEEN -86399 AND 86399),
    boundary_start_minutes INTEGER
        CHECK (boundary_start_minutes BETWEEN 0 AND 1439),
    FOREIGN KEY (category_id) REFERENCES categories(id)
        ON UPDATE RESTRICT
        ON DELETE RESTRICT,
    CHECK ((boundary_utc_offset_seconds IS NULL) = (boundary_start_minutes IS NULL))
) STRICT;

CREATE INDEX sessions_operational_day_index
    ON sessions(operational_day, started_at_utc);
CREATE INDEX sessions_category_index
    ON sessions(category_id, started_at_utc);

CREATE TRIGGER sessions_temporal_insert_guard
BEFORE INSERT ON sessions
WHEN NEW.elapsed_seconds <= 0
   OR ((NEW.boundary_utc_offset_seconds IS NULL) != (NEW.boundary_start_minutes IS NULL))
BEGIN
    SELECT RAISE(ABORT, 'completed sessions require positive elapsed time and complete boundary provenance');
END;

CREATE TRIGGER sessions_temporal_update_guard
BEFORE UPDATE OF elapsed_seconds, boundary_utc_offset_seconds, boundary_start_minutes ON sessions
WHEN NEW.elapsed_seconds <= 0
   OR ((NEW.boundary_utc_offset_seconds IS NULL) != (NEW.boundary_start_minutes IS NULL))
BEGIN
    SELECT RAISE(ABORT, 'completed sessions require positive elapsed time and complete boundary provenance');
END;

CREATE TABLE active_session (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    stable_id TEXT NOT NULL UNIQUE,
    category_id INTEGER NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    started_at_utc TEXT NOT NULL,
    recovery_kind TEXT NOT NULL DEFAULT 'live'
        CHECK (recovery_kind IN ('live', 'detached', 'recovered')),
    FOREIGN KEY (category_id) REFERENCES categories(id)
        ON UPDATE RESTRICT
        ON DELETE RESTRICT
) STRICT;

CREATE TABLE runtime_checkpoint (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    status TEXT NOT NULL
        CHECK (status IN ('pending', 'recovering', 'committed', 'quarantined')),
    detached_at_utc TEXT NOT NULL,
    simulation_time_utc TEXT NOT NULL,
    active_session_stable_id TEXT,
    payload_json TEXT NOT NULL
) STRICT;

CREATE TABLE sand_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    formation_id TEXT NOT NULL,
    quantum_seconds INTEGER NOT NULL CHECK (quantum_seconds > 0),
    grid_width INTEGER NOT NULL CHECK (grid_width >= 0),
    grid_height INTEGER NOT NULL CHECK (grid_height >= 0),
    payload_json TEXT NOT NULL,
    updated_at_utc TEXT NOT NULL
) STRICT;

CREATE TABLE sand_snapshots (
    id INTEGER PRIMARY KEY,
    formation_id TEXT NOT NULL,
    snapshot_kind TEXT NOT NULL
        CHECK (snapshot_kind IN (
            'daily', 'daily-contribution', 'manual', 'formation_end', 'recovery'
        )),
    operational_day TEXT,
    quantum_seconds INTEGER NOT NULL CHECK (quantum_seconds > 0),
    payload_json TEXT NOT NULL,
    captured_at_utc TEXT NOT NULL
) STRICT;

CREATE INDEX sand_snapshots_formation_index
    ON sand_snapshots(formation_id, captured_at_utc);

CREATE TABLE category_tags (
    category_id INTEGER NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    tag TEXT NOT NULL CHECK (length(trim(tag)) > 0),
    PRIMARY KEY (category_id, ordinal),
    UNIQUE (category_id, tag),
    FOREIGN KEY (category_id) REFERENCES categories(id)
        ON UPDATE RESTRICT
        ON DELETE CASCADE
) STRICT;

CREATE TABLE runtime_transitions (
    operation_id TEXT PRIMARY KEY,
    operation_kind TEXT NOT NULL
        CHECK (operation_kind IN ('finish', 'switch', 'reset')),
    expected_active_stable_id TEXT NOT NULL,
    resulting_active_stable_id TEXT,
    completed_session_id INTEGER,
    elapsed_seconds INTEGER NOT NULL CHECK (elapsed_seconds >= 0),
    source TEXT NOT NULL,
    applied_at_utc TEXT NOT NULL,
    acknowledged_at_utc TEXT,
    FOREIGN KEY (completed_session_id) REFERENCES sessions(id)
        ON UPDATE RESTRICT
        ON DELETE SET NULL
) STRICT;

CREATE INDEX runtime_transitions_unacknowledged_index
    ON runtime_transitions(operation_kind, source, acknowledged_at_utc, applied_at_utc);
CREATE INDEX runtime_transitions_expected_active_index
    ON runtime_transitions(expected_active_stable_id, applied_at_utc);

PRAGMA user_version = 1;
"#;

#[derive(Debug, Error)]
pub(crate) enum SqliteStoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database schema version {found} is newer than the supported version {supported}")]
    UnsupportedSchema { found: i64, supported: i64 },
    #[error("there is no active session to finish")]
    NoActiveSession,
    #[error("completed sessions must have positive elapsed time")]
    InvalidSessionDuration,
}

#[derive(Debug)]
pub(crate) struct NewActiveSession<'a> {
    pub stable_id: &'a str,
    pub category_id: i64,
    pub description: &'a str,
    pub started_at_utc: &'a str,
    pub recovery_kind: &'a str,
}

#[derive(Debug)]
pub(crate) struct SessionCompletion<'a> {
    pub ended_at_utc: &'a str,
    pub operational_day: &'a str,
    pub elapsed_seconds: i64,
    pub boundary_utc_offset_seconds: i32,
    pub boundary_start_minutes: u16,
    pub source: &'a str,
}

pub(crate) struct SqliteRepository {
    connection: Connection,
}

impl SqliteRepository {
    pub fn open(path: &Path) -> Result<Self, SqliteStoreError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    #[cfg(test)]
    fn open_in_memory() -> Result<Self, SqliteStoreError> {
        let connection = Connection::open_in_memory()?;
        Self::from_connection(connection)
    }

    fn from_connection(mut connection: Connection) -> Result<Self, SqliteStoreError> {
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;

        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        match version {
            0 => {
                let transaction =
                    connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
                transaction.execute_batch(CURRENT_SCHEMA)?;
                transaction.commit()?;
            }
            CURRENT_SCHEMA_VERSION => {}
            found => {
                return Err(SqliteStoreError::UnsupportedSchema {
                    found,
                    supported: CURRENT_SCHEMA_VERSION,
                });
            }
        }

        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> Result<i64, SqliteStoreError> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    pub fn integrity_check(&self) -> Result<String, SqliteStoreError> {
        Ok(self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?)
    }

    pub fn metadata_value(&self, key: &str) -> Result<Option<String>, SqliteStoreError> {
        Ok(self
            .connection
            .query_row(
                "SELECT value FROM database_metadata WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn start_session(&mut self, active: &NewActiveSession<'_>) -> Result<(), SqliteStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO active_session (
                singleton,
                stable_id,
                category_id,
                description,
                started_at_utc,
                recovery_kind
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![
                active.stable_id,
                active.category_id,
                active.description,
                active.started_at_utc,
                active.recovery_kind,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn finish_active_session(
        &mut self,
        completion: &SessionCompletion<'_>,
    ) -> Result<i64, SqliteStoreError> {
        if completion.elapsed_seconds <= 0 {
            return Err(SqliteStoreError::InvalidSessionDuration);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let active = transaction
            .query_row(
                "SELECT stable_id, category_id, description, started_at_utc
                 FROM active_session
                 WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or(SqliteStoreError::NoActiveSession)?;

        transaction.execute(
            "INSERT INTO sessions (
                stable_id,
                category_id,
                description,
                started_at_utc,
                ended_at_utc,
                operational_day,
                elapsed_seconds,
                boundary_utc_offset_seconds,
                boundary_start_minutes,
                source
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                active.0,
                active.1,
                active.2,
                active.3,
                completion.ended_at_utc,
                completion.operational_day,
                completion.elapsed_seconds,
                completion.boundary_utc_offset_seconds,
                completion.boundary_start_minutes,
                completion.source,
            ],
        )?;
        let session_id = transaction.last_insert_rowid();

        transaction.execute("DELETE FROM active_session WHERE singleton = 1", [])?;
        transaction.commit()?;
        Ok(session_id)
    }

    #[cfg(test)]
    fn active_session_count(&self) -> Result<i64, SqliteStoreError> {
        Ok(self
            .connection
            .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))?)
    }

    #[cfg(test)]
    fn completed_session_count(&self) -> Result<i64, SqliteStoreError> {
        Ok(self
            .connection
            .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active(stable_id: &str, category_id: i64) -> NewActiveSession<'_> {
        NewActiveSession {
            stable_id,
            category_id,
            description: "Read chapter 4",
            started_at_utc: "2026-08-01T10:00:00Z",
            recovery_kind: "live",
        }
    }

    fn completion() -> SessionCompletion<'static> {
        SessionCompletion {
            ended_at_utc: "2026-08-01T11:00:00Z",
            operational_day: "2026-08-01",
            elapsed_seconds: 3600,
            boundary_utc_offset_seconds: -21600,
            boundary_start_minutes: 360,
            source: "runtime",
        }
    }

    #[test]
    fn new_database_applies_current_schema_and_idle_category() {
        let repository = SqliteRepository::open_in_memory().expect("database should open");

        assert_eq!(repository.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        assert_eq!(repository.integrity_check().unwrap(), "ok");
        let idle: (String, i64) = repository
            .connection
            .query_row(
                "SELECT name, balance_effect FROM categories WHERE id = 0",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(idle, ("idle".to_string(), 0));
    }

    #[test]
    fn current_schema_has_exact_product_tables_and_columns() {
        let repository = SqliteRepository::open_in_memory().expect("database should open");
        let mut statement = repository
            .connection
            .prepare(
                "SELECT name FROM sqlite_schema
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .unwrap();
        let tables = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            tables,
            [
                "active_session",
                "categories",
                "category_tags",
                "database_metadata",
                "runtime_checkpoint",
                "runtime_transitions",
                "sand_snapshots",
                "sand_state",
                "sessions",
            ]
        );

        let expected_columns = [
            (
                "sessions",
                vec![
                    "id",
                    "stable_id",
                    "category_id",
                    "description",
                    "started_at_utc",
                    "ended_at_utc",
                    "operational_day",
                    "elapsed_seconds",
                    "source",
                    "boundary_utc_offset_seconds",
                    "boundary_start_minutes",
                ],
            ),
            (
                "active_session",
                vec![
                    "singleton",
                    "stable_id",
                    "category_id",
                    "description",
                    "started_at_utc",
                    "recovery_kind",
                ],
            ),
            (
                "runtime_checkpoint",
                vec![
                    "singleton",
                    "status",
                    "detached_at_utc",
                    "simulation_time_utc",
                    "active_session_stable_id",
                    "payload_json",
                ],
            ),
            (
                "sand_state",
                vec![
                    "singleton",
                    "formation_id",
                    "quantum_seconds",
                    "grid_width",
                    "grid_height",
                    "payload_json",
                    "updated_at_utc",
                ],
            ),
            (
                "sand_snapshots",
                vec![
                    "id",
                    "formation_id",
                    "snapshot_kind",
                    "operational_day",
                    "quantum_seconds",
                    "payload_json",
                    "captured_at_utc",
                ],
            ),
            ("category_tags", vec!["category_id", "ordinal", "tag"]),
        ];
        for (table, expected) in expected_columns {
            let mut statement = repository
                .connection
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap();
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(columns, expected, "unexpected columns for {table}");
        }
    }

    #[test]
    fn non_current_database_version_is_rejected() {
        let connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 7).unwrap();
        let error = match SqliteRepository::from_connection(connection) {
            Ok(_) => panic!("non-current development database must be rejected"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            SqliteStoreError::UnsupportedSchema {
                found: 7,
                supported: CURRENT_SCHEMA_VERSION
            }
        ));
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let mut repository = SqliteRepository::open_in_memory().expect("database should open");

        let error = repository
            .start_session(&active("missing-category", 999))
            .expect_err("unknown categories must be rejected");

        assert!(matches!(error, SqliteStoreError::Sqlite(_)));
        assert_eq!(repository.active_session_count().unwrap(), 0);
    }

    #[test]
    fn active_to_completed_transition_is_atomic() {
        let mut repository = SqliteRepository::open_in_memory().expect("database should open");
        repository
            .connection
            .execute(
                "INSERT INTO categories(id, name, color_index, balance_effect)
                 VALUES (1, 'Study', 1, 1)",
                [],
            )
            .unwrap();
        repository
            .start_session(&active("session-1", 1))
            .expect("session should start");

        let id = repository
            .finish_active_session(&completion())
            .expect("session should finish");

        assert_eq!(id, 1);
        assert_eq!(repository.active_session_count().unwrap(), 0);
        assert_eq!(repository.completed_session_count().unwrap(), 1);
    }

    #[test]
    fn failed_completion_rolls_back_and_preserves_active_session() {
        let mut repository = SqliteRepository::open_in_memory().expect("database should open");
        repository
            .connection
            .execute(
                "INSERT INTO categories(id, name, color_index, balance_effect)
                 VALUES (1, 'Study', 1, 1)",
                [],
            )
            .unwrap();
        repository
            .start_session(&active("session-1", 1))
            .expect("session should start");
        repository
            .connection
            .execute(
                "INSERT INTO sessions (
                    stable_id,
                    category_id,
                    description,
                    started_at_utc,
                    ended_at_utc,
                    operational_day,
                    elapsed_seconds,
                    source
                 ) VALUES (
                    'session-1',
                    1,
                    '',
                    '2026-08-01T09:00:00Z',
                    '2026-08-01T09:30:00Z',
                    '2026-08-01',
                    1800,
                    'fixture'
                 )",
                [],
            )
            .unwrap();

        repository
            .finish_active_session(&completion())
            .expect_err("duplicate stable identity must fail");

        assert_eq!(repository.active_session_count().unwrap(), 1);
        assert_eq!(repository.completed_session_count().unwrap(), 1);
    }

    #[test]
    fn category_deletion_is_restricted_when_history_references_it() {
        let mut repository = SqliteRepository::open_in_memory().expect("database should open");
        repository
            .connection
            .execute(
                "INSERT INTO categories(id, name, color_index, balance_effect)
                 VALUES (1, 'Study', 1, 1)",
                [],
            )
            .unwrap();
        repository
            .start_session(&active("session-1", 1))
            .expect("session should start");
        repository
            .finish_active_session(&completion())
            .expect("session should finish");
        repository
            .connection
            .execute(
                "UPDATE categories
                 SET archived_at_utc = '2026-08-01T12:00:00Z'
                 WHERE id = 1",
                [],
            )
            .unwrap();

        repository
            .connection
            .execute("DELETE FROM categories WHERE id = 1", [])
            .expect_err("referenced categories must not be deleted");
        assert_eq!(repository.completed_session_count().unwrap(), 1);
    }
}
