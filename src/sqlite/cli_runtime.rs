use std::{collections::BTreeMap, path::Path, process};

use chrono::{DateTime, SecondsFormat, Utc};

use crate::{
    constants::COLORS,
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, OperationalDayPolicy, Session,
        civil_time_for_utc, day_boundary_config, is_drift_name, operational_day_key_for_utc,
    },
    runtime_identity::transition_identity,
    temporal,
};

use super::{
    NewActiveSession, SessionCompletion, runtime::open_cli_repository, runtime_coordination,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliStartResult {
    pub category_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliStopResult {
    pub elapsed_seconds: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SqliteCliSession {
    pub id: usize,
    pub stable_id: String,
    pub date: String,
    pub category_id: u64,
    pub category_name: String,
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
    description: Option<String>,
    category_name: String,
) -> Result<SqliteCliStartResult, String> {
    let mut repository = open_cli_repository(database_path)?;
    let categories = repository
        .list_categories(false)
        .map_err(|error| error.to_string())?;
    let requested = category_name.trim();
    if requested.is_empty() {
        return Err("Layer is required; use 'idle' explicitly for baseline time".to_string());
    }
    let category = if is_drift_name(requested) || requested == "0" {
        categories.iter().find(|category| category.id == 0)
    } else {
        categories.iter().find(|category| {
            category.name.eq_ignore_ascii_case(requested) || category.id.to_string() == requested
        })
    }
    .ok_or_else(|| format!("Layer '{requested}' not found"))?;

    let description = canonicalize_and_remember_tag(
        &mut repository,
        category.id,
        description.as_deref().unwrap_or_default(),
    )?;
    let now = Utc::now();
    let started_at_utc = now.to_rfc3339_opts(SecondsFormat::Millis, true);

    if let Some(active) = repository
        .active_session()
        .map_err(|error| error.to_string())?
    {
        if active.category_id == category.id {
            if !description.is_empty() {
                repository
                    .update_active_description(&description)
                    .map_err(|error| error.to_string())?;
            }
        } else {
            let interval = checked_active_interval(&active.started_at_utc, now, false)?;
            let next_stable_id = stable_id("cli", now);
            let operation_id = transition_identity(
                "switch",
                &active.stable_id,
                interval.ended_at_utc,
                &category.id.to_string(),
            )
            .operation_id;
            let ended_at_utc = interval
                .ended_at_utc
                .to_rfc3339_opts(SecondsFormat::Millis, true);
            let operational_day = operational_day_key_for_utc(interval.ended_at_utc)
                .format("%Y-%m-%d")
                .to_string();
            let policy = OperationalDayPolicy::from_config(day_boundary_config());
            runtime_coordination::switch_active_session(
                &mut repository,
                &active.stable_id,
                &operation_id,
                &SessionCompletion {
                    ended_at_utc: &ended_at_utc,
                    operational_day: &operational_day,
                    elapsed_seconds: i64::try_from(interval.elapsed_seconds).map_err(|_| {
                        "Active session duration exceeds SQLite's supported range".to_string()
                    })?,
                    boundary_utc_offset_seconds: policy.utc_offset_seconds,
                    boundary_start_minutes: policy.start_minutes,
                    source: "cli-runtime",
                },
                &NewActiveSession {
                    stable_id: &next_stable_id,
                    category_id: category.id,
                    description: &description,
                    started_at_utc: &started_at_utc,
                    recovery_kind: "live",
                },
            )
            .map_err(|error| error.to_string())?;
        }
    } else {
        let stable_id = stable_id("cli", now);
        runtime_coordination::start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id: &stable_id,
                category_id: category.id,
                description: &description,
                started_at_utc: &started_at_utc,
                recovery_kind: "live",
            },
        )
        .map_err(|error| error.to_string())?;
    }

    Ok(SqliteCliStartResult {
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
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No active layer session to stop".to_string())?;
    if active.category_id == 0 {
        return Err("No active layer session to stop (already idle)".to_string());
    }

    let idle = repository
        .list_categories(false)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|category| category.id == 0)
        .ok_or_else(|| "Reserved idle layer is missing".to_string())?;
    let now = Utc::now();
    let interval = checked_active_interval(&active.started_at_utc, now, accept_clock_jump)?;
    let elapsed_seconds = interval.elapsed_seconds;
    let next_stable_id = stable_id("cli-idle", now);
    let operation_id =
        transition_identity("switch", &active.stable_id, interval.ended_at_utc, "idle")
            .operation_id;
    let ended_at_utc = interval
        .ended_at_utc
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let operational_day = operational_day_key_for_utc(interval.ended_at_utc)
        .format("%Y-%m-%d")
        .to_string();
    let policy = OperationalDayPolicy::from_config(day_boundary_config());
    runtime_coordination::switch_active_session(
        &mut repository,
        &active.stable_id,
        &operation_id,
        &SessionCompletion {
            ended_at_utc: &ended_at_utc,
            operational_day: &operational_day,
            elapsed_seconds: i64::try_from(elapsed_seconds).map_err(|_| {
                "Active session duration exceeds SQLite's supported range".to_string()
            })?,
            boundary_utc_offset_seconds: policy.utc_offset_seconds,
            boundary_start_minutes: policy.start_minutes,
            source: "cli-runtime",
        },
        &NewActiveSession {
            stable_id: &next_stable_id,
            category_id: idle.id,
            description: "",
            started_at_utc: &ended_at_utc,
            recovery_kind: "live",
        },
    )
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

fn canonicalize_and_remember_tag(
    repository: &mut super::SqliteRepository,
    category_id: i64,
    tag: &str,
) -> Result<String, String> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let mut tags = repository
        .category_tags()
        .map_err(|error| error.to_string())?
        .remove(&category_id)
        .unwrap_or_default();
    let canonical = tags
        .iter()
        .find(|existing| existing.eq_ignore_ascii_case(trimmed))
        .cloned()
        .unwrap_or_else(|| trimmed.to_string());
    tags.retain(|existing| !existing.eq_ignore_ascii_case(&canonical));
    tags.insert(0, canonical.clone());
    tags.truncate(crate::constants::CATEGORY_SETTINGS.max_tags_per_category);
    repository
        .replace_category_tags(category_id, &tags)
        .map_err(|error| error.to_string())?;
    Ok(canonical)
}

fn checked_active_interval(
    started_at_utc: &str,
    now: DateTime<Utc>,
    accept_clock_jump: bool,
) -> Result<temporal::ReconciledInterval, String> {
    let started_at = DateTime::parse_from_rfc3339(started_at_utc)
        .map_err(|error| format!("Active session has an invalid start timestamp: {error}"))?
        .with_timezone(&Utc);
    temporal::checked_wall_interval(started_at, now, accept_clock_jump)
}

fn stable_id(prefix: &str, at: DateTime<Utc>) -> String {
    format!(
        "{prefix}-{}-{}",
        at.to_rfc3339_opts(SecondsFormat::Nanos, true),
        process::id()
    )
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
