use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process,
};

use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{profile, storage};

use super::{
    SqliteRepository, SqliteStoreError,
    legacy_import::{
        LegacyImportError, LegacyImportOptions, LegacyImportOutcome, LegacyImportPaths,
        LegacyImportPlan, LegacyImportSummary,
    },
};

const REPORT_SCHEMA_VERSION: u8 = 1;
const AUTHORITY_MARKER_SCHEMA_VERSION: u8 = 1;
const DEFAULT_DATABASE_FILENAME: &str = "strata.sqlite3";
const MIGRATION_DIRECTORY_NAME: &str = "storage_migration";
const AUTHORITY_MARKER_FILENAME: &str = "storage_authority.json";
const LOCK_FILENAME: &str = "migration.lock";

#[derive(Debug, Clone)]
pub(crate) struct ControlledMigrationOptions {
    pub dry_run: bool,
    pub include_active_recovery: bool,
    pub database_path: Option<PathBuf>,
    pub report_path: Option<PathBuf>,
    pub utc_offset_seconds: i32,
    pub operational_day_start_minutes: u16,
    pub quantum_seconds: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ControlledMigrationStatus {
    DryRunVerified,
    Published,
    AlreadyPublished,
    RecoveredPublication,
}

impl ControlledMigrationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::DryRunVerified => "dry-run-verified",
            Self::Published => "published",
            Self::AlreadyPublished => "already-published",
            Self::RecoveredPublication => "recovered-publication",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ControlledMigrationReport {
    pub schema_version: u8,
    pub status: ControlledMigrationStatus,
    pub active_authority: String,
    pub sqlite_candidate_status: String,
    pub source_fingerprint: String,
    source_summary: LegacyImportSummary,
    pub database_path: Option<String>,
    pub backup_path: Option<String>,
    pub machine_report_path: Option<String>,
    pub authority_marker_path: Option<String>,
    pub integrity_check: Option<String>,
    pub utc_offset_seconds: i32,
    pub operational_day_start_minutes: u16,
    pub quantum_seconds: i64,
    pub active_recovery_included: bool,
    pub started_at_utc: String,
    pub completed_at_utc: String,
}

impl ControlledMigrationReport {
    pub fn to_pretty_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn print_human(&self) {
        println!("SQLite migration: {}", self.status.as_str());
        println!("Live authority: {}", self.active_authority);
        println!("Source fingerprint: {}", self.source_fingerprint);
        println!("Categories: {}", self.source_summary.category_count);
        println!("Sessions: {}", self.source_summary.session_count);
        println!(
            "Elapsed seconds: {}",
            self.source_summary.total_elapsed_seconds
        );
        println!("Snapshots: {}", self.source_summary.snapshot_count);
        println!("Tags: {}", self.source_summary.tag_count);
        println!(
            "Active recovery: {}",
            if self.source_summary.active_session_present || self.source_summary.checkpoint_present
            {
                "present"
            } else {
                "none"
            }
        );
        if let Some(path) = &self.database_path {
            println!("SQLite candidate: {path}");
        }
        if let Some(path) = &self.backup_path {
            println!("Legacy backup: {path}");
        }
        if let Some(path) = &self.machine_report_path {
            println!("Machine report: {path}");
        }
        if let Some(path) = &self.authority_marker_path {
            println!("Authority marker: {path}");
        }
        if let Some(result) = &self.integrity_check {
            println!("Integrity check: {result}");
        }
    }
}

#[derive(Debug, Error)]
pub(super) enum ControlledMigrationError {
    #[error("legacy import validation failed: {0}")]
    Legacy(#[from] LegacyImportError),
    #[error("SQLite candidate failed: {0}")]
    Sqlite(#[from] SqliteStoreError),
    #[error("I/O error while {operation} {path}: {message}")]
    Io {
        operation: &'static str,
        path: String,
        message: String,
    },
    #[error("JSON error while {operation}: {message}")]
    Json {
        operation: &'static str,
        message: String,
    },
    #[error(
        "legacy state contains an active interval; stop tracking or pass --include-active-recovery"
    )]
    ActiveRecoveryRequiresOptIn,
    #[error("legacy authority changed during migration ({before} -> {after}); retry")]
    SourceChanged { before: String, after: String },
    #[error("legacy backup {path} does not match source fingerprint {expected}")]
    BackupMismatch { path: String, expected: String },
    #[error("SQLite target already exists but is not this verified migration: {path}")]
    ExistingDatabase { path: String },
    #[error("authority marker conflicts with this migration: {path}")]
    AuthorityMarkerConflict { path: String },
    #[error("another migration is active: {path}")]
    MigrationLocked { path: String },
    #[error("published SQLite candidate failed verification: {0}")]
    PublicationMismatch(String),
}

#[derive(Debug, Clone)]
struct MigrationLayout {
    data_dir: PathBuf,
    state_dir: PathBuf,
    sessions_csv: PathBuf,
    database_path: PathBuf,
    report_path_override: Option<PathBuf>,
}

impl MigrationLayout {
    fn runtime(options: &ControlledMigrationOptions) -> Result<Self, ControlledMigrationError> {
        let data_dir = absolute_path(storage::get_data_dir())?;
        let state_dir = absolute_path(storage::get_state_dir())?;
        let sessions_csv = absolute_path(storage::get_time_log_path())?;
        let database_path = absolute_path(
            options
                .database_path
                .clone()
                .unwrap_or_else(|| data_dir.join(DEFAULT_DATABASE_FILENAME)),
        )?;
        let report_path_override = options.report_path.clone().map(absolute_path).transpose()?;
        Ok(Self {
            data_dir,
            state_dir,
            sessions_csv,
            database_path,
            report_path_override,
        })
    }

