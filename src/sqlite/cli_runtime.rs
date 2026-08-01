use std::{collections::BTreeMap, path::Path, process};

use chrono::{DateTime, FixedOffset, Local, SecondsFormat, Utc};

use crate::{
    constants::COLORS,
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, Session, is_drift_name,
        operational_day_key_for_local, runtime_settings,
    },
};

use super::{NewActiveSession, SessionCompletion, authority::open_cli_repository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliStartResult {
    pub project: String,
    pub category_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliStopResult {
    pub elapsed_seconds: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliSession {
    pub id: usize,
    pub date: String,
    pub category_id: u64,
    pub category_name: String,
    pub project: String,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: usize,
}

impl SqliteCliSession {
    pub fn as_domain_session(&self) -> Session {
        Session {
            id: self.id,
            date: self.date.clone(),
            category_id: CategoryId::new(self.category_id),
            description: self.description.clone(),
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            elapsed_seconds: self.elapsed_seconds,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SqliteCliSnapshot {
    pub categories: Vec<Category>,
    pub sessions: Vec<SqliteCliSession>,
}

pub(crate) fn start_session(
    database_path: &Path,
    project: String,
    description: Option<String>,
    category_name: Option<String>,
) -> Result<SqliteCliStartResult, String> {
    let mut repository = open_cli_repository(database_path)?;
    if repository
        .active_session()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        return Err(
            "An active session is already running; stop it before starting another".to_string(),
        );
    }

    let categories = repository
        .list_categories(false)
        .map_err(|error| error.to_string())?;
    let requested = category_name.unwrap_or_else(|| DRIFT_CATEGORY_CONFIG_NAME.to_string());
    let category = if is_drift_name(&requested) || requested == "0" {
        categories.iter().find(|category| category.id == 0)
    } else {
        categories.iter().find(|category| {
            category.name == requested || category.id.to_string() == requested
        })
    }
    .ok_or_else(|| format!("Category '{requested}' not found"))?;

    let now = Utc::now();
    let stable_id = format!(
        "cli-{}-{}",
        now.to_rfc3339_opts(SecondsFormat::Nanos, true),
        process::id()
    );
    let started_at_utc = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let description = description.unwrap_or_default();

    repository
        .start_session(&NewActiveSession {
            stable_id: &stable_id,
            project: &project,
            category_id: category.id,
            description: &description,
            started_at_utc: &started_at_utc,
            recovery_kind: "live",
        })
        .map_err(|error| error.to_string())?;

    Ok(SqliteCliStartResult {
        project,
        category_name: display_category_name(category.id, &category.name),
    })
}

pub(crate) fn stop_session(database_path: &Path) -> Result<SqliteCliStopResult, String> {
    let mut repository = open_cli_repository(database_path)?;
    let active = repository
        .active_session()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No active session to stop".to_string())?;

    let started_at = DateTime::parse_from_rfc3339(&active.started_at_utc)
        .map_err(|error| format!("Active session has an invalid start timestamp: {error}"))?
        .with_timezone(&Utc);
    let ended_at = Utc::now();
    let elapsed_i64 = (ended_at - started_at).num_seconds().max(0);
    let elapsed_seconds = usize::try_from(elapsed_i64)
        .map_err(|_| "Active session duration exceeds this platform's limits".to_string())?;
    let operational_day = operational_day_key_for_local(&Local::now())
        .format("%Y-%m-%d")
        .to_string();
    let ended_at_utc = ended_at.to_rfc3339_opts(SecondsFormat::Millis, true);

    repository
        .finish_active_session(&SessionCompletion {
            ended_at_utc: &ended_at_utc,
            operational_day: &operational_day,
            elapsed_seconds: elapsed_i64,
            source: "cli-runtime",
        })
        .map_err(|error| error.to_string())?;

    Ok(SqliteCliStopResult { elapsed_seconds })
}

pub(crate) fn read_snapshot(database_path: &Path) -> Result<SqliteCliSnapshot, String> {
    let repository = open_cli_repository(database_path)?;
    let category_records = repository
        .list_categories(true)
        .map_err(|error| error.to_string())?;
    let session_records = repository
        .list_sessions()
        .map_err(|error| error.to_string())?;

    let mut category_names = BTreeMap::new();
    let mut categories = Vec::with_capacity(category_records.len());
    for record in category_records {
        let id = u64::try_from(record.id)
            .map_err(|_| format!("Category ID {} is outside the supported range", record.id))?;
        let color_index = usize::try_from(record.color_index)
            .map_err(|_| format!("Category color {} is invalid", record.color_index))?;
        let karma_effect = i8::try_from(record.balance_effect)
            .map_err(|_| format!("Category balance {} is invalid", record.balance_effect))?;
        let name = display_category_name(record.id, &record.name);
        category_names.insert(record.id, name.clone());
        categories.push(Category {
            id: CategoryId::new(id),
            name,
            color: COLORS[color_index % COLORS.len()],
            description: record.description,
            karma_effect,
        });
    }

    let mut sessions = Vec::with_capacity(session_records.len());
    for record in session_records {
        let id = usize::try_from(record.id)
            .map_err(|_| format!("Session ID {} is outside the supported range", record.id))?;
        let category_id = u64::try_from(record.category_id).map_err(|_| {
            format!(
                "Session {} has category ID {} outside the supported range",
                record.id, record.category_id
            )
        })?;
        let elapsed_seconds = usize::try_from(record.elapsed_seconds).map_err(|_| {
            format!(
                "Session {} duration {} is outside the supported range",
                record.id, record.elapsed_seconds
            )
        })?;
        let category_name = category_names
            .get(&record.category_id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Session {} references missing category {}",
                    record.id, record.category_id
                )
            })?;
        sessions.push(SqliteCliSession {
            id,
            date: record.operational_day,
            category_id,
            category_name,
            project: record.project,
            description: record.description,
            start_time: local_clock(&record.started_at_utc)?,
            end_time: local_clock(&record.ended_at_utc)?,
            elapsed_seconds,
        });
    }

    Ok(SqliteCliSnapshot {
        categories,
        sessions,
    })
}

fn local_clock(timestamp: &str) -> Result<String, String> {
    let utc = DateTime::parse_from_rfc3339(timestamp)
        .map_err(|error| format!("Invalid SQLite session timestamp '{timestamp}': {error}"))?
        .with_timezone(&Utc);
    let configured_offset = runtime_settings().day_boundary.utc_offset_seconds;
    let offset = FixedOffset::east_opt(configured_offset)
        .ok_or_else(|| format!("Configured UTC offset {configured_offset} is invalid"))?;
    Ok(utc.with_timezone(&offset).format("%H:%M:%S").to_string())
}

fn display_category_name(category_id: i64, stored_name: &str) -> String {
    if category_id == 0 {
        DRIFT_CATEGORY_CONFIG_NAME.to_string()
    } else {
        stored_name.to_string()
    }
}
