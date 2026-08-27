use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process,
};

use chrono::{
    DateTime, Duration as ChronoDuration, FixedOffset, NaiveDate, SecondsFormat, Timelike, Utc,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    constants::COLORS,
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, OperationalDayPolicy, Session,
        day_boundary_config, runtime_settings,
    },
    sand::{DailySedimentSlice, SandState, SedimentSnapshot, daily_contribution_from_slices},
    temporal,
    storage::{CategoryTagsState, LoadedCategories, LoadedSessions},
};

use super::{
    NewActiveSession, SessionCompletion, repository::SandStateRecord, runtime::open_cli_repository,
    runtime_coordination,
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

#[derive(Debug, Clone)]
pub(crate) struct HistoricalActivePreview {
    pub stable_id: String,
    pub category_id: CategoryId,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: DateTime<Utc>,
    pub elapsed_seconds: usize,
    pub operational_day_policy: OperationalDayPolicy,
}

#[derive(Debug, Clone)]
pub(crate) struct HistoricalMissedActivityRequest {
    pub source_session_id: usize,
    pub target_category_id: CategoryId,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: DateTime<Utc>,
    pub description: String,
    pub active_preview: Option<HistoricalActivePreview>,
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
            category_id: as_i64(category_id.0, "category ID")?,
            description,
            started_at_utc: &started,
            recovery_kind: "live",
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(stable_id)
}

pub(crate) fn initial_active_stable_id(started_at_utc: DateTime<Utc>) -> String {
    stable_id("tui", started_at_utc)
}

pub(crate) struct InitialActiveGenerationRequest<'a, T> {
    pub active_stable_id: &'a str,
    pub category_id: CategoryId,
    pub description: &'a str,
    pub started_at_utc: DateTime<Utc>,
    pub detached_at_utc: DateTime<Utc>,
    pub simulation_time_utc: DateTime<Utc>,
    pub checkpoint: &'a T,
}

pub(crate) fn start_active_session_with_checkpoint<T: Serialize>(
    database_path: &Path,
    request: InitialActiveGenerationRequest<'_, T>,
) -> Result<(), String> {
    let InitialActiveGenerationRequest {
        active_stable_id,
        category_id,
        description,
        started_at_utc,
        detached_at_utc,
        simulation_time_utc,
        checkpoint,
    } = request;
    let mut repository = open_cli_repository(database_path)?;
    let started = timestamp(started_at_utc);
    let payload_json = serde_json::to_string(checkpoint).map_err(|error| error.to_string())?;
    runtime_coordination::start_active_session_with_checkpoint(
        &mut repository,
        &NewActiveSession {
            stable_id: active_stable_id,
            category_id: as_i64(category_id.0, "category ID")?,
            description,
            started_at_utc: &started,
            recovery_kind: "live",
        },
        &timestamp(detached_at_utc),
        &timestamp(simulation_time_utc),
        &payload_json,
    )
    .map_err(|error| error.to_string())
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
                    i64::from(category.balance_effect),
                    i64::try_from(sort_order).map_err(|_| "too many categories".to_string())?,
                ],
            )
            .map_err(|error| error.to_string())?;
    }

    if let Some(expected_active_stable_id) = expected_active_stable_id {
        let actual: Option<(i64, String)> = transaction
            .query_row(
                "SELECT category_id, stable_id FROM active_session WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if actual.as_ref().is_none_or(|(category_id, stable_id)| {
            *category_id != active_id || stable_id.as_str() != expected_active_stable_id
        }) {
            return Err(format!(
                "active session changed concurrently; expected {} on category {}, found {}",
                expected_active_stable_id,
                active_id,
                actual
                    .map(|(category_id, stable_id)| format!(
                        "{stable_id} on category {category_id}"
                    ))
                    .unwrap_or_else(|| "no active session".to_string())
            ));
        }
    }
    runtime_coordination::maybe_inject_test_fault("category-sync", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;

    Ok(load_state(database_path)?.archived_categories)
}

