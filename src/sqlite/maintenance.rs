use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Write},
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::DateTime;
use csv::{ReaderBuilder, StringRecord, Terminator, WriterBuilder};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    CURRENT_SCHEMA_VERSION, SqliteRepository,
    category_lifecycle::{CategoryIdentitySnapshot, CategoryReferenceCounts},
    repository::{
        ActiveSessionRecord, CategoryLifecycleReceiptRecord, CategoryRecord, CheckpointRecord,
        CheckpointStatus, RepositorySnapshot, SandSnapshotRecord, SandStateRecord, SessionRecord,
        SnapshotKind,
    },
};

const MAINTENANCE_REPORT_SCHEMA_VERSION: u8 = 1;
const PORTABLE_BUNDLE_SCHEMA_VERSION: u8 = 3;
const MANIFEST_FILENAME: &str = "manifest.json";
const CATEGORIES_FILENAME: &str = "categories.csv";
const CATEGORY_TAGS_FILENAME: &str = "category_tags.csv";
const SESSIONS_FILENAME: &str = "sessions.csv";
const ACTIVE_SESSION_FILENAME: &str = "active_session.csv";
const CHECKPOINT_FILENAME: &str = "runtime_checkpoint.csv";
const SAND_STATE_FILENAME: &str = "sand_state.csv";
const SAND_SNAPSHOTS_FILENAME: &str = "sand_snapshots.csv";
const CATEGORY_LIFECYCLE_RECEIPTS_FILENAME: &str = "category_lifecycle_receipts.csv";
const BUNDLE_FILES: [&str; 8] = [
    CATEGORIES_FILENAME,
    CATEGORY_TAGS_FILENAME,
    SESSIONS_FILENAME,
    ACTIVE_SESSION_FILENAME,
    CHECKPOINT_FILENAME,
    SAND_STATE_FILENAME,
    SAND_SNAPSHOTS_FILENAME,
    CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
];