    fn legacy_paths(&self) -> LegacyImportPaths {
        LegacyImportPaths::from_roots(&self.data_dir, &self.state_dir, self.sessions_csv.clone())
    }

    fn migration_root(&self) -> PathBuf {
        self.state_dir.join(MIGRATION_DIRECTORY_NAME)
    }

    fn authority_marker_path(&self) -> PathBuf {
        self.state_dir.join(AUTHORITY_MARKER_FILENAME)
    }

    fn report_path(&self, fingerprint: &str) -> PathBuf {
        self.report_path_override.clone().unwrap_or_else(|| {
            self.migration_root()
                .join("reports")
                .join(format!("{fingerprint}.json"))
        })
    }

    fn backup_path(&self, fingerprint: &str) -> PathBuf {
        self.migration_root().join("backups").join(fingerprint)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StorageAuthorityMarker {
    schema_version: u8,
    #[serde(default)]
    profile_id: Option<String>,
    active_authority: String,
    sqlite_candidate: SqliteCandidateMarker,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SqliteCandidateMarker {
    status: String,
    source_fingerprint: String,
    database_path: String,
    backup_path: String,
    report_path: String,
    verified_at_utc: String,
}

#[derive(Debug, Clone, Serialize)]
struct BackupProvenance {
    schema_version: u8,
    source_fingerprint: String,
    copied_at_utc: String,
    files: Vec<BackupProvenanceEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct BackupProvenanceEntry {
    logical_name: String,
    original_path: String,
    byte_count: u64,
}

#[derive(Debug, Clone)]
struct ExistingCandidate {
    summary: LegacyImportSummary,
    integrity_check: String,
}

pub(super) fn run_controlled_migration(
    options: ControlledMigrationOptions,
) -> Result<ControlledMigrationReport, ControlledMigrationError> {
    let layout = MigrationLayout::runtime(&options)?;
    execute_controlled_migration(&layout, &options)
}

fn execute_controlled_migration(
    layout: &MigrationLayout,
    options: &ControlledMigrationOptions,
) -> Result<ControlledMigrationReport, ControlledMigrationError> {
    let started_at_utc = now_utc();
    let import_options = LegacyImportOptions {
        utc_offset_seconds: options.utc_offset_seconds,
        operational_day_start_minutes: options.operational_day_start_minutes,
        quantum_seconds: options.quantum_seconds,
    };
    let legacy_paths = layout.legacy_paths();
    let initial_plan = stable_plan(&legacy_paths, import_options)?;
    enforce_active_recovery_policy(&initial_plan, options)?;

    if options.dry_run {
        return Ok(build_report(
            ControlledMigrationStatus::DryRunVerified,
            initial_plan.summary().clone(),
            options,
            None,
            None,
            None,
            None,
            None,
            started_at_utc,
        ));
    }

    fs::create_dir_all(layout.migration_root()).map_err(|error| {
        io_error(
            "creating migration directory",
            &layout.migration_root(),
            error,
        )
    })?;
    let _lock = MigrationLock::acquire(&layout.migration_root().join(LOCK_FILENAME))?;

    let plan = stable_plan(&legacy_paths, import_options)?;
    enforce_active_recovery_policy(&plan, options)?;
    let fingerprint = plan.summary().source_fingerprint.clone();

    let existing_candidate = inspect_existing_candidate(&layout.database_path, &fingerprint)?;
    let backup_path = ensure_immutable_backup(layout, &legacy_paths, &plan, import_options)?;
    let backup_plan =
        LegacyImportPlan::from_paths(&backup_legacy_paths(&backup_path), import_options)?;
    if backup_plan.summary() != plan.summary() {
        return Err(ControlledMigrationError::BackupMismatch {
            path: display_path(&backup_path),
            expected: fingerprint,
        });
    }

    let report_path = layout.report_path(&fingerprint);
    let authority_marker_path = layout.authority_marker_path();

    if let Some(existing) = existing_candidate {
        if existing.summary != *plan.summary() {
            return Err(ControlledMigrationError::PublicationMismatch(
                "existing candidate verification summary differs from the current source"
                    .to_string(),
            ));
        }
        let artifacts_existed = authority_marker_path.exists() && report_path.exists();
        validate_or_create_artifacts(
            layout,
            options,
            &existing.summary,
            &backup_path,
            &report_path,
            &authority_marker_path,
            &existing.integrity_check,
            ControlledMigrationStatus::RecoveredPublication,
            &started_at_utc,
        )?;
        return Ok(build_report(
            if artifacts_existed {
                ControlledMigrationStatus::AlreadyPublished
            } else {
                ControlledMigrationStatus::RecoveredPublication
            },
            existing.summary,
            options,
            Some(&layout.database_path),
            Some(&backup_path),
            Some(&report_path),
            Some(&authority_marker_path),
            Some(existing.integrity_check),
            started_at_utc,
        ));
    }

    if layout.database_path.exists() {
        return Err(ControlledMigrationError::ExistingDatabase {
            path: display_path(&layout.database_path),
        });
    }

    let temp_path = temporary_database_path(&layout.database_path);
    ensure_parent(&temp_path)?;
    remove_stale_database_files(&temp_path)?;
    let mut temp_guard = TemporaryDatabaseGuard::new(temp_path.clone());

    let _integrity_check = {
        let mut repository = SqliteRepository::open(&temp_path)?;
        let outcome = repository.import_legacy(&backup_plan)?;
        let imported_summary = match outcome {
            LegacyImportOutcome::Imported(summary)
            | LegacyImportOutcome::AlreadyImported(summary) => summary,
        };
        if imported_summary != *plan.summary() {
            return Err(ControlledMigrationError::PublicationMismatch(
                "import result differs from the validated source summary".to_string(),
            ));
        }
        let integrity = repository.integrity_check()?;
        if integrity != "ok" {
            return Err(ControlledMigrationError::PublicationMismatch(format!(
                "temporary database integrity check returned {integrity}"
            )));
        }
        prepare_database_for_publication(&repository)?;
        integrity
    };

    sync_file(&temp_path)?;
    if layout.database_path.exists() {
        return Err(ControlledMigrationError::ExistingDatabase {
            path: display_path(&layout.database_path),
        });
    }
    ensure_parent(&layout.database_path)?;
    fs::rename(&temp_path, &layout.database_path)
        .map_err(|error| io_error("publishing SQLite candidate", &layout.database_path, error))?;
    temp_guard.disarm();
    sync_parent(&layout.database_path)?;

    let published =
        inspect_existing_candidate(&layout.database_path, &fingerprint)?.ok_or_else(|| {
            ControlledMigrationError::PublicationMismatch(
                "published database has no verified matching import".to_string(),
            )
        })?;
    if published.summary != *plan.summary() {
        return Err(ControlledMigrationError::PublicationMismatch(
            "published verification summary differs from source".to_string(),
        ));
    }

    validate_or_create_artifacts(
        layout,
        options,
        &published.summary,
        &backup_path,
        &report_path,
        &authority_marker_path,
        &published.integrity_check,
        ControlledMigrationStatus::Published,
        &started_at_utc,
    )?;

    Ok(build_report(
        ControlledMigrationStatus::Published,
        published.summary,
        options,
        Some(&layout.database_path),
        Some(&backup_path),
        Some(&report_path),
        Some(&authority_marker_path),
        Some(published.integrity_check),
        started_at_utc,
    ))
}

pub(super) fn verify_candidate_for_cli_activation(
    database_path: &Path,
    report: &ControlledMigrationReport,
) -> Result<String, ControlledMigrationError> {
    let controlled_options = ControlledMigrationOptions {
        dry_run: true,
        include_active_recovery: true,
        database_path: Some(database_path.to_path_buf()),
        report_path: None,
        utc_offset_seconds: report.utc_offset_seconds,
        operational_day_start_minutes: report.operational_day_start_minutes,
        quantum_seconds: report.quantum_seconds,
    };
    let layout = MigrationLayout::runtime(&controlled_options)?;
    let import_options = LegacyImportOptions {
        utc_offset_seconds: report.utc_offset_seconds,
        operational_day_start_minutes: report.operational_day_start_minutes,
        quantum_seconds: report.quantum_seconds,
    };
    let plan = stable_plan(&layout.legacy_paths(), import_options)?;
    if plan.summary().source_fingerprint != report.source_fingerprint {
        return Err(ControlledMigrationError::SourceChanged {
            before: report.source_fingerprint.clone(),
            after: plan.summary().source_fingerprint.clone(),
        });
    }
    let existing = inspect_existing_candidate(database_path, &report.source_fingerprint)?
        .ok_or_else(|| {
            ControlledMigrationError::PublicationMismatch(
                "verified SQLite candidate is missing".to_string(),
            )
        })?;
    if existing.summary != *plan.summary() || existing.summary != report.source_summary {
        return Err(ControlledMigrationError::PublicationMismatch(
            "SQLite candidate no longer matches the live legacy authority".to_string(),
        ));
    }
    if existing.integrity_check != "ok" {
        return Err(ControlledMigrationError::PublicationMismatch(format!(
            "candidate integrity check returned {}",
            existing.integrity_check
        )));
    }
    Ok(existing.integrity_check)
}

fn stable_plan(
    paths: &LegacyImportPaths,
    options: LegacyImportOptions,
) -> Result<LegacyImportPlan, ControlledMigrationError> {
    let first = LegacyImportPlan::from_paths(paths, options)?;
    let second = LegacyImportPlan::from_paths(paths, options)?;
    let before = first.summary().source_fingerprint.clone();
    let after = second.summary().source_fingerprint.clone();
    if before != after {
        return Err(ControlledMigrationError::SourceChanged { before, after });
    }
    Ok(second)
}

fn enforce_active_recovery_policy(
    plan: &LegacyImportPlan,
    options: &ControlledMigrationOptions,
) -> Result<(), ControlledMigrationError> {
    if (plan.summary().active_session_present || plan.summary().checkpoint_present)
        && !options.include_active_recovery
    {
        return Err(ControlledMigrationError::ActiveRecoveryRequiresOptIn);
    }
    Ok(())
}

fn ensure_immutable_backup(
    layout: &MigrationLayout,
    live_paths: &LegacyImportPaths,
    expected_plan: &LegacyImportPlan,
    options: LegacyImportOptions,
) -> Result<PathBuf, ControlledMigrationError> {
    let fingerprint = &expected_plan.summary().source_fingerprint;
    let backup_path = layout.backup_path(fingerprint);
    if backup_path.exists() {
        let backup_plan =
            LegacyImportPlan::from_paths(&backup_legacy_paths(&backup_path), options)?;
        if backup_plan.summary() != expected_plan.summary() {
            return Err(ControlledMigrationError::BackupMismatch {
                path: display_path(&backup_path),
                expected: fingerprint.clone(),
            });
        }
        let live_after = LegacyImportPlan::from_paths(live_paths, options)?;
        if live_after.summary().source_fingerprint != *fingerprint {
            return Err(ControlledMigrationError::SourceChanged {
                before: fingerprint.clone(),
                after: live_after.summary().source_fingerprint.clone(),
            });
        }
        return Ok(backup_path);
    }

    let parent = backup_path
        .parent()
        .ok_or_else(|| ControlledMigrationError::Io {
            operation: "resolving backup parent",
            path: display_path(&backup_path),
            message: "path has no parent".to_string(),
        })?;
    fs::create_dir_all(parent)
        .map_err(|error| io_error("creating backup parent", parent, error))?;
    let staging_path = parent.join(format!(".{}.partial-{}", fingerprint, process::id()));
    if staging_path.exists() {
        fs::remove_dir_all(&staging_path)
            .map_err(|error| io_error("removing stale backup staging", &staging_path, error))?;
    }
    fs::create_dir_all(staging_path.join("data"))
        .map_err(|error| io_error("creating backup data directory", &staging_path, error))?;
    fs::create_dir_all(staging_path.join("state/sand_history"))
        .map_err(|error| io_error("creating backup state directory", &staging_path, error))?;

    let source_files = existing_source_files(live_paths)?;
    let mut provenance_entries = Vec::new();
    for source in source_files {
        let destination = backup_destination(&staging_path, &source.logical_name);
        ensure_parent(&destination)?;
        let bytes = fs::read(&source.path)
            .map_err(|error| io_error("reading legacy source", &source.path, error))?;
        write_new_file(&destination, &bytes)?;
        provenance_entries.push(BackupProvenanceEntry {
            logical_name: source.logical_name,
            original_path: display_path(&source.path),
            byte_count: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }

    let provenance = BackupProvenance {
        schema_version: 1,
        source_fingerprint: fingerprint.clone(),
        copied_at_utc: now_utc(),
        files: provenance_entries,
    };
    write_new_json_atomic(&staging_path.join("source_paths.json"), &provenance)?;

    let staging_plan = LegacyImportPlan::from_paths(&backup_legacy_paths(&staging_path), options)?;
    if staging_plan.summary() != expected_plan.summary() {
        return Err(ControlledMigrationError::BackupMismatch {
            path: display_path(&staging_path),
            expected: fingerprint.clone(),
        });
    }

    let live_after = LegacyImportPlan::from_paths(live_paths, options)?;
    if live_after.summary().source_fingerprint != *fingerprint {
        return Err(ControlledMigrationError::SourceChanged {
            before: fingerprint.clone(),
            after: live_after.summary().source_fingerprint.clone(),
        });
    }

    fs::rename(&staging_path, &backup_path)
        .map_err(|error| io_error("publishing legacy backup", &backup_path, error))?;
    sync_parent(&backup_path)?;
    Ok(backup_path)
}

#[derive(Debug)]
struct SourceFile {
    logical_name: String,
    path: PathBuf,
}

fn existing_source_files(
    paths: &LegacyImportPaths,
) -> Result<Vec<SourceFile>, ControlledMigrationError> {
    let mut files = Vec::new();
    let fixed = [
        ("categories.csv", &paths.categories_csv),
        ("time_log.csv", &paths.sessions_csv),
        ("active_session.json", &paths.active_session_json),
        ("detached_runtime.json", &paths.detached_runtime_json),
        ("sand_state.json", &paths.sand_state_json),
        ("category_tags.json", &paths.category_tags_json),
    ];
    for (logical_name, path) in fixed {
        if path.is_file() {
            files.push(SourceFile {
                logical_name: logical_name.to_string(),
                path: path.clone(),
            });
        }
    }

    if paths.sand_history_dir.exists() {
        let mut entries = fs::read_dir(&paths.sand_history_dir)
            .map_err(|error| io_error("reading sand history", &paths.sand_history_dir, error))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| io_error("reading sand history", &paths.sand_history_dir, error))?;
        entries.sort();
        for path in entries {
            if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("json")
            {
                continue;
            }
            let filename = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| ControlledMigrationError::Io {
                    operation: "reading sand-history filename",
                    path: display_path(&path),
                    message: "filename is not UTF-8".to_string(),
                })?;
            files.push(SourceFile {
                logical_name: format!("sand_history/{filename}"),
                path,
            });
        }
    }
    files.sort_by(|left, right| left.logical_name.cmp(&right.logical_name));
    Ok(files)
}

fn backup_destination(root: &Path, logical_name: &str) -> PathBuf {
    match logical_name {
        "categories.csv" | "time_log.csv" => root.join("data").join(logical_name),
        _ => root.join("state").join(logical_name),
    }
}

fn backup_legacy_paths(root: &Path) -> LegacyImportPaths {
    LegacyImportPaths::from_roots(
        &root.join("data"),
        &root.join("state"),
        root.join("data/time_log.csv"),
    )
}

fn inspect_existing_candidate(
    path: &Path,
    fingerprint: &str,
) -> Result<Option<ExistingCandidate>, ControlledMigrationError> {
    if !path.exists() {
        return Ok(None);
    }
    let connection = match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(connection) => connection,
        Err(_) => {
            return Err(ControlledMigrationError::ExistingDatabase {
                path: display_path(path),
            });
        }
    };
    let import_table_exists: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_schema
             WHERE type = 'table' AND name = 'legacy_imports'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        })?;
    if import_table_exists != 1 {
        return Err(ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        });
    }

