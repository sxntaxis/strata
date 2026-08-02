use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

use chrono::{DateTime, Local, NaiveDate, Utc};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use directories::ProjectDirs;
use ratatui::style::Color;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::{
    constants::COLORS,
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, DRIFT_CATEGORY_ID, OperationalDayPolicy,
        Session,
    },
    sand::SandState,
};

#[derive(Debug)]
pub struct LoadedCategories {
    pub categories: Vec<Category>,
    pub archived_categories: Vec<Category>,
    pub next_category_id: u64,
}

#[derive(Debug)]
pub struct LoadedSessions {
    pub sessions: Vec<Session>,
    pub next_session_id: usize,
}

const LEGACY_CATEGORIES_HEADER: [&str; 5] =
    ["id", "name", "description", "color_index", "karma_effect"];
const CATEGORIES_HEADER: [&str; 6] = [
    "id",
    "name",
    "description",
    "color_index",
    "karma_effect",
    "archived",
];
const LEGACY_SESSIONS_HEADER: [&str; 8] = [
    "id",
    "date",
    "category_id",
    "category_name",
    "description",
    "start_time",
    "end_time",
    "elapsed_seconds",
];
const TEMPORAL_SESSIONS_HEADER: [&str; 12] = [
    "id",
    "date",
    "category_id",
    "category_name",
    "description",
    "start_time",
    "end_time",
    "elapsed_seconds",
    "started_at_utc",
    "ended_at_utc",
    "boundary_utc_offset_seconds",
    "boundary_start_minutes",
];
const SESSIONS_HEADER: [&str; 13] = [
    "id",
    "date",
    "category_id",
    "category_name",
    "project",
    "description",
    "start_time",
    "end_time",
    "elapsed_seconds",
    "started_at_utc",
    "ended_at_utc",
    "boundary_utc_offset_seconds",
    "boundary_start_minutes",
];
const BACKUP_RETENTION_MAX_FILES: usize = 10;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Invalid CSV schema for {file}: expected '{expected}', found '{found}'")]
    InvalidCsvSchema {
        file: &'static str,
        expected: String,
        found: String,
    },
    #[error("Invalid CSV data for {file} row {row}: {message}")]
    InvalidCsvData {
        file: &'static str,
        row: usize,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryTagsState {
    pub version: u8,
    pub tags_by_category: HashMap<u64, Vec<String>>,
}

impl CategoryTagsState {
    pub const VERSION: u8 = 1;
}

impl Default for CategoryTagsState {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            tags_by_category: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeStorageSettings {
    pub time_log_path: Option<PathBuf>,
}

fn runtime_storage_settings_cell() -> &'static RwLock<RuntimeStorageSettings> {
    static CELL: OnceLock<RwLock<RuntimeStorageSettings>> = OnceLock::new();
    CELL.get_or_init(|| RwLock::new(RuntimeStorageSettings::default()))
}

pub fn runtime_storage_settings() -> RuntimeStorageSettings {
    runtime_storage_settings_cell()
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

pub fn set_runtime_storage_settings(settings: RuntimeStorageSettings) {
    if let Ok(mut guard) = runtime_storage_settings_cell().write() {
        *guard = settings;
    }
}

fn default_categories_loaded() -> LoadedCategories {
    LoadedCategories {
        categories: vec![Category {
            id: DRIFT_CATEGORY_ID,
            name: DRIFT_CATEGORY_CONFIG_NAME.to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 0,
        }],
        archived_categories: Vec::new(),
        next_category_id: 1,
    }
}

fn default_sessions_loaded() -> LoadedSessions {
    LoadedSessions {
        sessions: vec![],
        next_session_id: 1,
    }
}

fn csv_header_matches(headers: &StringRecord, expected: &[&str]) -> bool {
    headers.len() == expected.len()
        && headers
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| actual == *expected)
}

fn csv_header_string(headers: &StringRecord) -> String {
    headers.iter().collect::<Vec<_>>().join(",")
}

#[cfg(test)]
pub fn load_categories_from_csv(path: &Path) -> LoadedCategories {
    match try_load_categories_from_csv(path) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("Warning: Could not load categories file: {}", e);
            default_categories_loaded()
        }
    }
}

