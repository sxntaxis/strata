use std::{collections::BTreeSet, fmt::Write as _, path::Path};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use crate::{
    category_lifecycle::{
        SedimentCategoryReferences, checkpoint_has_transition_receipt,
        count_checkpoint_category_references, count_sand_state_category, count_snapshot_category,
        reassign_checkpoint_category, reassign_sand_state_category, reassign_snapshot_category,
    },
    domain::{CategoryId, OperationalDayPolicy},
    sand::{
        DailySedimentSlice, SandState, SedimentSnapshot, SedimentSnapshotKind,
        daily_contribution_from_slices,
    },
    temporal,
};

use super::{SqliteRepository, runtime_coordination};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CategoryIdentitySnapshot {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub color_index: i64,
    pub balance_effect: i64,
    pub archived_at_utc: Option<String>,
    pub sort_order: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CategoryReferenceCounts {
    pub completed_sessions: u64,
    pub active_sessions: u64,
    pub tags: u64,
    pub sand_placed: u64,
    pub sand_pending: u64,
    pub snapshot_placed: u64,
    pub snapshot_pending: u64,
    pub checkpoint_references: u64,
}

impl CategoryReferenceCounts {
    pub(crate) fn total(&self) -> Result<u64, String> {
        [
            self.completed_sessions,
            self.active_sessions,
            self.tags,
            self.sand_placed,
            self.sand_pending,
            self.snapshot_placed,
            self.snapshot_pending,
            self.checkpoint_references,
        ]
        .into_iter()
        .try_fold(0u64, |total, count| {
            total
                .checked_add(count)
                .ok_or_else(|| "category reference total exceeds u64".to_string())
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CategoryLifecyclePreview {
    pub source: CategoryIdentitySnapshot,
    pub target: Option<CategoryIdentitySnapshot>,
    pub references: CategoryReferenceCounts,
    pub checkpoint_status: Option<String>,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CategoryLifecycleRequest<'a> {
    pub source_category_id: i64,
    pub target_category_id: Option<i64>,
    pub expected_revision: &'a str,
    pub applied_at_utc: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CategoryLifecycleReceipt {
    pub operation_id: String,
    pub operation_kind: String,
    pub source: CategoryIdentitySnapshot,
    pub target: Option<CategoryIdentitySnapshot>,
    pub preview_revision: String,
    pub references: CategoryReferenceCounts,
    pub applied_at_utc: String,
    pub already_applied: bool,
}

pub(crate) fn identity_high_watermark_at(database_path: &Path) -> Result<u64, String> {
    let repository = SqliteRepository::open(database_path).map_err(|error| error.to_string())?;
    let maximum: i64 = repository
        .connection
        .query_row(
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
        )
        .map_err(|error| error.to_string())?;
    u64::try_from(maximum)
        .map_err(|_| "SQLite category identity high-watermark is negative".to_string())
}

pub(crate) fn preview_at(
    database_path: &Path,
    source_category_id: CategoryId,
    target_category_id: Option<CategoryId>,
) -> Result<CategoryLifecyclePreview, String> {
    let repository = SqliteRepository::open(database_path).map_err(|error| error.to_string())?;
    preview(
        &repository,
        i64::try_from(source_category_id.0)
            .map_err(|_| "source category identity exceeds SQLite range".to_string())?,
        target_category_id
            .map(|target| i64::try_from(target.0))
            .transpose()
            .map_err(|_| "target category identity exceeds SQLite range".to_string())?,
    )
}

pub(crate) fn apply_at(
    database_path: &Path,
    source_category_id: CategoryId,
    target_category_id: Option<CategoryId>,
    expected_revision: &str,
    applied_at_utc: DateTime<Utc>,
) -> Result<CategoryLifecycleReceipt, String> {
    let mut repository =
        SqliteRepository::open(database_path).map_err(|error| error.to_string())?;
    apply(
        &mut repository,
        CategoryLifecycleRequest {
            source_category_id: i64::try_from(source_category_id.0)
                .map_err(|_| "source category identity exceeds SQLite range".to_string())?,
            target_category_id: target_category_id
                .map(|target| i64::try_from(target.0))
                .transpose()
                .map_err(|_| "target category identity exceeds SQLite range".to_string())?,
            expected_revision,
            applied_at_utc: &applied_at_utc.to_rfc3339(),
        },
    )
}

pub(crate) fn preview(
    repository: &SqliteRepository,
    source_category_id: i64,
    target_category_id: Option<i64>,
) -> Result<CategoryLifecyclePreview, String> {
    preview_on(
        &repository.connection,
        source_category_id,
        target_category_id,
    )
}

pub(crate) fn apply(
    repository: &mut SqliteRepository,
    request: CategoryLifecycleRequest<'_>,
) -> Result<CategoryLifecycleReceipt, String> {
    validate_request(&request)?;
    let operation_id = operation_id(
        request.source_category_id,
        request.target_category_id,
        request.expected_revision,
    );
    if let Some(mut receipt) = query_receipt(&repository.connection, &operation_id)? {
        receipt.already_applied = true;
        return Ok(receipt);
    }

    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    if let Some(mut receipt) = query_receipt(&transaction, &operation_id)? {
        receipt.already_applied = true;
        return Ok(receipt);
    }

    let preview = preview_on(
        &transaction,
        request.source_category_id,
        request.target_category_id,
    )?;
    if preview.revision != request.expected_revision {
        return Err(format!(
            "category lifecycle preview is stale; expected {}, found {}",
            request.expected_revision, preview.revision
        ));
    }

    if request.target_category_id.is_none() && preview.references.total()? != 0 {
        return Err(format!(
            "category {} still has {} references and cannot be permanently deleted",
            request.source_category_id,
            preview.references.total()?
        ));
    }

    let staged = if let Some(target_category_id) = request.target_category_id {
        Some(stage_merge(
            &transaction,
            request.source_category_id,
            target_category_id,
            preview.checkpoint_status.as_deref(),
        )?)
    } else {
        None
    };

    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "before-write")
        .map_err(|error| error.to_string())?;

    if let Some(target_category_id) = request.target_category_id {
        transaction
            .execute(
                "UPDATE sessions SET category_id = ?1 WHERE category_id = ?2",
                params![target_category_id, request.source_category_id],
            )
            .map_err(|error| error.to_string())?;
    }
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "sessions")
        .map_err(|error| error.to_string())?;

    if let Some(target_category_id) = request.target_category_id {
        transaction
            .execute(
                "UPDATE active_session SET category_id = ?1 WHERE category_id = ?2",
                params![target_category_id, request.source_category_id],
            )
            .map_err(|error| error.to_string())?;
    }
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "active")
        .map_err(|error| error.to_string())?;

    if let Some(staged) = staged.as_ref() {
        replace_merged_tags(
            &transaction,
            request.source_category_id,
            request.target_category_id.expect("merge target exists"),
            &staged.tags,
        )?;
    }
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "tags")
        .map_err(|error| error.to_string())?;

    if let Some(staged) = staged.as_ref()
        && let Some(payload_json) = staged.sand_state_json.as_deref()
    {
        transaction
            .execute(
                "UPDATE sand_state SET payload_json = ?1 WHERE singleton = 1",
                params![payload_json],
            )
            .map_err(|error| error.to_string())?;
    }
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "sand")
        .map_err(|error| error.to_string())?;

    if let Some(staged) = staged.as_ref() {
        for snapshot in &staged.snapshots {
            match snapshot.payload_json.as_deref() {
                Some(payload_json) => {
                    transaction
                        .execute(
                            "UPDATE sand_snapshots SET payload_json = ?1 WHERE id = ?2",
                            params![payload_json, snapshot.id],
                        )
                        .map_err(|error| error.to_string())?;
                }
                None => {
                    transaction
                        .execute(
                            "DELETE FROM sand_snapshots WHERE id = ?1",
                            params![snapshot.id],
                        )
                        .map_err(|error| error.to_string())?;
                }
            }
        }
    }
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "snapshots")
        .map_err(|error| error.to_string())?;

    if let Some(staged) = staged.as_ref()
        && let Some(payload_json) = staged.checkpoint_json.as_deref()
    {
        transaction
            .execute(
                "UPDATE runtime_checkpoint SET payload_json = ?1 WHERE singleton = 1",
                params![payload_json],
            )
            .map_err(|error| error.to_string())?;
    }
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "checkpoint")
        .map_err(|error| error.to_string())?;

    if request.target_category_id.is_some() {
        let residual = reference_counts_on(&transaction, request.source_category_id)?;
        if residual.total()? != 0 {
            return Err(format!(
                "category reassignment left {} residual source references",
                residual.total()?
            ));
        }
    }

    let deleted = transaction
        .execute(
            "DELETE FROM categories WHERE id = ?1",
            params![request.source_category_id],
        )
        .map_err(|error| error.to_string())?;
    if deleted != 1 {
        return Err(format!(
            "source category {} changed concurrently before deletion",
            request.source_category_id
        ));
    }
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "category")
        .map_err(|error| error.to_string())?;

    let receipt = CategoryLifecycleReceipt {
        operation_id: operation_id.clone(),
        operation_kind: if request.target_category_id.is_some() {
            "merge".to_string()
        } else {
            "delete".to_string()
        },
        source: preview.source,
        target: preview.target,
        preview_revision: preview.revision,
        references: preview.references,
        applied_at_utc: request.applied_at_utc.to_string(),
        already_applied: false,
    };
    insert_receipt(&transaction, &receipt)?;
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "receipt")
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("category-lifecycle", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(receipt)
}