    let integrity_check: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        })?;
    if integrity_check != "ok" {
        return Err(ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        });
    }

    let row = connection
        .query_row(
            "SELECT status, verification_json
             FROM legacy_imports
             WHERE source_fingerprint = ?1",
            params![fingerprint],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|_| ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        })?;
    let Some((status, verification_json)) = row else {
        return Err(ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        });
    };
    if status != "verified" {
        return Err(ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        });
    }
    let verification_json =
        verification_json.ok_or_else(|| ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        })?;
    let summary: LegacyImportSummary =
        serde_json::from_str(&verification_json).map_err(|error| {
            ControlledMigrationError::Json {
                operation: "reading SQLite verification summary",
                message: error.to_string(),
            }
        })?;

    let metadata: BTreeMap<String, String> = {
        let mut statement = connection
            .prepare(
                "SELECT key, value FROM database_metadata
                 WHERE key IN ('legacy_import_fingerprint', 'legacy_import_status')",
            )
            .map_err(|_| ControlledMigrationError::ExistingDatabase {
                path: display_path(path),
            })?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| ControlledMigrationError::ExistingDatabase {
                path: display_path(path),
            })?;
        rows.collect::<Result<_, _>>()
            .map_err(|_| ControlledMigrationError::ExistingDatabase {
                path: display_path(path),
            })?
    };
    if metadata
        .get("legacy_import_fingerprint")
        .map(String::as_str)
        != Some(fingerprint)
        || metadata.get("legacy_import_status").map(String::as_str) != Some("verified")
    {
        return Err(ControlledMigrationError::ExistingDatabase {
            path: display_path(path),
        });
    }

    Ok(Some(ExistingCandidate {
        summary,
        integrity_check,
    }))
}