pub fn try_load_categories_from_csv(path: &Path) -> Result<LoadedCategories, StorageError> {
    if !path.exists() {
        return Ok(default_categories_loaded());
    }

    let mut reader = ReaderBuilder::new().has_headers(true).from_path(path)?;
    let headers = reader.headers()?.clone();
    let has_archived_state = csv_header_matches(&headers, &CATEGORIES_HEADER);
    if !has_archived_state && !csv_header_matches(&headers, &LEGACY_CATEGORIES_HEADER) {
        return Err(StorageError::InvalidCsvSchema {
            file: "categories.csv",
            expected: format!(
                "{} or {}",
                LEGACY_CATEGORIES_HEADER.join(","),
                CATEGORIES_HEADER.join(",")
            ),
            found: csv_header_string(&headers),
        });
    }

    let mut loaded = default_categories_loaded();
    let mut seen_ids = HashSet::from([DRIFT_CATEGORY_ID.0]);
    let mut seen_names = HashSet::from([DRIFT_CATEGORY_CONFIG_NAME.to_string()]);

    for (index, record) in reader.records().enumerate() {
        let row = index + 2;
        let record = record?;
        let id_raw = record.get(0).unwrap_or_default();
        let id = id_raw
            .parse::<u64>()
            .map_err(|error| StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("invalid category ID '{id_raw}': {error}"),
            })?;
        if id == DRIFT_CATEGORY_ID.0 {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: "idle category ID 0 is implicit and must not appear as a catalog row"
                    .to_string(),
            });
        }
        if !seen_ids.insert(id) {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("duplicate category ID {id}"),
            });
        }

        let name = record.get(1).unwrap_or_default().trim().to_string();
        if name.is_empty() || crate::domain::is_drift_name(&name) {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("invalid or reserved category name '{name}'"),
            });
        }
        let normalized_name = name.to_ascii_lowercase();
        if !seen_names.insert(normalized_name) {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("duplicate category name '{name}'"),
            });
        }

        let description = record.get(2).unwrap_or_default().to_string();
        let color_raw = record.get(3).unwrap_or_default();
        let color_idx =
            color_raw
                .parse::<usize>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "categories.csv",
                    row,
                    message: format!("invalid color index '{color_raw}': {error}"),
                })?;
        if color_idx >= COLORS.len() {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!(
                    "color index {color_idx} is outside 0..{}",
                    COLORS.len().saturating_sub(1)
                ),
            });
        }
        let karma_raw = record.get(4).unwrap_or_default();
        let karma_effect =
            karma_raw
                .parse::<i8>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "categories.csv",
                    row,
                    message: format!("invalid karma effect '{karma_raw}': {error}"),
                })?;
        if !(-1..=1).contains(&karma_effect) {
            return Err(StorageError::InvalidCsvData {
                file: "categories.csv",
                row,
                message: format!("karma effect {karma_effect} is outside -1..1"),
            });
        }
        let archived = if has_archived_state {
            let raw = record.get(5).unwrap_or_default();
            raw.parse::<bool>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "categories.csv",
                    row,
                    message: format!("invalid archived state '{raw}': {error}"),
                })?
        } else {
            false
        };

        let category = Category {
            id: CategoryId::new(id),
            name,
            color: COLORS[color_idx],
            description,
            karma_effect,
        };
        if archived {
            loaded.archived_categories.push(category);
        } else {
            loaded.categories.push(category);
        }
        loaded.next_category_id = loaded.next_category_id.max(id.saturating_add(1));
    }

    Ok(loaded)
}

#[cfg(test)]
pub fn load_sessions_from_csv(path: &Path, categories: &[Category]) -> LoadedSessions {
    match try_load_sessions_from_csv(path, categories) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("Warning: Could not load sessions file: {}", e);
            default_sessions_loaded()
        }
    }
}