#[derive(Debug, Clone)]
pub(crate) struct BundleExportOptions {
    pub database_path: PathBuf,
    pub output_directory: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct BundleImportOptions {
    pub bundle_directory: PathBuf,
    pub database_path: PathBuf,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct DoctorOptions {
    pub database_path: PathBuf,
    pub authority_marker_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackupOptions {
    pub database_path: PathBuf,
    pub backup_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct RestoreOptions {
    pub backup_path: PathBuf,
    pub database_path: PathBuf,
    pub replace: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct MaintenanceCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SnapshotCounts {
    pub categories: usize,
    pub category_tags: usize,
    pub sessions: usize,
    pub active_sessions: usize,
    pub checkpoints: usize,
    pub sand_states: usize,
    pub sand_snapshots: usize,
    pub category_lifecycle_receipts: usize,
    pub total_elapsed_seconds: i64,
}

impl SnapshotCounts {
    fn from_snapshot(snapshot: &RepositorySnapshot) -> Result<Self, MaintenanceError> {
        let total_elapsed_seconds =
            snapshot.sessions.iter().try_fold(0_i64, |total, session| {
                total.checked_add(session.elapsed_seconds).ok_or_else(|| {
                    MaintenanceError::InvalidData(
                        "session elapsed total exceeds supported range".to_string(),
                    )
                })
            })?;
        Ok(Self {
            categories: snapshot.categories.len(),
            category_tags: snapshot.category_tags.values().map(Vec::len).sum(),
            sessions: snapshot.sessions.len(),
            active_sessions: usize::from(snapshot.active_session.is_some()),
            checkpoints: usize::from(snapshot.checkpoint.is_some()),
            sand_states: usize::from(snapshot.sand_state.is_some()),
            sand_snapshots: snapshot.sand_snapshots.len(),
            category_lifecycle_receipts: snapshot.category_lifecycle_receipts.len(),
            total_elapsed_seconds,
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SqliteMaintenanceReport {
    pub schema_version: u8,
    pub operation: String,
    pub status: String,
    pub source_path: Option<String>,
    pub target_path: Option<String>,
    pub previous_database_path: Option<String>,
    pub bundle_fingerprint: Option<String>,
    pub database_schema_version: Option<i64>,
    pub counts: Option<SnapshotCounts>,
    pub healthy: Option<bool>,
    pub checks: Vec<MaintenanceCheck>,
}

impl SqliteMaintenanceReport {
    pub fn to_pretty_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn print_human(&self) {
        println!("SQLite maintenance: {}", self.operation);
        println!("Status: {}", self.status);
        if let Some(path) = &self.source_path {
            println!("Source: {path}");
        }
        if let Some(path) = &self.target_path {
            println!("Target: {path}");
        }
        if let Some(path) = &self.previous_database_path {
            println!("Previous database: {path}");
        }
        if let Some(fingerprint) = &self.bundle_fingerprint {
            println!("Bundle fingerprint: {fingerprint}");
        }
        if let Some(version) = self.database_schema_version {
            println!("Schema version: {version}");
        }
        if let Some(counts) = &self.counts {
            println!("Categories: {}", counts.categories);
            println!("Tags: {}", counts.category_tags);
            println!("Sessions: {}", counts.sessions);
            println!("Elapsed seconds: {}", counts.total_elapsed_seconds);
            println!("Snapshots: {}", counts.sand_snapshots);
        }
        if let Some(healthy) = self.healthy {
            println!("Healthy: {}", if healthy { "yes" } else { "no" });
        }
        for check in &self.checks {
            println!(
                "[{}] {}: {}",
                if check.passed { "PASS" } else { "FAIL" },
                check.name,
                check.detail
            );
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy.unwrap_or(true)
    }
}

#[derive(Debug, Error)]
pub(super) enum MaintenanceError {
    #[error("I/O error while {operation} {path}: {message}")]
    Io {
        operation: &'static str,
        path: String,
        message: String,
    },
    #[error("CSV error in {path}: {message}")]
    Csv { path: String, message: String },
    #[error("JSON error in {path}: {message}")]
    Json { path: String, message: String },
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("SQLite store error: {0}")]
    Store(#[from] super::SqliteStoreError),
    #[error("repository error: {0}")]
    Repository(#[from] super::repository::RepositoryError),
    #[error("invalid portable bundle: {0}")]
    InvalidBundle(String),
    #[error("invalid database state: {0}")]
    InvalidData(String),
    #[error("target already exists: {0}")]
    TargetExists(String),
    #[error("temporary maintenance artifact already exists: {0}")]
    TemporaryArtifactExists(String),
    #[error("another SQLite maintenance operation is active: {0}")]
    MaintenanceLocked(String),
    #[error("database sidecar indicates an active or uncheckpointed database: {0}")]
    DatabaseSidecar(String),
    #[error("database doctor reported an unhealthy database")]
    DoctorFailed,
    #[error("restored or imported database does not match its verified source")]
    SnapshotMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BundleFileManifest {
    name: String,
    byte_count: u64,
    fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BundleManifest {
    schema_version: u8,
    database_schema_version: i64,
    bundle_fingerprint: String,
    counts: SnapshotCounts,
    files: Vec<BundleFileManifest>,
}

#[derive(Debug)]
struct MaintenanceLock {
    path: PathBuf,
}

impl MaintenanceLock {
    fn acquire(database_path: &Path) -> Result<Self, MaintenanceError> {
        let path = suffixed_path(database_path, ".maintenance.lock");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == ErrorKind::AlreadyExists {
                    MaintenanceError::MaintenanceLocked(display_path(&path))
                } else {
                    io_error("creating maintenance lock", &path, error)
                }
            })?;
        writeln!(file, "pid={}", process::id())
            .map_err(|error| io_error("writing maintenance lock", &path, error))?;
        file.sync_all()
            .map_err(|error| io_error("syncing maintenance lock", &path, error))?;
        Ok(Self { path })
    }
}

impl Drop for MaintenanceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(super) fn export_bundle(
    options: BundleExportOptions,
) -> Result<SqliteMaintenanceReport, MaintenanceError> {
    let database_path = absolute_existing_path(&options.database_path)?;
    let output_directory = absolute_output_path(&options.output_directory)?;
    if output_directory.exists() {
        return Err(MaintenanceError::TargetExists(display_path(
            &output_directory,
        )));
    }

    let _lock = MaintenanceLock::acquire(&database_path)?;
    require_healthy_database(&database_path)?;
    let mut repository = SqliteRepository::open(&database_path)?;
    let snapshot = repository.read_consistent_snapshot()?;
    let database_schema_version = repository.schema_version()?;
    let counts = SnapshotCounts::from_snapshot(&snapshot)?;
    drop(repository);

    let temporary_directory = suffixed_path(&output_directory, ".tmp");
    if temporary_directory.exists() {
        return Err(MaintenanceError::TemporaryArtifactExists(display_path(
            &temporary_directory,
        )));
    }

    let result = (|| {
        fs::create_dir_all(&temporary_directory).map_err(|error| {
            io_error(
                "creating temporary bundle directory",
                &temporary_directory,
                error,
            )
        })?;

        let files = serialize_snapshot(&snapshot)?;
        let mut file_manifest = Vec::with_capacity(BUNDLE_FILES.len());
        for filename in BUNDLE_FILES {
            let bytes = files.get(filename).ok_or_else(|| {
                MaintenanceError::InvalidData(format!(
                    "serializer did not produce required file {filename}"
                ))
            })?;
            let path = temporary_directory.join(filename);
            write_bytes_sync(&path, bytes)?;
            file_manifest.push(BundleFileManifest {
                name: filename.to_string(),
                byte_count: u64::try_from(bytes.len()).map_err(|_| {
                    MaintenanceError::InvalidData(format!(
                        "serialized file {filename} exceeds supported size"
                    ))
                })?,
                fingerprint: fingerprint_bytes(bytes),
            });
        }

        let bundle_fingerprint = fingerprint_manifest_files(&file_manifest);
        let manifest = BundleManifest {
            schema_version: PORTABLE_BUNDLE_SCHEMA_VERSION,
            database_schema_version,
            bundle_fingerprint: bundle_fingerprint.clone(),
            counts: counts.clone(),
            files: file_manifest,
        };
        let mut manifest_bytes =
            serde_json::to_vec_pretty(&manifest).map_err(|error| MaintenanceError::Json {
                path: MANIFEST_FILENAME.to_string(),
                message: error.to_string(),
            })?;
        manifest_bytes.push(b'\n');
        write_bytes_sync(
            &temporary_directory.join(MANIFEST_FILENAME),
            &manifest_bytes,
        )?;
        sync_directory(&temporary_directory)?;
        ensure_parent(&output_directory)?;
        fs::rename(&temporary_directory, &output_directory)
            .map_err(|error| io_error("publishing portable bundle", &output_directory, error))?;
        sync_parent(&output_directory)?;

        Ok(SqliteMaintenanceReport {
            schema_version: MAINTENANCE_REPORT_SCHEMA_VERSION,
            operation: "sqlite-export".to_string(),
            status: "exported".to_string(),
            source_path: Some(display_path(&database_path)),
            target_path: Some(display_path(&output_directory)),
            previous_database_path: None,
            bundle_fingerprint: Some(bundle_fingerprint),
            database_schema_version: Some(database_schema_version),
            counts: Some(counts),
            healthy: Some(true),
            checks: vec![pass_check(
                "consistent-snapshot",
                "all bundle files were produced from one SQLite read transaction",
            )],
        })
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary_directory);
    }
    result
}

pub(super) fn import_bundle(
    options: BundleImportOptions,
) -> Result<SqliteMaintenanceReport, MaintenanceError> {
    let bundle_directory = absolute_existing_path(&options.bundle_directory)?;
    if !bundle_directory.is_dir() {
        return Err(MaintenanceError::InvalidBundle(format!(
            "{} is not a directory",
            display_path(&bundle_directory)
        )));
    }
    let (manifest, snapshot) = read_bundle(&bundle_directory)?;
    if manifest.database_schema_version != CURRENT_SCHEMA_VERSION {
        return Err(MaintenanceError::InvalidBundle(format!(
            "bundle schema target {} does not match supported SQLite schema {}",
            manifest.database_schema_version, CURRENT_SCHEMA_VERSION
        )));
    }

    if options.dry_run {
        let temporary_path = dry_run_import_path()?;
        remove_database_artifacts(&temporary_path);
        let result = (|| {
            let schema_version = validate_import_candidate(
                &temporary_path,
                &snapshot,
                &manifest.bundle_fingerprint,
            )?;
            Ok(SqliteMaintenanceReport {
                schema_version: MAINTENANCE_REPORT_SCHEMA_VERSION,
                operation: "sqlite-import".to_string(),
                status: "validated".to_string(),
                source_path: Some(display_path(&bundle_directory)),
                target_path: None,
                previous_database_path: None,
                bundle_fingerprint: Some(manifest.bundle_fingerprint.clone()),
                database_schema_version: Some(schema_version),
                counts: Some(manifest.counts.clone()),
                healthy: Some(true),
                checks: vec![
                    pass_check("manifest", "all file sizes and fingerprints matched"),
                    pass_check(
                        "validation-only",
                        "the full import and repository reconciliation passed without publication",
                    ),
                    pass_check(
                        "round-trip",
                        "the disposable repository snapshot matched the bundle exactly",
                    ),
                ],
            })
        })();
        remove_database_artifacts(&temporary_path);
        return result;
    }

    let database_path = absolute_output_path(&options.database_path)?;
    if database_path.exists() {
        return Err(MaintenanceError::TargetExists(display_path(&database_path)));
    }
    ensure_no_sidecars(&database_path)?;
    ensure_parent(&database_path)?;

    let _lock = MaintenanceLock::acquire(&database_path)?;
    let temporary_path = suffixed_path(&database_path, ".import.tmp");
    if temporary_path.exists() {
        return Err(MaintenanceError::TemporaryArtifactExists(display_path(
            &temporary_path,
        )));
    }
    ensure_parent(&temporary_path)?;

    let result = (|| {
        let schema_version =
            validate_import_candidate(&temporary_path, &snapshot, &manifest.bundle_fingerprint)?;
        sync_file(&temporary_path)?;
        fs::rename(&temporary_path, &database_path)
            .map_err(|error| io_error("publishing imported database", &database_path, error))?;
        sync_parent(&database_path)?;

        Ok(SqliteMaintenanceReport {
            schema_version: MAINTENANCE_REPORT_SCHEMA_VERSION,
            operation: "sqlite-import".to_string(),
            status: "imported".to_string(),
            source_path: Some(display_path(&bundle_directory)),
            target_path: Some(display_path(&database_path)),
            previous_database_path: None,
            bundle_fingerprint: Some(manifest.bundle_fingerprint),
            database_schema_version: Some(schema_version),
            counts: Some(manifest.counts),
            healthy: Some(true),
            checks: vec![
                pass_check("manifest", "all file sizes and fingerprints matched"),
                pass_check(
                    "round-trip",
                    "the imported repository snapshot matched the bundle exactly",
                ),
            ],
        })
    })();

    if result.is_err() {
        remove_database_artifacts(&temporary_path);
    }
    result
}

fn validate_import_candidate(
    temporary_path: &Path,
    snapshot: &RepositorySnapshot,
    bundle_fingerprint: &str,
) -> Result<i64, MaintenanceError> {
    let mut repository = SqliteRepository::open(temporary_path)?;
    import_snapshot(&mut repository, snapshot, bundle_fingerprint)?;
    checkpoint_database(&repository.connection)?;
    let imported = repository.read_consistent_snapshot()?;
    if imported != *snapshot {
        return Err(MaintenanceError::SnapshotMismatch);
    }
    let schema_version = repository.schema_version()?;
    drop(repository);
    require_healthy_database(temporary_path)?;
    Ok(schema_version)
}

fn dry_run_import_path() -> Result<PathBuf, MaintenanceError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| MaintenanceError::InvalidData(error.to_string()))?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "strata-sqlite-import-validation-{}-{nonce}.sqlite3",
        process::id()
    )))
}

pub(super) fn doctor(options: DoctorOptions) -> Result<SqliteMaintenanceReport, MaintenanceError> {
    let database_path = absolute_existing_path(&options.database_path)?;
    let marker_path = options
        .authority_marker_path
        .as_deref()
        .map(absolute_output_path)
        .transpose()?;
    doctor_at(&database_path, marker_path.as_deref())
}

pub(super) fn backup(options: BackupOptions) -> Result<SqliteMaintenanceReport, MaintenanceError> {
    let database_path = absolute_existing_path(&options.database_path)?;
    let backup_path = absolute_output_path(&options.backup_path)?;
    if backup_path.exists() {
        return Err(MaintenanceError::TargetExists(display_path(&backup_path)));
    }
    ensure_no_sidecars(&backup_path)?;

    let _lock = MaintenanceLock::acquire(&database_path)?;
    require_healthy_database(&database_path)?;
    let temporary_path = suffixed_path(&backup_path, ".tmp");
    if temporary_path.exists() {
        return Err(MaintenanceError::TemporaryArtifactExists(display_path(
            &temporary_path,
        )));
    }
    ensure_parent(&temporary_path)?;

    let result = (|| {
        let connection = Connection::open(&database_path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute("VACUUM INTO ?1", params![path_text(&temporary_path)?])?;
        drop(connection);

        require_healthy_database(&temporary_path)?;
        let source_snapshot = read_repository_snapshot(&database_path)?;
        let backup_snapshot = read_repository_snapshot(&temporary_path)?;
        if source_snapshot != backup_snapshot {
            return Err(MaintenanceError::SnapshotMismatch);
        }
        let counts = SnapshotCounts::from_snapshot(&source_snapshot)?;
        sync_file(&temporary_path)?;
        fs::rename(&temporary_path, &backup_path)
            .map_err(|error| io_error("publishing SQLite backup", &backup_path, error))?;
        sync_parent(&backup_path)?;

        Ok(SqliteMaintenanceReport {
            schema_version: MAINTENANCE_REPORT_SCHEMA_VERSION,
            operation: "sqlite-backup".to_string(),
            status: "backed-up".to_string(),
            source_path: Some(display_path(&database_path)),
            target_path: Some(display_path(&backup_path)),
            previous_database_path: None,
            bundle_fingerprint: None,
            database_schema_version: Some(CURRENT_SCHEMA_VERSION),
            counts: Some(counts),
            healthy: Some(true),
            checks: vec![
                pass_check("vacuum-into", "backup was produced by SQLite"),
                pass_check(
                    "snapshot-parity",
                    "backup and source repository snapshots matched",
                ),
            ],
        })
    })();

    if result.is_err() {
        remove_database_artifacts(&temporary_path);
    }
    result
}

pub(super) fn restore(
    options: RestoreOptions,
) -> Result<SqliteMaintenanceReport, MaintenanceError> {
    let backup_path = absolute_existing_path(&options.backup_path)?;
    let database_path = absolute_output_path(&options.database_path)?;
    require_healthy_database(&backup_path)?;
    ensure_no_sidecars(&database_path)?;
    ensure_parent(&database_path)?;

    if database_path.exists() && !options.replace {
        return Err(MaintenanceError::TargetExists(display_path(&database_path)));
    }

    let _lock = MaintenanceLock::acquire(&database_path)?;
    let temporary_path = suffixed_path(&database_path, ".restore.tmp");
    let previous_path = suffixed_path(&database_path, ".restore-previous");
    if temporary_path.exists() {
        return Err(MaintenanceError::TemporaryArtifactExists(display_path(
            &temporary_path,
        )));
    }
    if database_path.exists() && previous_path.exists() {
        return Err(MaintenanceError::TemporaryArtifactExists(display_path(
            &previous_path,
        )));
    }
    ensure_parent(&temporary_path)?;

    let result = (|| {
        fs::copy(&backup_path, &temporary_path)
            .map_err(|error| io_error("copying restore candidate", &temporary_path, error))?;
        sync_file(&temporary_path)?;
        require_healthy_database(&temporary_path)?;

        let backup_snapshot = read_repository_snapshot(&backup_path)?;
        let restored_snapshot = read_repository_snapshot(&temporary_path)?;
        if backup_snapshot != restored_snapshot {
            return Err(MaintenanceError::SnapshotMismatch);
        }
        let counts = SnapshotCounts::from_snapshot(&backup_snapshot)?;

        let mut moved_previous = false;
        if database_path.exists() {
            fs::rename(&database_path, &previous_path)
                .map_err(|error| io_error("preserving previous database", &previous_path, error))?;
            moved_previous = true;
        }

        if let Err(error) = fs::rename(&temporary_path, &database_path) {
            if moved_previous {
                let _ = fs::rename(&previous_path, &database_path);
            }
            return Err(io_error(
                "publishing restored database",
                &database_path,
                error,
            ));
        }
        sync_parent(&database_path)?;

        Ok(SqliteMaintenanceReport {
            schema_version: MAINTENANCE_REPORT_SCHEMA_VERSION,
            operation: "sqlite-restore".to_string(),
            status: "restored".to_string(),
            source_path: Some(display_path(&backup_path)),
            target_path: Some(display_path(&database_path)),
            previous_database_path: moved_previous.then(|| display_path(&previous_path)),
            bundle_fingerprint: None,
            database_schema_version: Some(CURRENT_SCHEMA_VERSION),
            counts: Some(counts),
            healthy: Some(true),
            checks: vec![
                pass_check(
                    "temporary-verification",
                    "restore candidate passed doctor before publication",
                ),
                pass_check(
                    "snapshot-parity",
                    "restore candidate and backup repository snapshots matched",
                ),
            ],
        })
    })();

    if result.is_err() {
        remove_database_artifacts(&temporary_path);
    }
    result
}

fn doctor_at(
    database_path: &Path,
    authority_marker_path: Option<&Path>,
) -> Result<SqliteMaintenanceReport, MaintenanceError> {
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(std::time::Duration::from_secs(2))?;
    connection.pragma_update(None, "foreign_keys", true)?;

    let mut checks = Vec::new();
    let schema_version: i64 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    checks.push(check(
        "schema-version",
        schema_version == CURRENT_SCHEMA_VERSION,
        format!("found {schema_version}; supported {CURRENT_SCHEMA_VERSION}"),
    ));

    let integrity_results = pragma_text_rows(&connection, "PRAGMA integrity_check")?;
    let integrity_ok = integrity_results.len() == 1 && integrity_results[0] == "ok";
    checks.push(check(
        "integrity-check",
        integrity_ok,
        integrity_results.join("; "),
    ));

    let foreign_key_violations = foreign_key_violations(&connection)?;
    checks.push(check(
        "foreign-key-check",
        foreign_key_violations.is_empty(),
        if foreign_key_violations.is_empty() {
            "no violations".to_string()
        } else {
            foreign_key_violations.join("; ")
        },
    ));

    let required_tables = [
        "schema_migrations",
        "database_metadata",
        "categories",
        "sessions",
        "active_session",
        "runtime_checkpoint",
        "sand_state",
        "sand_snapshots",
        "legacy_imports",
        "category_tags",
        "category_lifecycle_receipts",
    ];
    let existing_tables = sqlite_tables(&connection)?;
    let missing_tables: Vec<_> = required_tables
        .iter()
        .filter(|table| !existing_tables.contains(**table))
        .copied()
        .collect();
    checks.push(check(
        "required-tables",
        missing_tables.is_empty(),
        if missing_tables.is_empty() {
            "all required tables are present".to_string()
        } else {
            format!("missing {}", missing_tables.join(", "))
        },
    ));

    let lifecycle_issues = if existing_tables.contains("category_lifecycle_receipts") {
        database_category_lifecycle_issues(&connection)?
    } else {
        vec!["category_lifecycle_receipts table missing".to_string()]
    };
    checks.push(check(
        "category-lifecycle-integrity",
        lifecycle_issues.is_empty(),
        if lifecycle_issues.is_empty() {
            "all lifecycle receipts and retired identities are coherent".to_string()
        } else {
            lifecycle_issues.join("; ")
        },
    ));

    let metadata_authority = if existing_tables.contains("database_metadata") {
        connection
            .query_row(
                "SELECT value FROM database_metadata WHERE key = 'storage_authority'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
    } else {
        None
    };
    let authority_ok = matches!(metadata_authority.as_deref(), Some("sqlite"));
    checks.push(check(
        "database-authority-metadata",
        authority_ok,
        metadata_authority.unwrap_or_else(|| "missing".to_string()),
    ));

    let pending_imports = if existing_tables.contains("legacy_imports") {
        connection.query_row(
            "SELECT count(*) FROM legacy_imports WHERE status = 'pending'",
            [],
            |row| row.get::<_, i64>(0),
        )?
    } else {
        -1
    };
    checks.push(check(
        "pending-imports",
        pending_imports == 0,
        if pending_imports >= 0 {
            pending_imports.to_string()
        } else {
            "legacy_imports table missing".to_string()
        },
    ));

    if let Some(marker_path) = authority_marker_path {
        checks.push(check_authority_marker(marker_path, database_path)?);
    }

    let healthy = checks.iter().all(|item| item.passed);
    let counts = if healthy {
        Some(SnapshotCounts::from_snapshot(&read_repository_snapshot(
            database_path,
        )?)?)
    } else {
        None
    };

    Ok(SqliteMaintenanceReport {
        schema_version: MAINTENANCE_REPORT_SCHEMA_VERSION,
        operation: "sqlite-doctor".to_string(),
        status: if healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        source_path: Some(display_path(database_path)),
        target_path: None,
        previous_database_path: None,
        bundle_fingerprint: None,
        database_schema_version: Some(schema_version),
        counts,
        healthy: Some(healthy),
        checks,
    })
}

fn require_healthy_database(path: &Path) -> Result<(), MaintenanceError> {
    let report = doctor_at(path, None)?;
    if report.is_healthy() {
        Ok(())
    } else {
        Err(MaintenanceError::DoctorFailed)
    }
}

fn check_authority_marker(
    marker_path: &Path,
    database_path: &Path,
) -> Result<MaintenanceCheck, MaintenanceError> {
    if !marker_path.exists() {
        return Ok(check(
            "authority-marker",
            false,
            format!("missing {}", display_path(marker_path)),
        ));
    }
    let bytes = read_bytes(marker_path)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| MaintenanceError::Json {
            path: display_path(marker_path),
            message: error.to_string(),
        })?;
    let active_authority = value
        .get("active_authority")
        .and_then(serde_json::Value::as_str);
    let candidate = value.get("sqlite_candidate");
    let status = candidate
        .and_then(|entry| entry.get("status"))
        .and_then(serde_json::Value::as_str);
    let marked_database = candidate
        .and_then(|entry| entry.get("database_path"))
        .and_then(serde_json::Value::as_str);
    let marked_path_matches = marked_database
        .map(Path::new)
        .map(absolute_output_path)
        .transpose()?
        .is_some_and(|path| path == database_path);
    let passed = active_authority == Some("legacy-files")
        && status == Some("verified")
        && marked_path_matches;
    Ok(check(
        "authority-marker",
        passed,
        format!(
            "active={}, candidate={}, path-match={}",
            active_authority.unwrap_or("missing"),
            status.unwrap_or("missing"),
            marked_path_matches
        ),
    ))
}

fn serialize_snapshot(
    snapshot: &RepositorySnapshot,
) -> Result<BTreeMap<&'static str, Vec<u8>>, MaintenanceError> {
    let mut files = BTreeMap::new();
    files.insert(
        CATEGORIES_FILENAME,
        serialize_categories(&snapshot.categories)?,
    );
    files.insert(
        CATEGORY_TAGS_FILENAME,
        serialize_category_tags(&snapshot.category_tags)?,
    );
    files.insert(SESSIONS_FILENAME, serialize_sessions(&snapshot.sessions)?);
    files.insert(
        ACTIVE_SESSION_FILENAME,
        serialize_active_session(snapshot.active_session.as_ref())?,
    );
    files.insert(
        CHECKPOINT_FILENAME,
        serialize_checkpoint(snapshot.checkpoint.as_ref())?,
    );
    files.insert(
        SAND_STATE_FILENAME,
        serialize_sand_state(snapshot.sand_state.as_ref())?,
    );
    files.insert(
        SAND_SNAPSHOTS_FILENAME,
        serialize_sand_snapshots(&snapshot.sand_snapshots)?,
    );
    files.insert(
        CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
        serialize_category_lifecycle_receipts(&snapshot.category_lifecycle_receipts)?,
    );
    Ok(files)
}

fn csv_writer() -> WriterBuilder {
    let mut builder = WriterBuilder::new();
    builder
        .has_headers(false)
        .terminator(Terminator::Any(b'\n'));
    builder
}

fn finish_writer(writer: csv::Writer<Vec<u8>>, path: &str) -> Result<Vec<u8>, MaintenanceError> {
    writer.into_inner().map_err(|error| MaintenanceError::Csv {
        path: path.to_string(),
        message: error.error().to_string(),
    })
}

fn serialize_categories(categories: &[CategoryRecord]) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        CATEGORIES_FILENAME,
        [
            "id",
            "name",
            "description",
            "color_index",
            "balance_effect",
            "archived_at_utc",
        ],
    )?;
    for category in categories {
        write_record(
            &mut writer,
            CATEGORIES_FILENAME,
            [
                category.id.to_string(),
                category.name.clone(),
                category.description.clone(),
                category.color_index.to_string(),
                category.balance_effect.to_string(),
                category.archived_at_utc.clone().unwrap_or_default(),
            ],
        )?;
    }
    finish_writer(writer, CATEGORIES_FILENAME)
}

fn serialize_category_tags(tags: &BTreeMap<i64, Vec<String>>) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        CATEGORY_TAGS_FILENAME,
        ["category_id", "ordinal", "tag"],
    )?;
    for (category_id, values) in tags {
        for (ordinal, tag) in values.iter().enumerate() {
            write_record(
                &mut writer,
                CATEGORY_TAGS_FILENAME,
                [category_id.to_string(), ordinal.to_string(), tag.clone()],
            )?;
        }
    }
    finish_writer(writer, CATEGORY_TAGS_FILENAME)
}

fn serialize_sessions(sessions: &[SessionRecord]) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        SESSIONS_FILENAME,
        [
            "id",
            "stable_id",
            "project",
            "category_id",
            "description",
            "started_at_utc",
            "ended_at_utc",
            "operational_day",
            "elapsed_seconds",
            "boundary_utc_offset_seconds",
            "boundary_start_minutes",
            "source",
        ],
    )?;
    for session in sessions {
        write_record(
            &mut writer,
            SESSIONS_FILENAME,
            [
                session.id.to_string(),
                session.stable_id.clone(),
                session.project.clone(),
                session.category_id.to_string(),
                session.description.clone(),
                session.started_at_utc.clone(),
                session.ended_at_utc.clone(),
                session.operational_day.clone(),
                session.elapsed_seconds.to_string(),
                session
                    .boundary_utc_offset_seconds
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                session
                    .boundary_start_minutes
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                session.source.clone(),
            ],
        )?;
    }
    finish_writer(writer, SESSIONS_FILENAME)
}

