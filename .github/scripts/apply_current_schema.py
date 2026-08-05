from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


def sub_once(text: str, pattern: str, replacement: str, label: str) -> str:
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return result


CURRENT_SCHEMA = r'''const CURRENT_SCHEMA_VERSION: i64 = 1;
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

CREATE TABLE category_lifecycle_receipts (
    operation_id TEXT PRIMARY KEY,
    operation_kind TEXT NOT NULL CHECK (operation_kind IN ('merge', 'delete')),
    source_category_id INTEGER NOT NULL,
    target_category_id INTEGER,
    source_metadata_json TEXT NOT NULL,
    target_metadata_json TEXT,
    preview_revision TEXT NOT NULL,
    reference_counts_json TEXT NOT NULL,
    applied_at_utc TEXT NOT NULL,
    CHECK (
        (operation_kind = 'merge' AND target_category_id IS NOT NULL)
        OR (operation_kind = 'delete' AND target_category_id IS NULL)
    ),
    CHECK (target_category_id IS NULL OR target_category_id != source_category_id)
) STRICT;

CREATE UNIQUE INDEX category_lifecycle_receipts_preview_unique
    ON category_lifecycle_receipts(
        source_category_id,
        COALESCE(target_category_id, -1),
        preview_revision
    );

INSERT INTO database_metadata(key, value)
VALUES ('storage_authority', 'sqlite');

PRAGMA user_version = 1;
"#;'''

path = Path("src/sqlite.rs")
text = path.read_text()
text = sub_once(
    text,
    r"const CURRENT_SCHEMA_VERSION: i64 = 7;.*?const MIGRATION_7: &str = r#\".*?\"#;",
    CURRENT_SCHEMA,
    "migration chain",
)
text = text.replace(
    '''    #[error("SQLite storage authority conflict: expected {expected}, found {found}")]
    AuthorityConflict { expected: String, found: String },
''',
    "",
)
text = sub_once(
    text,
    r"    fn from_connection\(mut connection: Connection\) -> Result<Self, SqliteStoreError> \{.*?\n        Ok\(Self \{ connection \}\)\n    \}",
    '''    fn from_connection(mut connection: Connection) -> Result<Self, SqliteStoreError> {
        connection.busy_timeout(BUSY_TIMEOUT)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;

        let version: i64 =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
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
    }''',
    "schema initialization",
)
text = sub_once(
    text,
    r"\n    pub fn transition_storage_authority\(.*?\n    \}\n\n    pub fn start_session",
    "\n    pub fn start_session",
    "authority transition",
)
text = sub_once(
    text,
    r"    #\[test\]\n    fn new_database_applies_schema_and_idle_category\(\) \{.*?\n    #\[test\]\n    fn foreign_keys_are_enforced",
    '''    #[test]
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
    fn current_schema_has_no_compatibility_tables_or_columns() {
        let repository = SqliteRepository::open_in_memory().expect("database should open");
        for table in ["schema_migrations", "legacy_imports"] {
            let count: i64 = repository
                .connection
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0, "compatibility table {table} must not exist");
        }
        for table in [
            "sessions",
            "active_session",
            "runtime_checkpoint",
            "sand_state",
            "sand_snapshots",
            "category_tags",
        ] {
            let mut statement = repository
                .connection
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap();
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(!columns.iter().any(|column| column == "legacy_import_id"));
        }
    }

    #[test]
    fn non_current_database_version_is_rejected() {
        let connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 7).unwrap();
        let error = SqliteRepository::from_connection(connection)
            .expect_err("non-current development database must be rejected");
        assert!(matches!(
            error,
            SqliteStoreError::UnsupportedSchema {
                found: 7,
                supported: CURRENT_SCHEMA_VERSION
            }
        ));
    }

    #[test]
    fn foreign_keys_are_enforced''',
    "schema tests",
)
path.write_text(text)