pub(crate) fn update_active_description(
    database_path: &Path,
    expected_active_stable_id: &str,
    description: &str,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    runtime_coordination::update_active_description(
        &mut repository,
        expected_active_stable_id,
        description,
    )
    .map_err(|error| error.to_string())
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
            "SELECT id, category_id, operational_day, elapsed_seconds FROM sessions ORDER BY id",
        )
        .map_err(|error| error.to_string())?;
    let stored = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                (
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
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
        if expected.0 != category_id || expected.1 != session.date || expected.2 != elapsed {
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

fn correction_stable_id(
    source_stable_id: &str,
    role: &str,
    started_at_utc: DateTime<Utc>,
    category_id: CategoryId,
) -> String {
    format!(
        "history:{source_stable_id}:{role}:{}:{}",
        started_at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true),
        category_id.0,
    )
}

fn fragment_operational_day(
    ended_at_utc: DateTime<Utc>,
    policy: OperationalDayPolicy,
) -> Result<NaiveDate, String> {
    temporal::operational_day_from_policy(ended_at_utc, policy)
}

fn write_history_fragment(
    connection: &rusqlite::Connection,
    existing_id: Option<i64>,
    stable_id: &str,
    category_id: CategoryId,
    description: &str,
    started_at_utc: DateTime<Utc>,
    ended_at_utc: DateTime<Utc>,
    elapsed_seconds: usize,
    policy: OperationalDayPolicy,
) -> Result<(), String> {
    if elapsed_seconds == 0 {
        return Err("historical correction refuses a zero-length fragment".to_string());
    }
    let operational_day = fragment_operational_day(ended_at_utc, policy)?;
    let category_id = as_i64(category_id.0, "category ID")?;
    let elapsed_seconds = i64::try_from(elapsed_seconds)
        .map_err(|_| "historical correction duration is too large".to_string())?;
    let started = timestamp(started_at_utc);
    let ended = timestamp(ended_at_utc);
    let day = operational_day.format("%Y-%m-%d").to_string();
    const SOURCE: &str = "tui-history-correction";

    if let Some(id) = existing_id {
        let changed = connection
            .execute(
                "UPDATE sessions
                 SET stable_id = ?1,
                     category_id = ?2,
                     description = ?3,
                     started_at_utc = ?4,
                     ended_at_utc = ?5,
                     operational_day = ?6,
                     elapsed_seconds = ?7,
                     boundary_utc_offset_seconds = ?8,
                     boundary_start_minutes = ?9,
                     source = ?10
                 WHERE id = ?11",
                params![
                    stable_id,
                    category_id,
                    description,
                    started,
                    ended,
                    day,
                    elapsed_seconds,
                    policy.utc_offset_seconds,
                    policy.start_minutes,
                    SOURCE,
                    id,
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err(format!("SQLite session {id} disappeared during historical correction"));
        }
    } else {
        connection
            .execute(
                "INSERT INTO sessions (
                    stable_id, category_id, description,
                    started_at_utc, ended_at_utc, operational_day,
                    elapsed_seconds, boundary_utc_offset_seconds,
                    boundary_start_minutes, source
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    stable_id,
                    category_id,
                    description,
                    started,
                    ended,
                    day,
                    elapsed_seconds,
                    policy.utc_offset_seconds,
                    policy.start_minutes,
                    SOURCE,
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn daily_slices_from_session_rows(
    sessions: &[super::repository::SessionRecord],
    day: NaiveDate,
) -> Result<Vec<DailySedimentSlice>, String> {
    let mut result = Vec::new();
    for session in sessions {
        let elapsed_seconds = usize::try_from(session.elapsed_seconds)
            .map_err(|_| format!("session {} duration is invalid", session.id))?;
        if elapsed_seconds == 0 {
            continue;
        }
        let session_id = usize::try_from(session.id)
            .map_err(|_| format!("session {} ID is outside the supported range", session.id))?;
        let category_id = u64::try_from(session.category_id)
            .map_err(|_| format!("session {} category identity is invalid", session.id))?;

        match (
            session.boundary_utc_offset_seconds,
            session.boundary_start_minutes,
        ) {
            (Some(offset), Some(start_minutes)) => {
                let policy = OperationalDayPolicy {
                    utc_offset_seconds: i32::try_from(offset).map_err(|_| {
                        format!("session {} boundary offset is invalid", session.id)
                    })?,
                    start_minutes: u16::try_from(start_minutes).map_err(|_| {
                        format!("session {} boundary start is invalid", session.id)
                    })?,
                };
                let started_at_utc = parse_utc(&session.started_at_utc)?;
                let ended_at_utc = parse_utc(&session.ended_at_utc)?;
                for slice in temporal::allocate_operational_day_slices(
                    started_at_utc,
                    ended_at_utc,
                    elapsed_seconds,
                    policy,
                )? {
                    if slice.operational_day != day {
                        continue;
                    }
                    result.push(DailySedimentSlice {
                        category_id,
                        elapsed_seconds: slice.elapsed_seconds,
                        start_time: temporal::civil_from_policy(slice.started_at_utc, policy)?
                            .format("%H:%M:%S")
                            .to_string(),
                        end_time: temporal::civil_from_policy(slice.ended_at_utc, policy)?
                            .format("%H:%M:%S")
                            .to_string(),
                        session_id,
                    });
                }
            }
            (None, None) => {
                let Ok(stored_day) =
                    NaiveDate::parse_from_str(&session.operational_day, "%Y-%m-%d")
                else {
                    continue;
                };
                if stored_day != day {
                    continue;
                }
                result.push(DailySedimentSlice {
                    category_id,
                    elapsed_seconds,
                    start_time: local_clock(&session.started_at_utc, None)?,
                    end_time: local_clock(&session.ended_at_utc, None)?,
                    session_id,
                });
            }
            _ => {
                return Err(format!(
                    "session {} has partial boundary provenance during historical correction",
                    session.id
                ));
            }
        }
    }
    Ok(result)
}

fn daily_slices_from_active_preview(
    preview: &HistoricalActivePreview,
    day: NaiveDate,
) -> Result<Vec<DailySedimentSlice>, String> {
    if preview.elapsed_seconds == 0 {
        return Ok(Vec::new());
    }
    let slices = temporal::allocate_operational_day_slices(
        preview.started_at_utc,
        preview.ended_at_utc,
        preview.elapsed_seconds,
        preview.operational_day_policy,
    )?;
    let mut result = Vec::new();
    for slice in slices {
        if slice.operational_day != day {
            continue;
        }
        result.push(DailySedimentSlice {
            category_id: preview.category_id.0,
            elapsed_seconds: slice.elapsed_seconds,
            start_time: temporal::civil_from_policy(
                slice.started_at_utc,
                preview.operational_day_policy,
            )?
            .format("%H:%M:%S")
            .to_string(),
            end_time: temporal::civil_from_policy(
                slice.ended_at_utc,
                preview.operational_day_policy,
            )?
            .format("%H:%M:%S")
            .to_string(),
            session_id: usize::MAX,
        });
    }
    Ok(result)
}

fn validate_historical_active_preview(
    transaction: &rusqlite::Transaction<'_>,
    preview: Option<&HistoricalActivePreview>,
) -> Result<(), String> {
    let persisted: Option<(String, i64, String)> = transaction
        .query_row(
            "SELECT stable_id, category_id, started_at_utc
             FROM active_session WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;

    match (persisted, preview) {
        (None, None) => Ok(()),
        (Some((stable_id, _, _)), None) => Err(format!(
            "historical correction is missing preview for active generation {stable_id}"
        )),
        (None, Some(preview)) => Err(format!(
            "historical correction preview {} has no persisted active generation",
            preview.stable_id
        )),
        (Some((stable_id, category_id, started_at_utc)), Some(preview)) => {
            let preview_category = as_i64(preview.category_id.0, "active category ID")?;
            let persisted_start = parse_utc(&started_at_utc)?;
            if stable_id != preview.stable_id
                || category_id != preview_category
                || persisted_start != preview.started_at_utc
            {
                return Err("historical correction active preview is stale".to_string());
            }
            let elapsed = i64::try_from(preview.elapsed_seconds)
                .map_err(|_| "active preview duration exceeds chrono range".to_string())?;
            let expected_end = preview
                .started_at_utc
                .checked_add_signed(ChronoDuration::seconds(elapsed))
                .ok_or_else(|| "active preview end exceeds chrono range".to_string())?;
            if expected_end != preview.ended_at_utc {
                return Err(
                    "historical correction active preview does not conserve whole seconds"
                        .to_string(),
                );
            }
            Ok(())
        }
    }
}

fn replace_daily_contributions_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    affected_days: &BTreeSet<NaiveDate>,
    active_preview: Option<&HistoricalActivePreview>,
) -> Result<(), String> {
    let sessions = super::repository::query_all_sessions(transaction)
        .map_err(|error| error.to_string())?;
    let existing_sand: Option<(String, i64)> = transaction
        .query_row(
            "SELECT formation_id, quantum_seconds FROM sand_state WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (formation_id, quantum_seconds) =
        existing_sand.unwrap_or_else(|| ("default".to_string(), 1));
    let captured_at_utc = timestamp(Utc::now());

    for day in affected_days {
        let day_key = day.format("%Y-%m-%d").to_string();
        let mut slices = daily_slices_from_session_rows(&sessions, *day)?;
        if let Some(preview) = active_preview {
            slices.extend(daily_slices_from_active_preview(preview, *day)?);
        }
        let expected = daily_contribution_from_slices(&day_key, &slices);
        transaction
            .execute(
                "DELETE FROM sand_snapshots
                 WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1",
                params![day_key],
            )
            .map_err(|error| error.to_string())?;
        if let Some(snapshot) = expected {
            let payload_json =
                serde_json::to_string(&snapshot).map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO sand_snapshots (
                        formation_id, snapshot_kind, operational_day, quantum_seconds,
                        payload_json, captured_at_utc
                     ) VALUES (?1, 'daily-contribution', ?2, ?3, ?4, ?5)",
                    params![
                        formation_id,
                        day_key,
                        quantum_seconds,
                        payload_json,
                        captured_at_utc,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn log_missed_activity(
    database_path: &Path,
    request: HistoricalMissedActivityRequest,
) -> Result<BTreeSet<NaiveDate>, String> {
    runtime_coordination::maybe_inject_test_fault("history-correction", "before-write")
        .map_err(|error| error.to_string())?;
    if request.target_category_id == crate::domain::DRIFT_CATEGORY_ID {
        return Err("missed activity target must be a non-Idle layer".to_string());
    }
    if request.started_at_utc >= request.ended_at_utc {
        return Err("missed activity From must be before To".to_string());
    }

    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let source_id = i64::try_from(request.source_session_id)
        .map_err(|_| "source session ID is too large".to_string())?;
    let sessions = super::repository::query_all_sessions(&transaction)
        .map_err(|error| error.to_string())?;
    let source = sessions
        .iter()
        .find(|session| session.id == source_id)
        .cloned()
        .ok_or_else(|| format!("SQLite session {source_id} does not exist"))?;
    if source.category_id != 0 {
        return Err("missed activity can only reclassify a completed Idle session".to_string());
    }

    let target_category_id = as_i64(request.target_category_id.0, "category ID")?;
    let target_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM categories
                WHERE id = ?1 AND archived_at_utc IS NULL AND id != 0
             )",
            params![target_category_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !target_exists {
        return Err(format!(
            "target layer {} is not an active non-Idle layer",
            request.target_category_id.0
        ));
    }

    let source_started_at_utc = parse_utc(&source.started_at_utc)?;
    let source_recorded_end_utc = parse_utc(&source.ended_at_utc)?;
    let source_elapsed = usize::try_from(source.elapsed_seconds)
        .map_err(|_| "source session duration is invalid".to_string())?;
    let source_elapsed_i64 = i64::try_from(source_elapsed)
        .map_err(|_| "source session duration exceeds chrono range".to_string())?;
    let source_effective_end_utc = source_started_at_utc
        .checked_add_signed(ChronoDuration::seconds(source_elapsed_i64))
        .ok_or_else(|| "source session end exceeds chrono range".to_string())?;
    if source_recorded_end_utc < source_effective_end_utc {
        return Err("source session ends before its canonical elapsed duration".to_string());
    }
    if request.started_at_utc < source_started_at_utc
        || request.ended_at_utc > source_effective_end_utc
    {
        return Err("missed activity interval is outside the selected Idle session".to_string());
    }
    if request.started_at_utc.nanosecond() != source_started_at_utc.nanosecond()
        || request.ended_at_utc.nanosecond() != source_started_at_utc.nanosecond()
    {
        return Err(
            "missed activity boundaries must align to the source session whole-second lattice"
                .to_string(),
        );
    }

    let Some(source_offset) = source.boundary_utc_offset_seconds else {
        return Err("selected Idle session lacks boundary provenance".to_string());
    };
    let Some(source_start_minutes) = source.boundary_start_minutes else {
        return Err("selected Idle session lacks boundary provenance".to_string());
    };
    let policy = OperationalDayPolicy {
        utc_offset_seconds: i32::try_from(source_offset)
            .map_err(|_| "selected Idle session has invalid boundary offset".to_string())?,
        start_minutes: u16::try_from(source_start_minutes)
            .map_err(|_| "selected Idle session has invalid boundary start".to_string())?,
    };

    let before_seconds = usize::try_from((request.started_at_utc - source_started_at_utc).num_seconds())
        .map_err(|_| "missed activity start precedes the source session".to_string())?;
    let activity_seconds = usize::try_from((request.ended_at_utc - request.started_at_utc).num_seconds())
        .map_err(|_| "missed activity duration is invalid".to_string())?;
    let consumed = before_seconds
        .checked_add(activity_seconds)
        .ok_or_else(|| "missed activity duration overflowed".to_string())?;
    if activity_seconds == 0 || consumed > source_elapsed {
        return Err("missed activity interval is not representable in canonical whole seconds".to_string());
    }
    let after_seconds = source_elapsed - consumed;

    for other in sessions.iter().filter(|session| session.id != source_id) {
        let other_start = parse_utc(&other.started_at_utc)?;
        let other_elapsed = i64::try_from(other.elapsed_seconds)
            .map_err(|_| format!("session {} duration is invalid", other.id))?;
        let other_end = other_start
            .checked_add_signed(ChronoDuration::seconds(other_elapsed))
            .ok_or_else(|| format!("session {} end exceeds chrono range", other.id))?;
        if other_start < request.ended_at_utc && other_end > request.started_at_utc {
            return Err(format!(
                "missed activity overlaps completed non-source session {} (layer {})",
                other.id, other.category_id
            ));
        }
    }
    validate_historical_active_preview(&transaction, request.active_preview.as_ref())?;
    if let Some(active_preview) = request.active_preview.as_ref()
        && active_preview.started_at_utc < request.ended_at_utc
    {
        return Err("missed activity overlaps the current active session".to_string());
    }

    let affected_days = temporal::allocate_operational_day_slices(
        source_started_at_utc,
        source_recorded_end_utc,
        source_elapsed,
        policy,
    )?
    .into_iter()
    .map(|slice| slice.operational_day)
    .collect::<BTreeSet<_>>();

    let source_row_category;
    let source_row_description;
    let source_row_end;
    let source_row_elapsed;
    if before_seconds > 0 {
        source_row_category = crate::domain::DRIFT_CATEGORY_ID;
        source_row_description = source.description.as_str();
        source_row_end = request.started_at_utc;
        source_row_elapsed = before_seconds;
    } else {
        source_row_category = request.target_category_id;
        source_row_description = request.description.as_str();
        source_row_end = if after_seconds == 0 {
            source_recorded_end_utc
        } else {
            request.ended_at_utc
        };
        source_row_elapsed = activity_seconds;
    }
    write_history_fragment(
        &transaction,
        Some(source_id),
        &source.stable_id,
        source_row_category,
        source_row_description,
        source_started_at_utc,
        source_row_end,
        source_row_elapsed,
        policy,
    )?;

    if before_seconds > 0 {
        let stable_id = correction_stable_id(
            &source.stable_id,
            "activity",
            request.started_at_utc,
            request.target_category_id,
        );
        write_history_fragment(
            &transaction,
            None,
            &stable_id,
            request.target_category_id,
            &request.description,
            request.started_at_utc,
            if after_seconds == 0 {
                source_recorded_end_utc
            } else {
                request.ended_at_utc
            },
            activity_seconds,
            policy,
        )?;
    }

    if after_seconds > 0 {
        let stable_id = correction_stable_id(
            &source.stable_id,
            "idle-after",
            request.ended_at_utc,
            crate::domain::DRIFT_CATEGORY_ID,
        );
        write_history_fragment(
            &transaction,
            None,
            &stable_id,
            crate::domain::DRIFT_CATEGORY_ID,
            &source.description,
            request.ended_at_utc,
            source_recorded_end_utc,
            after_seconds,
            policy,
        )?;
    }

    runtime_coordination::maybe_inject_test_fault("history-correction", "sessions")
        .map_err(|error| error.to_string())?;
    replace_daily_contributions_in_transaction(
        &transaction,
        &affected_days,
        request.active_preview.as_ref(),
    )?;
    runtime_coordination::maybe_inject_test_fault("history-correction", "daily")
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("history-correction", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;

    Ok(affected_days)
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
                payload_json, updated_at_utc
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(singleton) DO UPDATE SET
                formation_id = excluded.formation_id,
                quantum_seconds = excluded.quantum_seconds,
                grid_width = excluded.grid_width,
                grid_height = excluded.grid_height,
                payload_json = excluded.payload_json,
                updated_at_utc = excluded.updated_at_utc",
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

fn decode_day_end_snapshot(
    operational_day: &str,
    payload_json: &str,
) -> Result<SedimentSnapshot, String> {
    if let Ok(snapshot) = serde_json::from_str::<SedimentSnapshot>(payload_json) {
        if snapshot.is_authentic_day_end_for(operational_day) {
            return Ok(snapshot);
        }
        return Err(format!(
            "day-end snapshot for {operational_day} has incompatible typed payload"
        ));
    }
    let state = serde_json::from_str::<SandState>(payload_json).map_err(|error| {
        format!("invalid day-end snapshot payload for {operational_day}: {error}")
    })?;
    Ok(SedimentSnapshot::legacy_daily_payload(
        operational_day.to_string(),
        state,
    ))
}

pub(crate) fn save_day_end_snapshot(
    database_path: &Path,
    operational_day: &str,
    snapshot: &SedimentSnapshot,
    captured_at_utc: DateTime<Utc>,
) -> Result<(), String> {
    if !snapshot.is_authentic_day_end_for(operational_day) {
        return Err(format!(
            "refusing non-authentic day-end snapshot for {operational_day}"
        ));
    }
    runtime_coordination::maybe_inject_test_fault("day-end-snapshot", "before-write")
        .map_err(|error| error.to_string())?;
    let mut repository = open_cli_repository(database_path)?;
    let existing_sand = repository.sand_state().map_err(|error| error.to_string())?;
    let formation_id = existing_sand
        .as_ref()
        .map(|record| record.formation_id.as_str())
        .unwrap_or("default");
    let quantum_seconds = existing_sand
        .as_ref()
        .map(|record| record.quantum_seconds)
        .unwrap_or(1);
    let payload_json = serde_json::to_string(snapshot).map_err(|error| error.to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let existing_payload: Option<String> = transaction
        .query_row(
            "SELECT payload_json FROM sand_snapshots
             WHERE snapshot_kind = 'daily' AND operational_day = ?1
             ORDER BY id ASC LIMIT 1",
            params![operational_day],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some(existing_payload) = existing_payload {
        decode_day_end_snapshot(operational_day, &existing_payload)?;
        return Ok(());
    }
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
                timestamp(captured_at_utc),
            ],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("day-end-snapshot", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn load_day_end_snapshot(
    database_path: &Path,
    operational_day: &str,
) -> Result<Option<SedimentSnapshot>, String> {
    let repository = open_cli_repository(database_path)?;
    let payload: Option<String> = repository
        .connection
        .query_row(
            "SELECT payload_json FROM sand_snapshots
             WHERE snapshot_kind = 'daily' AND operational_day = ?1
             ORDER BY id ASC LIMIT 1",
            params![operational_day],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    payload
        .map(|value| decode_day_end_snapshot(operational_day, &value))
        .transpose()
}

pub(crate) fn save_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
    snapshot: &SedimentSnapshot,
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
    let payload_json = serde_json::to_string(snapshot).map_err(|error| error.to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO sand_snapshots (
                formation_id, snapshot_kind, operational_day, quantum_seconds,
                payload_json, captured_at_utc
             ) VALUES (?1, 'daily-contribution', ?2, ?3, ?4, ?5)",
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

pub(crate) struct ClearAllStateRequest<'a, T> {
    pub expected_active_stable_id: &'a str,
    pub resulting_active_stable_id: &'a str,
    pub resulting_started_at_utc: DateTime<Utc>,
    pub state: &'a SandState,
    pub daily_updates: &'a [(String, Option<SedimentSnapshot>)],
    pub detached_at_utc: DateTime<Utc>,
    pub simulation_time_utc: DateTime<Utc>,
    pub checkpoint: &'a T,
}

pub(crate) fn clear_all_state<T: Serialize>(
    database_path: &Path,
    request: ClearAllStateRequest<'_, T>,
) -> Result<(), String> {
    let ClearAllStateRequest {
        expected_active_stable_id,
        resulting_active_stable_id,
        resulting_started_at_utc,
        state,
        daily_updates,
        detached_at_utc,
        simulation_time_utc,
        checkpoint,
    } = request;
    runtime_coordination::maybe_inject_test_fault("clear-all", "before-write")
        .map_err(|error| error.to_string())?;
    if expected_active_stable_id.trim().is_empty() || resulting_active_stable_id.trim().is_empty() {
        return Err("clear-all requires non-empty active stable identities".to_string());
    }
    if !state.grains.is_empty()
        || !state.pending_grains.is_empty()
        || !state.pending_runs.is_empty()
    {
        return Err("clear-all refuses a non-empty sediment payload".to_string());
    }
    let mut repository = open_cli_repository(database_path)?;
    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;
    let checkpoint_json = serde_json::to_string(checkpoint).map_err(|error| error.to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;

    let active: Option<(String, i64, String)> = transaction
        .query_row(
            "SELECT stable_id, category_id, description
             FROM active_session WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((actual_stable_id, _, _)) = active else {
        return Err("there is no active TUI session to clear".to_string());
    };
    if actual_stable_id != expected_active_stable_id {
        return Err(format!(
            "active session changed concurrently; expected {expected_active_stable_id}, found {actual_stable_id}"
        ));
    }

    let checkpoint_state: Option<(String, Option<String>)> = transaction
        .query_row(
            "SELECT status, active_session_stable_id
             FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some((status, checkpoint_active)) = checkpoint_state {
        let replaceable = matches!(status.as_str(), "pending" | "committed")
            && checkpoint_active.as_deref() == Some(expected_active_stable_id);
        if !replaceable {
            let identity = checkpoint_active.as_deref().unwrap_or("missing");
            return Err(format!(
                "runtime checkpoint is {status} for {identity}; expected pending/committed for {expected_active_stable_id}"
            ));
        }
    }

    if resulting_active_stable_id != expected_active_stable_id {
        let changed = transaction
            .execute(
                "UPDATE active_session
                 SET stable_id = ?1, started_at_utc = ?2, recovery_kind = 'live'
                 WHERE singleton = 1 AND stable_id = ?3",
                params![
                    resulting_active_stable_id,
                    timestamp(resulting_started_at_utc),
                    expected_active_stable_id,
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("active session changed during clear-all".to_string());
        }
    }
    runtime_coordination::maybe_inject_test_fault("clear-all", "active")
        .map_err(|error| error.to_string())?;

    let existing_sand: Option<(String, i64)> = transaction
        .query_row(
            "SELECT formation_id, quantum_seconds FROM sand_state WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (formation_id, quantum_seconds) =
        existing_sand.unwrap_or_else(|| ("default".to_string(), 1));
    let captured_at_utc = timestamp(Utc::now());
    transaction
        .execute(
            "INSERT INTO sand_state (
                singleton, formation_id, quantum_seconds, grid_width, grid_height,
                payload_json, updated_at_utc
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(singleton) DO UPDATE SET
                formation_id = excluded.formation_id,
                quantum_seconds = excluded.quantum_seconds,
                grid_width = excluded.grid_width,
                grid_height = excluded.grid_height,
                payload_json = excluded.payload_json,
                updated_at_utc = excluded.updated_at_utc",
            params![
                formation_id,
                quantum_seconds,
                i64::try_from(state.grid_width)
                    .map_err(|_| "sand width is too large".to_string())?,
                i64::try_from(state.grid_height)
                    .map_err(|_| "sand height is too large".to_string())?,
                payload_json,
                captured_at_utc,
            ],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("clear-all", "sand")
        .map_err(|error| error.to_string())?;

    for (operational_day, snapshot) in daily_updates {
        transaction
            .execute(
                "DELETE FROM sand_snapshots
                 WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1",
                params![operational_day],
            )
            .map_err(|error| error.to_string())?;
        if let Some(snapshot) = snapshot {
            let daily_json = serde_json::to_string(snapshot).map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO sand_snapshots (
                        formation_id, snapshot_kind, operational_day, quantum_seconds,
                        payload_json, captured_at_utc
                     ) VALUES (?1, 'daily-contribution', ?2, ?3, ?4, ?5)",
                    params![
                        formation_id,
                        operational_day,
                        quantum_seconds,
                        daily_json,
                        captured_at_utc,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    runtime_coordination::maybe_inject_test_fault("clear-all", "daily")
        .map_err(|error| error.to_string())?;

    transaction
        .execute(
            "INSERT INTO runtime_checkpoint (
                singleton, status, detached_at_utc, simulation_time_utc,
                active_session_stable_id, payload_json
             ) VALUES (1, 'pending', ?1, ?2, ?3, ?4)
             ON CONFLICT(singleton) DO UPDATE SET
                status = 'pending',
                detached_at_utc = excluded.detached_at_utc,
                simulation_time_utc = excluded.simulation_time_utc,
                active_session_stable_id = excluded.active_session_stable_id,
                payload_json = excluded.payload_json",
            params![
                timestamp(detached_at_utc),
                timestamp(simulation_time_utc),
                resulting_active_stable_id,
                checkpoint_json,
            ],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("clear-all", "checkpoint")
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("clear-all", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn load_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
) -> Result<Option<SedimentSnapshot>, String> {
    let repository = open_cli_repository(database_path)?;
    let payload: Option<String> = repository
        .connection
        .query_row(
            "SELECT payload_json FROM sand_snapshots
             WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1
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
            "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1",
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
    pub was_committed: bool,
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
    let claimed_stable_id = match claimed.active_session_stable_id.as_deref() {
        Some(stable_id) => stable_id,
        None => {
            runtime_coordination::quarantine_checkpoint(&mut repository)
                .map_err(|error| error.to_string())?;
            return Err(
                "Runtime checkpoint has no active stable identity; evidence quarantined"
                    .to_string(),
            );
        }
    };
    let authoritative_active = repository
        .active_session()
        .map_err(|error| error.to_string())?;
    if authoritative_active
        .as_ref()
        .map(|active| active.stable_id.as_str())
        != Some(claimed_stable_id)
    {
        runtime_coordination::quarantine_checkpoint(&mut repository)
            .map_err(|error| error.to_string())?;
        let actual = authoritative_active
            .as_ref()
            .map(|active| active.stable_id.as_str())
            .unwrap_or("no active session");
        return Err(format!(
            "Runtime checkpoint active identity {claimed_stable_id} does not match authoritative active session {actual}; evidence quarantined"
        ));
    }
    match serde_json::from_str(&claimed.payload_json) {
        Ok(payload) => Ok(Some(SqliteClaimedCheckpoint {
            active_session_stable_id: claimed.active_session_stable_id,
            payload,
            was_committed: claimed.was_committed,
        })),
        Err(error) => {
            runtime_coordination::quarantine_checkpoint(&mut repository)
                .map_err(|quarantine_error| quarantine_error.to_string())?;
            Err(format!("Invalid runtime checkpoint payload: {error}"))
        }
    }
}

pub(crate) fn replace_recovering_checkpoint<T: Serialize>(
    database_path: &Path,
    expected_active_stable_id: &str,
    payload: &T,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let payload_json = serde_json::to_string(payload).map_err(|error| error.to_string())?;
    runtime_coordination::replace_recovering_checkpoint_payload(
        &mut repository,
        expected_active_stable_id,
        &payload_json,
    )
    .map_err(|error| error.to_string())
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
    daily_contribution: &SedimentSnapshot,
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
    let daily_payload_json =
        serde_json::to_string(daily_contribution).map_err(|error| error.to_string())?;
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
        &daily_payload_json,
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
    let balance_effect = i8::try_from(balance_effect)
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
        balance_effect,
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

    use chrono::TimeZone;

    use super::*;
    use crate::sqlite::{
        SqliteRepository,
        repository::{NewCategoryRecord, NewSessionRecord},
    };

    fn repository_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-sqlite007-{name}-{}-{}.sqlite3",
            process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    #[derive(serde::Serialize)]
    struct BootstrapCheckpointFixture {
        schema_version: u8,
    }

    fn prepare_bootstrap_repository(path: &Path) {
        let mut repository = SqliteRepository::open(path).unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
    }

    #[test]
    fn initial_bootstrap_binds_active_and_checkpoint_row_identity() {
        let path = repository_file("initial-bootstrap-identity");
        prepare_bootstrap_repository(&path);
        let started_at_utc = DateTime::parse_from_rfc3339("2026-08-03T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let stable_id = initial_active_stable_id(started_at_utc);
        let checkpoint = BootstrapCheckpointFixture { schema_version: 3 };

        start_active_session_with_checkpoint(
            &path,
            InitialActiveGenerationRequest {
                active_stable_id: &stable_id,
                category_id: CategoryId::new(1),
                description: "Focused",
                started_at_utc,
                detached_at_utc: started_at_utc,
                simulation_time_utc: started_at_utc,
                checkpoint: &checkpoint,
            },
        )
        .unwrap();

        let repository = open_cli_repository(&path).unwrap();
        let row: (String, String, String) = repository
            .connection
            .query_row(
                "SELECT active_session.stable_id,
                        runtime_checkpoint.active_session_stable_id,
                        runtime_checkpoint.payload_json
                 FROM active_session CROSS JOIN runtime_checkpoint
                 WHERE active_session.singleton = 1 AND runtime_checkpoint.singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&row.2).unwrap();
        assert_eq!(row.0, stable_id);
        assert_eq!(row.1, stable_id);
        assert_eq!(
            payload
                .get("schema_version")
                .and_then(serde_json::Value::as_u64),
            Some(3)
        );
        drop(repository);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn initial_bootstrap_refuses_preexisting_checkpoint_without_overwrite() {
        let path = repository_file("initial-bootstrap-existing-checkpoint");
        prepare_bootstrap_repository(&path);
        let repository = open_cli_repository(&path).unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO runtime_checkpoint (
                    singleton, status, detached_at_utc, simulation_time_utc,
                    active_session_stable_id, payload_json
                 ) VALUES (1, 'quarantined', '2026-08-03T17:00:00Z',
                    '2026-08-03T17:00:00Z', NULL, '{}')",
                [],
            )
            .unwrap();
        drop(repository);

        let started_at_utc = DateTime::parse_from_rfc3339("2026-08-03T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let stable_id = initial_active_stable_id(started_at_utc);
        let checkpoint = BootstrapCheckpointFixture { schema_version: 3 };
        let error = start_active_session_with_checkpoint(
            &path,
            InitialActiveGenerationRequest {
                active_stable_id: &stable_id,
                category_id: CategoryId::new(1),
                description: "Focused",
                started_at_utc,
                detached_at_utc: started_at_utc,
                simulation_time_utc: started_at_utc,
                checkpoint: &checkpoint,
            },
        )
        .unwrap_err();
        assert!(error.contains("no checkpoint before initial active generation"));

        let repository = open_cli_repository(&path).unwrap();
        let active_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
            .unwrap();
        let checkpoint: (String, String) = repository
            .connection
            .query_row(
                "SELECT status, payload_json FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(active_count, 0);
        assert_eq!(checkpoint.0, "quarantined");
        assert_eq!(checkpoint.1, "{}");
        drop(repository);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn category_sync_preserves_active_session_description() {
        let path = repository_file("category-sync-active-description");
        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "Layer metadata",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
        drop(repository);

        let started_at = Utc.with_ymd_and_hms(2026, 8, 25, 18, 0, 0).unwrap();
        let stable_id =
            ensure_active_session(&path, CategoryId::new(1), "Session subtitle", started_at)
                .unwrap();
        let mut state = load_state(&path).unwrap();
        state.loaded_categories.categories[1].description = "Changed metadata".to_string();

        sync_categories(
            &path,
            &state.loaded_categories.categories,
            CategoryId::new(1),
            Some(&stable_id),
        )
        .unwrap();

        let reloaded = load_state(&path).unwrap();
        assert_eq!(
            reloaded.loaded_categories.categories[1].description,
            "Changed metadata"
        );
        assert_eq!(
            reloaded.active_session.unwrap().description,
            "Session subtitle"
        );
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn category_order_and_archival_round_trip() {
        let path = repository_file("categories");
        let mut repository = SqliteRepository::open(&path).unwrap();
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
    fn session_sync_preserves_chronology_and_concurrent_rows() {
        let path = repository_file("sessions");
        let mut repository = SqliteRepository::open(&path).unwrap();
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
                    id, stable_id, category_id, description, started_at_utc,
                    ended_at_utc, operational_day, elapsed_seconds, source
                 ) VALUES (7, 'stable', 1, 'old',
                    '2026-08-01T12:00:00Z', '2026-08-01T13:00:00Z',
                    '2026-08-01', 3600, 'cli-runtime')",
                [],
            )
            .unwrap();
        drop(repository);

        let mut state = load_state(&path).unwrap();
        state.loaded_sessions.sessions[0].description = "edited".to_string();
        sync_sessions(&path, &state.loaded_sessions.sessions).unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let row: (String, String) = repository
            .connection
            .query_row(
                "SELECT description, started_at_utc FROM sessions WHERE id = 7",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(row.0, "edited");
        assert_eq!(row.1, "2026-08-01T12:00:00Z");

        repository
            .connection
            .execute(
                "INSERT INTO sessions (
                    id, stable_id, category_id, description, started_at_utc,
                    ended_at_utc, operational_day, elapsed_seconds, source
                 ) VALUES (8, 'concurrent', 1, 'external',
                    '2026-08-01T14:00:00Z', '2026-08-01T15:00:00Z',
                    '2026-08-01', 3600, 'cli-runtime')",
                [],
            )
            .unwrap();
        drop(repository);
        sync_sessions(&path, &state.loaded_sessions.sessions).unwrap();
        update_session_description(&path, 7, "explicit-edit").unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let preserved: (String, String, String) = repository
            .connection
            .query_row(
                "SELECT description, started_at_utc, source FROM sessions WHERE id = 7",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(preserved.0, "explicit-edit");
        assert_eq!(preserved.1, "2026-08-01T12:00:00Z");
        assert_eq!(preserved.2, "cli-runtime");
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
            .start_session(&NewActiveSession {
                stable_id: "checkpoint-active",
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
            pending_runs: Vec::new(),
        };
        save_sand_state(&path, &state).unwrap();
        let daily = SedimentSnapshot::daily_contribution(
            "2026-08-01".to_string(),
            "revision-a".to_string(),
            state.clone(),
        );
        save_daily_snapshot(&path, "2026-08-01", &daily).unwrap();
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
            Some(daily.clone())
        );
        let repository = open_cli_repository(&path).unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO sand_snapshots (
                    formation_id, snapshot_kind, operational_day, quantum_seconds,
                    payload_json, captured_at_utc
                 ) VALUES ('default', 'daily', '2026-08-01', 1, '{}', '2026-08-01T12:00:00Z')",
                [],
            )
            .unwrap();
        drop(repository);
        save_daily_snapshot(&path, "2026-08-01", &daily).unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let daily_count: i64 = repository
            .connection
            .query_row(
                "SELECT count(*) FROM sand_snapshots
                 WHERE snapshot_kind = 'daily' AND operational_day = '2026-08-01'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            daily_count, 1,
            "existing daily snapshot must remain untouched"
        );
        drop(repository);

        let checkpoint: Option<SqliteClaimedCheckpoint<BTreeMap<String, String>>> =
            load_checkpoint(&path).unwrap();
        assert_eq!(
            checkpoint.unwrap().payload.get("status").unwrap(),
            "detached"
        );
        commit_checkpoint_recovery(&path, "checkpoint-active", "2026-08-01", &state, &daily)
            .unwrap();
        clear_checkpoint(&path).unwrap();
        assert!(
            load_checkpoint::<BTreeMap<String, String>>(&path)
                .unwrap()
                .is_none()
        );
        std::fs::remove_file(path).ok();
    }
}

#[cfg(test)]
mod checkpoint_identity_tests {
    use std::path::{Path, PathBuf};

    use serde_json::Value;

    use super::*;
    use crate::sqlite::{
        NewActiveSession, SqliteRepository, repository::NewCategoryRecord, runtime_coordination,
    };

    fn database_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-checkpoint-identity-{}-{}.sqlite3",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn remove_database(path: &Path) {
        std::fs::remove_file(path).ok();
        std::fs::remove_file(format!("{}-wal", path.display())).ok();
        std::fs::remove_file(format!("{}-shm", path.display())).ok();
    }

    #[test]
    fn startup_quarantines_checkpoint_without_active_identity() {
        let path = database_path();
        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
        runtime_coordination::start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id: "active-a",
                category_id: 1,
                description: "",
                started_at_utc: "2026-08-01T10:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        runtime_coordination::save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T10:01:00Z",
            "2026-08-01T10:00:59Z",
            "{}",
        )
        .unwrap();
        repository
            .connection
            .execute(
                "UPDATE runtime_checkpoint SET active_session_stable_id = NULL WHERE singleton = 1",
                [],
            )
            .unwrap();
        drop(repository);

        let error = load_checkpoint::<Value>(&path).unwrap_err();
        assert!(error.contains("has no active stable identity; evidence quarantined"));
        let repository = SqliteRepository::open(&path).unwrap();
        let status: String = repository
            .connection
            .query_row(
                "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "quarantined");
        drop(repository);
        remove_database(&path);
    }
}

#[cfg(test)]
mod clear_all_transaction_tests {
    use std::path::{Path, PathBuf};

    use serde_json::Value;

    use super::*;
    use crate::sqlite::{NewActiveSession, SqliteRepository, runtime_coordination};

    fn database_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-clear-all-{label}-{}-{}.sqlite3",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn remove_database(path: &Path) {
        std::fs::remove_file(path).ok();
        std::fs::remove_file(format!("{}-wal", path.display())).ok();
        std::fs::remove_file(format!("{}-shm", path.display())).ok();
    }

    fn state(grains: bool) -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: if grains {
                vec![crate::sand::SandStateGrain {
                    x: 0,
                    y: 1,
                    category_id: 0,
                }]
            } else {
                Vec::new()
            },
            frame_count: 3,
            sweep_left_to_right: true,
            rng_state: 9,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        }
    }

    fn seed(path: &Path) {
        let mut repository = SqliteRepository::open(path).unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO sessions (
                    stable_id, category_id, description, started_at_utc,
                    ended_at_utc, operational_day, elapsed_seconds, source
                 ) VALUES ('completed-idle', 0, '', '2026-08-01T10:00:00Z',
                    '2026-08-01T11:00:00Z', '2026-08-01', 3600, 'test')",
                [],
            )
            .unwrap();
        runtime_coordination::start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id: "idle-a",
                category_id: 0,
                description: "",
                started_at_utc: "2026-08-01T11:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        drop(repository);
        save_sand_state(path, &state(true)).unwrap();
        let daily = SedimentSnapshot::daily_contribution(
            "2026-08-01".to_string(),
            "before".to_string(),
            state(true),
        );
        save_daily_snapshot(path, "2026-08-01", &daily).unwrap();
        save_checkpoint(
            path,
            "idle-a",
            Utc::now(),
            Utc::now(),
            &serde_json::json!({"before": true}),
        )
        .unwrap();
    }

    #[test]
    fn authentic_day_end_snapshot_is_first_write_wins_and_round_trips_exact_topology() {
        let path = database_path("day-end-snapshot");
        let state = SandState {
            version: SandState::VERSION,
            grid_width: 7,
            grid_height: 5,
            grains: vec![crate::sand::SandStateGrain {
                x: 3,
                y: 4,
                category_id: 0,
            }],
            frame_count: 19,
            sweep_left_to_right: false,
            rng_state: 77,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        };
        let snapshot =
            SedimentSnapshot::day_end_checkpoint("2026-08-01".to_string(), state.clone());
        let captured = DateTime::parse_from_rfc3339("2026-08-02T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        save_day_end_snapshot(&path, "2026-08-01", &snapshot, captured).unwrap();
        assert_eq!(
            load_day_end_snapshot(&path, "2026-08-01").unwrap(),
            Some(snapshot.clone())
        );
        save_day_end_snapshot(&path, "2026-08-01", &snapshot, captured).unwrap();

        let mut conflicting_state = state;
        conflicting_state.grains[0].x = 4;
        let conflicting =
            SedimentSnapshot::day_end_checkpoint("2026-08-01".to_string(), conflicting_state);
        save_day_end_snapshot(&path, "2026-08-01", &conflicting, captured).unwrap();
        assert_eq!(
            load_day_end_snapshot(&path, "2026-08-01").unwrap(),
            Some(snapshot)
        );
        remove_database(&path);
    }

    #[test]
    fn bare_daily_state_loads_as_legacy_authentic_visual_checkpoint() {
        let path = database_path("legacy-day-end-snapshot");
        let state = SandState {
            version: SandState::VERSION,
            grid_width: 3,
            grid_height: 2,
            grains: Vec::new(),
            frame_count: 4,
            sweep_left_to_right: true,
            rng_state: 5,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        };
        let payload = serde_json::to_string(&state).unwrap();
        let repository = open_cli_repository(&path).unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO sand_snapshots (
                    formation_id, snapshot_kind, operational_day, quantum_seconds,
                    payload_json, captured_at_utc
                 ) VALUES ('default', 'daily', '2026-08-01', 1, ?1, '2026-08-02T12:00:00Z')",
                params![payload],
            )
            .unwrap();
        drop(repository);

        let loaded = load_day_end_snapshot(&path, "2026-08-01")
            .unwrap()
            .expect("legacy day-end state should load");
        assert!(loaded.is_authentic_day_end_for("2026-08-01"));
        assert_eq!(loaded.state, state);
        assert_eq!(
            loaded.provenance,
            crate::sand::SedimentSnapshotProvenance::LegacyDailyRow
        );
        remove_database(&path);
    }

    #[test]
    fn clear_all_is_atomic_and_preserves_committed_history() {
        let path = database_path("commit");
        seed(&path);
        let empty = state(false);
        let checkpoint = serde_json::json!({"clear_all": {"operation_id": "clear"}});
        let updates = [("2026-08-01".to_string(), None)];
        clear_all_state(
            &path,
            ClearAllStateRequest {
                expected_active_stable_id: "idle-a",
                resulting_active_stable_id: "idle-b",
                resulting_started_at_utc: Utc::now(),
                state: &empty,
                daily_updates: &updates,
                detached_at_utc: Utc::now(),
                simulation_time_utc: Utc::now(),
                checkpoint: &checkpoint,
            },
        )
        .unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let session_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(session_count, 1);
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "idle-b"
        );
        let checkpoint = repository.checkpoint().unwrap().unwrap();
        assert_eq!(
            checkpoint.active_session_stable_id.as_deref(),
            Some("idle-b")
        );
        let payload: Value = serde_json::from_str(&checkpoint.payload_json).unwrap();
        assert_eq!(payload["clear_all"]["operation_id"], "clear");
        drop(repository);
        assert_eq!(load_sand_state(&path).unwrap(), Some(empty));
        assert!(load_daily_snapshot(&path, "2026-08-01").unwrap().is_none());
        remove_database(&path);
    }

    #[test]
    fn clear_all_fault_rolls_back_every_authority() {
        let path = database_path("rollback");
        seed(&path);
        let result = runtime_coordination::with_test_fault("clear-all", "commit", "io", || {
            let empty = state(false);
            let updates = [("2026-08-01".to_string(), None)];
            let checkpoint = serde_json::json!({"clear_all": true});
            clear_all_state(
                &path,
                ClearAllStateRequest {
                    expected_active_stable_id: "idle-a",
                    resulting_active_stable_id: "idle-b",
                    resulting_started_at_utc: Utc::now(),
                    state: &empty,
                    daily_updates: &updates,
                    detached_at_utc: Utc::now(),
                    simulation_time_utc: Utc::now(),
                    checkpoint: &checkpoint,
                },
            )
        });
        assert!(result.is_err());
        let repository = open_cli_repository(&path).unwrap();
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "idle-a"
        );
        assert_eq!(
            repository
                .connection
                .query_row("SELECT count(*) FROM sessions", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(repository);
        assert_eq!(load_sand_state(&path).unwrap(), Some(state(true)));
        assert!(load_daily_snapshot(&path, "2026-08-01").unwrap().is_some());
        remove_database(&path);
    }
}

#[cfg(test)]
mod clear_all_additional_transaction_tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::sqlite::{NewActiveSession, SqliteRepository, runtime_coordination};

    fn database_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-clear-all-extra-{label}-{}-{}.sqlite3",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn remove_database(path: &Path) {
        std::fs::remove_file(path).ok();
        std::fs::remove_file(format!("{}-wal", path.display())).ok();
        std::fs::remove_file(format!("{}-shm", path.display())).ok();
    }

    fn state(grains: bool) -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: if grains {
                vec![crate::sand::SandStateGrain {
                    x: 0,
                    y: 1,
                    category_id: 0,
                }]
            } else {
                Vec::new()
            },
            frame_count: 3,
            sweep_left_to_right: true,
            rng_state: 9,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        }
    }

    fn seed(path: &Path) {
        let mut repository = SqliteRepository::open(path).unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO sessions (
                    stable_id, category_id, description, started_at_utc,
                    ended_at_utc, operational_day, elapsed_seconds, source
                 ) VALUES ('completed-idle', 0, '', '2026-08-01T10:00:00Z',
                    '2026-08-01T11:00:00Z', '2026-08-01', 3600, 'test')",
                [],
            )
            .unwrap();
        runtime_coordination::start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id: "active-a",
                category_id: 0,
                description: "",
                started_at_utc: "2026-08-01T11:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        drop(repository);
        save_sand_state(path, &state(true)).unwrap();
        let daily = SedimentSnapshot::daily_contribution(
            "2026-08-01".to_string(),
            "before".to_string(),
            state(true),
        );
        save_daily_snapshot(path, "2026-08-01", &daily).unwrap();
        save_checkpoint(
            path,
            "active-a",
            Utc::now(),
            Utc::now(),
            &serde_json::json!({"before": true}),
        )
        .unwrap();
    }

    fn assert_original_state(path: &Path) {
        let repository = open_cli_repository(path).unwrap();
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "active-a"
        );
        assert_eq!(
            repository
                .connection
                .query_row("SELECT count(*) FROM sessions", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(repository);
        assert_eq!(load_sand_state(path).unwrap(), Some(state(true)));
        assert!(load_daily_snapshot(path, "2026-08-01").unwrap().is_some());
    }

    #[test]
    fn every_clear_all_kill_point_rolls_back_all_authorities() {
        for point in [
            "before-write",
            "active",
            "sand",
            "daily",
            "checkpoint",
            "commit",
        ] {
            let path = database_path(point);
            seed(&path);
            let empty = state(false);
            let updates = [("2026-08-01".to_string(), None)];
            let checkpoint = serde_json::json!({"clear_all": true});
            let result = runtime_coordination::with_test_fault("clear-all", point, "io", || {
                clear_all_state(
                    &path,
                    ClearAllStateRequest {
                        expected_active_stable_id: "active-a",
                        resulting_active_stable_id: "active-b",
                        resulting_started_at_utc: Utc::now(),
                        state: &empty,
                        daily_updates: &updates,
                        detached_at_utc: Utc::now(),
                        simulation_time_utc: Utc::now(),
                        checkpoint: &checkpoint,
                    },
                )
            });
            assert!(result.is_err(), "kill point {point} unexpectedly committed");
            assert_original_state(&path);
            remove_database(&path);
        }
    }

    #[test]
    fn non_reset_clear_preserves_active_identity_and_start() {
        let path = database_path("preserve-active");
        seed(&path);
        let repository = open_cli_repository(&path).unwrap();
        let prior_start: String = repository
            .connection
            .query_row(
                "SELECT started_at_utc FROM active_session WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(repository);
        let empty = state(false);
        let updates = [("2026-08-01".to_string(), None)];
        let checkpoint = serde_json::json!({"clear_all": true});
        clear_all_state(
            &path,
            ClearAllStateRequest {
                expected_active_stable_id: "active-a",
                resulting_active_stable_id: "active-a",
                resulting_started_at_utc: Utc::now(),
                state: &empty,
                daily_updates: &updates,
                detached_at_utc: Utc::now(),
                simulation_time_utc: Utc::now(),
                checkpoint: &checkpoint,
            },
        )
        .unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let active = repository.active_session().unwrap().unwrap();
        assert_eq!(active.stable_id, "active-a");
        let resulting_start: String = repository
            .connection
            .query_row(
                "SELECT started_at_utc FROM active_session WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(resulting_start, prior_start);
        assert_eq!(
            repository
                .connection
                .query_row("SELECT count(*) FROM sessions", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn clear_all_refuses_non_empty_resulting_sediment() {
        let path = database_path("non-empty");
        seed(&path);
        let non_empty = state(true);
        let updates = [("2026-08-01".to_string(), None)];
        let checkpoint = serde_json::json!({"clear_all": true});
        let error = clear_all_state(
            &path,
            ClearAllStateRequest {
                expected_active_stable_id: "active-a",
                resulting_active_stable_id: "active-b",
                resulting_started_at_utc: Utc::now(),
                state: &non_empty,
                daily_updates: &updates,
                detached_at_utc: Utc::now(),
                simulation_time_utc: Utc::now(),
                checkpoint: &checkpoint,
            },
        )
        .unwrap_err();
        assert!(error.contains("non-empty"));
        assert_original_state(&path);
        remove_database(&path);
    }

    fn seed_history_session(
        path: &Path,
        category_id: i64,
        stable_id: &str,
        started_at_utc: &str,
        ended_at_utc: &str,
        operational_day: &str,
        elapsed_seconds: i64,
    ) {
        let mut repository = SqliteRepository::open(path).unwrap();
        if repository
            .list_categories(true)
            .unwrap()
            .iter()
            .all(|category| category.id != 1)
        {
            repository
                .create_category(&NewCategoryRecord {
                    name: "Work",
                    description: "",
                    color_index: 1,
                    balance_effect: 1,
                })
                .unwrap();
        }
        repository
            .insert_session(&NewSessionRecord {
                stable_id,
                category_id,
                description: "",
                started_at_utc,
                ended_at_utc,
                operational_day,
                elapsed_seconds,
                boundary_utc_offset_seconds: -6 * 60 * 60,
                boundary_start_minutes: 0,
                source: "tui-runtime",
            })
            .unwrap();
    }

    #[test]
    fn missed_activity_splits_idle_transactionally_and_rewrites_each_affected_day() {
        let path = repository_file("history-missed-cross-day");
        seed_history_session(
            &path,
            0,
            "idle-a",
            "2026-08-02T05:30:00.500Z",
            "2026-08-02T06:30:00.700Z",
            "2026-08-02",
            3600,
        );

        let affected_days = log_missed_activity(
            &path,
            HistoricalMissedActivityRequest {
                source_session_id: 1,
                target_category_id: CategoryId::new(1),
                started_at_utc: DateTime::parse_from_rfc3339("2026-08-02T05:45:00.500Z")
                    .unwrap()
                    .with_timezone(&Utc),
                ended_at_utc: DateTime::parse_from_rfc3339("2026-08-02T06:15:00.500Z")
                    .unwrap()
                    .with_timezone(&Utc),
                description: String::new(),
                active_preview: None,
            },
        )
        .unwrap();

        assert_eq!(
            affected_days,
            [
                NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 8, 2).unwrap(),
            ]
            .into_iter()
            .collect()
        );

        let repository = open_cli_repository(&path).unwrap();
        let rows = repository.list_sessions().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows.iter().map(|row| row.category_id).collect::<Vec<_>>(),
            vec![0, 1, 0]
        );
        assert_eq!(
            rows.iter().map(|row| row.elapsed_seconds).collect::<Vec<_>>(),
            vec![900, 1800, 900]
        );
        assert_eq!(rows.iter().map(|row| row.elapsed_seconds).sum::<i64>(), 3600);
        assert_eq!(rows[0].stable_id, "idle-a");
        assert!(rows.iter().all(|row| row.source == "tui-history-correction"));
        assert_eq!(rows[0].ended_at_utc, rows[1].started_at_utc);
        assert_eq!(rows[1].ended_at_utc, rows[2].started_at_utc);
        assert_eq!(rows[2].ended_at_utc, "2026-08-02T06:30:00.700Z");
        drop(repository);

        let first = load_daily_snapshot(&path, "2026-08-01")
            .unwrap()
            .expect("first affected day should be rewritten");
        let second = load_daily_snapshot(&path, "2026-08-02")
            .unwrap()
            .expect("second affected day should be rewritten");
        let first_mass = first
            .state
            .pending_runs
            .iter()
            .map(|run| run.count)
            .sum::<usize>();
        let second_mass = second
            .state
            .pending_runs
            .iter()
            .map(|run| run.count)
            .sum::<usize>();
        assert_eq!(first_mass, 1799);
        assert_eq!(second_mass, 1801);
        assert!(
            first
                .state
                .pending_runs
                .iter()
                .any(|run| run.category_id == 1 && run.count == 899)
        );
        assert!(
            second
                .state
                .pending_runs
                .iter()
                .any(|run| run.category_id == 1 && run.count == 901)
        );

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn missed_activity_daily_rewrite_preserves_current_active_preview() {
        let path = repository_file("history-missed-active-preview");
        seed_history_session(
            &path,
            0,
            "idle-a",
            "2026-08-02T10:00:00Z",
            "2026-08-02T11:00:00Z",
            "2026-08-02",
            3600,
        );
        let mut repository = open_cli_repository(&path).unwrap();
        runtime_coordination::start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id: "active-now",
                category_id: 1,
                description: "",
                started_at_utc: "2026-08-02T11:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        drop(repository);

        let policy = OperationalDayPolicy {
            utc_offset_seconds: -6 * 60 * 60,
            start_minutes: 0,
        };
        let active_started = Utc.with_ymd_and_hms(2026, 8, 2, 11, 0, 0).unwrap();
        let active_preview = HistoricalActivePreview {
            stable_id: "active-now".to_string(),
            category_id: CategoryId::new(1),
            started_at_utc: active_started,
            ended_at_utc: active_started + ChronoDuration::seconds(600),
            elapsed_seconds: 600,
            operational_day_policy: policy,
        };

        log_missed_activity(
            &path,
            HistoricalMissedActivityRequest {
                source_session_id: 1,
                target_category_id: CategoryId::new(1),
                started_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 10, 15, 0).unwrap(),
                ended_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 10, 45, 0).unwrap(),
                description: String::new(),
                active_preview: Some(active_preview),
            },
        )
        .unwrap();

        let daily = load_daily_snapshot(&path, "2026-08-02")
            .unwrap()
            .expect("affected current day should retain completed plus active mass");
        let total_mass = daily
            .state
            .pending_runs
            .iter()
            .map(|run| run.count)
            .sum::<usize>();
        let work_mass = daily
            .state
            .pending_runs
            .iter()
            .filter(|run| run.category_id == 1)
            .map(|run| run.count)
            .sum::<usize>();
        assert_eq!(total_mass, 4200);
        assert_eq!(work_mass, 2400);

        let repository = open_cli_repository(&path).unwrap();
        let active = repository.active_session().unwrap().unwrap();
        assert_eq!(active.stable_id, "active-now");
        assert_eq!(active.category_id, 1);
        drop(repository);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn missed_activity_kill_points_roll_back_sessions_and_daily_contribution() {
        for point in ["before-write", "sessions", "daily", "commit"] {
            let path = repository_file(&format!("history-missed-kill-{point}"));
            seed_history_session(
                &path,
                0,
                "idle-a",
                "2026-08-02T10:00:00Z",
                "2026-08-02T11:00:00Z",
                "2026-08-02",
                3600,
            );
            let result = runtime_coordination::with_test_fault(
                "history-correction",
                point,
                "io",
                || {
                    log_missed_activity(
                        &path,
                        HistoricalMissedActivityRequest {
                            source_session_id: 1,
                            target_category_id: CategoryId::new(1),
                            started_at_utc: Utc
                                .with_ymd_and_hms(2026, 8, 2, 10, 15, 0)
                                .unwrap(),
                            ended_at_utc: Utc
                                .with_ymd_and_hms(2026, 8, 2, 10, 45, 0)
                                .unwrap(),
                            description: String::new(),
                            active_preview: None,
                        },
                    )
                },
            );
            assert!(result.is_err(), "kill point {point} unexpectedly committed");
            let repository = open_cli_repository(&path).unwrap();
            let rows = repository.list_sessions().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].stable_id, "idle-a");
            assert_eq!(rows[0].category_id, 0);
            assert_eq!(rows[0].elapsed_seconds, 3600);
            drop(repository);
            assert!(load_daily_snapshot(&path, "2026-08-02").unwrap().is_none());
            std::fs::remove_file(path).ok();
        }
    }

    #[test]
    fn missed_activity_rejects_non_idle_source_without_mutation() {
        let path = repository_file("history-missed-non-idle");
        seed_history_session(
            &path,
            1,
            "work-a",
            "2026-08-02T16:00:00Z",
            "2026-08-02T17:00:00Z",
            "2026-08-02",
            3600,
        );
        let error = log_missed_activity(
            &path,
            HistoricalMissedActivityRequest {
                source_session_id: 1,
                target_category_id: CategoryId::new(1),
                started_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 16, 15, 0).unwrap(),
                ended_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 16, 45, 0).unwrap(),
                description: String::new(),
                active_preview: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("only reclassify a completed Idle session"));
        let repository = open_cli_repository(&path).unwrap();
        let rows = repository.list_sessions().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].stable_id, "work-a");
        assert_eq!(rows[0].category_id, 1);
        assert_eq!(rows[0].source, "tui-runtime");
        drop(repository);
        std::fs::remove_file(path).ok();
    }

}