fn serialize_active_session(
    active: Option<&ActiveSessionRecord>,
) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        ACTIVE_SESSION_FILENAME,
        [
            "stable_id",
            "project",
            "category_id",
            "description",
            "started_at_utc",
            "recovery_kind",
        ],
    )?;
    if let Some(active) = active {
        write_record(
            &mut writer,
            ACTIVE_SESSION_FILENAME,
            [
                active.stable_id.clone(),
                active.project.clone(),
                active.category_id.to_string(),
                active.description.clone(),
                active.started_at_utc.clone(),
                active.recovery_kind.clone(),
            ],
        )?;
    }
    finish_writer(writer, ACTIVE_SESSION_FILENAME)
}

fn serialize_checkpoint(
    checkpoint: Option<&CheckpointRecord>,
) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        CHECKPOINT_FILENAME,
        [
            "status",
            "detached_at_utc",
            "simulation_time_utc",
            "active_session_stable_id",
            "payload_json",
        ],
    )?;
    if let Some(checkpoint) = checkpoint {
        write_record(
            &mut writer,
            CHECKPOINT_FILENAME,
            [
                checkpoint_status_name(checkpoint.status).to_string(),
                checkpoint.detached_at_utc.clone(),
                checkpoint.simulation_time_utc.clone(),
                checkpoint
                    .active_session_stable_id
                    .clone()
                    .unwrap_or_default(),
                checkpoint.payload_json.clone(),
            ],
        )?;
    }
    finish_writer(writer, CHECKPOINT_FILENAME)
}

