use std::{path::Path, time::Duration};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use thiserror::Error;

mod legacy_import;
mod maintenance;
mod migration_command;
mod repository;

pub(crate) use maintenance::{
    BackupOptions, BundleExportOptions, BundleImportOptions, DoctorOptions, RestoreOptions,
    SqliteMaintenanceReport,
};
pub(crate) use migration_command::{ControlledMigrationOptions, ControlledMigrationReport};

pub(crate) fn run_controlled_migration(
    options: ControlledMigrationOptions,
) -> Result<ControlledMigrationReport, String> {
    migration_command::run_controlled_migration(options).map_err(|error| error.to_string())
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

const CURRENT_SCHEMA_VERSION: i64 = 2;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

const MIGRATION_1: &str = r#"
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at_utc TEXT NOT NULL
) STRICT;

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
    archived_at_utc TEXT
) STRICT;

CREATE UNIQUE INDEX categories_active_name_unique
    ON categories(name)
    WHERE archived_at_utc IS NULL;

INSERT INTO categories (
    id,
    name,
    description,
    color_index,
    balance_effect,
    archived_at_utc
) VALUES (0, 'idle', '', 0, 0, NULL);

CREATE TABLE projects (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE,
    archived_at_utc TEXT
) STRICT;

CREATE UNIQUE INDEX projects_active_name_unique
    ON projects(name)
    WHERE archived_at_utc IS NULL;

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    stable_id TEXT NOT NULL UNIQUE,
    project TEXT NOT NULL DEFAULT '',
    category_id INTEGER NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    started_at_utc TEXT NOT NULL,
    ended_at_utc TEXT NOT NULL,
    operational_day TEXT NOT NULL,
    elapsed_seconds INTEGER NOT NULL CHECK (elapsed_seconds >= 0),
    source TEXT NOT NULL DEFAULT 'runtime',
    FOREIGN KEY (category_id) REFERENCES categories(id)
        ON UPDATE RESTRICT
        ON DELETE RESTRICT
) STRICT;

CREATE INDEX sessions_operational_day_index
    ON sessions(operational_day, started_at_utc);
CREATE INDEX sessions_category_index
    ON sessions(category_id, started_at_utc);

CREATE TABLE active_session (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    stable_id TEXT NOT NULL UNIQUE,
    project TEXT NOT NULL DEFAULT '',
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
        CHECK (snapshot_kind IN ('daily', 'manual', 'formation_end', 'recovery')),
    operational_day TEXT,
    quantum_seconds INTEGER NOT NULL CHECK (quantum_seconds > 0),
    payload_json TEXT NOT NULL,
    captured_at_utc TEXT NOT NULL
) STRICT;

CREATE INDEX sand_snapshots_formation_index
    ON sand_snapshots(formation_id, captured_at_utc);

INSERT INTO schema_migrations(version, applied_at_utc)
VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

INSERT INTO database_metadata(key, value)
VALUES ('storage_authority', 'sqlite-candidate');

PRAGMA user_version = 1;
"#;

const MIGRATION_2: &str = r#"
CREATE TABLE legacy_imports (
    id INTEGER PRIMARY KEY,
    source_fingerprint TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK (status IN ('pending', 'verified')),
    source_manifest_json TEXT NOT NULL,
    utc_offset_seconds INTEGER NOT NULL
        CHECK (utc_offset_seconds BETWEEN -86399 AND 86399),
    operational_day_start_minutes INTEGER NOT NULL
        CHECK (operational_day_start_minutes BETWEEN 0 AND 1439),
    quantum_seconds INTEGER NOT NULL CHECK (quantum_seconds > 0),
    category_count INTEGER NOT NULL CHECK (category_count >= 0),
    session_count INTEGER NOT NULL CHECK (session_count >= 0),
    total_elapsed_seconds INTEGER NOT NULL CHECK (total_elapsed_seconds >= 0),
    active_session_present INTEGER NOT NULL
        CHECK (active_session_present IN (0, 1)),
    checkpoint_present INTEGER NOT NULL
        CHECK (checkpoint_present IN (0, 1)),
    sand_state_present INTEGER NOT NULL
        CHECK (sand_state_present IN (0, 1)),
    snapshot_count INTEGER NOT NULL CHECK (snapshot_count >= 0),
    verification_json TEXT,
    started_at_utc TEXT NOT NULL,
    completed_at_utc TEXT
) STRICT;

CREATE TABLE category_tags (
    category_id INTEGER NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    tag TEXT NOT NULL CHECK (length(trim(tag)) > 0),
    legacy_import_id INTEGER,
    PRIMARY KEY (category_id, ordinal),
    UNIQUE (category_id, tag),
    FOREIGN KEY (category_id) REFERENCES categories(id)
        ON UPDATE RESTRICT
        ON DELETE CASCADE,
    FOREIGN KEY (legacy_import_id) REFERENCES legacy_imports(id)
        ON DELETE RESTRICT
) STRICT;

