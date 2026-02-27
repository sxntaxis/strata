use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Local;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use directories::ProjectDirs;
use ratatui::style::Color;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::{
    constants::COLORS,
    domain::{Category, CategoryId, Session},
    sand::SandState,
};

#[derive(Debug)]
pub struct LoadedCategories {
    pub categories: Vec<Category>,
    pub next_category_id: u64,
}

#[derive(Debug)]
pub struct LoadedSessions {
    pub sessions: Vec<Session>,
    pub next_session_id: usize,
}

const CATEGORIES_HEADER: [&str; 5] = ["id", "name", "description", "color_index", "karma_effect"];
const SESSIONS_HEADER: [&str; 8] = [
    "id",
    "date",
    "category_id",
    "category_name",
    "description",
    "start_time",
    "end_time",
    "elapsed_seconds",
];

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

fn default_categories_loaded() -> LoadedCategories {
    LoadedCategories {
        categories: vec![Category {
            id: CategoryId::new(0),
            name: "none".to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 0,
        }],
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
    if !csv_header_matches(&headers, &CATEGORIES_HEADER) {
        return Err(StorageError::InvalidCsvSchema {
            file: "categories.csv",
            expected: CATEGORIES_HEADER.join(","),
            found: csv_header_string(&headers),
        });
    }

    let mut loaded = default_categories_loaded();

    for record in reader.records() {
        let record = record?;

        let Some(id_raw) = record.get(0) else {
            continue;
        };
        let id: u64 = match id_raw.parse() {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Warning: Invalid category ID '{}', skipping", id_raw);
                continue;
            }
        };

        if id == 0 {
            continue;
        }

        let name = record.get(1).unwrap_or_default().trim().to_string();
        if name.is_empty() || name.eq_ignore_ascii_case("none") {
            continue;
        }

        let description = record.get(2).unwrap_or_default().to_string();
        let color_idx = record
            .get(3)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0)
            % COLORS.len();
        let karma_effect = record
            .get(4)
            .and_then(|value| value.parse::<i8>().ok())
            .unwrap_or(1);

        loaded.categories.push(Category {
            id: CategoryId::new(id),
            name,
            color: COLORS[color_idx],
            description,
            karma_effect,
        });
        loaded.next_category_id = loaded.next_category_id.max(id + 1);
    }

    Ok(loaded)
}

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
    if !csv_header_matches(&headers, &SESSIONS_HEADER) {
        return Err(StorageError::InvalidCsvSchema {
            file: "time_log.csv",
            expected: SESSIONS_HEADER.join(","),
            found: csv_header_string(&headers),
        });
    }

    let mut loaded = default_sessions_loaded();

    for record in reader.records() {
        let record = record?;

        let Some(id_raw) = record.get(0) else {
            continue;
        };
        let id: usize = match id_raw.parse() {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Warning: Invalid session ID '{}', skipping", id_raw);
                continue;
            }
        };

        let category_id = record
            .get(2)
            .and_then(|value| value.parse::<u64>().ok())
            .and_then(|raw| category_by_id.get(&raw).copied())
            .unwrap_or(CategoryId::new(0));

        loaded.sessions.push(Session {
            id,
            date: record.get(1).unwrap_or_default().to_string(),
            category_id,
            description: record.get(4).unwrap_or_default().to_string(),
            start_time: record.get(5).unwrap_or_default().to_string(),
            end_time: record.get(6).unwrap_or_default().to_string(),
            elapsed_seconds: record
                .get(7)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0),
        });

        loaded.next_session_id = loaded.next_session_id.max(id + 1);
    }

    Ok(loaded)
}

pub fn save_categories_to_csv(path: &Path, categories: &[Category]) -> Result<(), String> {
    let mut writer = WriterBuilder::new().has_headers(false).from_writer(vec![]);
    writer
        .write_record(CATEGORIES_HEADER)
        .map_err(|e| e.to_string())?;

    for category in categories {
        if category.id.0 == 0 {
            continue;
        }

        let color_pos = COLORS
            .iter()
            .position(|&color| color == category.color)
            .unwrap_or(0);

        writer
            .write_record([
                category.id.0.to_string(),
                category.name.clone(),
                category.description.clone(),
                color_pos.to_string(),
                category.karma_effect.to_string(),
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
            .unwrap_or("none");

        writer
            .write_record([
                session.id.to_string(),
                session.date.clone(),
                session.category_id.0.to_string(),
                category_name.to_string(),
                session.description.clone(),
                session.start_time.clone(),
                session.end_time.clone(),
                session.elapsed_seconds.to_string(),
            ])
            .map_err(|e| e.to_string())?;
    }

    let bytes = writer.into_inner().map_err(|e| e.error().to_string())?;
    let content = String::from_utf8_lossy(&bytes).to_string();

    atomic_write(path, &content)
}

pub fn get_data_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "strata", "strata") {
        let data_dir = proj_dirs.data_dir().to_path_buf();
        fs::create_dir_all(&data_dir).ok();
        data_dir
    } else {
        PathBuf::from(".")
    }
}

pub fn get_state_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "strata", "strata") {
        if let Some(state_dir) = proj_dirs.state_dir() {
            let dir = state_dir.to_path_buf();
            fs::create_dir_all(&dir).ok();
            return dir;
        }
    }
    PathBuf::from(".")
}

pub fn get_active_session_path() -> PathBuf {
    get_state_dir().join("active_session.json")
}

pub fn get_sand_state_path() -> PathBuf {
    get_state_dir().join("sand_state.json")
}

pub fn get_category_tags_path() -> PathBuf {
    get_state_dir().join("category_tags.json")
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

        while backups.len() > 10 {
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
            description: "plan, review".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
        }];

        save_sessions_to_csv(&path, &sessions, &categories).unwrap();
        let loaded = load_sessions_from_csv(&path, &categories);

        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].id, 7);
        assert_eq!(loaded.sessions[0].category_id, CategoryId::new(2));
        assert_eq!(loaded.sessions[0].elapsed_seconds, 3600);
        assert_eq!(loaded.sessions[0].description, "plan, review");

        fs::remove_file(path).ok();
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
        };

        save_sand_state(&path, &state).unwrap();
        let loaded = load_sand_state(&path).expect("sand state should load");
        assert_eq!(loaded, state);

        delete_file_if_exists(&path).unwrap();
        assert!(!path.exists());
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
