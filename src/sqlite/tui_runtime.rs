use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process,
};

use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    constants::COLORS,
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, OperationalDayPolicy, Session,
        day_boundary_config, runtime_settings,
    },
    sand::SandState,
    storage::{CategoryTagsState, LoadedCategories, LoadedSessions},
};

use super::{
    NewActiveSession, SessionCompletion, authority::open_cli_repository,
    repository::SandStateRecord, runtime_coordination,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteTuiActiveSession {
    pub stable_id: String,
    pub category_id: CategoryId,
    pub description: String,
    pub started_at_utc: DateTime<Utc>,
}

#[derive(Debug)]
pub(crate) struct SqliteTuiState {
    pub loaded_categories: LoadedCategories,
    pub loaded_sessions: LoadedSessions,
    pub archived_categories: Vec<Category>,
    pub category_tags: CategoryTagsState,
    pub active_session: Option<SqliteTuiActiveSession>,
}

pub(crate) fn load_state(database_path: &Path) -> Result<SqliteTuiState, String> {
    runtime_coordination::maybe_inject_test_fault("state-load", "before-read")
        .map_err(|error| error.to_string())?;
    let repository = open_cli_repository(database_path)?;
    let category_rows = repository
        .list_categories(true)
        .map_err(|error| error.to_string())?;
    let session_rows = repository
        .list_sessions()
        .map_err(|error| error.to_string())?;
    let tag_rows = repository
        .category_tags()
        .map_err(|error| error.to_string())?;
    let active_row = repository
        .active_session()
        .map_err(|error| error.to_string())?;

    let mut active_categories = Vec::new();
    let mut archived_categories = Vec::new();
    let mut max_category_id = 0u64;
    for row in category_rows {
        let category = category_from_row(
            row.id,
            &row.name,
            &row.description,
            row.color_index,
            row.balance_effect,
        )?;
        max_category_id = max_category_id.max(category.id.0);
        if row.archived_at_utc.is_some() {
            archived_categories.push(category);
        } else {
            active_categories.push(category);
        }
    }
    active_categories.sort_by_key(|category| category_sort_order(database_path, category.id.0));

    let mut sessions = Vec::with_capacity(session_rows.len());
    let mut max_session_id = 0usize;
    for row in session_rows {
        let id = usize::try_from(row.id)
            .map_err(|_| format!("Session ID {} is outside the supported range", row.id))?;
        let category_id = u64::try_from(row.category_id).map_err(|_| {
            format!(
                "Session {} has category ID {} outside the supported range",
                row.id, row.category_id
            )
        })?;
        let elapsed_seconds = usize::try_from(row.elapsed_seconds).map_err(|_| {
            format!(
                "Session {} duration {} is outside the supported range",
                row.id, row.elapsed_seconds
            )
        })?;
        max_session_id = max_session_id.max(id);
        let started_at_utc = parse_utc(&row.started_at_utc)?;
        let ended_at_utc = parse_utc(&row.ended_at_utc)?;
        let operational_day_policy =
            match (row.boundary_utc_offset_seconds, row.boundary_start_minutes) {
                (Some(offset), Some(start_minutes)) => Some(OperationalDayPolicy {
                    utc_offset_seconds: i32::try_from(offset).map_err(|_| {
                        format!("Session {} boundary UTC offset is outside i32", row.id)
                    })?,
                    start_minutes: u16::try_from(start_minutes).map_err(|_| {
                        format!("Session {} boundary start minute is outside u16", row.id)
                    })?,
                }),
                (None, None) => None,
                _ => {
                    return Err(format!(
                        "Session {} has partial boundary provenance",
                        row.id
                    ));
                }
            };
        sessions.push(Session {
            id,
            date: row.operational_day,
            category_id: CategoryId::new(category_id),
            project: row.project,
            description: row.description,
            start_time: local_clock(&row.started_at_utc, operational_day_policy)?,
            end_time: local_clock(&row.ended_at_utc, operational_day_policy)?,
            elapsed_seconds,
            started_at_utc: Some(started_at_utc),
            ended_at_utc: Some(ended_at_utc),
            operational_day_policy,
        });
    }

    let mut tags_by_category = BTreeMap::new();
    for (category_id, tags) in tag_rows {
        let category_id = u64::try_from(category_id)
            .map_err(|_| format!("Category tag identity {category_id} is invalid"))?;
        tags_by_category.insert(category_id, tags);
    }

    let active_session = active_row
        .map(|row| {
            let category_id = u64::try_from(row.category_id).map_err(|_| {
                format!(
                    "Active session category {} is outside the supported range",
                    row.category_id
                )
            })?;
            let started_at_utc = DateTime::parse_from_rfc3339(&row.started_at_utc)
                .map_err(|error| format!("Invalid active-session timestamp: {error}"))?
                .with_timezone(&Utc);
            Ok::<SqliteTuiActiveSession, String>(SqliteTuiActiveSession {
                stable_id: row.stable_id,
                category_id: CategoryId::new(category_id),
                description: row.description,
                started_at_utc,
            })
        })
        .transpose()?;

    Ok(SqliteTuiState {
        loaded_categories: LoadedCategories {
            categories: active_categories,
            next_category_id: max_category_id.saturating_add(1).max(1),
        },
        loaded_sessions: LoadedSessions {
            sessions,
            next_session_id: max_session_id.saturating_add(1).max(1),
        },
        archived_categories,
        category_tags: CategoryTagsState {
            version: CategoryTagsState::VERSION,
            tags_by_category: tags_by_category.into_iter().collect(),
        },
        active_session,
    })
}

pub(crate) fn ensure_active_session(
    database_path: &Path,
    category_id: CategoryId,
    description: &str,
    started_at_utc: DateTime<Utc>,
) -> Result<String, String> {
    let mut repository = open_cli_repository(database_path)?;
    let stable_id = stable_id("tui", started_at_utc);
    let started = timestamp(started_at_utc);
    runtime_coordination::start_active_session(
        &mut repository,
        &NewActiveSession {
            stable_id: &stable_id,
            project: "",
            category_id: as_i64(category_id.0, "category ID")?,
            description,
            started_at_utc: &started,
            recovery_kind: "live",
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(stable_id)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn switch_active_session(
    database_path: &Path,
    expected_active_stable_id: &str,
    operation_id: &str,
    next_stable_id: &str,
    next_category_id: CategoryId,
    next_description: &str,
    switched_at_utc: DateTime<Utc>,
    operational_day: &str,
    elapsed_seconds: usize,
) -> Result<runtime_coordination::RuntimeTransitionReceipt, String> {
    let mut repository = open_cli_repository(database_path)?;
    let switched = timestamp(switched_at_utc);
    let policy = OperationalDayPolicy::from_config(day_boundary_config());
    runtime_coordination::switch_active_session(
        &mut repository,
        expected_active_stable_id,
        operation_id,
        &SessionCompletion {
            ended_at_utc: &switched,
            operational_day,
            elapsed_seconds: as_i64(elapsed_seconds as u64, "elapsed seconds")?,
            boundary_utc_offset_seconds: policy.utc_offset_seconds,
            boundary_start_minutes: policy.start_minutes,
            source: "tui-runtime",
        },
        &NewActiveSession {
            stable_id: next_stable_id,
            project: "",
            category_id: as_i64(next_category_id.0, "category ID")?,
            description: next_description,
            started_at_utc: &switched,
            recovery_kind: "live",
        },
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn finish_active_session(
    database_path: &Path,
    expected_active_stable_id: &str,
    operation_id: &str,
    ended_at_utc: DateTime<Utc>,
    operational_day: &str,
    elapsed_seconds: usize,
) -> Result<runtime_coordination::RuntimeTransitionReceipt, String> {
    let mut repository = open_cli_repository(database_path)?;
    let ended = timestamp(ended_at_utc);
    let policy = OperationalDayPolicy::from_config(day_boundary_config());
    runtime_coordination::finish_active_session(
        &mut repository,
        expected_active_stable_id,
        operation_id,
        &SessionCompletion {
            ended_at_utc: &ended,
            operational_day,
            elapsed_seconds: as_i64(elapsed_seconds as u64, "elapsed seconds")?,
            boundary_utc_offset_seconds: policy.utc_offset_seconds,
            boundary_start_minutes: policy.start_minutes,
            source: "tui-runtime",
        },
        true,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn reset_active_session(
    database_path: &Path,
    expected_active_stable_id: &str,
    operation_id: &str,
    next_stable_id: &str,
    started_at_utc: DateTime<Utc>,
) -> Result<runtime_coordination::RuntimeTransitionReceipt, String> {
    let mut repository = open_cli_repository(database_path)?;
    let active = repository
        .active_session()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "there is no active TUI session to reset".to_string())?;
    let started = timestamp(started_at_utc);
    runtime_coordination::reset_active_session(
        &mut repository,
        expected_active_stable_id,
        operation_id,
        &NewActiveSession {
            stable_id: next_stable_id,
            project: &active.project,
            category_id: active.category_id,
            description: &active.description,
            started_at_utc: &started,
            recovery_kind: "live",
        },
        &started,
        "tui-runtime",
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn sync_categories(
    database_path: &Path,
    categories: &[Category],
    active_category_id: CategoryId,
    expected_active_stable_id: Option<&str>,
) -> Result<Vec<Category>, String> {
    runtime_coordination::maybe_inject_test_fault("category-sync", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;

    let current_ids = categories
        .iter()
        .map(|category| as_i64(category.id.0, "category ID"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if !current_ids.contains(&0) {
        return Err("the active category set is missing the reserved idle category".to_string());
    }
    let active_id = as_i64(active_category_id.0, "active category ID")?;
    if !current_ids.contains(&active_id) {
        return Err("the active category cannot be archived".to_string());
    }

    for (sort_order, category) in categories.iter().enumerate() {
        let id = as_i64(category.id.0, "category ID")?;
        let name = if id == 0 {
            "idle".to_string()
        } else {
            category.name.trim().to_string()
        };
        let color_index = COLORS
            .iter()
            .position(|color| *color == category.color)
            .unwrap_or(0);
        transaction
            .execute(
                "INSERT INTO categories (
                    id, name, description, color_index, balance_effect, archived_at_utc, sort_order
                 ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
                    color_index = excluded.color_index,
                    balance_effect = excluded.balance_effect,
                    archived_at_utc = NULL,
                    sort_order = excluded.sort_order",
                params![
                    id,
                    name,
                    category.description,
                    i64::try_from(color_index).unwrap_or(0),
                    i64::from(category.karma_effect),
                    i64::try_from(sort_order).map_err(|_| "too many categories".to_string())?,
                ],
            )
            .map_err(|error| error.to_string())?;
    }

    if let Some(expected_active_stable_id) = expected_active_stable_id {
        let active_description = categories
            .iter()
            .find(|category| category.id == active_category_id)
            .map(|category| category.description.as_str())
            .unwrap_or_default();
        let changed = transaction
            .execute(
                "UPDATE active_session SET description = ?1
                 WHERE singleton = 1 AND category_id = ?2 AND stable_id = ?3",
                params![active_description, active_id, expected_active_stable_id],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            let actual: Option<String> = transaction
                .query_row(
                    "SELECT stable_id FROM active_session WHERE singleton = 1",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?;
            return Err(format!(
                "active session changed concurrently; expected {}, found {}",
                expected_active_stable_id,
                actual.unwrap_or_else(|| "no active session".to_string())
            ));
        }
    }
    runtime_coordination::maybe_inject_test_fault("category-sync", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;

    Ok(load_state(database_path)?.archived_categories)
}

pub(crate) fn archive_category(
    database_path: &Path,
    category_id: CategoryId,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("category-archive", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let category_id = as_i64(category_id.0, "category ID")?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let active_category_id: Option<i64> = transaction
        .query_row(
            "SELECT category_id FROM active_session WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if active_category_id == Some(category_id) {
        return Err("the active category cannot be archived".to_string());
    }
    let changed = transaction
        .execute(
            "UPDATE categories
             SET archived_at_utc = ?1
             WHERE id = ?2 AND archived_at_utc IS NULL",
            params![timestamp(Utc::now()), category_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("active category {category_id} does not exist"));
    }
    runtime_coordination::maybe_inject_test_fault("category-archive", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn sync_category_tags(
    database_path: &Path,
    tags: &CategoryTagsState,
    category_ids: &[CategoryId],
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("category-tags", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    for category_id in category_ids {
        let category_id = as_i64(category_id.0, "category ID")?;
        let category_id_u64 = u64::try_from(category_id)
            .map_err(|_| format!("Category ID {category_id} is invalid"))?;
        let values = tags
            .tags_by_category
            .get(&category_id_u64)
            .cloned()
            .unwrap_or_default();
        let mut normalized = Vec::with_capacity(values.len());
        let mut seen = BTreeSet::new();
        for value in values {
            let value = value.trim();
            if value.is_empty() {
                return Err("category tag is empty".to_string());
            }
            if !seen.insert(value.to_string()) {
                return Err(format!("duplicate category tag '{value}'"));
            }
            normalized.push(value.to_string());
        }
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id = ?1)",
                params![category_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if !exists {
            return Err(format!("category {category_id} does not exist"));
        }
        transaction
            .execute(
                "DELETE FROM category_tags WHERE category_id = ?1",
                params![category_id],
            )
            .map_err(|error| error.to_string())?;
        for (ordinal, tag) in normalized.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO category_tags(category_id, ordinal, tag)
                     VALUES (?1, ?2, ?3)",
                    params![
                        category_id,
                        i64::try_from(ordinal).map_err(|_| "too many category tags".to_string())?,
                        tag,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    runtime_coordination::maybe_inject_test_fault("category-tags", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn sync_sessions(database_path: &Path, sessions: &[Session]) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("session-sync", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let mut statement = transaction
        .prepare(
            "SELECT id, project, category_id, operational_day, elapsed_seconds FROM sessions ORDER BY id",
        )
        .map_err(|error| error.to_string())?;
    let stored = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                (
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ),
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);

    for session in sessions {
        let id = i64::try_from(session.id).map_err(|_| "session ID is too large".to_string())?;
        let expected = stored.get(&id).ok_or_else(|| {
            format!(
                "TUI session {id} does not exist in SQLite; runtime insertion must use the active-session transaction"
            )
        })?;
        let category_id = as_i64(session.category_id.0, "category ID")?;
        let elapsed = as_i64(session.elapsed_seconds as u64, "elapsed seconds")?;
        if expected.0 != session.project
            || expected.1 != category_id
            || expected.2 != session.date
            || expected.3 != elapsed
        {
            return Err(format!(
                "TUI session {id} identity or chronology diverged from SQLite authority"
            ));
        }
        transaction
            .execute(
                "UPDATE sessions SET description = ?1 WHERE id = ?2",
                params![session.description, id],
            )
            .map_err(|error| error.to_string())?;
    }
    runtime_coordination::maybe_inject_test_fault("session-sync", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn update_session_description(
    database_path: &Path,
    session_id: usize,
    description: &str,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("session-edit", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let session_id =
        i64::try_from(session_id).map_err(|_| "session ID is too large".to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let changed = transaction
        .execute(
            "UPDATE sessions SET description = ?1 WHERE id = ?2",
            params![description, session_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    runtime_coordination::maybe_inject_test_fault("session-edit", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn delete_session(database_path: &Path, session_id: usize) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("session-delete", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let session_id =
        i64::try_from(session_id).map_err(|_| "session ID is too large".to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let changed = transaction
        .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    runtime_coordination::maybe_inject_test_fault("session-delete", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn delete_drift_sessions_for_day(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("drift-session-delete", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM sessions WHERE category_id = 0 AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("drift-session-delete", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn save_sand_state(database_path: &Path, state: &SandState) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("sand-state", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let existing = repository.sand_state().map_err(|error| error.to_string())?;
    let formation_id = existing
        .as_ref()
        .map(|record| record.formation_id.as_str())
        .unwrap_or("default");
    let quantum_seconds = existing
        .as_ref()
        .map(|record| record.quantum_seconds)
        .unwrap_or(1);
    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;
    let grid_width =
        i64::try_from(state.grid_width).map_err(|_| "sand width is too large".to_string())?;
    let grid_height =
        i64::try_from(state.grid_height).map_err(|_| "sand height is too large".to_string())?;
    let updated_at_utc = timestamp(Utc::now());
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
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
                formation_id,
                quantum_seconds,
                grid_width,
                grid_height,
                payload_json,
                updated_at_utc,
            ],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("sand-state", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn load_sand_state(database_path: &Path) -> Result<Option<SandState>, String> {
    let repository = open_cli_repository(database_path)?;
    repository
        .sand_state()
        .map_err(|error| error.to_string())?
        .map(|record| serde_json::from_str(&record.payload_json).map_err(|error| error.to_string()))
        .transpose()
}

pub(crate) fn save_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
    state: &SandState,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("daily-snapshot", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let existing = repository.sand_state().map_err(|error| error.to_string())?;
    let formation_id = existing
        .as_ref()
        .map(|record| record.formation_id.as_str())
        .unwrap_or("default");
    let quantum_seconds = existing
        .as_ref()
        .map(|record| record.quantum_seconds)
        .unwrap_or(1);
    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily' AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO sand_snapshots (
                formation_id, snapshot_kind, operational_day, quantum_seconds,
                payload_json, captured_at_utc
             ) VALUES (?1, 'daily', ?2, ?3, ?4, ?5)",
            params![
                formation_id,
                operational_day,
                quantum_seconds,
                payload_json,
                timestamp(Utc::now()),
            ],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("daily-snapshot", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn load_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
) -> Result<Option<SandState>, String> {
    let repository = open_cli_repository(database_path)?;
    let payload: Option<String> = repository
        .connection
        .query_row(
            "SELECT payload_json FROM sand_snapshots
             WHERE snapshot_kind = 'daily' AND operational_day = ?1
             ORDER BY id DESC LIMIT 1",
            params![operational_day],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    payload
        .map(|value| serde_json::from_str(&value).map_err(|error| error.to_string()))
        .transpose()
}

pub(crate) fn delete_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("daily-snapshot-delete", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily' AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("daily-snapshot-delete", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

#[derive(Debug)]
pub(crate) struct SqliteClaimedCheckpoint<T> {
    pub active_session_stable_id: Option<String>,
    pub payload: T,
}

pub(crate) fn save_checkpoint<T: Serialize>(
    database_path: &Path,
    expected_active_stable_id: &str,
    detached_at_utc: DateTime<Utc>,
    simulation_time_utc: DateTime<Utc>,
    payload: &T,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let payload_json = serde_json::to_string(payload).map_err(|error| error.to_string())?;
    runtime_coordination::save_checkpoint(
        &mut repository,
        expected_active_stable_id,
        &timestamp(detached_at_utc),
        &timestamp(simulation_time_utc),
        &payload_json,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn load_checkpoint<T: DeserializeOwned>(
    database_path: &Path,
) -> Result<Option<SqliteClaimedCheckpoint<T>>, String> {
    let mut repository = open_cli_repository(database_path)?;
    let Some(claimed) = runtime_coordination::claim_checkpoint(&mut repository)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    match serde_json::from_str(&claimed.payload_json) {
        Ok(payload) => Ok(Some(SqliteClaimedCheckpoint {
            active_session_stable_id: claimed.active_session_stable_id,
            payload,
        })),
        Err(error) => {
            runtime_coordination::quarantine_checkpoint(&mut repository)
                .map_err(|quarantine_error| quarantine_error.to_string())?;
            Err(format!("Invalid runtime checkpoint payload: {error}"))
        }
    }
}

pub(crate) fn quarantine_checkpoint(database_path: &Path) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    runtime_coordination::quarantine_checkpoint(&mut repository).map_err(|error| error.to_string())
}

pub(crate) fn commit_checkpoint_recovery(
    database_path: &Path,
    expected_active_stable_id: &str,
    operational_day: &str,
    state: &SandState,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let existing = repository.sand_state().map_err(|error| error.to_string())?;
    let formation_id = existing
        .as_ref()
        .map(|record| record.formation_id.as_str())
        .unwrap_or("default");
    let quantum_seconds = existing
        .as_ref()
        .map(|record| record.quantum_seconds)
        .unwrap_or(1);
    let now = timestamp(Utc::now());
    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;
    runtime_coordination::commit_checkpoint_recovery(
        &mut repository,
        expected_active_stable_id,
        operational_day,
        &SandStateRecord {
            formation_id: formation_id.to_string(),
            quantum_seconds,
            grid_width: i64::try_from(state.grid_width)
                .map_err(|_| "sand width is too large".to_string())?,
            grid_height: i64::try_from(state.grid_height)
                .map_err(|_| "sand height is too large".to_string())?,
            payload_json,
            updated_at_utc: now.clone(),
        },
        &now,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn clear_checkpoint(database_path: &Path) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    runtime_coordination::clear_committed_checkpoint(&mut repository)
        .map_err(|error| error.to_string())
}

fn category_from_row(
    id: i64,
    stored_name: &str,
    description: &str,
    color_index: i64,
    balance_effect: i64,
) -> Result<Category, String> {
    let id = u64::try_from(id).map_err(|_| format!("Category ID {id} is invalid"))?;
    let color_index = usize::try_from(color_index)
        .map_err(|_| format!("Category color index {color_index} is invalid"))?;
    let karma_effect = i8::try_from(balance_effect)
        .map_err(|_| format!("Category balance {balance_effect} is invalid"))?;
    Ok(Category {
        id: CategoryId::new(id),
        name: if id == 0 {
            DRIFT_CATEGORY_CONFIG_NAME.to_string()
        } else {
            stored_name.to_string()
        },
        color: COLORS[color_index % COLORS.len()],
        description: description.to_string(),
        karma_effect,
    })
}

fn category_sort_order(database_path: &Path, category_id: u64) -> i64 {
    let Ok(repository) = open_cli_repository(database_path) else {
        return i64::try_from(category_id).unwrap_or(i64::MAX);
    };
    repository
        .connection
        .query_row(
            "SELECT sort_order FROM categories WHERE id = ?1",
            params![i64::try_from(category_id).unwrap_or(i64::MAX)],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| i64::try_from(category_id).unwrap_or(i64::MAX))
}

fn parse_utc(timestamp_value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(timestamp_value)
        .map_err(|error| format!("Invalid SQLite timestamp '{timestamp_value}': {error}"))
        .map(|value| value.with_timezone(&Utc))
}

fn local_clock(
    timestamp_value: &str,
    policy: Option<OperationalDayPolicy>,
) -> Result<String, String> {
    let utc = parse_utc(timestamp_value)?;
    let offset_seconds = policy
        .map(|value| value.utc_offset_seconds)
        .unwrap_or_else(|| runtime_settings().day_boundary.utc_offset_seconds);
    let offset = FixedOffset::east_opt(offset_seconds)
        .ok_or_else(|| format!("Configured UTC offset {offset_seconds} is invalid"))?;
    Ok(utc.with_timezone(&offset).format("%H:%M:%S").to_string())
}

fn stable_id(prefix: &str, now: DateTime<Utc>) -> String {
    format!(
        "{prefix}-{}-{}",
        now.to_rfc3339_opts(SecondsFormat::Nanos, true),
        process::id()
    )
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn as_i64(value: u64, label: &str) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("{label} {value} is outside SQLite's supported range"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::sqlite::{SqliteRepository, repository::NewCategoryRecord};

    fn repository_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-sqlite007-{name}-{}-{}.sqlite3",
            process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[test]
    fn category_order_and_archival_round_trip() {
        let path = repository_file("categories");
        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .transition_storage_authority("sqlite-candidate", "sqlite-cli", "2026-08-01T12:00:00Z")
            .unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Rest",
                description: "",
                color_index: 1,
                balance_effect: -1,
            })
            .unwrap();
        drop(repository);

        let mut state = load_state(&path).unwrap();
        state.loaded_categories.categories.swap(1, 2);
        state.loaded_categories.categories.pop();
        let archived = sync_categories(
            &path,
            &state.loaded_categories.categories,
            CategoryId::new(0),
            None,
        )
        .unwrap();
        assert_eq!(archived.len(), 0, "sync alone must not archive absent rows");
        archive_category(&path, CategoryId::new(1)).unwrap();
        let mut reloaded = load_state(&path).unwrap();
        assert_eq!(reloaded.loaded_categories.categories[1].name, "Rest");
        assert_eq!(reloaded.archived_categories[0].name, "Work");
        let restored = reloaded.archived_categories.remove(0);
        reloaded.loaded_categories.categories.push(restored);
        sync_categories(
            &path,
            &reloaded.loaded_categories.categories,
            CategoryId::new(0),
            None,
        )
        .unwrap();
        let restored = load_state(&path).unwrap();
        assert_eq!(
            restored.loaded_categories.categories[2].id,
            CategoryId::new(1)
        );
        assert!(restored.archived_categories.is_empty());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn session_sync_preserves_project_and_chronology() {
        let path = repository_file("sessions");
        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .transition_storage_authority("sqlite-candidate", "sqlite-cli", "2026-08-01T12:00:00Z")
            .unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO sessions (
                    id, stable_id, project, category_id, description, started_at_utc,
                    ended_at_utc, operational_day, elapsed_seconds, source
                 ) VALUES (7, 'stable', 'preserved-project', 1, 'old',
                    '2026-08-01T12:00:00Z', '2026-08-01T13:00:00Z',
                    '2026-08-01', 3600, 'cli-runtime')",
                [],
            )
            .unwrap();
        drop(repository);

        let mut state = load_state(&path).unwrap();
        assert_eq!(
            state.loaded_sessions.sessions[0].project,
            "preserved-project"
        );
        state.loaded_sessions.sessions[0].description = "edited".to_string();
        sync_sessions(&path, &state.loaded_sessions.sessions).unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let row: (String, String, String) = repository
            .connection
            .query_row(
                "SELECT project, description, started_at_utc FROM sessions WHERE id = 7",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(row.0, "preserved-project");
        assert_eq!(row.1, "edited");
        assert_eq!(row.2, "2026-08-01T12:00:00Z");

        repository
            .connection
            .execute(
                "INSERT INTO sessions (
                    id, stable_id, project, category_id, description, started_at_utc,
                    ended_at_utc, operational_day, elapsed_seconds, source
                 ) VALUES (8, 'concurrent', 'external-project', 1, 'external',
                    '2026-08-01T14:00:00Z', '2026-08-01T15:00:00Z',
                    '2026-08-01', 3600, 'cli-runtime')",
                [],
            )
            .unwrap();
        drop(repository);
        sync_sessions(&path, &state.loaded_sessions.sessions).unwrap();
        update_session_description(&path, 7, "explicit-edit").unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let preserved: (String, String, String, String) = repository
            .connection
            .query_row(
                "SELECT project, description, started_at_utc, source FROM sessions WHERE id = 7",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(preserved.0, "preserved-project");
        assert_eq!(preserved.1, "explicit-edit");
        assert_eq!(preserved.2, "2026-08-01T12:00:00Z");
        assert_eq!(preserved.3, "cli-runtime");
        let concurrent_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sessions WHERE id = 8", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(concurrent_count, 1);
        drop(repository);
        delete_session(&path, 7).unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let remaining: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            remaining, 1,
            "explicit deletion must not remove concurrent rows"
        );
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn sand_and_checkpoint_round_trip() {
        let path = repository_file("runtime-state");
        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .transition_storage_authority("sqlite-candidate", "sqlite-cli", "2026-08-01T12:00:00Z")
            .unwrap();
        repository
            .start_session(&NewActiveSession {
                stable_id: "checkpoint-active",
                project: "",
                category_id: 0,
                description: "",
                started_at_utc: "2026-08-01T12:00:00Z",
                recovery_kind: "live",
            })
            .unwrap();
        drop(repository);
        let state = SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: Vec::new(),
            frame_count: 3,
            sweep_left_to_right: true,
            rng_state: 9,
            pending_grains: Vec::new(),
        };
        save_sand_state(&path, &state).unwrap();
        save_daily_snapshot(&path, "2026-08-01", &state).unwrap();
        save_checkpoint(
            &path,
            "checkpoint-active",
            Utc::now(),
            Utc::now(),
            &BTreeMap::from([("status", "detached")]),
        )
        .unwrap();
        assert_eq!(load_sand_state(&path).unwrap(), Some(state.clone()));
        assert_eq!(
            load_daily_snapshot(&path, "2026-08-01").unwrap(),
            Some(state.clone())
        );
        let checkpoint: Option<SqliteClaimedCheckpoint<BTreeMap<String, String>>> =
            load_checkpoint(&path).unwrap();
        assert_eq!(
            checkpoint.unwrap().payload.get("status").unwrap(),
            "detached"
        );
        commit_checkpoint_recovery(&path, "checkpoint-active", "2026-08-01", &state).unwrap();
        clear_checkpoint(&path).unwrap();
        assert!(
            load_checkpoint::<BTreeMap<String, String>>(&path)
                .unwrap()
                .is_none()
        );
        std::fs::remove_file(path).ok();
    }
}
