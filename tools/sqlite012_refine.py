from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


# A validation-only CLI call must not resolve/create the default data directory.
replace_once(
    "src/cli.rs",
    "    let report = sqlite::run_bundle_import(sqlite::BundleImportOptions {\n        bundle_directory: bundle,\n        database_path: database.unwrap_or_else(default_sqlite_database_path),\n        dry_run,\n    })?;",
    "    let database_path = match database {\n        Some(path) => path,\n        None if dry_run => PathBuf::new(),\n        None => default_sqlite_database_path(),\n    };\n    let report = sqlite::run_bundle_import(sqlite::BundleImportOptions {\n        bundle_directory: bundle,\n        database_path,\n        dry_run,\n    })?;",
)

# Bind backup provenance to the source manifest durably stored in SQLite.
replace_once(
    "src/sqlite/legacy_disposition.rs",
    "    collections::BTreeSet,",
    "    collections::{BTreeMap, BTreeSet},",
)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    'const ARCHIVE_MANIFEST_FILENAME: &str = "legacy_evidence_manifest.json";\n',
    'const ARCHIVE_MANIFEST_FILENAME: &str = "legacy_evidence_manifest.json";\nconst ARCHIVE_INTENT_FILENAME: &str = ".strata_legacy_archive_intent.json";\n',
)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    "#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\nstruct ArchiveManifest {",
    "#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\nstruct ArchiveIntent {\n    schema_version: u8,\n    source_fingerprint: String,\n    output_path: String,\n}\n\n#[derive(Debug, Clone, Deserialize)]\nstruct StoredSourceManifest {\n    entries: Vec<StoredSourceManifestEntry>,\n}\n\n#[derive(Debug, Clone, Deserialize)]\nstruct StoredSourceManifestEntry {\n    logical_name: String,\n    path: String,\n    exists: bool,\n    byte_count: usize,\n    content_fingerprint: Option<String>,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\nstruct ArchiveManifest {",
)

old_staging = '''    let staging = archive_staging_path(&output, &context.source_fingerprint)?;
    if staging.exists() {
        validate_archive(&context, &staging)
            .map_err(|_| LegacyEvidenceError::StagingConflict(display_path(&staging)))?;
        ensure_parent(&output)?;
        fs::rename(&staging, &output)
            .map_err(|error| io_error("recovering archive publication", &output, error))?;
        sync_parent(&output)?;
        return Ok(build_report(
            &context,
            "sqlite-legacy-archive",
            "recovered-archive",
            Some(&output),
            None,
        ));
    }

    ensure_parent(&staging)?;
    fs::create_dir(&staging)
        .map_err(|error| io_error("creating archive staging directory", &staging, error))?;
'''
new_staging = '''    let staging = archive_staging_path(&output, &context.source_fingerprint)?;
    let mut recovered_partial = false;
    if staging.exists() {
        validate_archive_intent(&context, &staging, &output)
            .map_err(|_| LegacyEvidenceError::StagingConflict(display_path(&staging)))?;
        if validate_archive(&context, &staging).is_ok() {
            ensure_parent(&output)?;
            fs::rename(&staging, &output)
                .map_err(|error| io_error("recovering archive publication", &output, error))?;
            sync_parent(&output)?;
            return Ok(build_report(
                &context,
                "sqlite-legacy-archive",
                "recovered-archive",
                Some(&output),
                None,
            ));
        }
        fs::remove_dir_all(&staging)
            .map_err(|error| io_error("discarding interrupted archive staging", &staging, error))?;
        recovered_partial = true;
    }

    ensure_parent(&staging)?;
    fs::create_dir(&staging)
        .map_err(|error| io_error("creating archive staging directory", &staging, error))?;
    write_new_json(
        &staging.join(ARCHIVE_INTENT_FILENAME),
        &ArchiveIntent {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            source_fingerprint: context.source_fingerprint.clone(),
            output_path: display_path(&output),
        },
    )?;
'''
replace_once("src/sqlite/legacy_disposition.rs", old_staging, new_staging)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    '        "archived",\n        Some(&output),',
    '        if recovered_partial {\n            "recovered-archive"\n        } else {\n            "archived"\n        },\n        Some(&output),',
)

# Test seam creates a durable, owned partial stage exactly like a process crash would leave.
replace_once(
    "src/sqlite/legacy_disposition.rs",
    "#[cfg(test)]\npub(super) fn remove_with_test_failure(",
    "#[cfg(test)]\npub(super) fn create_test_partial_archive(\n    options: LegacyEvidenceArchiveOptions,\n) -> Result<(), String> {\n    let context = load_context(&options.authority_marker_path).map_err(|error| error.to_string())?;\n    require_all_live_matches(&context).map_err(|error| error.to_string())?;\n    let output = absolute_output_path(&options.output_directory).map_err(|error| error.to_string())?;\n    let staging = archive_staging_path(&output, &context.source_fingerprint)\n        .map_err(|error| error.to_string())?;\n    ensure_parent(&staging).map_err(|error| error.to_string())?;\n    fs::create_dir(&staging).map_err(|error| error.to_string())?;\n    write_new_json(\n        &staging.join(ARCHIVE_INTENT_FILENAME),\n        &ArchiveIntent {\n            schema_version: ARCHIVE_SCHEMA_VERSION,\n            source_fingerprint: context.source_fingerprint.clone(),\n            output_path: display_path(&output),\n        },\n    )\n    .map_err(|error| error.to_string())?;\n    let first = context.files.first().ok_or_else(|| \"no evidence files\".to_string())?;\n    let destination = staging.join(&first.archive_relative_path);\n    write_new_file(\n        &destination,\n        &read_regular_file(&first.backup_path, \"reading migration backup evidence\")\n            .map_err(|error| error.to_string())?,\n    )\n    .map_err(|error| error.to_string())?;\n    sync_directory(&staging).map_err(|error| error.to_string())\n}\n\n#[cfg(test)]\npub(super) fn remove_with_test_failure(",
)