#[allow(clippy::too_many_arguments)]
fn validate_or_create_artifacts(
    layout: &MigrationLayout,
    options: &ControlledMigrationOptions,
    summary: &LegacyImportSummary,
    backup_path: &Path,
    report_path: &Path,
    marker_path: &Path,
    integrity_check: &str,
    status: ControlledMigrationStatus,
    started_at_utc: &str,
) -> Result<(), ControlledMigrationError> {
    let report = build_report(
        status,
        summary.clone(),
        options,
        Some(&layout.database_path),
        Some(backup_path),
        Some(report_path),
        Some(marker_path),
        Some(integrity_check.to_string()),
        started_at_utc.to_string(),
    );
    ensure_report(report_path, &report)?;

    let marker = StorageAuthorityMarker {
        schema_version: AUTHORITY_MARKER_SCHEMA_VERSION,
        profile_id: Some(profile::profile_id()),
        active_authority: "legacy-files".to_string(),
        sqlite_candidate: SqliteCandidateMarker {
            status: "verified".to_string(),
            source_fingerprint: summary.source_fingerprint.clone(),
            database_path: display_path(&layout.database_path),
            backup_path: display_path(backup_path),
            report_path: display_path(report_path),
            verified_at_utc: report.completed_at_utc.clone(),
        },
    };
    ensure_authority_marker(marker_path, &marker)
}