ALTER TABLE sessions
    ADD COLUMN legacy_import_id INTEGER
        REFERENCES legacy_imports(id) ON DELETE RESTRICT;
ALTER TABLE active_session
    ADD COLUMN legacy_import_id INTEGER
        REFERENCES legacy_imports(id) ON DELETE RESTRICT;
ALTER TABLE runtime_checkpoint
    ADD COLUMN legacy_import_id INTEGER
        REFERENCES legacy_imports(id) ON DELETE RESTRICT;
ALTER TABLE sand_state
    ADD COLUMN legacy_import_id INTEGER
        REFERENCES legacy_imports(id) ON DELETE RESTRICT;
ALTER TABLE sand_snapshots
    ADD COLUMN legacy_import_id INTEGER
        REFERENCES legacy_imports(id) ON DELETE RESTRICT;

CREATE INDEX sessions_legacy_import_index
    ON sessions(legacy_import_id, id);
CREATE INDEX sand_snapshots_legacy_import_index
    ON sand_snapshots(legacy_import_id, id);

INSERT INTO schema_migrations(version, applied_at_utc)
VALUES (2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

PRAGMA user_version = 2;
"#;

#[derive(Debug, Error)]
pub(crate) enum SqliteStoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database schema version {found} is newer than the supported version {supported}")]
    UnsupportedSchema { found: i64, supported: i64 },
    #[error("there is no active session to finish")]
    NoActiveSession,
}

#[derive(Debug)]
pub(crate) struct NewActiveSession<'a> {
    pub stable_id: &'a str,
    pub project: &'a str,
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

        let mut version: i64 =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > CURRENT_SCHEMA_VERSION {
            return Err(SqliteStoreError::UnsupportedSchema {
                found: version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }

        if version < 1 {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute_batch(MIGRATION_1)?;
            transaction.commit()?;
            version = 1;
        }

        if version < 2 {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute_batch(MIGRATION_2)?;
            transaction.commit()?;
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

    pub fn start_session(&mut self, active: &NewActiveSession<'_>) -> Result<(), SqliteStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO active_session (
                singleton,
                stable_id,
                project,
                category_id,
                description,
                started_at_utc,
                recovery_kind
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                active.stable_id,
                active.project,
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
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let active = transaction
            .query_row(
                "SELECT stable_id, project, category_id, description, started_at_utc
                 FROM active_session
                 WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or(SqliteStoreError::NoActiveSession)?;

        transaction.execute(
            "INSERT INTO sessions (
                stable_id,
                project,
                category_id,
                description,
                started_at_utc,
                ended_at_utc,
                operational_day,
                elapsed_seconds,
                source
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                active.0,
                active.1,
                active.2,
                active.3,
                active.4,
                completion.ended_at_utc,
                completion.operational_day,
                completion.elapsed_seconds,
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
            project: "Study",
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
            source: "runtime",
        }
    }

    #[test]
    fn new_database_applies_schema_and_idle_category() {
        let repository = SqliteRepository::open_in_memory().expect("database should open");

        assert_eq!(repository.schema_version().unwrap(), 2);
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

        let import_table: String = repository
            .connection
            .query_row(
                "SELECT name FROM sqlite_schema
                 WHERE type = 'table' AND name = 'legacy_imports'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(import_table, "legacy_imports");
    }

    #[test]
    fn version_one_database_is_upgraded_without_losing_history() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATION_1).unwrap();
        connection
            .execute(
                "INSERT INTO categories(id, name, color_index, balance_effect)
                 VALUES (1, 'Study', 1, 1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO sessions (
                    id,
                    stable_id,
                    project,
                    category_id,
                    description,
                    started_at_utc,
                    ended_at_utc,
                    operational_day,
                    elapsed_seconds,
                    source
                ) VALUES (
                    7,
                    'existing-session',
                    '',
                    1,
                    '',
                    '2026-08-01T10:00:00Z',
                    '2026-08-01T11:00:00Z',
                    '2026-08-01',
                    3600,
                    'runtime'
                )",
                [],
            )
            .unwrap();

        let repository =
            SqliteRepository::from_connection(connection).expect("migration should succeed");

        assert_eq!(repository.schema_version().unwrap(), 2);
        assert_eq!(repository.completed_session_count().unwrap(), 1);
        let legacy_import_id: Option<i64> = repository
            .connection
            .query_row(
                "SELECT legacy_import_id FROM sessions WHERE id = 7",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_import_id, None);
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
                    project,
                    category_id,
                    description,
                    started_at_utc,
                    ended_at_utc,
                    operational_day,
                    elapsed_seconds,
                    source
                 ) VALUES (
                    'session-1',
                    'fixture',
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
