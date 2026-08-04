use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    category_lifecycle::{
        checkpoint_has_transition_receipt, count_checkpoint_category_references,
        count_sand_state_category, count_snapshot_category, reassign_checkpoint_category,
        reassign_sand_state_category, reassign_snapshot_category,
    },
    constants::COLORS,
    domain::{Category, CategoryId, OperationalDayPolicy, Session},
    sand::{
        DailySedimentSlice, SandState, SedimentSnapshot, SedimentSnapshotKind,
        daily_contribution_from_slices,
    },
    storage, temporal,
};

const PREPARED_VERSION: u8 = 1;
const LEDGER_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LegacyCategoryLifecyclePaths {
    pub categories_csv: PathBuf,
    pub sessions_csv: PathBuf,
    pub category_tags_json: PathBuf,
    pub sand_state_json: PathBuf,
    pub detached_runtime_json: PathBuf,
    pub sand_history_dir: PathBuf,
    pub prepared_json: PathBuf,
    pub ledger_json: PathBuf,
}

impl LegacyCategoryLifecyclePaths {
    pub(crate) fn runtime() -> Self {
        Self {
            categories_csv: storage::get_categories_path(),
            sessions_csv: storage::get_time_log_path(),
            category_tags_json: storage::get_category_tags_path(),
            sand_state_json: storage::get_sand_state_path(),
            detached_runtime_json: storage::get_detached_runtime_path(),
            sand_history_dir: storage::get_sand_history_dir(),
            prepared_json: storage::get_category_lifecycle_prepared_path(),
            ledger_json: storage::get_category_lifecycle_ledger_path(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyCategorySnapshot {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub color_index: usize,
    pub balance_effect: i8,
    pub archived: bool,
}

impl LegacyCategorySnapshot {
    fn from_category(category: &Category, archived: bool) -> Result<Self, String> {
        let color_index = COLORS
            .iter()
            .position(|color| *color == category.color)
            .ok_or_else(|| format!("category {} uses an unsupported color", category.id.0))?;
        Ok(Self {
            id: category.id.0,
            name: category.name.clone(),
            description: category.description.clone(),
            color_index,
            balance_effect: category.karma_effect,
            archived,
        })
    }

    fn to_category(&self) -> Result<Category, String> {
        let color = COLORS.get(self.color_index).copied().ok_or_else(|| {
            format!(
                "category {} has unsupported color index {}",
                self.id, self.color_index
            )
        })?;
        Ok(Category {
            id: CategoryId::new(self.id),
            name: self.name.clone(),
            color,
            description: self.description.clone(),
            karma_effect: self.balance_effect,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacySessionSnapshot {
    pub id: usize,
    pub date: String,
    pub category_id: u64,
    pub project: String,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: usize,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub operational_day_policy: Option<OperationalDayPolicySnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OperationalDayPolicySnapshot {
    pub utc_offset_seconds: i32,
    pub start_minutes: u16,
}

impl LegacySessionSnapshot {
    fn from_session(session: &Session) -> Self {
        Self {
            id: session.id,
            date: session.date.clone(),
            category_id: session.category_id.0,
            project: session.project.clone(),
            description: session.description.clone(),
            start_time: session.start_time.clone(),
            end_time: session.end_time.clone(),
            elapsed_seconds: session.elapsed_seconds,
            started_at_utc: session.started_at_utc,
            ended_at_utc: session.ended_at_utc,
            operational_day_policy: session.operational_day_policy.map(|policy| {
                OperationalDayPolicySnapshot {
                    utc_offset_seconds: policy.utc_offset_seconds,
                    start_minutes: policy.start_minutes,
                }
            }),
        }
    }

    fn to_session(&self) -> Session {
        Session {
            id: self.id,
            date: self.date.clone(),
            category_id: CategoryId::new(self.category_id),
            project: self.project.clone(),
            description: self.description.clone(),
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            elapsed_seconds: self.elapsed_seconds,
            started_at_utc: self.started_at_utc,
            ended_at_utc: self.ended_at_utc,
            operational_day_policy: self.operational_day_policy.map(|policy| {
                OperationalDayPolicy {
                    utc_offset_seconds: policy.utc_offset_seconds,
                    start_minutes: policy.start_minutes,
                }
            }),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyCategoryReferenceCounts {
    pub completed_sessions: u64,
    pub active_session: u64,
    pub tags: u64,
    pub sand_placed: u64,
    pub sand_pending: u64,
    pub history_placed: u64,
    pub history_pending: u64,
    pub checkpoint_references: u64,
}

impl LegacyCategoryReferenceCounts {
    pub(crate) fn total(&self) -> Result<u64, String> {
        [
            self.completed_sessions,
            self.active_session,
            self.tags,
            self.sand_placed,
            self.sand_pending,
            self.history_placed,
            self.history_pending,
            self.checkpoint_references,
        ]
        .into_iter()
        .try_fold(0u64, |total, value| {
            total
                .checked_add(value)
                .ok_or_else(|| "legacy category reference count exceeds u64".to_string())
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyCategoryLifecycleReview {
    pub source: LegacyCategorySnapshot,
    pub target: Option<LegacyCategorySnapshot>,
    pub references: LegacyCategoryReferenceCounts,
    pub checkpoint_custody: String,
    pub revision: String,
    pub confirmation_phrase: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyLifecycleReceipt {
    pub operation_id: String,
    pub operation_kind: String,
    pub source: LegacyCategorySnapshot,
    pub target: Option<LegacyCategorySnapshot>,
    pub preview_revision: String,
    pub references: LegacyCategoryReferenceCounts,
    pub applied_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyCategoryLifecycleLedger {
    pub version: u8,
    pub receipts: Vec<LegacyLifecycleReceipt>,
}

impl Default for LegacyCategoryLifecycleLedger {
    fn default() -> Self {
        Self {
            version: LEDGER_VERSION,
            receipts: Vec::new(),
        }
    }
}

impl LegacyCategoryLifecycleLedger {
    pub(crate) fn retired_ids(&self) -> BTreeSet<u64> {
        self.receipts
            .iter()
            .map(|receipt| receipt.source.id)
            .collect()
    }

    pub(crate) fn identity_high_watermark(&self) -> u64 {
        self.receipts.iter().fold(0, |maximum, receipt| {
            maximum
                .max(receipt.source.id)
                .max(receipt.target.as_ref().map(|target| target.id).unwrap_or(0))
        })
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.version != LEDGER_VERSION {
            return Err(format!(
                "unsupported legacy lifecycle ledger version {}",
                self.version
            ));
        }
        let mut operations = BTreeSet::new();
        let mut retired = BTreeSet::new();
        for receipt in &self.receipts {
            validate_committed_receipt(receipt)?;
            if !operations.insert(receipt.operation_id.clone()) {
                return Err(format!(
                    "duplicate legacy lifecycle operation {}",
                    receipt.operation_id
                ));
            }
            if !retired.insert(receipt.source.id) {
                return Err(format!(
                    "legacy category identity {} is retired more than once",
                    receipt.source.id
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyHistoryArtifact {
    pub filename: String,
    pub payload_json: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyLifecycleResult {
    pub categories: Vec<LegacyCategorySnapshot>,
    pub sessions: Vec<LegacySessionSnapshot>,
    pub tags: storage::CategoryTagsState,
    pub sand_state: Option<SandState>,
    pub history: Vec<LegacyHistoryArtifact>,
    pub detached_checkpoint_json: Option<String>,
    pub ledger: LegacyCategoryLifecycleLedger,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LegacyPreparedCategoryLifecycle {
    pub version: u8,
    pub review: LegacyCategoryLifecycleReview,
    pub receipt: LegacyLifecycleReceipt,
    pub result: LegacyLifecycleResult,
}

impl LegacyPreparedCategoryLifecycle {
    fn validate(&self) -> Result<(), String> {
        if self.version != PREPARED_VERSION {
            return Err(format!(
                "unsupported prepared legacy lifecycle version {}",
                self.version
            ));
        }
        validate_committed_receipt(&self.receipt)?;
        self.result.ledger.validate()?;
        if self.review.source != self.receipt.source
            || self.review.target != self.receipt.target
            || self.review.revision != self.receipt.preview_revision
            || self.review.references != self.receipt.references
        {
            return Err(
                "prepared legacy lifecycle receipt does not match its reviewed preview".to_string(),
            );
        }
        if !self
            .result
            .ledger
            .receipts
            .iter()
            .any(|receipt| receipt.operation_id == self.receipt.operation_id)
        {
            return Err("prepared legacy lifecycle result omits its permanent receipt".to_string());
        }
        validate_result(&self.result, self.receipt.source.id)?;
        Ok(())
    }
}

pub(crate) fn load_ledger(
    paths: &LegacyCategoryLifecyclePaths,
) -> Result<LegacyCategoryLifecycleLedger, String> {
    if !paths.ledger_json.exists() {
        return Ok(LegacyCategoryLifecycleLedger::default());
    }
    let ledger = storage::read_json::<LegacyCategoryLifecycleLedger>(&paths.ledger_json)?;
    ledger.validate()?;
    Ok(ledger)
}

pub(crate) fn next_category_id(
    catalog_next_id: u64,
    ledger: &LegacyCategoryLifecycleLedger,
) -> Result<u64, String> {
    catalog_next_id
        .max(
            ledger
                .identity_high_watermark()
                .checked_add(1)
                .ok_or_else(|| "legacy category identity space is exhausted".to_string())?,
        )
        .checked_add(0)
        .ok_or_else(|| "legacy category identity space is exhausted".to_string())
}

pub(crate) fn build_review(
    paths: &LegacyCategoryLifecyclePaths,
    source_category_id: u64,
    target_category_id: Option<u64>,
) -> Result<LegacyCategoryLifecycleReview, String> {
    let authority = load_authority(paths)?;
    build_review_from_authority(&authority, source_category_id, target_category_id)
}

pub(crate) fn prepare(
    paths: &LegacyCategoryLifecyclePaths,
    source_category_id: u64,
    target_category_id: Option<u64>,
    expected_revision: &str,
    applied_at_utc: DateTime<Utc>,
) -> Result<LegacyPreparedCategoryLifecycle, String> {
    if paths.prepared_json.exists() {
        return Err(
            "a prepared legacy category lifecycle receipt already requires replay".to_string(),
        );
    }
    let authority = load_authority(paths)?;
    let review = build_review_from_authority(&authority, source_category_id, target_category_id)?;
    if review.revision != expected_revision {
        return Err(format!(
            "legacy category lifecycle preview is stale; expected {}, found {}",
            expected_revision, review.revision
        ));
    }
    let result = stage_result(&authority, &review, applied_at_utc)?;
    let operation_kind = if target_category_id.is_some() {
        "merge"
    } else {
        "delete"
    };
    let operation_id = operation_id(
        operation_kind,
        source_category_id,
        target_category_id,
        &review.revision,
    );
    let receipt = LegacyLifecycleReceipt {
        operation_id,
        operation_kind: operation_kind.to_string(),
        source: review.source.clone(),
        target: review.target.clone(),
        preview_revision: review.revision.clone(),
        references: review.references.clone(),
        applied_at_utc,
    };
    let mut result = result;
    if let Some(existing) = result
        .ledger
        .receipts
        .iter()
        .find(|existing| existing.operation_id == receipt.operation_id)
    {
        if existing != &receipt {
            return Err(
                "legacy lifecycle operation ID conflicts with existing receipt".to_string(),
            );
        }
    } else {
        result.ledger.receipts.push(receipt.clone());
    }
    result.ledger.validate()?;
    let prepared = LegacyPreparedCategoryLifecycle {
        version: PREPARED_VERSION,
        review,
        receipt,
        result,
    };
    prepared.validate()?;
    storage::write_json_atomic(&paths.prepared_json, &prepared)?;
    Ok(prepared)
}

pub(crate) fn replay_prepared(
    paths: &LegacyCategoryLifecyclePaths,
) -> Result<Option<LegacyLifecycleReceipt>, String> {
    if !paths.prepared_json.exists() {
        return Ok(None);
    }
    let prepared = storage::read_json::<LegacyPreparedCategoryLifecycle>(&paths.prepared_json)?;
    prepared.validate()?;
    publish_result(paths, &prepared)?;
    storage::delete_file_if_exists(&paths.prepared_json)?;
    Ok(Some(prepared.receipt))
}

pub(crate) fn has_prepared(paths: &LegacyCategoryLifecyclePaths) -> bool {
    paths.prepared_json.exists()
}

#[derive(Debug)]
struct LoadedAuthority {
    categories: Vec<LegacyCategorySnapshot>,
    sessions: Vec<LegacySessionSnapshot>,
    tags: storage::CategoryTagsState,
    sand_state: Option<SandState>,
    history: Vec<LoadedHistoryArtifact>,
    detached_checkpoint_json: Option<String>,
    ledger: LegacyCategoryLifecycleLedger,
    revision_material: Vec<u8>,
}

#[derive(Debug)]
struct LoadedHistoryArtifact {
    filename: String,
    payload_json: String,
    parsed: ParsedHistory,
}

#[derive(Debug)]
enum ParsedHistory {
    Snapshot(SedimentSnapshot),
    State(SandState),
}

fn load_authority(paths: &LegacyCategoryLifecyclePaths) -> Result<LoadedAuthority, String> {
    if paths.prepared_json.exists() {
        return Err(
            "prepared legacy category lifecycle evidence must be replayed before a new preview"
                .to_string(),
        );
    }
    let loaded_categories = storage::try_load_categories_from_csv(&paths.categories_csv)
        .map_err(|error| error.to_string())?;
    let mut categories = Vec::new();
    for category in &loaded_categories.categories {
        if category.id.0 != 0 {
            categories.push(LegacyCategorySnapshot::from_category(category, false)?);
        }
    }
    for category in &loaded_categories.archived_categories {
        categories.push(LegacyCategorySnapshot::from_category(category, true)?);
    }
    categories.sort_by_key(|category| category.id);

    let mut session_catalog = loaded_categories.categories.clone();
    session_catalog.extend(loaded_categories.archived_categories.iter().cloned());
    let loaded_sessions =
        storage::try_load_sessions_from_csv(&paths.sessions_csv, &session_catalog)
            .map_err(|error| error.to_string())?;
    let sessions = loaded_sessions
        .sessions
        .iter()
        .map(LegacySessionSnapshot::from_session)
        .collect::<Vec<_>>();
    let tags = storage::try_load_category_tags(&paths.category_tags_json)?;
    let sand_state = if paths.sand_state_json.exists() {
        Some(storage::read_json::<SandState>(&paths.sand_state_json)?)
    } else {
        None
    };
    let detached_checkpoint_json = if paths.detached_runtime_json.exists() {
        Some(
            fs::read_to_string(&paths.detached_runtime_json)
                .map_err(|error| format!("cannot read detached runtime checkpoint: {error}"))?,
        )
    } else {
        None
    };
    let history = load_history(&paths.sand_history_dir)?;
    let ledger = load_ledger(paths)?;
    let current_ids = categories
        .iter()
        .map(|category| category.id)
        .collect::<BTreeSet<_>>();
    if let Some(reused) = ledger
        .retired_ids()
        .into_iter()
        .find(|retired| current_ids.contains(retired))
    {
        return Err(format!(
            "legacy category identity {reused} is retired but present in the catalog"
        ));
    }

    let mut revision_material = Vec::new();
    append_file_material(&mut revision_material, "categories", &paths.categories_csv)?;
    append_file_material(&mut revision_material, "sessions", &paths.sessions_csv)?;
    append_file_material(&mut revision_material, "tags", &paths.category_tags_json)?;
    append_file_material(&mut revision_material, "sand", &paths.sand_state_json)?;
    append_file_material(
        &mut revision_material,
        "checkpoint",
        &paths.detached_runtime_json,
    )?;
    append_file_material(&mut revision_material, "ledger", &paths.ledger_json)?;
    for artifact in &history {
        revision_material.extend_from_slice(artifact.filename.as_bytes());
        revision_material.push(0);
        revision_material.extend_from_slice(artifact.payload_json.as_bytes());
        revision_material.push(0xff);
    }

    Ok(LoadedAuthority {
        categories,
        sessions,
        tags,
        sand_state,
        history,
        detached_checkpoint_json,
        ledger,
        revision_material,
    })
}

fn load_history(directory: &Path) -> Result<Vec<LoadedHistoryArtifact>, String> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(directory)
        .map_err(|error| format!("cannot read legacy sand history: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot read legacy sand history entry: {error}"))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut artifacts = Vec::new();
    for path in paths {
        let filename = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "legacy sand history filename is not UTF-8".to_string())?
            .to_string();
        if filename.contains('/') || filename.contains('\\') || filename == "." || filename == ".."
        {
            return Err(format!("unsafe legacy sand history filename {filename}"));
        }
        let payload_json = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read legacy sand history {filename}: {error}"))?;
        let parsed = if let Ok(snapshot) = serde_json::from_str::<SedimentSnapshot>(&payload_json) {
            ParsedHistory::Snapshot(snapshot)
        } else if let Ok(state) = serde_json::from_str::<SandState>(&payload_json) {
            ParsedHistory::State(state)
        } else {
            return Err(format!(
                "legacy sand history {filename} has an unsupported payload"
            ));
        };
        artifacts.push(LoadedHistoryArtifact {
            filename,
            payload_json,
            parsed,
        });
    }
    Ok(artifacts)
}

fn build_review_from_authority(
    authority: &LoadedAuthority,
    source_category_id: u64,
    target_category_id: Option<u64>,
) -> Result<LegacyCategoryLifecycleReview, String> {
    if source_category_id == 0 {
        return Err("the reserved idle category cannot be merged or deleted".to_string());
    }
    if target_category_id == Some(source_category_id) {
        return Err("legacy category lifecycle source and target must differ".to_string());
    }
    let source = authority
        .categories
        .iter()
        .find(|category| category.id == source_category_id)
        .cloned()
        .ok_or_else(|| format!("source category {source_category_id} does not exist"))?;
    let target = target_category_id
        .map(|target_id| {
            authority
                .categories
                .iter()
                .find(|category| category.id == target_id)
                .cloned()
                .ok_or_else(|| format!("target category {target_id} does not exist"))
        })
        .transpose()?;
    if authority.ledger.retired_ids().contains(&source_category_id) {
        return Err(format!(
            "source category {source_category_id} is already retired"
        ));
    }

    let completed_sessions = u64::try_from(
        authority
            .sessions
            .iter()
            .filter(|session| session.category_id == source_category_id)
            .count(),
    )
    .map_err(|_| "legacy completed-session count exceeds u64".to_string())?;
    let tags = u64::try_from(
        authority
            .tags
            .tags_by_category
            .get(&source_category_id)
            .map(Vec::len)
            .unwrap_or(0),
    )
    .map_err(|_| "legacy tag count exceeds u64".to_string())?;
    let sand = authority
        .sand_state
        .as_ref()
        .map(|state| count_sand_state_category(state, source_category_id))
        .transpose()?
        .unwrap_or_default();
    let mut history_placed = 0u64;
    let mut history_pending = 0u64;
    for artifact in &authority.history {
        let references = match &artifact.parsed {
            ParsedHistory::Snapshot(snapshot) => {
                count_snapshot_category(snapshot, source_category_id)?
            }
            ParsedHistory::State(state) => count_sand_state_category(state, source_category_id)?,
        };
        history_placed = history_placed
            .checked_add(references.placed)
            .ok_or_else(|| "legacy history placed count exceeds u64".to_string())?;
        history_pending = history_pending
            .checked_add(references.pending)
            .ok_or_else(|| "legacy history pending count exceeds u64".to_string())?;
    }

    let (active_session, checkpoint_references, checkpoint_custody) =
        if let Some(payload) = authority.detached_checkpoint_json.as_deref() {
            if checkpoint_has_transition_receipt(payload)? {
                return Err(
                    "detached runtime checkpoint carries unresolved transition custody".to_string(),
                );
            }
            let value: Value = serde_json::from_str(payload)
                .map_err(|error| format!("invalid detached runtime checkpoint: {error}"))?;
            let active = value
                .get("active_category_id")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    "detached runtime checkpoint has no active category identity".to_string()
                })?;
            (
                u64::from(active == source_category_id),
                count_checkpoint_category_references(payload, source_category_id)?,
                "pending legacy checkpoint".to_string(),
            )
        } else {
            (0, 0, "absent".to_string())
        };

    let references = LegacyCategoryReferenceCounts {
        completed_sessions,
        active_session,
        tags,
        sand_placed: sand.placed,
        sand_pending: sand.pending,
        history_placed,
        history_pending,
        checkpoint_references,
    };
    if target_category_id.is_none() && references.total()? != 0 {
        return Err(format!(
            "category {source_category_id} still has {} references and cannot be permanently deleted",
            references.total()?
        ));
    }
    let mut material = authority.revision_material.clone();
    material.extend_from_slice(source_category_id.to_string().as_bytes());
    material.push(0);
    material.extend_from_slice(
        target_category_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "delete".to_string())
            .as_bytes(),
    );
    let revision = format!("{:016x}", fnv1a(&material));
    let confirmation_phrase =
        confirmation_phrase(source_category_id, target_category_id, &revision);
    Ok(LegacyCategoryLifecycleReview {
        source,
        target,
        references,
        checkpoint_custody,
        revision,
        confirmation_phrase,
    })
}

fn stage_result(
    authority: &LoadedAuthority,
    review: &LegacyCategoryLifecycleReview,
    applied_at_utc: DateTime<Utc>,
) -> Result<LegacyLifecycleResult, String> {
    let source_id = review.source.id;
    let target_id = review.target.as_ref().map(|target| target.id);
    let mut categories = authority.categories.clone();
    categories.retain(|category| category.id != source_id);
    let current_ids = categories
        .iter()
        .map(|category| category.id)
        .collect::<BTreeSet<_>>();
    if current_ids.contains(&source_id) {
        return Err("legacy lifecycle staging retained the source category".to_string());
    }

    let mut sessions = authority.sessions.clone();
    if let Some(target_id) = target_id {
        for session in &mut sessions {
            if session.category_id == source_id {
                session.category_id = target_id;
            }
        }
    }

    let mut tags = authority.tags.clone();
    if let Some(target_id) = target_id {
        let target_tags = tags.tags_by_category.remove(&target_id).unwrap_or_default();
        let source_tags = tags.tags_by_category.remove(&source_id).unwrap_or_default();
        let mut merged = Vec::new();
        let mut seen = BTreeSet::new();
        for tag in target_tags.into_iter().chain(source_tags) {
            if seen.insert(tag.clone()) {
                merged.push(tag);
            }
        }
        if !merged.is_empty() {
            tags.tags_by_category.insert(target_id, merged);
        }
    } else {
        tags.tags_by_category.remove(&source_id);
    }

    let sand_state = authority
        .sand_state
        .clone()
        .map(|mut state| {
            if let Some(target_id) = target_id {
                reassign_sand_state_category(&mut state, source_id, target_id)?;
            }
            Ok::<SandState, String>(state)
        })
        .transpose()?;

    let affected_days = affected_days(&authority.sessions, source_id)?;
    let mut history = Vec::new();
    for artifact in &authority.history {
        let payload_json = match &artifact.parsed {
            ParsedHistory::Snapshot(snapshot)
                if snapshot.kind == SedimentSnapshotKind::DailyContribution
                    && snapshot
                        .operational_day
                        .as_deref()
                        .and_then(|day| NaiveDate::parse_from_str(day, "%Y-%m-%d").ok())
                        .is_some_and(|day| affected_days.contains(&day)) =>
            {
                let day = snapshot.operational_day.as_deref().ok_or_else(|| {
                    format!(
                        "daily contribution {} has no operational day",
                        artifact.filename
                    )
                })?;
                regenerate_daily_contribution(
                    day,
                    snapshot.state.grid_width,
                    snapshot.state.grid_height,
                    &sessions,
                )?
                .map(|snapshot| serde_json::to_string(&snapshot))
                .transpose()
                .map_err(|error| {
                    format!(
                        "cannot serialize daily contribution {}: {error}",
                        artifact.filename
                    )
                })?
            }
            ParsedHistory::Snapshot(snapshot) => {
                let mut snapshot = snapshot.clone();
                if let Some(target_id) = target_id {
                    reassign_snapshot_category(&mut snapshot, source_id, target_id)?;
                }
                Some(serde_json::to_string(&snapshot).map_err(|error| {
                    format!("cannot serialize history {}: {error}", artifact.filename)
                })?)
            }
            ParsedHistory::State(state) => {
                let mut state = state.clone();
                if let Some(target_id) = target_id {
                    reassign_sand_state_category(&mut state, source_id, target_id)?;
                }
                Some(serde_json::to_string(&state).map_err(|error| {
                    format!("cannot serialize history {}: {error}", artifact.filename)
                })?)
            }
        };
        history.push(LegacyHistoryArtifact {
            filename: artifact.filename.clone(),
            payload_json,
        });
    }

    if !affected_days.is_empty() {
        let (width, height) = sand_state
            .as_ref()
            .map(|state| (state.grid_width, state.grid_height))
            .ok_or_else(|| {
                "legacy lifecycle cannot regenerate daily contributions without canonical sand dimensions"
                    .to_string()
            })?;
        for day in affected_days {
            let filename = format!("{}.contribution.json", day.format("%Y-%m-%d"));
            if history.iter().any(|artifact| artifact.filename == filename) {
                continue;
            }
            let payload_json = regenerate_daily_contribution(
                &day.format("%Y-%m-%d").to_string(),
                width,
                height,
                &sessions,
            )?
            .map(|snapshot| serde_json::to_string(&snapshot))
            .transpose()
            .map_err(|error| format!("cannot serialize daily contribution {filename}: {error}"))?;
            history.push(LegacyHistoryArtifact {
                filename,
                payload_json,
            });
        }
    }
    history.sort_by(|left, right| left.filename.cmp(&right.filename));

    let detached_checkpoint_json = authority
        .detached_checkpoint_json
        .as_deref()
        .map(|payload| {
            if let Some(target_id) = target_id {
                reassign_checkpoint_category(payload, source_id, target_id).map(|(json, _)| json)
            } else {
                Ok(payload.to_string())
            }
        })
        .transpose()?;

    let receipt = LegacyLifecycleReceipt {
        operation_id: operation_id(
            if target_id.is_some() {
                "merge"
            } else {
                "delete"
            },
            source_id,
            target_id,
            &review.revision,
        ),
        operation_kind: if target_id.is_some() {
            "merge".to_string()
        } else {
            "delete".to_string()
        },
        source: review.source.clone(),
        target: review.target.clone(),
        preview_revision: review.revision.clone(),
        references: review.references.clone(),
        applied_at_utc,
    };
    let mut ledger = authority.ledger.clone();
    ledger.receipts.push(receipt);
    ledger.validate()?;

    let result = LegacyLifecycleResult {
        categories,
        sessions,
        tags,
        sand_state,
        history,
        detached_checkpoint_json,
        ledger,
    };
    validate_result(&result, source_id)?;
    Ok(result)
}

fn affected_days(
    sessions: &[LegacySessionSnapshot],
    source_category_id: u64,
) -> Result<BTreeSet<NaiveDate>, String> {
    let mut days = BTreeSet::new();
    for session in sessions
        .iter()
        .filter(|session| session.category_id == source_category_id)
    {
        let started = session.started_at_utc.ok_or_else(|| {
            format!(
                "session {} has no UTC start; lifecycle daily reconstruction is unsafe",
                session.id
            )
        })?;
        let ended = session.ended_at_utc.ok_or_else(|| {
            format!(
                "session {} has no UTC end; lifecycle daily reconstruction is unsafe",
                session.id
            )
        })?;
        let policy = session.operational_day_policy.ok_or_else(|| {
            format!(
                "session {} has no operational-day policy; lifecycle daily reconstruction is unsafe",
                session.id
            )
        })?;
        days.extend(
            temporal::allocate_operational_day_slices(
                started,
                ended,
                session.elapsed_seconds,
                OperationalDayPolicy {
                    utc_offset_seconds: policy.utc_offset_seconds,
                    start_minutes: policy.start_minutes,
                },
            )?
            .into_iter()
            .map(|slice| slice.operational_day),
        );
    }
    Ok(days)
}

fn regenerate_daily_contribution(
    operational_day: &str,
    width: usize,
    height: usize,
    sessions: &[LegacySessionSnapshot],
) -> Result<Option<SedimentSnapshot>, String> {
    let day = NaiveDate::parse_from_str(operational_day, "%Y-%m-%d")
        .map_err(|error| format!("invalid daily contribution day {operational_day}: {error}"))?;
    let mut slices = Vec::new();
    for session in sessions {
        let (Some(started), Some(ended), Some(policy)) = (
            session.started_at_utc,
            session.ended_at_utc,
            session.operational_day_policy,
        ) else {
            continue;
        };
        let policy = OperationalDayPolicy {
            utc_offset_seconds: policy.utc_offset_seconds,
            start_minutes: policy.start_minutes,
        };
        for slice in temporal::allocate_operational_day_slices(
            started,
            ended,
            session.elapsed_seconds,
            policy,
        )? {
            if slice.operational_day == day {
                slices.push(DailySedimentSlice {
                    category_id: session.category_id,
                    elapsed_seconds: slice.elapsed_seconds,
                    start_time: temporal::civil_from_policy(slice.started_at_utc, policy)?
                        .format("%H:%M:%S")
                        .to_string(),
                    end_time: temporal::civil_from_policy(slice.ended_at_utc, policy)?
                        .format("%H:%M:%S")
                        .to_string(),
                    session_id: session.id,
                });
            }
        }
    }
    Ok(daily_contribution_from_slices(
        operational_day,
        width,
        height,
        &slices,
    ))
}

fn validate_result(result: &LegacyLifecycleResult, source_category_id: u64) -> Result<(), String> {
    if result
        .categories
        .iter()
        .any(|category| category.id == source_category_id)
    {
        return Err("legacy lifecycle result retains source catalog identity".to_string());
    }
    if result
        .sessions
        .iter()
        .any(|session| session.category_id == source_category_id)
    {
        return Err("legacy lifecycle result retains source session identity".to_string());
    }
    if result
        .tags
        .tags_by_category
        .contains_key(&source_category_id)
    {
        return Err("legacy lifecycle result retains source tags".to_string());
    }
    if let Some(state) = result.sand_state.as_ref()
        && count_sand_state_category(state, source_category_id)?.total()? != 0
    {
        return Err("legacy lifecycle result retains source canonical sediment".to_string());
    }
    for artifact in &result.history {
        if let Some(payload) = artifact.payload_json.as_deref() {
            if let Ok(snapshot) = serde_json::from_str::<SedimentSnapshot>(payload) {
                if count_snapshot_category(&snapshot, source_category_id)?.total()? != 0 {
                    return Err(format!(
                        "legacy lifecycle result retains source history in {}",
                        artifact.filename
                    ));
                }
            } else if let Ok(state) = serde_json::from_str::<SandState>(payload) {
                if count_sand_state_category(&state, source_category_id)?.total()? != 0 {
                    return Err(format!(
                        "legacy lifecycle result retains source history in {}",
                        artifact.filename
                    ));
                }
            } else {
                return Err(format!(
                    "legacy lifecycle result history {} is malformed",
                    artifact.filename
                ));
            }
        }
    }
    if let Some(payload) = result.detached_checkpoint_json.as_deref()
        && count_checkpoint_category_references(payload, source_category_id)? != 0
    {
        return Err("legacy lifecycle result retains source checkpoint identity".to_string());
    }
    Ok(())
}

fn validate_committed_receipt(receipt: &LegacyLifecycleReceipt) -> Result<(), String> {
    if receipt.source.id == 0 {
        return Err("legacy lifecycle receipt cannot retire idle".to_string());
    }
    match receipt.operation_kind.as_str() {
        "merge" => {
            let target = receipt
                .target
                .as_ref()
                .ok_or_else(|| "legacy merge receipt has no target".to_string())?;
            if target.id == receipt.source.id || target.id == 0 {
                return Err("legacy merge receipt has invalid target identity".to_string());
            }
        }
        "delete" => {
            if receipt.target.is_some() || receipt.references.total()? != 0 {
                return Err(
                    "legacy delete receipt must be targetless and reference-free".to_string(),
                );
            }
        }
        other => return Err(format!("unsupported legacy lifecycle operation {other}")),
    }
    let expected = operation_id(
        &receipt.operation_kind,
        receipt.source.id,
        receipt.target.as_ref().map(|target| target.id),
        &receipt.preview_revision,
    );
    if receipt.operation_id != expected {
        return Err("legacy lifecycle receipt operation ID is inconsistent".to_string());
    }
    Ok(())
}

fn publish_result(
    paths: &LegacyCategoryLifecyclePaths,
    prepared: &LegacyPreparedCategoryLifecycle,
) -> Result<(), String> {
    let result = &prepared.result;
    let active_categories = result
        .categories
        .iter()
        .filter(|category| !category.archived)
        .map(LegacyCategorySnapshot::to_category)
        .collect::<Result<Vec<_>, _>>()?;
    let archived_categories = result
        .categories
        .iter()
        .filter(|category| category.archived)
        .map(LegacyCategorySnapshot::to_category)
        .collect::<Result<Vec<_>, _>>()?;
    let sessions = result
        .sessions
        .iter()
        .map(LegacySessionSnapshot::to_session)
        .collect::<Vec<_>>();
    let mut full_catalog = active_categories.clone();
    full_catalog.extend(archived_categories.iter().cloned());

    maybe_inject_test_fault("prepared")?;
    storage::save_sessions_to_csv(&paths.sessions_csv, &sessions, &full_catalog)?;
    maybe_inject_test_fault("sessions")?;
    storage::save_category_tags(&paths.category_tags_json, &result.tags)?;
    maybe_inject_test_fault("tags")?;
    match result.sand_state.as_ref() {
        Some(state) => storage::save_sand_state(&paths.sand_state_json, state)?,
        None => storage::delete_file_if_exists(&paths.sand_state_json)?,
    }
    maybe_inject_test_fault("sand")?;
    fs::create_dir_all(&paths.sand_history_dir)
        .map_err(|error| format!("cannot create legacy sand history directory: {error}"))?;
    for artifact in &result.history {
        let path = safe_history_path(&paths.sand_history_dir, &artifact.filename)?;
        match artifact.payload_json.as_deref() {
            Some(payload) => write_raw_json_atomic(&path, payload)?,
            None => storage::delete_file_if_exists(&path)?,
        }
    }
    maybe_inject_test_fault("history")?;
    match result.detached_checkpoint_json.as_deref() {
        Some(payload) => write_raw_json_atomic(&paths.detached_runtime_json, payload)?,
        None => storage::delete_file_if_exists(&paths.detached_runtime_json)?,
    }
    maybe_inject_test_fault("checkpoint")?;
    storage::save_category_catalog_to_csv(
        &paths.categories_csv,
        &active_categories,
        &archived_categories,
    )?;
    maybe_inject_test_fault("catalog")?;
    storage::write_json_atomic(&paths.ledger_json, &result.ledger)?;
    maybe_inject_test_fault("ledger")?;
    Ok(())
}

fn safe_history_path(directory: &Path, filename: &str) -> Result<PathBuf, String> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename == "."
        || filename == ".."
    {
        return Err(format!("unsafe legacy history filename {filename}"));
    }
    Ok(directory.join(filename))
}

fn write_raw_json_atomic(path: &Path, payload: &str) -> Result<(), String> {
    let value: Value = serde_json::from_str(payload).map_err(|error| {
        format!(
            "invalid prepared JSON payload for {}: {error}",
            path.display()
        )
    })?;
    storage::write_json_atomic(path, &value)
}

fn append_file_material(material: &mut Vec<u8>, label: &str, path: &Path) -> Result<(), String> {
    material.extend_from_slice(label.as_bytes());
    material.push(0);
    if path.exists() {
        let bytes = fs::read(path)
            .map_err(|error| format!("cannot read lifecycle source {}: {error}", path.display()))?;
        material.extend_from_slice(&bytes);
    }
    material.push(0xff);
    Ok(())
}

fn confirmation_phrase(source: u64, target: Option<u64>, revision: &str) -> String {
    match target {
        Some(target) => format!("MERGE {source} INTO {target} {revision}"),
        None => format!("DELETE {source} {revision}"),
    }
}

fn operation_id(kind: &str, source: u64, target: Option<u64>, revision: &str) -> String {
    format!(
        "legacy-category-{kind}:{source}:{}:{revision}",
        target
            .map(|target| target.to_string())
            .unwrap_or_else(|| "none".to_string())
    )
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
thread_local! {
    static TEST_FAULT: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

fn maybe_inject_test_fault(phase: &str) -> Result<(), String> {
    #[cfg(test)]
    if TEST_FAULT.with(|fault| fault.borrow().as_deref() == Some(phase)) {
        return Err(format!("injected legacy lifecycle failure at {phase}"));
    }
    let _ = phase;
    Ok(())
}

#[cfg(test)]
struct TestFaultReset;

#[cfg(test)]
impl Drop for TestFaultReset {
    fn drop(&mut self) {
        TEST_FAULT.with(|fault| *fault.borrow_mut() = None);
    }
}

#[cfg(test)]
fn with_test_fault<T>(phase: &str, operation: impl FnOnce() -> T) -> T {
    TEST_FAULT.with(|fault| *fault.borrow_mut() = Some(phase.to_string()));
    let _reset = TestFaultReset;
    operation()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sand::{PendingGrainRun, SandStateGrain};

    fn unique_root(label: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("strata-{label}-{}-{stamp}", std::process::id()))
    }

    fn paths(root: &Path) -> LegacyCategoryLifecyclePaths {
        LegacyCategoryLifecyclePaths {
            categories_csv: root.join("data/categories.csv"),
            sessions_csv: root.join("data/time_log.csv"),
            category_tags_json: root.join("state/category_tags.json"),
            sand_state_json: root.join("state/sand_state.json"),
            detached_runtime_json: root.join("state/detached_runtime.json"),
            sand_history_dir: root.join("state/sand_history"),
            prepared_json: root.join("state/category_lifecycle_prepared.json"),
            ledger_json: root.join("state/category_lifecycle_ledger.json"),
        }
    }

    fn category(id: u64, name: &str) -> Category {
        Category {
            id: CategoryId::new(id),
            name: name.to_string(),
            color: COLORS[(id as usize - 1) % COLORS.len()],
            description: format!("{name} metadata"),
            karma_effect: if id == 1 { 1 } else { -1 },
        }
    }

    fn sand_state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 4,
            grid_height: 4,
            grains: vec![
                SandStateGrain {
                    x: 0,
                    y: 0,
                    category_id: 1,
                },
                SandStateGrain {
                    x: 1,
                    y: 0,
                    category_id: 2,
                },
            ],
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 1,
            pending_grains: vec![1],
            pending_runs: vec![PendingGrainRun {
                category_id: 1,
                count: 3,
            }],
        }
    }

    fn session() -> Session {
        Session {
            id: 1,
            date: "2026-08-03".to_string(),
            category_id: CategoryId::new(1),
            project: "Project".to_string(),
            description: "completed".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: Some("2026-08-03T16:00:00Z".parse().unwrap()),
            ended_at_utc: Some("2026-08-03T17:00:00Z".parse().unwrap()),
            operational_day_policy: Some(OperationalDayPolicy {
                utc_offset_seconds: -21600,
                start_minutes: 360,
            }),
        }
    }

    fn checkpoint() -> String {
        serde_json::json!({
            "schema_version": 3,
            "detached_at_utc": "2026-08-03T18:00:00Z",
            "simulation_time_utc": "2026-08-03T18:00:00Z",
            "spawn_accumulator_nanos": 0,
            "physics_accumulator_nanos": 0,
            "active_category_id": 1,
            "active_description": "active",
            "active_session_started_at_utc": "2026-08-03T18:00:00Z",
            "sand_state": sand_state(),
            "pending_mutations": [{"SwitchLayer": {"category_id": 1}}],
            "recovery_target_utc": null,
            "legacy_recovery_committed": false,
            "legacy_transition": null,
            "legacy_finish": null,
            "clear_all": null
        })
        .to_string()
    }

    fn seed(paths: &LegacyCategoryLifecyclePaths) {
        fs::create_dir_all(paths.categories_csv.parent().unwrap()).unwrap();
        fs::create_dir_all(paths.category_tags_json.parent().unwrap()).unwrap();
        fs::create_dir_all(&paths.sand_history_dir).unwrap();
        storage::save_category_catalog_to_csv(
            &paths.categories_csv,
            &[category(1, "Source"), category(2, "Target")],
            &[],
        )
        .unwrap();
        storage::save_sessions_to_csv(
            &paths.sessions_csv,
            &[session()],
            &[category(1, "Source"), category(2, "Target")],
        )
        .unwrap();
        let mut tags = storage::CategoryTagsState::default();
        tags.tags_by_category
            .insert(1, vec!["source".to_string(), "shared".to_string()]);
        tags.tags_by_category
            .insert(2, vec!["target".to_string(), "shared".to_string()]);
        storage::save_category_tags(&paths.category_tags_json, &tags).unwrap();
        storage::save_sand_state(&paths.sand_state_json, &sand_state()).unwrap();
        write_raw_json_atomic(&paths.detached_runtime_json, &checkpoint()).unwrap();
        let contribution = daily_contribution_from_slices(
            "2026-08-03",
            4,
            4,
            &[DailySedimentSlice {
                category_id: 1,
                elapsed_seconds: 3600,
                start_time: "10:00:00".to_string(),
                end_time: "11:00:00".to_string(),
                session_id: 1,
            }],
        )
        .unwrap();
        storage::write_json_atomic(
            &paths.sand_history_dir.join("2026-08-03.contribution.json"),
            &contribution,
        )
        .unwrap();
    }

    #[test]
    fn merge_preview_and_replay_cover_every_legacy_authority() {
        let root = unique_root("legacy-lifecycle-merge");
        let paths = paths(&root);
        seed(&paths);
        let review = build_review(&paths, 1, Some(2)).unwrap();
        assert_eq!(review.references.completed_sessions, 1);
        assert_eq!(review.references.active_session, 1);
        assert!(review.references.sand_placed > 0);
        assert!(review.references.history_placed > 0);
        assert!(review.references.checkpoint_references > 0);
        assert_eq!(
            review.confirmation_phrase,
            format!("MERGE 1 INTO 2 {}", review.revision)
        );
        prepare(
            &paths,
            1,
            Some(2),
            &review.revision,
            "2026-08-03T19:00:00Z".parse().unwrap(),
        )
        .unwrap();
        assert!(paths.prepared_json.exists());
        replay_prepared(&paths).unwrap();
        assert!(!paths.prepared_json.exists());
        let result = load_authority(&paths).unwrap();
        assert!(result.categories.iter().all(|category| category.id != 1));
        assert!(
            result
                .sessions
                .iter()
                .all(|session| session.category_id == 2)
        );
        assert!(!result.tags.tags_by_category.contains_key(&1));
        assert_eq!(
            result.tags.tags_by_category.get(&2).cloned().unwrap(),
            vec!["target", "shared", "source"]
        );
        assert_eq!(
            count_sand_state_category(result.sand_state.as_ref().unwrap(), 1)
                .unwrap()
                .total()
                .unwrap(),
            0
        );
        assert_eq!(
            count_checkpoint_category_references(
                result.detached_checkpoint_json.as_deref().unwrap(),
                1
            )
            .unwrap(),
            0
        );
        assert_eq!(result.ledger.retired_ids(), BTreeSet::from([1]));
        assert_eq!(next_category_id(3, &result.ledger).unwrap(), 3);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn startup_order_replays_prepared_before_loading_catalog_and_identity() {
        let root = unique_root("legacy-lifecycle-startup");
        let paths = paths(&root);
        seed(&paths);
        let review = build_review(&paths, 1, Some(2)).unwrap();
        prepare(
            &paths,
            1,
            Some(2),
            &review.revision,
            "2026-08-03T19:00:00Z".parse().unwrap(),
        )
        .unwrap();
        assert!(has_prepared(&paths));

        replay_prepared(&paths).unwrap();
        let loaded = storage::try_load_categories_from_csv(&paths.categories_csv).unwrap();
        let ledger = load_ledger(&paths).unwrap();
        let next = next_category_id(loaded.next_category_id, &ledger).unwrap();
        assert!(!has_prepared(&paths));
        assert!(loaded.categories.iter().all(|category| category.id.0 != 1));
        assert_eq!(next, 3);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn targetless_delete_requires_complete_zero_reference_preview() {
        let root = unique_root("legacy-lifecycle-delete");
        let paths = paths(&root);
        seed(&paths);
        assert!(
            build_review(&paths, 1, None)
                .unwrap_err()
                .contains("still has")
        );
        let disposable = category(3, "Disposable");
        storage::save_category_catalog_to_csv(
            &paths.categories_csv,
            &[category(1, "Source"), category(2, "Target"), disposable],
            &[],
        )
        .unwrap();
        let review = build_review(&paths, 3, None).unwrap();
        prepare(
            &paths,
            3,
            None,
            &review.revision,
            "2026-08-03T19:00:00Z".parse().unwrap(),
        )
        .unwrap();
        replay_prepared(&paths).unwrap();
        let result = load_authority(&paths).unwrap();
        assert!(result.categories.iter().all(|category| category.id != 3));
        assert_eq!(result.ledger.retired_ids(), BTreeSet::from([3]));
        assert_eq!(next_category_id(3, &result.ledger).unwrap(), 4);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn stale_preview_self_merge_idle_and_receipt_checkpoint_fail_closed() {
        let root = unique_root("legacy-lifecycle-refusal");
        let paths = paths(&root);
        seed(&paths);
        assert!(
            build_review(&paths, 0, Some(2))
                .unwrap_err()
                .contains("idle")
        );
        assert!(
            build_review(&paths, 1, Some(1))
                .unwrap_err()
                .contains("differ")
        );
        let review = build_review(&paths, 1, Some(2)).unwrap();
        let mut tags = storage::try_load_category_tags(&paths.category_tags_json).unwrap();
        tags.tags_by_category
            .entry(2)
            .or_default()
            .push("changed".to_string());
        storage::save_category_tags(&paths.category_tags_json, &tags).unwrap();
        assert!(
            prepare(
                &paths,
                1,
                Some(2),
                &review.revision,
                "2026-08-03T19:00:00Z".parse().unwrap(),
            )
            .unwrap_err()
            .contains("stale")
        );
        let mut value: Value = serde_json::from_str(&checkpoint()).unwrap();
        value["legacy_transition"] = serde_json::json!({"operation_id": "pending"});
        storage::write_json_atomic(&paths.detached_runtime_json, &value).unwrap();
        assert!(
            build_review(&paths, 1, Some(2))
                .unwrap_err()
                .contains("transition custody")
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn every_replay_fault_retains_prepared_evidence_and_clean_retry_converges() {
        for phase in [
            "prepared",
            "sessions",
            "tags",
            "sand",
            "history",
            "checkpoint",
            "catalog",
            "ledger",
        ] {
            let root = unique_root(&format!("legacy-lifecycle-{phase}"));
            let paths = paths(&root);
            seed(&paths);
            let review = build_review(&paths, 1, Some(2)).unwrap();
            prepare(
                &paths,
                1,
                Some(2),
                &review.revision,
                "2026-08-03T19:00:00Z".parse().unwrap(),
            )
            .unwrap();
            let failed = with_test_fault(phase, || replay_prepared(&paths));
            assert!(failed.is_err(), "phase {phase} unexpectedly succeeded");
            assert!(paths.prepared_json.exists(), "phase {phase}");
            replay_prepared(&paths).unwrap();
            assert!(!paths.prepared_json.exists(), "phase {phase}");
            let result = load_authority(&paths).unwrap();
            assert!(result.categories.iter().all(|category| category.id != 1));
            assert_eq!(result.ledger.receipts.len(), 1);
            fs::remove_dir_all(root).ok();
        }
    }
}