fn serialize_sand_state(state: Option<&SandStateRecord>) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        SAND_STATE_FILENAME,
        [
            "formation_id",
            "quantum_seconds",
            "grid_width",
            "grid_height",
            "payload_json",
            "updated_at_utc",
        ],
    )?;
    if let Some(state) = state {
        write_record(
            &mut writer,
            SAND_STATE_FILENAME,
            [
                state.formation_id.clone(),
                state.quantum_seconds.to_string(),
                state.grid_width.to_string(),
                state.grid_height.to_string(),
                state.payload_json.clone(),
                state.updated_at_utc.clone(),
            ],
        )?;
    }
    finish_writer(writer, SAND_STATE_FILENAME)
}

fn serialize_sand_snapshots(snapshots: &[SandSnapshotRecord]) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        SAND_SNAPSHOTS_FILENAME,
        [
            "id",
            "formation_id",
            "snapshot_kind",
            "operational_day",
            "quantum_seconds",
            "payload_json",
            "captured_at_utc",
        ],
    )?;
    for snapshot in snapshots {
        write_record(
            &mut writer,
            SAND_SNAPSHOTS_FILENAME,
            [
                snapshot.id.to_string(),
                snapshot.formation_id.clone(),
                snapshot_kind_name(snapshot.snapshot_kind).to_string(),
                snapshot.operational_day.clone().unwrap_or_default(),
                snapshot.quantum_seconds.to_string(),
                snapshot.payload_json.clone(),
                snapshot.captured_at_utc.clone(),
            ],
        )?;
    }
    finish_writer(writer, SAND_SNAPSHOTS_FILENAME)
}

fn serialize_category_lifecycle_receipts(
    receipts: &[CategoryLifecycleReceiptRecord],
) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
        [
            "operation_id",
            "operation_kind",
            "source_category_id",
            "target_category_id",
            "source_metadata_json",
            "target_metadata_json",
            "preview_revision",
            "reference_counts_json",
            "applied_at_utc",
        ],
    )?;
    for receipt in receipts {
        write_record(
            &mut writer,
            CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
            [
                receipt.operation_id.clone(),
                receipt.operation_kind.clone(),
                receipt.source_category_id.to_string(),
                receipt
                    .target_category_id
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                receipt.source_metadata_json.clone(),
                receipt.target_metadata_json.clone().unwrap_or_default(),
                receipt.preview_revision.clone(),
                receipt.reference_counts_json.clone(),
                receipt.applied_at_utc.clone(),
            ],
        )?;
    }
    finish_writer(writer, CATEGORY_LIFECYCLE_RECEIPTS_FILENAME)
}

fn write_record<I, T>(
    writer: &mut csv::Writer<Vec<u8>>,
    path: &str,
    record: I,
) -> Result<(), MaintenanceError>
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    writer
        .write_record(record)
        .map_err(|error| MaintenanceError::Csv {
            path: path.to_string(),
            message: error.to_string(),
        })
}

fn read_bundle(directory: &Path) -> Result<(BundleManifest, RepositorySnapshot), MaintenanceError> {
    validate_bundle_entries(directory)?;
    let manifest_path = directory.join(MANIFEST_FILENAME);
    let manifest_bytes = read_bytes(&manifest_path)?;
    let manifest: BundleManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|error| MaintenanceError::Json {
            path: display_path(&manifest_path),
            message: error.to_string(),
        })?;
    if manifest.schema_version != PORTABLE_BUNDLE_SCHEMA_VERSION {
        return Err(MaintenanceError::InvalidBundle(format!(
            "unsupported manifest schema {}; expected {}",
            manifest.schema_version, PORTABLE_BUNDLE_SCHEMA_VERSION
        )));
    }
    let expected_names: Vec<_> = BUNDLE_FILES.iter().map(|name| name.to_string()).collect();
    let actual_names: Vec<_> = manifest
        .files
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
    if actual_names != expected_names {
        return Err(MaintenanceError::InvalidBundle(
            "manifest file order or membership is invalid".to_string(),
        ));
    }

    let mut bytes_by_name = BTreeMap::new();
    for entry in &manifest.files {
        let path = directory.join(&entry.name);
        let bytes = read_bytes(&path)?;
        let byte_count = u64::try_from(bytes.len()).map_err(|_| {
            MaintenanceError::InvalidBundle(format!("{} exceeds supported size", entry.name))
        })?;
        if byte_count != entry.byte_count {
            return Err(MaintenanceError::InvalidBundle(format!(
                "{} byte count differs from manifest",
                entry.name
            )));
        }
        if fingerprint_bytes(&bytes) != entry.fingerprint {
            return Err(MaintenanceError::InvalidBundle(format!(
                "{} fingerprint differs from manifest",
                entry.name
            )));
        }
        bytes_by_name.insert(entry.name.clone(), bytes);
    }
    if fingerprint_manifest_files(&manifest.files) != manifest.bundle_fingerprint {
        return Err(MaintenanceError::InvalidBundle(
            "bundle fingerprint differs from file manifest".to_string(),
        ));
    }

    let snapshot = parse_snapshot(&bytes_by_name)?;
    let counts = SnapshotCounts::from_snapshot(&snapshot)?;
    if counts != manifest.counts {
        return Err(MaintenanceError::InvalidBundle(
            "manifest counts differ from parsed bundle contents".to_string(),
        ));
    }
    validate_snapshot_references(&snapshot)?;
    Ok((manifest, snapshot))
}

fn validate_bundle_entries(directory: &Path) -> Result<(), MaintenanceError> {
    let mut names = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| io_error("reading bundle directory", directory, error))?
    {
        let entry =
            entry.map_err(|error| io_error("reading bundle directory", directory, error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| io_error("reading bundle entry type", &entry.path(), error))?;
        if !file_type.is_file() {
            return Err(MaintenanceError::InvalidBundle(format!(
                "unexpected non-file entry {}",
                display_path(&entry.path())
            )));
        }
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    let mut expected: Vec<_> = BUNDLE_FILES.iter().map(|name| name.to_string()).collect();
    expected.push(MANIFEST_FILENAME.to_string());
    expected.sort();
    if names != expected {
        return Err(MaintenanceError::InvalidBundle(format!(
            "bundle entries differ from required set: found {}",
            names.join(", ")
        )));
    }
    Ok(())
}

fn parse_snapshot(
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<RepositorySnapshot, MaintenanceError> {
    Ok(RepositorySnapshot {
        categories: parse_categories(required_file(files, CATEGORIES_FILENAME)?)?,
        category_tags: parse_category_tags(required_file(files, CATEGORY_TAGS_FILENAME)?)?,
        sessions: parse_sessions(required_file(files, SESSIONS_FILENAME)?)?,
        active_session: parse_active_session(required_file(files, ACTIVE_SESSION_FILENAME)?)?,
        checkpoint: parse_checkpoint(required_file(files, CHECKPOINT_FILENAME)?)?,
        sand_state: parse_sand_state(required_file(files, SAND_STATE_FILENAME)?)?,
        sand_snapshots: parse_sand_snapshots(required_file(files, SAND_SNAPSHOTS_FILENAME)?)?,
        category_lifecycle_receipts: parse_category_lifecycle_receipts(required_file(
            files,
            CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
        )?)?,
    })
}

fn required_file<'a>(
    files: &'a BTreeMap<String, Vec<u8>>,
    name: &str,
) -> Result<&'a [u8], MaintenanceError> {
    files
        .get(name)
        .map(Vec::as_slice)
        .ok_or_else(|| MaintenanceError::InvalidBundle(format!("missing {name}")))
}

fn csv_records<'a>(
    path: &'a str,
    bytes: &'a [u8],
    expected_header: &[&str],
) -> Result<Vec<StringRecord>, MaintenanceError> {
    let mut reader = ReaderBuilder::new().has_headers(true).from_reader(bytes);
    let header = reader
        .headers()
        .map_err(|error| csv_error(path, error))?
        .clone();
    if header.iter().collect::<Vec<_>>() != expected_header {
        return Err(MaintenanceError::InvalidBundle(format!(
            "{path} header is invalid"
        )));
    }
    reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| csv_error(path, error))
}

fn parse_categories(bytes: &[u8]) -> Result<Vec<CategoryRecord>, MaintenanceError> {
    let records = csv_records(
        CATEGORIES_FILENAME,
        bytes,
        &[
            "id",
            "name",
            "description",
            "color_index",
            "balance_effect",
            "archived_at_utc",
        ],
    )?;
    let mut categories = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        categories.push(CategoryRecord {
            id: parse_i64(CATEGORIES_FILENAME, index, field(record, 0)?, "id")?,
            name: field(record, 1)?.to_string(),
            description: field(record, 2)?.to_string(),
            color_index: parse_i64(CATEGORIES_FILENAME, index, field(record, 3)?, "color_index")?,
            balance_effect: parse_i64(
                CATEGORIES_FILENAME,
                index,
                field(record, 4)?,
                "balance_effect",
            )?,
            archived_at_utc: optional_string(field(record, 5)?),
        });
    }
    Ok(categories)
}