pub fn try_load_sessions_from_csv(
    path: &Path,
    categories: &[Category],
) -> Result<LoadedSessions, StorageError> {
    if !path.exists() {
        return Ok(default_sessions_loaded());
    }

    let category_by_id: HashMap<u64, CategoryId> = categories
        .iter()
        .map(|category| (category.id.0, category.id))
        .collect();

    let mut reader = ReaderBuilder::new().has_headers(true).from_path(path)?;
    let headers = reader.headers()?.clone();
    let has_project = csv_header_matches(&headers, &SESSIONS_HEADER);
    let has_temporal_provenance =
        has_project || csv_header_matches(&headers, &TEMPORAL_SESSIONS_HEADER);
    if !has_temporal_provenance && !csv_header_matches(&headers, &LEGACY_SESSIONS_HEADER) {
        return Err(StorageError::InvalidCsvSchema {
            file: "time_log.csv",
            expected: format!(
                "{}, {}, or {}",
                LEGACY_SESSIONS_HEADER.join(","),
                TEMPORAL_SESSIONS_HEADER.join(","),
                SESSIONS_HEADER.join(",")
            ),
            found: csv_header_string(&headers),
        });
    }

    let shift = usize::from(has_project);
    let project_index = has_project.then_some(4);
    let description_index = 4 + shift;
    let start_time_index = 5 + shift;
    let end_time_index = 6 + shift;
    let elapsed_index = 7 + shift;
    let absolute_start_index = 8 + shift;
    let absolute_end_index = 9 + shift;
    let boundary_offset_index = 10 + shift;
    let boundary_start_index = 11 + shift;

    let mut loaded = default_sessions_loaded();
    let mut seen_ids = HashSet::new();

    for (index, record) in reader.records().enumerate() {
        let row = index + 2;
        let record = record?;

        let id_raw = record.get(0).unwrap_or_default();
        let id = id_raw
            .parse::<usize>()
            .map_err(|error| StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: format!("invalid session ID '{id_raw}': {error}"),
            })?;
        if id == 0 {
            return Err(StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: "session ID 0 is reserved".to_string(),
            });
        }
        if !seen_ids.insert(id) {
            return Err(StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: format!("duplicate session ID {id}"),
            });
        }

        let category_raw = record.get(2).unwrap_or_default();
        let category_value =
            category_raw
                .parse::<u64>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "time_log.csv",
                    row,
                    message: format!("invalid category ID '{category_raw}': {error}"),
                })?;
        let category_id = category_by_id.get(&category_value).copied().ok_or_else(|| {
            StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: format!(
                    "unknown category ID {category_value}; restore its catalog row or explicitly reassign the session"
                ),
            }
        })?;

        let provenance_is_empty = (absolute_start_index..=boundary_start_index)
            .all(|field| record.get(field).unwrap_or_default().trim().is_empty());
        let (started_at_utc, ended_at_utc, operational_day_policy) = if has_temporal_provenance
            && provenance_is_empty
        {
            (None, None, None)
        } else if has_temporal_provenance {
            let parse_timestamp =
                |field: usize, label: &str| -> Result<DateTime<Utc>, StorageError> {
                    let raw = record.get(field).unwrap_or_default();
                    DateTime::parse_from_rfc3339(raw)
                        .map(|value| value.with_timezone(&Utc))
                        .map_err(|error| StorageError::InvalidCsvData {
                            file: "time_log.csv",
                            row,
                            message: format!("invalid {label} '{raw}': {error}"),
                        })
                };
            let started = parse_timestamp(absolute_start_index, "start timestamp")?;
            let ended = parse_timestamp(absolute_end_index, "end timestamp")?;
            let utc_offset_seconds = record
                .get(boundary_offset_index)
                .unwrap_or_default()
                .parse::<i32>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "time_log.csv",
                    row,
                    message: format!("invalid boundary UTC offset: {error}"),
                })?;
            if chrono::FixedOffset::east_opt(utc_offset_seconds).is_none() {
                return Err(StorageError::InvalidCsvData {
                    file: "time_log.csv",
                    row,
                    message: format!("boundary UTC offset {utc_offset_seconds} is unsupported"),
                });
            }
            let start_minutes = record
                .get(boundary_start_index)
                .unwrap_or_default()
                .parse::<u16>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "time_log.csv",
                    row,
                    message: format!("invalid boundary start minute: {error}"),
                })?;
            if start_minutes > 1439 {
                return Err(StorageError::InvalidCsvData {
                    file: "time_log.csv",
                    row,
                    message: format!("boundary start minute {start_minutes} is outside 0..1439"),
                });
            }
            (
                Some(started),
                Some(ended),
                Some(OperationalDayPolicy {
                    utc_offset_seconds,
                    start_minutes,
                }),
            )
        } else {
            (None, None, None)
        };

        let elapsed_raw = record.get(elapsed_index).unwrap_or_default();
        let elapsed_seconds =
            elapsed_raw
                .parse::<usize>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "time_log.csv",
                    row,
                    message: format!("invalid elapsed seconds '{elapsed_raw}': {error}"),
                })?;

        loaded.sessions.push(Session {
            id,
            date: record.get(1).unwrap_or_default().to_string(),
            category_id,
            project: project_index
                .and_then(|field| record.get(field))
                .unwrap_or_default()
                .to_string(),
            description: record
                .get(description_index)
                .unwrap_or_default()
                .to_string(),
            start_time: record.get(start_time_index).unwrap_or_default().to_string(),
            end_time: record.get(end_time_index).unwrap_or_default().to_string(),
            elapsed_seconds,
            started_at_utc,
            ended_at_utc,
            operational_day_policy,
        });

        loaded.next_session_id = loaded.next_session_id.max(id + 1);
    }

    Ok(loaded)
}

