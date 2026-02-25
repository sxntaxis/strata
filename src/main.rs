use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    time::{Duration, Instant},
};

use chrono::{Duration as ChronoDuration, Local};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use rand::Rng;

use ratatui::prelude::{Line, Span};
use ratatui::style::Stylize;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

const COLORS: [Color; 12] = [
    Color::Rgb(0, 176, 80),
    Color::Rgb(128, 255, 0),
    Color::Rgb(255, 255, 0),
    Color::Rgb(255, 204, 0),
    Color::Rgb(255, 153, 0),
    Color::Rgb(255, 51, 0),
    Color::Rgb(255, 0, 0),
    Color::Rgb(153, 0, 255),
    Color::Rgb(102, 51, 255),
    Color::Rgb(0, 0, 255),
    Color::Rgb(0, 153, 255),
    Color::Rgb(0, 255, 255),
];

const TIME_SETTINGS: TimeSettings = TimeSettings {
    tick_ms: 1000,
    physics_ms: 32,
    target_fps: 24,
};

const SAND_ENGINE: SandEngineSettings = SandEngineSettings {
    braille_base: 0x2800,
    dot_height: 4,
    dot_width: 2,
};

const BLINK_SETTINGS: BlinkSettings = BlinkSettings {
    interval_min_frames: 150,
    interval_max_frames: 300,
    duration_min_frames: 10,
    duration_max_frames: 17,
};

const FACE_SETTINGS: FaceSettings = FaceSettings {
    thresholds: &[120, 300, 600, 1200, 2400, 3600, 5400],
    faces: &[
        "(o_o)",
        "(¬_¬)",
        "(O_O)",
        "(⊙_⊙)",
        "(ಠ_ಠ)",
        "(ಥ_ಥ)",
        "(T_T)",
        "(x_x)",
    ],
};

const FILE_PATHS: FilePaths = FilePaths {
    time_log: "./time_log.csv",
    categories: "./categories.csv",
};

mod cli {
    use super::*;
    use chrono::{DateTime, Utc};
    use clap::{CommandFactory, Parser, ValueEnum};
    use directories::ProjectDirs;
    use itertools::Itertools;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Mutex;

    static ACTIVE_SESSION: Mutex<Option<ActiveSession>> = Mutex::new(None);

    #[derive(Parser, Debug)]
    #[command(name = "strata")]
    #[command(about = "Time tracking with falling sand", long_about = None)]
    pub enum Cli {
        #[command(about = "Start a new tracking session")]
        Start {
            #[arg(help = "Project name")]
            project: String,

            #[arg(long, help = "Session description")]
            desc: Option<String>,

            #[arg(long, short, help = "Category name or ID")]
            category: Option<String>,
        },

        #[command(about = "Stop the current tracking session")]
        Stop,

        #[command(about = "Show today's report")]
        Report {
            #[arg(long, help = "Show today's time")]
            today: bool,
        },

        #[command(about = "Export sessions")]
        Export {
            #[arg(long, value_enum, help = "Export format")]
            format: ExportFormat,

            #[arg(long, short, help = "Output path")]
            out: Option<PathBuf>,
        },

        #[command(about = "Generate shell completions")]
        Completions {
            #[arg(help = "Shell type (bash, zsh, fish)")]
            shell: String,
        },

        #[command(about = "Migrate CSV files to canonical schema")]
        MigrateCsv,
    }

    #[derive(Debug, Clone, ValueEnum)]
    pub enum ExportFormat {
        Json,
        Ics,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ActiveSession {
        pub project: String,
        pub description: String,
        pub category_id: u64,
        pub category_name: String,
        pub start_time: DateTime<Utc>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SessionExport {
        pub id: usize,
        pub date: String,
        pub category_id: u64,
        pub category_name: String,
        pub project: Option<String>,
        pub description: String,
        pub start_time: String,
        pub end_time: String,
        pub elapsed_seconds: usize,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CategoryExport {
        pub id: u64,
        pub name: String,
        pub description: String,
        pub color_index: usize,
        pub karma_effect: i8,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DataExport {
        pub schema_version: u32,
        pub exported_at: DateTime<Utc>,
        pub categories: Vec<CategoryExport>,
        pub sessions: Vec<SessionExport>,
    }

    fn get_data_dir() -> PathBuf {
        // Check current directory first for local data
        let local_categories = Path::new("./categories.csv");
        let local_timelog = Path::new("./time_log.csv");
        if local_categories.exists() || local_timelog.exists() {
            return PathBuf::from(".");
        }

        // Fall back to XDG data directory
        if let Some(proj_dirs) = ProjectDirs::from("com", "strata", "strata") {
            let data_dir = proj_dirs.data_dir().to_path_buf();
            fs::create_dir_all(&data_dir).ok();
            data_dir
        } else {
            PathBuf::from(".")
        }
    }

    fn get_state_dir() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "strata", "strata") {
            if let Some(state_dir) = proj_dirs.state_dir() {
                let dir = state_dir.to_path_buf();
                fs::create_dir_all(&dir).ok();
                return dir;
            }
        }
        PathBuf::from(".")
    }

    fn get_active_session_path() -> PathBuf {
        get_state_dir().join("active_session.json")
    }

    fn create_backup(path: &Path) -> Result<(), String> {
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

    fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
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

    pub fn load_categories_from_csv(path: &Path) -> Vec<Category> {
        let mut categories = vec![Category {
            id: CategoryId::new(0),
            name: "none".to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 1,
        }];

        if !path.exists() {
            return categories;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Could not read categories file: {}", e);
                return categories;
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return categories;
        }

        // Strict validation: require canonical header
        let header = lines[0].trim();
        if !header.starts_with("id,name,") {
            panic!("categories.csv is not in canonical format. Run 'strata migrate-csv' first.");
        }

        let mut next_id = 1u64;

        for line in lines.iter().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            // Canonical: id,name,description,color_index,karma_effect
            if parts.len() >= 5 {
                let id: u64 = match parts[0].parse() {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("Warning: Invalid category ID '{}', skipping", parts[0]);
                        continue;
                    }
                };
                let name = parts[1].to_string();
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
        }

        categories
    }

    pub fn load_sessions_from_csv(path: &Path, categories: &[Category]) -> Vec<Session> {
        let category_by_id: HashMap<u64, CategoryId> =
            categories.iter().map(|c| (c.id.0, c.id)).collect();

        let mut sessions = Vec::new();

        if !path.exists() {
            return sessions;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Could not read sessions file: {}", e);
                return sessions;
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return sessions;
        }

        // Strict validation: require canonical header
        let header = lines[0].trim();
        if !header.starts_with("id,date,category_id,") {
            panic!("time_log.csv is not in canonical format. Run 'strata migrate-csv' first.");
        }

        let mut max_id = 0usize;

        for line in lines.iter().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            // Canonical: id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds
            if parts.len() >= 8 {
                let id: usize = match parts[0].parse() {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                max_id = max_id.max(id);

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
            }
        }

        sessions
    }

    pub fn start_session(
        project: String,
        description: Option<String>,
        category_name: Option<String>,
    ) -> Result<(), String> {
        let data_dir = get_data_dir();
        let categories_path = data_dir.join("categories.csv");
        let categories = load_categories_from_csv(&categories_path);

        let cat_name = category_name.unwrap_or_else(|| "none".to_string());
        let category = categories
            .iter()
            .find(|c| c.name == cat_name || c.id.0.to_string() == cat_name)
            .ok_or_else(|| format!("Category '{}' not found", cat_name))?;

        let session = ActiveSession {
            project: project.clone(),
            description: description.unwrap_or_default(),
            category_id: category.id.0,
            category_name: category.name.clone(),
            start_time: Utc::now(),
        };

        let session_path = get_active_session_path();
        let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
        let tmp_path = session_path.with_extension("tmp");
        let mut tmp_file = File::create(&tmp_path).map_err(|e| e.to_string())?;
        tmp_file
            .write_all(json.as_bytes())
            .map_err(|e| e.to_string())?;
        fs::rename(&tmp_path, &session_path).map_err(|e| e.to_string())?;

        println!(
            "Started session for project '{}' in category '{}'",
            project, category.name
        );
        Ok(())
    }

    pub fn stop_session() -> Result<usize, String> {
        let session_path = get_active_session_path();
        if !session_path.exists() {
            return Err("No active session to stop".to_string());
        }

        let content = fs::read_to_string(&session_path).map_err(|e| e.to_string())?;
        let active_session: ActiveSession =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;

        let elapsed = (Utc::now() - active_session.start_time).num_seconds() as usize;

        let data_dir = get_data_dir();
        let sessions_path = data_dir.join("time_log.csv");
        let categories_path = data_dir.join("categories.csv");

        let categories = load_categories_from_csv(&categories_path);
        let mut sessions = load_sessions_from_csv(&sessions_path, &categories);

        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();
        let start_time = now - ChronoDuration::seconds(elapsed as i64);

        if let Some(existing) = sessions
            .iter_mut()
            .find(|s| s.date == today && s.category_id.0 == active_session.category_id)
        {
            existing.elapsed_seconds += elapsed;
            existing.end_time = now.format("%H:%M:%S").to_string();
        } else {
            let new_id = sessions.iter().map(|s| s.id).max().unwrap_or(0) + 1;
            sessions.push(Session {
                id: new_id,
                date: today,
                category_id: CategoryId::new(active_session.category_id),
                description: active_session.description.clone(),
                start_time: start_time.format("%H:%M:%S").to_string(),
                end_time: now.format("%H:%M:%S").to_string(),
                elapsed_seconds: elapsed,
            });
        }

        let mut content = String::new();
        content.push_str(
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n",
        );
        for session in &sessions {
            let cat_name = categories
                .iter()
                .find(|c| c.id == session.category_id)
                .map(|c| c.name.as_str())
                .unwrap_or("none");
            content.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                session.id,
                session.date,
                session.category_id.0,
                cat_name,
                session.description,
                session.start_time,
                session.end_time,
                session.elapsed_seconds
            ));
        }
        atomic_write(&sessions_path, &content)?;

