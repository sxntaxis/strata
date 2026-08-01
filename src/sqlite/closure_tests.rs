use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::params;
use serde::Serialize;

use super::{
    BundleExportOptions, BundleImportOptions, LegacyEvidenceArchiveOptions,
    LegacyEvidenceInventoryOptions, LegacyEvidenceRemoveOptions, SqliteRepository,
    legacy_disposition, run_bundle_export, run_bundle_import,
};

fn unique_root(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "strata-sqlite012-{name}-{}-{nonce}",
        std::process::id()
    ))
}

fn remove_database(path: &Path) {
    fs::remove_file(path).ok();
    fs::remove_file(format!("{}-wal", path.display())).ok();
    fs::remove_file(format!("{}-shm", path.display())).ok();
}

#[test]
fn bundle_import_dry_run_uses_full_validation_without_publishing() {
    let root = unique_root("dry-run");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("source.sqlite3");
    let repository = SqliteRepository::open(&source).unwrap();
    drop(repository);
    let bundle = root.join("bundle");
    run_bundle_export(BundleExportOptions {
        database_path: source.clone(),
        output_directory: bundle.clone(),
    })
    .unwrap();

    let target = root.join("uncreated/target.sqlite3");
    let report = run_bundle_import(BundleImportOptions {
        bundle_directory: bundle.clone(),
        database_path: target.clone(),
        dry_run: true,
    })
    .unwrap();

    assert_eq!(report.status, "validated");
    assert!(report.target_path.is_none());
    assert!(!target.exists());
    assert!(!target.parent().unwrap().exists());

    let imported = root.join("imported.sqlite3");
    run_bundle_import(BundleImportOptions {
        bundle_directory: bundle,
        database_path: imported.clone(),
        dry_run: false,
    })
    .unwrap();
    assert!(imported.exists());

    remove_database(&source);
    remove_database(&imported);
    fs::remove_dir_all(root).ok();
}

#[derive(Serialize)]
struct TestMarker<'a> {
    schema_version: u8,
    active_authority: &'a str,
    sqlite_candidate: TestCandidate<'a>,
    sqlite_cli_activation: TestActivation<'a>,
}

#[derive(Serialize)]
struct TestCandidate<'a> {
    status: &'a str,
    source_fingerprint: &'a str,
    database_path: String,
    backup_path: String,
    report_path: String,
    verified_at_utc: &'a str,
}

#[derive(Serialize)]
struct TestActivation<'a> {
    status: &'a str,
    previous_authority: &'a str,
    source_fingerprint: &'a str,
    database_path: String,
    started_at_utc: &'a str,
    completed_at_utc: Option<&'a str>,
}

#[derive(Serialize)]
struct TestProvenance<'a> {
    schema_version: u8,
    source_fingerprint: &'a str,
    copied_at_utc: &'a str,
    files: Vec<TestProvenanceEntry>,
}

#[derive(Serialize)]
struct TestProvenanceEntry {
    logical_name: String,
    original_path: String,
    byte_count: u64,
}

struct EvidenceFixture {
    root: PathBuf,
    marker: PathBuf,
    database: PathBuf,
    live_categories: PathBuf,
    live_sessions: PathBuf,
    archive: PathBuf,
    fingerprint: String,
}