fn parse_category_tags(bytes: &[u8]) -> Result<BTreeMap<i64, Vec<String>>, MaintenanceError> {
    let records = csv_records(
        CATEGORY_TAGS_FILENAME,
        bytes,
        &["category_id", "ordinal", "tag"],
    )?;
    let mut tags: BTreeMap<i64, Vec<(usize, String)>> = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        let category_id = parse_i64(
            CATEGORY_TAGS_FILENAME,
            index,
            field(record, 0)?,
            "category_id",
        )?;
        let ordinal = parse_usize(CATEGORY_TAGS_FILENAME, index, field(record, 1)?, "ordinal")?;
        let tag = field(record, 2)?.to_string();
        tags.entry(category_id).or_default().push((ordinal, tag));
    }
    let mut result = BTreeMap::new();
    for (category_id, mut values) in tags {
        values.sort_by_key(|(ordinal, _)| *ordinal);
        for (expected, (actual, _)) in values.iter().enumerate() {
            if expected != *actual {
                return Err(MaintenanceError::InvalidBundle(format!(
                    "{CATEGORY_TAGS_FILENAME} ordinals for category {category_id} are not contiguous"
                )));
            }
        }
        result.insert(
            category_id,
            values.into_iter().map(|(_, tag)| tag).collect(),
        );
    }
    Ok(result)
}

fn parse_sessions(bytes: &[u8]) -> Result<Vec<SessionRecord>, MaintenanceError> {
    let records = csv_records(
        SESSIONS_FILENAME,
        bytes,
        &[
            "id",
            "stable_id",
            "project",
            "category_id",
            "description",
            "started_at_utc",
            "ended_at_utc",
            "operational_day",
            "elapsed_seconds",
            "boundary_utc_offset_seconds",
            "boundary_start_minutes",
            "source",
        ],
    )?;
    let mut sessions = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        sessions.push(SessionRecord {
            id: parse_i64(SESSIONS_FILENAME, index, field(record, 0)?, "id")?,
            stable_id: field(record, 1)?.to_string(),
            project: field(record, 2)?.to_string(),
            category_id: parse_i64(SESSIONS_FILENAME, index, field(record, 3)?, "category_id")?,
            description: field(record, 4)?.to_string(),
            started_at_utc: field(record, 5)?.to_string(),
            ended_at_utc: field(record, 6)?.to_string(),
            operational_day: field(record, 7)?.to_string(),
            elapsed_seconds: parse_i64(
                SESSIONS_FILENAME,
                index,
                field(record, 8)?,
                "elapsed_seconds",
            )?,
            boundary_utc_offset_seconds: optional_i64(
                SESSIONS_FILENAME,
                index,
                field(record, 9)?,
                "boundary_utc_offset_seconds",
            )?,
            boundary_start_minutes: optional_i64(
                SESSIONS_FILENAME,
                index,
                field(record, 10)?,
                "boundary_start_minutes",
            )?,
            source: field(record, 11)?.to_string(),
        });
    }
    Ok(sessions)
}

fn parse_active_session(bytes: &[u8]) -> Result<Option<ActiveSessionRecord>, MaintenanceError> {
    let records = csv_records(
        ACTIVE_SESSION_FILENAME,
        bytes,
        &[
            "stable_id",
            "project",
            "category_id",
            "description",
            "started_at_utc",
            "recovery_kind",
        ],
    )?;
    if records.len() > 1 {
        return Err(MaintenanceError::InvalidBundle(format!(
            "{ACTIVE_SESSION_FILENAME} contains more than one row"
        )));
    }
    records
        .first()
        .map(|record| {
            Ok(ActiveSessionRecord {
                stable_id: field(record, 0)?.to_string(),
                project: field(record, 1)?.to_string(),
                category_id: parse_i64(
                    ACTIVE_SESSION_FILENAME,
                    0,
                    field(record, 2)?,
                    "category_id",
                )?,
                description: field(record, 3)?.to_string(),
                started_at_utc: field(record, 4)?.to_string(),
                recovery_kind: field(record, 5)?.to_string(),
            })
        })
        .transpose()
}

fn parse_checkpoint(bytes: &[u8]) -> Result<Option<CheckpointRecord>, MaintenanceError> {
    let records = csv_records(
        CHECKPOINT_FILENAME,
        bytes,
        &[
            "status",
            "detached_at_utc",
            "simulation_time_utc",
            "active_session_stable_id",
            "payload_json",
        ],
    )?;
    if records.len() > 1 {
        return Err(MaintenanceError::InvalidBundle(format!(
            "{CHECKPOINT_FILENAME} contains more than one row"
        )));
    }
    records
        .first()
        .map(|record| {
            Ok(CheckpointRecord {
                status: parse_checkpoint_status(field(record, 0)?)?,
                detached_at_utc: field(record, 1)?.to_string(),
                simulation_time_utc: field(record, 2)?.to_string(),
                active_session_stable_id: optional_string(field(record, 3)?),
                payload_json: field(record, 4)?.to_string(),
            })
        })
        .transpose()
}

fn parse_sand_state(bytes: &[u8]) -> Result<Option<SandStateRecord>, MaintenanceError> {
    let records = csv_records(
        SAND_STATE_FILENAME,
        bytes,
        &[
            "formation_id",
            "quantum_seconds",
            "grid_width",
            "grid_height",
            "payload_json",
            "updated_at_utc",
        ],
    )?;
    if records.len() > 1 {
        return Err(MaintenanceError::InvalidBundle(format!(
            "{SAND_STATE_FILENAME} contains more than one row"
        )));
    }
    records
        .first()
        .map(|record| {
            Ok(SandStateRecord {
                formation_id: field(record, 0)?.to_string(),
                quantum_seconds: parse_i64(
                    SAND_STATE_FILENAME,
                    0,
                    field(record, 1)?,
                    "quantum_seconds",
                )?,
                grid_width: parse_i64(SAND_STATE_FILENAME, 0, field(record, 2)?, "grid_width")?,
                grid_height: parse_i64(SAND_STATE_FILENAME, 0, field(record, 3)?, "grid_height")?,
                payload_json: field(record, 4)?.to_string(),
                updated_at_utc: field(record, 5)?.to_string(),
            })
        })
        .transpose()
}

fn parse_sand_snapshots(bytes: &[u8]) -> Result<Vec<SandSnapshotRecord>, MaintenanceError> {
    let records = csv_records(
        SAND_SNAPSHOTS_FILENAME,
        bytes,
        &[
            "id",
            "formation_id",
            "snapshot_kind",
            "operational_day",
            "quantum_seconds",
            "payload_json",
            "captured_at_utc",
        ],
    )?;
    let mut snapshots = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        snapshots.push(SandSnapshotRecord {
            id: parse_i64(SAND_SNAPSHOTS_FILENAME, index, field(record, 0)?, "id")?,
            formation_id: field(record, 1)?.to_string(),
            snapshot_kind: parse_snapshot_kind(field(record, 2)?)?,
            operational_day: optional_string(field(record, 3)?),
            quantum_seconds: parse_i64(
                SAND_SNAPSHOTS_FILENAME,
                index,
                field(record, 4)?,
                "quantum_seconds",
            )?,
            payload_json: field(record, 5)?.to_string(),
            captured_at_utc: field(record, 6)?.to_string(),
        });
    }
    Ok(snapshots)
}

fn parse_category_lifecycle_receipts(
    bytes: &[u8],
) -> Result<Vec<CategoryLifecycleReceiptRecord>, MaintenanceError> {
    let records = csv_records(
        CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
        bytes,
        &[
            "operation_id",
            "operation_kind",
            "source_category_id",
            "target_category_id",
            "source_metadata_json",
            "target_metadata_json",
            "preview_revision",
            "reference_counts_json",
            "applied_at_utc",
        ],
    )?;
    let mut receipts = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        receipts.push(CategoryLifecycleReceiptRecord {
            operation_id: field(record, 0)?.to_string(),
            operation_kind: field(record, 1)?.to_string(),
            source_category_id: parse_i64(
                CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
                index,
                field(record, 2)?,
                "source_category_id",
            )?,
            target_category_id: optional_i64(
                CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
                index,
                field(record, 3)?,
                "target_category_id",
            )?,
            source_metadata_json: field(record, 4)?.to_string(),
            target_metadata_json: optional_string(field(record, 5)?),
            preview_revision: field(record, 6)?.to_string(),
            reference_counts_json: field(record, 7)?.to_string(),
            applied_at_utc: field(record, 8)?.to_string(),
        });
    }
    Ok(receipts)
}

fn validate_category_lifecycle_receipt(
    receipt: &CategoryLifecycleReceiptRecord,
) -> Result<(), MaintenanceError> {
    require_text(&receipt.operation_id, "category lifecycle operation id")?;
    require_text(
        &receipt.preview_revision,
        "category lifecycle preview revision",
    )?;
    require_text(
        &receipt.applied_at_utc,
        "category lifecycle application timestamp",
    )?;
    DateTime::parse_from_rfc3339(&receipt.applied_at_utc).map_err(|error| {
        MaintenanceError::InvalidBundle(format!(
            "category lifecycle application timestamp is invalid: {error}"
        ))
    })?;
    if receipt.source_category_id <= 0 {
        return Err(MaintenanceError::InvalidBundle(
            "category lifecycle receipt source must be a positive identity".to_string(),
        ));
    }
    let source: CategoryIdentitySnapshot = serde_json::from_str(&receipt.source_metadata_json)
        .map_err(|error| {
            MaintenanceError::InvalidBundle(format!(
                "category lifecycle source metadata is invalid: {error}"
            ))
        })?;
    if source.id != receipt.source_category_id {
        return Err(MaintenanceError::InvalidBundle(
            "category lifecycle source metadata identity does not match its receipt".to_string(),
        ));
    }
    let _counts: CategoryReferenceCounts = serde_json::from_str(&receipt.reference_counts_json)
        .map_err(|error| {
            MaintenanceError::InvalidBundle(format!(
                "category lifecycle reference counts are invalid: {error}"
            ))
        })?;
    match receipt.operation_kind.as_str() {
        "merge" => {
            let target_id = receipt.target_category_id.ok_or_else(|| {
                MaintenanceError::InvalidBundle(
                    "merge receipt has no target category identity".to_string(),
                )
            })?;
            if target_id <= 0 || target_id == receipt.source_category_id {
                return Err(MaintenanceError::InvalidBundle(
                    "merge receipt has an invalid target category identity".to_string(),
                ));
            }
            let target_json = receipt.target_metadata_json.as_deref().ok_or_else(|| {
                MaintenanceError::InvalidBundle("merge receipt has no target metadata".to_string())
            })?;
            let target: CategoryIdentitySnapshot =
                serde_json::from_str(target_json).map_err(|error| {
                    MaintenanceError::InvalidBundle(format!(
                        "category lifecycle target metadata is invalid: {error}"
                    ))
                })?;
            if target.id != target_id {
                return Err(MaintenanceError::InvalidBundle(
                    "category lifecycle target metadata identity does not match its receipt"
                        .to_string(),
                ));
            }
        }
        "delete" => {
            if receipt.target_category_id.is_some() || receipt.target_metadata_json.is_some() {
                return Err(MaintenanceError::InvalidBundle(
                    "delete receipt unexpectedly names a target category".to_string(),
                ));
            }
        }
        other => {
            return Err(MaintenanceError::InvalidBundle(format!(
                "unknown category lifecycle operation kind {other}"
            )));
        }
    }
    Ok(())
}

