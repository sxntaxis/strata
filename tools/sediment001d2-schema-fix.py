from pathlib import Path


path = Path("src/sqlite.rs")
text = path.read_text()

text = text.replace("const CURRENT_SCHEMA_VERSION: i64 = 5;", "const CURRENT_SCHEMA_VERSION: i64 = 6;", 1)

anchor = """#[derive(Debug, Error)]
pub(crate) enum SqliteStoreError {
"""
migration = r'''const MIGRATION_6: &str = r#"
CREATE TABLE sand_snapshots_v6 (
    id INTEGER PRIMARY KEY,
    formation_id TEXT NOT NULL,
    snapshot_kind TEXT NOT NULL
        CHECK (snapshot_kind IN (
            'daily', 'daily-contribution', 'manual', 'formation_end', 'recovery'
        )),
    operational_day TEXT,
    quantum_seconds INTEGER NOT NULL CHECK (quantum_seconds > 0),
    payload_json TEXT NOT NULL,
    captured_at_utc TEXT NOT NULL,
    legacy_import_id INTEGER
        REFERENCES legacy_imports(id) ON DELETE RESTRICT
) STRICT;

INSERT INTO sand_snapshots_v6 (
    id, formation_id, snapshot_kind, operational_day, quantum_seconds,
    payload_json, captured_at_utc, legacy_import_id
)
SELECT
    id, formation_id, snapshot_kind, operational_day, quantum_seconds,
    payload_json, captured_at_utc, legacy_import_id
FROM sand_snapshots;

DROP TABLE sand_snapshots;
ALTER TABLE sand_snapshots_v6 RENAME TO sand_snapshots;

CREATE INDEX sand_snapshots_formation_index
    ON sand_snapshots(formation_id, captured_at_utc);
CREATE INDEX sand_snapshots_legacy_import_index
    ON sand_snapshots(legacy_import_id, id);

INSERT INTO schema_migrations(version, applied_at_utc)
VALUES (6, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

PRAGMA user_version = 6;
"#;

'''
if text.count(anchor) != 1:
    raise SystemExit("SQLite error enum anchor was not found")
text = text.replace(anchor, migration + anchor, 1)

old_apply = '''        if version < 5 {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute_batch(MIGRATION_5)?;
            transaction.commit()?;
        }

        Ok(Self { connection })
'''
new_apply = '''        if version < 5 {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute_batch(MIGRATION_5)?;
            transaction.commit()?;
            version = 5;
        }

        if version < 6 {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute_batch(MIGRATION_6)?;
            transaction.commit()?;
        }

        Ok(Self { connection })
'''
if text.count(old_apply) != 1:
    raise SystemExit("SQLite migration application block was not found")
text = text.replace(old_apply, new_apply, 1)

text = text.replace("repository.schema_version().unwrap(), 5", "repository.schema_version().unwrap(), 6")

anchor = '''    #[test]
    fn foreign_keys_are_enforced() {
'''
proof = r'''    #[test]
    fn version_five_snapshot_schema_upgrades_without_losing_legacy_evidence() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATION_1).unwrap();
        connection.execute_batch(MIGRATION_2).unwrap();
        connection.execute_batch(MIGRATION_3).unwrap();
        connection.execute_batch(MIGRATION_4).unwrap();
        connection.execute_batch(MIGRATION_5).unwrap();
        connection
            .execute(
                "INSERT INTO sand_snapshots (
                    formation_id, snapshot_kind, operational_day, quantum_seconds,
                    payload_json, captured_at_utc, legacy_import_id
                 ) VALUES ('default', 'daily', '2026-08-01', 1, '{}',
                           '2026-08-01T12:00:00Z', NULL)",
                [],
            )
            .unwrap();

        let repository =
            SqliteRepository::from_connection(connection).expect("migration should succeed");

        assert_eq!(repository.schema_version().unwrap(), 6);
        let legacy_count: i64 = repository
            .connection
            .query_row(
                "SELECT count(*) FROM sand_snapshots WHERE snapshot_kind = 'daily'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_count, 1);
        repository
            .connection
            .execute(
                "INSERT INTO sand_snapshots (
                    formation_id, snapshot_kind, operational_day, quantum_seconds,
                    payload_json, captured_at_utc, legacy_import_id
                 ) VALUES ('default', 'daily-contribution', '2026-08-01', 1, '{}',
                           '2026-08-01T13:00:00Z', NULL)",
                [],
            )
            .expect("schema 6 must accept typed daily contributions");
    }

'''
if text.count(anchor) != 1:
    raise SystemExit("SQLite migration-test anchor was not found")
path.write_text(text.replace(anchor, proof + anchor, 1))

Path(__file__).unlink(missing_ok=True)
