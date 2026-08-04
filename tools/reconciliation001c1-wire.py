from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:160]!r}")
    file.write_text(content.replace(old, new, 1))


replace_once(
    "src/lib.rs",
    "mod cli;\nmod constants;\n",
    "mod category_lifecycle;\nmod cli;\nmod constants;\n",
)

replace_once(
    "src/sqlite.rs",
    "mod authority;\nmod cli_runtime;\n",
    "mod authority;\nmod category_lifecycle;\nmod cli_runtime;\n",
)
replace_once(
    "src/sqlite.rs",
    "pub(crate) use authority::{\n    RuntimeAuthority, SqliteCliActivationOptions, activate_sqlite_cli, resolve_runtime_authority,\n};\n",
    "pub(crate) use authority::{\n    RuntimeAuthority, SqliteCliActivationOptions, activate_sqlite_cli, resolve_runtime_authority,\n};\n#[allow(unused_imports)]\npub(crate) use category_lifecycle::{\n    CategoryLifecyclePreview, CategoryLifecycleReceipt, CategoryLifecycleRequest,\n    CategoryReferenceCounts, apply as apply_category_lifecycle, preview as preview_category_lifecycle,\n};\n",
)
replace_once(
    "src/sqlite.rs",
    "const CURRENT_SCHEMA_VERSION: i64 = 6;",
    "const CURRENT_SCHEMA_VERSION: i64 = 7;",
)
replace_once(
    "src/sqlite.rs",
    '''const MIGRATION_6: &str = r#"\nCREATE TABLE sand_snapshots_v6 (\n''',
    '''const MIGRATION_6: &str = r#"\nCREATE TABLE sand_snapshots_v6 (\n''',
)
marker = '''PRAGMA user_version = 6;\n"#;\n\n#[derive(Debug, Error)]\n'''
migration = '''PRAGMA user_version = 6;\n"#;\n\nconst MIGRATION_7: &str = r#"\nCREATE TABLE category_lifecycle_receipts (\n    operation_id TEXT PRIMARY KEY,\n    operation_kind TEXT NOT NULL CHECK (operation_kind IN ('merge', 'delete')),\n    source_category_id INTEGER NOT NULL,\n    target_category_id INTEGER,\n    source_metadata_json TEXT NOT NULL,\n    target_metadata_json TEXT,\n    preview_revision TEXT NOT NULL,\n    reference_counts_json TEXT NOT NULL,\n    applied_at_utc TEXT NOT NULL,\n    CHECK (\n        (operation_kind = 'merge' AND target_category_id IS NOT NULL)\n        OR (operation_kind = 'delete' AND target_category_id IS NULL)\n    ),\n    CHECK (target_category_id IS NULL OR target_category_id != source_category_id)\n) STRICT;\n\nCREATE UNIQUE INDEX category_lifecycle_receipts_preview_unique\n    ON category_lifecycle_receipts(\n        source_category_id,\n        COALESCE(target_category_id, -1),\n        preview_revision\n    );\n\nINSERT INTO schema_migrations(version, applied_at_utc)\nVALUES (7, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));\n\nPRAGMA user_version = 7;\n"#;\n\n#[derive(Debug, Error)]\n'''
replace_once("src/sqlite.rs", marker, migration)
replace_once(
    "src/sqlite.rs",
    '''        if version < 6 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_6)?;\n            transaction.commit()?;\n        }\n\n        Ok(Self { connection })\n''',
    '''        if version < 6 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_6)?;\n            transaction.commit()?;\n            version = 6;\n        }\n\n        if version < 7 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_7)?;\n            transaction.commit()?;\n        }\n\n        Ok(Self { connection })\n''',
)

replace_once(
    "src/sqlite/category_lifecycle.rs",
    "            let mut current: SedimentSnapshot = serde_json::from_str(&payload_json).map_err(|error| {",
    "            let current: SedimentSnapshot = serde_json::from_str(&payload_json).map_err(|error| {",
)