fn database_category_lifecycle_issues(
    connection: &Connection,
) -> Result<Vec<String>, MaintenanceError> {
    let mut statement = connection.prepare(
        "SELECT operation_id, operation_kind, source_category_id, target_category_id,
                source_metadata_json, target_metadata_json, preview_revision,
                reference_counts_json, applied_at_utc
         FROM category_lifecycle_receipts
         ORDER BY applied_at_utc, operation_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(CategoryLifecycleReceiptRecord {
            operation_id: row.get(0)?,
            operation_kind: row.get(1)?,
            source_category_id: row.get(2)?,
            target_category_id: row.get(3)?,
            source_metadata_json: row.get(4)?,
            target_metadata_json: row.get(5)?,
            preview_revision: row.get(6)?,
            reference_counts_json: row.get(7)?,
            applied_at_utc: row.get(8)?,
        })
    })?;
    let mut issues = Vec::new();
    let mut operation_ids = BTreeSet::new();
    for row in rows {
        let receipt = row?;
        if !operation_ids.insert(receipt.operation_id.clone()) {
            issues.push(format!(
                "duplicate lifecycle operation id {}",
                receipt.operation_id
            ));
        }
        if let Err(error) = validate_category_lifecycle_receipt(&receipt) {
            issues.push(error.to_string());
        }
    }
    let mut collision_statement = connection.prepare(
        "SELECT categories.id
         FROM categories
         JOIN category_lifecycle_receipts
           ON category_lifecycle_receipts.source_category_id = categories.id
         ORDER BY categories.id",
    )?;
    let collisions = collision_statement.query_map([], |row| row.get::<_, i64>(0))?;
    for collision in collisions {
        issues.push(format!(
            "retired category identity {} is present in the active catalog",
            collision?
        ));
    }
    Ok(issues)
}

fn validate_snapshot_references(snapshot: &RepositorySnapshot) -> Result<(), MaintenanceError> {
    if snapshot.categories.is_empty() {
        return Err(MaintenanceError::InvalidBundle(
            "categories.csv contains no idle category".to_string(),
        ));
    }

    let mut category_ids = BTreeSet::new();
    let mut retired_category_ids = BTreeSet::new();
    let mut lifecycle_operation_ids = BTreeSet::new();
    for receipt in &snapshot.category_lifecycle_receipts {
        validate_category_lifecycle_receipt(receipt)?;
        if !lifecycle_operation_ids.insert(receipt.operation_id.as_str()) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate category lifecycle operation id {}",
                receipt.operation_id
            )));
        }
        retired_category_ids.insert(receipt.source_category_id);
    }

    let mut active_names = BTreeSet::new();
    for category in &snapshot.categories {
        if !category_ids.insert(category.id) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate category id {}",
                category.id
            )));
        }
        if retired_category_ids.contains(&category.id) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "category id {} was retired by a lifecycle receipt and cannot be active",
                category.id
            )));
        }
        require_text(&category.name, "category name")?;
        if category.color_index < 0 || !(-1..=1).contains(&category.balance_effect) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "category {} has invalid display or balance values",
                category.id
            )));
        }
        if category.archived_at_utc.is_none()
            && !active_names.insert(category.name.to_ascii_lowercase())
        {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate active category name {}",
                category.name
            )));
        }
    }
    let idle = snapshot
        .categories
        .iter()
        .find(|category| category.id == 0)
        .ok_or_else(|| {
            MaintenanceError::InvalidBundle("reserved idle category id 0 is missing".to_string())
        })?;
    if idle.name != "idle" || idle.archived_at_utc.is_some() || idle.balance_effect != 0 {
        return Err(MaintenanceError::InvalidBundle(
            "reserved idle category is invalid".to_string(),
        ));
    }

    for (category_id, tags) in &snapshot.category_tags {
        require_category(*category_id, &category_ids)?;
        let mut seen = BTreeSet::new();
        for tag in tags {
            require_text(tag, "category tag")?;
            if !seen.insert(tag) {
                return Err(MaintenanceError::InvalidBundle(format!(
                    "category {category_id} has duplicate tag {tag}"
                )));
            }
        }
    }

    let mut session_ids = BTreeSet::new();
    let mut stable_ids = BTreeSet::new();
    for session in &snapshot.sessions {
        if !session_ids.insert(session.id) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate session id {}",
                session.id
            )));
        }
        if !stable_ids.insert(session.stable_id.as_str()) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate session stable id {}",
                session.stable_id
            )));
        }
        validate_session_record(session)?;
        require_category(session.category_id, &category_ids)?;
    }

    if let Some(active) = &snapshot.active_session {
        require_text(&active.stable_id, "active session stable id")?;
        require_text(&active.started_at_utc, "active session start timestamp")?;
        require_category(active.category_id, &category_ids)?;
        if stable_ids.contains(active.stable_id.as_str()) {
            return Err(MaintenanceError::InvalidBundle(
                "active stable id duplicates a completed session".to_string(),
            ));
        }
        if !matches!(
            active.recovery_kind.as_str(),
            "live" | "detached" | "recovered"
        ) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "unknown active recovery kind {}",
                active.recovery_kind
            )));
        }
    }

    if let Some(checkpoint) = &snapshot.checkpoint {
        require_text(&checkpoint.detached_at_utc, "checkpoint detached timestamp")?;
        require_text(
            &checkpoint.simulation_time_utc,
            "checkpoint simulation timestamp",
        )?;
        validate_json(&checkpoint.payload_json, "checkpoint payload")?;
        if let Some(stable_id) = &checkpoint.active_session_stable_id {
            let active_matches = snapshot
                .active_session
                .as_ref()
                .is_some_and(|active| active.stable_id == *stable_id);
            if !active_matches {
                return Err(MaintenanceError::InvalidBundle(
                    "checkpoint active identity does not match active_session.csv".to_string(),
                ));
            }
        }
    }

    if let Some(state) = &snapshot.sand_state {
        validate_sand_state_record(state)?;
    }

    let mut snapshot_ids = BTreeSet::new();
    for item in &snapshot.sand_snapshots {
        if !snapshot_ids.insert(item.id) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate sand snapshot id {}",
                item.id
            )));
        }
        require_text(&item.formation_id, "snapshot formation id")?;
        require_text(&item.captured_at_utc, "snapshot capture timestamp")?;
        if item.quantum_seconds <= 0 {
            return Err(MaintenanceError::InvalidBundle(
                "snapshot quantum must be positive".to_string(),
            ));
        }
        if matches!(
            item.snapshot_kind,
            SnapshotKind::Daily | SnapshotKind::DailyContribution
        ) && item.operational_day.is_none()
        {
            return Err(MaintenanceError::InvalidBundle(
                "daily snapshot requires an operational day".to_string(),
            ));
        }
        validate_json(&item.payload_json, "sand snapshot payload")?;
    }
    Ok(())
}

fn validate_session_record(session: &SessionRecord) -> Result<(), MaintenanceError> {
    require_text(&session.stable_id, "session stable id")?;
    require_text(&session.started_at_utc, "session start timestamp")?;
    require_text(&session.ended_at_utc, "session end timestamp")?;
    require_text(&session.operational_day, "session operational day")?;
    require_text(&session.source, "session source")?;
    if session.elapsed_seconds < 0 {
        return Err(MaintenanceError::InvalidBundle(
            "session elapsed seconds cannot be negative".to_string(),
        ));
    }
    Ok(())
}

fn validate_sand_state_record(state: &SandStateRecord) -> Result<(), MaintenanceError> {
    require_text(&state.formation_id, "sand formation id")?;
    require_text(&state.updated_at_utc, "sand update timestamp")?;
    if state.quantum_seconds <= 0 || state.grid_width < 0 || state.grid_height < 0 {
        return Err(MaintenanceError::InvalidBundle(
            "sand state dimensions or quantum are invalid".to_string(),
        ));
    }
    validate_json(&state.payload_json, "sand state payload")
}

