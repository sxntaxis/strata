use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use thiserror::Error;

use super::repository::{ActiveSessionRecord, SandStateRecord};
use super::{NewActiveSession, SessionCompletion, SqliteRepository, SqliteStoreError};

#[derive(Debug, Error)]
pub(crate) enum CoordinationError {
    #[error(transparent)]
    Store(#[from] SqliteStoreError),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid runtime transition: {0}")]
    InvalidInput(String),
    #[error("there is no active session")]
    NoActiveSession,
    #[error("active session changed concurrently; expected {expected}, found {actual}")]
    ActiveSessionConflict { expected: String, actual: String },
    #[error("runtime transition receipt {operation_id} conflicts with the requested operation")]
    ReceiptConflict { operation_id: String },
    #[error("runtime checkpoint is {actual}; expected {expected}")]
    CheckpointConflict { expected: String, actual: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeTransitionReceipt {
    pub operation_id: String,
    pub operation_kind: String,
    pub expected_active_stable_id: String,
    pub resulting_active_stable_id: Option<String>,
    pub completed_session_id: Option<i64>,
    pub elapsed_seconds: i64,
    pub source: String,
    pub applied_at_utc: String,
    pub acknowledged_at_utc: Option<String>,
    pub already_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClaimedCheckpoint {
    pub active_session_stable_id: Option<String>,
    pub payload_json: String,
}

pub(crate) fn start_active_session(
    repository: &mut SqliteRepository,
    active: &NewActiveSession<'_>,
) -> Result<(), CoordinationError> {
    require_non_empty(active.stable_id, "active stable ID")?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(current) = query_active(&transaction)? {
        return Err(CoordinationError::ActiveSessionConflict {
            expected: "no active session".to_string(),
            actual: current.stable_id,
        });
    }
    insert_active(&transaction, active)?;
    transaction.commit()?;
    Ok(())
}

pub(crate) fn finish_active_session(
    repository: &mut SqliteRepository,
    expected_active_stable_id: &str,
    operation_id: &str,
    completion: &SessionCompletion<'_>,
    acknowledge_immediately: bool,
) -> Result<RuntimeTransitionReceipt, CoordinationError> {
    validate_transition_inputs(expected_active_stable_id, operation_id, completion)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(receipt) = query_receipt(&transaction, operation_id)? {
        return validate_existing_receipt(receipt, "finish", expected_active_stable_id, None);
    }

    let active = query_active(&transaction)?.ok_or(CoordinationError::NoActiveSession)?;
    require_expected_active(&active, expected_active_stable_id)?;
    let completed_session_id = insert_completed(&transaction, &active, completion)?;
    delete_expected_active(&transaction, expected_active_stable_id)?;
    let acknowledged_at_utc = acknowledge_immediately.then_some(completion.ended_at_utc);
    insert_receipt(
        &transaction,
        operation_id,
        "finish",
        expected_active_stable_id,
        None,
        Some(completed_session_id),
        completion.elapsed_seconds,
        completion.source,
        completion.ended_at_utc,
        acknowledged_at_utc,
    )?;
    transaction.commit()?;
    Ok(RuntimeTransitionReceipt {
        operation_id: operation_id.to_string(),
        operation_kind: "finish".to_string(),
        expected_active_stable_id: expected_active_stable_id.to_string(),
        resulting_active_stable_id: None,
        completed_session_id: Some(completed_session_id),
        elapsed_seconds: completion.elapsed_seconds,
        source: completion.source.to_string(),
        applied_at_utc: completion.ended_at_utc.to_string(),
        acknowledged_at_utc: acknowledged_at_utc.map(ToString::to_string),
        already_applied: false,
    })
}

pub(crate) fn switch_active_session(
    repository: &mut SqliteRepository,
    expected_active_stable_id: &str,
    operation_id: &str,
    completion: &SessionCompletion<'_>,
    next: &NewActiveSession<'_>,
) -> Result<RuntimeTransitionReceipt, CoordinationError> {
    validate_transition_inputs(expected_active_stable_id, operation_id, completion)?;
    require_non_empty(next.stable_id, "next active stable ID")?;
    if next.stable_id == expected_active_stable_id {
        return Err(CoordinationError::InvalidInput(
            "switch must create a new active stable identity".to_string(),
        ));
    }
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(receipt) = query_receipt(&transaction, operation_id)? {
        return validate_existing_receipt(
            receipt,
            "switch",
            expected_active_stable_id,
            Some(next.stable_id),
        );
    }

    let active = query_active(&transaction)?.ok_or(CoordinationError::NoActiveSession)?;
    require_expected_active(&active, expected_active_stable_id)?;
    let completed_session_id = insert_completed(&transaction, &active, completion)?;
    delete_expected_active(&transaction, expected_active_stable_id)?;
    insert_active(&transaction, next)?;
    insert_receipt(
        &transaction,
        operation_id,
        "switch",
        expected_active_stable_id,
        Some(next.stable_id),
        Some(completed_session_id),
        completion.elapsed_seconds,
        completion.source,
        completion.ended_at_utc,
        Some(completion.ended_at_utc),
    )?;
    transaction.commit()?;
    Ok(RuntimeTransitionReceipt {
        operation_id: operation_id.to_string(),
        operation_kind: "switch".to_string(),
        expected_active_stable_id: expected_active_stable_id.to_string(),
        resulting_active_stable_id: Some(next.stable_id.to_string()),
        completed_session_id: Some(completed_session_id),
        elapsed_seconds: completion.elapsed_seconds,
        source: completion.source.to_string(),
        applied_at_utc: completion.ended_at_utc.to_string(),
        acknowledged_at_utc: Some(completion.ended_at_utc.to_string()),
        already_applied: false,
    })
}

pub(crate) fn reset_active_session(
    repository: &mut SqliteRepository,
    expected_active_stable_id: &str,
    operation_id: &str,
    next: &NewActiveSession<'_>,
    applied_at_utc: &str,
    source: &str,
) -> Result<RuntimeTransitionReceipt, CoordinationError> {
    require_non_empty(expected_active_stable_id, "expected active stable ID")?;
    require_non_empty(operation_id, "operation ID")?;
    require_non_empty(next.stable_id, "next active stable ID")?;
    require_non_empty(applied_at_utc, "applied timestamp")?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    if let Some(receipt) = query_receipt(&transaction, operation_id)? {
        return validate_existing_receipt(
            receipt,
            "reset",
            expected_active_stable_id,
            Some(next.stable_id),
        );
    }

    let active = query_active(&transaction)?.ok_or(CoordinationError::NoActiveSession)?;
    require_expected_active(&active, expected_active_stable_id)?;
    delete_expected_active(&transaction, expected_active_stable_id)?;
    insert_active(&transaction, next)?;
    insert_receipt(
        &transaction,
        operation_id,
        "reset",
        expected_active_stable_id,
        Some(next.stable_id),
        None,
        0,
        source,
        applied_at_utc,
        Some(applied_at_utc),
    )?;
    transaction.commit()?;
    Ok(RuntimeTransitionReceipt {
        operation_id: operation_id.to_string(),
        operation_kind: "reset".to_string(),
        expected_active_stable_id: expected_active_stable_id.to_string(),
        resulting_active_stable_id: Some(next.stable_id.to_string()),
        completed_session_id: None,
        elapsed_seconds: 0,
        source: source.to_string(),
        applied_at_utc: applied_at_utc.to_string(),
        acknowledged_at_utc: Some(applied_at_utc.to_string()),
        already_applied: false,
    })
}

pub(crate) fn update_active_description(
    repository: &mut SqliteRepository,
    expected_active_stable_id: &str,
    description: &str,
) -> Result<(), CoordinationError> {
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = transaction.execute(
        "UPDATE active_session SET description = ?1 WHERE singleton = 1 AND stable_id = ?2",
        params![description, expected_active_stable_id],
    )?;
    if changed != 1 {
        let actual = query_active(&transaction)?
            .map(|active| active.stable_id)
            .unwrap_or_else(|| "no active session".to_string());
        return Err(CoordinationError::ActiveSessionConflict {
            expected: expected_active_stable_id.to_string(),
            actual,
        });
    }
    transaction.commit()?;
    Ok(())
}

pub(crate) fn latest_unacknowledged_finish(
    repository: &SqliteRepository,
    source: &str,
) -> Result<Option<RuntimeTransitionReceipt>, CoordinationError> {
    let mut statement = repository.connection.prepare(
        "SELECT operation_id, operation_kind, expected_active_stable_id,
                resulting_active_stable_id, completed_session_id, elapsed_seconds,
                source, applied_at_utc, acknowledged_at_utc
         FROM runtime_transitions
         WHERE operation_kind = 'finish' AND source = ?1 AND acknowledged_at_utc IS NULL
         ORDER BY applied_at_utc DESC, operation_id DESC
         LIMIT 2",
    )?;
    let receipts = statement
        .query_map(params![source], receipt_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    if receipts.len() > 1 {
        return Err(CoordinationError::InvalidInput(
            "multiple unacknowledged finish receipts require database repair".to_string(),
        ));
    }
    Ok(receipts.into_iter().next())
}

pub(crate) fn acknowledge_transition(
    repository: &mut SqliteRepository,
    operation_id: &str,
    acknowledged_at_utc: &str,
) -> Result<(), CoordinationError> {
    require_non_empty(operation_id, "operation ID")?;
    require_non_empty(acknowledged_at_utc, "acknowledgement timestamp")?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = transaction.execute(
        "UPDATE runtime_transitions
         SET acknowledged_at_utc = COALESCE(acknowledged_at_utc, ?1)
         WHERE operation_id = ?2",
        params![acknowledged_at_utc, operation_id],
    )?;
    if changed != 1 {
        return Err(CoordinationError::InvalidInput(format!(
            "runtime transition receipt '{operation_id}' does not exist"
        )));
    }
    transaction.commit()?;
    Ok(())
}

pub(crate) fn save_checkpoint(
    repository: &mut SqliteRepository,
    expected_active_stable_id: &str,
    detached_at_utc: &str,
    simulation_time_utc: &str,
    payload_json: &str,
) -> Result<(), CoordinationError> {
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let active = query_active(&transaction)?.ok_or(CoordinationError::NoActiveSession)?;
    require_expected_active(&active, expected_active_stable_id)?;
    let existing_status: Option<String> = transaction
        .query_row(
            "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(status) = existing_status
        && status != "committed"
    {
        return Err(CoordinationError::CheckpointConflict {
            expected: "no checkpoint or committed".to_string(),
            actual: status,
        });
    }
    transaction.execute(
        "INSERT INTO runtime_checkpoint (
            singleton, status, detached_at_utc, simulation_time_utc,
            active_session_stable_id, payload_json, legacy_import_id
         ) VALUES (1, 'pending', ?1, ?2, ?3, ?4, NULL)
         ON CONFLICT(singleton) DO UPDATE SET
            status = 'pending',
            detached_at_utc = excluded.detached_at_utc,
            simulation_time_utc = excluded.simulation_time_utc,
            active_session_stable_id = excluded.active_session_stable_id,
            payload_json = excluded.payload_json,
            legacy_import_id = NULL",
        params![
            detached_at_utc,
            simulation_time_utc,
            expected_active_stable_id,
            payload_json,
        ],
    )?;
    transaction.commit()?;
    Ok(())
}

pub(crate) fn claim_checkpoint(
    repository: &mut SqliteRepository,
) -> Result<Option<ClaimedCheckpoint>, CoordinationError> {
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let record: Option<(String, Option<String>, String)> = transaction
        .query_row(
            "SELECT status, active_session_stable_id, payload_json
             FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((status, active_session_stable_id, payload_json)) = record else {
        transaction.commit()?;
        return Ok(None);
    };
    match status.as_str() {
        "pending" => {
            transaction.execute(
                "UPDATE runtime_checkpoint SET status = 'recovering'
                 WHERE singleton = 1 AND status = 'pending'",
                [],
            )?;
            transaction.commit()?;
            Ok(Some(ClaimedCheckpoint {
                active_session_stable_id,
                payload_json,
            }))
        }
        "recovering" => {
            transaction.commit()?;
            Ok(Some(ClaimedCheckpoint {
                active_session_stable_id,
                payload_json,
            }))
        }
        "committed" => {
            transaction.execute(
                "DELETE FROM runtime_checkpoint WHERE singleton = 1 AND status = 'committed'",
                [],
            )?;
            transaction.commit()?;
            Ok(None)
        }
        "quarantined" => Err(CoordinationError::CheckpointConflict {
            expected: "pending, recovering, or committed".to_string(),
            actual: status,
        }),
        _ => Err(CoordinationError::CheckpointConflict {
            expected: "known checkpoint status".to_string(),
            actual: status,
        }),
    }
}

pub(crate) fn quarantine_checkpoint(
    repository: &mut SqliteRepository,
) -> Result<(), CoordinationError> {
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = transaction.execute(
        "UPDATE runtime_checkpoint SET status = 'quarantined'
         WHERE singleton = 1 AND status IN ('pending', 'recovering')",
        [],
    )?;
    if changed != 1 {
        let actual: Option<String> = transaction
            .query_row(
                "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        return Err(CoordinationError::CheckpointConflict {
            expected: "pending or recovering".to_string(),
            actual: actual.unwrap_or_else(|| "missing".to_string()),
        });
    }
    transaction.commit()?;
    Ok(())
}

pub(crate) fn commit_checkpoint_recovery(
    repository: &mut SqliteRepository,
    expected_active_stable_id: &str,
    operational_day: &str,
    state: &SandStateRecord,
    captured_at_utc: &str,
) -> Result<(), CoordinationError> {
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let status: Option<String> = transaction
        .query_row(
            "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if status.as_deref() != Some("recovering") {
        return Err(CoordinationError::CheckpointConflict {
            expected: "recovering".to_string(),
            actual: status.unwrap_or_else(|| "missing".to_string()),
        });
    }
    let active = query_active(&transaction)?.ok_or(CoordinationError::NoActiveSession)?;
    require_expected_active(&active, expected_active_stable_id)?;
    transaction.execute(
        "INSERT INTO sand_state (
            singleton, formation_id, quantum_seconds, grid_width, grid_height,
            payload_json, updated_at_utc, legacy_import_id
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, NULL)
         ON CONFLICT(singleton) DO UPDATE SET
            formation_id = excluded.formation_id,
            quantum_seconds = excluded.quantum_seconds,
            grid_width = excluded.grid_width,
            grid_height = excluded.grid_height,
            payload_json = excluded.payload_json,
            updated_at_utc = excluded.updated_at_utc,
            legacy_import_id = NULL",
        params![
            state.formation_id,
            state.quantum_seconds,
            state.grid_width,
            state.grid_height,
            state.payload_json,
            state.updated_at_utc,
        ],
    )?;
    transaction.execute(
        "DELETE FROM sand_snapshots
         WHERE snapshot_kind = 'daily' AND operational_day = ?1",
        params![operational_day],
    )?;
    transaction.execute(
        "INSERT INTO sand_snapshots (
            formation_id, snapshot_kind, operational_day, quantum_seconds,
            payload_json, captured_at_utc, legacy_import_id
         ) VALUES (?1, 'daily', ?2, ?3, ?4, ?5, NULL)",
        params![
            state.formation_id,
            operational_day,
            state.quantum_seconds,
            state.payload_json,
            captured_at_utc,
        ],
    )?;
    let changed = transaction.execute(
        "UPDATE runtime_checkpoint SET status = 'committed'
         WHERE singleton = 1 AND status = 'recovering'",
        [],
    )?;
    if changed != 1 {
        return Err(CoordinationError::CheckpointConflict {
            expected: "recovering".to_string(),
            actual: "changed concurrently".to_string(),
        });
    }
    transaction.commit()?;
    Ok(())
}

pub(crate) fn clear_committed_checkpoint(
    repository: &mut SqliteRepository,
) -> Result<(), CoordinationError> {
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let status: Option<String> = transaction
        .query_row(
            "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match status.as_deref() {
        None => {
            transaction.commit()?;
            Ok(())
        }
        Some("committed") => {
            transaction.execute(
                "DELETE FROM runtime_checkpoint WHERE singleton = 1 AND status = 'committed'",
                [],
            )?;
            transaction.commit()?;
            Ok(())
        }
        Some(actual) => Err(CoordinationError::CheckpointConflict {
            expected: "committed or missing".to_string(),
            actual: actual.to_string(),
        }),
    }
}

fn validate_transition_inputs(
    expected_active_stable_id: &str,
    operation_id: &str,
    completion: &SessionCompletion<'_>,
) -> Result<(), CoordinationError> {
    require_non_empty(expected_active_stable_id, "expected active stable ID")?;
    require_non_empty(operation_id, "operation ID")?;
    require_non_empty(completion.ended_at_utc, "completion timestamp")?;
    require_non_empty(completion.operational_day, "operational day")?;
    require_non_empty(completion.source, "completion source")?;
    if completion.elapsed_seconds < 0 {
        return Err(CoordinationError::InvalidInput(
            "elapsed seconds cannot be negative".to_string(),
        ));
    }
    Ok(())
}

fn require_non_empty(value: &str, label: &str) -> Result<(), CoordinationError> {
    if value.trim().is_empty() {
        Err(CoordinationError::InvalidInput(format!(
            "{label} cannot be empty"
        )))
    } else {
        Ok(())
    }
}

fn require_expected_active(
    active: &ActiveSessionRecord,
    expected_active_stable_id: &str,
) -> Result<(), CoordinationError> {
    if active.stable_id == expected_active_stable_id {
        Ok(())
    } else {
        Err(CoordinationError::ActiveSessionConflict {
            expected: expected_active_stable_id.to_string(),
            actual: active.stable_id.clone(),
        })
    }
}

fn validate_existing_receipt(
    mut receipt: RuntimeTransitionReceipt,
    operation_kind: &str,
    expected_active_stable_id: &str,
    resulting_active_stable_id: Option<&str>,
) -> Result<RuntimeTransitionReceipt, CoordinationError> {
    if receipt.operation_kind != operation_kind
        || receipt.expected_active_stable_id != expected_active_stable_id
        || receipt.resulting_active_stable_id.as_deref() != resulting_active_stable_id
    {
        return Err(CoordinationError::ReceiptConflict {
            operation_id: receipt.operation_id,
        });
    }
    receipt.already_applied = true;
    Ok(receipt)
}

fn query_active(connection: &Connection) -> Result<Option<ActiveSessionRecord>, rusqlite::Error> {
    connection
        .query_row(
            "SELECT stable_id, project, category_id, description, started_at_utc, recovery_kind
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
        .optional()
}

fn insert_active(
    transaction: &Transaction<'_>,
    active: &NewActiveSession<'_>,
) -> Result<(), rusqlite::Error> {
    transaction.execute(
        "INSERT INTO active_session (
            singleton, stable_id, project, category_id, description,
            started_at_utc, recovery_kind, legacy_import_id
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, NULL)",
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

fn insert_completed(
    transaction: &Transaction<'_>,
    active: &ActiveSessionRecord,
    completion: &SessionCompletion<'_>,
) -> Result<i64, rusqlite::Error> {
    transaction.execute(
        "INSERT INTO sessions (
            stable_id, project, category_id, description, started_at_utc,
            ended_at_utc, operational_day, elapsed_seconds, source, legacy_import_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL)",
        params![
            active.stable_id,
            active.project,
            active.category_id,
            active.description,
            active.started_at_utc,
            completion.ended_at_utc,
            completion.operational_day,
            completion.elapsed_seconds,
            completion.source,
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

fn delete_expected_active(
    transaction: &Transaction<'_>,
    expected_active_stable_id: &str,
) -> Result<(), CoordinationError> {
    let changed = transaction.execute(
        "DELETE FROM active_session WHERE singleton = 1 AND stable_id = ?1",
        params![expected_active_stable_id],
    )?;
    if changed != 1 {
        let actual = query_active(transaction)?
            .map(|active| active.stable_id)
            .unwrap_or_else(|| "no active session".to_string());
        return Err(CoordinationError::ActiveSessionConflict {
            expected: expected_active_stable_id.to_string(),
            actual,
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_receipt(
    transaction: &Transaction<'_>,
    operation_id: &str,
    operation_kind: &str,
    expected_active_stable_id: &str,
    resulting_active_stable_id: Option<&str>,
    completed_session_id: Option<i64>,
    elapsed_seconds: i64,
    source: &str,
    applied_at_utc: &str,
    acknowledged_at_utc: Option<&str>,
) -> Result<(), rusqlite::Error> {
    transaction.execute(
        "INSERT INTO runtime_transitions (
            operation_id, operation_kind, expected_active_stable_id,
            resulting_active_stable_id, completed_session_id, elapsed_seconds,
            source, applied_at_utc, acknowledged_at_utc
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            operation_id,
            operation_kind,
            expected_active_stable_id,
            resulting_active_stable_id,
            completed_session_id,
            elapsed_seconds,
            source,
            applied_at_utc,
            acknowledged_at_utc,
        ],
    )?;
    Ok(())
}

fn query_receipt(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<RuntimeTransitionReceipt>, rusqlite::Error> {
    connection
        .query_row(
            "SELECT operation_id, operation_kind, expected_active_stable_id,
                    resulting_active_stable_id, completed_session_id, elapsed_seconds,
                    source, applied_at_utc, acknowledged_at_utc
             FROM runtime_transitions WHERE operation_id = ?1",
            params![operation_id],
            receipt_from_row,
        )
        .optional()
}

fn receipt_from_row(row: &rusqlite::Row<'_>) -> Result<RuntimeTransitionReceipt, rusqlite::Error> {
    Ok(RuntimeTransitionReceipt {
        operation_id: row.get(0)?,
        operation_kind: row.get(1)?,
        expected_active_stable_id: row.get(2)?,
        resulting_active_stable_id: row.get(3)?,
        completed_session_id: row.get(4)?,
        elapsed_seconds: row.get(5)?,
        source: row.get(6)?,
        applied_at_utc: row.get(7)?,
        acknowledged_at_utc: row.get(8)?,
        already_applied: false,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Barrier},
        thread,
    };

    use super::*;
    use crate::sqlite::repository::NewCategoryRecord;

    fn database_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-sqlite008-{name}-{}-{}.sqlite3",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn remove_database(path: &Path) {
        std::fs::remove_file(path).ok();
        std::fs::remove_file(format!("{}-wal", path.display())).ok();
        std::fs::remove_file(format!("{}-shm", path.display())).ok();
    }

    fn seed(path: &Path, stable_id: &str) {
        let mut repository = SqliteRepository::open(path).unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
        start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id,
                project: "Project",
                category_id: 1,
                description: "",
                started_at_utc: "2026-08-01T10:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
    }

    fn completion(source: &'static str) -> SessionCompletion<'static> {
        SessionCompletion {
            ended_at_utc: "2026-08-01T11:00:00Z",
            operational_day: "2026-08-01",
            elapsed_seconds: 3600,
            source,
        }
    }

    #[test]
    fn concurrent_identical_finish_converges_on_one_receipt() {
        let path = database_path("concurrent-finish");
        seed(&path, "active-a");
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let path = path.clone();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                let mut repository = SqliteRepository::open(&path).unwrap();
                barrier.wait();
                finish_active_session(
                    &mut repository,
                    "active-a",
                    "finish:active-a",
                    &completion("cli-runtime"),
                    false,
                )
                .unwrap()
            }));
        }
        barrier.wait();
        let receipts = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            receipts[0].completed_session_id,
            receipts[1].completed_session_id
        );
        assert!(receipts.iter().any(|receipt| receipt.already_applied));
        let repository = SqliteRepository::open(&path).unwrap();
        let session_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        let receipt_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM runtime_transitions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(session_count, 1);
        assert_eq!(receipt_count, 1);
        assert!(repository.active_session().unwrap().is_none());
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn stale_finish_cannot_finalize_replacement_active_session() {
        let path = database_path("stale-finish");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        switch_active_session(
            &mut repository,
            "active-a",
            "switch:a:b",
            &completion("tui-runtime"),
            &NewActiveSession {
                stable_id: "active-b",
                project: "",
                category_id: 1,
                description: "",
                started_at_utc: "2026-08-01T11:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        let error = finish_active_session(
            &mut repository,
            "active-a",
            "finish:stale-a",
            &completion("cli-runtime"),
            false,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CoordinationError::ActiveSessionConflict { ref actual, .. } if actual == "active-b"
        ));
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "active-b"
        );
        assert_eq!(repository.list_sessions().unwrap().len(), 1);
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn finish_receipt_can_be_recovered_then_acknowledged() {
        let path = database_path("finish-receipt");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        let applied = finish_active_session(
            &mut repository,
            "active-a",
            "finish:active-a",
            &completion("cli-runtime"),
            false,
        )
        .unwrap();
        let recovered = latest_unacknowledged_finish(&repository, "cli-runtime")
            .unwrap()
            .unwrap();
        assert_eq!(recovered.operation_id, applied.operation_id);
        acknowledge_transition(
            &mut repository,
            &recovered.operation_id,
            "2026-08-01T11:00:01Z",
        )
        .unwrap();
        assert!(
            latest_unacknowledged_finish(&repository, "cli-runtime")
                .unwrap()
                .is_none()
        );
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn checkpoint_commit_is_atomic_and_recovering_is_reclaimable() {
        let path = database_path("checkpoint");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T11:00:00Z",
            "2026-08-01T10:59:00Z",
            "{\"schema_version\":1}",
        )
        .unwrap();
        let first = claim_checkpoint(&mut repository).unwrap().unwrap();
        let second = claim_checkpoint(&mut repository).unwrap().unwrap();
        assert_eq!(first, second);
        repository
            .connection
            .execute_batch(
                "CREATE TRIGGER fail_recovery_commit
                 BEFORE UPDATE OF status ON runtime_checkpoint
                 WHEN NEW.status = 'committed'
                 BEGIN SELECT RAISE(ABORT, 'injected checkpoint failure'); END;",
            )
            .unwrap();
        let state = SandStateRecord {
            formation_id: "default".to_string(),
            quantum_seconds: 1,
            grid_width: 2,
            grid_height: 2,
            payload_json: "{\"version\":1,\"grid_width\":2,\"grid_height\":2,\"grains\":[],\"frame_count\":0,\"sweep_left_to_right\":true,\"rng_state\":1}".to_string(),
            updated_at_utc: "2026-08-01T11:00:00Z".to_string(),
        };
        assert!(
            commit_checkpoint_recovery(
                &mut repository,
                "active-a",
                "2026-08-01",
                &state,
                "2026-08-01T11:00:00Z",
            )
            .is_err()
        );
        let status: String = repository
            .connection
            .query_row(
                "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let sand_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sand_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status, "recovering");
        assert_eq!(sand_count, 0);
        repository
            .connection
            .execute_batch("DROP TRIGGER fail_recovery_commit")
            .unwrap();
        commit_checkpoint_recovery(
            &mut repository,
            "active-a",
            "2026-08-01",
            &state,
            "2026-08-01T11:00:00Z",
        )
        .unwrap();
        clear_committed_checkpoint(&mut repository).unwrap();
        assert!(repository.checkpoint().unwrap().is_none());
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn completed_session_deletion_retains_receipt_without_dangling_reference() {
        let path = database_path("receipt-session-delete");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        let receipt = finish_active_session(
            &mut repository,
            "active-a",
            "finish:active-a",
            &completion("tui-runtime"),
            true,
        )
        .unwrap();
        let completed_id = receipt.completed_session_id.unwrap();
        repository.delete_session(completed_id).unwrap();
        let retained: (i64, Option<i64>) = repository
            .connection
            .query_row(
                "SELECT count(*), max(completed_session_id)
                 FROM runtime_transitions WHERE operation_id = 'finish:active-a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(retained.0, 1);
        assert_eq!(retained.1, None);
        drop(repository);
        remove_database(&path);
    }
}
