use std::{collections::BTreeMap, path::Path, process};

use chrono::{DateTime, SecondsFormat, Utc};

use crate::{
    constants::COLORS,
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, OperationalDayPolicy, Session,
        civil_time_for_utc, day_boundary_config, is_drift_name, operational_day_key_for_utc,
    },
    temporal,
};

use super::{
    NewActiveSession, SessionCompletion, authority::open_cli_repository, runtime_coordination,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliStartResult {
    pub project: String,
    pub category_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliStopResult {
    pub elapsed_seconds: usize,
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliSession {
    pub id: usize,
    pub stable_id: String,
    pub date: String,
    pub category_id: u64,
    pub category_name: String,
    pub project: String,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: usize,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: DateTime<Utc>,
    pub operational_day_policy: Option<OperationalDayPolicy>,
}

impl SqliteCliSession {
    pub fn as_domain_session(&self) -> Session {
        Session {
            id: self.id,
            date: self.date.clone(),
            category_id: CategoryId::new(self.category_id),
            project: self.project.clone(),
            description: self.description.clone(),
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            elapsed_seconds: self.elapsed_seconds,
            started_at_utc: Some(self.started_at_utc),
            ended_at_utc: Some(self.ended_at_utc),
            operational_day_policy: self.operational_day_policy,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SqliteCliActiveSession {
    pub stable_id: String,
    pub project: String,
    pub category_id: u64,
    pub category_name: String,
    pub description: String,
    pub started_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub(crate) struct SqliteCliSnapshot {
    pub categories: Vec<Category>,
    pub sessions: Vec<SqliteCliSession>,
    pub active_session: Option<SqliteCliActiveSession>,
}

pub(crate) fn start_session(
    database_path: &Path,
    project: String,
    description: Option<String>,
    category_name: String,
) -> Result<SqliteCliStartResult, String> {
    let mut repository = open_cli_repository(database_path)?;

    let categories = repository
        .list_categories(false)
        .map_err(|error| error.to_string())?;
    let requested = category_name.trim();
    if requested.is_empty() {
        return Err("Category is required; use --category idle for baseline time".to_string());
    }
    let category = if is_drift_name(requested) || requested == "0" {
        categories.iter().find(|category| category.id == 0)
    } else {
        categories.iter().find(|category| {
            category.name.eq_ignore_ascii_case(requested) || category.id.to_string() == requested
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

    runtime_coordination::start_active_session(
        &mut repository,
        &NewActiveSession {
            stable_id: &stable_id,
            project: &project,
            category_id: category.id,
            description: &description,
            started_at_utc: &started_at_utc,
            recovery_kind: "live",
        },
    )
    .map_err(|error| error.to_string())?;

    Ok(SqliteCliStartResult {
        project,
        category_name: display_category_name(category.id, &category.name),
    })
}

pub(crate) fn stop_session(
    database_path: &Path,
    accept_clock_jump: bool,
) -> Result<SqliteCliStopResult, String> {
    let mut repository = open_cli_repository(database_path)?;
    let active = repository
        .active_session()
        .map_err(|error| error.to_string())?;

    let receipt = if let Some(active) = active {
        let started_at = DateTime::parse_from_rfc3339(&active.started_at_utc)
            .map_err(|error| format!("Active session has an invalid start timestamp: {error}"))?
            .with_timezone(&Utc);
        let interval = temporal::checked_wall_interval(started_at, Utc::now(), accept_clock_jump)?;
        let ended_at = interval.ended_at_utc;
        let elapsed_i64 = i64::try_from(interval.elapsed_seconds)
            .map_err(|_| "Active session duration exceeds SQLite's supported range".to_string())?;
        let operational_day = operational_day_key_for_utc(ended_at)
            .format("%Y-%m-%d")
            .to_string();
        let ended_at_utc = ended_at.to_rfc3339_opts(SecondsFormat::Millis, true);
        let operation_id = format!("finish:{}", active.stable_id);
        let policy = OperationalDayPolicy::from_config(day_boundary_config());
        runtime_coordination::finish_active_session(
            &mut repository,
            &active.stable_id,
            &operation_id,
            &SessionCompletion {
                ended_at_utc: &ended_at_utc,
                operational_day: &operational_day,
                elapsed_seconds: elapsed_i64,
                boundary_utc_offset_seconds: policy.utc_offset_seconds,
                boundary_start_minutes: policy.start_minutes,
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
    runtime_coordination::acknowledge_transition(&mut repository, operation_id, &acknowledged_at)
        .map_err(|error| error.to_string())
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
        let started_at_utc = parse_utc(&record.started_at_utc)?;
        let ended_at_utc = parse_utc(&record.ended_at_utc)?;
        let operational_day_policy = match (
            record.boundary_utc_offset_seconds,
            record.boundary_start_minutes,
        ) {
            (Some(offset), Some(start_minutes)) => Some(OperationalDayPolicy {
                utc_offset_seconds: i32::try_from(offset).map_err(|_| {
                    format!("Session {} boundary UTC offset is outside i32", record.id)
                })?,
                start_minutes: u16::try_from(start_minutes).map_err(|_| {
                    format!("Session {} boundary start minute is outside u16", record.id)
                })?,
            }),
            (None, None) => None,
            _ => {
                return Err(format!(
                    "Session {} has partial boundary provenance",
                    record.id
                ));
            }
        };
        sessions.push(SqliteCliSession {
            id,
            stable_id: record.stable_id,
            date: record.operational_day,
            category_id,
            category_name,
            project: record.project,
            description: record.description,
            start_time: local_clock(&record.started_at_utc, operational_day_policy)?,
            end_time: local_clock(&record.ended_at_utc, operational_day_policy)?,
            elapsed_seconds,
            started_at_utc,
            ended_at_utc,
            operational_day_policy,
        });
    }

    let active_session = repository
        .active_session()
        .map_err(|error| error.to_string())?
        .map(|active| {
            let category_id = u64::try_from(active.category_id).map_err(|_| {
                format!(
                    "Active session has category ID {} outside the supported range",
                    active.category_id
                )
            })?;
            let category_name = category_names
                .get(&active.category_id)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "Active session references missing category {}",
                        active.category_id
                    )
                })?;
            Ok::<SqliteCliActiveSession, String>(SqliteCliActiveSession {
                stable_id: active.stable_id,
                project: active.project,
                category_id,
                category_name,
                description: active.description,
                started_at_utc: parse_utc(&active.started_at_utc)?,
            })
        })
        .transpose()?;

    Ok(SqliteCliSnapshot {
        categories,
        sessions,
        active_session,
    })
}

fn parse_utc(timestamp: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map_err(|error| format!("Invalid SQLite session timestamp '{timestamp}': {error}"))
        .map(|value| value.with_timezone(&Utc))
}

fn local_clock(timestamp: &str, policy: Option<OperationalDayPolicy>) -> Result<String, String> {
    let utc = parse_utc(timestamp)?;
    let civil = match policy {
        Some(policy) => temporal::civil_from_policy(utc, policy)?,
        None => civil_time_for_utc(utc),
    };
    Ok(civil.format("%H:%M:%S").to_string())
}

fn display_category_name(category_id: i64, stored_name: &str) -> String {
    if category_id == 0 {
        DRIFT_CATEGORY_CONFIG_NAME.to_string()
    } else {
        stored_name.to_string()
    }
}