replace_once(
    "src/sqlite/legacy_disposition.rs",
    "    verify_database(&database_path, &marker.sqlite_candidate.source_fingerprint)?;",
    "    let stored_manifest =\n        verify_database(&database_path, &marker.sqlite_candidate.source_fingerprint)?;",
)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    "    let mut names = BTreeSet::new();\n    let mut files = Vec::with_capacity(provenance.files.len());",
    "    let expected_existing = stored_manifest.values().filter(|entry| entry.exists).count();\n    if provenance.files.len() != expected_existing {\n        return Err(LegacyEvidenceError::InvalidProvenance(format!(\n            \"source_paths.json contains {} files but the verified import manifest contains {expected_existing}\",\n            provenance.files.len()\n        )));\n    }\n\n    let mut names = BTreeSet::new();\n    let mut files = Vec::with_capacity(provenance.files.len());",
)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    "        let original_path = PathBuf::from(&entry.original_path);",
    "        let stored = stored_manifest.get(&entry.logical_name).ok_or_else(|| {\n            LegacyEvidenceError::InvalidProvenance(format!(\n                \"{} is absent from the verified SQLite source manifest\",\n                entry.logical_name\n            ))\n        })?;\n        let stored_byte_count = u64::try_from(stored.byte_count).map_err(|_| {\n            LegacyEvidenceError::InvalidProvenance(format!(\n                \"{} exceeds supported size\",\n                entry.logical_name\n            ))\n        })?;\n        if !stored.exists\n            || stored.path != entry.original_path\n            || stored_byte_count != entry.byte_count\n        {\n            return Err(LegacyEvidenceError::InvalidProvenance(format!(\n                \"{} differs from the verified SQLite source manifest\",\n                entry.logical_name\n            )));\n        }\n        let original_path = PathBuf::from(&entry.original_path);",
)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    "        if byte_count != entry.byte_count {\n            return Err(LegacyEvidenceError::InvalidProvenance(format!(\n                \"{} byte count differs from provenance\",\n                entry.logical_name\n            )));\n        }",
    "        if byte_count != entry.byte_count\n            || stored.content_fingerprint.as_deref() != Some(fingerprint_bytes(&backup_bytes).as_str())\n        {\n            return Err(LegacyEvidenceError::InvalidProvenance(format!(\n                \"{} content differs from the verified SQLite source manifest\",\n                entry.logical_name\n            )));\n        }",
)

new_verify = r'''fn verify_database(
    path: &Path,
    fingerprint: &str,
) -> Result<BTreeMap<String, StoredSourceManifestEntry>, LegacyEvidenceError> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(LegacyEvidenceError::AuthorityVerification(format!(
            "integrity check returned {integrity}"
        )));
    }
    let metadata = |key: &str| -> Result<Option<String>, rusqlite::Error> {
        connection
            .query_row(
                "SELECT value FROM database_metadata WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
    };
    if metadata("storage_authority")?.as_deref() != Some("sqlite-cli")
        || metadata("legacy_import_fingerprint")?.as_deref() != Some(fingerprint)
        || metadata("legacy_import_status")?.as_deref() != Some("verified")
    {
        return Err(LegacyEvidenceError::AuthorityVerification(
            "database metadata does not match migration provenance".to_string(),
        ));
    }
    let manifest_json: String = connection
        .query_row(
            "SELECT source_manifest_json FROM legacy_imports
             WHERE source_fingerprint = ?1 AND status = 'verified'",
            params![fingerprint],
            |row| row.get(0),
        )
        .map_err(|error| {
            LegacyEvidenceError::AuthorityVerification(format!(
                "verified source manifest is unavailable: {error}"
            ))
        })?;
    let manifest: StoredSourceManifest = serde_json::from_str(&manifest_json).map_err(|error| {
        LegacyEvidenceError::AuthorityVerification(format!(
            "verified source manifest is invalid: {error}"
        ))
    })?;
    let mut entries = BTreeMap::new();
    for entry in manifest.entries {
        if entries.insert(entry.logical_name.clone(), entry).is_some() {
            return Err(LegacyEvidenceError::AuthorityVerification(
                "verified source manifest contains duplicate logical names".to_string(),
            ));
        }
    }
    Ok(entries)
}

'''
replace_between(
    "src/sqlite/legacy_disposition.rs",
    "fn verify_database(",
    "fn validate_archive(",
    new_verify,
)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    "fn validate_archive(context: &EvidenceContext, archive: &Path) -> Result<(), LegacyEvidenceError> {",
    "fn validate_archive_intent(\n    context: &EvidenceContext,\n    staging: &Path,\n    output: &Path,\n) -> Result<(), LegacyEvidenceError> {\n    let intent: ArchiveIntent = read_json(&staging.join(ARCHIVE_INTENT_FILENAME))?;\n    if intent.schema_version != ARCHIVE_SCHEMA_VERSION\n        || intent.source_fingerprint != context.source_fingerprint\n        || intent.output_path != display_path(output)\n    {\n        return Err(LegacyEvidenceError::StagingConflict(display_path(staging)));\n    }\n    Ok(())\n}\n\nfn validate_archive(context: &EvidenceContext, archive: &Path) -> Result<(), LegacyEvidenceError> {",
)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    '    format!("fnv1a64-{hash:016x}")',
    '    format!("{hash:016x}")',
)