fn import_snapshot(
    repository: &mut SqliteRepository,
    snapshot: &RepositorySnapshot,
    bundle_fingerprint: &str,
) -> Result<(), MaintenanceError> {
    validate_snapshot_references(snapshot)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;

    let idle = snapshot
        .categories
        .iter()
        .find(|category| category.id == 0)
        .ok_or_else(|| MaintenanceError::InvalidBundle("idle category missing".to_string()))?;
    transaction.execute(
        "UPDATE categories
         SET name = ?1, description = ?2, color_index = ?3, balance_effect = ?4,
             archived_at_utc = NULL
         WHERE id = 0",
        params![
            idle.name,
            idle.description,
            idle.color_index,
            idle.balance_effect,
        ],
    )?;

    for category in snapshot
        .categories
        .iter()
        .filter(|category| category.id != 0)
    {
        transaction.execute(
            "INSERT INTO categories(
                id, name, description, color_index, balance_effect, archived_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                category.id,
                category.name,
                category.description,
                category.color_index,
                category.balance_effect,
                category.archived_at_utc,
            ],
        )?;
    }

    for (category_id, tags) in &snapshot.category_tags {
        for (ordinal, tag) in tags.iter().enumerate() {
            let ordinal = i64::try_from(ordinal).map_err(|_| {
                MaintenanceError::InvalidBundle("too many category tags".to_string())
            })?;
            transaction.execute(
                "INSERT INTO category_tags(category_id, ordinal, tag)
                 VALUES (?1, ?2, ?3)",
                params![category_id, ordinal, tag],
            )?;
        }
    }

    for session in &snapshot.sessions {
        transaction.execute(
            "INSERT INTO sessions(
                id, stable_id, project, category_id, description, started_at_utc,
                ended_at_utc, operational_day, elapsed_seconds,
                boundary_utc_offset_seconds, boundary_start_minutes, source
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                session.id,
                session.stable_id,
                session.project,
                session.category_id,
                session.description,
                session.started_at_utc,
                session.ended_at_utc,
                session.operational_day,
                session.elapsed_seconds,
                session.boundary_utc_offset_seconds,
                session.boundary_start_minutes,
                session.source,
            ],
        )?;
    }

    if let Some(active) = &snapshot.active_session {
        transaction.execute(
            "INSERT INTO active_session(
                singleton, stable_id, project, category_id, description,
                started_at_utc, recovery_kind
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
    }

    if let Some(checkpoint) = &snapshot.checkpoint {
        transaction.execute(
            "INSERT INTO runtime_checkpoint(
                singleton, status, detached_at_utc, simulation_time_utc,
                active_session_stable_id, payload_json
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![
                checkpoint_status_name(checkpoint.status),
                checkpoint.detached_at_utc,
                checkpoint.simulation_time_utc,
                checkpoint.active_session_stable_id,
                checkpoint.payload_json,
            ],
        )?;
    }

    if let Some(state) = &snapshot.sand_state {
        transaction.execute(
            "INSERT INTO sand_state(
                singleton, formation_id, quantum_seconds, grid_width, grid_height,
                payload_json, updated_at_utc
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                state.formation_id,
                state.quantum_seconds,
                state.grid_width,
                state.grid_height,
                state.payload_json,
                state.updated_at_utc,
            ],
        )?;
    }

    for item in &snapshot.sand_snapshots {
        transaction.execute(
            "INSERT INTO sand_snapshots(
                id, formation_id, snapshot_kind, operational_day,
                quantum_seconds, payload_json, captured_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                item.id,
                item.formation_id,
                snapshot_kind_name(item.snapshot_kind),
                item.operational_day,
                item.quantum_seconds,
                item.payload_json,
                item.captured_at_utc,
            ],
        )?;
    }

    for receipt in &snapshot.category_lifecycle_receipts {
        transaction.execute(
            "INSERT INTO category_lifecycle_receipts(
                operation_id, operation_kind, source_category_id, target_category_id,
                source_metadata_json, target_metadata_json, preview_revision,
                reference_counts_json, applied_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                receipt.operation_id,
                receipt.operation_kind,
                receipt.source_category_id,
                receipt.target_category_id,
                receipt.source_metadata_json,
                receipt.target_metadata_json,
                receipt.preview_revision,
                receipt.reference_counts_json,
                receipt.applied_at_utc,
            ],
        )?;
    }

    transaction.execute(
        "INSERT INTO database_metadata(key, value)
         VALUES ('portable_bundle_fingerprint', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![bundle_fingerprint],
    )?;
    transaction.commit()?;
    Ok(())
}

fn read_repository_snapshot(path: &Path) -> Result<RepositorySnapshot, MaintenanceError> {
    let mut repository = SqliteRepository::open(path)?;
    repository.read_consistent_snapshot().map_err(Into::into)
}

fn checkpoint_database(connection: &Connection) -> Result<(), MaintenanceError> {
    let result: (i64, i64, i64) =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
    if result.0 != 0 {
        return Err(MaintenanceError::InvalidData(format!(
            "WAL checkpoint remained busy: {result:?}"
        )));
    }
    Ok(())
}

fn pragma_text_rows(connection: &Connection, sql: &str) -> Result<Vec<String>, MaintenanceError> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn foreign_key_violations(connection: &Connection) -> Result<Vec<String>, MaintenanceError> {
    let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
    let rows = statement.query_map([], |row| {
        let table: String = row.get(0)?;
        let rowid: Option<i64> = row.get(1)?;
        let parent: String = row.get(2)?;
        let constraint: i64 = row.get(3)?;
        Ok(format!(
            "table={table}, rowid={}, parent={parent}, constraint={constraint}",
            rowid.map_or_else(|| "null".to_string(), |value| value.to_string())
        ))
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn sqlite_tables(connection: &Connection) -> Result<BTreeSet<String>, MaintenanceError> {
    let mut statement =
        connection.prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<BTreeSet<_>, _>>().map_err(Into::into)
}

fn checkpoint_status_name(status: CheckpointStatus) -> &'static str {
    match status {
        CheckpointStatus::Pending => "pending",
        CheckpointStatus::Recovering => "recovering",
        CheckpointStatus::Committed => "committed",
        CheckpointStatus::Quarantined => "quarantined",
    }
}

fn parse_checkpoint_status(value: &str) -> Result<CheckpointStatus, MaintenanceError> {
    match value {
        "pending" => Ok(CheckpointStatus::Pending),
        "recovering" => Ok(CheckpointStatus::Recovering),
        "committed" => Ok(CheckpointStatus::Committed),
        "quarantined" => Ok(CheckpointStatus::Quarantined),
        other => Err(MaintenanceError::InvalidBundle(format!(
            "unknown checkpoint status {other}"
        ))),
    }
}

fn snapshot_kind_name(kind: SnapshotKind) -> &'static str {
    match kind {
        SnapshotKind::Daily => "daily",
        SnapshotKind::DailyContribution => "daily-contribution",
        SnapshotKind::Manual => "manual",
        SnapshotKind::FormationEnd => "formation_end",
        SnapshotKind::Recovery => "recovery",
    }
}

fn parse_snapshot_kind(value: &str) -> Result<SnapshotKind, MaintenanceError> {
    match value {
        "daily" => Ok(SnapshotKind::Daily),
        "daily-contribution" => Ok(SnapshotKind::DailyContribution),
        "manual" => Ok(SnapshotKind::Manual),
        "formation_end" => Ok(SnapshotKind::FormationEnd),
        "recovery" => Ok(SnapshotKind::Recovery),
        other => Err(MaintenanceError::InvalidBundle(format!(
            "unknown snapshot kind {other}"
        ))),
    }
}

fn fingerprint_manifest_files(files: &[BundleFileManifest]) -> String {
    let mut state = Fnv64::new();
    for file in files {
        state.update(file.name.as_bytes());
        state.update(&[0]);
        state.update(file.byte_count.to_string().as_bytes());
        state.update(&[0]);
        state.update(file.fingerprint.as_bytes());
        state.update(&[0xff]);
    }
    state.finish()
}

fn fingerprint_bytes(bytes: &[u8]) -> String {
    let mut state = Fnv64::new();
    state.update(bytes);
    state.finish()
}

struct Fnv64(u64);

impl Fnv64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn finish(self) -> String {
        format!("{:016x}", self.0)
    }
}

fn field(record: &StringRecord, index: usize) -> Result<&str, MaintenanceError> {
    record
        .get(index)
        .ok_or_else(|| MaintenanceError::InvalidBundle("CSV row is incomplete".to_string()))
}

fn parse_i64(
    path: &str,
    row_index: usize,
    value: &str,
    field_name: &str,
) -> Result<i64, MaintenanceError> {
    value.parse::<i64>().map_err(|_| {
        MaintenanceError::InvalidBundle(format!(
            "{path} row {} has invalid {field_name}",
            row_index + 2
        ))
    })
}

fn parse_usize(
    path: &str,
    row_index: usize,
    value: &str,
    field_name: &str,
) -> Result<usize, MaintenanceError> {
    value.parse::<usize>().map_err(|_| {
        MaintenanceError::InvalidBundle(format!(
            "{path} row {} has invalid {field_name}",
            row_index + 2
        ))
    })
}

fn optional_i64(
    path: &str,
    index: usize,
    value: &str,
    field_name: &str,
) -> Result<Option<i64>, MaintenanceError> {
    if value.is_empty() {
        Ok(None)
    } else {
        parse_i64(path, index, value, field_name).map(Some)
    }
}

fn optional_string(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn require_text(value: &str, label: &str) -> Result<(), MaintenanceError> {
    if value.trim().is_empty() {
        return Err(MaintenanceError::InvalidBundle(format!("{label} is empty")));
    }
    Ok(())
}

fn require_category(
    category_id: i64,
    category_ids: &BTreeSet<i64>,
) -> Result<(), MaintenanceError> {
    if category_ids.contains(&category_id) {
        Ok(())
    } else {
        Err(MaintenanceError::InvalidBundle(format!(
            "unknown category reference {category_id}"
        )))
    }
}

fn validate_json(value: &str, label: &str) -> Result<(), MaintenanceError> {
    serde_json::from_str::<serde_json::Value>(value)
        .map(|_| ())
        .map_err(|error| {
            MaintenanceError::InvalidBundle(format!("{label} is invalid JSON: {error}"))
        })
}

fn check(name: &str, passed: bool, detail: String) -> MaintenanceCheck {
    MaintenanceCheck {
        name: name.to_string(),
        passed,
        detail,
    }
}

fn pass_check(name: &str, detail: &str) -> MaintenanceCheck {
    check(name, true, detail.to_string())
}

fn csv_error(path: &str, error: csv::Error) -> MaintenanceError {
    MaintenanceError::Csv {
        path: path.to_string(),
        message: error.to_string(),
    }
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, MaintenanceError> {
    let mut file = File::open(path).map_err(|error| io_error("opening", path, error))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| io_error("reading", path, error))?;
    Ok(bytes)
}

fn write_bytes_sync(path: &Path, bytes: &[u8]) -> Result<(), MaintenanceError> {
    ensure_parent(path)?;
    let mut file = File::create(path).map_err(|error| io_error("creating", path, error))?;
    file.write_all(bytes)
        .map_err(|error| io_error("writing", path, error))?;
    file.sync_all()
        .map_err(|error| io_error("syncing", path, error))
}

fn sync_file(path: &Path) -> Result<(), MaintenanceError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| io_error("syncing", path, error))
}

fn sync_directory(path: &Path) -> Result<(), MaintenanceError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| io_error("syncing directory", path, error))
}

fn sync_parent(path: &Path) -> Result<(), MaintenanceError> {
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), MaintenanceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| io_error("creating parent directory", parent, error))?;
    }
    Ok(())
}

fn ensure_no_sidecars(database_path: &Path) -> Result<(), MaintenanceError> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = suffixed_path(database_path, suffix);
        if sidecar.exists() {
            return Err(MaintenanceError::DatabaseSidecar(display_path(&sidecar)));
        }
    }
    Ok(())
}

fn remove_database_artifacts(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(suffixed_path(path, "-wal"));
    let _ = fs::remove_file(suffixed_path(path, "-shm"));
}

fn absolute_existing_path(path: &Path) -> Result<PathBuf, MaintenanceError> {
    path.canonicalize()
        .map_err(|error| io_error("resolving", path, error))
}