impl EvidenceFixture {
    fn create() -> Self {
        let root = unique_root("evidence");
        let data = root.join("live-data");
        let state = root.join("live-state");
        let backup = state.join("storage_migration/backups/test-fingerprint");
        fs::create_dir_all(backup.join("data")).unwrap();
        fs::create_dir_all(backup.join("state")).unwrap();
        fs::create_dir_all(&data).unwrap();

        let live_categories = data.join("categories.csv");
        let live_sessions = data.join("time_log.csv");
        let categories = b"id,name,description,color_index,karma_effect\n1,Work,,0,1\n";
        let sessions =
            b"id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n";
        fs::write(&live_categories, categories).unwrap();
        fs::write(&live_sessions, sessions).unwrap();
        fs::write(backup.join("data/categories.csv"), categories).unwrap();
        fs::write(backup.join("data/time_log.csv"), sessions).unwrap();

        let fingerprint = "test-fingerprint".to_string();
        let provenance = TestProvenance {
            schema_version: 1,
            source_fingerprint: &fingerprint,
            copied_at_utc: "2026-08-01T20:00:00Z",
            files: vec![
                TestProvenanceEntry {
                    logical_name: "categories.csv".to_string(),
                    original_path: live_categories.to_string_lossy().into_owned(),
                    byte_count: u64::try_from(categories.len()).unwrap(),
                },
                TestProvenanceEntry {
                    logical_name: "time_log.csv".to_string(),
                    original_path: live_sessions.to_string_lossy().into_owned(),
                    byte_count: u64::try_from(sessions.len()).unwrap(),
                },
            ],
        };
        fs::write(
            backup.join("source_paths.json"),
            serde_json::to_vec_pretty(&provenance).unwrap(),
        )
        .unwrap();

        let database = data.join("strata.sqlite3");
        let repository = SqliteRepository::open(&database).unwrap();
        repository
            .connection
            .execute(
                "UPDATE database_metadata SET value = 'sqlite-cli' WHERE key = 'storage_authority'",
                [],
            )
            .unwrap();
        for (key, value) in [
            ("legacy_import_fingerprint", fingerprint.as_str()),
            ("legacy_import_status", "verified"),
        ] {
            repository
                .connection
                .execute(
                    "INSERT INTO database_metadata(key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )
                .unwrap();
        }
        drop(repository);

        let marker = state.join("storage_authority.json");
        let marker_value = TestMarker {
            schema_version: 1,
            active_authority: "sqlite-cli",
            sqlite_candidate: TestCandidate {
                status: "verified",
                source_fingerprint: &fingerprint,
                database_path: database.to_string_lossy().into_owned(),
                backup_path: backup.to_string_lossy().into_owned(),
                report_path: state.join("report.json").to_string_lossy().into_owned(),
                verified_at_utc: "2026-08-01T20:00:00Z",
            },
            sqlite_cli_activation: TestActivation {
                status: "active",
                previous_authority: "legacy-files",
                source_fingerprint: &fingerprint,
                database_path: database.to_string_lossy().into_owned(),
                started_at_utc: "2026-08-01T20:01:00Z",
                completed_at_utc: Some("2026-08-01T20:01:01Z"),
            },
        };
        fs::write(&marker, serde_json::to_vec_pretty(&marker_value).unwrap()).unwrap();

        Self {
            archive: root.join("custody/legacy-evidence"),
            root,
            marker,
            database,
            live_categories,
            live_sessions,
            fingerprint,
        }
    }
}

impl Drop for EvidenceFixture {
    fn drop(&mut self) {
        remove_database(&self.database);
        fs::remove_dir_all(&self.root).ok();
    }
}

#[test]
fn legacy_evidence_archive_and_interrupted_removal_are_retry_safe() {
    let fixture = EvidenceFixture::create();
    let inventory = legacy_disposition::inventory(LegacyEvidenceInventoryOptions {
        authority_marker_path: fixture.marker.clone(),
    })
    .unwrap();
    assert!(inventory.is_healthy());
    assert_eq!(inventory.files.len(), 2);

    let archived = legacy_disposition::archive(LegacyEvidenceArchiveOptions {
        authority_marker_path: fixture.marker.clone(),
        output_directory: fixture.archive.clone(),
        confirm: true,
    })
    .unwrap();
    assert_eq!(archived.status, "archived");
    assert!(
        fixture
            .archive
            .join("legacy_evidence_manifest.json")
            .exists()
    );

    let repeated = legacy_disposition::archive(LegacyEvidenceArchiveOptions {
        authority_marker_path: fixture.marker.clone(),
        output_directory: fixture.archive.clone(),
        confirm: true,
    })
    .unwrap();
    assert_eq!(repeated.status, "already-archived");

    let first = legacy_disposition::remove_with_test_failure(
        LegacyEvidenceRemoveOptions {
            authority_marker_path: fixture.marker.clone(),
            archive_directory: fixture.archive.clone(),
            confirm_fingerprint: fixture.fingerprint.clone(),
        },
        1,
    );
    assert!(first.is_err());
    assert!(!fixture.live_categories.exists());
    assert!(fixture.live_sessions.exists());

    let recovered = legacy_disposition::remove(LegacyEvidenceRemoveOptions {
        authority_marker_path: fixture.marker.clone(),
        archive_directory: fixture.archive.clone(),
        confirm_fingerprint: fixture.fingerprint.clone(),
    })
    .unwrap();
    assert_eq!(recovered.status, "recovered-removal");
    assert!(!fixture.live_categories.exists());
    assert!(!fixture.live_sessions.exists());
    assert!(fixture.database.exists());
}

#[test]
fn legacy_evidence_disposition_refuses_changed_sources_and_wrong_confirmation() {
    let fixture = EvidenceFixture::create();
    fs::write(&fixture.live_categories, b"changed\n").unwrap();

    let error = legacy_disposition::archive(LegacyEvidenceArchiveOptions {
        authority_marker_path: fixture.marker.clone(),
        output_directory: fixture.archive.clone(),
        confirm: true,
    })
    .unwrap_err();
    assert!(error.contains("differs from the verified migration backup"));

    fs::write(
        &fixture.live_categories,
        b"id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
    )
    .unwrap();
    legacy_disposition::archive(LegacyEvidenceArchiveOptions {
        authority_marker_path: fixture.marker.clone(),
        output_directory: fixture.archive.clone(),
        confirm: true,
    })
    .unwrap();
    let error = legacy_disposition::remove(LegacyEvidenceRemoveOptions {
        authority_marker_path: fixture.marker.clone(),
        archive_directory: fixture.archive.clone(),
        confirm_fingerprint: "wrong".to_string(),
    })
    .unwrap_err();
    assert!(error.contains(&fixture.fingerprint));
    assert!(fixture.live_categories.exists());
    assert!(fixture.live_sessions.exists());
}