fn ensure_report(
    path: &Path,
    report: &ControlledMigrationReport,
) -> Result<(), ControlledMigrationError> {
    if path.exists() {
        let bytes =
            fs::read(path).map_err(|error| io_error("reading migration report", path, error))?;
        let existing: ControlledMigrationReport =
            serde_json::from_slice(&bytes).map_err(|error| ControlledMigrationError::Json {
                operation: "reading migration report",
                message: error.to_string(),
            })?;
        if existing.source_fingerprint != report.source_fingerprint
            || existing.database_path != report.database_path
        {
            return Err(ControlledMigrationError::AuthorityMarkerConflict {
                path: display_path(path),
            });
        }
        return Ok(());
    }
    write_new_json_atomic(path, report)
}

fn ensure_authority_marker(
    path: &Path,
    marker: &StorageAuthorityMarker,
) -> Result<(), ControlledMigrationError> {
    if path.exists() {
        let bytes =
            fs::read(path).map_err(|error| io_error("reading authority marker", path, error))?;
        let existing: StorageAuthorityMarker =
            serde_json::from_slice(&bytes).map_err(|error| ControlledMigrationError::Json {
                operation: "reading authority marker",
                message: error.to_string(),
            })?;
        if existing.schema_version != marker.schema_version
            || existing.active_authority != "legacy-files"
            || existing.sqlite_candidate.source_fingerprint
                != marker.sqlite_candidate.source_fingerprint
            || existing.sqlite_candidate.database_path != marker.sqlite_candidate.database_path
        {
            return Err(ControlledMigrationError::AuthorityMarkerConflict {
                path: display_path(path),
            });
        }
        return Ok(());
    }
    write_new_json_atomic(path, marker)
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    status: ControlledMigrationStatus,
    summary: LegacyImportSummary,
    options: &ControlledMigrationOptions,
    database_path: Option<&Path>,
    backup_path: Option<&Path>,
    report_path: Option<&Path>,
    marker_path: Option<&Path>,
    integrity_check: Option<String>,
    started_at_utc: String,
) -> ControlledMigrationReport {
    ControlledMigrationReport {
        schema_version: REPORT_SCHEMA_VERSION,
        status,
        active_authority: "legacy-files".to_string(),
        sqlite_candidate_status: if status == ControlledMigrationStatus::DryRunVerified {
            "not-published".to_string()
        } else {
            "verified".to_string()
        },
        source_fingerprint: summary.source_fingerprint.clone(),
        source_summary: summary,
        database_path: database_path.map(display_path),
        backup_path: backup_path.map(display_path),
        machine_report_path: report_path.map(display_path),
        authority_marker_path: marker_path.map(display_path),
        integrity_check,
        utc_offset_seconds: options.utc_offset_seconds,
        operational_day_start_minutes: options.operational_day_start_minutes,
        quantum_seconds: options.quantum_seconds,
        active_recovery_included: options.include_active_recovery,
        started_at_utc,
        completed_at_utc: now_utc(),
    }
}