pub fn save_categories_to_csv(path: &Path, categories: &[Category]) -> Result<(), String> {
    save_category_catalog_to_csv(path, categories, &[])
}

pub fn save_category_catalog_to_csv(
    path: &Path,
    active_categories: &[Category],
    archived_categories: &[Category],
) -> Result<(), String> {
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(vec![]);
    writer
        .write_record(CATEGORIES_HEADER)
        .map_err(|e| e.to_string())?;

    for (category, archived) in active_categories
        .iter()
        .map(|category| (category, false))
        .chain(archived_categories.iter().map(|category| (category, true)))
    {
        if category.id == DRIFT_CATEGORY_ID {
            continue;
        }

        let color_pos = COLORS
            .iter()
            .position(|&color| color == category.color)
            .ok_or_else(|| {
                format!(
                    "category {} uses a color outside the persisted palette",
                    category.id.0
                )
            })?;

        writer
            .write_record([
                category.id.0.to_string(),
                category.name.clone(),
                category.description.clone(),
                color_pos.to_string(),
                category.karma_effect.to_string(),
                archived.to_string(),
            ])
            .map_err(|e| e.to_string())?;
    }

    let bytes = writer.into_inner().map_err(|e| e.error().to_string())?;
    let content = String::from_utf8_lossy(&bytes).to_string();

    atomic_write(path, &content)
}

pub fn save_sessions_to_csv(
    path: &Path,
    sessions: &[Session],
    categories: &[Category],
) -> Result<(), String> {
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(vec![]);
    writer
        .write_record(SESSIONS_HEADER)
        .map_err(|e| e.to_string())?;

    for session in sessions {
        let category_name = categories
            .iter()
            .find(|category| category.id == session.category_id)
            .map(|category| category.name.as_str())
            .ok_or_else(|| {
                format!(
                    "session {} references unknown category ID {}",
                    session.id, session.category_id.0
                )
            })?;

        let (started_at_utc, ended_at_utc, offset, start_minutes) = match (
            session.started_at_utc,
            session.ended_at_utc,
            session.operational_day_policy,
        ) {
            (Some(started), Some(ended), Some(policy)) => (
                started.to_rfc3339(),
                ended.to_rfc3339(),
                policy.utc_offset_seconds.to_string(),
                policy.start_minutes.to_string(),
            ),
            _ => (String::new(), String::new(), String::new(), String::new()),
        };

        writer
            .write_record([
                session.id.to_string(),
                session.date.clone(),
                session.category_id.0.to_string(),
                category_name.to_string(),
                session.project.clone(),
                session.description.clone(),
                session.start_time.clone(),
                session.end_time.clone(),
                session.elapsed_seconds.to_string(),
                started_at_utc,
                ended_at_utc,
                offset,
                start_minutes,
            ])
            .map_err(|e| e.to_string())?;
    }

    let bytes = writer.into_inner().map_err(|e| e.error().to_string())?;
    let content = String::from_utf8_lossy(&bytes).to_string();

    atomic_write(path, &content)
}

pub fn get_data_dir() -> PathBuf {
    if let Ok(raw_override) = std::env::var("STRATA_DATA_DIR") {
        let trimmed = raw_override.trim();
        if !trimmed.is_empty() {
            let override_dir = PathBuf::from(trimmed);
            fs::create_dir_all(&override_dir).ok();
            return override_dir;
        }
    }

    if let Some(proj_dirs) = ProjectDirs::from("com", "strata", "strata") {
        let data_dir = proj_dirs.data_dir().to_path_buf();
        fs::create_dir_all(&data_dir).ok();
        data_dir
    } else {
        PathBuf::from(".")
    }
}