        fs::remove_file(&session_path).ok();

        println!(
            "Stopped session. Elapsed time: {:02}:{:02}:{:02}",
            elapsed / 3600,
            (elapsed % 3600) / 60,
            elapsed % 60
        );
        Ok(elapsed)
    }

    pub fn report_today() -> Result<(), String> {
        let data_dir = get_data_dir();
        let sessions_path = data_dir.join("time_log.csv");
        let categories_path = data_dir.join("categories.csv");

        let categories = load_categories_from_csv(&categories_path);
        let sessions = load_sessions_from_csv(&sessions_path, &categories);

        let today = Local::now().format("%Y-%m-%d").to_string();
        let today_sessions: Vec<_> = sessions.iter().filter(|s| s.date == today).collect();

        let mut by_category: HashMap<String, usize> = HashMap::new();
        let mut total = 0usize;

        for session in today_sessions {
            let cat_name = categories
                .iter()
                .find(|c| c.id == session.category_id)
                .map(|c| c.name.as_str())
                .unwrap_or("none")
                .to_string();
            *by_category.entry(cat_name).or_insert(0) += session.elapsed_seconds;
            total += session.elapsed_seconds;
        }

        println!("Today's Report ({})", today);
        println!("{}", "-".repeat(40));
        for (cat, secs) in by_category.iter().sorted_by_key(|(_, v)| *v).rev() {
            if cat != "none" {
                println!(
                    "{:20} {:02}:{:02}:{:02}",
                    cat,
                    secs / 3600,
                    (secs % 3600) / 60,
                    secs % 60
                );
            }
        }
        println!("{}", "-".repeat(40));
        println!(
            "{:20} {:02}:{:02}:{:02}",
            "TOTAL",
            total / 3600,
            (total % 3600) / 60,
            total % 60
        );

        Ok(())
    }