fn absolute_output_path(path: &Path) -> Result<PathBuf, MaintenanceError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| io_error("resolving current directory for", path, error))
}

fn path_text(path: &Path) -> Result<&str, MaintenanceError> {
    path.to_str().ok_or_else(|| {
        MaintenanceError::InvalidData(format!("path is not valid UTF-8: {}", display_path(path)))
    })
}

fn suffixed_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn io_error(operation: &'static str, path: &Path, error: std::io::Error) -> MaintenanceError {
    MaintenanceError::Io {
        operation,
        path: display_path(path),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::sqlite::{
        NewActiveSession,
        category_lifecycle::{CategoryLifecycleRequest, apply, preview},
        repository::{NewCategoryRecord, NewSandSnapshotRecord, NewSessionRecord},
    };

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "strata_sqlite005_{label}_{}_{}",
            process::id(),
            nanos
        ))
    }

    fn fixture_database(path: &Path) -> RepositorySnapshot {
        let mut repository = SqliteRepository::open(path).unwrap();
        let study_id = repository
            .create_category(&NewCategoryRecord {
                name: "Study",
                description: "Deep reading",
                color_index: 2,
                balance_effect: 1,
            })
            .unwrap();
        let disposable_id = repository
            .create_category(&NewCategoryRecord {
                name: "Disposable",
                description: "retired before export",
                color_index: 3,
                balance_effect: 0,
            })
            .unwrap();
        let lifecycle_preview = preview(&repository, disposable_id, None).unwrap();
        apply(
            &mut repository,
            CategoryLifecycleRequest {
                source_category_id: disposable_id,
                target_category_id: None,
                expected_revision: &lifecycle_preview.revision,
                applied_at_utc: "2026-08-01T14:00:00Z",
            },
        )
        .unwrap();
        repository
            .replace_category_tags(study_id, &["reading".to_string(), "focus".to_string()])
            .unwrap();
        repository
            .insert_session(&NewSessionRecord {
                stable_id: "session-7",
                project: "Notebook",
                category_id: study_id,
                description: "Read chapter",
                started_at_utc: "2026-08-01T15:00:00Z",
                ended_at_utc: "2026-08-01T16:00:00Z",
                operational_day: "2026-08-01",
                elapsed_seconds: 3600,
                boundary_utc_offset_seconds: -21600,
                boundary_start_minutes: 360,
                source: "fixture",
            })
            .unwrap();
        repository
            .start_session(&NewActiveSession {
                stable_id: "active-8",
                project: "Strata",
                category_id: study_id,
                description: "Repository work",
                started_at_utc: "2026-08-01T16:00:00Z",
                recovery_kind: "detached",
            })
            .unwrap();
        repository
            .save_checkpoint(&CheckpointRecord {
                status: CheckpointStatus::Pending,
                detached_at_utc: "2026-08-01T16:15:00Z".to_string(),
                simulation_time_utc: "2026-08-01T16:15:00Z".to_string(),
                active_session_stable_id: Some("active-8".to_string()),
                payload_json: r#"{"pending":[]}"#.to_string(),
            })
            .unwrap();
        repository
            .save_sand_state(&SandStateRecord {
                formation_id: "formation-1".to_string(),
                quantum_seconds: 1,
                grid_width: 10,
                grid_height: 5,
                payload_json: r#"{"grains":[{"x":1,"y":2,"category_id":1}]}"#.to_string(),
                updated_at_utc: "2026-08-01T16:15:00Z".to_string(),
            })
            .unwrap();
        repository
            .insert_sand_snapshot(&NewSandSnapshotRecord {
                formation_id: "formation-1",
                snapshot_kind: SnapshotKind::Daily,
                operational_day: Some("2026-08-01"),
                quantum_seconds: 1,
                payload_json: r#"{"grains":[]}"#,
                captured_at_utc: "2026-08-01T16:00:00Z",
            })
            .unwrap();
        repository.read_consistent_snapshot().unwrap()
    }

    fn collect_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            files.insert(
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).unwrap(),
            );
        }
        files
    }

    #[test]
    fn deterministic_bundle_round_trip_preserves_repository_snapshot() {
        let root = unique_root("bundle_roundtrip");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.sqlite3");
        let expected = fixture_database(&source);
        let bundle_a = root.join("bundle-a");
        let bundle_b = root.join("bundle-b");
        let imported = root.join("imported.sqlite3");

        export_bundle(BundleExportOptions {
            database_path: source.clone(),
            output_directory: bundle_a.clone(),
        })
        .unwrap();
        export_bundle(BundleExportOptions {
            database_path: source.clone(),
            output_directory: bundle_b.clone(),
        })
        .unwrap();

        assert_eq!(collect_files(&bundle_a), collect_files(&bundle_b));
        assert!(bundle_a.join(CATEGORY_LIFECYCLE_RECEIPTS_FILENAME).exists());

        import_bundle(BundleImportOptions {
            bundle_directory: bundle_a.clone(),
            database_path: imported.clone(),
            dry_run: false,
        })
        .unwrap();

        assert_eq!(read_repository_snapshot(&imported).unwrap(), expected);
        assert!(doctor_at(&imported, None).unwrap().is_healthy());
        let mut imported_repository = SqliteRepository::open(&imported).unwrap();
        let post_import_id = imported_repository
            .create_category(&NewCategoryRecord {
                name: "After bundle",
                description: "",
                color_index: 4,
                balance_effect: 0,
            })
            .unwrap();
        assert_eq!(
            post_import_id, 3,
            "portable round-trip must preserve retired category identity"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bundle_fingerprint_rejects_modified_source_bytes() {
        let root = unique_root("bundle_corrupt");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.sqlite3");
        fixture_database(&source);
        let bundle = root.join("bundle");
        export_bundle(BundleExportOptions {
            database_path: source,
            output_directory: bundle.clone(),
        })
        .unwrap();

        let sessions = bundle.join(SESSIONS_FILENAME);
        let mut content = fs::read_to_string(&sessions).unwrap();
        content.push('\n');
        fs::write(&sessions, content).unwrap();

        let error = import_bundle(BundleImportOptions {
            bundle_directory: bundle,
            database_path: root.join("imported.sqlite3"),
            dry_run: false,
        })
        .unwrap_err();
        assert!(
            error.to_string().contains("byte count") || error.to_string().contains("fingerprint")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn doctor_rejects_tampered_lifecycle_receipts_and_retired_identity_reuse() {
        let root = unique_root("lifecycle_doctor");
        fs::create_dir_all(&root).unwrap();
        let database = root.join("lifecycle.sqlite3");
        fixture_database(&database);
        assert!(doctor_at(&database, None).unwrap().is_healthy());

        let connection = Connection::open(&database).unwrap();
        connection
            .execute(
                "UPDATE category_lifecycle_receipts
                 SET source_metadata_json = '{}'
                 WHERE source_category_id = 2",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO categories(
                    id, name, description, color_index, balance_effect, archived_at_utc,
                    sort_order
                 ) VALUES (2, 'Reused', '', 1, 0, NULL, 99)",
                [],
            )
            .unwrap();
        drop(connection);

        let report = doctor_at(&database, None).unwrap();
        assert!(!report.is_healthy());
        let lifecycle = report
            .checks
            .iter()
            .find(|check| check.name == "category-lifecycle-integrity")
            .unwrap();
        assert!(!lifecycle.passed);
        assert!(lifecycle.detail.contains("source metadata"));
        assert!(lifecycle.detail.contains("retired category identity 2"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn doctor_detects_unsupported_schema_and_foreign_key_damage() {
        let root = unique_root("doctor");
        fs::create_dir_all(&root).unwrap();
        let database = root.join("doctor.sqlite3");
        fixture_database(&database);

        {
            let connection = Connection::open(&database).unwrap();
            connection
                .pragma_update(None, "foreign_keys", false)
                .unwrap();
            connection
                .execute(
                    "UPDATE sessions SET category_id = 999 WHERE stable_id = 'session-7'",
                    [],
                )
                .unwrap();
            connection
                .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
                .unwrap();
        }

        let report = doctor_at(&database, None).unwrap();
        assert!(!report.is_healthy());
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "schema-version" && !check.passed)
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "foreign-key-check" && !check.passed)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn backup_and_restore_verify_snapshot_before_publication() {
        let root = unique_root("backup_restore");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.sqlite3");
        let expected = fixture_database(&source);
        let backup_path = root.join("backup.sqlite3");
        let restored = root.join("restored.sqlite3");

        backup(BackupOptions {
            database_path: source,
            backup_path: backup_path.clone(),
        })
        .unwrap();
        restore(RestoreOptions {
            backup_path,
            database_path: restored.clone(),
            replace: false,
        })
        .unwrap();

        assert_eq!(read_repository_snapshot(&restored).unwrap(), expected);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_preserves_previous_database_and_refuses_stale_temporary_state() {
        let root = unique_root("restore_safety");
        fs::create_dir_all(&root).unwrap();
        let backup_path = root.join("backup.sqlite3");
        let target = root.join("target.sqlite3");
        fixture_database(&backup_path);
        SqliteRepository::open(&target).unwrap();

        let stale = suffixed_path(&target, ".restore.tmp");
        fs::write(&stale, b"interrupted").unwrap();
        let before = fs::read(&target).unwrap();

        let error = restore(RestoreOptions {
            backup_path: backup_path.clone(),
            database_path: target.clone(),
            replace: true,
        })
        .unwrap_err();
        assert!(matches!(
            error,
            MaintenanceError::TemporaryArtifactExists(_)
        ));
        assert_eq!(fs::read(&target).unwrap(), before);

        fs::remove_file(stale).unwrap();
        let report = restore(RestoreOptions {
            backup_path,
            database_path: target.clone(),
            replace: true,
        })
        .unwrap();
        let previous = report.previous_database_path.unwrap();
        assert!(Path::new(&previous).exists());
        assert!(doctor_at(&target, None).unwrap().is_healthy());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn maintenance_lock_prevents_concurrent_backup() {
        let root = unique_root("lock");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.sqlite3");
        fixture_database(&source);
        let lock = suffixed_path(&source, ".maintenance.lock");
        fs::write(&lock, b"pid=1\n").unwrap();

        let error = backup(BackupOptions {
            database_path: source,
            backup_path: root.join("backup.sqlite3"),
        })
        .unwrap_err();
        assert!(matches!(error, MaintenanceError::MaintenanceLocked(_)));

        let _ = fs::remove_dir_all(root);
    }
}