pub fn get_config_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "strata", "strata") {
        let config_dir = proj_dirs.config_dir().to_path_buf();
        fs::create_dir_all(&config_dir).ok();
        config_dir
    } else {
        PathBuf::from(".")
    }
}

pub fn get_state_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "strata", "strata")
        && let Some(state_dir) = proj_dirs.state_dir()
    {
        let dir = state_dir.to_path_buf();
        fs::create_dir_all(&dir).ok();
        return dir;
    }
    PathBuf::from(".")
}

pub fn get_active_session_path() -> PathBuf {
    get_state_dir().join("active_session.json")
}

pub fn get_detached_runtime_path() -> PathBuf {
    get_state_dir().join("detached_runtime.json")
}

pub fn get_sand_state_path() -> PathBuf {
    get_state_dir().join("sand_state.json")
}

pub fn get_sand_history_dir() -> PathBuf {
    let dir = get_state_dir().join("sand_history");
    fs::create_dir_all(&dir).ok();
    dir
}

#[cfg(test)]
pub fn get_sand_history_path_for_day(day: NaiveDate) -> PathBuf {
    let filename = format!("{}.json", day.format("%Y-%m-%d"));
    get_sand_history_dir().join(filename)
}

pub fn get_sand_contribution_path_for_day(day: NaiveDate) -> PathBuf {
    let filename = format!("{}.contribution.json", day.format("%Y-%m-%d"));
    get_sand_history_dir().join(filename)
}

pub fn get_category_tags_path() -> PathBuf {
    get_state_dir().join("category_tags.json")
}

pub fn get_keymap_path() -> PathBuf {
    get_config_dir().join("keymap.json")
}

pub fn get_categories_path() -> PathBuf {
    get_data_dir().join("categories.csv")
}

pub fn get_time_log_path() -> PathBuf {
    if let Some(override_path) = runtime_storage_settings().time_log_path {
        if let Some(parent) = override_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        return override_path;
    }

    get_data_dir().join("time_log.csv")
}

pub fn normalize_time_log_path_input(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = PathBuf::from(trimmed);
    let normalized = if candidate
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
    {
        candidate
    } else {
        candidate.join("time_log.csv")
    };

    Some(normalized)
}

pub fn load_sand_state(path: &Path) -> Option<SandState> {
    if !path.exists() {
        return None;
    }

    match read_json::<SandState>(path) {
        Ok(state) if state.version == SandState::VERSION => Some(state),
        Ok(_) => {
            eprintln!("Warning: Unsupported sand state version, ignoring saved layout");
            None
        }
        Err(e) => {
            eprintln!("Warning: Could not load sand state: {}", e);
            None
        }
    }
}

pub fn save_sand_state(path: &Path, state: &SandState) -> Result<(), String> {
    write_json_atomic(path, state)
}

pub fn load_category_tags(path: &Path) -> CategoryTagsState {
    if !path.exists() {
        return CategoryTagsState::default();
    }

    match read_json::<CategoryTagsState>(path) {
        Ok(mut state) if state.version == CategoryTagsState::VERSION => {
            for tags in state.tags_by_category.values_mut() {
                tags.retain(|tag| !tag.trim().is_empty());
            }
            state
        }
        Ok(_) => {
            eprintln!("Warning: Unsupported category tags version, ignoring saved tags");
            CategoryTagsState::default()
        }
        Err(e) => {
            eprintln!("Warning: Could not load category tags: {}", e);
            CategoryTagsState::default()
        }
    }
}

pub fn save_category_tags(path: &Path, tags_state: &CategoryTagsState) -> Result<(), String> {
    write_json_atomic(path, tags_state)
}

pub fn file_exists(path: &Path) -> bool {
    path.exists()
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    atomic_write(path, &json)
}

pub fn delete_file_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn write_text_file(path: &Path, content: &str) -> Result<(), String> {
    atomic_write(path, content)
}