fn prepare_database_for_publication(
    repository: &SqliteRepository,
) -> Result<(), ControlledMigrationError> {
    repository
        .connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(SqliteStoreError::from)?;
    repository
        .connection
        .pragma_update(None, "journal_mode", "DELETE")
        .map_err(SqliteStoreError::from)?;
    repository
        .connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(SqliteStoreError::from)?;
    Ok(())
}

fn temporary_database_path(database_path: &Path) -> PathBuf {
    let filename = database_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(DEFAULT_DATABASE_FILENAME);
    database_path.with_file_name(format!(".{filename}.migrating-{}", process::id()))
}

fn remove_stale_database_files(path: &Path) -> Result<(), ControlledMigrationError> {
    for candidate in [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
    ] {
        if candidate.exists() {
            fs::remove_file(&candidate).map_err(|error| {
                io_error("removing stale temporary database", &candidate, error)
            })?;
        }
    }
    Ok(())
}

struct TemporaryDatabaseGuard {
    path: PathBuf,
    armed: bool,
}

impl TemporaryDatabaseGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryDatabaseGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = remove_stale_database_files(&self.path);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct LockRecord {
    pid: u32,
    created_at_utc: String,
}

struct MigrationLock {
    path: PathBuf,
}

impl MigrationLock {
    fn acquire(path: &Path) -> Result<Self, ControlledMigrationError> {
        ensure_parent(path)?;
        match create_lock_file(path) {
            Ok(()) => Ok(Self {
                path: path.to_path_buf(),
            }),
            Err(error) if error.kind() == ErrorKind::AlreadyExists && lock_is_stale(path) => {
                fs::remove_file(path).map_err(|remove_error| {
                    io_error("removing stale migration lock", path, remove_error)
                })?;
                create_lock_file(path).map_err(|create_error| {
                    io_error("creating migration lock", path, create_error)
                })?;
                Ok(Self {
                    path: path.to_path_buf(),
                })
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                Err(ControlledMigrationError::MigrationLocked {
                    path: display_path(path),
                })
            }
            Err(error) => Err(io_error("creating migration lock", path, error)),
        }
    }
}

impl Drop for MigrationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn create_lock_file(path: &Path) -> Result<(), std::io::Error> {
    let record = LockRecord {
        pid: process::id(),
        created_at_utc: now_utc(),
    };
    let bytes = serde_json::to_vec_pretty(&record)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()
}

fn lock_is_stale(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    let Ok(record) = serde_json::from_slice::<LockRecord>(&bytes) else {
        return false;
    };
    if record.pid == process::id() {
        return false;
    }
    process_is_absent(record.pid)
}

#[cfg(target_os = "linux")]
fn process_is_absent(pid: u32) -> bool {
    !Path::new("/proc").join(pid.to_string()).exists()
}