fn validate_request(request: &CategoryLifecycleRequest<'_>) -> Result<(), String> {
    if request.source_category_id == 0 {
        return Err("the reserved idle category cannot be merged or deleted".to_string());
    }
    if request.target_category_id == Some(request.source_category_id) {
        return Err("category merge source and target must differ".to_string());
    }
    if request.expected_revision.trim().is_empty() {
        return Err("category lifecycle preview revision cannot be empty".to_string());
    }
    if request.applied_at_utc.trim().is_empty() {
        return Err("category lifecycle application timestamp cannot be empty".to_string());
    }
    DateTime::parse_from_rfc3339(request.applied_at_utc)
        .map_err(|error| format!("invalid category lifecycle timestamp: {error}"))?;
    Ok(())
}

fn preview_on(
    connection: &Connection,
    source_category_id: i64,
    target_category_id: Option<i64>,
) -> Result<CategoryLifecyclePreview, String> {
    if source_category_id == 0 {
        return Err("the reserved idle category cannot be merged or deleted".to_string());
    }
    if target_category_id == Some(source_category_id) {
        return Err("category merge source and target must differ".to_string());
    }
    let source = query_category(connection, source_category_id)?
        .ok_or_else(|| format!("source category {source_category_id} does not exist"))?;
    let target = target_category_id
        .map(|category_id| {
            query_category(connection, category_id)?
                .ok_or_else(|| format!("target category {category_id} does not exist"))
        })
        .transpose()?;
    let references = reference_counts_on(connection, source_category_id)?;
    let checkpoint_status = connection
        .query_row(
            "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let revision = preview_revision(connection, &source, target.as_ref(), &references)?;
    Ok(CategoryLifecyclePreview {
        source,
        target,
        references,
        checkpoint_status,
        revision,
    })
}

fn query_category(
    connection: &Connection,
    category_id: i64,
) -> Result<Option<CategoryIdentitySnapshot>, String> {
    connection
        .query_row(
            "SELECT id, name, description, color_index, balance_effect,
                    archived_at_utc, sort_order
             FROM categories WHERE id = ?1",
            params![category_id],
            |row| {
                Ok(CategoryIdentitySnapshot {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    color_index: row.get(3)?,
                    balance_effect: row.get(4)?,
                    archived_at_utc: row.get(5)?,
                    sort_order: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn reference_counts_on(
    connection: &Connection,
    source_category_id: i64,
) -> Result<CategoryReferenceCounts, String> {
    let completed_sessions = count_query(
        connection,
        "SELECT count(*) FROM sessions WHERE category_id = ?1",
        source_category_id,
    )?;
    let active_sessions = count_query(
        connection,
        "SELECT count(*) FROM active_session WHERE category_id = ?1",
        source_category_id,
    )?;
    let tags = count_query(
        connection,
        "SELECT count(*) FROM category_tags WHERE category_id = ?1",
        source_category_id,
    )?;
    let source_u64 = u64::try_from(source_category_id)
        .map_err(|_| format!("category ID {source_category_id} is outside u64"))?;

    let sand = connection
        .query_row(
            "SELECT payload_json FROM sand_state WHERE singleton = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .map(|payload| {
            let state: SandState = serde_json::from_str(&payload)
                .map_err(|error| format!("invalid canonical sand state JSON: {error}"))?;
            count_sand_state_category(&state, source_u64)
        })
        .transpose()?
        .unwrap_or_default();

    let mut snapshot = SedimentCategoryReferences::default();
    let mut statement = connection
        .prepare("SELECT id, payload_json FROM sand_snapshots ORDER BY id")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (id, payload) = row.map_err(|error| error.to_string())?;
        let references = count_snapshot_payload(&payload, source_u64)
            .map_err(|error| format!("snapshot {id}: {error}"))?;
        snapshot.placed = snapshot
            .placed
            .checked_add(references.placed)
            .ok_or_else(|| "snapshot placed reference count exceeds u64".to_string())?;
        snapshot.pending = snapshot
            .pending
            .checked_add(references.pending)
            .ok_or_else(|| "snapshot pending reference count exceeds u64".to_string())?;
    }

    let checkpoint_references = connection
        .query_row(
            "SELECT payload_json FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .map(|payload| count_checkpoint_category_references(&payload, source_u64))
        .transpose()?
        .unwrap_or(0);

    Ok(CategoryReferenceCounts {
        completed_sessions,
        active_sessions,
        tags,
        sand_placed: sand.placed,
        sand_pending: sand.pending,
        snapshot_placed: snapshot.placed,
        snapshot_pending: snapshot.pending,
        checkpoint_references,
    })
}

fn count_query(connection: &Connection, sql: &str, category_id: i64) -> Result<u64, String> {
    let count: i64 = connection
        .query_row(sql, params![category_id], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    u64::try_from(count).map_err(|_| format!("negative reference count {count}"))
}

fn count_snapshot_payload(
    payload_json: &str,
    source_category_id: u64,
) -> Result<SedimentCategoryReferences, String> {
    if let Ok(snapshot) = serde_json::from_str::<SedimentSnapshot>(payload_json) {
        return count_snapshot_category(&snapshot, source_category_id);
    }
    let state: SandState = serde_json::from_str(payload_json)
        .map_err(|error| format!("unsupported sediment snapshot payload: {error}"))?;
    count_sand_state_category(&state, source_category_id)
}

fn preview_revision(
    connection: &Connection,
    source: &CategoryIdentitySnapshot,
    target: Option<&CategoryIdentitySnapshot>,
    references: &CategoryReferenceCounts,
) -> Result<String, String> {
    let mut material = String::new();
    let schema_version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    writeln!(&mut material, "schema={schema_version}").map_err(|error| error.to_string())?;
    writeln!(
        &mut material,
        "source={}",
        serde_json::to_string(source).map_err(|error| error.to_string())?
    )
    .map_err(|error| error.to_string())?;
    writeln!(
        &mut material,
        "target={}",
        serde_json::to_string(&target).map_err(|error| error.to_string())?
    )
    .map_err(|error| error.to_string())?;
    writeln!(
        &mut material,
        "references={}",
        serde_json::to_string(references).map_err(|error| error.to_string())?
    )
    .map_err(|error| error.to_string())?;

    append_rows(
        connection,
        &mut material,
        "SELECT id, stable_id, project, category_id, description, started_at_utc,
                ended_at_utc, operational_day, elapsed_seconds,
                boundary_utc_offset_seconds, boundary_start_minutes, source
         FROM sessions ORDER BY id",
        12,
        "session",
    )?;
    append_rows(
        connection,
        &mut material,
        "SELECT singleton, stable_id, project, category_id, description,
                started_at_utc, recovery_kind
         FROM active_session ORDER BY singleton",
        7,
        "active",
    )?;
    append_rows(
        connection,
        &mut material,
        "SELECT category_id, ordinal, tag
         FROM category_tags ORDER BY category_id, ordinal",
        3,
        "tag",
    )?;
    append_rows(
        connection,
        &mut material,
        "SELECT singleton, formation_id, quantum_seconds, grid_width, grid_height,
                payload_json, updated_at_utc
         FROM sand_state ORDER BY singleton",
        7,
        "sand",
    )?;
    append_rows(
        connection,
        &mut material,
        "SELECT id, formation_id, snapshot_kind, operational_day, quantum_seconds,
                payload_json, captured_at_utc
         FROM sand_snapshots ORDER BY id",
        7,
        "snapshot",
    )?;
    append_rows(
        connection,
        &mut material,
        "SELECT singleton, status, detached_at_utc, simulation_time_utc,
                active_session_stable_id, payload_json
         FROM runtime_checkpoint ORDER BY singleton",
        6,
        "checkpoint",
    )?;
    append_rows(
        connection,
        &mut material,
        "SELECT id, name, description, color_index, balance_effect,
                archived_at_utc, sort_order
         FROM categories ORDER BY id",
        7,
        "category",
    )?;
    Ok(format!("{:016x}", fnv1a(material.as_bytes())))
}

fn append_rows(
    connection: &Connection,
    material: &mut String,
    sql: &str,
    column_count: usize,
    label: &str,
) -> Result<(), String> {
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let mut values = Vec::with_capacity(column_count);
        for index in 0..column_count {
            let value = row.get_ref(index).map_err(|error| error.to_string())?;
            let encoded = match value {
                rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                rusqlite::types::ValueRef::Integer(value) => serde_json::Value::from(value),
                rusqlite::types::ValueRef::Real(value) => serde_json::Value::from(value),
                rusqlite::types::ValueRef::Text(value) => serde_json::Value::from(
                    std::str::from_utf8(value)
                        .map_err(|error| format!("invalid UTF-8 in preview material: {error}"))?,
                ),
                rusqlite::types::ValueRef::Blob(value) => {
                    serde_json::Value::from(format!("blob:{}", stable_bytes(value)))
                }
            };
            values.push(encoded);
        }
        writeln!(
            material,
            "{label}={}",
            serde_json::to_string(&values).map_err(|error| error.to_string())?
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn stable_bytes(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a(bytes))
}

#[derive(Debug)]
struct StagedMerge {
    tags: Vec<String>,
    sand_state_json: Option<String>,
    snapshots: Vec<StagedSnapshot>,
    checkpoint_json: Option<String>,
}

#[derive(Debug)]
struct StagedSnapshot {
    id: i64,
    payload_json: Option<String>,
}

fn stage_merge(
    connection: &Connection,
    source_category_id: i64,
    target_category_id: i64,
    checkpoint_status: Option<&str>,
) -> Result<StagedMerge, String> {
    if matches!(checkpoint_status, Some("recovering" | "quarantined")) {
        return Err(format!(
            "runtime checkpoint is {}; category reassignment requires pending, committed, or absent evidence",
            checkpoint_status.unwrap_or("unknown")
        ));
    }
    let source_u64 = u64::try_from(source_category_id)
        .map_err(|_| format!("category ID {source_category_id} is outside u64"))?;
    let target_u64 = u64::try_from(target_category_id)
        .map_err(|_| format!("category ID {target_category_id} is outside u64"))?;

    let tags = merged_tags(connection, source_category_id, target_category_id)?;
    let sand_state_json = connection
        .query_row(
            "SELECT payload_json FROM sand_state WHERE singleton = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .map(|payload| {
            let mut state: SandState = serde_json::from_str(&payload)
                .map_err(|error| format!("invalid canonical sand state JSON: {error}"))?;
            let before = sediment_mass(&state)?;
            reassign_sand_state_category(&mut state, source_u64, target_u64)?;
            let after = sediment_mass(&state)?;
            if before != after {
                return Err(
                    "canonical sediment mass changed during category reassignment".to_string(),
                );
            }
            serde_json::to_string(&state)
                .map_err(|error| format!("cannot serialize reassigned sand state: {error}"))
        })
        .transpose()?;

    let sessions = staged_sessions(connection, source_category_id, target_category_id)?;
    let snapshots = stage_snapshots(connection, source_u64, target_u64, &sessions)?;
    let checkpoint_json = connection
        .query_row(
            "SELECT payload_json FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .map(|payload| {
            if checkpoint_has_transition_receipt(&payload)? {
                return Err(
                    "runtime checkpoint carries unresolved transition custody; category reassignment is blocked"
                        .to_string(),
                );
            }
            let (updated, _) = reassign_checkpoint_category(&payload, source_u64, target_u64)?;
            Ok::<String, String>(updated)
        })
        .transpose()?;

    Ok(StagedMerge {
        tags,
        sand_state_json,
        snapshots,
        checkpoint_json,
    })
}

fn sediment_mass(state: &SandState) -> Result<u64, String> {
    let placed = u64::try_from(state.grains.len())
        .map_err(|_| "placed sediment mass exceeds u64".to_string())?;
    let legacy = u64::try_from(state.pending_grains.len())
        .map_err(|_| "legacy pending sediment mass exceeds u64".to_string())?;
    let runs = state.pending_runs.iter().try_fold(0u64, |total, run| {
        let count = u64::try_from(run.count)
            .map_err(|_| "pending sediment mass exceeds u64".to_string())?;
        total
            .checked_add(count)
            .ok_or_else(|| "pending sediment mass exceeds u64".to_string())
    })?;
    placed
        .checked_add(legacy)
        .and_then(|total| total.checked_add(runs))
        .ok_or_else(|| "total sediment mass exceeds u64".to_string())
}

fn merged_tags(
    connection: &Connection,
    source_category_id: i64,
    target_category_id: i64,
) -> Result<Vec<String>, String> {
    let mut merged = Vec::new();
    let mut seen = BTreeSet::new();
    for category_id in [target_category_id, source_category_id] {
        let mut statement = connection
            .prepare(
                "SELECT tag FROM category_tags
                 WHERE category_id = ?1 ORDER BY ordinal",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![category_id], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?;
        for row in rows {
            let tag = row.map_err(|error| error.to_string())?;
            if seen.insert(tag.clone()) {
                merged.push(tag);
            }
        }
    }
    Ok(merged)
}

fn replace_merged_tags(
    transaction: &Transaction<'_>,
    source_category_id: i64,
    target_category_id: i64,
    tags: &[String],
) -> Result<(), String> {
    transaction
        .execute(
            "DELETE FROM category_tags WHERE category_id IN (?1, ?2)",
            params![source_category_id, target_category_id],
        )
        .map_err(|error| error.to_string())?;
    for (ordinal, tag) in tags.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO category_tags(category_id, ordinal, tag)
                 VALUES (?1, ?2, ?3)",
                params![
                    target_category_id,
                    i64::try_from(ordinal).map_err(|_| "too many merged tags".to_string())?,
                    tag,
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct StagedSession {
    id: usize,
    category_id: u64,
    started_at_utc: DateTime<Utc>,
    ended_at_utc: DateTime<Utc>,
    elapsed_seconds: usize,
    policy: OperationalDayPolicy,
}

fn staged_sessions(
    connection: &Connection,
    source_category_id: i64,
    target_category_id: i64,
) -> Result<Vec<StagedSession>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, category_id, started_at_utc, ended_at_utc, elapsed_seconds,
                    boundary_utc_offset_seconds, boundary_start_minutes
             FROM sessions ORDER BY id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut sessions = Vec::new();
    for row in rows {
        let (id, category_id, started, ended, elapsed, offset, start_minutes) =
            row.map_err(|error| error.to_string())?;
        let (offset, start_minutes) = match (offset, start_minutes) {
            (Some(offset), Some(start_minutes)) => (offset, start_minutes),
            (None, None) => {
                return Err(format!(
                    "session {id} has no boundary provenance; daily contribution reassignment cannot be certified"
                ));
            }
            _ => return Err(format!("session {id} has partial boundary provenance")),
        };
        let effective_category = if category_id == source_category_id {
            target_category_id
        } else {
            category_id
        };
        sessions.push(StagedSession {
            id: usize::try_from(id).map_err(|_| format!("session ID {id} is invalid"))?,
            category_id: u64::try_from(effective_category)
                .map_err(|_| format!("session {id} category is invalid"))?,
            started_at_utc: DateTime::parse_from_rfc3339(&started)
                .map_err(|error| format!("session {id} start timestamp is invalid: {error}"))?
                .with_timezone(&Utc),
            ended_at_utc: DateTime::parse_from_rfc3339(&ended)
                .map_err(|error| format!("session {id} end timestamp is invalid: {error}"))?
                .with_timezone(&Utc),
            elapsed_seconds: usize::try_from(elapsed)
                .map_err(|_| format!("session {id} elapsed duration is invalid"))?,
            policy: OperationalDayPolicy {
                utc_offset_seconds: i32::try_from(offset)
                    .map_err(|_| format!("session {id} UTC offset is invalid"))?,
                start_minutes: u16::try_from(start_minutes)
                    .map_err(|_| format!("session {id} boundary minute is invalid"))?,
            },
        });
    }
    Ok(sessions)
}

fn stage_snapshots(
    connection: &Connection,
    source_category_id: u64,
    target_category_id: u64,
    sessions: &[StagedSession],
) -> Result<Vec<StagedSnapshot>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id, snapshot_kind, operational_day, payload_json
             FROM sand_snapshots ORDER BY id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut staged = Vec::new();
    for row in rows {
        let (id, snapshot_kind, operational_day, payload_json) =
            row.map_err(|error| error.to_string())?;
        let payload_json = if snapshot_kind == "daily-contribution" {
            let current: SedimentSnapshot =
                serde_json::from_str(&payload_json).map_err(|error| {
                    format!("daily contribution snapshot {id} is malformed: {error}")
                })?;
            if current.kind != SedimentSnapshotKind::DailyContribution {
                return Err(format!(
                    "snapshot {id} row kind is daily-contribution but payload kind differs"
                ));
            }
            let day = operational_day.as_deref().ok_or_else(|| {
                format!("daily contribution snapshot {id} has no operational day")
            })?;
            let slices = daily_slices(day, sessions)?;
            daily_contribution_from_slices(
                day,
                current.state.grid_width,
                current.state.grid_height,
                &slices,
            )
            .map(|snapshot| serde_json::to_string(&snapshot))
            .transpose()
            .map_err(|error| format!("cannot serialize daily snapshot {id}: {error}"))?
        } else if let Ok(mut snapshot) = serde_json::from_str::<SedimentSnapshot>(&payload_json) {
            let before = sediment_mass(&snapshot.state)?;
            reassign_snapshot_category(&mut snapshot, source_category_id, target_category_id)?;
            if sediment_mass(&snapshot.state)? != before {
                return Err(format!("snapshot {id} changed mass during reassignment"));
            }
            Some(
                serde_json::to_string(&snapshot)
                    .map_err(|error| format!("cannot serialize snapshot {id}: {error}"))?,
            )
        } else {
            let mut state: SandState = serde_json::from_str(&payload_json)
                .map_err(|error| format!("snapshot {id} has unsupported payload: {error}"))?;
            let before = sediment_mass(&state)?;
            reassign_sand_state_category(&mut state, source_category_id, target_category_id)?;
            if sediment_mass(&state)? != before {
                return Err(format!("snapshot {id} changed mass during reassignment"));
            }
            Some(
                serde_json::to_string(&state)
                    .map_err(|error| format!("cannot serialize snapshot {id}: {error}"))?,
            )
        };
        staged.push(StagedSnapshot { id, payload_json });
    }
    Ok(staged)
}

fn daily_slices(
    operational_day: &str,
    sessions: &[StagedSession],
) -> Result<Vec<DailySedimentSlice>, String> {
    let day = chrono::NaiveDate::parse_from_str(operational_day, "%Y-%m-%d")
        .map_err(|error| format!("invalid snapshot operational day: {error}"))?;
    let mut slices = Vec::new();
    for session in sessions {
        for slice in temporal::allocate_operational_day_slices(
            session.started_at_utc,
            session.ended_at_utc,
            session.elapsed_seconds,
            session.policy,
        )? {
            if slice.operational_day == day {
                slices.push(DailySedimentSlice {
                    category_id: session.category_id,
                    elapsed_seconds: slice.elapsed_seconds,
                    start_time: temporal::civil_from_policy(slice.started_at_utc, session.policy)?
                        .format("%H:%M:%S")
                        .to_string(),
                    end_time: temporal::civil_from_policy(slice.ended_at_utc, session.policy)?
                        .format("%H:%M:%S")
                        .to_string(),
                    session_id: session.id,
                });
            }
        }
    }
    Ok(slices)
}

fn operation_id(source: i64, target: Option<i64>, revision: &str) -> String {
    format!(
        "category-{}:{}:{}:{}",
        if target.is_some() { "merge" } else { "delete" },
        source,
        target
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string()),
        revision
    )
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    receipt: &CategoryLifecycleReceipt,
) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO category_lifecycle_receipts (
                operation_id, operation_kind, source_category_id, target_category_id,
                source_metadata_json, target_metadata_json, preview_revision,
                reference_counts_json, applied_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                receipt.operation_id,
                receipt.operation_kind,
                receipt.source.id,
                receipt.target.as_ref().map(|target| target.id),
                serde_json::to_string(&receipt.source).map_err(|error| error.to_string())?,
                receipt
                    .target
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(|error| error.to_string())?,
                receipt.preview_revision,
                serde_json::to_string(&receipt.references).map_err(|error| error.to_string())?,
                receipt.applied_at_utc,
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn query_receipt(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<CategoryLifecycleReceipt>, String> {
    connection
        .query_row(
            "SELECT operation_id, operation_kind, source_metadata_json,
                    target_metadata_json, preview_revision, reference_counts_json,
                    applied_at_utc
             FROM category_lifecycle_receipts WHERE operation_id = ?1",
            params![operation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .map(
            |(
                operation_id,
                operation_kind,
                source_json,
                target_json,
                preview_revision,
                references_json,
                applied_at_utc,
            )| {
                Ok(CategoryLifecycleReceipt {
                    operation_id,
                    operation_kind,
                    source: serde_json::from_str(&source_json).map_err(|error| {
                        format!("invalid category receipt source metadata: {error}")
                    })?,
                    target: target_json
                        .map(|json| serde_json::from_str(&json))
                        .transpose()
                        .map_err(|error| {
                            format!("invalid category receipt target metadata: {error}")
                        })?,
                    preview_revision,
                    references: serde_json::from_str(&references_json)
                        .map_err(|error| format!("invalid category receipt counts: {error}"))?,
                    applied_at_utc,
                    already_applied: false,
                })
            },
        )
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        sand::{PendingGrainRun, SandStateGrain, SedimentSnapshotProvenance},
        sqlite::{
            NewActiveSession, SqliteRepository,
            repository::{
                NewCategoryRecord, NewSandSnapshotRecord, NewSessionRecord, SnapshotKind,
            },
            runtime_coordination,
        },
    };

    fn seeded_repository(checkpoint_status: &str) -> SqliteRepository {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let source = repository
            .create_category(&NewCategoryRecord {
                name: "Source",
                description: "source metadata",
                color_index: 1,
                balance_effect: 1,
            })
            .unwrap();
        let target = repository
            .create_category(&NewCategoryRecord {
                name: "Target",
                description: "target metadata",
                color_index: 2,
                balance_effect: -1,
            })
            .unwrap();
        assert_eq!((source, target), (1, 2));
        repository
            .insert_session(&NewSessionRecord {
                stable_id: "session-source",
                project: "Project",
                category_id: source,
                description: "completed",
                started_at_utc: "2026-08-03T16:00:00Z",
                ended_at_utc: "2026-08-03T17:00:00Z",
                operational_day: "2026-08-03",
                elapsed_seconds: 3600,
                boundary_utc_offset_seconds: -21600,
                boundary_start_minutes: 360,
                source: "test",
            })
            .unwrap();
        runtime_coordination::start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id: "active-source",
                project: "",
                category_id: source,
                description: "active",
                started_at_utc: "2026-08-03T18:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        repository
            .replace_category_tags(source, &["source-tag".to_string(), "shared".to_string()])
            .unwrap();
        repository
            .replace_category_tags(target, &["target-tag".to_string(), "shared".to_string()])
            .unwrap();

        let sand = sand_state();
        repository
            .connection
            .execute(
                "INSERT INTO sand_state (
                    singleton, formation_id, quantum_seconds, grid_width, grid_height,
                    payload_json, updated_at_utc
                 ) VALUES (1, 'default', 1, 2, 2, ?1, '2026-08-03T18:00:00Z')",
                params![serde_json::to_string(&sand).unwrap()],
            )
            .unwrap();
        let daily = SedimentSnapshot::cumulative_checkpoint(
            Some("2026-08-03".to_string()),
            "seed".to_string(),
            SedimentSnapshotProvenance::RuntimeCanonical,
            sand.clone(),
        );
        repository
            .insert_sand_snapshot(&NewSandSnapshotRecord {
                formation_id: "default",
                snapshot_kind: SnapshotKind::Manual,
                operational_day: Some("2026-08-03"),
                quantum_seconds: 1,
                payload_json: &serde_json::to_string(&daily).unwrap(),
                captured_at_utc: "2026-08-03T18:00:00Z",
            })
            .unwrap();
        let contribution = daily_contribution_from_slices(
            "2026-08-03",
            2,
            2,
            &[DailySedimentSlice {
                category_id: 1,
                elapsed_seconds: 3600,
                start_time: "10:00:00".to_string(),
                end_time: "11:00:00".to_string(),
                session_id: 1,
            }],
        )
        .unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO sand_snapshots (
                    formation_id, snapshot_kind, operational_day, quantum_seconds,
                    payload_json, captured_at_utc
                 ) VALUES ('default', 'daily-contribution', '2026-08-03', 1, ?1,
                           '2026-08-03T18:00:00Z')",
                params![serde_json::to_string(&contribution).unwrap()],
            )
            .unwrap();

        let checkpoint = checkpoint_json(false);
        repository
            .connection
            .execute(
                "INSERT INTO runtime_checkpoint (
                    singleton, status, detached_at_utc, simulation_time_utc,
                    active_session_stable_id, payload_json
                 ) VALUES (1, ?1, '2026-08-03T18:00:00Z', '2026-08-03T18:00:00Z',
                           'active-source', ?2)",
                params![checkpoint_status, checkpoint],
            )
            .unwrap();
        repository
    }

    fn sand_state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
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
                count: 5,
            }],
        }
    }

    fn checkpoint_json(with_receipt: bool) -> String {
        serde_json::json!({
            "schema_version": 1,
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
            "clear_all": if with_receipt {
                serde_json::json!({"operation_id": "pending-clear"})
            } else {
                serde_json::Value::Null
            }
        })
        .to_string()
    }

    fn merge(repository: &mut SqliteRepository) -> Result<CategoryLifecycleReceipt, String> {
        let preview = preview(repository, 1, Some(2))?;
        apply(
            repository,
            CategoryLifecycleRequest {
                source_category_id: 1,
                target_category_id: Some(2),
                expected_revision: &preview.revision,
                applied_at_utc: "2026-08-03T19:00:00Z",
            },
        )
    }

    #[test]
    fn successful_merge_reassigns_every_authority_and_preserves_mass() {
        let mut repository = seeded_repository("pending");
        let preview = preview(&repository, 1, Some(2)).unwrap();
        assert_eq!(preview.references.completed_sessions, 1);
        assert_eq!(preview.references.active_sessions, 1);
        assert_eq!(preview.references.tags, 2);
        assert!(preview.references.sand_placed > 0);
        assert!(preview.references.snapshot_placed > 0);
        assert!(preview.references.checkpoint_references > 0);

        let receipt = merge(&mut repository).unwrap();
        assert_eq!(receipt.operation_kind, "merge");
        assert!(query_category(&repository.connection, 1).unwrap().is_none());
        assert!(query_category(&repository.connection, 2).unwrap().is_some());
        let completed_category: i64 = repository
            .connection
            .query_row("SELECT category_id FROM sessions WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        let active_category: i64 = repository
            .connection
            .query_row(
                "SELECT category_id FROM active_session WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((completed_category, active_category), (2, 2));
        let completed_identity: (String, String, String, String, String, i64) = repository
            .connection
            .query_row(
                "SELECT stable_id, project, description, started_at_utc, ended_at_utc,
                        elapsed_seconds
                 FROM sessions WHERE id = 1",
                [],
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
            )
            .unwrap();
        assert_eq!(
            completed_identity,
            (
                "session-source".to_string(),
                "Project".to_string(),
                "completed".to_string(),
                "2026-08-03T16:00:00Z".to_string(),
                "2026-08-03T17:00:00Z".to_string(),
                3600,
            )
        );
        let active_identity: (String, String, String, String) = repository
            .connection
            .query_row(
                "SELECT stable_id, description, started_at_utc, recovery_kind
                 FROM active_session WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            active_identity,
            (
                "active-source".to_string(),
                "active".to_string(),
                "2026-08-03T18:00:00Z".to_string(),
                "live".to_string(),
            )
        );
        let target = query_category(&repository.connection, 2).unwrap().unwrap();
        assert_eq!(target.name, "Target");
        assert_eq!(target.description, "target metadata");
        assert_eq!(target.color_index, 2);
        assert_eq!(target.balance_effect, -1);
        let tags = repository.category_tags().unwrap();
        assert_eq!(
            tags.get(&2).unwrap(),
            &vec![
                "target-tag".to_string(),
                "shared".to_string(),
                "source-tag".to_string(),
            ]
        );
        assert!(!tags.contains_key(&1));

        let state: SandState =
            serde_json::from_str(&repository.sand_state().unwrap().unwrap().payload_json).unwrap();
        assert_eq!(
            count_sand_state_category(&state, 1)
                .unwrap()
                .total()
                .unwrap(),
            0
        );
        assert_eq!(sediment_mass(&state).unwrap(), 8);

        let checkpoint: String = repository
            .connection
            .query_row(
                "SELECT payload_json FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count_checkpoint_category_references(&checkpoint, 1).unwrap(),
            0
        );
        assert!(count_checkpoint_category_references(&checkpoint, 2).unwrap() > 0);

        let daily_json: String = repository
            .connection
            .query_row(
                "SELECT payload_json FROM sand_snapshots
                 WHERE snapshot_kind = 'daily-contribution'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let daily: SedimentSnapshot = serde_json::from_str(&daily_json).unwrap();
        assert_eq!(
            count_snapshot_category(&daily, 1).unwrap().total().unwrap(),
            0
        );
        assert!(count_snapshot_category(&daily, 2).unwrap().total().unwrap() > 0);

        let residual = reference_counts_on(&repository.connection, 1).unwrap();
        assert_eq!(residual.total().unwrap(), 0);
        let receipt_count: i64 = repository
            .connection
            .query_row(
                "SELECT count(*) FROM category_lifecycle_receipts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(receipt_count, 1);
        let retry = apply(
            &mut repository,
            CategoryLifecycleRequest {
                source_category_id: 1,
                target_category_id: Some(2),
                expected_revision: &receipt.preview_revision,
                applied_at_utc: "2026-08-03T20:00:00Z",
            },
        )
        .unwrap();
        assert!(retry.already_applied);
        assert_eq!(retry.operation_id, receipt.operation_id);
        let receipt_count_after_retry: i64 = repository
            .connection
            .query_row(
                "SELECT count(*) FROM category_lifecycle_receipts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(receipt_count_after_retry, 1);
        let next = repository
            .create_category(&NewCategoryRecord {
                name: "After merge",
                description: "",
                color_index: 3,
                balance_effect: 0,
            })
            .unwrap();
        assert_eq!(next, 3, "merged source identity must remain retired");
    }

    #[test]
    fn stale_preview_and_protected_or_receipt_checkpoint_fail_closed() {
        let mut repository = seeded_repository("pending");
        let preview = preview(&repository, 1, Some(2)).unwrap();
        repository
            .connection
            .execute(
                "UPDATE sessions SET description = 'changed' WHERE id = 1",
                [],
            )
            .unwrap();
        let error = apply(
            &mut repository,
            CategoryLifecycleRequest {
                source_category_id: 1,
                target_category_id: Some(2),
                expected_revision: &preview.revision,
                applied_at_utc: "2026-08-03T19:00:00Z",
            },
        )
        .unwrap_err();
        assert!(error.contains("stale"));
        assert!(query_category(&repository.connection, 1).unwrap().is_some());

        let mut recovering = seeded_repository("recovering");
        assert!(merge(&mut recovering).unwrap_err().contains("recovering"));
        assert!(query_category(&recovering.connection, 1).unwrap().is_some());

        let mut receipt = seeded_repository("pending");
        receipt
            .connection
            .execute(
                "UPDATE runtime_checkpoint SET payload_json = ?1 WHERE singleton = 1",
                params![checkpoint_json(true)],
            )
            .unwrap();
        assert!(
            merge(&mut receipt)
                .unwrap_err()
                .contains("transition custody")
        );
        assert!(query_category(&receipt.connection, 1).unwrap().is_some());
    }

    #[test]
    fn permanent_delete_requires_complete_zero_reference_preview() {
        let mut referenced = seeded_repository("pending");
        let referenced_preview = preview(&referenced, 1, None).unwrap();
        assert!(referenced_preview.references.total().unwrap() > 0);
        assert!(
            apply(
                &mut referenced,
                CategoryLifecycleRequest {
                    source_category_id: 1,
                    target_category_id: None,
                    expected_revision: &referenced_preview.revision,
                    applied_at_utc: "2026-08-03T19:00:00Z",
                },
            )
            .unwrap_err()
            .contains("still has")
        );

        let mut empty = SqliteRepository::open_in_memory().unwrap();
        empty
            .create_category(&NewCategoryRecord {
                name: "Disposable",
                description: "",
                color_index: 1,
                balance_effect: 0,
            })
            .unwrap();
        let preview = preview(&empty, 1, None).unwrap();
        assert_eq!(preview.references.total().unwrap(), 0);
        let receipt = apply(
            &mut empty,
            CategoryLifecycleRequest {
                source_category_id: 1,
                target_category_id: None,
                expected_revision: &preview.revision,
                applied_at_utc: "2026-08-03T19:00:00Z",
            },
        )
        .unwrap();
        assert_eq!(receipt.operation_kind, "delete");
        assert!(query_category(&empty.connection, 1).unwrap().is_none());
        let replacement = empty
            .create_category(&NewCategoryRecord {
                name: "Replacement",
                description: "",
                color_index: 2,
                balance_effect: 0,
            })
            .unwrap();
        assert_eq!(replacement, 2, "retired stable identity must not be reused");
    }

    #[test]
    fn source_idle_self_merge_and_missing_identity_are_rejected() {
        let repository = seeded_repository("pending");
        assert!(
            preview(&repository, 0, Some(2))
                .unwrap_err()
                .contains("idle")
        );
        assert!(
            preview(&repository, 1, Some(1))
                .unwrap_err()
                .contains("differ")
        );
        assert!(
            preview(&repository, 99, Some(2))
                .unwrap_err()
                .contains("does not exist")
        );
        assert!(
            preview(&repository, 1, Some(99))
                .unwrap_err()
                .contains("does not exist")
        );
    }

    #[test]
    fn archived_source_and_target_remain_unambiguous_by_identity() {
        let mut repository = seeded_repository("pending");
        repository
            .archive_category(1, "2026-08-03T18:30:00Z")
            .unwrap();
        repository
            .archive_category(2, "2026-08-03T18:31:00Z")
            .unwrap();
        let preview = preview(&repository, 1, Some(2)).unwrap();
        assert!(preview.source.archived_at_utc.is_some());
        assert!(preview.target.as_ref().unwrap().archived_at_utc.is_some());
        merge(&mut repository).unwrap();
        assert!(query_category(&repository.connection, 1).unwrap().is_none());
        assert!(
            query_category(&repository.connection, 2)
                .unwrap()
                .unwrap()
                .archived_at_utc
                .is_some()
        );
    }

    #[test]
    fn every_publication_fault_rolls_back_to_the_same_preview_revision() {
        for phase in [
            "before-write",
            "sessions",
            "active",
            "tags",
            "sand",
            "snapshots",
            "checkpoint",
            "category",
            "receipt",
            "commit",
        ] {
            let mut repository = seeded_repository("pending");
            let before = preview(&repository, 1, Some(2)).unwrap();
            let result = runtime_coordination::with_test_fault(
                "category-lifecycle",
                phase,
                "commit",
                || {
                    apply(
                        &mut repository,
                        CategoryLifecycleRequest {
                            source_category_id: 1,
                            target_category_id: Some(2),
                            expected_revision: &before.revision,
                            applied_at_utc: "2026-08-03T19:00:00Z",
                        },
                    )
                },
            );
            assert!(result.is_err(), "phase {phase} unexpectedly succeeded");
            let after = preview(&repository, 1, Some(2)).unwrap();
            assert_eq!(after.revision, before.revision, "phase {phase}");
            let receipt_count: i64 = repository
                .connection
                .query_row(
                    "SELECT count(*) FROM category_lifecycle_receipts",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(receipt_count, 0, "phase {phase}");
        }
    }
}
