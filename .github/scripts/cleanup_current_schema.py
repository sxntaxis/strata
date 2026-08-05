from pathlib import Path
import re


def sub_once(text: str, pattern: str, replacement: str, label: str) -> str:
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return result


# Replace self-referential compatibility assertions with positive exact-schema proofs.
path = Path("src/sqlite.rs")
text = path.read_text()
text = sub_once(
    text,
    r"    #\[test\]\n    fn current_schema_has_no_compatibility_tables_or_columns\(\) \{.*?\n    \}\n\n    #\[test\]\n    fn non_current_database_version_is_rejected",
    '''    #[test]
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
                "category_lifecycle_receipts",
                "category_tags",
                "database_metadata",
                "projects",
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
                    "id", "stable_id", "project", "category_id", "description",
                    "started_at_utc", "ended_at_utc", "operational_day",
                    "elapsed_seconds", "source", "boundary_utc_offset_seconds",
                    "boundary_start_minutes",
                ],
            ),
            (
                "active_session",
                vec![
                    "singleton", "stable_id", "project", "category_id", "description",
                    "started_at_utc", "recovery_kind",
                ],
            ),
            (
                "runtime_checkpoint",
                vec![
                    "singleton", "status", "detached_at_utc", "simulation_time_utc",
                    "active_session_stable_id", "payload_json",
                ],
            ),
            (
                "sand_state",
                vec![
                    "singleton", "formation_id", "quantum_seconds", "grid_width",
                    "grid_height", "payload_json", "updated_at_utc",
                ],
            ),
            (
                "sand_snapshots",
                vec![
                    "id", "formation_id", "snapshot_kind", "operational_day",
                    "quantum_seconds", "payload_json", "captured_at_utc",
                ],
            ),
            (
                "category_tags",
                vec!["category_id", "ordinal", "tag"],
            ),
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
    fn non_current_database_version_is_rejected''',
    "positive schema proof",
)
# Delete any old upgrade tests if an earlier pattern left them behind.
text = re.sub(
    r"\n    #\[test\]\n    fn version_one_database_is_upgraded_without_losing_history\(\) \{.*?\n    \}\n",
    "\n",
    text,
    flags=re.S,
)
text = re.sub(
    r"\n    #\[test\]\n    fn version_five_snapshot_schema_upgrades_without_losing_legacy_evidence\(\) \{.*?\n    \}\n",
    "\n",
    text,
    flags=re.S,
)
path.write_text(text)

# Normalize category lifecycle hashing and merged tags to the current columns only.
path = Path("src/sqlite/category_lifecycle.rs")
text = path.read_text()
replacements = {
    "SELECT category_id, ordinal, tag, legacy_import_id": "SELECT category_id, ordinal, tag",
    "payload_json, updated_at_utc, legacy_import_id": "payload_json, updated_at_utc",
    "payload_json, captured_at_utc, legacy_import_id": "payload_json, captured_at_utc",
    "active_session_stable_id, payload_json, legacy_import_id": "active_session_stable_id, payload_json",
}
for old, new in replacements.items():
    text = text.replace(old, new)
text = text.replace("        4,\n    )?;", "        3,\n    )?;", 1)
# The next three ordered hashes previously included one import column each.
text = text.replace("        8,\n    )?;", "        7,\n    )?;", 2)
text = text.replace("        7,\n    )?;", "        6,\n    )?;", 1)
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
    "current merged tags",
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
    "current tag replacement",
)
# Fixture inserts now match the reduced current tables.
text = re.sub(r",\s*legacy_import_id", "", text)
text = re.sub(r",\s*NULL\)(?=\s*(?:\"|,|;))", ")", text)
path.write_text(text)

# Remove any remaining import provenance clauses from direct runtime SQL.
for file_name in [
    "src/sqlite/runtime_coordination.rs",
    "src/sqlite/repository.rs",
    "src/sqlite/tui_runtime.rs",
]:
    path = Path(file_name)
    text = path.read_text()
    text = re.sub(r",\s*legacy_import_id", "", text)
    text = re.sub(r",?\s*legacy_import_id = NULL", "", text)
    text = re.sub(r",\s*NULL\)(?=\s*(?:ON CONFLICT|\"|,|;))", ")", text)
    text = re.sub(
        r"\n\s*\.transition_storage_authority\(.*?\)\n\s*\.unwrap\(\);",
        "",
        text,
        flags=re.S,
    )
    path.write_text(text)

path = Path("src/sqlite/fault_certification.rs")
text = path.read_text()
text = re.sub(
    r"\n\s*repository\n\s*\.transition_storage_authority\(.*?\)\n\s*\.unwrap\(\);",
    "",
    text,
    flags=re.S,
)
path.write_text(text)

# Maintenance asserts the exact current product schema, with no import queue.
path = Path("src/sqlite/maintenance.rs")
text = path.read_text()
text = re.sub(r'^\s*"schema_migrations",\n', '', text, flags=re.M)
text = re.sub(r'^\s*"legacy_imports",\n', '', text, flags=re.M)
text = re.sub(
    r"\n    let pending_imports = if existing_tables\.contains\(\"legacy_imports\"\) \{.*?\n    \}\);\n",
    "\n",
    text,
    flags=re.S,
)
path.write_text(text)

# Final architectural certification.
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

print("current schema cleanup and certification passed")
