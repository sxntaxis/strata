use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::Local;
use directories::ProjectDirs;
use ratatui::style::Color;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    constants::COLORS,
    domain::{Category, CategoryId, Session},
    sand::SandState,
};

pub struct LoadedCategories {
    pub categories: Vec<Category>,
    pub next_category_id: u64,
}

pub struct LoadedSessions {
    pub sessions: Vec<Session>,
    pub next_session_id: usize,
}

pub fn load_categories_from_csv(path: &Path) -> LoadedCategories {
    let mut categories = vec![Category {
        id: CategoryId::new(0),
        name: "none".to_string(),
        color: Color::White,
        description: String::new(),
        karma_effect: 1,
    }];

    if !path.exists() {
        return LoadedCategories {
            categories,
            next_category_id: 1,
        };
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read categories file: {}", e);
            return LoadedCategories {
                categories,
                next_category_id: 1,
            };
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return LoadedCategories {
            categories,
            next_category_id: 1,
        };
    }

    let header = lines[0].trim();
    if !header.starts_with("id,name,") {
        panic!("categories.csv is not in canonical format. Run 'strata migrate-csv' first.");
    }

    let mut next_id = 1u64;

    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 5 {
            continue;
        }

        let id: u64 = match parts[0].parse() {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Warning: Invalid category ID '{}', skipping", parts[0]);
                continue;
            }
        };

        if id == 0 {
            continue;
        }

        let name = parts[1].to_string();
        if name == "none" {
            continue;
        }

        let description = parts[2].to_string();
        let color_idx: usize = parts[3].parse().unwrap_or(0) % COLORS.len();
        let karma_effect: i8 = parts[4].parse().unwrap_or(1);

        categories.push(Category {
            id: CategoryId::new(id),
            name,
            color: COLORS[color_idx],
            description,
            karma_effect,
        });
        next_id = next_id.max(id + 1);
    }

    LoadedCategories {
        categories,
        next_category_id: next_id,
    }
}

pub fn load_sessions_from_csv(path: &Path, categories: &[Category]) -> LoadedSessions {
    let category_by_id: HashMap<u64, CategoryId> =
        categories.iter().map(|c| (c.id.0, c.id)).collect();
    let mut sessions = Vec::new();

    if !path.exists() {
        return LoadedSessions {
            sessions,
            next_session_id: 1,
        };
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not read sessions file: {}", e);
            return LoadedSessions {
                sessions,
                next_session_id: 1,
            };
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return LoadedSessions {
            sessions,
            next_session_id: 1,
        };
    }

    let header = lines[0].trim();
    if !header.starts_with("id,date,category_id,") {
        panic!("time_log.csv is not in canonical format. Run 'strata migrate-csv' first.");
    }

    let mut max_id = 0usize;

    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 8 {
            continue;
        }

        let id: usize = match parts[0].parse() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let category_id: u64 = parts[2].parse().unwrap_or(0);
        let category_id = category_by_id
            .get(&category_id)
            .copied()
            .unwrap_or(CategoryId::new(0));

        sessions.push(Session {
            id,
            date: parts[1].to_string(),
            category_id,
            description: parts[4].to_string(),
            start_time: parts[5].to_string(),
            end_time: parts[6].to_string(),
            elapsed_seconds: parts[7].parse().unwrap_or(0),
        });

        max_id = max_id.max(id);
    }

    LoadedSessions {
        sessions,
        next_session_id: max_id + 1,
    }
}

pub fn save_categories_to_csv(path: &Path, categories: &[Category]) -> Result<(), String> {
    let mut content = String::new();
    content.push_str("id,name,description,color_index,karma_effect\n");

    for category in categories {
        if category.id.0 == 0 {
            continue;
        }

        let color_pos = COLORS
            .iter()
            .position(|&color| color == category.color)
            .unwrap_or(0);
        content.push_str(&format!(
            "{},{},{},{},{}\n",
            category.id.0, category.name, category.description, color_pos, category.karma_effect
        ));
    }

    atomic_write(path, &content)
}