pub fn create_backup(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let backup_dir = path.parent().unwrap_or(Path::new(".")).join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!(
        "{}.{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        timestamp
    );
    let backup_path = backup_dir.join(&filename);
    fs::copy(path, &backup_path).map_err(|e| e.to_string())?;

    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    if let Ok(entries) = fs::read_dir(&backup_dir) {
        let mut backups: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(&*stem))
            .collect();
        backups.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

        while backups.len() > BACKUP_RETENTION_MAX_FILES {
            if let Some(oldest) = backups.first() {
                let _ = fs::remove_file(oldest.path());
                backups.remove(0);
            }
        }
    }

    Ok(())
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if path.exists() {
        create_backup(path)?;
    }

    let tmp_path = path.with_extension("tmp");
    let mut tmp_file = File::create(&tmp_path).map_err(|e| e.to_string())?;
    tmp_file
        .write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    tmp_file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use serde::{Deserialize, Serialize};

    use super::*;

    fn unique_path(prefix: &str, extension: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        PathBuf::from(format!("/tmp/{}_{}.{}", prefix, now, extension))
    }

    #[test]
    fn test_categories_round_trip() {
        let path = unique_path("strata_categories_roundtrip", "csv");
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: "focus, deep work".to_string(),
                karma_effect: 1,
            },
        ];

        save_categories_to_csv(&path, &categories).unwrap();
        let loaded = load_categories_from_csv(&path);

        assert_eq!(loaded.categories.len(), 2);
        assert_eq!(loaded.categories[1].id, CategoryId::new(1));
        assert_eq!(loaded.categories[1].name, "Work");
        assert_eq!(loaded.categories[1].description, "focus, deep work");

        fs::remove_file(path).ok();
    }

    #[test]
    fn category_catalog_is_backward_compatible_and_preserves_archived_rows() {
        let legacy_path = unique_path("strata_categories_legacy_catalog", "csv");
        fs::write(
            &legacy_path,
            "id,name,description,color_index,karma_effect\n1,Work,focus,0,1\n",
        )
        .unwrap();
        let legacy = try_load_categories_from_csv(&legacy_path).unwrap();
        assert_eq!(legacy.categories.len(), 2);
        assert!(legacy.archived_categories.is_empty());

        let catalog_path = unique_path("strata_categories_archived_catalog", "csv");
        let active = vec![
            Category {
                id: DRIFT_CATEGORY_ID,
                name: "idle".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 0,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: "focus".to_string(),
                karma_effect: 1,
            },
        ];
        let archived = vec![Category {
            id: CategoryId::new(2),
            name: "Old Client".to_string(),
            color: COLORS[1],
            description: "historical".to_string(),
            karma_effect: -1,
        }];
        save_category_catalog_to_csv(&catalog_path, &active, &archived).unwrap();
        let loaded = try_load_categories_from_csv(&catalog_path).unwrap();
        assert_eq!(loaded.categories.len(), 2);
        assert_eq!(loaded.archived_categories.len(), 1);
        assert_eq!(loaded.archived_categories[0].id, CategoryId::new(2));
        assert_eq!(loaded.archived_categories[0].name, "Old Client");
        assert_eq!(loaded.archived_categories[0].description, "historical");
        assert_eq!(loaded.archived_categories[0].karma_effect, -1);

        fs::remove_file(legacy_path).ok();
        fs::remove_file(catalog_path).ok();
    }

    #[test]
    fn malformed_duplicate_session_ids_and_elapsed_fail_closed() {
        let categories = default_categories_loaded().categories;

        let malformed_id_path = unique_path("strata_sessions_malformed_id", "csv");
        fs::write(
            &malformed_id_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\nnot-an-id,2026-08-01,0,idle,break,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let malformed_id = try_load_sessions_from_csv(&malformed_id_path, &categories).unwrap_err();
        assert!(
            malformed_id
                .to_string()
                .contains("invalid session ID 'not-an-id'")
        );

        let zero_id_path = unique_path("strata_sessions_zero_id", "csv");
        fs::write(
            &zero_id_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n0,2026-08-01,0,idle,break,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let zero_id = try_load_sessions_from_csv(&zero_id_path, &categories).unwrap_err();
        assert!(zero_id.to_string().contains("session ID 0 is reserved"));

        let duplicate_path = unique_path("strata_sessions_duplicate_id", "csv");
        fs::write(
            &duplicate_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,0,idle,first,10:00:00,11:00:00,3600\n1,2026-08-01,0,idle,second,11:00:00,12:00:00,3600\n",
        )
        .unwrap();
        let duplicate = try_load_sessions_from_csv(&duplicate_path, &categories).unwrap_err();
        assert!(duplicate.to_string().contains("duplicate session ID 1"));

        let elapsed_path = unique_path("strata_sessions_malformed_elapsed", "csv");
        fs::write(
            &elapsed_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,0,idle,break,10:00:00,11:00:00,not-seconds\n",
        )
        .unwrap();
        let elapsed = try_load_sessions_from_csv(&elapsed_path, &categories).unwrap_err();
        assert!(
            elapsed
                .to_string()
                .contains("invalid elapsed seconds 'not-seconds'")
        );

        fs::remove_file(malformed_id_path).ok();
        fs::remove_file(zero_id_path).ok();
        fs::remove_file(duplicate_path).ok();
        fs::remove_file(elapsed_path).ok();
    }

    #[test]
    fn unknown_or_malformed_session_category_fails_closed_but_idle_remains_valid() {
        let categories = default_categories_loaded().categories;
        let missing_path = unique_path("strata_sessions_missing_category", "csv");
        fs::write(
            &missing_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,42,Missing,work,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let missing = try_load_sessions_from_csv(&missing_path, &categories).unwrap_err();
        assert!(missing.to_string().contains("unknown category ID 42"));

        let malformed_path = unique_path("strata_sessions_malformed_category", "csv");
        fs::write(
            &malformed_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,not-an-id,Missing,work,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let malformed = try_load_sessions_from_csv(&malformed_path, &categories).unwrap_err();
        assert!(
            malformed
                .to_string()
                .contains("invalid category ID 'not-an-id'")
        );

        let idle_path = unique_path("strata_sessions_idle_category", "csv");
        fs::write(
            &idle_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,0,idle,break,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let idle = try_load_sessions_from_csv(&idle_path, &categories).unwrap();
        assert_eq!(idle.sessions[0].category_id, DRIFT_CATEGORY_ID);

        fs::remove_file(missing_path).ok();
        fs::remove_file(malformed_path).ok();
        fs::remove_file(idle_path).ok();
    }

    #[test]
    fn session_writer_refuses_unknown_category_identity() {
        let path = unique_path("strata_sessions_unknown_writer", "csv");
        let session = Session {
            id: 7,
            date: "2026-08-01".to_string(),
            category_id: CategoryId::new(99),
            project: String::new(),
            description: String::new(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: None,
            ended_at_utc: None,
            operational_day_policy: None,
        };
        let error =
            save_sessions_to_csv(&path, &[session], &default_categories_loaded().categories)
                .unwrap_err();
        assert!(error.contains("unknown category ID 99"));
        assert!(!path.exists());
    }

    #[test]
    fn archived_category_metadata_keeps_session_labels_round_trippable() {
        let path = unique_path("strata_sessions_archived_label", "csv");
        let archived = Category {
            id: CategoryId::new(8),
            name: "Archived Work".to_string(),
            color: COLORS[2],
            description: "retained".to_string(),
            karma_effect: 1,
        };
        let session = Session {
            id: 8,
            date: "2026-08-01".to_string(),
            category_id: archived.id,
            project: String::new(),
            description: "historical".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: None,
            ended_at_utc: None,
            operational_day_policy: None,
        };
        let mut catalog = default_categories_loaded().categories;
        catalog.push(archived.clone());
        save_sessions_to_csv(&path, &[session], &catalog).unwrap();
        let loaded = try_load_sessions_from_csv(&path, &catalog).unwrap();
        assert_eq!(loaded.sessions[0].category_id, archived.id);
        let csv = fs::read_to_string(&path).unwrap();
        assert!(csv.contains("Archived Work"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_sessions_round_trip() {
        let path = unique_path("strata_sessions_roundtrip", "csv");
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(2),
                name: "DeepWork".to_string(),
                color: COLORS[1],
                description: String::new(),
                karma_effect: 1,
            },
        ];
        let sessions = vec![Session {
            id: 7,
            date: "2026-02-25".to_string(),
            category_id: CategoryId::new(2),
            project: "Client A".to_string(),
            description: "plan, review".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: None,
            ended_at_utc: None,
            operational_day_policy: None,
        }];

        save_sessions_to_csv(&path, &sessions, &categories).unwrap();
        let loaded = load_sessions_from_csv(&path, &categories);

        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].id, 7);
        assert_eq!(loaded.sessions[0].category_id, CategoryId::new(2));
        assert_eq!(loaded.sessions[0].elapsed_seconds, 3600);
        assert_eq!(loaded.sessions[0].project, "Client A");
        assert_eq!(loaded.sessions[0].description, "plan, review");

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_legacy_session_headers_load_without_inventing_project() {
        let categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "idle".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 0,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: COLORS[0],
                description: String::new(),
                karma_effect: 1,
            },
        ];
        let legacy_path = unique_path("strata_sessions_legacy_project", "csv");
        fs::write(
            &legacy_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,1,Work,legacy,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let legacy = load_sessions_from_csv(&legacy_path, &categories);
        assert_eq!(legacy.sessions[0].project, "");

        let temporal_path = unique_path("strata_sessions_temporal_project", "csv");
        fs::write(
            &temporal_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds,started_at_utc,ended_at_utc,boundary_utc_offset_seconds,boundary_start_minutes\n1,2026-08-01,1,Work,temporal,10:00:00,11:00:00,3600,2026-08-01T16:00:00Z,2026-08-01T17:00:00Z,-21600,360\n",
        )
        .unwrap();
        let temporal = load_sessions_from_csv(&temporal_path, &categories);
        assert_eq!(temporal.sessions[0].project, "");

        fs::remove_file(legacy_path).ok();
        fs::remove_file(temporal_path).ok();
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestJsonValue {
        name: String,
        count: usize,
    }

    #[test]
    fn test_json_helper_round_trip() {
        let path = unique_path("strata_json_roundtrip", "json");
        let value = TestJsonValue {
            name: "sample".to_string(),
            count: 3,
        };

        write_json_atomic(&path, &value).unwrap();
        let loaded: TestJsonValue = read_json(&path).unwrap();
        assert_eq!(loaded, value);

        delete_file_if_exists(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_sand_contribution_path_is_distinct_from_legacy_history() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
        let legacy = get_sand_history_path_for_day(day);
        let contribution = get_sand_contribution_path_for_day(day);
        assert_ne!(legacy, contribution);
        assert!(
            contribution
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".contribution.json")
        );
    }

    #[test]
    fn test_sand_state_round_trip() {
        let path = unique_path("strata_sand_state_roundtrip", "json");
        let state = SandState {
            version: SandState::VERSION,
            grid_width: 8,
            grid_height: 6,
            grains: vec![
                crate::sand::SandStateGrain {
                    x: 1,
                    y: 2,
                    category_id: 3,
                },
                crate::sand::SandStateGrain {
                    x: 4,
                    y: 5,
                    category_id: 0,
                },
            ],
            frame_count: 12,
            sweep_left_to_right: false,
            rng_state: 12345,
            pending_grains: vec![3],
            pending_runs: Vec::new(),
        };

        save_sand_state(&path, &state).unwrap();
        let loaded = load_sand_state(&path).expect("sand state should load");
        assert_eq!(loaded, state);

        delete_file_if_exists(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_sand_history_path_for_day_uses_expected_filename() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 25).expect("valid day");
        let path = get_sand_history_path_for_day(day);
        let display = path.to_string_lossy();

        assert!(display.ends_with("sand_history/2026-02-25.json"));
    }

    #[test]
    fn test_category_tags_round_trip() {
        let path = unique_path("strata_category_tags_roundtrip", "json");
        let mut state = CategoryTagsState::default();
        state
            .tags_by_category
            .insert(2, vec!["focus".to_string(), "deep work".to_string()]);
        state
            .tags_by_category
            .insert(0, vec!["idle".to_string(), "break".to_string()]);

        save_category_tags(&path, &state).unwrap();
        let loaded = load_category_tags(&path);
        assert_eq!(loaded, state);

        delete_file_if_exists(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_try_load_categories_invalid_schema_returns_error() {
        let path = unique_path("strata_categories_invalid_schema", "csv");
        fs::write(&path, "name,description\nwork,focus\n").unwrap();

        let err = try_load_categories_from_csv(&path).expect_err("schema should be rejected");
        assert!(matches!(err, StorageError::InvalidCsvSchema { .. }));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_try_load_sessions_invalid_schema_returns_error() {
        let path = unique_path("strata_sessions_invalid_schema", "csv");
        fs::write(&path, "date,category,elapsed\n2026-02-25,work,120\n").unwrap();

        let categories = default_categories_loaded().categories;
        let err =
            try_load_sessions_from_csv(&path, &categories).expect_err("schema should be rejected");
        assert!(matches!(err, StorageError::InvalidCsvSchema { .. }));

        fs::remove_file(path).ok();
    }
}