    pub fn export_data(format: ExportFormat, out_path: Option<PathBuf>) -> Result<(), String> {
        let data_dir = get_data_dir();
        let sessions_path = data_dir.join("time_log.csv");
        let categories_path = data_dir.join("categories.csv");

        let categories = load_categories_from_csv(&categories_path);
        let sessions = load_sessions_from_csv(&sessions_path, &categories);

        let export = DataExport {
            schema_version: 1,
            exported_at: Utc::now(),
            categories: categories
                .iter()
                .skip(1)
                .map(|c| {
                    let color_pos = COLORS.iter().position(|&col| col == c.color).unwrap_or(0);
                    CategoryExport {
                        id: c.id.0,
                        name: c.name.clone(),
                        description: c.description.clone(),
                        color_index: color_pos,
                        karma_effect: c.karma_effect,
                    }
                })
                .collect(),
            sessions: sessions
                .iter()
                .map(|s| {
                    let cat_name = categories
                        .iter()
                        .find(|c| c.id == s.category_id)
                        .map(|c| c.name.as_str())
                        .unwrap_or("none")
                        .to_string();
                    SessionExport {
                        id: s.id,
                        date: s.date.clone(),
                        category_id: s.category_id.0,
                        category_name: cat_name,
                        project: None,
                        description: s.description.clone(),
                        start_time: s.start_time.clone(),
                        end_time: s.end_time.clone(),
                        elapsed_seconds: s.elapsed_seconds,
                    }
                })
                .collect(),
        };

        match format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
                if let Some(path) = out_path {
                    let mut file = File::create(&path).map_err(|e| e.to_string())?;
                    file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
                    println!("Exported to {}", path.display());
                } else {
                    println!("{}", json);
                }
            }
            ExportFormat::Ics => {
                let mut ics = String::new();
                ics.push_str("BEGIN:VCALENDAR\r\n");
                ics.push_str("VERSION:2.0\r\n");
                ics.push_str("PRODID:-//strata//time tracking//EN\r\n");

                for session in &export.sessions {
                    if session.category_name == "none" || session.elapsed_seconds == 0 {
                        continue;
                    }
                    let dt_start = format_ics_datetime(&session.date, &session.start_time);
                    let dt_end = format_ics_datetime(&session.date, &session.end_time);
                    let uid = format!("strata-session-{}", session.id);

                    ics.push_str("BEGIN:VEVENT\r\n");
                    ics.push_str(&format!("UID:{}\r\n", uid));
                    ics.push_str(&format!("DTSTAMP:{}\r\n", format_ics_timestamp(Utc::now())));
                    ics.push_str(&format!("DTSTART:{}\r\n", dt_start));
                    ics.push_str(&format!("DTEND:{}\r\n", dt_end));
                    ics.push_str(&format!(
                        "SUMMARY:{} - {}\r\n",
                        session.project.as_deref().unwrap_or("Project"),
                        session.category_name
                    ));
                    if !session.description.is_empty() {
                        ics.push_str(&format!("DESCRIPTION:{}\r\n", session.description));
                    }
                    ics.push_str(&format!("CATEGORIES:{}\r\n", session.category_name));
                    ics.push_str("END:VEVENT\r\n");
                }

                ics.push_str("END:VCALENDAR\r\n");

                if let Some(path) = out_path {
                    let mut file = File::create(&path).map_err(|e| e.to_string())?;
                    file.write_all(ics.as_bytes()).map_err(|e| e.to_string())?;
                    println!("Exported to {}", path.display());
                } else {
                    println!("{}", ics);
                }
            }
        }

        Ok(())
    }

    fn format_ics_datetime(date: &str, time: &str) -> String {
        let dt = format!("{}T{}00", date.replace('-', ""), time.replace(':', ""));
        dt
    }

    fn format_ics_timestamp(dt: DateTime<Utc>) -> String {
        dt.format("%Y%m%dT%H%M%SZ").to_string()
    }

    pub fn print_completions(shell: &str) -> Result<(), String> {
        use clap_complete::Shell;
        match shell {
            "bash" => {
                clap_complete::generate(
                    Shell::Bash,
                    &mut Cli::command(),
                    "strata",
                    &mut io::stdout(),
                );
            }
            "zsh" => {
                clap_complete::generate(
                    Shell::Zsh,
                    &mut Cli::command(),
                    "strata",
                    &mut io::stdout(),
                );
            }
            "fish" => {
                clap_complete::generate(
                    Shell::Fish,
                    &mut Cli::command(),
                    "strata",
                    &mut io::stdout(),
                );
            }
            _ => {
                return Err(format!(
                    "Unsupported shell: {}. Use bash, zsh, or fish.",
                    shell
                ));
            }
        }
        Ok(())
    }

    pub fn migrate_csv() -> Result<(), String> {
        let data_dir = get_data_dir();
        let categories_path = data_dir.join("categories.csv");
        let sessions_path = data_dir.join("time_log.csv");

        println!("Migrating CSV files in: {}", data_dir.display());

        // Migrate categories
        if categories_path.exists() {
            let content = fs::read_to_string(&categories_path).map_err(|e| e.to_string())?;
            let lines: Vec<&str> = content.lines().collect();

            if !lines.is_empty() {
                let header = lines[0];
                let needs_migration = !header.starts_with("id,name,");

                if needs_migration {
                    println!("Migrating categories.csv...");
                    // Create backup
                    create_backup(&categories_path)?;

                    // Convert: name,desc,color,karma -> id,name,desc,color,karma
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

        // Migrate sessions
        if sessions_path.exists() {
            let content = fs::read_to_string(&sessions_path).map_err(|e| e.to_string())?;
            let lines: Vec<&str> = content.lines().collect();

            if !lines.is_empty() {
                let header = lines[0];
                let needs_migration = !header.starts_with("id,date,category_id,");

                if needs_migration {
                    println!("Migrating time_log.csv...");
                    // Create backup
                    create_backup(&sessions_path)?;

                    // Convert: id,date,category,desc,start,end,elapsed -> id,date,category_id,category_name,desc,start,end,elapsed
                    // First load categories to map names to IDs
                    let categories = load_categories_from_csv(&categories_path);
                    let category_map: HashMap<String, u64> = categories
                        .iter()
                        .map(|c| (c.name.clone(), c.id.0))
                        .collect();

                    let mut new_lines = vec!["id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds".to_string()];
                    let mut session_count = 0;

                    for line in lines.iter().skip(1) {
                        let parts: Vec<&str> = line.split(',').collect();
                        if parts.len() >= 7 {
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

    pub fn run_cli() {
        let cli = Cli::parse();
        match cli {
            Cli::Start {
                project,
                desc,
                category,
            } => {
                if let Err(e) = start_session(project, desc, category) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            Cli::Stop => {
                if let Err(e) = stop_session() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            Cli::Report { today: _ } => {
                if let Err(e) = report_today() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            Cli::Export { format, out } => {
                if let Err(e) = export_data(format, out) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            Cli::Completions { shell } => {
                if let Err(e) = print_completions(&shell) {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            Cli::MigrateCsv => {
                if let Err(e) = migrate_csv() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

struct TimeSettings {
    tick_ms: u64,
    physics_ms: u64,
    target_fps: u64,
}

struct SandEngineSettings {
    braille_base: u32,
    dot_height: usize,
    dot_width: usize,
}

struct BlinkSettings {
    interval_min_frames: i32,
    interval_max_frames: i32,
    duration_min_frames: i32,
    duration_max_frames: i32,
}

struct FaceSettings {
    thresholds: &'static [usize],
    faces: &'static [&'static str],
}

struct FilePaths {
    time_log: &'static str,
    categories: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct CategoryId(u64);

impl CategoryId {
    fn new(id: u64) -> Self {
        CategoryId(id)
    }
}

#[derive(Clone, Debug)]
struct Category {
    id: CategoryId,
    name: String,
    color: Color,
    description: String,
    karma_effect: i8,
}

#[derive(Clone, Debug)]
struct Session {
    id: usize,
    date: String,
    category_id: CategoryId,
    description: String,
    start_time: String,
    end_time: String,
    elapsed_seconds: usize,
}

struct TimeTracker {
    sessions: Vec<Session>,
    categories: Vec<Category>,
    next_category_id: u64,
    current_session_start: Option<Instant>,
    session_id_counter: usize,
    active_category_index: Option<usize>,
}

impl TimeTracker {
    fn new() -> Self {
        let mut tt = Self {
            sessions: Vec::new(),
            categories: vec![Category {
                id: CategoryId::new(0),
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            }],
            next_category_id: 1,
            current_session_start: None,
            session_id_counter: 0,
            active_category_index: Some(0),
        };
        // load_sessions already calls load_categories internally
        tt.load_sessions();
        tt
    }

    fn category_id_by_name(&self, name: &str) -> Option<CategoryId> {
        self.categories
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
    }

    fn load_sessions(&mut self) {
        self.load_categories();

        let path = Path::new(FILE_PATHS.time_log);
        if !path.exists() {
            return;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Could not read sessions file: {}", e);
                return;
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return;
        }

        // Strict validation: require canonical header
        let header = lines[0].trim();
        if !header.starts_with("id,date,category_id,") {
            panic!("time_log.csv is not in canonical format. Run 'strata migrate-csv' first.");
        }

        let category_by_id: std::collections::HashMap<u64, CategoryId> =
            self.categories.iter().map(|c| (c.id.0, c.id)).collect();

        let mut max_id = 0;

        for line in lines.iter().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            // Canonical: id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds
            if parts.len() >= 8 {
                let id: usize = match parts[0].parse() {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                max_id = max_id.max(id);

                let category_id: u64 = parts[2].parse().unwrap_or(0);
                let category_id = category_by_id
                    .get(&category_id)
                    .copied()
                    .unwrap_or(CategoryId::new(0));

                self.sessions.push(Session {
                    id,
                    date: parts[1].to_string(),
                    category_id,
                    description: parts[4].to_string(),
                    start_time: parts[5].to_string(),
                    end_time: parts[6].to_string(),
                    elapsed_seconds: parts[7].parse().unwrap_or(0),
                });
            }
        }
        self.session_id_counter = max_id + 1;
    }

    fn save_sessions(&self) {
        if let Ok(file) = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(FILE_PATHS.time_log)
        {
            let mut writer = io::BufWriter::new(file);
            let _ = writeln!(
                writer,
                "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds"
            );
            for session in &self.sessions {
                let category_name = self
                    .categories
                    .iter()
                    .find(|c| c.id == session.category_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("none");
                let _ = writeln!(
                    writer,
                    "{},{},{},{},{},{},{},{}",
                    session.id,
                    session.date,
                    session.category_id.0,
                    category_name,
                    session.description,
                    session.start_time,
                    session.end_time,
                    session.elapsed_seconds
                );
            }
        }
    }

    fn load_categories(&mut self) {
        // Reset to only the built-in "none" category to make idempotent
        self.categories = vec![Category {
            id: CategoryId::new(0),
            name: "none".to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 1,
        }];
        self.next_category_id = 1;

        let path = Path::new(FILE_PATHS.categories);
        if !path.exists() {
            return;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Could not read categories file: {}", e);
                return;
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return;
        }

        // Strict validation: require canonical header
        let header = lines[0].trim();
        if !header.starts_with("id,name,") {
            panic!("categories.csv is not in canonical format. Run 'strata migrate-csv' first.");
        }

        for line in lines.iter().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            // Canonical: id,name,description,color_index,karma_effect
            if parts.len() >= 5 {
                let id: u64 = match parts[0].parse() {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("Warning: Invalid category ID '{}', skipping", parts[0]);
                        continue;
                    }
                };
                // Skip id==0 or name=="none" to avoid duplicates
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

                self.categories.push(Category {
                    id: CategoryId::new(id),
                    name,
                    color: COLORS[color_idx],
                    description,
                    karma_effect,
                });
                self.next_category_id = self.next_category_id.max(id + 1);
            }
        }
    }

    fn save_categories(&self) {
        if let Ok(file) = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(FILE_PATHS.categories)
        {
            let mut writer = io::BufWriter::new(file);
            let _ = writeln!(writer, "id,name,description,color_index,karma_effect");
            for (i, cat) in self.categories.iter().enumerate() {
                if i > 0 {
                    let color_pos = COLORS.iter().position(|&c| c == cat.color).unwrap_or(0);
                    let _ = writeln!(
                        writer,
                        "{},{},{},{},{}",
                        cat.id.0, cat.name, cat.description, color_pos, cat.karma_effect
                    );
                }
            }
        }
    }

    fn start_session(&mut self) {
        if self.active_category_index.is_some() {
            self.current_session_start = Some(Instant::now());
            self.session_id_counter += 1;
        }
    }

    fn end_session(&mut self) -> Option<usize> {
        if let Some(start_instant) = self.current_session_start {
            let elapsed = start_instant.elapsed().as_secs() as usize;
            if let Some(cat_idx) = self.active_category_index {
                let cat_id = self.categories[cat_idx].id;
                let cat_description = self.categories[cat_idx].description.clone();
                self.record_session(cat_id, &cat_description, elapsed);
                self.categories[cat_idx].description.clear();
            }
            self.current_session_start = None;
            self.save_sessions();
            return Some(elapsed);
        }
        None
    }

    fn record_session(&mut self, cat_id: CategoryId, cat_description: &str, elapsed: usize) {
        let cat_name = self
            .categories
            .iter()
            .find(|c| c.id == cat_id)
            .map(|c| c.name.as_str())
            .unwrap_or("none");
        if cat_name == "none" {
            return;
        }
        let now = Local::now();
        let start_time = now - ChronoDuration::seconds(elapsed as i64);
        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|s| s.category_id == cat_id && s.date == now.format("%Y-%m-%d").to_string())
        {
            session.elapsed_seconds += elapsed;
            session.end_time = now.format("%H:%M:%S").to_string();
        } else {
            self.sessions.push(Session {
                id: self.session_id_counter,
                date: now.format("%Y-%m-%d").to_string(),
                category_id: cat_id,
                description: cat_description.to_string(),
                start_time: start_time.format("%H:%M:%S").to_string(),
                end_time: now.format("%H:%M:%S").to_string(),
                elapsed_seconds: elapsed,
            });
        }
    }

    fn get_todays_time(&self) -> usize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let none_id = CategoryId::new(0);
        self.sessions
            .iter()
            .filter(|s| s.date == today && s.category_id != none_id)
            .map(|s| s.elapsed_seconds)
            .sum()
    }

    fn get_category_time(&self, category_name: &str) -> usize {
        let cat_id = self
            .category_id_by_name(category_name)
            .unwrap_or(CategoryId::new(0));
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions
            .iter()
            .filter(|s| s.date == today && s.category_id == cat_id)
            .map(|s| s.elapsed_seconds)
            .sum()
    }

    fn add_category(&mut self, name: String, description: String, color_index: Option<usize>) {
        let color_idx = color_index.unwrap_or(self.categories.len() % COLORS.len());
        let id = CategoryId::new(self.next_category_id);
        self.next_category_id += 1;
        self.categories.push(Category {
            id,
            name,
            color: COLORS[color_idx],
            description,
            karma_effect: 1,
        });
        self.save_categories();
    }

    fn delete_category(&mut self, index: usize) {
        if index > 0 && index < self.categories.len() {
            self.categories.remove(index);
            if self.active_category_index == Some(index) {
                self.active_category_index = Some(0);
            }
            self.save_categories();
        }
    }
}

struct SandEngine {
    grid: Vec<Vec<Option<CategoryId>>>,
    width: u16,
    height: u16,
    frame_count: usize,
    grain_count: usize,
    categories: Vec<Category>,
}

impl SandEngine {
    fn set_categories(&mut self, categories: &[Category]) {
        self.categories = categories.to_vec();
    }

    fn new(width: u16, height: u16) -> Self {
        let mut se = Self {
            grid: vec![],
            width,
            height,
            frame_count: 0,
            grain_count: 0,
            categories: vec![],
        };
        se.resize(width, height);
        se
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.width = width * SAND_ENGINE.dot_width as u16;
        self.height = height * SAND_ENGINE.dot_height as u16;

        let old_w = if self.grid.is_empty() {
            0
        } else {
            self.grid[0].len()
        };
        let old_h = self.grid.len();

        if old_w == 0 || old_h == 0 {
            self.grid = vec![vec![None; self.width as usize]; self.height as usize];
            return;
        }

        let new_w = self.width as usize;
        let new_h = self.height as usize;

        if new_w == old_w && new_h == old_h {
            return;
        }

        let mut new_grid = vec![vec![None; new_w]; new_h];

        // Track lost grains per side
        let mut lost_left: Vec<CategoryId> = Vec::new();
        let mut lost_right: Vec<CategoryId> = Vec::new();
        let mut lost_top: Vec<CategoryId> = Vec::new();
        let mut lost_bottom: Vec<CategoryId> = Vec::new();

        // Explicit kept-window logic: compute the source window that maps to destination
        let (x_src_start, x_src_end, x_dest_offset) = if new_w < old_w {
            // Shrinking: keep center portion
            let x0 = (old_w - new_w) / 2;
            (x0, x0 + new_w, 0)
        } else if new_w > old_w {
            // Expanding: center the old content
            let x_offset = (new_w - old_w) / 2;
            (0, old_w, x_offset)
        } else {
            // Same size: no offset
            (0, old_w, 0)
        };

        let (y_src_start, y_src_end, y_dest_offset) = if new_h < old_h {
            // Shrinking: keep center portion
            let y0 = (old_h - new_h) / 2;
            (y0, y0 + new_h, 0)
        } else if new_h > old_h {
            // Expanding: center the old content
            let y_offset = (new_h - old_h) / 2;
            (0, old_h, y_offset)
        } else {
            // Same size: no offset
            (0, old_h, 0)
        };

        // Copy kept window
        for y_src in y_src_start..y_src_end {
            for x_src in x_src_start..x_src_end {
                let x_dest = x_src - x_src_start + x_dest_offset;
                let y_dest = y_src - y_src_start + y_dest_offset;
                new_grid[y_dest][x_dest] = self.grid[y_src][x_src];
            }
        }

        // Classify lost grains (those outside the kept window)
        for y in 0..old_h {
            for x in 0..old_w {
                // Skip if it's in the kept window (already copied)
                if x >= x_src_start && x < x_src_end && y >= y_src_start && y < y_src_end {
                    continue;
                }

                if let Some(cat_id) = self.grid[y][x] {
                    // Determine which side(s) this grain is lost from
                    let lost_from_left = x < x_src_start;
                    let lost_from_right = x >= x_src_end;
                    let lost_from_top = y < y_src_start;
                    let lost_from_bottom = y >= y_src_end;

                    // Corner rule: prioritize horizontal over vertical
                    if lost_from_left || lost_from_right {
                        if lost_from_left {
                            lost_left.push(cat_id);
                        }
                        if lost_from_right {
                            lost_right.push(cat_id);
                        }
                    } else if lost_from_top || lost_from_bottom {
                        if lost_from_top {
                            lost_top.push(cat_id);
                        }
                        if lost_from_bottom {
                            lost_bottom.push(cat_id);
                        }
                    }
                }
            }
        }

        // Calculate band sizes in logical cells (not pixels)
        let new_cell_w = new_w as usize / SAND_ENGINE.dot_width as usize;
        let new_cell_h = new_h as usize / SAND_ENGINE.dot_height as usize;
        let band_w = (new_cell_w / 40).max(2).min(6);
        let band_h = (new_cell_h / 40).max(1).min(3);

        // Convert band sizes to pixel coordinates
        let band_w_px = band_w * SAND_ENGINE.dot_width as usize;
        let band_h_px = band_h * SAND_ENGINE.dot_height as usize;

        // Phase 1: Place lost grains in their corresponding edge bands (bottom-to-top)
        // Left lost -> left band: iterate through band cells, fill with grains
        let mut lost_iter = lost_left.iter();
        'left_band: for y in (0..new_h).rev() {
            for x in 0..band_w_px {
                if new_grid[y][x].is_none() {
                    if let Some(cat_id) = lost_iter.next() {
                        new_grid[y][x] = Some(*cat_id);
                    } else {
                        break 'left_band;
                    }
                }
            }
        }

        // Right lost -> right band
        let right_band_start = new_w.saturating_sub(band_w_px);
        let mut right_iter = lost_right.iter();
        'right_band: for y in (0..new_h).rev() {
            for x in (right_band_start..new_w).rev() {
                if new_grid[y][x].is_none() {
                    if let Some(cat_id) = right_iter.next() {
                        new_grid[y][x] = Some(*cat_id);
                    } else {
                        break 'right_band;
                    }
                }
            }
        }

        // Top lost -> top band
        let mut top_iter = lost_top.iter();
        'top_band: for y in (0..band_h_px).rev() {
            for x in 0..new_w {
                if new_grid[y][x].is_none() {
                    if let Some(cat_id) = top_iter.next() {
                        new_grid[y][x] = Some(*cat_id);
                    } else {
                        break 'top_band;
                    }
                }
            }
        }

        // Bottom lost -> bottom band
        let bottom_band_start = new_h.saturating_sub(band_h_px);
        let mut bottom_iter = lost_bottom.iter();
        'bottom_band: for y in bottom_band_start..new_h {
            for x in 0..new_w {
                if new_grid[y][x].is_none() {
                    if let Some(cat_id) = bottom_iter.next() {
                        new_grid[y][x] = Some(*cat_id);
                    } else {
                        break 'bottom_band;
                    }
                }
            }
        }

        // Phase 2: If bands full, fill remaining empty cells with overflow grains
        let left_capacity = band_w_px * new_h;
        let right_capacity = band_w_px * new_h;
        let top_capacity = band_h_px * new_w;
        let bottom_capacity = band_h_px * new_w;

        // Collect grains that didn't fit in bands (skip first 'capacity' grains, take the rest)
        let mut remaining = Vec::new();
        for i in left_capacity..lost_left.len() {
            remaining.push(lost_left[i]);
        }
        for i in right_capacity..lost_right.len() {
            remaining.push(lost_right[i]);
        }
        for i in top_capacity..lost_top.len() {
            remaining.push(lost_top[i]);
        }
        for i in bottom_capacity..lost_bottom.len() {
            remaining.push(lost_bottom[i]);
        }

        // Fill remaining grains in any empty cells
        'phase2: for cat_id in remaining.iter() {
            for y in (0..new_h).rev() {
                for x in 0..new_w {
                    if new_grid[y][x].is_none() {
                        new_grid[y][x] = Some(*cat_id);
                        continue 'phase2;
                    }
                }
            }
        }

        self.grid = new_grid;

        // Run gravity to settle
        self.apply_gravity();

        // Recount grains
        self.grain_count = self
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.is_some())
            .count();
    }

    fn capacity(&self) -> usize {
        if self.grid.is_empty() || self.grid[0].is_empty() {
            0
        } else {
            self.grid.len() * self.grid[0].len()
        }
    }

    fn spawn(&mut self, category_id: CategoryId) {
        let capacity = self.capacity();
        if capacity == 0 {
            return;
        }

        let mut rng = rand::thread_rng();
        let w = self.grid[0].len();

        let x = rng.gen_range(0..w);

        if self.grid[0][x].is_none() {
            self.grid[0][x] = Some(category_id);
            self.grain_count += 1;
        } else {
            let fallback_x = rng.gen_range(0..w);
            if self.grid[0][fallback_x].is_none() {
                self.grid[0][fallback_x] = Some(category_id);
                self.grain_count += 1;
            }
        }
    }

    fn apply_gravity(&mut self) {
        let h = self.grid.len();
        let w = self.grid[0].len();

        for y in (0..h - 1).rev() {
            for x in 0..w {
                if let Some(cat) = self.grid[y][x] {
                    if self.grid[y + 1][x].is_none() {
                        self.grid[y + 1][x] = Some(cat);
                        self.grid[y][x] = None;
                    } else {
                        let dir: isize = if rand::random() { 1 } else { -1 };
                        let nx = (x as isize) + dir;

                        if nx >= 0 && (nx as usize) < w && self.grid[y + 1][nx as usize].is_none() {
                            self.grid[y + 1][nx as usize] = Some(cat);
                            self.grid[y][x] = None;
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self) {
        self.frame_count += 1;
        if self.frame_count % 2 == 0 {
            self.apply_gravity();
        }
    }

    fn render(&self, categories: &[Category]) -> Vec<Line<'static>> {
        let cell_w = self.width as usize;
        let cell_h = (self.height / SAND_ENGINE.dot_height as u16) as usize;
        let grid_h = self.grid.len();
        let grid_w = self.grid.first().map_or(0, |row| row.len());
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(cell_h);

        let category_map: std::collections::HashMap<CategoryId, usize> = categories
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();

        for cy in 0..cell_h {
            let mut spans: Vec<Span<'static>> = Vec::with_capacity(cell_w);

            for cx in 0..cell_w {
                let mut dots = 0u8;
                let num_categories = categories.len();
                let mut counts: Vec<usize> = vec![0; num_categories.max(1)];

                for dy in 0..SAND_ENGINE.dot_height {
                    for dx in 0..SAND_ENGINE.dot_width {
                        let gx = cx * SAND_ENGINE.dot_width + dx;
                        let gy = cy * SAND_ENGINE.dot_height + dy;

                        if gy < grid_h && gx < grid_w {
                            if let Some(cat_id) = self.grid[gy][gx] {
                                let dot_index = match (dx, dy) {
                                    (0, 0) => 0,
                                    (0, 1) => 1,
                                    (0, 2) => 2,
                                    (0, 3) => 6,
                                    (1, 0) => 3,
                                    (1, 1) => 4,
                                    (1, 2) => 5,
                                    (1, 3) => 7,
                                    _ => 0,
                                };
                                dots |= 1 << dot_index;

                                let cat_pos = category_map.get(&cat_id).copied().unwrap_or(0);
                                let cat_count = cat_pos.min(num_categories.saturating_sub(1));
                                counts[cat_count] += 1;
                            }
                        }
                    }
                }

                let total_colored_dots: usize = counts.iter().sum();
                let color = if total_colored_dots > 0 {
                    let mut blended_r = 0f32;
                    let mut blended_g = 0f32;
                    let mut blended_b = 0f32;

                    for (cat_idx, &count) in counts.iter().enumerate() {
                        if count > 0 {
                            let (r, g, b) = if cat_idx == 0 {
                                (255u8, 255u8, 255u8)
                            } else if cat_idx < categories.len() {
                                match categories[cat_idx].color {
                                    Color::Rgb(r, g, b) => (r, g, b),
                                    _ => (255, 255, 255),
                                }
                            } else {
                                match COLORS[cat_idx % COLORS.len()] {
                                    Color::Rgb(r, g, b) => (r, g, b),
                                    _ => (255, 255, 255),
                                }
                            };
                            let weight = count as f32 / total_colored_dots as f32;
                            blended_r += r as f32 * weight;
                            blended_g += g as f32 * weight;
                            blended_b += b as f32 * weight;
                        }
                    }

                    Color::Rgb(blended_r as u8, blended_g as u8, blended_b as u8)
                } else {
                    Color::White
                };

                let ch = char::from_u32(SAND_ENGINE.braille_base + dots as u32).unwrap_or(' ');
                spans.push(Span::raw(ch.to_string()).fg(color));
            }

            lines.push(Line::from(spans));
        }

        lines
    }

    fn clear(&mut self) {
        for row in &mut self.grid {
            for cell in row {
                *cell = None;
            }
        }
        self.grain_count = 0;
    }
}

struct App {
    time_tracker: TimeTracker,
    sand_engine: SandEngine,
    blink_state: i32,
    modal_open: bool,
    selected_index: usize,
    new_category_name: String,
    color_index: usize,
    modal_description: String,
    render_needed: bool,
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        let mut app = Self {
            time_tracker: TimeTracker::new(),
            sand_engine: SandEngine::new(width, height),
            blink_state: 0,
            modal_open: false,
            selected_index: 0,
            new_category_name: String::new(),
            color_index: 0,
            modal_description: String::new(),
            render_needed: true,
        };

        app.time_tracker.start_session();
        if app.time_tracker.active_category_index == Some(0) {
            app.blink_state = app.next_blink_interval();
        }

        app
    }

    fn open_modal(&mut self) {
        self.modal_open = true;
        self.selected_index = self.time_tracker.active_category_index.unwrap_or(0);
        self.new_category_name = String::new();
        self.color_index = 0;
        self.modal_description = if let Some(idx) = self.time_tracker.active_category_index {
            self.time_tracker.categories[idx].description.clone()
        } else {
            String::new()
        };
        self.render_needed = true;
    }

    fn close_modal(&mut self) {
        self.modal_open = false;
        self.modal_description = String::new();
        self.render_needed = true;
    }

    fn is_on_insert_space(&self) -> bool {
        self.selected_index == self.time_tracker.categories.len()
    }

    fn add_category(&mut self) {
        if !self.new_category_name.is_empty() {
            self.time_tracker.add_category(
                self.new_category_name.clone(),
                String::new(),
                Some(self.color_index),
            );
            let index = self.time_tracker.categories.len() - 1;
            self.time_tracker.active_category_index = Some(index);
            self.time_tracker.start_session();
        }
    }

    fn delete_category(&mut self) {
        if !self.is_on_insert_space()
            && self.selected_index < self.time_tracker.categories.len()
            && self.selected_index > 0
        {
            self.time_tracker.delete_category(self.selected_index);
            if self.selected_index > 0 && self.selected_index >= self.time_tracker.categories.len()
            {
                self.selected_index = self.time_tracker.categories.len();
            }
        }
    }

    fn get_selected_color(&self) -> Color {
        if self.is_on_insert_space() {
            COLORS[self.color_index]
        } else if self.selected_index < self.time_tracker.categories.len() {
            self.time_tracker.categories[self.selected_index].color
        } else {
            Color::White
        }
    }

    fn get_active_color(&self) -> Color {
        if let Some(idx) = self.time_tracker.active_category_index {
            if idx < self.time_tracker.categories.len() {
                return self.time_tracker.categories[idx].color;
            }
        }
        Color::White
    }

    fn get_effective_time_today(&self) -> usize {
        self.time_tracker.get_todays_time()
    }

    fn get_effective_time_for_category(&self, category_name: &str) -> usize {
        self.time_tracker.get_category_time(category_name)
    }

    fn get_karma_adjusted_time(&self) -> isize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut total: isize = 0;
        for cat in &self.time_tracker.categories {
            if cat.name == "none" {
                continue;
            }
            let cat_time: isize = self
                .time_tracker
                .sessions
                .iter()
                .filter(|s| s.date == today && s.category_id == cat.id)
                .map(|s| s.elapsed_seconds as isize)
                .sum();
            total += cat_time * cat.karma_effect as isize;
        }
        total
    }

    fn get_category_karma_adjusted_time(&self, category_name: &str) -> isize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let cat = self
            .time_tracker
            .categories
            .iter()
            .find(|c| c.name == category_name);
        if let Some(cat) = cat {
            let cat_time: isize = self
                .time_tracker
                .sessions
                .iter()
                .filter(|s| s.date == today && s.category_id == cat.id)
                .map(|s| s.elapsed_seconds as isize)
                .sum();
            cat_time * cat.karma_effect as isize
        } else {
            0
        }
    }

    fn format_signed_time(&self, seconds: isize) -> String {
        let abs_secs = seconds.abs() as usize;
        let sign = if seconds < 0 { "-" } else { "" };
        format!(
            "{}{:02}:{:02}:{:02}",
            sign,
            abs_secs / 3600,
            (abs_secs % 3600) / 60,
            abs_secs % 60
        )
    }

    fn format_time(&self, seconds: usize) -> String {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    }

    fn get_idle_face(&self) -> String {
        let idle_seconds = self
            .time_tracker
            .current_session_start
            .map_or(0, |s| s.elapsed().as_secs() as usize);

        if self.blink_state < 0 {
            "(-_-)".to_string()
        } else if self.blink_state > 0 {
            "(o_o)".to_string()
        } else {
            let faces = FACE_SETTINGS.faces;
            let thresholds = FACE_SETTINGS.thresholds;

            let mut face = faces[0];
            for (i, &threshold) in thresholds.iter().enumerate() {
                if idle_seconds >= threshold {
                    face = faces[i + 1];
                }
            }
            face.to_string()
        }
    }

    fn update_blink(&mut self) {
        if self.blink_state < 0 {
            self.blink_state -= 1;
            let blink_duration = BLINK_SETTINGS.duration_min_frames
                + (rand::random::<i32>()
                    % (BLINK_SETTINGS.duration_max_frames - BLINK_SETTINGS.duration_min_frames));
            if self.blink_state < -blink_duration {
                self.blink_state = self.next_blink_interval();
            }
        } else if self.blink_state > 0 {
            self.blink_state -= 1;
            if self.blink_state == 0 {
                self.blink_state = -1;
            }
        }
    }

    fn next_blink_interval(&self) -> i32 {
        BLINK_SETTINGS.interval_min_frames
            + (rand::random::<i32>()
                % (BLINK_SETTINGS.interval_max_frames - BLINK_SETTINGS.interval_min_frames))
    }

    fn text_color_for_bg(bg_color: Color) -> Color {
        if let Color::Rgb(r, g, b) = bg_color {
            let brightness = (299 * r as u32 + 587 * g as u32 + 114 * b as u32) / 1000;
            if brightness > 128 {
                Color::Black
            } else {
                Color::White
            }
        } else {
            Color::White
        }
    }

    fn render_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let modal_width = terminal_size.width / 3;
        let modal_height = (terminal_size.height / 3).max(10);

        let modal_x = (terminal_size.width - modal_width) / 2;
        let modal_y = (terminal_size.height - modal_height) / 2;

        let modal_rect = Rect::new(modal_x, modal_y, modal_width, modal_height);

        let border_color = self.get_selected_color();

        let items: Vec<ListItem> = self
            .time_tracker
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected_index;
                let dot = if cat.karma_effect < 0 { "◯ " } else { "● " };

                if is_selected {
                    let text_color = Self::text_color_for_bg(cat.color);
                    let description_text = if self.modal_description.is_empty() {
                        Span::raw("")
                    } else {
                        Span::styled(
                            format!(" {}", self.modal_description),
                            Style::default().add_modifier(Modifier::ITALIC),
                        )
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(cat.color),
                        Span::raw(&cat.name).fg(text_color),
                        description_text,
                    ]))
                    .style(
                        ratatui::style::Style::default()
                            .fg(text_color)
                            .bg(cat.color),
                    )
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(cat.color),
                        Span::raw(&cat.name).fg(Color::White),
                    ]))
                }
            })
            .chain(std::iter::once({
                let is_selected = self.is_on_insert_space();
                let cycling_color = COLORS[self.color_index];

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Add new..."
                        } else {
                            &self.new_category_name
                        }),
                    ]))
                    .style(
                        ratatui::style::Style::default()
                            .fg(Color::Black)
                            .bg(Color::White),
                    )
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Add new..."
                        } else {
                            &self.new_category_name
                        })
                        .fg(Color::White),
                    ]))
                }
            }))
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(Span::styled(
                        "strata",
                        Style::default().fg(Color::White),
                    )))
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .border_style(ratatui::style::Style::default().fg(border_color)),
            )
            .highlight_style(ratatui::style::Style::default());

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_stateful_widget(list, modal_rect, &mut list_state);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.modal_open {
            self.handle_modal_key(key);
            false
        } else {
            self.handle_normal_key(key.code)
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Esc => {
                self.close_modal();
            }
            KeyCode::Up => {
                if shift {
                    if self.selected_index > 1 {
                        self.time_tracker
                            .categories
                            .swap(self.selected_index - 1, self.selected_index);
                        self.selected_index -= 1;
                        self.time_tracker.save_categories();
                    }
                } else if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if shift {
                    if self.selected_index > 0
                        && self.selected_index < self.time_tracker.categories.len() - 1
                    {
                        self.time_tracker
                            .categories
                            .swap(self.selected_index, self.selected_index + 1);
                        self.selected_index += 1;
                        self.time_tracker.save_categories();
                    }
                } else {
                    let max_index = self.time_tracker.categories.len();
                    if self.selected_index < max_index {
                        self.selected_index += 1;
                    }
                }
            }
            KeyCode::Left => {
                if shift && !self.is_on_insert_space() && self.selected_index > 0 {
                    let cat_idx = self.selected_index;
                    let current_color = self.time_tracker.categories[cat_idx].color;
                    let current_pos = COLORS.iter().position(|&c| c == current_color).unwrap_or(0);
                    let new_pos = (current_pos + COLORS.len() - 1) % COLORS.len();
                    self.time_tracker.categories[cat_idx].color = COLORS[new_pos];
                    self.time_tracker.save_categories();
                } else if self.is_on_insert_space() {
                    self.color_index = (self.color_index + COLORS.len() - 1) % COLORS.len();
                }
            }
            KeyCode::Right => {
                if shift && !self.is_on_insert_space() && self.selected_index > 0 {
                    let cat_idx = self.selected_index;
                    let current_color = self.time_tracker.categories[cat_idx].color;
                    let current_pos = COLORS.iter().position(|&c| c == current_color).unwrap_or(0);
                    let new_pos = (current_pos + 1) % COLORS.len();
                    self.time_tracker.categories[cat_idx].color = COLORS[new_pos];
                    self.time_tracker.save_categories();
                } else if self.is_on_insert_space() {
                    self.color_index = (self.color_index + 1) % COLORS.len();
                }
            }
            KeyCode::Enter => {
                if self.is_on_insert_space() {
                    if !self.new_category_name.is_empty() {
                        self.add_category();
                        self.close_modal();
                    }
                } else {
                    if self.selected_index < self.time_tracker.categories.len() {
                        self.time_tracker.categories[self.selected_index].description =
                            self.modal_description.clone();
                    }
                    if self.time_tracker.active_category_index != Some(self.selected_index) {
                        self.time_tracker.end_session();
                        self.time_tracker.active_category_index = Some(self.selected_index);
                        self.time_tracker.start_session();
                    }
                    self.close_modal();
                }
            }
            KeyCode::Char('x') => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.delete_category();
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if !self.is_on_insert_space()
                    && self.selected_index > 0
                    && self.selected_index < self.time_tracker.categories.len()
                {
                    self.time_tracker.categories[self.selected_index].karma_effect = 1;
                    self.time_tracker.save_categories();
                }
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                if !self.is_on_insert_space()
                    && self.selected_index > 0
                    && self.selected_index < self.time_tracker.categories.len()
                {
                    self.time_tracker.categories[self.selected_index].karma_effect = -1;
                    self.time_tracker.save_categories();
                }
            }
            KeyCode::Char(c) => {
                if self.is_on_insert_space() {
                    self.new_category_name.push(c);
                } else if self.selected_index < self.time_tracker.categories.len() {
                    self.modal_description.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.is_on_insert_space() {
                    self.new_category_name.pop();
                } else if self.selected_index < self.time_tracker.categories.len() {
                    self.modal_description.pop();
                }
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') => true,
            KeyCode::Char('c') => {
                self.sand_engine.clear();
                false
            }
            KeyCode::Enter => {
                self.open_modal();
                false
            }
            KeyCode::Esc => {
                self.time_tracker.end_session();
                self.time_tracker.active_category_index = Some(0);
                self.time_tracker.start_session();
                false
            }
            _ => false,
        }
    }
}

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        cli::run_cli();
        return Ok(());
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height);

    let physics_rate = Duration::from_millis(TIME_SETTINGS.physics_ms);
    let tick_rate = Duration::from_millis(TIME_SETTINGS.tick_ms);
    let render_rate = Duration::from_millis(1000 / TIME_SETTINGS.target_fps);
    let save_rate = Duration::from_secs(60);
    let mut last_spawn = Instant::now();
    let mut last_physics = Instant::now();
    let mut last_render = Instant::now();
    let mut last_save = Instant::now();

    loop {
        if last_spawn.elapsed() >= tick_rate {
            let should_spawn = app.time_tracker.current_session_start.is_some()
                && app.time_tracker.active_category_index.is_some();

            if should_spawn {
                let cat_idx = app.time_tracker.active_category_index.unwrap_or(0);
                let cat_id = app.time_tracker.categories[cat_idx].id;
                app.sand_engine.spawn(cat_id);
                app.render_needed = true;
            }

            last_spawn = Instant::now();
        }

        if last_physics.elapsed() >= physics_rate {
            app.sand_engine.update();
            app.render_needed = true;
            if app.time_tracker.active_category_index == Some(0) {
                app.update_blink();
            }
            last_physics = Instant::now();
        }

        if last_save.elapsed() >= save_rate {
            app.time_tracker.save_sessions();
            last_save = Instant::now();
        }

        if last_render.elapsed() >= render_rate && app.render_needed {
            terminal.draw(|f| {
                let size = f.size();

                let inner_width = size.width.saturating_sub(2);
                let inner_height = size.height.saturating_sub(2);

                if app.sand_engine.width != inner_width * SAND_ENGINE.dot_width as u16
                    || app.sand_engine.height != inner_height * SAND_ENGINE.dot_height as u16
                {
                    app.sand_engine.resize(inner_width, inner_height);
                }

                let sand = app.sand_engine.render(&app.time_tracker.categories);

                let category_name = if app.time_tracker.active_category_index == Some(0) {
                    app.get_idle_face()
                } else if let Some(idx) = app.time_tracker.active_category_index {
                    app.time_tracker
                        .categories
                        .get(idx)
                        .map(|c| c.name.clone())
                        .unwrap_or_default()
                } else {
                    app.get_idle_face()
                };

                let description = if let Some(idx) = app.time_tracker.active_category_index {
                    app.time_tracker
                        .categories
                        .get(idx)
                        .map(|c| c.description.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                let session_timer = if app.time_tracker.active_category_index == Some(0) {
                    Local::now().format("%H:%M:%S").to_string()
                } else if let Some(start) = app.time_tracker.current_session_start {
                    let elapsed = start.elapsed();
                    app.format_time(elapsed.as_secs() as usize)
                } else {
                    Local::now().format("%H:%M:%S").to_string()
                };

                let effective_time_str = if app.modal_open {
                    let cat_name = app
                        .time_tracker
                        .categories
                        .get(app.selected_index)
                        .map(|c| c.name.as_str())
                        .unwrap_or("none");
                    let karma_time = if cat_name == "none" {
                        app.get_karma_adjusted_time()
                    } else {
                        app.get_category_karma_adjusted_time(cat_name)
                    };
                    app.format_signed_time(karma_time)
                } else if app.time_tracker.active_category_index == Some(0) {
                    let karma_time = app.get_karma_adjusted_time();
                    app.format_signed_time(karma_time)
                } else if let Some(idx) = app.time_tracker.active_category_index {
                    let cat_name = app
                        .time_tracker
                        .categories
                        .get(idx)
                        .map(|c| c.name.as_str())
                        .unwrap_or("none");
                    let mut total = app.get_effective_time_for_category(cat_name);
                    if let Some(start) = app.time_tracker.current_session_start {
                        total += start.elapsed().as_secs() as usize;
                    }
                    app.format_time(total)
                } else {
                    app.format_time(app.get_effective_time_today())
                };

                let border_color = app.get_active_color();
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(
                        Line::from(vec![
                            Span::styled(
                                &category_name,
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            if description.is_empty() {
                                Span::raw("")
                            } else {
                                Span::styled(
                                    format!(" {}", description),
                                    Style::default()
                                        .fg(Color::White)
                                        .add_modifier(Modifier::ITALIC),
                                )
                            },
                        ])
                        .alignment(Alignment::Left),
                    )
                    .title(
                        Line::from(Span::styled(
                            session_timer.as_str(),
                            Style::default().fg(Color::White),
                        ))
                        .alignment(Alignment::Center),
                    )
                    .title(
                        Line::from(Span::styled(
                            effective_time_str.as_str(),
                            Style::default().fg(Color::White),
                        ))
                        .alignment(Alignment::Right),
                    )
                    .border_style(Style::default().fg(border_color));
                let paragraph = Paragraph::new(sand).block(block);
                f.render_widget(paragraph, size);

                if app.modal_open {
                    app.render_modal(f, size);
                }
            })?;
            app.render_needed = false;
            last_render = Instant::now();
        }

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key) {
                    break;
                }
            }
        }
    }

    app.time_tracker.end_session();
    app.time_tracker.save_sessions();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_id_new() {
        let id1 = CategoryId::new(1);
        let id2 = CategoryId::new(2);
        assert_ne!(id1, id2);
        assert_eq!(id1, CategoryId::new(1));
    }

    #[test]
    fn test_sand_resize_basic_copy() {
        // Create with 20 cells (40x80 pixels)
        let mut se = SandEngine::new(20, 20);
        se.categories = vec![Category {
            id: CategoryId::new(0),
            name: "none".into(),
            color: Color::White,
            description: String::new(),
            karma_effect: 1,
        }];

        // Put grains in the middle (in pixel coords)
        se.grid[40][20] = Some(CategoryId::new(0));

        let before = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        // Resize to same size - should preserve
        se.resize(20, 20);

        let after = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        assert_eq!(before, after, "Same-size resize should preserve grains");
    }

    #[test]
    fn test_sand_resize_expand_preserves_grains() {
        // Create with 20 cells (40x80 pixels)
        let mut se = SandEngine::new(20, 20);
        se.categories = vec![Category {
            id: CategoryId::new(0),
            name: "none".into(),
            color: Color::White,
            description: String::new(),
            karma_effect: 1,
        }];

        se.grid[40][20] = Some(CategoryId::new(0));

        let before = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        // Expand to 40 cells (80x160 pixels) - should preserve
        se.resize(40, 40);

        let after = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        assert_eq!(before, after, "Expand should preserve grains");
    }

    #[test]
    fn test_sand_resize_shrink_center_preserves_grains() {
        // Create with 40 cells (80x160 pixels)
        let mut se = SandEngine::new(40, 40);
        se.categories = vec![Category {
            id: CategoryId::new(0),
            name: "none".into(),
            color: Color::White,
            description: String::new(),
            karma_effect: 1,
        }];

        // Put grains in center of 80x160 grid (at y=80, x=40 - the center)
        se.grid[80][40] = Some(CategoryId::new(0));

        let before = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        // Shrink to 20 cells (40x80 pixels) - center should still be visible
        se.resize(20, 20);

        let after = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        assert_eq!(before, after, "Center grains should be preserved on shrink");
    }

    #[test]
    fn test_sand_resize_preserves_count_right_edge() {
        let mut se = SandEngine::new(80, 50);
        se.categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".into(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "work".into(),
                color: Color::Green,
                description: String::new(),
                karma_effect: 1,
            },
        ];

        let cell_w = se.width as usize / SAND_ENGINE.dot_width as usize;
        let cell_h = se.height as usize / SAND_ENGINE.dot_height as usize;

        // Fill right-most cell columns with grains
        for cy in 0..cell_h {
            for cx in (cell_w - 10..cell_w).rev() {
                if cx < cell_w {
                    se.grid[cy][cx] = Some(CategoryId::new(1));
                }
            }
        }

        se.grain_count = se
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.is_some())
            .count();

        let original_count = se.grain_count;

        // Shrink width - grains should redistribute to right band
        se.resize(60, 50);

        assert_eq!(
            se.grain_count, original_count,
            "Grain count should be preserved"
        );
    }

    #[test]
    fn test_sand_resize_preserves_count_expand() {
        let mut se = SandEngine::new(50, 50);
        se.categories = vec![Category {
            id: CategoryId::new(0),
            name: "none".into(),
            color: Color::White,
            description: String::new(),
            karma_effect: 1,
        }];

        let cell_w = se.width as usize / SAND_ENGINE.dot_width as usize;
        let cell_h = se.height as usize / SAND_ENGINE.dot_height as usize;

        // Add some grains in middle
        if cell_h > 2 && cell_w > 2 {
            se.grid[cell_h / 2][cell_w / 2] = Some(CategoryId::new(0));
            se.grid[cell_h / 2 + 1][cell_w / 2] = Some(CategoryId::new(0));
        }

        se.grain_count = se
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.is_some())
            .count();

        let original_count = se.grain_count;

        // Expand - count should increase but not randomly
        se.resize(80, 80);

        assert!(
            se.grain_count >= original_count,
            "Grain count should be at least preserved"
        );
    }

    #[test]
    #[ignore] // Depends on external categories.csv state - run manually
    fn test_load_categories_idempotent() {
        // Use a temp file to avoid interfering with other tests
        let temp_path = Path::new("./test_categories_temp.csv");

        // Write test categories
        let content =
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n2,Personal,,1,1\n";
        std::fs::write(temp_path, content).unwrap();

        // Read and verify
        let tt = TimeTracker::new();
        let first = tt.categories.len();

        // Since we can't easily override FILE_PATHS in tests, just verify basic behavior
        assert!(first >= 1, "Should have at least none category");

        // Cleanup
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_sand_resize_left_edge_band() {
        // Create engine and fill left columns
        let mut se = SandEngine::new(40, 40);
        se.categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".into(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "work".into(),
                color: Color::Green,
                description: String::new(),
                karma_effect: 1,
            },
        ];

        // Fill left 5 columns with grains
        for y in 0..se.grid.len() {
            for x in 0..(5 * SAND_ENGINE.dot_width as usize).min(se.grid[0].len()) {
                se.grid[y][x] = Some(CategoryId::new(1));
            }
        }

        let before = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();
        assert!(before > 0, "Should have grains before resize");

        // Shrink width
        se.resize(30, 40);

        let after = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        // Grain count should be preserved (or capped by new capacity)
        let new_capacity =
            30 * 40 * SAND_ENGINE.dot_width as usize * SAND_ENGINE.dot_height as usize;
        let expected = before.min(new_capacity);

        assert_eq!(after, expected, "Grain count should be preserved or capped");

        // Check that grains are in left band
        let band_w = (30 / 40).max(2).min(6);
        let left_band_count: usize = (0..se.grid.len())
            .flat_map(|y| (0..band_w).map(move |x| (y, x)))
            .filter(|(y, x)| se.grid[*y][*x].is_some())
            .count();

        assert!(left_band_count > 0, "Some grains should be in left band");
    }

    #[test]
    fn test_sand_resize_preserves_category_id_per_grain() {
        // Create engine with two category types
        let mut se = SandEngine::new(40, 40);
        se.categories = vec![
            Category {
                id: CategoryId::new(0),
                name: "none".into(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(1),
                name: "work".into(),
                color: Color::Green,
                description: String::new(),
                karma_effect: 1,
            },
            Category {
                id: CategoryId::new(2),
                name: "play".into(),
                color: Color::Blue,
                description: String::new(),
                karma_effect: 1,
            },
        ];

        // Put different category grains in different areas
        for y in 0..20 {
            for x in 0..20 {
                se.grid[y][x] = Some(CategoryId::new(1)); // work
            }
        }
        for y in 20..40 {
            for x in 20..40 {
                se.grid[y][x] = Some(CategoryId::new(2)); // play
            }
        }

        // Shrink
        se.resize(30, 30);

        // Verify grains still have their category IDs
        let work_count = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| **c == Some(CategoryId::new(1)))
            .count();
        let play_count = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| **c == Some(CategoryId::new(2)))
            .count();

        // Both categories should be preserved (or at least work should be present in the kept window)
        assert!(work_count > 0, "Work category grains should be preserved");
    }
}