pub fn save_sessions_to_csv(
    path: &Path,
    sessions: &[Session],
    categories: &[Category],
) -> Result<(), String> {
    let mut content = String::new();
    content.push_str(
        "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n",
    );

    for session in sessions {
        let category_name = categories
            .iter()
            .find(|category| category.id == session.category_id)
            .map(|category| category.name.as_str())
            .unwrap_or("none");

        content.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            session.id,
            session.date,
            session.category_id.0,
            category_name,
            session.description,
            session.start_time,
            session.end_time,
            session.elapsed_seconds
        ));
    }

    atomic_write(path, &content)
}

pub fn get_data_dir() -> PathBuf {
    let local_categories = Path::new("./categories.csv");
    let local_timelog = Path::new("./time_log.csv");
    if local_categories.exists() || local_timelog.exists() {
        return PathBuf::from(".");
    }

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

pub fn migrate_csv() -> Result<(), String> {
    let data_dir = get_data_dir();
    let categories_path = data_dir.join("categories.csv");
    let sessions_path = data_dir.join("time_log.csv");

    println!("Migrating CSV files in: {}", data_dir.display());

    if categories_path.exists() {
        let content = fs::read_to_string(&categories_path).map_err(|e| e.to_string())?;
        let lines: Vec<&str> = content.lines().collect();

        if !lines.is_empty() {
            let header = lines[0];
            let needs_migration = !header.starts_with("id,name,");

            if needs_migration {
                println!("Migrating categories.csv...");
                create_backup(&categories_path)?;

                let mut new_lines =
                    vec!["id,name,description,color_index,karma_effect".to_string()];
                let mut next_id = 1u64;

                for line in lines.iter().skip(1) {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        let name = parts[0];
                        let desc = parts.get(1).unwrap_or(&"").to_string();
                        let color = parts.get(2).unwrap_or(&"0");
                        let karma = parts.get(3).unwrap_or(&"1");

                        new_lines
                            .push(format!("{},{},{},{},{}", next_id, name, desc, color, karma));
                        next_id += 1;
                    }
                }

                let new_content = new_lines.join("\n");
                atomic_write(&categories_path, &new_content)?;
                println!("  Migrated {} categories", next_id - 1);
            } else {
                println!("categories.csv already in canonical format");
            }
        }
    }

    if sessions_path.exists() {
        let content = fs::read_to_string(&sessions_path).map_err(|e| e.to_string())?;
        let lines: Vec<&str> = content.lines().collect();

        if !lines.is_empty() {
            let header = lines[0];
            let needs_migration = !header.starts_with("id,date,category_id,");

            if needs_migration {
                println!("Migrating time_log.csv...");
                create_backup(&sessions_path)?;

                let categories = load_categories_from_csv(&categories_path).categories;
                let category_map: HashMap<String, u64> = categories
                    .iter()
                    .map(|c| (c.name.clone(), c.id.0))
                    .collect();

                let mut new_lines =
                    vec!["id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds".to_string()];
                let mut session_count = 0;

                for line in lines.iter().skip(1) {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() < 7 {
                        continue;
                    }

                    let id = parts[0];
                    let date = parts[1];
                    let cat_name = parts[2];
                    let desc = parts.get(3).unwrap_or(&"").to_string();
                    let start = parts.get(4).unwrap_or(&"");
                    let end_time = parts.get(5).unwrap_or(&"");
                    let elapsed = parts.get(6).unwrap_or(&"0");

                    let cat_id = category_map.get(cat_name).copied().unwrap_or(0);

                    new_lines.push(format!(
                        "{},{},{},{},{},{},{},{}",
                        id, date, cat_id, cat_name, desc, start, end_time, elapsed
                    ));
                    session_count += 1;
                }

                let new_content = new_lines.join("\n");
                atomic_write(&sessions_path, &new_content)?;
                println!("  Migrated {} sessions", session_count);
            } else {
                println!("time_log.csv already in canonical format");
            }
        }
    }

    println!("Migration complete!");
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
                description: "focus".to_string(),
                karma_effect: 1,
            },
        ];

        save_categories_to_csv(&path, &categories).unwrap();
        let loaded = load_categories_from_csv(&path);

        assert_eq!(loaded.categories.len(), 2);
        assert_eq!(loaded.categories[1].id, CategoryId::new(1));
        assert_eq!(loaded.categories[1].name, "Work");
        assert_eq!(loaded.categories[1].description, "focus");

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
            description: "plan".to_string(),
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
}
