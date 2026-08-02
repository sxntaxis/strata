use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use chrono::{
    DateTime, Duration as ChronoDuration, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime,
    SecondsFormat, TimeZone, Utc,
};
use csv::{ReaderBuilder, StringRecord};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{constants::COLORS, domain::DRIFT_CATEGORY_CONFIG_NAME};

use super::SqliteRepository;

const CATEGORIES_HEADER: [&str; 5] = ["id", "name", "description", "color_index", "karma_effect"];
const LEGACY_SESSIONS_HEADER: [&str; 8] = [
    "id",
    "date",
    "category_id",
    "category_name",
    "description",
    "start_time",
    "end_time",
    "elapsed_seconds",
];
const SESSIONS_HEADER: [&str; 12] = [
    "id",
    "date",
    "category_id",
    "category_name",
    "description",
    "start_time",
    "end_time",
    "elapsed_seconds",
    "started_at_utc",
    "ended_at_utc",
    "boundary_utc_offset_seconds",
    "boundary_start_minutes",
];

#[derive(Debug, Clone)]
pub(super) struct LegacyImportPaths {
    pub categories_csv: PathBuf,
    pub sessions_csv: PathBuf,
    pub active_session_json: PathBuf,
    pub detached_runtime_json: PathBuf,
    pub sand_state_json: PathBuf,
    pub category_tags_json: PathBuf,
    pub sand_history_dir: PathBuf,
}

