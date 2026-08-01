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
    domain::{Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, Session, runtime_settings},
    sand::SandState,
    storage::{CategoryTagsState, LoadedCategories, LoadedSessions},
};

use super::{
    NewActiveSession, SessionCompletion,
    authority::open_cli_repository,
    repository::{CheckpointRecord, CheckpointStatus, SandStateRecord},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteTuiActiveSession {
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
        sessions.push(Session {
            id,
            date: row.operational_day,
            category_id: CategoryId::new(category_id),
            description: row.description,
            start_time: local_clock(&row.started_at_utc)?,
            end_time: local_clock(&row.ended_at_utc)?,
            elapsed_seconds,
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
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    if repository
        .active_session()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        return Ok(());
    }
    let stable_id = stable_id("tui", started_at_utc);
    let started = timestamp(started_at_utc);
    repository
        .start_session(&NewActiveSession {
            stable_id: &stable_id,
            project: "",
            category_id: as_i64(category_id.0, "category ID")?,
            description,
            started_at_utc: &started,
            recovery_kind: "live",
        })
        .map_err(|error| error.to_string())
}

pub(crate) fn switch_active_session(
    database_path: &Path,
    next_category_id: CategoryId,
    next_description: &str,
    switched_at_utc: DateTime<Utc>,
    operational_day: &str,
    elapsed_seconds: usize,
) -> Result<i64, String> {
    let mut repository = open_cli_repository(database_path)?;
    let switched = timestamp(switched_at_utc);
    let stable_id = stable_id("tui", switched_at_utc);
    repository
        .switch_active_session(
            &SessionCompletion {
                ended_at_utc: &switched,
                operational_day,
                elapsed_seconds: as_i64(elapsed_seconds as u64, "elapsed seconds")?,
                source: "tui-runtime",
            },
            &NewActiveSession {
                stable_id: &stable_id,
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
    ended_at_utc: DateTime<Utc>,
    operational_day: &str,
    elapsed_seconds: usize,
) -> Result<i64, String> {
    let mut repository = open_cli_repository(database_path)?;
    let ended = timestamp(ended_at_utc);
    repository
        .finish_active_session(&SessionCompletion {
            ended_at_utc: &ended,
            operational_day,
            elapsed_seconds: as_i64(elapsed_seconds as u64, "elapsed seconds")?,
            source: "tui-runtime",
        })
        .map_err(|error| error.to_string())
}

pub(crate) fn reset_active_session(
    database_path: &Path,
    started_at_utc: DateTime<Utc>,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let active = transaction
        .query_row(
            "SELECT project, category_id, description FROM active_session WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "there is no active TUI session to reset".to_string())?;
    transaction
        .execute("DELETE FROM active_session WHERE singleton = 1", [])
        .map_err(|error| error.to_string())?;
    let stable_id = stable_id("tui-reset", started_at_utc);
    transaction
        .execute(
            "INSERT INTO active_session (
                singleton, stable_id, project, category_id, description, started_at_utc, recovery_kind
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, 'live')",
            params![stable_id, active.0, active.1, active.2, timestamp(started_at_utc)],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn sync_categories(
    database_path: &Path,
    categories: &[Category],
    active_category_id: CategoryId,
) -> Result<Vec<Category>, String> {
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
        return Err("the active category set is missing the reserved drift category".to_string());
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

    let active_description = categories
        .iter()
        .find(|category| category.id == active_category_id)
        .map(|category| category.description.as_str())
        .unwrap_or_default();
    transaction
        .execute(
            "UPDATE active_session SET description = ?1 WHERE singleton = 1 AND category_id = ?2",
            params![active_description, active_id],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;

    Ok(load_state(database_path)?.archived_categories)
}

pub(crate) fn archive_category(
    database_path: &Path,
    category_id: CategoryId,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let category_id = as_i64(category_id.0, "category ID")?;
    let active_category_id = repository
        .active_session()
        .map_err(|error| error.to_string())?
        .map(|active| active.category_id);
    if active_category_id == Some(category_id) {
        return Err("the active category cannot be archived".to_string());
    }
    repository
        .archive_category(category_id, &timestamp(Utc::now()))
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn sync_category_tags(
    database_path: &Path,
    tags: &CategoryTagsState,
    category_ids: &[CategoryId],
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    for category_id in category_ids {
        let category_id = as_i64(category_id.0, "category ID")?;
        let category_id_u64 = u64::try_from(category_id)
            .map_err(|_| format!("Category ID {category_id} is invalid"))?;
        let values = tags
            .tags_by_category
            .get(&category_id_u64)
            .cloned()
            .unwrap_or_default();
        repository
            .replace_category_tags(category_id, &values)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn sync_sessions(database_path: &Path, sessions: &[Session]) -> Result<(), String> {
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
    transaction.commit().map_err(|error| error.to_string())
}

pub(crate) fn update_session_description(
    database_path: &Path,
    session_id: usize,
    description: &str,
) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    let session_id =
        i64::try_from(session_id).map_err(|_| "session ID is too large".to_string())?;
    let changed = repository
        .connection
        .execute(
            "UPDATE sessions SET description = ?1 WHERE id = ?2",
            params![description, session_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    Ok(())
}

pub(crate) fn delete_session(database_path: &Path, session_id: usize) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    let session_id =
        i64::try_from(session_id).map_err(|_| "session ID is too large".to_string())?;
    let changed = repository
        .connection
        .execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("SQLite session {session_id} does not exist"));
    }
    Ok(())
}

pub(crate) fn delete_drift_sessions_for_day(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    repository
        .connection
        .execute(
            "DELETE FROM sessions WHERE category_id = 0 AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn save_sand_state(database_path: &Path, state: &SandState) -> Result<(), String> {
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
    repository
        .save_sand_state(&SandStateRecord {
            formation_id: formation_id.to_string(),
            quantum_seconds,
            grid_width: i64::try_from(state.grid_width)
                .map_err(|_| "sand width is too large".to_string())?,
            grid_height: i64::try_from(state.grid_height)
                .map_err(|_| "sand height is too large".to_string())?,
            payload_json,
            updated_at_utc: timestamp(Utc::now()),
        })
        .map_err(|error| error.to_string())
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
    let repository = open_cli_repository(database_path)?;
    repository
        .connection
        .execute(
            "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily' AND operational_day = ?1",
            params![operational_day],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn save_checkpoint<T: Serialize>(
    database_path: &Path,
    detached_at_utc: DateTime<Utc>,
    simulation_time_utc: DateTime<Utc>,
    payload: &T,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    let active_stable_id = repository
        .active_session()
        .map_err(|error| error.to_string())?
        .map(|active| active.stable_id);
    let payload_json = serde_json::to_string(payload).map_err(|error| error.to_string())?;
    repository
        .save_checkpoint(&CheckpointRecord {
            status: CheckpointStatus::Pending,
            detached_at_utc: timestamp(detached_at_utc),
            simulation_time_utc: timestamp(simulation_time_utc),
            active_session_stable_id: active_stable_id,
            payload_json,
        })
        .map_err(|error| error.to_string())
}

pub(crate) fn load_checkpoint<T: DeserializeOwned>(
    database_path: &Path,
) -> Result<Option<T>, String> {
    let repository = open_cli_repository(database_path)?;
    repository
        .checkpoint()
        .map_err(|error| error.to_string())?
        .map(|record| serde_json::from_str(&record.payload_json).map_err(|error| error.to_string()))
        .transpose()
}

pub(crate) fn clear_checkpoint(database_path: &Path) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;
    repository
        .connection
        .execute("DELETE FROM runtime_checkpoint WHERE singleton = 1", [])
        .map_err(|error| error.to_string())?;
    Ok(())
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

fn local_clock(timestamp_value: &str) -> Result<String, String> {
    let utc = DateTime::parse_from_rfc3339(timestamp_value)
        .map_err(|error| format!("Invalid SQLite timestamp '{timestamp_value}': {error}"))?
        .with_timezone(&Utc);
    let configured_offset = runtime_settings().day_boundary.utc_offset_seconds;
    let offset = FixedOffset::east_opt(configured_offset)
        .ok_or_else(|| format!("Configured UTC offset {configured_offset} is invalid"))?;
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
        drop(repository);
        let state = SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: Vec::new(),
            frame_count: 3,
            sweep_left_to_right: true,
            rng_state: 9,
        };
        save_sand_state(&path, &state).unwrap();
        save_daily_snapshot(&path, "2026-08-01", &state).unwrap();
        save_checkpoint(
            &path,
            Utc::now(),
            Utc::now(),
            &BTreeMap::from([("status", "detached")]),
        )
        .unwrap();
        assert_eq!(load_sand_state(&path).unwrap(), Some(state.clone()));
        assert_eq!(
            load_daily_snapshot(&path, "2026-08-01").unwrap(),
            Some(state)
        );
        let checkpoint: Option<BTreeMap<String, String>> = load_checkpoint(&path).unwrap();
        assert_eq!(checkpoint.unwrap().get("status").unwrap(), "detached");
        clear_checkpoint(&path).unwrap();
        assert!(
            load_checkpoint::<BTreeMap<String, String>>(&path)
                .unwrap()
                .is_none()
        );
        std::fs::remove_file(path).ok();
    }
}
