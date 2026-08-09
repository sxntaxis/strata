use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use thiserror::Error;

use crate::{domain::OperationalDayPolicy, temporal};

use super::{NewActiveSession, SessionCompletion, SqliteRepository, SqliteStoreError};

#[derive(Debug, Error)]
pub(crate) enum RepositoryError {
    #[error(transparent)]
    Store(#[from] SqliteStoreError),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid repository input: {0}")]
    InvalidInput(String),
    #[error("{entity} {identity} was not found")]
    NotFound {
        entity: &'static str,
        identity: String,
    },
    #[error("there is no active session to switch")]
    NoActiveSession,
    #[error("checkpoint transition from {from} to {to} is not allowed")]
    InvalidCheckpointTransition { from: String, to: String },
    #[error("checkpoint status changed concurrently; expected {expected}, found {actual}")]
    CheckpointStatusConflict { expected: String, actual: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryRecord {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub color_index: i64,
    pub balance_effect: i64,
    pub archived_at_utc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewCategoryRecord<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub color_index: i64,
    pub balance_effect: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionRecord {
    pub id: i64,
    pub stable_id: String,
    pub project: String,
    pub category_id: i64,
    pub description: String,
    pub started_at_utc: String,
    pub ended_at_utc: String,
    pub operational_day: String,
    pub elapsed_seconds: i64,
    pub boundary_utc_offset_seconds: Option<i64>,
    pub boundary_start_minutes: Option<i64>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewSessionRecord<'a> {
    pub stable_id: &'a str,
    pub project: &'a str,
    pub category_id: i64,
    pub description: &'a str,
    pub started_at_utc: &'a str,
    pub ended_at_utc: &'a str,
    pub operational_day: &'a str,
    pub elapsed_seconds: i64,
    pub boundary_utc_offset_seconds: i32,
    pub boundary_start_minutes: u16,
    pub source: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveSessionRecord {
    pub stable_id: String,
    pub project: String,
    pub category_id: i64,
    pub description: String,
    pub started_at_utc: String,
    pub recovery_kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckpointStatus {
    Pending,
    Recovering,
    Committed,
    Quarantined,
}

impl CheckpointStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Recovering => "recovering",
            Self::Committed => "committed",
            Self::Quarantined => "quarantined",
        }
    }

    fn parse(value: &str) -> Result<Self, RepositoryError> {
        match value {
            "pending" => Ok(Self::Pending),
            "recovering" => Ok(Self::Recovering),
            "committed" => Ok(Self::Committed),
            "quarantined" => Ok(Self::Quarantined),
            other => Err(RepositoryError::InvalidInput(format!(
                "unknown checkpoint status '{other}'"
            ))),
        }
    }

    fn allows(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Pending, Self::Recovering | Self::Quarantined)
                | (Self::Recovering, Self::Committed | Self::Quarantined)
                | (Self::Quarantined, Self::Pending)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointRecord {
    pub status: CheckpointStatus,
    pub detached_at_utc: String,
    pub simulation_time_utc: String,
    pub active_session_stable_id: Option<String>,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SandStateRecord {
    pub formation_id: String,
    pub quantum_seconds: i64,
    pub grid_width: i64,
    pub grid_height: i64,
    pub payload_json: String,
    pub updated_at_utc: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotKind {
    Daily,
    DailyContribution,
    Manual,
    FormationEnd,
    Recovery,
}

impl SnapshotKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::DailyContribution => "daily-contribution",
            Self::Manual => "manual",
            Self::FormationEnd => "formation_end",
            Self::Recovery => "recovery",
        }
    }

    fn parse(value: &str) -> Result<Self, RepositoryError> {
        match value {
            "daily" => Ok(Self::Daily),
            "daily-contribution" => Ok(Self::DailyContribution),
            "manual" => Ok(Self::Manual),
            "formation_end" => Ok(Self::FormationEnd),
            "recovery" => Ok(Self::Recovery),
            other => Err(RepositoryError::InvalidInput(format!(
                "unknown snapshot kind '{other}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SandSnapshotRecord {
    pub id: i64,
    pub formation_id: String,
    pub snapshot_kind: SnapshotKind,
    pub operational_day: Option<String>,
    pub quantum_seconds: i64,
    pub payload_json: String,
    pub captured_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewSandSnapshotRecord<'a> {
    pub formation_id: &'a str,
    pub snapshot_kind: SnapshotKind,
    pub operational_day: Option<&'a str>,
    pub quantum_seconds: i64,
    pub payload_json: &'a str,
    pub captured_at_utc: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryLifecycleReceiptRecord {
    pub operation_id: String,
    pub operation_kind: String,
    pub source_category_id: i64,
    pub target_category_id: Option<i64>,
    pub source_metadata_json: String,
    pub target_metadata_json: Option<String>,
    pub preview_revision: String,
    pub reference_counts_json: String,
    pub applied_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepositorySnapshot {
    pub categories: Vec<CategoryRecord>,
    pub category_tags: BTreeMap<i64, Vec<String>>,
    pub sessions: Vec<SessionRecord>,
    pub active_session: Option<ActiveSessionRecord>,
    pub checkpoint: Option<CheckpointRecord>,
    pub sand_state: Option<SandStateRecord>,
    pub sand_snapshots: Vec<SandSnapshotRecord>,
    pub category_lifecycle_receipts: Vec<CategoryLifecycleReceiptRecord>,
}

impl SqliteRepository {
    pub fn list_categories(
        &self,
        include_archived: bool,
    ) -> Result<Vec<CategoryRecord>, RepositoryError> {
        query_categories(&self.connection, include_archived)
    }

    pub fn create_category(
        &mut self,
        category: &NewCategoryRecord<'_>,
    ) -> Result<i64, RepositoryError> {
        validate_category(category)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let maximum_identity: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(identity), 0)
             FROM (
                 SELECT id AS identity FROM categories
                 UNION ALL
                 SELECT source_category_id AS identity FROM category_lifecycle_receipts
                 UNION ALL
                 SELECT target_category_id AS identity FROM category_lifecycle_receipts
                 WHERE target_category_id IS NOT NULL
             )",
            [],
            |row| row.get(0),
        )?;
        let id = maximum_identity.checked_add(1).ok_or_else(|| {
            RepositoryError::InvalidInput("category identity space is exhausted".to_string())
        })?;
        transaction.execute(
            "INSERT INTO categories(id, name, description, color_index, balance_effect)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                category.name.trim(),
                category.description,
                category.color_index,
                category.balance_effect,
            ],
        )?;
        transaction.commit()?;
        Ok(id)
    }

    pub fn update_category(
        &mut self,
        category_id: i64,
        category: &NewCategoryRecord<'_>,
    ) -> Result<(), RepositoryError> {
        if category_id == 0 {
            return Err(RepositoryError::InvalidInput(
                "the reserved idle category cannot be edited".to_string(),
            ));
        }
        validate_category(category)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE categories
             SET name = ?1, description = ?2, color_index = ?3, balance_effect = ?4
             WHERE id = ?5",
            params![
                category.name.trim(),
                category.description,
                category.color_index,
                category.balance_effect,
                category_id,
            ],
        )?;
        ensure_changed(changed, "category", category_id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn archive_category(
        &mut self,
        category_id: i64,
        archived_at_utc: &str,
    ) -> Result<(), RepositoryError> {
        if category_id == 0 {
            return Err(RepositoryError::InvalidInput(
                "the reserved idle category cannot be archived".to_string(),
            ));
        }
        require_non_empty(archived_at_utc, "archive timestamp")?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE categories
             SET archived_at_utc = ?1
             WHERE id = ?2 AND archived_at_utc IS NULL",
            params![archived_at_utc, category_id],
        )?;
        ensure_changed(changed, "active category", category_id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn restore_category(&mut self, category_id: i64) -> Result<(), RepositoryError> {
        if category_id == 0 {
            return Ok(());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE categories
             SET archived_at_utc = NULL
             WHERE id = ?1 AND archived_at_utc IS NOT NULL",
            params![category_id],
        )?;
        ensure_changed(changed, "archived category", category_id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn replace_category_tags(
        &mut self,
        category_id: i64,
        tags: &[String],
    ) -> Result<(), RepositoryError> {
        let normalized = normalize_tags(tags)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE id = ?1)",
            params![category_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(not_found("category", category_id));
        }
        transaction.execute(
            "DELETE FROM category_tags WHERE category_id = ?1",
            params![category_id],
        )?;
        for (ordinal, tag) in normalized.iter().enumerate() {
            let ordinal = i64::try_from(ordinal)
                .map_err(|_| RepositoryError::InvalidInput("too many category tags".to_string()))?;
            transaction.execute(
                "INSERT INTO category_tags(category_id, ordinal, tag)
                 VALUES (?1, ?2, ?3)",
                params![category_id, ordinal, tag],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn category_tags(&self) -> Result<BTreeMap<i64, Vec<String>>, RepositoryError> {
        query_category_tags(&self.connection)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRecord>, RepositoryError> {
        query_all_sessions(&self.connection)
    }

    pub fn list_sessions_between(
        &self,
        start_operational_day: &str,
        end_operational_day: &str,
    ) -> Result<Vec<SessionRecord>, RepositoryError> {
        require_non_empty(start_operational_day, "start operational day")?;
        require_non_empty(end_operational_day, "end operational day")?;
        if start_operational_day > end_operational_day {
            return Err(RepositoryError::InvalidInput(
                "start operational day is after end operational day".to_string(),
            ));
        }
        let start =
            NaiveDate::parse_from_str(start_operational_day, "%Y-%m-%d").map_err(|error| {
                RepositoryError::InvalidInput(format!("invalid start operational day: {error}"))
            })?;
        let end = NaiveDate::parse_from_str(end_operational_day, "%Y-%m-%d").map_err(|error| {
            RepositoryError::InvalidInput(format!("invalid end operational day: {error}"))
        })?;
        Ok(query_all_sessions(&self.connection)?
            .into_iter()
            .filter(|session| session_overlaps_range(session, start, end))
            .collect())
    }

    pub fn insert_session(
        &mut self,
        session: &NewSessionRecord<'_>,
    ) -> Result<i64, RepositoryError> {
        validate_session(session)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        insert_session_record(&transaction, session)?;
        let id = transaction.last_insert_rowid();
        transaction.commit()?;
        Ok(id)
    }

    pub fn update_session(
        &mut self,
        session_id: i64,
        session: &NewSessionRecord<'_>,
    ) -> Result<(), RepositoryError> {
        validate_session(session)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE sessions
             SET stable_id = ?1,
                 project = ?2,
                 category_id = ?3,
                 description = ?4,
                 started_at_utc = ?5,
                 ended_at_utc = ?6,
                 operational_day = ?7,
                 elapsed_seconds = ?8,
                 boundary_utc_offset_seconds = ?9,
                 boundary_start_minutes = ?10,
                 source = ?11
             WHERE id = ?12",
            params![
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
                session_id,
            ],
        )?;
        ensure_changed(changed, "session", session_id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_session(&mut self, session_id: i64) -> Result<(), RepositoryError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed =
            transaction.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        ensure_changed(changed, "session", session_id)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn active_session(&self) -> Result<Option<ActiveSessionRecord>, RepositoryError> {
        query_active_session(&self.connection)
    }

    pub fn switch_active_session(
        &mut self,
        completion: &SessionCompletion<'_>,
        next: &NewActiveSession<'_>,
    ) -> Result<i64, RepositoryError> {
        validate_active(next)?;
        if completion.elapsed_seconds < 0 {
            return Err(RepositoryError::InvalidInput(
                "elapsed seconds cannot be negative".to_string(),
            ));
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active = query_active_session(&transaction)?.ok_or(RepositoryError::NoActiveSession)?;
        if active.stable_id == next.stable_id {
            return Err(RepositoryError::InvalidInput(
                "the next active session must have a new stable identity".to_string(),
            ));
        }
        transaction.execute(
            "INSERT INTO sessions (
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
                source
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                active.stable_id,
                active.project,
                active.category_id,
                active.description,
                active.started_at_utc,
                completion.ended_at_utc,
                completion.operational_day,
                completion.elapsed_seconds,
                completion.boundary_utc_offset_seconds,
                completion.boundary_start_minutes,
                completion.source,
            ],
        )?;
        let completed_id = transaction.last_insert_rowid();
        transaction.execute("DELETE FROM active_session WHERE singleton = 1", [])?;
        insert_active_session(&transaction, next)?;
        transaction.commit()?;
        Ok(completed_id)
    }

    pub fn update_active_description(&mut self, description: &str) -> Result<(), RepositoryError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE active_session SET description = ?1 WHERE singleton = 1",
            params![description],
        )?;
        if changed == 0 {
            return Err(RepositoryError::NoActiveSession);
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn save_checkpoint(
        &mut self,
        checkpoint: &CheckpointRecord,
    ) -> Result<(), RepositoryError> {
        require_non_empty(&checkpoint.detached_at_utc, "detached timestamp")?;
        require_non_empty(&checkpoint.simulation_time_utc, "simulation timestamp")?;
        require_non_empty(&checkpoint.payload_json, "checkpoint payload")?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO runtime_checkpoint (
                singleton,
                status,
                detached_at_utc,
                simulation_time_utc,
                active_session_stable_id,
                payload_json
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(singleton) DO UPDATE SET
                status = excluded.status,
                detached_at_utc = excluded.detached_at_utc,
                simulation_time_utc = excluded.simulation_time_utc,
                active_session_stable_id = excluded.active_session_stable_id,
                payload_json = excluded.payload_json",
            params![
                checkpoint.status.as_str(),
                checkpoint.detached_at_utc,
                checkpoint.simulation_time_utc,
                checkpoint.active_session_stable_id,
                checkpoint.payload_json,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn checkpoint(&self) -> Result<Option<CheckpointRecord>, RepositoryError> {
        query_checkpoint(&self.connection)
    }

    pub fn transition_checkpoint(
        &mut self,
        expected: CheckpointStatus,
        next: CheckpointStatus,
    ) -> Result<(), RepositoryError> {
        if !expected.allows(next) {
            return Err(RepositoryError::InvalidCheckpointTransition {
                from: expected.as_str().to_string(),
                to: next.as_str().to_string(),
            });
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE runtime_checkpoint
             SET status = ?1
             WHERE singleton = 1 AND status = ?2",
            params![next.as_str(), expected.as_str()],
        )?;
        if changed == 0 {
            let actual: Option<String> = transaction
                .query_row(
                    "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
                    [],
                    |row| row.get(0),
                )
                .optional()?;
            return match actual {
                Some(actual) => Err(RepositoryError::CheckpointStatusConflict {
                    expected: expected.as_str().to_string(),
                    actual,
                }),
                None => Err(not_found("checkpoint", 1)),
            };
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn clear_checkpoint(&mut self, expected: CheckpointStatus) -> Result<(), RepositoryError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "DELETE FROM runtime_checkpoint WHERE singleton = 1 AND status = ?1",
            params![expected.as_str()],
        )?;
        if changed == 0 {
            return Err(not_found("checkpoint in expected state", expected.as_str()));
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn save_sand_state(&mut self, state: &SandStateRecord) -> Result<(), RepositoryError> {
        validate_sand_state(state)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO sand_state (
                singleton,
                formation_id,
                quantum_seconds,
                grid_width,
                grid_height,
                payload_json,
                updated_at_utc
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(singleton) DO UPDATE SET
                formation_id = excluded.formation_id,
                quantum_seconds = excluded.quantum_seconds,
                grid_width = excluded.grid_width,
                grid_height = excluded.grid_height,
                payload_json = excluded.payload_json,
                updated_at_utc = excluded.updated_at_utc",
            params![
                state.formation_id,
                state.quantum_seconds,
                state.grid_width,
                state.grid_height,
                state.payload_json,
                state.updated_at_utc,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn sand_state(&self) -> Result<Option<SandStateRecord>, RepositoryError> {
        query_sand_state(&self.connection)
    }

    pub fn insert_sand_snapshot(
        &mut self,
        snapshot: &NewSandSnapshotRecord<'_>,
    ) -> Result<i64, RepositoryError> {
        validate_snapshot(snapshot)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "INSERT INTO sand_snapshots (
                formation_id,
                snapshot_kind,
                operational_day,
                quantum_seconds,
                payload_json,
                captured_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snapshot.formation_id,
                snapshot.snapshot_kind.as_str(),
                snapshot.operational_day,
                snapshot.quantum_seconds,
                snapshot.payload_json,
                snapshot.captured_at_utc,
            ],
        )?;
        let id = transaction.last_insert_rowid();
        transaction.commit()?;
        Ok(id)
    }

    pub fn list_sand_snapshots(&self) -> Result<Vec<SandSnapshotRecord>, RepositoryError> {
        query_sand_snapshots(&self.connection)
    }

    pub fn read_consistent_snapshot(&mut self) -> Result<RepositorySnapshot, RepositoryError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)?;
        let snapshot = RepositorySnapshot {
            categories: query_categories(&transaction, true)?,
            category_tags: query_category_tags(&transaction)?,
            sessions: query_all_sessions(&transaction)?,
            active_session: query_active_session(&transaction)?,
            checkpoint: query_checkpoint(&transaction)?,
            sand_state: query_sand_state(&transaction)?,
            sand_snapshots: query_sand_snapshots(&transaction)?,
            category_lifecycle_receipts: query_category_lifecycle_receipts(&transaction)?,
        };
        transaction.commit()?;
        Ok(snapshot)
    }
}

fn query_category_lifecycle_receipts(
    connection: &Connection,
) -> Result<Vec<CategoryLifecycleReceiptRecord>, RepositoryError> {
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
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn validate_category(category: &NewCategoryRecord<'_>) -> Result<(), RepositoryError> {
    require_non_empty(category.name, "category name")?;
    if category.name.eq_ignore_ascii_case("idle") || category.name.eq_ignore_ascii_case("none") {
        return Err(RepositoryError::InvalidInput(
            "the idle category name is reserved".to_string(),
        ));
    }
    if category.color_index < 0 {
        return Err(RepositoryError::InvalidInput(
            "category color index cannot be negative".to_string(),
        ));
    }
    if !(-1..=1).contains(&category.balance_effect) {
        return Err(RepositoryError::InvalidInput(
            "category balance effect must be -1, 0, or 1".to_string(),
        ));
    }
    Ok(())
}

fn validate_session(session: &NewSessionRecord<'_>) -> Result<(), RepositoryError> {
    require_non_empty(session.stable_id, "session stable identity")?;
    require_non_empty(session.started_at_utc, "session start timestamp")?;
    require_non_empty(session.ended_at_utc, "session end timestamp")?;
    require_non_empty(session.operational_day, "session operational day")?;
    require_non_empty(session.source, "session source")?;
    if session.elapsed_seconds <= 0 {
        return Err(RepositoryError::InvalidInput(
            "ordinary session elapsed seconds must be positive".to_string(),
        ));
    }
    if chrono::FixedOffset::east_opt(session.boundary_utc_offset_seconds).is_none() {
        return Err(RepositoryError::InvalidInput(
            "session boundary UTC offset is unsupported".to_string(),
        ));
    }
    if session.boundary_start_minutes > 1439 {
        return Err(RepositoryError::InvalidInput(
            "session boundary start minute is outside 0..1439".to_string(),
        ));
    }
    Ok(())
}

fn validate_active(active: &NewActiveSession<'_>) -> Result<(), RepositoryError> {
    require_non_empty(active.stable_id, "active-session stable identity")?;
    require_non_empty(active.started_at_utc, "active-session start timestamp")?;
    require_non_empty(active.recovery_kind, "active-session recovery kind")?;
    Ok(())
}

fn validate_sand_state(state: &SandStateRecord) -> Result<(), RepositoryError> {
    require_non_empty(&state.formation_id, "formation identity")?;
    require_non_empty(&state.payload_json, "sand payload")?;
    require_non_empty(&state.updated_at_utc, "sand update timestamp")?;
    if state.quantum_seconds <= 0 {
        return Err(RepositoryError::InvalidInput(
            "sand quantum must be positive".to_string(),
        ));
    }
    if state.grid_width < 0 || state.grid_height < 0 {
        return Err(RepositoryError::InvalidInput(
            "sand grid dimensions cannot be negative".to_string(),
        ));
    }
    Ok(())
}

fn validate_snapshot(snapshot: &NewSandSnapshotRecord<'_>) -> Result<(), RepositoryError> {
    require_non_empty(snapshot.formation_id, "snapshot formation identity")?;
    require_non_empty(snapshot.payload_json, "snapshot payload")?;
    require_non_empty(snapshot.captured_at_utc, "snapshot capture timestamp")?;
    if snapshot.quantum_seconds <= 0 {
        return Err(RepositoryError::InvalidInput(
            "snapshot quantum must be positive".to_string(),
        ));
    }
    if matches!(
        snapshot.snapshot_kind,
        SnapshotKind::Daily | SnapshotKind::DailyContribution
    ) && snapshot.operational_day.is_none()
    {
        return Err(RepositoryError::InvalidInput(
            "daily snapshots require an operational day".to_string(),
        ));
    }
    Ok(())
}

fn normalize_tags(tags: &[String]) -> Result<Vec<String>, RepositoryError> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::with_capacity(tags.len());
    for tag in tags {
        let tag = tag.trim();
        require_non_empty(tag, "category tag")?;
        if !seen.insert(tag.to_string()) {
            return Err(RepositoryError::InvalidInput(format!(
                "duplicate category tag '{tag}'"
            )));
        }
        normalized.push(tag.to_string());
    }
    Ok(normalized)
}

fn require_non_empty(value: &str, label: &str) -> Result<(), RepositoryError> {
    if value.trim().is_empty() {
        return Err(RepositoryError::InvalidInput(format!("{label} is empty")));
    }
    Ok(())
}

fn ensure_changed<T: ToString>(
    changed: usize,
    entity: &'static str,
    identity: T,
) -> Result<(), RepositoryError> {
    if changed == 0 {
        return Err(not_found(entity, identity));
    }
    Ok(())
}

fn not_found<T: ToString>(entity: &'static str, identity: T) -> RepositoryError {
    RepositoryError::NotFound {
        entity,
        identity: identity.to_string(),
    }
}

fn insert_session_record(
    connection: &Connection,
    session: &NewSessionRecord<'_>,
) -> Result<(), RepositoryError> {
    connection.execute(
        "INSERT INTO sessions (
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
            source
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
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
    Ok(())
}

fn insert_active_session(
    connection: &Connection,
    active: &NewActiveSession<'_>,
) -> Result<(), RepositoryError> {
    connection.execute(
        "INSERT INTO active_session (
            singleton,
            stable_id,
            project,
            category_id,
            description,
            started_at_utc,
            recovery_kind
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
    Ok(())
}

fn query_categories(
    connection: &Connection,
    include_archived: bool,
) -> Result<Vec<CategoryRecord>, RepositoryError> {
    let sql = if include_archived {
        "SELECT id, name, description, color_index, balance_effect, archived_at_utc
         FROM categories ORDER BY id"
    } else {
        "SELECT id, name, description, color_index, balance_effect, archived_at_utc
         FROM categories WHERE archived_at_utc IS NULL ORDER BY id"
    };
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([], map_category)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn query_category_tags(
    connection: &Connection,
) -> Result<BTreeMap<i64, Vec<String>>, RepositoryError> {
    let mut statement = connection.prepare(
        "SELECT category_id, tag
         FROM category_tags
         ORDER BY category_id, ordinal",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut tags = BTreeMap::new();
    for row in rows {
        let (category_id, tag) = row?;
        tags.entry(category_id).or_insert_with(Vec::new).push(tag);
    }
    Ok(tags)
}

fn query_all_sessions(connection: &Connection) -> Result<Vec<SessionRecord>, RepositoryError> {
    let mut statement = connection.prepare(
        "SELECT id, stable_id, project, category_id, description,
                started_at_utc, ended_at_utc, operational_day,
                elapsed_seconds, boundary_utc_offset_seconds,
                boundary_start_minutes, source
         FROM sessions
         ORDER BY operational_day, started_at_utc, id",
    )?;
    let rows = statement.query_map([], map_session)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn session_overlaps_range(session: &SessionRecord, start: NaiveDate, end: NaiveDate) -> bool {
    let fallback = || {
        NaiveDate::parse_from_str(&session.operational_day, "%Y-%m-%d")
            .is_ok_and(|day| day >= start && day <= end)
    };
    let (Some(offset), Some(start_minutes)) = (
        session.boundary_utc_offset_seconds,
        session.boundary_start_minutes,
    ) else {
        return fallback();
    };
    let Ok(offset) = i32::try_from(offset) else {
        return fallback();
    };
    let Ok(start_minutes) = u16::try_from(start_minutes) else {
        return fallback();
    };
    let Ok(started_at_utc) = DateTime::parse_from_rfc3339(&session.started_at_utc)
        .map(|value| value.with_timezone(&Utc))
    else {
        return fallback();
    };
    let Ok(ended_at_utc) =
        DateTime::parse_from_rfc3339(&session.ended_at_utc).map(|value| value.with_timezone(&Utc))
    else {
        return fallback();
    };
    let Ok(elapsed_seconds) = usize::try_from(session.elapsed_seconds) else {
        return fallback();
    };
    temporal::allocate_operational_day_slices(
        started_at_utc,
        ended_at_utc,
        elapsed_seconds,
        OperationalDayPolicy {
            utc_offset_seconds: offset,
            start_minutes,
        },
    )
    .map(|slices| {
        slices
            .iter()
            .any(|slice| slice.operational_day >= start && slice.operational_day <= end)
    })
    .unwrap_or_else(|_| fallback())
}

fn query_active_session(
    connection: &Connection,
) -> Result<Option<ActiveSessionRecord>, RepositoryError> {
    Ok(connection
        .query_row(
            "SELECT stable_id, project, category_id, description,
                    started_at_utc, recovery_kind
             FROM active_session WHERE singleton = 1",
            [],
            |row| {
                Ok(ActiveSessionRecord {
                    stable_id: row.get(0)?,
                    project: row.get(1)?,
                    category_id: row.get(2)?,
                    description: row.get(3)?,
                    started_at_utc: row.get(4)?,
                    recovery_kind: row.get(5)?,
                })
            },
        )
        .optional()?)
}

fn query_checkpoint(connection: &Connection) -> Result<Option<CheckpointRecord>, RepositoryError> {
    let raw = connection
        .query_row(
            "SELECT status, detached_at_utc, simulation_time_utc,
                    active_session_stable_id, payload_json
             FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    raw.map(
        |(status, detached_at_utc, simulation_time_utc, active_id, payload_json)| {
            Ok(CheckpointRecord {
                status: CheckpointStatus::parse(&status)?,
                detached_at_utc,
                simulation_time_utc,
                active_session_stable_id: active_id,
                payload_json,
            })
        },
    )
    .transpose()
}

fn query_sand_state(connection: &Connection) -> Result<Option<SandStateRecord>, RepositoryError> {
    Ok(connection
        .query_row(
            "SELECT formation_id, quantum_seconds, grid_width, grid_height,
                    payload_json, updated_at_utc
             FROM sand_state WHERE singleton = 1",
            [],
            |row| {
                Ok(SandStateRecord {
                    formation_id: row.get(0)?,
                    quantum_seconds: row.get(1)?,
                    grid_width: row.get(2)?,
                    grid_height: row.get(3)?,
                    payload_json: row.get(4)?,
                    updated_at_utc: row.get(5)?,
                })
            },
        )
        .optional()?)
}

fn query_sand_snapshots(
    connection: &Connection,
) -> Result<Vec<SandSnapshotRecord>, RepositoryError> {
    let mut statement = connection.prepare(
        "SELECT id, formation_id, snapshot_kind, operational_day,
                quantum_seconds, payload_json, captured_at_utc
         FROM sand_snapshots
         ORDER BY captured_at_utc, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;
    let mut snapshots = Vec::new();
    for row in rows {
        let (id, formation_id, kind, operational_day, quantum, payload, captured) = row?;
        snapshots.push(SandSnapshotRecord {
            id,
            formation_id,
            snapshot_kind: SnapshotKind::parse(&kind)?,
            operational_day,
            quantum_seconds: quantum,
            payload_json: payload,
            captured_at_utc: captured,
        });
    }
    Ok(snapshots)
}

fn map_category(row: &Row<'_>) -> rusqlite::Result<CategoryRecord> {
    Ok(CategoryRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        color_index: row.get(3)?,
        balance_effect: row.get(4)?,
        archived_at_utc: row.get(5)?,
    })
}

fn map_session(row: &Row<'_>) -> rusqlite::Result<SessionRecord> {
    Ok(SessionRecord {
        id: row.get(0)?,
        stable_id: row.get(1)?,
        project: row.get(2)?,
        category_id: row.get(3)?,
        description: row.get(4)?,
        started_at_utc: row.get(5)?,
        ended_at_utc: row.get(6)?,
        operational_day: row.get(7)?,
        elapsed_seconds: row.get(8)?,
        boundary_utc_offset_seconds: row.get(9)?,
        boundary_start_minutes: row.get(10)?,
        source: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn category(name: &str) -> NewCategoryRecord<'_> {
        NewCategoryRecord {
            name,
            description: "",
            color_index: 1,
            balance_effect: 1,
        }
    }

    fn session<'a>(stable_id: &'a str, category_id: i64, day: &'a str) -> NewSessionRecord<'a> {
        NewSessionRecord {
            stable_id,
            project: "Study",
            category_id,
            description: "Read",
            started_at_utc: "2026-08-01T16:00:00Z",
            ended_at_utc: "2026-08-01T17:00:00Z",
            operational_day: day,
            elapsed_seconds: 3600,
            boundary_utc_offset_seconds: -21600,
            boundary_start_minutes: 360,
            source: "test",
        }
    }

    fn active<'a>(stable_id: &'a str, category_id: i64) -> NewActiveSession<'a> {
        NewActiveSession {
            stable_id,
            project: "Study",
            category_id,
            description: "Continue",
            started_at_utc: "2026-08-01T17:00:00Z",
            recovery_kind: "live",
        }
    }

    #[test]
    fn categories_and_tags_support_archive_without_losing_history() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let category_id = repository.create_category(&category("Study")).unwrap();
        repository
            .replace_category_tags(category_id, &["focus".to_string(), "reading".to_string()])
            .unwrap();
        repository
            .insert_session(&session("session-1", category_id, "2026-08-01"))
            .unwrap();
        repository
            .archive_category(category_id, "2026-08-02T00:00:00Z")
            .unwrap();

        assert_eq!(repository.list_categories(false).unwrap().len(), 1);
        let all = repository.list_categories(true).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(
            all[1].archived_at_utc.as_deref(),
            Some("2026-08-02T00:00:00Z")
        );
        assert_eq!(
            repository.category_tags().unwrap().get(&category_id),
            Some(&vec!["focus".to_string(), "reading".to_string()])
        );
        assert_eq!(
            repository.list_sessions().unwrap()[0].category_id,
            category_id
        );

        repository.restore_category(category_id).unwrap();
        assert_eq!(repository.list_categories(false).unwrap().len(), 2);
    }

    #[test]
    fn bounded_session_query_includes_canonical_rows_that_overlap_the_day() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let category_id = repository.create_category(&category("Study")).unwrap();
        let mut crossing = session("crossing", category_id, "2026-08-02");
        crossing.started_at_utc = "2026-08-02T11:30:00Z";
        crossing.ended_at_utc = "2026-08-02T12:30:00Z";
        crossing.elapsed_seconds = 3600;
        repository.insert_session(&crossing).unwrap();

        let previous = repository
            .list_sessions_between("2026-08-01", "2026-08-01")
            .unwrap();
        let current = repository
            .list_sessions_between("2026-08-02", "2026-08-02")
            .unwrap();

        assert_eq!(previous.len(), 1);
        assert_eq!(current.len(), 1);
        assert_eq!(previous[0].stable_id, "crossing");
    }

    #[test]
    fn session_crud_and_bounds_are_repository_owned() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let category_id = repository.create_category(&category("Study")).unwrap();
        let first = repository
            .insert_session(&session("session-1", category_id, "2026-08-01"))
            .unwrap();
        let mut second = session("session-2", category_id, "2026-08-02");
        second.started_at_utc = "2026-08-02T16:00:00Z";
        second.ended_at_utc = "2026-08-02T17:00:00Z";
        repository.insert_session(&second).unwrap();

        let mut edited = session("session-1-edited", category_id, "2026-08-01");
        edited.description = "Edited";
        repository.update_session(first, &edited).unwrap();

        let bounded = repository
            .list_sessions_between("2026-08-01", "2026-08-01")
            .unwrap();
        assert_eq!(bounded.len(), 1);
        assert_eq!(bounded[0].description, "Edited");
        assert_eq!(bounded[0].stable_id, "session-1-edited");

        repository.delete_session(first).unwrap();
        assert_eq!(repository.list_sessions().unwrap().len(), 1);
    }

    #[test]
    fn active_switch_is_atomic_and_preserves_old_active_on_failure() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let category_id = repository.create_category(&category("Study")).unwrap();
        repository
            .start_session(&active("active-1", category_id))
            .unwrap();
        let completion = SessionCompletion {
            ended_at_utc: "2026-08-01T18:00:00Z",
            operational_day: "2026-08-01",
            elapsed_seconds: 3600,
            boundary_utc_offset_seconds: -21600,
            boundary_start_minutes: 360,
            source: "runtime",
        };
        let missing_category = active("active-2", 999);
        repository
            .switch_active_session(&completion, &missing_category)
            .unwrap_err();
        assert_eq!(repository.list_sessions().unwrap().len(), 0);
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "active-1"
        );

        repository
            .switch_active_session(&completion, &active("active-2", category_id))
            .unwrap();
        assert_eq!(repository.list_sessions().unwrap().len(), 1);
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "active-2"
        );
    }

    #[test]
    fn checkpoint_transitions_are_explicit_and_guarded() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        repository
            .save_checkpoint(&CheckpointRecord {
                status: CheckpointStatus::Pending,
                detached_at_utc: "2026-08-01T18:00:00Z".to_string(),
                simulation_time_utc: "2026-08-01T18:00:00Z".to_string(),
                active_session_stable_id: None,
                payload_json: "{}".to_string(),
            })
            .unwrap();
        repository
            .transition_checkpoint(CheckpointStatus::Pending, CheckpointStatus::Recovering)
            .unwrap();
        assert_eq!(
            repository.checkpoint().unwrap().unwrap().status,
            CheckpointStatus::Recovering
        );
        repository
            .transition_checkpoint(CheckpointStatus::Pending, CheckpointStatus::Recovering)
            .unwrap_err();
        repository
            .transition_checkpoint(CheckpointStatus::Recovering, CheckpointStatus::Committed)
            .unwrap();
        repository
            .clear_checkpoint(CheckpointStatus::Committed)
            .unwrap();
        assert!(repository.checkpoint().unwrap().is_none());
    }

    #[test]
    fn sand_state_and_snapshots_round_trip_through_repository() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let state = SandStateRecord {
            formation_id: "formation-1".to_string(),
            quantum_seconds: 1,
            grid_width: 4,
            grid_height: 3,
            payload_json: "{\"grains\":[]}".to_string(),
            updated_at_utc: "2026-08-01T18:00:00Z".to_string(),
        };
        repository.save_sand_state(&state).unwrap();
        let snapshot_id = repository
            .insert_sand_snapshot(&NewSandSnapshotRecord {
                formation_id: "formation-1",
                snapshot_kind: SnapshotKind::Daily,
                operational_day: Some("2026-08-01"),
                quantum_seconds: 1,
                payload_json: "{\"grains\":[]}",
                captured_at_utc: "2026-08-02T06:00:00Z",
            })
            .unwrap();

        assert_eq!(repository.sand_state().unwrap(), Some(state));
        let snapshots = repository.list_sand_snapshots().unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, snapshot_id);
        assert_eq!(snapshots[0].snapshot_kind, SnapshotKind::Daily);
    }
}
