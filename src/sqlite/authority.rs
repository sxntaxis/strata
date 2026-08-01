use std::{fs, path::{Path, PathBuf}};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage;

use super::{ControlledMigrationReport, SqliteRepository, migration_command};

const MARKER_SCHEMA_VERSION: u8 = 1;
const ACTIVATION_REPORT_SCHEMA_VERSION: u8 = 1;
const LEGACY_AUTHORITY: &str = "legacy-files";
const ACTIVATING_AUTHORITY: &str = "activating-sqlite-cli";
const SQLITE_CLI_AUTHORITY: &str = "sqlite-cli";
const VERIFIED_CANDIDATE: &str = "verified";
const DATABASE_CANDIDATE: &str = "sqlite-candidate";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeAuthority {
    LegacyFiles,
    SqliteCli { database_path: PathBuf },
}

#[derive(Debug, Clone)]
pub(crate) struct SqliteCliActivationOptions {
    pub database_path: Option<PathBuf>,
    pub confirm: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SqliteCliActivationStatus {
    Activated,
    AlreadyActive,
    RecoveredActivation,
}

impl SqliteCliActivationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Activated => "activated",
            Self::AlreadyActive => "already-active",
            Self::RecoveredActivation => "recovered-activation",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SqliteCliActivationReport {
    pub schema_version: u8,
    pub status: SqliteCliActivationStatus,
    pub active_authority: String,
    pub database_path: String,
    pub source_fingerprint: String,
    pub integrity_check: String,
    pub activated_at_utc: String,
}

impl SqliteCliActivationReport {
    pub fn to_pretty_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn print_human(&self) {
        println!("SQLite CLI activation: {}", self.status.as_str());
        println!("Active authority: {}", self.active_authority);
        println!("Database: {}", self.database_path);
        println!("Source fingerprint: {}", self.source_fingerprint);
        println!("Integrity check: {}", self.integrity_check);
        println!("Activated at: {}", self.activated_at_utc);
        println!("TUI status: blocked until its SQLite cutover");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StorageAuthorityMarker {
    schema_version: u8,
    active_authority: String,
    sqlite_candidate: SqliteCandidateMarker,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_at_utc: Option<String>,
}

#[derive(Debug, Error)]
enum AuthorityError {
    #[error("SQLite CLI activation requires --confirm")]
    ConfirmationRequired,
    #[error("SQLite migration authority marker does not exist: {0}")]
    MissingMarker(String),
    #[error("invalid SQLite authority marker: {0}")]
    InvalidMarker(String),
    #[error("SQLite candidate is not verified")]
    CandidateNotVerified,
    #[error("requested database {requested} does not match verified candidate {verified}")]
    DatabaseMismatch { requested: String, verified: String },
    #[error("SQLite authority activation is in an unsupported state: {0}")]
    UnsupportedAuthority(String),
    #[error("SQLite candidate report does not match its authority marker")]
    ReportMismatch,
    #[error("SQLite authority metadata conflict: expected {expected}, found {found}")]
    MetadataConflict { expected: String, found: String },
    #[error("I/O error: {0}")]
    Io(String),
    #[error("SQLite activation verification failed: {0}")]
    Verification(String),
}

pub(crate) fn authority_marker_path() -> PathBuf {
    storage::get_state_dir().join("storage_authority.json")
}

pub(crate) fn resolve_runtime_authority() -> Result<RuntimeAuthority, String> {
    let path = authority_marker_path();
    if !path.exists() {
        return Ok(RuntimeAuthority::LegacyFiles);
    }

    let marker = read_marker(&path).map_err(|error| error.to_string())?;
    validate_marker(&marker).map_err(|error| error.to_string())?;

    match marker.active_authority.as_str() {
        LEGACY_AUTHORITY => Ok(RuntimeAuthority::LegacyFiles),
        SQLITE_CLI_AUTHORITY => {
            let activation = marker.sqlite_cli_activation.as_ref().ok_or_else(|| {
                AuthorityError::InvalidMarker(
                    "sqlite-cli authority is missing activation provenance".to_string(),
                )
            });
            let activation = activation.map_err(|error| error.to_string())?;
            if activation.status != "active"
                || activation.source_fingerprint != marker.sqlite_candidate.source_fingerprint
                || activation.database_path != marker.sqlite_candidate.database_path
            {
                return Err(AuthorityError::InvalidMarker(
                    "sqlite-cli activation provenance does not match the candidate".to_string(),
                )
                .to_string());
            }
            Ok(RuntimeAuthority::SqliteCli {
                database_path: PathBuf::from(&marker.sqlite_candidate.database_path),
            })
        }
        ACTIVATING_AUTHORITY => Err(
            "SQLite CLI activation was interrupted; rerun `strata activate-sqlite --confirm`"
                .to_string(),
        ),
        other => Err(AuthorityError::UnsupportedAuthority(other.to_string()).to_string()),
    }
}

pub(crate) fn ensure_tui_legacy_allowed() -> Result<(), String> {
    match resolve_runtime_authority()? {
        RuntimeAuthority::LegacyFiles => Ok(()),
        RuntimeAuthority::SqliteCli { .. } => Err(
            "SQLite is authoritative for CLI operations; the legacy-backed TUI is disabled until the TUI SQLite cutover"
                .to_string(),
        ),
    }
}

pub(crate) fn open_cli_repository(path: &Path) -> Result<SqliteRepository, String> {
    let repository = SqliteRepository::open(path).map_err(|error| error.to_string())?;
    let metadata = repository
        .metadata_value("storage_authority")
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| "missing".to_string());
    if metadata != SQLITE_CLI_AUTHORITY {
        return Err(AuthorityError::MetadataConflict {
            expected: SQLITE_CLI_AUTHORITY.to_string(),
            found: metadata,
        }
        .to_string());
    }
    Ok(repository)
}

pub(crate) fn activate_sqlite_cli(
    options: SqliteCliActivationOptions,
) -> Result<SqliteCliActivationReport, String> {
    activate_sqlite_cli_inner(options).map_err(|error| error.to_string())
}

fn activate_sqlite_cli_inner(
    options: SqliteCliActivationOptions,
) -> Result<SqliteCliActivationReport, AuthorityError> {
    if !options.confirm {
        return Err(AuthorityError::ConfirmationRequired);
    }

    let marker_path = authority_marker_path();
    if !marker_path.exists() {
        return Err(AuthorityError::MissingMarker(display_path(&marker_path)));
    }
    let mut marker = read_marker(&marker_path)?;
    validate_marker(&marker)?;

    let verified_database = PathBuf::from(&marker.sqlite_candidate.database_path);
    if let Some(requested) = options.database_path {
        let requested = normalize_existing_path(&requested)?;
        let verified = normalize_existing_path(&verified_database)?;
        if requested != verified {
            return Err(AuthorityError::DatabaseMismatch {
                requested: display_path(&requested),
                verified: display_path(&verified),
            });
        }
    }

    if marker.active_authority == SQLITE_CLI_AUTHORITY {
        let repository = open_repository_for_activation(&verified_database)?;
        let metadata = repository
            .metadata_value("storage_authority")
            .map_err(|error| AuthorityError::Verification(error.to_string()))?
            .unwrap_or_else(|| "missing".to_string());
        if metadata != SQLITE_CLI_AUTHORITY {
            return Err(AuthorityError::MetadataConflict {
                expected: SQLITE_CLI_AUTHORITY.to_string(),
                found: metadata,
            });
        }
        let integrity_check = repository
            .integrity_check()
            .map_err(|error| AuthorityError::Verification(error.to_string()))?;
        let activation = marker.sqlite_cli_activation.as_ref().ok_or_else(|| {
            AuthorityError::InvalidMarker(
                "active sqlite-cli marker has no activation provenance".to_string(),
            )
        })?;
        return Ok(build_report(
            SqliteCliActivationStatus::AlreadyActive,
            &marker,
            integrity_check,
            activation
                .completed_at_utc
                .clone()
                .unwrap_or_else(|| activation.started_at_utc.clone()),
        ));
    }

    if marker.active_authority != LEGACY_AUTHORITY
        && marker.active_authority != ACTIVATING_AUTHORITY
    {
        return Err(AuthorityError::UnsupportedAuthority(
            marker.active_authority.clone(),
        ));
    }

    let migration_report = read_migration_report(&marker)?;
    if migration_report.source_fingerprint != marker.sqlite_candidate.source_fingerprint
        || migration_report.database_path.as_deref()
            != Some(marker.sqlite_candidate.database_path.as_str())
    {
        return Err(AuthorityError::ReportMismatch);
    }

    let mut repository = open_repository_for_activation(&verified_database)?;
    let metadata = repository
        .metadata_value("storage_authority")
        .map_err(|error| AuthorityError::Verification(error.to_string()))?
        .unwrap_or_else(|| "missing".to_string());

    let recovered = marker.active_authority == ACTIVATING_AUTHORITY;
    let started_at_utc = marker
        .sqlite_cli_activation
        .as_ref()
        .map(|activation| activation.started_at_utc.clone())
        .unwrap_or_else(now_utc);

    if metadata == DATABASE_CANDIDATE {
        migration_command::verify_candidate_for_cli_activation(
            &verified_database,
            &migration_report,
        )
        .map_err(|error| AuthorityError::Verification(error.to_string()))?;

        marker.active_authority = ACTIVATING_AUTHORITY.to_string();
        marker.sqlite_cli_activation = Some(SqliteCliActivationMarker {
            status: "activating".to_string(),
            previous_authority: LEGACY_AUTHORITY.to_string(),
            source_fingerprint: marker.sqlite_candidate.source_fingerprint.clone(),
            database_path: marker.sqlite_candidate.database_path.clone(),
            started_at_utc: started_at_utc.clone(),
            completed_at_utc: None,
        });
        write_marker(&marker_path, &marker)?;

        repository
            .transition_storage_authority(DATABASE_CANDIDATE, SQLITE_CLI_AUTHORITY, &started_at_utc)
            .map_err(|error| AuthorityError::Verification(error.to_string()))?;
    } else if metadata != SQLITE_CLI_AUTHORITY {
        return Err(AuthorityError::MetadataConflict {
            expected: format!("{DATABASE_CANDIDATE} or {SQLITE_CLI_AUTHORITY}"),
            found: metadata,
        });
    }

    let integrity_check = repository
        .integrity_check()
        .map_err(|error| AuthorityError::Verification(error.to_string()))?;
    if integrity_check != "ok" {
        return Err(AuthorityError::Verification(format!(
            "integrity check returned {integrity_check}"
        )));
    }

    let completed_at_utc = now_utc();
    marker.active_authority = SQLITE_CLI_AUTHORITY.to_string();
    marker.sqlite_cli_activation = Some(SqliteCliActivationMarker {
        status: "active".to_string(),
        previous_authority: LEGACY_AUTHORITY.to_string(),
        source_fingerprint: marker.sqlite_candidate.source_fingerprint.clone(),
        database_path: marker.sqlite_candidate.database_path.clone(),
        started_at_utc,
        completed_at_utc: Some(completed_at_utc.clone()),
    });
    write_marker(&marker_path, &marker)?;

    Ok(build_report(
        if recovered || metadata == SQLITE_CLI_AUTHORITY {
            SqliteCliActivationStatus::RecoveredActivation
        } else {
            SqliteCliActivationStatus::Activated
        },
        &marker,
        integrity_check,
        completed_at_utc,
    ))
}

fn build_report(
    status: SqliteCliActivationStatus,
    marker: &StorageAuthorityMarker,
    integrity_check: String,
    activated_at_utc: String,
) -> SqliteCliActivationReport {
    SqliteCliActivationReport {
        schema_version: ACTIVATION_REPORT_SCHEMA_VERSION,
        status,
        active_authority: marker.active_authority.clone(),
        database_path: marker.sqlite_candidate.database_path.clone(),
        source_fingerprint: marker.sqlite_candidate.source_fingerprint.clone(),
        integrity_check,
        activated_at_utc,
    }
}

fn validate_marker(marker: &StorageAuthorityMarker) -> Result<(), AuthorityError> {
    if marker.schema_version != MARKER_SCHEMA_VERSION {
        return Err(AuthorityError::InvalidMarker(format!(
            "unsupported schema version {}",
            marker.schema_version
        )));
    }
    if marker.sqlite_candidate.status != VERIFIED_CANDIDATE {
        return Err(AuthorityError::CandidateNotVerified);
    }
    if marker.sqlite_candidate.source_fingerprint.trim().is_empty()
        || marker.sqlite_candidate.database_path.trim().is_empty()
        || marker.sqlite_candidate.report_path.trim().is_empty()
    {
        return Err(AuthorityError::InvalidMarker(
            "verified candidate provenance is incomplete".to_string(),
        ));
    }
    Ok(())
}

fn read_marker(path: &Path) -> Result<StorageAuthorityMarker, AuthorityError> {
    let bytes = fs::read(path).map_err(|error| AuthorityError::Io(error.to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| AuthorityError::InvalidMarker(error.to_string()))
}

fn write_marker(path: &Path, marker: &StorageAuthorityMarker) -> Result<(), AuthorityError> {
    storage::write_json_atomic(path, marker).map_err(AuthorityError::Io)
}

fn read_migration_report(
    marker: &StorageAuthorityMarker,
) -> Result<ControlledMigrationReport, AuthorityError> {
    let path = PathBuf::from(&marker.sqlite_candidate.report_path);
    let bytes = fs::read(&path).map_err(|error| AuthorityError::Io(error.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|error| {
        AuthorityError::InvalidMarker(format!("migration report is invalid: {error}"))
    })
}

fn open_repository_for_activation(path: &Path) -> Result<SqliteRepository, AuthorityError> {
    SqliteRepository::open(path).map_err(|error| AuthorityError::Verification(error.to_string()))
}

fn normalize_existing_path(path: &Path) -> Result<PathBuf, AuthorityError> {
    fs::canonicalize(path).map_err(|error| AuthorityError::Io(error.to_string()))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