# Category lifecycle preview/merge no longer preserves import provenance.
path = Path("src/sqlite/category_lifecycle.rs")
text = path.read_text()
text = text.replace(
    '"SELECT category_id, ordinal, tag, legacy_import_id\n         FROM category_tags ORDER BY category_id, ordinal",\n        4,',
    '"SELECT category_id, ordinal, tag\n         FROM category_tags ORDER BY category_id, ordinal",\n        3,',
)
text = text.replace(
    '"SELECT singleton, formation_id, quantum_seconds, grid_width, grid_height,\n                payload_json, updated_at_utc, legacy_import_id\n         FROM sand_state ORDER BY singleton",\n        8,',
    '"SELECT singleton, formation_id, quantum_seconds, grid_width, grid_height,\n                payload_json, updated_at_utc\n         FROM sand_state ORDER BY singleton",\n        7,',
)
text = text.replace(
    '"SELECT id, formation_id, snapshot_kind, operational_day, quantum_seconds,\n                payload_json, captured_at_utc, legacy_import_id\n         FROM sand_snapshots ORDER BY id",\n        8,',
    '"SELECT id, formation_id, snapshot_kind, operational_day, quantum_seconds,\n                payload_json, captured_at_utc\n         FROM sand_snapshots ORDER BY id",\n        7,',
)
text = text.replace(
    '"SELECT singleton, status, detached_at_utc, simulation_time_utc,\n                active_session_stable_id, payload_json, legacy_import_id\n         FROM runtime_checkpoint ORDER BY singleton",\n        7,',
    '"SELECT singleton, status, detached_at_utc, simulation_time_utc,\n                active_session_stable_id, payload_json\n         FROM runtime_checkpoint ORDER BY singleton",\n        6,',
)
text = text.replace("tags: Vec<(String, Option<i64>)>,", "tags: Vec<String>,")
text = sub_once(
    text,
    r"fn merged_tags\(.*?\n\}\n\nfn replace_merged_tags",
    '''fn merged_tags(
    connection: &Connection,
    source_category_id: i64,
    target_category_id: i64,
) -> Result<Vec<String>, String> {
    let mut merged = Vec::new();
    let mut seen = BTreeSet::new();
    for category_id in [target_category_id, source_category_id] {
        let mut statement = connection
            .prepare(
                "SELECT tag FROM category_tags
                 WHERE category_id = ?1 ORDER BY ordinal",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![category_id], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?;
        for row in rows {
            let tag = row.map_err(|error| error.to_string())?;
            if seen.insert(tag.clone()) {
                merged.push(tag);
            }
        }
    }
    Ok(merged)
}

fn replace_merged_tags''',
    "merged tags",
)
text = sub_once(
    text,
    r"fn replace_merged_tags\(.*?\n\}\n\n#\[derive\(Clone, Debug\)\]",
    '''fn replace_merged_tags(
    transaction: &Transaction<'_>,
    source_category_id: i64,
    target_category_id: i64,
    tags: &[String],
) -> Result<(), String> {
    transaction
        .execute(
            "DELETE FROM category_tags WHERE category_id IN (?1, ?2)",
            params![source_category_id, target_category_id],
        )
        .map_err(|error| error.to_string())?;
    for (ordinal, tag) in tags.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO category_tags(category_id, ordinal, tag)
                 VALUES (?1, ?2, ?3)",
                params![
                    target_category_id,
                    i64::try_from(ordinal).map_err(|_| "too many merged tags".to_string())?,
                    tag,
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[derive(Clone, Debug)]''',
    "replace tags",
)
# Exact test fixture SQL shapes.
text = text.replace(
    "payload_json, updated_at_utc, legacy_import_id\n                 ) VALUES (1, 'default', 1, 2, 1, ?1, ?2, NULL)",
    "payload_json, updated_at_utc\n                 ) VALUES (1, 'default', 1, 2, 1, ?1, ?2)",
)
text = text.replace(
    "payload_json, captured_at_utc, legacy_import_id\n                 ) VALUES ('default', ?1, ?2, 1, ?3, ?4, NULL)",
    "payload_json, captured_at_utc\n                 ) VALUES ('default', ?1, ?2, 1, ?3, ?4)",
)
text = text.replace(
    "active_session_stable_id, payload_json, legacy_import_id\n                 ) VALUES (1, ?1, ?2, ?3, ?4, ?5, NULL)",
    "active_session_stable_id, payload_json\n                 ) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
)
path.write_text(text)

# Remove import-only columns from all SQLite statements.
for file_name in [
    "src/sqlite/runtime_coordination.rs",
    "src/sqlite/repository.rs",
    "src/sqlite/tui_runtime.rs",
]:
    path = Path(file_name)
    text = path.read_text()
    text = re.sub(r",\s*legacy_import_id", "", text)
    text = re.sub(r",\s*NULL\)(?=\s*(?:ON CONFLICT|\"|,|;))", ")", text)
    text = re.sub(r",?\s*legacy_import_id = NULL", "", text)
    text = re.sub(r",\s*legacy_import_id\s*\n", "\n", text)
    text = re.sub(r"\n\s*\.transition_storage_authority\(.*?\)\n\s*\.unwrap\(\);", "", text, flags=re.S)
    path.write_text(text)

# Remove activation-only fault proof.
path = Path("src/sqlite/fault_certification.rs")
text = path.read_text()
text = re.sub(
    r"\n\s*repository\n\s*\.transition_storage_authority\(.*?\)\n\s*\.unwrap\(\);",
    "",
    text,
    flags=re.S,
)
path.write_text(text)

# Doctor validates only current product tables and has no pending-import concept.
path = Path("src/sqlite/maintenance.rs")
text = path.read_text()
text = text.replace('        "schema_migrations",\n', "")
text = text.replace('        "legacy_imports",\n', "")
text = sub_once(
    text,
    r"\n    let pending_imports = if existing_tables\.contains\(\"legacy_imports\"\) \{.*?\n    \}\);\n",
    "\n",
    "pending import doctor check",
)
path.write_text(text)

# CSV crate is no longer part of the implementation.
path = Path("Cargo.toml")
text = path.read_text()
text = re.sub(r"^csv\s*=.*\n", "", text, flags=re.M)
path.write_text(text)

# Hard schema residue gate.
forbidden = [
    "legacy_import_id",
    "legacy_imports",
    "schema_migrations",
    "MIGRATION_1",
    "MIGRATION_2",
    "MIGRATION_3",
    "MIGRATION_4",
    "MIGRATION_5",
    "MIGRATION_6",
    "MIGRATION_7",
    "transition_storage_authority",
    "sqlite_cli_activated_at_utc",
]
violations = []
for source in list(Path("src").rglob("*.rs")) + [Path("Cargo.toml")]:
    content = source.read_text()
    for token in forbidden:
        if token in content:
            violations.append(f"{source}:{token}")
if violations:
    raise SystemExit("schema compatibility residue remains:\n" + "\n".join(violations))

print("current schema reset applied")