#[cfg(not(target_os = "linux"))]
fn process_is_absent(_pid: u32) -> bool {
    false
}

fn write_new_json_atomic<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), ControlledMigrationError> {
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| ControlledMigrationError::Json {
            operation: "serializing migration artifact",
            message: error.to_string(),
        })?;
    ensure_parent(path)?;
    let temp_path = path.with_file_name(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("artifact"),
        process::id()
    ));
    if temp_path.exists() {
        fs::remove_file(&temp_path)
            .map_err(|error| io_error("removing stale artifact", &temp_path, error))?;
    }
    write_new_file(&temp_path, &bytes)?;
    if path.exists() {
        let _ = fs::remove_file(&temp_path);
        return Err(ControlledMigrationError::AuthorityMarkerConflict {
            path: display_path(path),
        });
    }
    fs::rename(&temp_path, path)
        .map_err(|error| io_error("publishing migration artifact", path, error))?;
    sync_parent(path)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), ControlledMigrationError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| io_error("creating immutable file", path, error))?;
    file.write_all(bytes)
        .map_err(|error| io_error("writing immutable file", path, error))?;
    file.sync_all()
        .map_err(|error| io_error("syncing immutable file", path, error))
}

fn sync_file(path: &Path) -> Result<(), ControlledMigrationError> {
    let file = File::open(path).map_err(|error| io_error("opening file for sync", path, error))?;
    file.sync_all()
        .map_err(|error| io_error("syncing file", path, error))
}

fn sync_parent(path: &Path) -> Result<(), ControlledMigrationError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let directory = File::open(parent)
        .map_err(|error| io_error("opening parent directory for sync", parent, error))?;
    directory
        .sync_all()
        .map_err(|error| io_error("syncing parent directory", parent, error))
}