impl LegacyImportPaths {
    pub fn from_roots(data_dir: &Path, state_dir: &Path, sessions_csv: PathBuf) -> Self {
        Self {
            categories_csv: data_dir.join("categories.csv"),
            sessions_csv,
            active_session_json: state_dir.join("active_session.json"),
            detached_runtime_json: state_dir.join("detached_runtime.json"),
            sand_state_json: state_dir.join("sand_state.json"),
            category_tags_json: state_dir.join("category_tags.json"),
            sand_history_dir: state_dir.join("sand_history"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LegacyImportOptions {
    pub utc_offset_seconds: i32,
    pub operational_day_start_minutes: u16,
    pub quantum_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct LegacyImportSummary {
    pub source_fingerprint: String,
    pub category_count: i64,
    pub session_count: i64,
    pub total_elapsed_seconds: i64,
    pub category_totals: BTreeMap<i64, i64>,
    pub active_session_present: bool,
    pub checkpoint_present: bool,
    pub sand_state_present: bool,
    pub snapshot_count: i64,
    pub tag_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LegacyImportOutcome {
    Imported(LegacyImportSummary),
    AlreadyImported(LegacyImportSummary),
}

#[derive(Debug, Error)]
pub(super) enum LegacyImportError {
    #[error("I/O error while reading {path}: {message}")]
    Io { path: String, message: String },
    #[error("CSV error in {path}: {message}")]
    Csv { path: String, message: String },
    #[error("JSON error in {path}: {message}")]
    Json { path: String, message: String },
    #[error("invalid legacy source {path}: {message}")]
    InvalidSource {
        path: String,
        row: Option<usize>,
        message: String,
    },
    #[error("legacy source conflict: {0}")]
    SourceConflict(String),
    #[error("no legacy authority-bearing files were found")]
    NoLegacyData,
    #[error("invalid legacy import options: {0}")]
    InvalidOptions(String),
    #[error("SQLite candidate database is not empty")]
    DatabaseNotEmpty,
    #[error("legacy import verification failed: {0}")]
    VerificationMismatch(String),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Debug, Clone, Serialize)]
struct SourceManifest {
    entries: Vec<SourceManifestEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct SourceManifestEntry {
    logical_name: String,
    path: String,
    exists: bool,
    byte_count: usize,
    content_fingerprint: Option<String>,
}

#[derive(Debug)]
struct SourceCollection {
    manifest: SourceManifest,
    bytes: BTreeMap<String, Vec<u8>>,
    fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyCategory {
    id: i64,
    name: String,
    description: String,
    color_index: i64,
    balance_effect: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacySession {
    id: i64,
    stable_id: String,
    category_id: i64,
    description: String,
    started_at_utc: String,
    ended_at_utc: String,
    operational_day: String,
    elapsed_seconds: i64,
    boundary_utc_offset_seconds: i32,
    boundary_start_minutes: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyActiveSession {
    stable_id: String,
    project: String,
    category_id: i64,
    description: String,
    started_at_utc: String,
    recovery_kind: String,
}

#[derive(Debug, Clone)]
struct LegacyCheckpoint {
    detached_at_utc: String,
    simulation_time_utc: String,
    active_session_stable_id: Option<String>,
    payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyTag {
    category_id: i64,
    ordinal: i64,
    tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacySnapshot {
    operational_day: String,
    captured_at_utc: String,
    payload_json: String,
}

#[derive(Debug, Clone)]
pub(super) struct LegacyImportPlan {
    options: LegacyImportOptions,
    source_manifest_json: String,
    source_fingerprint: String,
    categories: Vec<LegacyCategory>,
    sessions: Vec<LegacySession>,
    active_session: Option<LegacyActiveSession>,
    checkpoint: Option<LegacyCheckpoint>,
    sand_state: Option<StrictSandState>,
    snapshots: Vec<LegacySnapshot>,
    tags: Vec<LegacyTag>,
    summary: LegacyImportSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StrictSandStateGrain {
    x: usize,
    y: usize,
    category_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StrictSandState {
    version: u8,
    grid_width: usize,
    grid_height: usize,
    grains: Vec<StrictSandStateGrain>,
    #[serde(default)]
    frame_count: usize,
    #[serde(default = "default_sweep_left_to_right")]
    sweep_left_to_right: bool,
    #[serde(default = "default_rng_state")]
    rng_state: u64,
}

fn default_sweep_left_to_right() -> bool {
    true
}

fn default_rng_state() -> u64 {
    0x9E37_79B9_7F4A_7C15
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveSessionJson {
    project: String,
    description: String,
    category_id: u64,
    category_name: String,
    start_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CategoryTagsJson {
    version: u8,
    tags_by_category: BTreeMap<u64, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum QueuedMutationRecord {
    SwitchLayer { category_id: u64 },
    ClearAllSand,
    ClearDriftSand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueuedMutationEventRecord {
    execute_at_utc: DateTime<Utc>,
    mutation: QueuedMutationRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DetachedRuntimeCheckpointJson {
    schema_version: u8,
    detached_at_utc: DateTime<Utc>,
    simulation_time_utc: DateTime<Utc>,
    spawn_accumulator_nanos: u64,
    physics_accumulator_nanos: u64,
    active_category_id: u64,
    active_description: String,
    active_session_started_at_utc: Option<DateTime<Utc>>,
    sand_state: StrictSandState,
    pending_mutations: Vec<QueuedMutationEventRecord>,
}

impl LegacyImportPlan {
    pub fn from_paths(
        paths: &LegacyImportPaths,
        options: LegacyImportOptions,
    ) -> Result<Self, LegacyImportError> {
        validate_options(options)?;
        let sources = collect_sources(paths)?;
        if sources.bytes.is_empty() {
            return Err(LegacyImportError::NoLegacyData);
        }

        let categories = match sources.bytes.get("categories.csv") {
            Some(bytes) => parse_categories(bytes)?,
            None => Vec::new(),
        };
        let category_names = category_names(&categories);
        let category_ids: BTreeSet<i64> = category_names.keys().copied().collect();

        let sessions = match sources.bytes.get("time_log.csv") {
            Some(bytes) => parse_sessions(bytes, &category_names, options, &sources.fingerprint)?,
            None => Vec::new(),
        };

        let active_from_json = match sources.bytes.get("active_session.json") {
            Some(bytes) => Some(parse_active_session(
                bytes,
                &category_names,
                &sources.fingerprint,
            )?),
            None => None,
        };

        let detached = match sources.bytes.get("detached_runtime.json") {
            Some(bytes) => Some(parse_detached_checkpoint(
                bytes,
                &category_ids,
                &sources.fingerprint,
            )?),
            None => None,
        };

        let detached_active = detached
            .as_ref()
            .and_then(|parsed| parsed.active_session.clone());
        if active_from_json.is_some() && detached_active.is_some() {
            return Err(LegacyImportError::SourceConflict(
                "active_session.json and detached_runtime.json both contain active intervals"
                    .to_string(),
            ));
        }
        let active_session = active_from_json.or(detached_active);
        let checkpoint = detached.as_ref().map(|parsed| parsed.checkpoint.clone());

        let standalone_sand = match sources.bytes.get("sand_state.json") {
            Some(bytes) => Some(parse_sand_state("sand_state.json", bytes, &category_ids)?),
            None => None,
        };
        let detached_sand = detached.as_ref().map(|parsed| parsed.sand_state.clone());
        if let (Some(standalone), Some(detached)) = (&standalone_sand, &detached_sand)
            && standalone != detached
        {
            return Err(LegacyImportError::SourceConflict(
                "sand_state.json differs from the sand state inside detached_runtime.json"
                    .to_string(),
            ));
        }
        let sand_state = detached_sand.or(standalone_sand);

        let snapshots = parse_snapshots(&sources, &category_ids, options)?;
        let tags = match sources.bytes.get("category_tags.json") {
            Some(bytes) => parse_tags(bytes, &category_ids)?,
            None => Vec::new(),
        };

        let total_elapsed_seconds = sessions.iter().map(|session| session.elapsed_seconds).sum();
        let mut category_totals = BTreeMap::new();
        for session in &sessions {
            *category_totals.entry(session.category_id).or_insert(0) += session.elapsed_seconds;
        }

        let summary = LegacyImportSummary {
            source_fingerprint: sources.fingerprint.clone(),
            category_count: i64::try_from(categories.len()).map_err(|_| {
                invalid(
                    "categories.csv",
                    None,
                    "category count exceeds SQLite limits",
                )
            })?,
            session_count: i64::try_from(sessions.len()).map_err(|_| {
                invalid("time_log.csv", None, "session count exceeds SQLite limits")
            })?,
            total_elapsed_seconds,
            category_totals,
            active_session_present: active_session.is_some(),
            checkpoint_present: checkpoint.is_some(),
            sand_state_present: sand_state.is_some(),
            snapshot_count: i64::try_from(snapshots.len()).map_err(|_| {
                invalid("sand_history", None, "snapshot count exceeds SQLite limits")
            })?,
            tag_count: i64::try_from(tags.len()).map_err(|_| {
                invalid(
                    "category_tags.json",
                    None,
                    "tag count exceeds SQLite limits",
                )
            })?,
        };

        let source_manifest_json =
            serde_json::to_string(&sources.manifest).map_err(|error| LegacyImportError::Json {
                path: "source manifest".to_string(),
                message: error.to_string(),
            })?;

        Ok(Self {
            options,
            source_manifest_json,
            source_fingerprint: sources.fingerprint,
            categories,
            sessions,
            active_session,
            checkpoint,
            sand_state,
            snapshots,
            tags,
            summary,
        })
    }

    pub fn summary(&self) -> &LegacyImportSummary {
        &self.summary
    }
}

#[derive(Debug, Clone)]
struct ParsedDetachedCheckpoint {
    active_session: Option<LegacyActiveSession>,
    checkpoint: LegacyCheckpoint,
    sand_state: StrictSandState,
}

impl SqliteRepository {
    pub(super) fn import_legacy(
        &mut self,
        plan: &LegacyImportPlan,
    ) -> Result<LegacyImportOutcome, LegacyImportError> {
        if let Some(existing) = self.existing_import(&plan.source_fingerprint)? {
            return Ok(LegacyImportOutcome::AlreadyImported(existing));
        }

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_candidate_is_empty(&transaction)?;

        let started_at_utc = now_utc();
        transaction.execute(
            "INSERT INTO legacy_imports (
                source_fingerprint,
                status,
                source_manifest_json,
                utc_offset_seconds,
                operational_day_start_minutes,
                quantum_seconds,
                category_count,
                session_count,
                total_elapsed_seconds,
                active_session_present,
                checkpoint_present,
                sand_state_present,
                snapshot_count,
                started_at_utc
             ) VALUES (?1, 'pending', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                plan.source_fingerprint,
                plan.source_manifest_json,
                plan.options.utc_offset_seconds,
                i64::from(plan.options.operational_day_start_minutes),
                plan.options.quantum_seconds,
                plan.summary.category_count,
                plan.summary.session_count,
                plan.summary.total_elapsed_seconds,
                bool_i64(plan.summary.active_session_present),
                bool_i64(plan.summary.checkpoint_present),
                bool_i64(plan.summary.sand_state_present),
                plan.summary.snapshot_count,
                started_at_utc,
            ],
        )?;
        let import_id = transaction.last_insert_rowid();
        let stable_prefix = &plan.source_fingerprint[..16];
        let formation_id = format!("legacy-{stable_prefix}-formation");

        for category in &plan.categories {
            transaction.execute(
                "INSERT INTO categories (
                    id,
                    name,
                    description,
                    color_index,
                    balance_effect
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    category.id,
                    category.name,
                    category.description,
                    category.color_index,
                    category.balance_effect,
                ],
            )?;
        }

        for tag in &plan.tags {
            transaction.execute(
                "INSERT INTO category_tags (
                    category_id,
                    ordinal,
                    tag,
                    legacy_import_id
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![tag.category_id, tag.ordinal, tag.tag, import_id],
            )?;
        }

        for session in &plan.sessions {
            transaction.execute(
                "INSERT INTO sessions (
                    id,
                    stable_id,
                    project,
                    category_id,
                    description,
                    started_at_utc,
                    ended_at_utc,
                    operational_day,
                    elapsed_seconds,
                    boundary_utc_offset_seconds,
                    boundary_start_minutes,
                    source,
                    legacy_import_id
                 ) VALUES (?1, ?2, '', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'legacy-csv', ?11)",
                params![
                    session.id,
                    session.stable_id,
                    session.category_id,
                    session.description,
                    session.started_at_utc,
                    session.ended_at_utc,
                    session.operational_day,
                    session.elapsed_seconds,
                    session.boundary_utc_offset_seconds,
                    session.boundary_start_minutes,
                    import_id,
                ],
            )?;
        }

        if let Some(active) = &plan.active_session {
            transaction.execute(
                "INSERT INTO active_session (
                    singleton,
                    stable_id,
                    project,
                    category_id,
                    description,
                    started_at_utc,
                    recovery_kind,
                    legacy_import_id
                 ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    active.stable_id,
                    active.project,
                    active.category_id,
                    active.description,
                    active.started_at_utc,
                    active.recovery_kind,
                    import_id,
                ],
            )?;
        }

        if let Some(checkpoint) = &plan.checkpoint {
            transaction.execute(
                "INSERT INTO runtime_checkpoint (
                    singleton,
                    status,
                    detached_at_utc,
                    simulation_time_utc,
                    active_session_stable_id,
                    payload_json,
                    legacy_import_id
                 ) VALUES (1, 'pending', ?1, ?2, ?3, ?4, ?5)",
                params![
                    checkpoint.detached_at_utc,
                    checkpoint.simulation_time_utc,
                    checkpoint.active_session_stable_id,
                    checkpoint.payload_json,
                    import_id,
                ],
            )?;
        }

        if let Some(sand_state) = &plan.sand_state {
            let payload_json =
                serde_json::to_string(sand_state).map_err(|error| LegacyImportError::Json {
                    path: "sand state".to_string(),
                    message: error.to_string(),
                })?;
            let updated_at = plan
                .checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.simulation_time_utc.clone())
                .unwrap_or_else(now_utc);
            transaction.execute(
                "INSERT INTO sand_state (
                    singleton,
                    formation_id,
                    quantum_seconds,
                    grid_width,
                    grid_height,
                    payload_json,
                    updated_at_utc,
                    legacy_import_id
                 ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    formation_id,
                    plan.options.quantum_seconds,
                    usize_to_i64(sand_state.grid_width, "sand grid width")?,
                    usize_to_i64(sand_state.grid_height, "sand grid height")?,
                    payload_json,
                    updated_at,
                    import_id,
                ],
            )?;
        }

        for snapshot in &plan.snapshots {
            transaction.execute(
                "INSERT INTO sand_snapshots (
                    formation_id,
                    snapshot_kind,
                    operational_day,
                    quantum_seconds,
                    payload_json,
                    captured_at_utc,
                    legacy_import_id
                 ) VALUES (?1, 'daily', ?2, ?3, ?4, ?5, ?6)",
                params![
                    formation_id,
                    snapshot.operational_day,
                    plan.options.quantum_seconds,
                    snapshot.payload_json,
                    snapshot.captured_at_utc,
                    import_id,
                ],
            )?;
        }

        verify_import(&transaction, plan, import_id)?;
        let verification_json =
            serde_json::to_string(&plan.summary).map_err(|error| LegacyImportError::Json {
                path: "verification summary".to_string(),
                message: error.to_string(),
            })?;
        let completed_at_utc = now_utc();
        transaction.execute(
            "UPDATE legacy_imports
             SET status = 'verified', verification_json = ?1, completed_at_utc = ?2
             WHERE id = ?3",
            params![verification_json, completed_at_utc, import_id],
        )?;
        transaction.execute(
            "INSERT INTO database_metadata(key, value)
             VALUES ('legacy_import_fingerprint', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![plan.source_fingerprint],
        )?;
        transaction.execute(
            "INSERT INTO database_metadata(key, value)
             VALUES ('legacy_import_status', 'verified')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )?;
        transaction.commit()?;

        Ok(LegacyImportOutcome::Imported(plan.summary.clone()))
    }

    fn existing_import(
        &self,
        fingerprint: &str,
    ) -> Result<Option<LegacyImportSummary>, LegacyImportError> {
        let result = self
            .connection
            .query_row(
                "SELECT status, verification_json
                 FROM legacy_imports
                 WHERE source_fingerprint = ?1",
                params![fingerprint],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;

        let Some((status, verification_json)) = result else {
            return Ok(None);
        };
        if status != "verified" {
            return Err(LegacyImportError::VerificationMismatch(format!(
                "stored import {fingerprint} has non-terminal status {status}"
            )));
        }
        let json = verification_json.ok_or_else(|| {
            LegacyImportError::VerificationMismatch(format!(
                "stored import {fingerprint} has no verification summary"
            ))
        })?;
        let summary = serde_json::from_str(&json).map_err(|error| LegacyImportError::Json {
            path: "stored verification summary".to_string(),
            message: error.to_string(),
        })?;
        Ok(Some(summary))
    }
}

fn validate_options(options: LegacyImportOptions) -> Result<(), LegacyImportError> {
    if FixedOffset::east_opt(options.utc_offset_seconds).is_none() {
        return Err(LegacyImportError::InvalidOptions(format!(
            "UTC offset {} is outside chrono's supported range",
            options.utc_offset_seconds
        )));
    }
    if options.operational_day_start_minutes >= 24 * 60 {
        return Err(LegacyImportError::InvalidOptions(format!(
            "operational-day start {} is outside 00:00-23:59",
            options.operational_day_start_minutes
        )));
    }
    if options.quantum_seconds <= 0 {
        return Err(LegacyImportError::InvalidOptions(
            "sand quantum must be positive".to_string(),
        ));
    }
    Ok(())
}

fn collect_sources(paths: &LegacyImportPaths) -> Result<SourceCollection, LegacyImportError> {
    let mut manifest_entries = Vec::new();
    let mut bytes = BTreeMap::new();
    collect_file(
        "categories.csv",
        &paths.categories_csv,
        &mut manifest_entries,
        &mut bytes,
    )?;
    collect_file(
        "time_log.csv",
        &paths.sessions_csv,
        &mut manifest_entries,
        &mut bytes,
    )?;
    collect_file(
        "active_session.json",
        &paths.active_session_json,
        &mut manifest_entries,
        &mut bytes,
    )?;
    collect_file(
        "detached_runtime.json",
        &paths.detached_runtime_json,
        &mut manifest_entries,
        &mut bytes,
    )?;
    collect_file(
        "sand_state.json",
        &paths.sand_state_json,
        &mut manifest_entries,
        &mut bytes,
    )?;
    collect_file(
        "category_tags.json",
        &paths.category_tags_json,
        &mut manifest_entries,
        &mut bytes,
    )?;

    if paths.sand_history_dir.exists() {
        let mut history_paths = fs::read_dir(&paths.sand_history_dir)
            .map_err(|error| io_error("sand_history", error))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| io_error("sand_history", error))?;
        history_paths.sort();
        for path in history_paths {
            if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("json")
            {
                continue;
            }
            let filename = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| invalid("sand_history", None, "snapshot filename is not UTF-8"))?;
            let logical_name = format!("sand_history/{filename}");
            collect_file(&logical_name, &path, &mut manifest_entries, &mut bytes)?;
        }
    }

    manifest_entries.sort_by(|left, right| left.logical_name.cmp(&right.logical_name));
    let mut fingerprint_state = Fnv64::new();
    for entry in &manifest_entries {
        fingerprint_state.write(entry.logical_name.as_bytes());
        fingerprint_state.write(&[u8::from(entry.exists)]);
        if let Some(content) = bytes.get(&entry.logical_name) {
            fingerprint_state.write(&(content.len() as u64).to_le_bytes());
            fingerprint_state.write(content);
        }
    }

    Ok(SourceCollection {
        manifest: SourceManifest {
            entries: manifest_entries,
        },
        bytes,
        fingerprint: fingerprint_state.finish_hex(),
    })
}

fn collect_file(
    logical_name: &str,
    path: &Path,
    manifest_entries: &mut Vec<SourceManifestEntry>,
    bytes: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), LegacyImportError> {
    if path.exists() {
        let content = fs::read(path).map_err(|error| io_error(logical_name, error))?;
        manifest_entries.push(SourceManifestEntry {
            logical_name: logical_name.to_string(),
            path: path.display().to_string(),
            exists: true,
            byte_count: content.len(),
            content_fingerprint: Some(fingerprint(&content)),
        });
        bytes.insert(logical_name.to_string(), content);
    } else {
        manifest_entries.push(SourceManifestEntry {
            logical_name: logical_name.to_string(),
            path: path.display().to_string(),
            exists: false,
            byte_count: 0,
            content_fingerprint: None,
        });
    }
    Ok(())
}

fn parse_categories(bytes: &[u8]) -> Result<Vec<LegacyCategory>, LegacyImportError> {
    let source = "categories.csv";
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(Cursor::new(bytes));
    validate_header(
        source,
        reader.headers().map_err(|error| csv_error(source, error))?,
        &CATEGORIES_HEADER,
    )?;

    let mut categories = Vec::new();
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for (index, record) in reader.records().enumerate() {
        let row = index + 2;
        let record = record.map_err(|error| csv_error(source, error))?;
        let id = parse_i64(source, row, &record, 0, "category ID")?;
        if id <= 0 {
            return Err(invalid(
                source,
                Some(row),
                "category ID must be greater than zero",
            ));
        }
        if !ids.insert(id) {
            return Err(invalid(
                source,
                Some(row),
                format!("duplicate category ID {id}"),
            ));
        }

        let name = required_field(source, row, &record, 1, "category name")?
            .trim()
            .to_string();
        if name.eq_ignore_ascii_case(DRIFT_CATEGORY_CONFIG_NAME)
            || name.eq_ignore_ascii_case("idle")
        {
            return Err(invalid(
                source,
                Some(row),
                format!("category name '{name}' is reserved for idle time"),
            ));
        }
        let normalized_name = name.to_lowercase();
        if !names.insert(normalized_name) {
            return Err(invalid(
                source,
                Some(row),
                format!("duplicate category name '{name}'"),
            ));
        }

        let color_index = parse_i64(source, row, &record, 3, "color index")?;
        if color_index < 0 || color_index >= COLORS.len() as i64 {
            return Err(invalid(
                source,
                Some(row),
                format!(
                    "color index {color_index} is outside 0..{}",
                    COLORS.len() - 1
                ),
            ));
        }
        let balance_effect = parse_i64(source, row, &record, 4, "karma effect")?;
        if !(-1..=1).contains(&balance_effect) {
            return Err(invalid(
                source,
                Some(row),
                format!("karma effect {balance_effect} is outside -1..1"),
            ));
        }

        categories.push(LegacyCategory {
            id,
            name,
            description: record.get(2).unwrap_or_default().to_string(),
            color_index,
            balance_effect,
        });
    }
    categories.sort_by_key(|category| category.id);
    Ok(categories)
}

fn category_names(categories: &[LegacyCategory]) -> BTreeMap<i64, String> {
    let mut names = BTreeMap::new();
    names.insert(0, DRIFT_CATEGORY_CONFIG_NAME.to_string());
    for category in categories {
        names.insert(category.id, category.name.clone());
    }
    names
}

fn parse_sessions(
    bytes: &[u8],
    category_names: &BTreeMap<i64, String>,
    options: LegacyImportOptions,
    fingerprint: &str,
) -> Result<Vec<LegacySession>, LegacyImportError> {
    let source = "time_log.csv";
    let default_offset = FixedOffset::east_opt(options.utc_offset_seconds)
        .ok_or_else(|| LegacyImportError::InvalidOptions("invalid fixed UTC offset".to_string()))?;
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(Cursor::new(bytes));
    let header = reader
        .headers()
        .map_err(|error| csv_error(source, error))?
        .clone();
    let has_temporal_provenance = header.iter().eq(SESSIONS_HEADER.iter().copied());
    if !has_temporal_provenance && !header.iter().eq(LEGACY_SESSIONS_HEADER.iter().copied()) {
        return Err(invalid(
            source,
            None,
            format!(
                "invalid header; expected '{}' or '{}'",
                LEGACY_SESSIONS_HEADER.join(","),
                SESSIONS_HEADER.join(",")
            ),
        ));
    }

    let mut sessions = Vec::new();
    let mut ids = HashSet::new();
    let stable_prefix = &fingerprint[..16];
    for (index, record) in reader.records().enumerate() {
        let row = index + 2;
        let record = record.map_err(|error| csv_error(source, error))?;
        let id = parse_i64(source, row, &record, 0, "session ID")?;
        if id <= 0 {
            return Err(invalid(
                source,
                Some(row),
                "session ID must be greater than zero",
            ));
        }
        if !ids.insert(id) {
            return Err(invalid(
                source,
                Some(row),
                format!("duplicate session ID {id}"),
            ));
        }

        let operational_day_raw = required_field(source, row, &record, 1, "operational day")?;
        let operational_day =
            NaiveDate::parse_from_str(operational_day_raw, "%Y-%m-%d").map_err(|error| {
                invalid(
                    source,
                    Some(row),
                    format!("invalid operational day: {error}"),
                )
            })?;
        let category_id = parse_i64(source, row, &record, 2, "category ID")?;
        let expected_category_name = category_names.get(&category_id).ok_or_else(|| {
            invalid(
                source,
                Some(row),
                format!("unknown category ID {category_id}"),
            )
        })?;
        let category_name = required_field(source, row, &record, 3, "category name")?;
        if !category_name.eq_ignore_ascii_case(expected_category_name) {
            return Err(invalid(
                source,
                Some(row),
                format!(
                    "category name '{category_name}' does not match ID {category_id} ('{expected_category_name}')"
                ),
            ));
        }

        let start_time = parse_time(source, row, &record, 5, "start time")?;
        let end_time = parse_time(source, row, &record, 6, "end time")?;
        let elapsed_seconds = parse_i64(source, row, &record, 7, "elapsed seconds")?;
        if elapsed_seconds < 0 {
            return Err(invalid(
                source,
                Some(row),
                "elapsed seconds cannot be negative",
            ));
        }

        let (started_at_utc, ended_at_utc, boundary_utc_offset_seconds, boundary_start_minutes) =
            if has_temporal_provenance {
                let started = DateTime::parse_from_rfc3339(required_field(
                    source,
                    row,
                    &record,
                    8,
                    "absolute start timestamp",
                )?)
                .map_err(|error| {
                    invalid(
                        source,
                        Some(row),
                        format!("invalid start timestamp: {error}"),
                    )
                })?
                .with_timezone(&Utc);
                let ended = DateTime::parse_from_rfc3339(required_field(
                    source,
                    row,
                    &record,
                    9,
                    "absolute end timestamp",
                )?)
                .map_err(|error| {
                    invalid(source, Some(row), format!("invalid end timestamp: {error}"))
                })?
                .with_timezone(&Utc);
                let offset_seconds = parse_i64(source, row, &record, 10, "boundary UTC offset")?;
                let offset_seconds = i32::try_from(offset_seconds)
                    .map_err(|_| invalid(source, Some(row), "boundary UTC offset exceeds i32"))?;
                if FixedOffset::east_opt(offset_seconds).is_none() {
                    return Err(invalid(
                        source,
                        Some(row),
                        "boundary UTC offset is unsupported",
                    ));
                }
                let start_minutes = parse_i64(source, row, &record, 11, "boundary start minutes")?;
                let start_minutes = u16::try_from(start_minutes).map_err(|_| {
                    invalid(source, Some(row), "boundary start minutes exceeds u16")
                })?;
                if start_minutes > 1439 {
                    return Err(invalid(
                        source,
                        Some(row),
                        "boundary start minutes is outside 0..1439",
                    ));
                }
                if (ended - started).num_seconds() != elapsed_seconds {
                    return Err(invalid(
                        source,
                        Some(row),
                        "absolute timestamps do not match elapsed_seconds",
                    ));
                }
                (
                    format_utc(started),
                    format_utc(ended),
                    offset_seconds,
                    start_minutes,
                )
            } else {
                let (started, ended) = reconstruct_absolute_times(
                    operational_day,
                    start_time,
                    end_time,
                    elapsed_seconds,
                    options.operational_day_start_minutes,
                    default_offset,
                )
                .map_err(|message| invalid(source, Some(row), message))?;
                (
                    started,
                    ended,
                    options.utc_offset_seconds,
                    options.operational_day_start_minutes,
                )
            };

        sessions.push(LegacySession {
            id,
            stable_id: format!("legacy-{stable_prefix}-session-{id}"),
            category_id,
            description: record.get(4).unwrap_or_default().to_string(),
            started_at_utc,
            ended_at_utc,
            operational_day: operational_day.format("%Y-%m-%d").to_string(),
            elapsed_seconds,
            boundary_utc_offset_seconds,
            boundary_start_minutes,
        });
    }
    sessions.sort_by_key(|session| session.id);
    Ok(sessions)
}

fn parse_active_session(
    bytes: &[u8],
    category_names: &BTreeMap<i64, String>,
    fingerprint: &str,
) -> Result<LegacyActiveSession, LegacyImportError> {
    let source = "active_session.json";
    let active: ActiveSessionJson =
        serde_json::from_slice(bytes).map_err(|error| json_error(source, error))?;
    let category_id = i64::try_from(active.category_id)
        .map_err(|_| invalid(source, None, "category ID exceeds SQLite integer range"))?;
    let expected_name = category_names
        .get(&category_id)
        .ok_or_else(|| invalid(source, None, format!("unknown category ID {category_id}")))?;
    if !active.category_name.eq_ignore_ascii_case(expected_name) {
        return Err(invalid(
            source,
            None,
            format!(
                "category name '{}' does not match ID {} ('{}')",
                active.category_name, category_id, expected_name
            ),
        ));
    }

    Ok(LegacyActiveSession {
        stable_id: format!("legacy-{}-active", &fingerprint[..16]),
        project: active.project,
        category_id,
        description: active.description,
        started_at_utc: format_utc(active.start_time),
        recovery_kind: "live".to_string(),
    })
}

fn parse_detached_checkpoint(
    bytes: &[u8],
    category_ids: &BTreeSet<i64>,
    fingerprint: &str,
) -> Result<ParsedDetachedCheckpoint, LegacyImportError> {
    let source = "detached_runtime.json";
    let checkpoint: DetachedRuntimeCheckpointJson =
        serde_json::from_slice(bytes).map_err(|error| json_error(source, error))?;
    if checkpoint.schema_version != 1 {
        return Err(invalid(
            source,
            None,
            format!(
                "unsupported checkpoint schema version {}",
                checkpoint.schema_version
            ),
        ));
    }
    let active_category_id = i64::try_from(checkpoint.active_category_id).map_err(|_| {
        invalid(
            source,
            None,
            "active category ID exceeds SQLite integer range",
        )
    })?;
    if !category_ids.contains(&active_category_id) {
        return Err(invalid(
            source,
            None,
            format!("unknown active category ID {active_category_id}"),
        ));
    }
    for mutation in &checkpoint.pending_mutations {
        if let QueuedMutationRecord::SwitchLayer { category_id } = &mutation.mutation {
            let category_id = i64::try_from(*category_id).map_err(|_| {
                invalid(
                    source,
                    None,
                    "queued category ID exceeds SQLite integer range",
                )
            })?;
            if !category_ids.contains(&category_id) {
                return Err(invalid(
                    source,
                    None,
                    format!("queued mutation references unknown category ID {category_id}"),
                ));
            }
        }
    }
    validate_sand_state(source, &checkpoint.sand_state, category_ids)?;

    let stable_id = format!("legacy-{}-active", &fingerprint[..16]);
    let active_session = checkpoint
        .active_session_started_at_utc
        .as_ref()
        .map(|started_at| LegacyActiveSession {
            stable_id: stable_id.clone(),
            project: String::new(),
            category_id: active_category_id,
            description: checkpoint.active_description.clone(),
            started_at_utc: format_utc(*started_at),
            recovery_kind: "detached".to_string(),
        });
    let payload_json = String::from_utf8(bytes.to_vec())
        .map_err(|error| invalid(source, None, format!("checkpoint is not UTF-8: {error}")))?;

    Ok(ParsedDetachedCheckpoint {
        active_session,
        checkpoint: LegacyCheckpoint {
            detached_at_utc: format_utc(checkpoint.detached_at_utc),
            simulation_time_utc: format_utc(checkpoint.simulation_time_utc),
            active_session_stable_id: checkpoint
                .active_session_started_at_utc
                .as_ref()
                .map(|_| stable_id),
            payload_json,
        },
        sand_state: checkpoint.sand_state,
    })
}

fn parse_sand_state(
    source: &str,
    bytes: &[u8],
    category_ids: &BTreeSet<i64>,
) -> Result<StrictSandState, LegacyImportError> {
    let state: StrictSandState =
        serde_json::from_slice(bytes).map_err(|error| json_error(source, error))?;
    validate_sand_state(source, &state, category_ids)?;
    Ok(state)
}

fn validate_sand_state(
    source: &str,
    state: &StrictSandState,
    category_ids: &BTreeSet<i64>,
) -> Result<(), LegacyImportError> {
    if state.version != 1 {
        return Err(invalid(
            source,
            None,
            format!("unsupported sand-state version {}", state.version),
        ));
    }
    state
        .grid_width
        .checked_mul(state.grid_height)
        .ok_or_else(|| {
            invalid(
                source,
                None,
                "sand-state dimensions overflow addressable space",
            )
        })?;
    let mut coordinates = HashSet::new();
    for grain in &state.grains {
        if grain.x >= state.grid_width || grain.y >= state.grid_height {
            return Err(invalid(
                source,
                None,
                format!(
                    "grain ({}, {}) is outside {}x{} grid",
                    grain.x, grain.y, state.grid_width, state.grid_height
                ),
            ));
        }
        let category_id = i64::try_from(grain.category_id).map_err(|_| {
            invalid(
                source,
                None,
                "grain category ID exceeds SQLite integer range",
            )
        })?;
        if !category_ids.contains(&category_id) {
            return Err(invalid(
                source,
                None,
                format!("grain references unknown category ID {category_id}"),
            ));
        }
        if !coordinates.insert((grain.x, grain.y)) {
            return Err(invalid(
                source,
                None,
                format!("duplicate grain coordinate ({}, {})", grain.x, grain.y),
            ));
        }
    }
    Ok(())
}

fn parse_snapshots(
    sources: &SourceCollection,
    category_ids: &BTreeSet<i64>,
    options: LegacyImportOptions,
) -> Result<Vec<LegacySnapshot>, LegacyImportError> {
    let offset = FixedOffset::east_opt(options.utc_offset_seconds)
        .ok_or_else(|| LegacyImportError::InvalidOptions("invalid fixed UTC offset".to_string()))?;
    let mut snapshots = Vec::new();
    for (logical_name, bytes) in &sources.bytes {
        let Some(filename) = logical_name.strip_prefix("sand_history/") else {
            continue;
        };
        let Some(day_raw) = filename.strip_suffix(".json") else {
            continue;
        };
        let operational_day = NaiveDate::parse_from_str(day_raw, "%Y-%m-%d").map_err(|error| {
            invalid(
                logical_name,
                None,
                format!("snapshot filename is not an operational day: {error}"),
            )
        })?;
        let state = parse_sand_state(logical_name, bytes, category_ids)?;
        let payload_json =
            serde_json::to_string(&state).map_err(|error| LegacyImportError::Json {
                path: logical_name.clone(),
                message: error.to_string(),
            })?;
        snapshots.push(LegacySnapshot {
            operational_day: day_raw.to_string(),
            captured_at_utc: operational_day_end_boundary(
                operational_day,
                options.operational_day_start_minutes,
                offset,
            )?,
            payload_json,
        });
    }
    snapshots.sort_by(|left, right| left.operational_day.cmp(&right.operational_day));
    Ok(snapshots)
}

fn parse_tags(
    bytes: &[u8],
    category_ids: &BTreeSet<i64>,
) -> Result<Vec<LegacyTag>, LegacyImportError> {
    let source = "category_tags.json";
    let tags: CategoryTagsJson =
        serde_json::from_slice(bytes).map_err(|error| json_error(source, error))?;
    if tags.version != 1 {
        return Err(invalid(
            source,
            None,
            format!("unsupported category-tags version {}", tags.version),
        ));
    }

    let mut parsed = Vec::new();
    for (category_id, category_tags) in tags.tags_by_category {
        let category_id = i64::try_from(category_id)
            .map_err(|_| invalid(source, None, "tag category ID exceeds SQLite integer range"))?;
        if !category_ids.contains(&category_id) {
            return Err(invalid(
                source,
                None,
                format!("tags reference unknown category ID {category_id}"),
            ));
        }
        let mut seen = HashSet::new();
        for (ordinal, tag) in category_tags.into_iter().enumerate() {
            let trimmed = tag.trim();
            if trimmed.is_empty() {
                return Err(invalid(source, None, "empty category tag"));
            }
            if !seen.insert(trimmed.to_lowercase()) {
                return Err(invalid(
                    source,
                    None,
                    format!("duplicate tag '{trimmed}' for category {category_id}"),
                ));
            }
            parsed.push(LegacyTag {
                category_id,
                ordinal: i64::try_from(ordinal).map_err(|_| {
                    invalid(source, None, "tag ordinal exceeds SQLite integer range")
                })?,
                tag: trimmed.to_string(),
            });
        }
    }
    parsed.sort_by(|left, right| {
        (left.category_id, left.ordinal).cmp(&(right.category_id, right.ordinal))
    });
    Ok(parsed)
}

fn reconstruct_absolute_times(
    operational_day: NaiveDate,
    start_time: NaiveTime,
    end_time: NaiveTime,
    elapsed_seconds: i64,
    day_start_minutes: u16,
    offset: FixedOffset,
) -> Result<(String, String), String> {
    let cutoff =
        NaiveTime::from_num_seconds_from_midnight_opt(u32::from(day_start_minutes) * 60, 0)
            .ok_or_else(|| "invalid operational-day cutoff".to_string())?;
    let start_date = if start_time < cutoff {
        operational_day
            .checked_add_signed(ChronoDuration::days(1))
            .ok_or_else(|| "start date overflow".to_string())?
    } else {
        operational_day
    };
    let start_naive = NaiveDateTime::new(start_date, start_time);
    let end_naive = start_naive
        .checked_add_signed(ChronoDuration::seconds(elapsed_seconds))
        .ok_or_else(|| "end timestamp overflow".to_string())?;
    if end_naive.time() != end_time {
        return Err(format!(
            "elapsed seconds imply end time {}, but CSV records {}",
            end_naive.time().format("%H:%M:%S"),
            end_time.format("%H:%M:%S")
        ));
    }
    let started = offset
        .from_local_datetime(&start_naive)
        .single()
        .ok_or_else(|| "fixed-offset start timestamp is ambiguous".to_string())?
        .with_timezone(&Utc);
    let ended = offset
        .from_local_datetime(&end_naive)
        .single()
        .ok_or_else(|| "fixed-offset end timestamp is ambiguous".to_string())?
        .with_timezone(&Utc);
    Ok((format_utc(started), format_utc(ended)))
}

fn operational_day_end_boundary(
    operational_day: NaiveDate,
    day_start_minutes: u16,
    offset: FixedOffset,
) -> Result<String, LegacyImportError> {
    let next_day = operational_day
        .checked_add_signed(ChronoDuration::days(1))
        .ok_or_else(|| invalid("sand_history", None, "snapshot boundary date overflow"))?;
    let cutoff =
        NaiveTime::from_num_seconds_from_midnight_opt(u32::from(day_start_minutes) * 60, 0)
            .ok_or_else(|| invalid("sand_history", None, "invalid snapshot cutoff"))?;
    let local = NaiveDateTime::new(next_day, cutoff);
    let utc = offset
        .from_local_datetime(&local)
        .single()
        .ok_or_else(|| invalid("sand_history", None, "snapshot boundary is ambiguous"))?
        .with_timezone(&Utc);
    Ok(format_utc(utc))
}

fn ensure_candidate_is_empty(transaction: &Transaction<'_>) -> Result<(), LegacyImportError> {
    let occupied: i64 = transaction.query_row(
        "SELECT
            (SELECT count(*) FROM categories WHERE id <> 0) +
            (SELECT count(*) FROM sessions) +
            (SELECT count(*) FROM active_session) +
            (SELECT count(*) FROM runtime_checkpoint) +
            (SELECT count(*) FROM sand_state) +
            (SELECT count(*) FROM sand_snapshots) +
            (SELECT count(*) FROM category_tags) +
            (SELECT count(*) FROM legacy_imports)",
        [],
        |row| row.get(0),
    )?;
    if occupied != 0 {
        return Err(LegacyImportError::DatabaseNotEmpty);
    }
    Ok(())
}

fn verify_import(
    transaction: &Transaction<'_>,
    plan: &LegacyImportPlan,
    import_id: i64,
) -> Result<(), LegacyImportError> {
    let category_ids = query_i64_values(
        transaction,
        "SELECT id FROM categories WHERE id <> 0 ORDER BY id",
        [],
    )?;
    let expected_category_ids: Vec<i64> =
        plan.categories.iter().map(|category| category.id).collect();
    if category_ids != expected_category_ids {
        return Err(mismatch(
            "category IDs",
            &expected_category_ids,
            &category_ids,
        ));
    }

    let session_ids = query_i64_values(
        transaction,
        "SELECT id FROM sessions WHERE legacy_import_id = ?1 ORDER BY id",
        params![import_id],
    )?;
    let expected_session_ids: Vec<i64> = plan.sessions.iter().map(|session| session.id).collect();
    if session_ids != expected_session_ids {
        return Err(mismatch("session IDs", &expected_session_ids, &session_ids));
    }

    let (session_count, elapsed_total): (i64, i64) = transaction.query_row(
        "SELECT count(*), coalesce(sum(elapsed_seconds), 0)
         FROM sessions WHERE legacy_import_id = ?1",
        params![import_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if session_count != plan.summary.session_count {
        return Err(LegacyImportError::VerificationMismatch(format!(
            "expected {} sessions, found {session_count}",
            plan.summary.session_count
        )));
    }
    if elapsed_total != plan.summary.total_elapsed_seconds {
        return Err(LegacyImportError::VerificationMismatch(format!(
            "expected {} elapsed seconds, found {elapsed_total}",
            plan.summary.total_elapsed_seconds
        )));
    }

    let mut category_totals = BTreeMap::new();
    let mut statement = transaction.prepare(
        "SELECT category_id, sum(elapsed_seconds)
         FROM sessions
         WHERE legacy_import_id = ?1
         GROUP BY category_id
         ORDER BY category_id",
    )?;
    let rows = statement.query_map(params![import_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (category_id, elapsed) = row?;
        category_totals.insert(category_id, elapsed);
    }
    if category_totals != plan.summary.category_totals {
        return Err(mismatch(
            "per-category elapsed totals",
            &plan.summary.category_totals,
            &category_totals,
        ));
    }

    let active_count: i64 = transaction.query_row(
        "SELECT count(*) FROM active_session WHERE legacy_import_id = ?1",
        params![import_id],
        |row| row.get(0),
    )?;
    if active_count != bool_i64(plan.summary.active_session_present) {
        return Err(LegacyImportError::VerificationMismatch(
            "active-session presence differs".to_string(),
        ));
    }
    if let Some(active) = &plan.active_session {
        let actual: (String, String, i64, String, String, String) = transaction.query_row(
            "SELECT stable_id, project, category_id, description, started_at_utc, recovery_kind
             FROM active_session WHERE legacy_import_id = ?1",
            params![import_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )?;
        let expected = (
            active.stable_id.clone(),
            active.project.clone(),
            active.category_id,
            active.description.clone(),
            active.started_at_utc.clone(),
            active.recovery_kind.clone(),
        );
        if actual != expected {
            return Err(mismatch("active session", &expected, &actual));
        }
    }

    verify_presence(
        transaction,
        "runtime_checkpoint",
        import_id,
        plan.summary.checkpoint_present,
    )?;
    verify_presence(
        transaction,
        "sand_state",
        import_id,
        plan.summary.sand_state_present,
    )?;

    let snapshot_count: i64 = transaction.query_row(
        "SELECT count(*) FROM sand_snapshots WHERE legacy_import_id = ?1",
        params![import_id],
        |row| row.get(0),
    )?;
    if snapshot_count != plan.summary.snapshot_count {
        return Err(LegacyImportError::VerificationMismatch(format!(
            "expected {} sand snapshots, found {snapshot_count}",
            plan.summary.snapshot_count
        )));
    }
    let tag_count: i64 = transaction.query_row(
        "SELECT count(*) FROM category_tags WHERE legacy_import_id = ?1",
        params![import_id],
        |row| row.get(0),
    )?;
    if tag_count != plan.summary.tag_count {
        return Err(LegacyImportError::VerificationMismatch(format!(
            "expected {} category tags, found {tag_count}",
            plan.summary.tag_count
        )));
    }

    let foreign_key_violation: Option<String> = transaction
        .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
        .optional()?;
    if let Some(table) = foreign_key_violation {
        return Err(LegacyImportError::VerificationMismatch(format!(
            "foreign-key check failed in table {table}"
        )));
    }
    Ok(())
}

fn verify_presence(
    transaction: &Transaction<'_>,
    table: &str,
    import_id: i64,
    expected: bool,
) -> Result<(), LegacyImportError> {
    let sql = format!("SELECT count(*) FROM {table} WHERE legacy_import_id = ?1");
    let count: i64 = transaction.query_row(&sql, params![import_id], |row| row.get(0))?;
    if count != bool_i64(expected) {
        return Err(LegacyImportError::VerificationMismatch(format!(
            "{table} presence differs"
        )));
    }
    Ok(())
}

fn query_i64_values<P: rusqlite::Params>(
    transaction: &Transaction<'_>,
    sql: &str,
    params: P,
) -> Result<Vec<i64>, LegacyImportError> {
    let mut statement = transaction.prepare(sql)?;
    let rows = statement.query_map(params, |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(LegacyImportError::Sqlite)
}

fn validate_header(
    source: &str,
    actual: &StringRecord,
    expected: &[&str],
) -> Result<(), LegacyImportError> {
    let matches = actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(left, right)| left == *right);
    if matches {
        return Ok(());
    }
    Err(invalid(
        source,
        Some(1),
        format!(
            "expected header '{}', found '{}'",
            expected.join(","),
            actual.iter().collect::<Vec<_>>().join(",")
        ),
    ))
}

fn required_field<'a>(
    source: &str,
    row: usize,
    record: &'a StringRecord,
    index: usize,
    label: &str,
) -> Result<&'a str, LegacyImportError> {
    let value = record.get(index).unwrap_or_default();
    if value.trim().is_empty() {
        return Err(invalid(source, Some(row), format!("{label} is empty")));
    }
    Ok(value)
}

fn parse_i64(
    source: &str,
    row: usize,
    record: &StringRecord,
    index: usize,
    label: &str,
) -> Result<i64, LegacyImportError> {
    let raw = required_field(source, row, record, index, label)?;
    raw.parse::<i64>().map_err(|error| {
        invalid(
            source,
            Some(row),
            format!("invalid {label} '{raw}': {error}"),
        )
    })
}

fn parse_time(
    source: &str,
    row: usize,
    record: &StringRecord,
    index: usize,
    label: &str,
) -> Result<NaiveTime, LegacyImportError> {
    let raw = required_field(source, row, record, index, label)?;
    NaiveTime::parse_from_str(raw, "%H:%M:%S").map_err(|error| {
        invalid(
            source,
            Some(row),
            format!("invalid {label} '{raw}': {error}"),
        )
    })
}

fn format_utc(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn bool_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn usize_to_i64(value: usize, label: &str) -> Result<i64, LegacyImportError> {
    i64::try_from(value).map_err(|_| LegacyImportError::InvalidSource {
        path: "sand state".to_string(),
        row: None,
        message: format!("{label} exceeds SQLite integer range"),
    })
}

fn now_utc() -> String {
    format_utc(Utc::now())
}

fn mismatch<T: std::fmt::Debug>(label: &str, expected: &T, actual: &T) -> LegacyImportError {
    LegacyImportError::VerificationMismatch(format!(
        "{label}: expected {expected:?}, found {actual:?}"
    ))
}

fn invalid(
    source: impl Into<String>,
    row: Option<usize>,
    message: impl Into<String>,
) -> LegacyImportError {
    LegacyImportError::InvalidSource {
        path: source.into(),
        row,
        message: message.into(),
    }
}

fn io_error(source: &str, error: std::io::Error) -> LegacyImportError {
    LegacyImportError::Io {
        path: source.to_string(),
        message: error.to_string(),
    }
}

fn csv_error(source: &str, error: csv::Error) -> LegacyImportError {
    LegacyImportError::Csv {
        path: source.to_string(),
        message: error.to_string(),
    }
}

fn json_error(source: &str, error: serde_json::Error) -> LegacyImportError {
    LegacyImportError::Json {
        path: source.to_string(),
        message: error.to_string(),
    }
}

struct Fnv64(u64);

impl Fnv64 {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn finish_hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

fn fingerprint(bytes: &[u8]) -> String {
    let mut state = Fnv64::new();
    state.write(bytes);
    state.finish_hex()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use super::*;

    struct Fixture {
        root: PathBuf,
        data: PathBuf,
        state: PathBuf,
        paths: LegacyImportPaths,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "strata-sqlite-import-{name}-{}-{nonce}",
                std::process::id()
            ));
            let data = root.join("data");
            let state = root.join("state");
            fs::create_dir_all(&data).unwrap();
            fs::create_dir_all(state.join("sand_history")).unwrap();
            let paths = LegacyImportPaths::from_roots(&data, &state, data.join("time_log.csv"));
            Self {
                root,
                data,
                state,
                paths,
            }
        }

        fn write_valid_sources(&self) {
            fs::write(
                &self.paths.categories_csv,
                "id,name,description,color_index,karma_effect\n\
                 1,Work,Deep work,2,1\n\
                 2,Break,Rest,3,-1\n",
            )
            .unwrap();
            fs::write(
                &self.paths.sessions_csv,
                "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n\
                 7,2026-07-31,1,Work,Late work,23:30:00,00:30:00,3600\n\
                 8,2026-08-01,2,Break,Coffee,05:30:00,05:45:00,900\n",
            )
            .unwrap();
            fs::write(
                &self.paths.active_session_json,
                r#"{
                    "project": "Client A",
                    "description": "Current task",
                    "category_id": 1,
                    "category_name": "Work",
                    "start_time": "2026-08-01T15:00:00Z"
                }"#,
            )
            .unwrap();
            let sand = r#"{
                "version": 1,
                "grid_width": 4,
                "grid_height": 3,
                "grains": [
                    {"x": 1, "y": 2, "category_id": 1},
                    {"x": 2, "y": 2, "category_id": 0}
                ],
                "frame_count": 12,
                "sweep_left_to_right": true,
                "rng_state": 42
            }"#;
            fs::write(&self.paths.sand_state_json, sand).unwrap();
            fs::write(
                &self.paths.detached_runtime_json,
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
                        "pending_mutations": [
                            {{
                                "execute_at_utc": "2026-08-01T16:01:00Z",
                                "mutation": "ClearDriftSand"
                            }}
                        ]
                    }}"#
                ),
            )
            .unwrap();
            fs::write(
                &self.paths.category_tags_json,
                r#"{
                    "version": 1,
                    "tags_by_category": {
                        "1": ["focus", "billable"],
                        "2": ["rest"]
                    }
                }"#,
            )
            .unwrap();
            fs::write(self.paths.sand_history_dir.join("2026-07-31.json"), sand).unwrap();
        }

        fn source_bytes(&self) -> BTreeMap<PathBuf, Vec<u8>> {
            let mut result = BTreeMap::new();
            for path in [
                &self.paths.categories_csv,
                &self.paths.sessions_csv,
                &self.paths.active_session_json,
                &self.paths.detached_runtime_json,
                &self.paths.sand_state_json,
                &self.paths.category_tags_json,
            ] {
                if path.exists() {
                    result.insert(path.clone(), fs::read(path).unwrap());
                }
            }
            for entry in fs::read_dir(&self.paths.sand_history_dir).unwrap() {
                let path = entry.unwrap().path();
                result.insert(path.clone(), fs::read(path).unwrap());
            }
            result
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).ok();
        }
    }

    fn options() -> LegacyImportOptions {
        LegacyImportOptions {
            utc_offset_seconds: -6 * 3600,
            operational_day_start_minutes: 6 * 60,
            quantum_seconds: 1,
        }
    }

    #[test]
    fn strict_import_preserves_sources_and_verifies_every_state_family() {
        let fixture = Fixture::new("complete");
        fixture.write_valid_sources();
        let before = fixture.source_bytes();
        let plan = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap();
        assert_eq!(plan.summary().session_count, 2);
        assert_eq!(plan.summary().total_elapsed_seconds, 4500);

        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let outcome = repository.import_legacy(&plan).unwrap();
        assert_eq!(
            outcome,
            LegacyImportOutcome::Imported(plan.summary().clone())
        );
        assert_eq!(fixture.source_bytes(), before);

        let first: (String, String) = repository
            .connection
            .query_row(
                "SELECT started_at_utc, ended_at_utc FROM sessions WHERE id = 7",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(first.0, "2026-08-01T05:30:00Z");
        assert_eq!(first.1, "2026-08-01T06:30:00Z");
        let second_start: String = repository
            .connection
            .query_row(
                "SELECT started_at_utc FROM sessions WHERE id = 8",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(second_start, "2026-08-02T11:30:00Z");

        let repeat = repository.import_legacy(&plan).unwrap();
        assert_eq!(
            repeat,
            LegacyImportOutcome::AlreadyImported(plan.summary().clone())
        );
        assert_eq!(repository.completed_session_count().unwrap(), 2);
    }

    #[test]
    fn unknown_category_rejects_plan_before_database_mutation() {
        let fixture = Fixture::new("unknown-category");
        fs::write(
            &fixture.paths.categories_csv,
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .unwrap();
        fs::write(
            &fixture.paths.sessions_csv,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n\
             1,2026-08-01,999,Missing,,10:00:00,11:00:00,3600\n",
        )
        .unwrap();

        let error = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap_err();
        assert!(error.to_string().contains("unknown category ID 999"));
    }

    #[test]
    fn duplicate_session_identity_is_rejected() {
        let fixture = Fixture::new("duplicate-session");
        fs::write(
            &fixture.paths.categories_csv,
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .unwrap();
        fs::write(
            &fixture.paths.sessions_csv,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n\
             1,2026-08-01,1,Work,,10:00:00,11:00:00,3600\n\
             1,2026-08-01,1,Work,,12:00:00,13:00:00,3600\n",
        )
        .unwrap();

        let error = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap_err();
        assert!(error.to_string().contains("duplicate session ID 1"));
    }

    #[test]
    fn active_sources_must_not_be_ambiguous() {
        let fixture = Fixture::new("ambiguous-active");
        fixture.write_valid_sources();
        let detached = fs::read_to_string(&fixture.paths.detached_runtime_json)
            .unwrap()
            .replace(
                "\"active_session_started_at_utc\": null",
                "\"active_session_started_at_utc\": \"2026-08-01T15:30:00Z\"",
            )
            .replace("\"active_category_id\": 0", "\"active_category_id\": 1");
        fs::write(&fixture.paths.detached_runtime_json, detached).unwrap();

        let error = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap_err();
        assert!(error.to_string().contains("both contain active intervals"));
    }

    #[test]
    fn failed_database_insert_rolls_back_entire_import() {
        let fixture = Fixture::new("rollback");
        fixture.write_valid_sources();
        let plan = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap();
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        repository
            .connection
            .execute_batch(
                "CREATE TRIGGER reject_legacy_session
                 BEFORE INSERT ON sessions
                 WHEN NEW.source = 'legacy-csv'
                 BEGIN
                     SELECT RAISE(ABORT, 'fixture rejection');
                 END;",
            )
            .unwrap();

        let error = repository.import_legacy(&plan).unwrap_err();
        assert!(matches!(error, LegacyImportError::Sqlite(_)));
        let categories: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM categories WHERE id <> 0", [], |row| {
                row.get(0)
            })
            .unwrap();
        let imports: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM legacy_imports", [], |row| row.get(0))
            .unwrap();
        assert_eq!(categories, 0);
        assert_eq!(imports, 0);
        assert_eq!(repository.completed_session_count().unwrap(), 0);
    }

    #[test]
    fn elapsed_duration_must_match_recorded_end_clock() {
        let fixture = Fixture::new("duration-mismatch");
        fs::write(
            &fixture.paths.categories_csv,
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .unwrap();
        fs::write(
            &fixture.paths.sessions_csv,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n\
             1,2026-08-01,1,Work,,10:00:00,11:00:00,120\n",
        )
        .unwrap();

        let error = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap_err();
        assert!(error.to_string().contains("elapsed seconds imply end time"));
    }
}