# Fixture stores the same source manifest that a real verified import records.
replace_once(
    "src/sqlite/closure_tests.rs",
    "        drop(repository);",
    "        let source_manifest = serde_json::json!({\n            \"entries\": [\n                {\n                    \"logical_name\": \"categories.csv\",\n                    \"path\": live_categories.to_string_lossy(),\n                    \"exists\": true,\n                    \"byte_count\": categories.len(),\n                    \"content_fingerprint\": test_fingerprint(categories),\n                },\n                {\n                    \"logical_name\": \"time_log.csv\",\n                    \"path\": live_sessions.to_string_lossy(),\n                    \"exists\": true,\n                    \"byte_count\": sessions.len(),\n                    \"content_fingerprint\": test_fingerprint(sessions),\n                }\n            ]\n        });\n        repository\n            .connection\n            .execute(\n                \"INSERT INTO legacy_imports (\n                    source_fingerprint, status, source_manifest_json, utc_offset_seconds,\n                    operational_day_start_minutes, quantum_seconds, category_count, session_count,\n                    total_elapsed_seconds, active_session_present, checkpoint_present,\n                    sand_state_present, snapshot_count, verification_json, started_at_utc, completed_at_utc\n                 ) VALUES (?1, 'verified', ?2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, '{}', ?3, ?3)\",\n                params![\n                    fingerprint,\n                    source_manifest.to_string(),\n                    \"2026-08-01T20:00:00Z\"\n                ],\n            )\n            .unwrap();\n        drop(repository);",
)
replace_once(
    "src/sqlite/closure_tests.rs",
    "fn remove_database(path: &Path) {",
    "fn test_fingerprint(bytes: &[u8]) -> String {\n    let mut hash = 0xcbf29ce484222325_u64;\n    for byte in bytes {\n        hash ^= u64::from(*byte);\n        hash = hash.wrapping_mul(0x100000001b3);\n    }\n    format!(\"{hash:016x}\")\n}\n\nfn remove_database(path: &Path) {",
)
replace_once(
    "src/sqlite/closure_tests.rs",
    "    let archived = legacy_disposition::archive(LegacyEvidenceArchiveOptions {\n        authority_marker_path: fixture.marker.clone(),\n        output_directory: fixture.archive.clone(),\n        confirm: true,\n    })\n    .unwrap();\n    assert_eq!(archived.status, \"archived\");",
    "    legacy_disposition::create_test_partial_archive(LegacyEvidenceArchiveOptions {\n        authority_marker_path: fixture.marker.clone(),\n        output_directory: fixture.archive.clone(),\n        confirm: true,\n    })\n    .unwrap();\n    let archived = legacy_disposition::archive(LegacyEvidenceArchiveOptions {\n        authority_marker_path: fixture.marker.clone(),\n        output_directory: fixture.archive.clone(),\n        confirm: true,\n    })\n    .unwrap();\n    assert_eq!(archived.status, \"recovered-archive\");",
)

# Explicitly prove source_paths.json cannot redirect disposition away from the SQLite-recorded paths.
append = r'''

#[test]
fn legacy_evidence_inventory_rejects_tampered_path_provenance() {
    let fixture = EvidenceFixture::create();
    let provenance_path = fixture
        .marker
        .parent()
        .unwrap()
        .join("storage_migration/backups/test-fingerprint/source_paths.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&provenance_path).unwrap()).unwrap();
    value["files"][0]["original_path"] = serde_json::Value::String(
        fixture.root.join("other/categories.csv").to_string_lossy().into_owned(),
    );
    fs::write(&provenance_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();

    let error = legacy_disposition::inventory(LegacyEvidenceInventoryOptions {
        authority_marker_path: fixture.marker.clone(),
    })
    .unwrap_err();
    assert!(error.contains("verified SQLite source manifest"));
}
'''
Path("src/sqlite/closure_tests.rs").write_text(
    Path("src/sqlite/closure_tests.rs").read_text() + append
)