fn ensure_parent(path: &Path) -> Result<(), ControlledMigrationError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| io_error("creating parent directory", parent, error))?;
    }
    Ok(())
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, ControlledMigrationError> {
    if path.is_absolute() {
        return Ok(path);
    }
    let current = std::env::current_dir().map_err(|error| ControlledMigrationError::Io {
        operation: "resolving current directory",
        path: ".".to_string(),
        message: error.to_string(),
    })?;
    Ok(current.join(path))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn io_error(
    operation: &'static str,
    path: &Path,
    error: std::io::Error,
) -> ControlledMigrationError {
    ControlledMigrationError::Io {
        operation,
        path: display_path(path),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct Fixture {
        root: PathBuf,
        data: PathBuf,
        state: PathBuf,
        database: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let root = PathBuf::from(format!("/tmp/strata_sqlite003_{name}_{nonce}"));
            let data = root.join("data");
            let state = root.join("state");
            fs::create_dir_all(&data).unwrap();
            fs::create_dir_all(&state).unwrap();
            fs::write(
                data.join("categories.csv"),
                "id,name,description,color_index,karma_effect\n1,Study,Focused work,0,1\n",
            )
            .unwrap();
            fs::write(
                data.join("time_log.csv"),
                "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n7,2026-08-01,1,Study,Read chapter,10:00:00,11:00:00,3600\n",
            )
            .unwrap();
            let database = data.join("strata.sqlite3");
            Self {
                root,
                data,
                state,
                database,
            }
        }

        fn layout(&self) -> MigrationLayout {
            MigrationLayout {
                data_dir: self.data.clone(),
                state_dir: self.state.clone(),
                sessions_csv: self.data.join("time_log.csv"),
                database_path: self.database.clone(),
                report_path_override: None,
            }
        }

        fn source_bytes(&self) -> BTreeMap<String, Vec<u8>> {
            BTreeMap::from([
                (
                    "categories.csv".to_string(),
                    fs::read(self.data.join("categories.csv")).unwrap(),
                ),
                (
                    "time_log.csv".to_string(),
                    fs::read(self.data.join("time_log.csv")).unwrap(),
                ),
            ])
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn options(dry_run: bool) -> ControlledMigrationOptions {
        ControlledMigrationOptions {
            dry_run,
            include_active_recovery: false,
            database_path: None,
            report_path: None,
            utc_offset_seconds: -6 * 60 * 60,
            operational_day_start_minutes: 6 * 60,
            quantum_seconds: 1,
        }
    }

    #[test]
    fn dry_run_validates_without_writing() {
        let fixture = Fixture::new("dry_run");
        let before = fixture.source_bytes();

        let report = execute_controlled_migration(&fixture.layout(), &options(true)).unwrap();

        assert_eq!(report.status, ControlledMigrationStatus::DryRunVerified);
        assert!(!fixture.database.exists());
        assert!(!fixture.state.join(MIGRATION_DIRECTORY_NAME).exists());
        assert!(!fixture.state.join(AUTHORITY_MARKER_FILENAME).exists());
        assert_eq!(fixture.source_bytes(), before);
    }

    #[test]
    fn publication_preserves_sources_and_creates_verified_artifacts() {
        let fixture = Fixture::new("publish");
        let before = fixture.source_bytes();

        let report = execute_controlled_migration(&fixture.layout(), &options(false)).unwrap();

        assert_eq!(report.status, ControlledMigrationStatus::Published);
        assert!(fixture.database.exists());
        assert_eq!(fixture.source_bytes(), before);
        let backup = PathBuf::from(report.backup_path.clone().unwrap());
        assert_eq!(
            fs::read(backup.join("data/categories.csv")).unwrap(),
            before["categories.csv"]
        );
        assert_eq!(
            fs::read(backup.join("data/time_log.csv")).unwrap(),
            before["time_log.csv"]
        );
        let marker: StorageAuthorityMarker = serde_json::from_slice(
            &fs::read(fixture.state.join(AUTHORITY_MARKER_FILENAME)).unwrap(),
        )
        .unwrap();
        assert_eq!(marker.active_authority, "legacy-files");
        assert_eq!(marker.sqlite_candidate.status, "verified");
        assert_eq!(report.integrity_check.as_deref(), Some("ok"));
    }

    #[test]
    fn active_recovery_requires_explicit_opt_in() {
        let fixture = Fixture::new("active");
        fs::write(
            fixture.state.join("active_session.json"),
            r#"{
                "project": "Study",
                "description": "Continue reading",
                "category_id": 1,
                "category_name": "Study",
                "start_time": "2026-08-01T10:00:00Z"
            }"#,
        )
        .unwrap();

        let error = execute_controlled_migration(&fixture.layout(), &options(false))
            .expect_err("active recovery should require opt in");
        assert!(matches!(
            error,
            ControlledMigrationError::ActiveRecoveryRequiresOptIn
        ));
        assert!(!fixture.database.exists());

        let mut allowed = options(false);
        allowed.include_active_recovery = true;
        let report = execute_controlled_migration(&fixture.layout(), &allowed).unwrap();
        assert_eq!(report.status, ControlledMigrationStatus::Published);
        assert!(report.source_summary.active_session_present);
    }

    #[test]
    fn detached_checkpoint_requires_explicit_opt_in() {
        let fixture = Fixture::new("detached_checkpoint");
        let sand = r#"{
            "version": 1,
            "grid_width": 1,
            "grid_height": 1,
            "grains": [],
            "frame_count": 0,
            "sweep_left_to_right": true,
            "rng_state": 42
        }"#;
        fs::write(
            fixture.state.join("detached_runtime.json"),
            format!(
                r#"{{
                    "schema_version": 1,
                    "detached_at_utc": "2026-08-01T16:00:00Z",
                    "simulation_time_utc": "2026-08-01T16:00:00Z",
                    "spawn_accumulator_nanos": 0,
                    "physics_accumulator_nanos": 0,
                    "active_category_id": 0,
                    "active_description": "",
                    "active_session_started_at_utc": null,
                    "sand_state": {sand},
                    "pending_mutations": []
                }}"#
            ),
        )
        .unwrap();

        let error = execute_controlled_migration(&fixture.layout(), &options(false))
            .expect_err("detached checkpoint should require opt in");
        assert!(matches!(
            error,
            ControlledMigrationError::ActiveRecoveryRequiresOptIn
        ));
        assert!(!fixture.database.exists());

        let mut allowed = options(false);
        allowed.include_active_recovery = true;
        let report = execute_controlled_migration(&fixture.layout(), &allowed).unwrap();
        assert_eq!(report.status, ControlledMigrationStatus::Published);
        assert!(report.source_summary.checkpoint_present);
        assert!(!report.source_summary.active_session_present);
    }

    #[test]
    fn repeated_run_is_idempotent() {
        let fixture = Fixture::new("repeat");
        let first = execute_controlled_migration(&fixture.layout(), &options(false)).unwrap();
        let second = execute_controlled_migration(&fixture.layout(), &options(false)).unwrap();

        assert_eq!(first.status, ControlledMigrationStatus::Published);
        assert_eq!(second.status, ControlledMigrationStatus::AlreadyPublished);
        assert_eq!(first.source_fingerprint, second.source_fingerprint);
    }

    #[test]
    fn rerun_recovers_missing_publication_artifacts() {
        let fixture = Fixture::new("recover");
        let first = execute_controlled_migration(&fixture.layout(), &options(false)).unwrap();
        fs::remove_file(fixture.state.join(AUTHORITY_MARKER_FILENAME)).unwrap();
        fs::remove_file(PathBuf::from(first.machine_report_path.unwrap())).unwrap();

        let recovered = execute_controlled_migration(&fixture.layout(), &options(false)).unwrap();

        assert_eq!(
            recovered.status,
            ControlledMigrationStatus::RecoveredPublication
        );
        assert!(fixture.state.join(AUTHORITY_MARKER_FILENAME).exists());
        assert!(PathBuf::from(recovered.machine_report_path.unwrap()).exists());
    }

    #[test]
    fn unrelated_existing_database_is_never_overwritten() {
        let fixture = Fixture::new("existing");
        fs::write(&fixture.database, b"not a strata database").unwrap();

        let error = execute_controlled_migration(&fixture.layout(), &options(false))
            .expect_err("unrelated target must be rejected");

        assert!(matches!(
            error,
            ControlledMigrationError::ExistingDatabase { .. }
        ));
        assert_eq!(
            fs::read(&fixture.database).unwrap(),
            b"not a strata database"
        );
    }
}
