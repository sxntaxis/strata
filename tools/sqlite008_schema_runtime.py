from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


# Schema v4 and module registration.
path = Path("src/sqlite.rs")
text = path.read_text()
text = replace_once(
    text,
    "mod repository;\nmod tui_runtime;",
    "mod repository;\nmod runtime_coordination;\nmod tui_runtime;",
    "runtime coordination module",
)
text = replace_once(
    text,
    "    read_snapshot as read_cli_snapshot, start_session as start_cli_session,\n    stop_session as stop_cli_session,\n};",
    "    acknowledge_stop as acknowledge_cli_stop, read_snapshot as read_cli_snapshot,\n    start_session as start_cli_session, stop_session as stop_cli_session,\n};",
    "CLI coordination exports",
)
text = replace_once(
    text,
    "const CURRENT_SCHEMA_VERSION: i64 = 3;",
    "const CURRENT_SCHEMA_VERSION: i64 = 4;",
    "schema version",
)
anchor = '''const MIGRATION_3: &str = r#"\nALTER TABLE categories\n    ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0);\n\nUPDATE categories SET sort_order = id;\n\nCREATE INDEX categories_active_order_index\n    ON categories(archived_at_utc, sort_order, id);\n\nINSERT INTO schema_migrations(version, applied_at_utc)\nVALUES (3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));\n\nPRAGMA user_version = 3;\n"#;\n'''
migration4 = anchor + '''\nconst MIGRATION_4: &str = r#"\nCREATE TABLE runtime_transitions (\n    operation_id TEXT PRIMARY KEY,\n    operation_kind TEXT NOT NULL\n        CHECK (operation_kind IN ('finish', 'switch', 'reset')),\n    expected_active_stable_id TEXT NOT NULL,\n    resulting_active_stable_id TEXT,\n    completed_session_id INTEGER,\n    elapsed_seconds INTEGER NOT NULL CHECK (elapsed_seconds >= 0),\n    source TEXT NOT NULL,\n    applied_at_utc TEXT NOT NULL,\n    acknowledged_at_utc TEXT,\n    FOREIGN KEY (completed_session_id) REFERENCES sessions(id)\n        ON UPDATE RESTRICT\n        ON DELETE RESTRICT\n) STRICT;\n\nCREATE INDEX runtime_transitions_unacknowledged_index\n    ON runtime_transitions(operation_kind, source, acknowledged_at_utc, applied_at_utc);\nCREATE INDEX runtime_transitions_expected_active_index\n    ON runtime_transitions(expected_active_stable_id, applied_at_utc);\n\nINSERT INTO schema_migrations(version, applied_at_utc)\nVALUES (4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));\n\nPRAGMA user_version = 4;\n"#;\n'''
text = replace_once(text, anchor, migration4, "migration 4")
text = replace_once(
    text,
    '''        if version < 3 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_3)?;\n            transaction.commit()?;\n        }\n\n        Ok(Self { connection })''',
    '''        if version < 3 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_3)?;\n            transaction.commit()?;\n            version = 3;\n        }\n\n        if version < 4 {\n            let transaction =\n                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;\n            transaction.execute_batch(MIGRATION_4)?;\n            transaction.commit()?;\n        }\n\n        Ok(Self { connection })''',
    "migration application",
)
text = text.replace("assert_eq!(repository.schema_version().unwrap(), 3);", "assert_eq!(repository.schema_version().unwrap(), 4);")
path.write_text(text)


# New coordination boundary.
Path("src/sqlite/runtime_coordination.rs").write_text(r'''use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use thiserror::Error;

use super::{NewActiveSession, SessionCompletion, SqliteRepository, SqliteStoreError};
use super::repository::{ActiveSessionRecord, SandStateRecord};

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
        assert_eq!(receipts[0].completed_session_id, receipts[1].completed_session_id);
        assert!(receipts.iter().any(|receipt| receipt.already_applied));
        let repository = SqliteRepository::open(&path).unwrap();
        let session_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        let receipt_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM runtime_transitions", [], |row| row.get(0))
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
        assert_eq!(repository.active_session().unwrap().unwrap().stable_id, "active-b");
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
}
''')


