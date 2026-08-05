use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    process,
};

use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const REPORT_SCHEMA_VERSION: u8 = 1;
const MARKER_SCHEMA_VERSION: u8 = 1;
const PROVENANCE_SCHEMA_VERSION: u8 = 1;
const ARCHIVE_SCHEMA_VERSION: u8 = 1;
const REMOVAL_SCHEMA_VERSION: u8 = 1;
const ARCHIVE_MANIFEST_FILENAME: &str = "legacy_evidence_manifest.json";
const ARCHIVE_INTENT_FILENAME: &str = ".strata_legacy_archive_intent.json";
const SOURCE_PATHS_FILENAME: &str = "source_paths.json";

#[derive(Debug, Clone)]
pub(crate) struct LegacyEvidenceInventoryOptions {
    pub authority_marker_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacyEvidenceArchiveOptions {
    pub authority_marker_path: PathBuf,
    pub output_directory: PathBuf,
    pub confirm: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LegacyEvidenceRemoveOptions {
    pub authority_marker_path: PathBuf,
    pub archive_directory: PathBuf,
    pub confirm_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LegacyEvidenceFileReport {
    pub logical_name: String,
    pub original_path: String,
    pub archive_relative_path: String,
    pub byte_count: u64,
    pub fingerprint: String,
    pub live_status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct LegacyEvidenceReport {
    pub schema_version: u8,
    pub operation: String,
    pub status: String,
    pub source_fingerprint: String,
    pub authority_marker_path: String,
    pub database_path: String,
    pub migration_backup_path: String,
    pub archive_path: Option<String>,
    pub removal_ledger_path: Option<String>,
    pub healthy: bool,
    pub files: Vec<LegacyEvidenceFileReport>,
}

impl LegacyEvidenceReport {
    pub fn to_pretty_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn print_human(&self) {
        println!("Legacy evidence: {}", self.operation);
        println!("Status: {}", self.status);
        println!("Source fingerprint: {}", self.source_fingerprint);
        println!("Authority marker: {}", self.authority_marker_path);
        println!("SQLite authority: {}", self.database_path);
        println!("Migration backup: {}", self.migration_backup_path);
        if let Some(path) = &self.archive_path {
            println!("Archive: {path}");
        }
        if let Some(path) = &self.removal_ledger_path {
            println!("Removal ledger: {path}");
        }
        println!(
            "Evidence healthy: {}",
            if self.healthy { "yes" } else { "no" }
        );
        for file in &self.files {
            println!(
                "[{}] {} -> {} ({} bytes)",
                file.live_status, file.logical_name, file.original_path, file.byte_count
            );
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy
    }
}

#[derive(Debug, Error)]
enum LegacyEvidenceError {
    #[error("legacy evidence archive requires --confirm")]
    ArchiveConfirmationRequired,
    #[error("legacy evidence removal requires --confirm-fingerprint {expected}")]
    RemovalConfirmationRequired { expected: String },
    #[error("invalid storage authority marker: {0}")]
    InvalidMarker(String),
    #[error("SQLite must be the active authority before legacy evidence disposition")]
    SqliteNotActive,
    #[error("invalid migration backup provenance: {0}")]
    InvalidProvenance(String),
    #[error("SQLite authority verification failed: {0}")]
    AuthorityVerification(String),
    #[error("legacy evidence differs from the verified migration backup: {0}")]
    EvidenceMismatch(String),
    #[error("legacy evidence archive is invalid: {0}")]
    InvalidArchive(String),
    #[error("legacy evidence target already exists but does not match: {0}")]
    TargetConflict(String),
    #[error("legacy evidence staging directory is invalid: {0}")]
    StagingConflict(String),
    #[error("legacy removal ledger conflicts with this request: {0}")]
    RemovalLedgerConflict(String),
    #[error("I/O error while {operation} {path}: {message}")]
    Io {
        operation: &'static str,
        path: String,
        message: String,
    },
    #[error("JSON error in {path}: {message}")]
    Json { path: String, message: String },
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StorageAuthorityMarker {
    schema_version: u8,
    active_authority: String,
    sqlite_candidate: SqliteCandidateMarker,
    sqlite_cli_activation: Option<SqliteCliActivationMarker>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SqliteCliActivationMarker {
    status: String,
    previous_authority: String,
    source_fingerprint: String,
    database_path: String,
    started_at_utc: String,
    completed_at_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BackupProvenance {
    schema_version: u8,
    source_fingerprint: String,
    copied_at_utc: String,
    files: Vec<BackupProvenanceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BackupProvenanceEntry {
    logical_name: String,
    original_path: String,
    byte_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ArchiveIntent {
    schema_version: u8,
    source_fingerprint: String,
    output_path: String,
}

#[derive(Debug, Clone, Deserialize)]
struct StoredSourceManifest {
    entries: Vec<StoredSourceManifestEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct StoredSourceManifestEntry {
    logical_name: String,
    path: String,
    exists: bool,
    byte_count: usize,
    content_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ArchiveManifest {
    schema_version: u8,
    source_fingerprint: String,
    authority_marker_path: String,
    database_path: String,
    migration_backup_path: String,
    archived_at_utc: String,
    files: Vec<ArchiveManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ArchiveManifestEntry {
    logical_name: String,
    original_path: String,
    archive_relative_path: String,
    byte_count: u64,
    fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RemovalLedger {
    schema_version: u8,
    source_fingerprint: String,
    archive_path: String,
    status: String,
    started_at_utc: String,
    completed_at_utc: Option<String>,
    files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveStatus {
    Matches,
    Missing,
    Changed,
}

impl LiveStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Matches => "matches",
            Self::Missing => "missing",
            Self::Changed => "changed",
        }
    }
}

#[derive(Debug, Clone)]
struct VerifiedEvidenceFile {
    logical_name: String,
    original_path: PathBuf,
    backup_path: PathBuf,
    archive_relative_path: PathBuf,
    byte_count: u64,
    fingerprint: String,
    live_status: LiveStatus,
}

#[derive(Debug, Clone)]
struct EvidenceContext {
    marker_path: PathBuf,
    database_path: PathBuf,
    backup_path: PathBuf,
    source_fingerprint: String,
    files: Vec<VerifiedEvidenceFile>,
}

pub(super) fn inventory(
    options: LegacyEvidenceInventoryOptions,
) -> Result<LegacyEvidenceReport, String> {
    inventory_inner(options).map_err(|error| error.to_string())
}

fn inventory_inner(
    options: LegacyEvidenceInventoryOptions,
) -> Result<LegacyEvidenceReport, LegacyEvidenceError> {
    let context = load_context(&options.authority_marker_path)?;
    Ok(build_report(
        &context,
        "sqlite-legacy-inventory",
        "inventoried",
        None,
        None,
    ))
}

pub(super) fn archive(
    options: LegacyEvidenceArchiveOptions,
) -> Result<LegacyEvidenceReport, String> {
    archive_inner(options).map_err(|error| error.to_string())
}

fn archive_inner(
    options: LegacyEvidenceArchiveOptions,
) -> Result<LegacyEvidenceReport, LegacyEvidenceError> {
    if !options.confirm {
        return Err(LegacyEvidenceError::ArchiveConfirmationRequired);
    }
    let context = load_context(&options.authority_marker_path)?;
    require_all_live_matches(&context)?;
    let output = absolute_output_path(&options.output_directory)?;
    if output.starts_with(&context.backup_path) || context.backup_path.starts_with(&output) {
        return Err(LegacyEvidenceError::TargetConflict(display_path(&output)));
    }

    if output.exists() {
        validate_archive(&context, &output)?;
        return Ok(build_report(
            &context,
            "sqlite-legacy-archive",
            "already-archived",
            Some(&output),
            None,
        ));
    }

    let staging = archive_staging_path(&output, &context.source_fingerprint)?;
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
    let result: Result<(), LegacyEvidenceError> = (|| {
        let mut manifest_files = Vec::with_capacity(context.files.len());
        for file in &context.files {
            let destination = staging.join(&file.archive_relative_path);
            ensure_parent(&destination)?;
            let bytes = read_regular_file(&file.backup_path, "reading migration backup evidence")?;
            write_new_file(&destination, &bytes)?;
            manifest_files.push(ArchiveManifestEntry {
                logical_name: file.logical_name.clone(),
                original_path: display_path(&file.original_path),
                archive_relative_path: path_text(&file.archive_relative_path)?,
                byte_count: file.byte_count,
                fingerprint: file.fingerprint.clone(),
            });
        }
        let provenance_source = context.backup_path.join(SOURCE_PATHS_FILENAME);
        let provenance_destination = staging.join(SOURCE_PATHS_FILENAME);
        write_new_file(
            &provenance_destination,
            &read_regular_file(&provenance_source, "reading migration provenance")?,
        )?;
        let manifest = ArchiveManifest {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            source_fingerprint: context.source_fingerprint.clone(),
            authority_marker_path: display_path(&context.marker_path),
            database_path: display_path(&context.database_path),
            migration_backup_path: display_path(&context.backup_path),
            archived_at_utc: now_utc(),
            files: manifest_files,
        };
        write_new_json(&staging.join(ARCHIVE_MANIFEST_FILENAME), &manifest)?;
        sync_directory(&staging)?;
        validate_archive(&context, &staging)?;
        ensure_parent(&output)?;
        fs::rename(&staging, &output)
            .map_err(|error| io_error("publishing legacy evidence archive", &output, error))?;
        sync_parent(&output)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result?;

    Ok(build_report(
        &context,
        "sqlite-legacy-archive",
        if recovered_partial {
            "recovered-archive"
        } else {
            "archived"
        },
        Some(&output),
        None,
    ))
}

pub(super) fn remove(options: LegacyEvidenceRemoveOptions) -> Result<LegacyEvidenceReport, String> {
    remove_with_hook(options, |_| Ok(())).map_err(|error| error.to_string())
}

fn remove_with_hook(
    options: LegacyEvidenceRemoveOptions,
    mut after_delete: impl FnMut(usize) -> Result<(), LegacyEvidenceError>,
) -> Result<LegacyEvidenceReport, LegacyEvidenceError> {
    let context = load_context(&options.authority_marker_path)?;
    if options.confirm_fingerprint != context.source_fingerprint {
        return Err(LegacyEvidenceError::RemovalConfirmationRequired {
            expected: context.source_fingerprint,
        });
    }
    let archive_path = absolute_existing_path(&options.archive_directory)?;
    validate_archive(&context, &archive_path)?;

    let ledger_path = removal_ledger_path(&context)?;
    let existing_ledger = if ledger_path.exists() {
        Some(read_json::<RemovalLedger>(&ledger_path)?)
    } else {
        None
    };
    if let Some(ledger) = &existing_ledger {
        if ledger.schema_version != REMOVAL_SCHEMA_VERSION
            || ledger.source_fingerprint != context.source_fingerprint
            || Path::new(&ledger.archive_path) != archive_path
        {
            return Err(LegacyEvidenceError::RemovalLedgerConflict(display_path(
                &ledger_path,
            )));
        }
        if ledger.status == "removed"
            && context
                .files
                .iter()
                .all(|file| !file.original_path.exists())
        {
            return Ok(build_report(
                &context,
                "sqlite-legacy-remove",
                "already-removed",
                Some(&archive_path),
                Some(&ledger_path),
            ));
        }
    } else {
        require_all_live_matches(&context)?;
        let ledger = RemovalLedger {
            schema_version: REMOVAL_SCHEMA_VERSION,
            source_fingerprint: context.source_fingerprint.clone(),
            archive_path: display_path(&archive_path),
            status: "removing".to_string(),
            started_at_utc: now_utc(),
            completed_at_utc: None,
            files: context
                .files
                .iter()
                .map(|file| display_path(&file.original_path))
                .collect(),
        };
        write_json_atomic(&ledger_path, &ledger)?;
    }

    let mut deleted = 0_usize;
    for file in &context.files {
        match fs::symlink_metadata(&file.original_path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(LegacyEvidenceError::EvidenceMismatch(format!(
                        "{} is not a regular file",
                        display_path(&file.original_path)
                    )));
                }
                let live = fs::read(&file.original_path).map_err(|error| {
                    io_error(
                        "reading legacy evidence before removal",
                        &file.original_path,
                        error,
                    )
                })?;
                let backup = read_regular_file(&file.backup_path, "reading migration backup")?;
                if live != backup {
                    return Err(LegacyEvidenceError::EvidenceMismatch(format!(
                        "{} changed after migration",
                        display_path(&file.original_path)
                    )));
                }
                fs::remove_file(&file.original_path).map_err(|error| {
                    io_error("removing legacy evidence", &file.original_path, error)
                })?;
                sync_parent(&file.original_path)?;
                deleted += 1;
                after_delete(deleted)?;
            }
            Err(error) if error.kind() == ErrorKind::NotFound && existing_ledger.is_some() => {}
            Err(error) => {
                return Err(io_error(
                    "inspecting legacy evidence before removal",
                    &file.original_path,
                    error,
                ));
            }
        }
    }

    let mut ledger = read_json::<RemovalLedger>(&ledger_path)?;
    ledger.status = "removed".to_string();
    ledger.completed_at_utc = Some(now_utc());
    write_json_atomic(&ledger_path, &ledger)?;

    let refreshed = load_context(&options.authority_marker_path)?;
    Ok(build_report(
        &refreshed,
        "sqlite-legacy-remove",
        if existing_ledger.is_some() {
            "recovered-removal"
        } else {
            "removed"
        },
        Some(&archive_path),
        Some(&ledger_path),
    ))
}

#[cfg(test)]
pub(super) fn create_test_partial_archive(
    options: LegacyEvidenceArchiveOptions,
) -> Result<(), String> {
    let context =
        load_context(&options.authority_marker_path).map_err(|error| error.to_string())?;
    require_all_live_matches(&context).map_err(|error| error.to_string())?;
    let output =
        absolute_output_path(&options.output_directory).map_err(|error| error.to_string())?;
    let staging = archive_staging_path(&output, &context.source_fingerprint)
        .map_err(|error| error.to_string())?;
    ensure_parent(&staging).map_err(|error| error.to_string())?;
    fs::create_dir(&staging).map_err(|error| error.to_string())?;
    write_new_json(
        &staging.join(ARCHIVE_INTENT_FILENAME),
        &ArchiveIntent {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            source_fingerprint: context.source_fingerprint.clone(),
            output_path: display_path(&output),
        },
    )
    .map_err(|error| error.to_string())?;
    let first = context
        .files
        .first()
        .ok_or_else(|| "no evidence files".to_string())?;
    let destination = staging.join(&first.archive_relative_path);
    write_new_file(
        &destination,
        &read_regular_file(&first.backup_path, "reading migration backup evidence")
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    sync_directory(&staging).map_err(|error| error.to_string())
}

#[cfg(test)]
pub(super) fn remove_with_test_failure(
    options: LegacyEvidenceRemoveOptions,
    fail_after: usize,
) -> Result<LegacyEvidenceReport, String> {
    remove_with_hook(options, |deleted| {
        if deleted == fail_after {
            Err(LegacyEvidenceError::Io {
                operation: "injecting legacy-removal interruption",
                path: "test".to_string(),
                message: "injected interruption".to_string(),
            })
        } else {
            Ok(())
        }
    })
    .map_err(|error| error.to_string())
}

fn load_context(marker_path: &Path) -> Result<EvidenceContext, LegacyEvidenceError> {
    let marker_path = absolute_existing_path(marker_path)?;
    let marker: StorageAuthorityMarker = read_json(&marker_path)?;
    validate_marker(&marker)?;
    let database_path = absolute_existing_path(Path::new(&marker.sqlite_candidate.database_path))?;
    let backup_path = absolute_existing_path(Path::new(&marker.sqlite_candidate.backup_path))?;
    let stored_manifest =
        verify_database(&database_path, &marker.sqlite_candidate.source_fingerprint)?;

    let provenance_path = backup_path.join(SOURCE_PATHS_FILENAME);
    let provenance: BackupProvenance = read_json(&provenance_path)?;
    if provenance.schema_version != PROVENANCE_SCHEMA_VERSION
        || provenance.source_fingerprint != marker.sqlite_candidate.source_fingerprint
    {
        return Err(LegacyEvidenceError::InvalidProvenance(display_path(
            &provenance_path,
        )));
    }

    let expected_existing = stored_manifest
        .values()
        .filter(|entry| entry.exists)
        .count();
    if provenance.files.len() != expected_existing {
        return Err(LegacyEvidenceError::InvalidProvenance(format!(
            "source_paths.json contains {} files but the verified import manifest contains {expected_existing}",
            provenance.files.len()
        )));
    }

    let mut names = BTreeSet::new();
    let mut files = Vec::with_capacity(provenance.files.len());
    for entry in provenance.files {
        if !names.insert(entry.logical_name.clone()) {
            return Err(LegacyEvidenceError::InvalidProvenance(format!(
                "duplicate logical name {}",
                entry.logical_name
            )));
        }
        let stored = stored_manifest.get(&entry.logical_name).ok_or_else(|| {
            LegacyEvidenceError::InvalidProvenance(format!(
                "{} is absent from the verified SQLite source manifest",
                entry.logical_name
            ))
        })?;
        let stored_byte_count = u64::try_from(stored.byte_count).map_err(|_| {
            LegacyEvidenceError::InvalidProvenance(format!(
                "{} exceeds supported size",
                entry.logical_name
            ))
        })?;
        if !stored.exists
            || stored.path != entry.original_path
            || stored_byte_count != entry.byte_count
        {
            return Err(LegacyEvidenceError::InvalidProvenance(format!(
                "{} differs from the verified SQLite source manifest",
                entry.logical_name
            )));
        }
        let original_path = PathBuf::from(&entry.original_path);
        if !original_path.is_absolute() {
            return Err(LegacyEvidenceError::InvalidProvenance(format!(
                "{} is not an absolute original path",
                entry.original_path
            )));
        }
        let archive_relative_path = archive_relative_path(&entry.logical_name)?;
        let backup_file = backup_path.join(&archive_relative_path);
        let backup_bytes = read_regular_file(&backup_file, "reading migration backup evidence")?;
        let byte_count = u64::try_from(backup_bytes.len()).map_err(|_| {
            LegacyEvidenceError::InvalidProvenance(format!(
                "{} exceeds supported size",
                entry.logical_name
            ))
        })?;
        if byte_count != entry.byte_count
            || stored.content_fingerprint.as_deref()
                != Some(fingerprint_bytes(&backup_bytes).as_str())
        {
            return Err(LegacyEvidenceError::InvalidProvenance(format!(
                "{} content differs from the verified SQLite source manifest",
                entry.logical_name
            )));
        }
        let live_status = compare_live(&original_path, &backup_bytes)?;
        files.push(VerifiedEvidenceFile {
            logical_name: entry.logical_name,
            original_path,
            backup_path: backup_file,
            archive_relative_path,
            byte_count,
            fingerprint: fingerprint_bytes(&backup_bytes),
            live_status,
        });
    }
    files.sort_by(|left, right| left.logical_name.cmp(&right.logical_name));

    Ok(EvidenceContext {
        marker_path,
        database_path,
        backup_path,
        source_fingerprint: marker.sqlite_candidate.source_fingerprint,
        files,
    })
}

fn validate_marker(marker: &StorageAuthorityMarker) -> Result<(), LegacyEvidenceError> {
    if marker.schema_version != MARKER_SCHEMA_VERSION
        || marker.active_authority != "sqlite"
        || marker.sqlite_candidate.status != "verified"
    {
        return Err(LegacyEvidenceError::SqliteNotActive);
    }
    let activation = marker.sqlite_cli_activation.as_ref().ok_or_else(|| {
        LegacyEvidenceError::InvalidMarker("missing SQLite activation provenance".to_string())
    })?;
    if activation.status != "active"
        || activation.source_fingerprint != marker.sqlite_candidate.source_fingerprint
        || activation.database_path != marker.sqlite_candidate.database_path
    {
        return Err(LegacyEvidenceError::InvalidMarker(
            "activation provenance does not match the verified candidate".to_string(),
        ));
    }
    Ok(())
}

fn verify_database(
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
    if metadata("storage_authority")?.as_deref() != Some("sqlite")
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

fn validate_archive_intent(
    context: &EvidenceContext,
    staging: &Path,
    output: &Path,
) -> Result<(), LegacyEvidenceError> {
    let intent: ArchiveIntent = read_json(&staging.join(ARCHIVE_INTENT_FILENAME))?;
    if intent.schema_version != ARCHIVE_SCHEMA_VERSION
        || intent.source_fingerprint != context.source_fingerprint
        || intent.output_path != display_path(output)
    {
        return Err(LegacyEvidenceError::StagingConflict(display_path(staging)));
    }
    Ok(())
}

fn validate_archive(context: &EvidenceContext, archive: &Path) -> Result<(), LegacyEvidenceError> {
    if !archive.is_dir() {
        return Err(LegacyEvidenceError::InvalidArchive(display_path(archive)));
    }
    let manifest_path = archive.join(ARCHIVE_MANIFEST_FILENAME);
    let manifest: ArchiveManifest = read_json(&manifest_path)?;
    if manifest.schema_version != ARCHIVE_SCHEMA_VERSION
        || manifest.source_fingerprint != context.source_fingerprint
        || manifest.database_path != display_path(&context.database_path)
        || manifest.files.len() != context.files.len()
    {
        return Err(LegacyEvidenceError::InvalidArchive(display_path(
            &manifest_path,
        )));
    }
    for (manifest_file, expected) in manifest.files.iter().zip(&context.files) {
        if manifest_file.logical_name != expected.logical_name
            || manifest_file.original_path != display_path(&expected.original_path)
            || manifest_file.archive_relative_path != path_text(&expected.archive_relative_path)?
            || manifest_file.byte_count != expected.byte_count
            || manifest_file.fingerprint != expected.fingerprint
        {
            return Err(LegacyEvidenceError::InvalidArchive(format!(
                "manifest entry {} does not match migration provenance",
                expected.logical_name
            )));
        }
        let archived = read_regular_file(
            &archive.join(&expected.archive_relative_path),
            "reading archived legacy evidence",
        )?;
        let backup = read_regular_file(&expected.backup_path, "reading migration backup")?;
        if archived != backup {
            return Err(LegacyEvidenceError::InvalidArchive(format!(
                "archived file {} differs from migration backup",
                expected.logical_name
            )));
        }
    }
    let archived_provenance = read_regular_file(
        &archive.join(SOURCE_PATHS_FILENAME),
        "reading archived migration provenance",
    )?;
    let migration_provenance = read_regular_file(
        &context.backup_path.join(SOURCE_PATHS_FILENAME),
        "reading migration provenance",
    )?;
    if archived_provenance != migration_provenance {
        return Err(LegacyEvidenceError::InvalidArchive(
            "archived source_paths.json differs from migration backup".to_string(),
        ));
    }
    Ok(())
}

fn require_all_live_matches(context: &EvidenceContext) -> Result<(), LegacyEvidenceError> {
    if let Some(file) = context
        .files
        .iter()
        .find(|file| file.live_status != LiveStatus::Matches)
    {
        return Err(LegacyEvidenceError::EvidenceMismatch(format!(
            "{} is {}",
            display_path(&file.original_path),
            file.live_status.as_str()
        )));
    }
    Ok(())
}

fn compare_live(path: &Path, backup: &[u8]) -> Result<LiveStatus, LegacyEvidenceError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Ok(LiveStatus::Changed)
        }
        Ok(_) => {
            let bytes = fs::read(path)
                .map_err(|error| io_error("reading live legacy evidence", path, error))?;
            Ok(if bytes == backup {
                LiveStatus::Matches
            } else {
                LiveStatus::Changed
            })
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(LiveStatus::Missing),
        Err(error) => Err(io_error("inspecting live legacy evidence", path, error)),
    }
}

fn build_report(
    context: &EvidenceContext,
    operation: &str,
    status: &str,
    archive_path: Option<&Path>,
    ledger_path: Option<&Path>,
) -> LegacyEvidenceReport {
    let files = context
        .files
        .iter()
        .map(|file| LegacyEvidenceFileReport {
            logical_name: file.logical_name.clone(),
            original_path: display_path(&file.original_path),
            archive_relative_path: file.archive_relative_path.to_string_lossy().into_owned(),
            byte_count: file.byte_count,
            fingerprint: file.fingerprint.clone(),
            live_status: file.live_status.as_str().to_string(),
        })
        .collect::<Vec<_>>();
    LegacyEvidenceReport {
        schema_version: REPORT_SCHEMA_VERSION,
        operation: operation.to_string(),
        status: status.to_string(),
        source_fingerprint: context.source_fingerprint.clone(),
        authority_marker_path: display_path(&context.marker_path),
        database_path: display_path(&context.database_path),
        migration_backup_path: display_path(&context.backup_path),
        archive_path: archive_path.map(display_path),
        removal_ledger_path: ledger_path.map(display_path),
        healthy: files.iter().all(|file| file.live_status == "matches") || status.contains("remov"),
        files,
    }
}

fn archive_relative_path(logical_name: &str) -> Result<PathBuf, LegacyEvidenceError> {
    if logical_name.is_empty()
        || logical_name.starts_with('/')
        || logical_name
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(LegacyEvidenceError::InvalidProvenance(format!(
            "invalid logical name {logical_name}"
        )));
    }
    Ok(match logical_name {
        "categories.csv" | "time_log.csv" => PathBuf::from("data").join(logical_name),
        _ => PathBuf::from("state").join(logical_name),
    })
}

fn archive_staging_path(output: &Path, fingerprint: &str) -> Result<PathBuf, LegacyEvidenceError> {
    let parent = output
        .parent()
        .ok_or_else(|| LegacyEvidenceError::TargetConflict(display_path(output)))?;
    let name = output
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| LegacyEvidenceError::TargetConflict(display_path(output)))?;
    Ok(parent.join(format!(".{name}.partial-{fingerprint}")))
}

fn removal_ledger_path(context: &EvidenceContext) -> Result<PathBuf, LegacyEvidenceError> {
    let state_dir = context.marker_path.parent().ok_or_else(|| {
        LegacyEvidenceError::RemovalLedgerConflict(display_path(&context.marker_path))
    })?;
    Ok(state_dir
        .join("storage_migration/removals")
        .join(format!("{}.json", context.source_fingerprint)))
}

fn read_regular_file(path: &Path, operation: &'static str) -> Result<Vec<u8>, LegacyEvidenceError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(operation, path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LegacyEvidenceError::InvalidProvenance(format!(
            "{} is not a regular file",
            display_path(path)
        )));
    }
    fs::read(path).map_err(|error| io_error(operation, path, error))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, LegacyEvidenceError> {
    let bytes = fs::read(path).map_err(|error| io_error("reading", path, error))?;
    serde_json::from_slice(&bytes).map_err(|error| LegacyEvidenceError::Json {
        path: display_path(path),
        message: error.to_string(),
    })
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), LegacyEvidenceError> {
    ensure_parent(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| io_error("creating", path, error))?;
    file.write_all(bytes)
        .map_err(|error| io_error("writing", path, error))?;
    file.sync_all()
        .map_err(|error| io_error("syncing", path, error))
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), LegacyEvidenceError> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|error| LegacyEvidenceError::Json {
            path: display_path(path),
            message: error.to_string(),
        })?;
    bytes.push(b'\n');
    write_new_file(path, &bytes)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), LegacyEvidenceError> {
    ensure_parent(path)?;
    let temporary = path.with_extension(format!("tmp-{}", process::id()));
    if temporary.exists() {
        fs::remove_file(&temporary)
            .map_err(|error| io_error("removing stale temporary ledger", &temporary, error))?;
    }
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|error| LegacyEvidenceError::Json {
            path: display_path(path),
            message: error.to_string(),
        })?;
    bytes.push(b'\n');
    write_new_file(&temporary, &bytes)?;
    fs::rename(&temporary, path).map_err(|error| io_error("publishing", path, error))?;
    sync_parent(path)
}

fn ensure_parent(path: &Path) -> Result<(), LegacyEvidenceError> {
    let parent = path.parent().ok_or_else(|| LegacyEvidenceError::Io {
        operation: "resolving parent",
        path: display_path(path),
        message: "path has no parent".to_string(),
    })?;
    fs::create_dir_all(parent).map_err(|error| io_error("creating parent", parent, error))
}

fn sync_directory(path: &Path) -> Result<(), LegacyEvidenceError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| io_error("syncing directory", path, error))
}

fn sync_parent(path: &Path) -> Result<(), LegacyEvidenceError> {
    let parent = path.parent().ok_or_else(|| LegacyEvidenceError::Io {
        operation: "resolving parent",
        path: display_path(path),
        message: "path has no parent".to_string(),
    })?;
    sync_directory(parent)
}

fn absolute_existing_path(path: &Path) -> Result<PathBuf, LegacyEvidenceError> {
    fs::canonicalize(path).map_err(|error| io_error("resolving", path, error))
}

fn absolute_output_path(path: &Path) -> Result<PathBuf, LegacyEvidenceError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .map_err(|error| io_error("resolving current directory", path, error))
    }
}

fn path_text(path: &Path) -> Result<String, LegacyEvidenceError> {
    path.to_str().map(str::to_string).ok_or_else(|| {
        LegacyEvidenceError::InvalidProvenance(format!(
            "path is not UTF-8: {}",
            path.to_string_lossy()
        ))
    })
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn fingerprint_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn io_error(operation: &'static str, path: &Path, error: std::io::Error) -> LegacyEvidenceError {
    LegacyEvidenceError::Io {
        operation,
        path: display_path(path),
        message: error.to_string(),
    }
}