# CLI runtime uses fenced, idempotent transitions.
path = Path("src/sqlite/cli_runtime.rs")
text = path.read_text()
text = replace_once(
    text,
    "use super::{NewActiveSession, SessionCompletion, authority::open_cli_repository};",
    "use super::{\n    NewActiveSession, SessionCompletion, authority::open_cli_repository, runtime_coordination,\n};",
    "CLI runtime imports",
)
text = replace_once(
    text,
    '''pub(crate) struct SqliteCliStopResult {\n    pub elapsed_seconds: usize,\n}''',
    '''pub(crate) struct SqliteCliStopResult {\n    pub elapsed_seconds: usize,\n    pub operation_id: String,\n}''',
    "CLI stop result",
)
old_start = '''    let mut repository = open_cli_repository(database_path)?;\n    if repository\n        .active_session()\n        .map_err(|error| error.to_string())?\n        .is_some()\n    {\n        return Err(\n            "An active session is already running; stop it before starting another".to_string(),\n        );\n    }\n\n    let categories = repository'''
new_start = '''    let mut repository = open_cli_repository(database_path)?;\n\n    let categories = repository'''
text = replace_once(text, old_start, new_start, "CLI start precheck")
text = replace_once(
    text,
    '''    repository\n        .start_session(&NewActiveSession {''',
    '''    runtime_coordination::start_active_session(\n        &mut repository,\n        &NewActiveSession {''',
    "CLI coordinated start",
)
text = replace_once(
    text,
    '''            recovery_kind: "live",\n        })\n        .map_err(|error| error.to_string())?;''',
    '''            recovery_kind: "live",\n        },\n    )\n    .map_err(|error| error.to_string())?;''',
    "CLI coordinated start close",
)
start = text.index("pub(crate) fn stop_session(database_path: &Path)")
end = text.index("\npub(crate) fn read_snapshot", start)
new_stop = r'''pub(crate) fn stop_session(database_path: &Path) -> Result<SqliteCliStopResult, String> {
    let mut repository = open_cli_repository(database_path)?;
    let active = repository
        .active_session()
        .map_err(|error| error.to_string())?;

    let receipt = if let Some(active) = active {
        let started_at = DateTime::parse_from_rfc3339(&active.started_at_utc)
            .map_err(|error| format!("Active session has an invalid start timestamp: {error}"))?
            .with_timezone(&Utc);
        let ended_at = Utc::now();
        let elapsed_i64 = (ended_at - started_at).num_seconds().max(0);
        let operational_day = operational_day_key_for_local(&Local::now())
            .format("%Y-%m-%d")
            .to_string();
        let ended_at_utc = ended_at.to_rfc3339_opts(SecondsFormat::Millis, true);
        let operation_id = format!("finish:{}", active.stable_id);
        runtime_coordination::finish_active_session(
            &mut repository,
            &active.stable_id,
            &operation_id,
            &SessionCompletion {
                ended_at_utc: &ended_at_utc,
                operational_day: &operational_day,
                elapsed_seconds: elapsed_i64,
                source: "cli-runtime",
            },
            false,
        )
        .map_err(|error| error.to_string())?
    } else {
        runtime_coordination::latest_unacknowledged_finish(&repository, "cli-runtime")
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "No active session to stop".to_string())?
    };

    let elapsed_seconds = usize::try_from(receipt.elapsed_seconds)
        .map_err(|_| "Active session duration exceeds this platform's limits".to_string())?;
    Ok(SqliteCliStopResult {
        elapsed_seconds,
        operation_id: receipt.operation_id,
    })
}

pub(crate) fn acknowledge_stop(database_path: &Path, operation_id: &str) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let acknowledged_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    runtime_coordination::acknowledge_transition(
        &mut repository,
        operation_id,
        &acknowledged_at,
    )
    .map_err(|error| error.to_string())
}
'''
text = text[:start] + new_stop + text[end:]
path.write_text(text)


# Flush the user-visible stop result before acknowledging its durable receipt.
path = Path("src/cli.rs")
text = path.read_text()
text = replace_once(text, "    io,\n", "    io::{self, Write},\n", "CLI Write import")
old = '''            let elapsed = stopped.elapsed_seconds;\n            println!(\n                "Stopped session. Elapsed time: {:02}:{:02}:{:02}",\n                elapsed / 3600,\n                (elapsed % 3600) / 60,\n                elapsed % 60\n            );\n            Ok(elapsed)'''
new = '''            let elapsed = stopped.elapsed_seconds;\n            println!(\n                "Stopped session. Elapsed time: {:02}:{:02}:{:02}",\n                elapsed / 3600,\n                (elapsed % 3600) / 60,\n                elapsed % 60\n            );\n            io::stdout().flush().map_err(|error| error.to_string())?;\n            sqlite::acknowledge_cli_stop(&database_path, &stopped.operation_id)?;\n            Ok(elapsed)'''
text = replace_once(text, old, new, "CLI stop acknowledgement")
path.write_text(text)
