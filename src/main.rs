use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

mod themes;

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

struct Config {
    performance: PerformancePreset,
    theme: String,
    data_dir: String,
    auto_save_interval: u64,
    spawn_location: SpawnLocation,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            performance: PerformancePreset::Medium,
            theme: "Dracula".to_string(),
            data_dir: "~/.local/share/strata".to_string(),
            auto_save_interval: 60,
            spawn_location: SpawnLocation::Random,
        }
    }
}

enum PerformancePreset {
    Low,
    Medium,
    High,
    Ultra,
}

impl PerformancePreset {
    fn physics_ms(&self) -> u64 {
        match self {
            PerformancePreset::Low => 64,
            PerformancePreset::Medium => 32,
            PerformancePreset::High => 16,
            PerformancePreset::Ultra => 8,
        }
    }

    fn gravity_freq(&self) -> usize {
        match self {
            PerformancePreset::Low => 4,
            PerformancePreset::Medium => 2,
            PerformancePreset::High => 1,
            PerformancePreset::Ultra => 1,
        }
    }

    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => PerformancePreset::Low,
            "high" => PerformancePreset::High,
            "ultra" => PerformancePreset::Ultra,
            _ => PerformancePreset::Medium,
        }
    }

    fn to_str(&self) -> &str {
        match self {
            PerformancePreset::Low => "low",
            PerformancePreset::Medium => "medium",
            PerformancePreset::High => "high",
            PerformancePreset::Ultra => "ultra",
        }
    }
}

enum SpawnLocation {
    Random,
    Centered,
}

impl SpawnLocation {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "centered" => SpawnLocation::Centered,
            _ => SpawnLocation::Random,
        }
    }

    fn to_str(&self) -> &str {
        match self {
            SpawnLocation::Random => "random",
            SpawnLocation::Centered => "centered",
        }
    }
}

fn get_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_dir = PathBuf::from(home).join(".config").join("strata");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("strata.config")
}

fn load_config() -> Config {
    let path = get_config_path();
    if !path.exists() {
        return Config::default();
    }

    if let Ok(file) = File::open(&path) {
        let reader = BufReader::new(file);
        let mut config = Config::default();

        for line in reader.lines() {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim();

                    match key {
                        "performance" => config.performance = PerformancePreset::from_str(value),
                        "theme" => config.theme = value.to_string(),
                        "data_dir" => config.data_dir = value.to_string(),
                        "auto_save_interval" => {
                            config.auto_save_interval = value.parse().unwrap_or(60)
                        }
                        "spawn" => config.spawn_location = SpawnLocation::from_str(value),
                        _ => {}
                    }
                }
            }
        }
        return config;
    }

    Config::default()
}

fn save_config(config: &Config) {
    let path = get_config_path();
    if let Ok(file) = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
    {
        let mut writer = io::BufWriter::new(file);
        let _ = writeln!(writer, "performance={}", config.performance.to_str());
        let _ = writeln!(writer, "theme={}", config.theme);
        let _ = writeln!(writer, "data_dir={}", config.data_dir);
        let _ = writeln!(writer, "auto_save_interval={}", config.auto_save_interval);
        let _ = writeln!(writer, "spawn={}", config.spawn_location.to_str());
    }
}

fn get_data_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let data_dir = PathBuf::from(home).join(".local/share/strata");
    fs::create_dir_all(&data_dir).ok();
    data_dir
}

fn get_sessions_path() -> PathBuf {
    get_data_dir().join("active_sessions")
}

fn get_my_pid() -> u32 {
    std::process::id()
}

struct ActiveSession {
    category: String,
    started_at: String,
    pid: u32,
}

fn load_active_sessions() -> Vec<ActiveSession> {
    let path = get_sessions_path();
    if !path.exists() {
        return Vec::new();
    }

    let mut sessions = Vec::new();
    if let Ok(file) = File::open(&path) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 3 {
                    sessions.push(ActiveSession {
                        category: parts[0].to_string(),
                        started_at: parts[1].to_string(),
                        pid: parts[2].parse().unwrap_or(0),
                    });
                }
            }
        }
    }
    sessions
}

fn save_active_sessions(sessions: &[ActiveSession]) {
    let path = get_sessions_path();
    if let Ok(file) = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
    {
        let mut writer = io::BufWriter::new(file);
        for session in sessions {
            let _ = writeln!(
                writer,
                "{}|{}|{}",
                session.category, session.started_at, session.pid
            );
        }
    }
}

fn register_active_category(category: &str) {
    let mut sessions = load_active_sessions();
    let my_pid = get_my_pid();
    let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    sessions.retain(|s| s.pid != my_pid && s.category != category);

    sessions.push(ActiveSession {
        category: category.to_string(),
        started_at: now,
        pid: my_pid,
    });

    save_active_sessions(&sessions);
}

fn unregister_active_category(category: &str) {
    let mut sessions = load_active_sessions();
    let my_pid = get_my_pid();
    sessions.retain(|s| s.pid != my_pid || s.category != category);
    save_active_sessions(&sessions);
}

fn get_other_instance_session(category: &str) -> Option<(String, u32)> {
    let sessions = load_active_sessions();
    for session in sessions {
        if session.category == category && session.pid != get_my_pid() {
            return Some((session.started_at, session.pid));
        }
    }
    None
}

fn check_other_instances_running() -> Vec<String> {
    let sessions = load_active_sessions();
    let mut warnings = Vec::new();
    let my_pid = get_my_pid();

    for session in sessions {
        if session.pid != my_pid {
            let path = PathBuf::from(format!("/proc/{}", session.pid));
            if path.exists() {
                warnings.push(format!(
                    "Category '{}' active in window (PID: {})",
                    session.category, session.pid
                ));
            }
        }
    }
    warnings
}

fn is_my_category_still_mine(category: &str) -> bool {
    let sessions = load_active_sessions();
    let my_pid = get_my_pid();

    for session in sessions {
        if session.category == category && session.pid == my_pid {
            return true;
        }
    }
    false
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

struct Category {
    name: String,
    color: Color,
    description: String,
    karma_effect: i8,
}

struct Session {
    id: usize,
    date: String,
    category: String,
    description: String,
    start_time: String,
    end_time: String,
    elapsed_seconds: usize,
}

struct TimeTracker {
    sessions: Vec<Session>,
    categories: Vec<Category>,
    current_session_start: Option<Instant>,
    session_start_elapsed_offset: Option<Duration>,
    session_id_counter: usize,
    active_category_index: Option<usize>,
}

impl TimeTracker {
    fn new() -> Self {
        let mut tt = Self {
            sessions: Vec::new(),
            categories: vec![Category {
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 1,
            }],
            current_session_start: None,
            session_start_elapsed_offset: None,
            session_id_counter: 0,
            active_category_index: Some(0),
        };
        tt.load_sessions();
        tt.load_categories();
        tt
    }

    fn load_sessions(&mut self) {
        let path = Path::new(FILE_PATHS.time_log);
        if !path.exists() {
            return;
        }
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            let mut max_id = 0;
            for line in reader.lines().skip(1) {
                if let Ok(line) = line {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 7 {
                        if let Ok(id) = parts[0].parse::<usize>() {
                            max_id = max_id.max(id);
                            self.sessions.push(Session {
                                id,
                                date: parts[1].to_string(),
                                category: parts[2].to_string(),
                                description: parts[3].to_string(),
                                start_time: parts[4].to_string(),
                                end_time: parts[5].to_string(),
                                elapsed_seconds: parts[6].parse().unwrap_or(0),
                            });
                        }
                    }
                }
            }
            self.session_id_counter = max_id + 1;
        }
    }

    fn save_sessions(&self) {
        let mut merged_sessions = Vec::new();

        if let Ok(file) = File::open(FILE_PATHS.time_log) {
            let reader = BufReader::new(file);
            for line in reader.lines().skip(1) {
                if let Ok(line) = line {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 7 {
                        if let Ok(id) = parts[0].parse::<usize>() {
                            merged_sessions.push(Session {
                                id,
                                date: parts[1].to_string(),
                                category: parts[2].to_string(),
                                description: parts[3].to_string(),
                                start_time: parts[4].to_string(),
                                end_time: parts[5].to_string(),
                                elapsed_seconds: parts[6].parse().unwrap_or(0),
                            });
                        }
                    }
                }
            }
        }

        let today = Local::now().format("%Y-%m-%d").to_string();

        for session in &self.sessions {
            if let Some(existing) = merged_sessions
                .iter_mut()
                .find(|s| s.id == session.id && s.date == today)
            {
                existing.elapsed_seconds = session.elapsed_seconds;
                existing.end_time = session.end_time.clone();
            } else {
                merged_sessions.push(Session {
                    id: session.id,
                    date: session.date.clone(),
                    category: session.category.clone(),
                    description: session.description.clone(),
                    start_time: session.start_time.clone(),
                    end_time: session.end_time.clone(),
                    elapsed_seconds: session.elapsed_seconds,
                });
            }
        }

        if let Ok(file) = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(FILE_PATHS.time_log)
        {
            let mut writer = io::BufWriter::new(file);
            let _ = writeln!(
                writer,
                "id,date,category,description,start_time,end_time,elapsed_seconds"
            );
            for session in &merged_sessions {
                let _ = writeln!(
                    writer,
                    "{},{},{},{},{},{},{}",
                    session.id,
                    session.date,
                    session.category,
                    session.description,
                    session.start_time,
                    session.end_time,
                    session.elapsed_seconds
                );
            }
        }
    }

    fn load_categories(&mut self) {
        let path = Path::new(FILE_PATHS.categories);
        if !path.exists() {
            return;
        }
        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);
            for line in reader.lines().skip(1) {
                if let Ok(line) = line {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        let name = parts[0].to_string();
                        let description = parts[1].to_string();
                        let color_idx: usize = parts[2].parse().unwrap_or(0) % COLORS.len();
                        let karma_effect: i8 = if parts.len() >= 4 {
                            parts[3].parse().unwrap_or(1)
                        } else {
                            1
                        };
                        self.categories.push(Category {
                            name,
                            color: COLORS[color_idx],
                            description,
                            karma_effect,
                        });
                    }
                }
            }
        }
    }

    fn save_categories(&self) {
        #[derive(Clone)]
        struct FileCategory {
            name: String,
            description: String,
            color_index: usize,
            karma_effect: i8,
        }

        let mut merged_categories: Vec<FileCategory> = Vec::new();

        if let Ok(file) = File::open(FILE_PATHS.categories) {
            let reader = BufReader::new(file);
            for line in reader.lines().skip(1) {
                if let Ok(line) = line {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 4 {
                        let name = parts[0].to_string();
                        let description = parts[1].to_string();
                        let color_index: usize = parts[2].parse().unwrap_or(0);
                        let karma_effect: i8 = parts[3].parse().unwrap_or(1);
                        merged_categories.push(FileCategory {
                            name,
                            description,
                            color_index,
                            karma_effect,
                        });
                    }
                }
            }
        }

        for (i, cat) in self.categories.iter().enumerate() {
            if i > 0 {
                if let Some(existing) = merged_categories.iter_mut().find(|c| c.name == cat.name) {
                    existing.description = cat.description.clone();
                    existing.color_index = COLORS.iter().position(|&c| c == cat.color).unwrap_or(0);
                    existing.karma_effect = cat.karma_effect;
                } else {
                    merged_categories.push(FileCategory {
                        name: cat.name.clone(),
                        description: cat.description.clone(),
                        color_index: COLORS.iter().position(|&c| c == cat.color).unwrap_or(0),
                        karma_effect: cat.karma_effect,
                    });
                }
            }
        }

        if let Ok(file) = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(FILE_PATHS.categories)
        {
            let mut writer = io::BufWriter::new(file);
            let _ = writeln!(writer, "name,description,color_index,karma_effect");
            for cat in &merged_categories {
                let _ = writeln!(
                    writer,
                    "{},{},{},{}",
                    cat.name, cat.description, cat.color_index, cat.karma_effect
                );
            }
        }
    }

    fn start_session(&mut self) {
        if self.active_category_index.is_some() {
            self.current_session_start = Some(Instant::now());
            self.session_start_elapsed_offset = None;
            self.session_id_counter += 1;
        }
    }

    fn start_session_synced(&mut self, elapsed_offset: Duration) {
        if self.active_category_index.is_some() {
            self.current_session_start = Some(Instant::now());
            self.session_start_elapsed_offset = Some(elapsed_offset);
            self.session_id_counter += 1;
        }
    }

    fn get_elapsed(&self) -> Duration {
        if let Some(start_instant) = self.current_session_start {
            let base_elapsed = start_instant.elapsed();
            if let Some(offset) = self.session_start_elapsed_offset {
                return offset + base_elapsed;
            }
            return base_elapsed;
        }
        Duration::ZERO
    }

    fn end_session(&mut self) -> Option<usize> {
        if let Some(start_instant) = self.current_session_start {
            let elapsed = self.get_elapsed().as_secs() as usize;
            if let Some(cat_idx) = self.active_category_index {
                let cat_name = self.categories[cat_idx].name.clone();
                let cat_description = self.categories[cat_idx].description.clone();
                self.record_session(&cat_name, &cat_description, elapsed);
                self.categories[cat_idx].description.clear();
            }
            self.current_session_start = None;
            self.session_start_elapsed_offset = None;
            self.save_sessions();
            return Some(elapsed);
        }
        None
    }

    fn record_session(&mut self, cat_name: &str, cat_description: &str, elapsed: usize) {
        if cat_name == "none" {
            return;
        }
        let now = Local::now();
        let start_time = now - ChronoDuration::seconds(elapsed as i64);
        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|s| s.category == cat_name && s.date == now.format("%Y-%m-%d").to_string())
        {
            session.elapsed_seconds += elapsed;
            session.end_time = now.format("%H:%M:%S").to_string();
        } else {
            self.sessions.push(Session {
                id: self.session_id_counter,
                date: now.format("%Y-%m-%d").to_string(),
                category: cat_name.to_string(),
                description: cat_description.to_string(),
                start_time: start_time.format("%H:%M:%S").to_string(),
                end_time: now.format("%H:%M:%S").to_string(),
                elapsed_seconds: elapsed,
            });
        }
    }

    fn get_todays_time(&self) -> usize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions
            .iter()
            .filter(|s| s.date == today && s.category != "none")
            .map(|s| s.elapsed_seconds)
            .sum()
    }

    fn get_category_time(&self, category_name: &str) -> usize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions
            .iter()
            .filter(|s| s.date == today && s.category == category_name)
            .map(|s| s.elapsed_seconds)
            .sum()
    }

    fn add_category(&mut self, name: String, description: String) {
        let color_idx = (self.categories.len()) % COLORS.len();
        self.categories.push(Category {
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
    grid: Vec<Vec<Option<usize>>>,
    width: u16,
    height: u16,
    frame_count: usize,
    grain_count: usize,
}

impl SandEngine {
    fn new(width: u16, height: u16) -> Self {
        let mut se = Self {
            grid: vec![],
            width,
            height,
            frame_count: 0,
            grain_count: 0,
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

        let mut lost_left: Vec<usize> = vec![0; 12];
        let mut lost_right: Vec<usize> = vec![0; 12];
        let mut lost_top: Vec<usize> = vec![0; 12];
        let mut lost_bottom: Vec<usize> = vec![0; 12];
        let mut kept_count = 0;

        // Horizontal: shift dots toward center
        let x_shift = if new_w > old_w {
            (new_w - old_w) / 2
        } else if new_w < old_w {
            (old_w - new_w) / 2
        } else {
            0
        };

        // Vertical: shift dots toward center
        let y_shift = if new_h > old_h {
            (new_h - old_h) / 2
        } else if new_h < old_h {
            (old_h - new_h) / 2
        } else {
            0
        };

        // Copy dots with symmetric shift
        for y in 0..old_h {
            for x in 0..old_w {
                let dest_x = if new_w >= old_w {
                    x + x_shift
                } else {
                    x.saturating_sub(x_shift)
                };
                let dest_y = if new_h >= old_h {
                    y + y_shift
                } else {
                    y.saturating_sub(y_shift)
                };

                if dest_x < new_w && dest_y < new_h {
                    new_grid[dest_y][dest_x] = self.grid[y][x];
                    if new_grid[dest_y][dest_x].is_some() {
                        kept_count += 1;
                    }
                }
            }
        }

        // Track lost dots from edges (both sides for symmetric squeeze)
        for y in 0..old_h {
            for x in 0..old_w {
                let dest_x = if new_w >= old_w {
                    x + x_shift
                } else {
                    x.saturating_sub(x_shift)
                };
                let dest_y = if new_h >= old_h {
                    y + y_shift
                } else {
                    y.saturating_sub(y_shift)
                };
                let was_copied = dest_x < new_w && dest_y < new_h;

                if !was_copied {
                    if let Some(cat) = self.grid[y][x] {
                        let idx = cat.min(11);
                        // Left edge lost (when shrinking from left)
                        if new_w < old_w && x < x_shift {
                            lost_left[idx] += 1;
                        }
                        // Right edge lost (when shrinking from right)
                        else if new_w < old_w && x >= new_w + x_shift {
                            lost_right[idx] += 1;
                        }
                        // Top edge lost (when shrinking from top)
                        if new_h < old_h && y < y_shift {
                            lost_top[idx] += 1;
                        }
                        // Bottom edge lost (when shrinking from bottom)
                        else if new_h < old_h && y >= new_h + y_shift {
                            lost_bottom[idx] += 1;
                        }
                    }
                }
            }
        }

        self.grid = new_grid;

        let lost_total = lost_left.iter().sum::<usize>()
            + lost_right.iter().sum::<usize>()
            + lost_top.iter().sum::<usize>()
            + lost_bottom.iter().sum::<usize>();

        if lost_total == 0 {
            self.grain_count = kept_count;
            return;
        }

        let new_capacity = new_w * new_h;
        let available_space = new_capacity.saturating_sub(kept_count);
        let to_redistribute = lost_total.min(available_space);

        let mut redistributed = 0;

        // Redistribute lost dots to nearest edges
        for cat_idx in 0..12 {
            if redistributed >= to_redistribute {
                break;
            }

            // Lost from left edge -> redistribute to left edge
            let cat_from_left = lost_left[cat_idx];
            if cat_from_left > 0 {
                let to_place = cat_from_left.min(to_redistribute.saturating_sub(redistributed));
                let mut placed = 0;
                for x in 0..new_w {
                    if placed >= to_place || redistributed >= to_redistribute {
                        break;
                    }
                    for y in 0..new_h {
                        if placed >= to_place || redistributed >= to_redistribute {
                            break;
                        }
                        if self.grid[y][x].is_none() {
                            self.grid[y][x] = Some(cat_idx);
                            placed += 1;
                            redistributed += 1;
                        }
                    }
                }
            }

            if redistributed >= to_redistribute {
                break;
            }

            // Lost from right edge -> redistribute to right edge
            let cat_from_right = lost_right[cat_idx];
            if cat_from_right > 0 {
                let to_place = cat_from_right.min(to_redistribute.saturating_sub(redistributed));
                let mut placed = 0;
                for offset in 0..new_w {
                    if placed >= to_place || redistributed >= to_redistribute {
                        break;
                    }
                    let x = new_w - 1 - offset;
                    for y in 0..new_h {
                        if placed >= to_place || redistributed >= to_redistribute {
                            break;
                        }
                        if self.grid[y][x].is_none() {
                            self.grid[y][x] = Some(cat_idx);
                            placed += 1;
                            redistributed += 1;
                        }
                    }
                }
            }

            if redistributed >= to_redistribute {
                break;
            }

            // Lost from top edge -> redistribute to top edge
            let cat_from_top = lost_top[cat_idx];
            if cat_from_top > 0 {
                let to_place = cat_from_top.min(to_redistribute.saturating_sub(redistributed));
                let mut placed = 0;
                for y in 0..new_h {
                    if placed >= to_place || redistributed >= to_redistribute {
                        break;
                    }
                    for x in 0..new_w {
                        if placed >= to_place || redistributed >= to_redistribute {
                            break;
                        }
                        if self.grid[y][x].is_none() {
                            self.grid[y][x] = Some(cat_idx);
                            placed += 1;
                            redistributed += 1;
                        }
                    }
                }
            }

            if redistributed >= to_redistribute {
                break;
            }

            // Lost from bottom edge -> redistribute to bottom edge
            let cat_from_bottom = lost_bottom[cat_idx];
            if cat_from_bottom > 0 {
                let to_place = cat_from_bottom.min(to_redistribute.saturating_sub(redistributed));
                let mut placed = 0;
                for offset in 0..new_h {
                    if placed >= to_place || redistributed >= to_redistribute {
                        break;
                    }
                    let y = new_h - 1 - offset;
                    for x in 0..new_w {
                        if placed >= to_place || redistributed >= to_redistribute {
                            break;
                        }
                        if self.grid[y][x].is_none() {
                            self.grid[y][x] = Some(cat_idx);
                            placed += 1;
                            redistributed += 1;
                        }
                    }
                }
            }
        }

        self.grain_count = kept_count + redistributed;
    }

    fn capacity(&self) -> usize {
        if self.grid.is_empty() || self.grid[0].is_empty() {
            0
        } else {
            self.grid.len() * self.grid[0].len()
        }
    }

    fn spawn(&mut self, category_idx: usize, centered: bool) {
        let capacity = self.capacity();
        if capacity == 0 {
            return;
        }

        let mut rng = rand::thread_rng();
        let w = self.grid[0].len();

        let x = if centered { w / 2 } else { rng.gen_range(0..w) };

        if self.grid[0][x].is_none() {
            self.grid[0][x] = Some(category_idx);
            self.grain_count += 1;
        } else if centered {
            for offset in 1..w {
                let left = (w / 2).saturating_sub(offset);
                let right = (w / 2) + offset;
                if left < w && self.grid[0][left].is_none() {
                    self.grid[0][left] = Some(category_idx);
                    self.grain_count += 1;
                    return;
                }
                if right < w && self.grid[0][right].is_none() {
                    self.grid[0][right] = Some(category_idx);
                    self.grain_count += 1;
                    return;
                }
            }
        } else {
            let fallback_x = rng.gen_range(0..w);
            if self.grid[0][fallback_x].is_none() {
                self.grid[0][fallback_x] = Some(category_idx);
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

    fn update(&mut self, gravity_freq: usize) {
        self.frame_count += 1;
        if self.frame_count % gravity_freq == 0 {
            self.apply_gravity();
        }
    }

    fn render(&self, category_colors: &[Color]) -> Vec<Line<'static>> {
        let cell_w = self.width as usize;
        let cell_h = (self.height / SAND_ENGINE.dot_height as u16) as usize;
        let grid_h = self.grid.len();
        let grid_w = self.grid.first().map_or(0, |row| row.len());
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(cell_h);

        for cy in 0..cell_h {
            let mut spans: Vec<Span<'static>> = Vec::with_capacity(cell_w);

            for cx in 0..cell_w {
                let mut dots = 0u8;
                let mut best_cat = 0;
                let mut best_count = 0;
                let mut counts: Vec<usize> = vec![0; 12];

                for dy in 0..SAND_ENGINE.dot_height {
                    for dx in 0..SAND_ENGINE.dot_width {
                        let gx = cx * SAND_ENGINE.dot_width + dx;
                        let gy = cy * SAND_ENGINE.dot_height + dy;

                        if gy < grid_h && gx < grid_w {
                            if let Some(cat_idx) = self.grid[gy][gx] {
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

                                let cat_count = cat_idx.min(11);
                                counts[cat_count] += 1;
                                if counts[cat_count] > best_count {
                                    best_count = counts[cat_count];
                                    best_cat = cat_count;
                                }
                            }
                        }
                    }
                }

                let color = if best_count > 0 {
                    if best_cat == 0 {
                        Color::White
                    } else if best_cat < category_colors.len() {
                        category_colors[best_cat]
                    } else {
                        COLORS[best_cat % COLORS.len()]
                    }
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
    settings_open: bool,
    selected_index: usize,
    new_category_name: String,
    color_index: usize,
    modal_description: String,
    config: Config,
    settings_selected_index: usize,
    startup_warnings: Vec<String>,
    category_conflict_warning: Option<String>,
    pending_category_index: Option<usize>,
    render_needed: bool,
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        let config = load_config();
        let startup_warnings = check_other_instances_running();

        let mut app = Self {
            time_tracker: TimeTracker::new(),
            sand_engine: SandEngine::new(width, height),
            blink_state: 0,
            modal_open: false,
            settings_open: false,
            selected_index: 0,
            new_category_name: String::new(),
            color_index: 0,
            modal_description: String::new(),
            config,
            settings_selected_index: 0,
            startup_warnings,
            category_conflict_warning: None,
            pending_category_index: None,
            render_needed: true,
        };

        if !app.startup_warnings.is_empty() {
            app.render_needed = true;
        }

        app.time_tracker.start_session();
        register_active_category("none");
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

    fn select_category(&mut self, index: usize) {
        if self.time_tracker.active_category_index == Some(0) && index > 0 {
            unregister_active_category("none");
        }
        self.time_tracker.end_session();
        self.time_tracker.active_category_index = Some(index);
        if index > 0 {
            let cat_name = self.time_tracker.categories[index].name.clone();
            register_active_category(&cat_name);
        }
        self.time_tracker.start_session();
    }

    fn select_category_synced(&mut self, index: usize, offset: Duration) {
        if self.time_tracker.active_category_index == Some(0) && index > 0 {
            unregister_active_category("none");
        }
        self.time_tracker.end_session();
        self.time_tracker.active_category_index = Some(index);
        if index > 0 {
            let cat_name = self.time_tracker.categories[index].name.clone();
            register_active_category(&cat_name);
        }
        self.time_tracker.start_session_synced(offset);
    }

    fn is_on_insert_space(&self) -> bool {
        self.selected_index == self.time_tracker.categories.len()
    }

    fn add_category(&mut self) {
        if !self.new_category_name.is_empty() {
            self.time_tracker
                .add_category(self.new_category_name.clone(), String::new());
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
        let theme_colors = self.get_theme_colors();
        if self.is_on_insert_space() {
            theme_colors[self.color_index % 12]
        } else if self.selected_index < self.time_tracker.categories.len() {
            theme_colors[self.selected_index % 12]
        } else {
            Color::White
        }
    }

    fn get_active_color(&self) -> Color {
        let theme_colors = self.get_theme_colors();
        if let Some(idx) = self.time_tracker.active_category_index {
            if idx < self.time_tracker.categories.len() {
                return theme_colors[idx % 12];
            }
        }
        Color::White
    }

    fn get_theme_colors(&self) -> [Color; 12] {
        if let Some(theme) = themes::get_theme_by_name(&self.config.theme) {
            theme.colors
        } else {
            COLORS
        }
    }

    fn get_current_theme_index(&self) -> usize {
        themes::THEMES
            .iter()
            .position(|t| t.name.eq_ignore_ascii_case(&self.config.theme))
            .unwrap_or(0)
    }

    fn get_performance_index(&self) -> usize {
        match self.config.performance {
            PerformancePreset::Low => 0,
            PerformancePreset::Medium => 1,
            PerformancePreset::High => 2,
            PerformancePreset::Ultra => 3,
        }
    }

    fn get_spawn_index(&self) -> usize {
        match self.config.spawn_location {
            SpawnLocation::Random => 0,
            SpawnLocation::Centered => 1,
        }
    }

    fn open_settings(&mut self) {
        self.settings_open = true;
        self.settings_selected_index = 0;
        self.render_needed = true;
    }

    fn close_settings(&mut self) {
        self.settings_open = false;
        self.render_needed = true;
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
                .filter(|s| s.date == today && s.category == cat.name)
                .map(|s| s.elapsed_seconds as isize)
                .sum();
            total += cat_time * cat.karma_effect as isize;
        }
        total
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
        let theme_colors = self.get_theme_colors();

        let items: Vec<ListItem> = self
            .time_tracker
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected_index;
                let dot = if cat.karma_effect < 0 { "◯ " } else { "● " };
                let theme_color = theme_colors[i % 12];

                if is_selected {
                    let text_color = Self::text_color_for_bg(theme_color);
                    let description_text = if self.modal_description.is_empty() {
                        Span::raw("")
                    } else {
                        Span::styled(
                            format!(" {}", self.modal_description),
                            Style::default().add_modifier(Modifier::ITALIC),
                        )
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(theme_color),
                        Span::raw(&cat.name).fg(text_color),
                        description_text,
                    ]))
                    .style(
                        ratatui::style::Style::default()
                            .fg(text_color)
                            .bg(theme_color),
                    )
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(theme_color),
                        Span::raw(&cat.name).fg(Color::White),
                    ]))
                }
            })
            .chain(std::iter::once({
                let is_selected = self.is_on_insert_space();
                let cycling_color = theme_colors[self.color_index % 12];

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

    fn render_settings(&self, f: &mut Frame, terminal_size: Rect) {
        let modal_width = (terminal_size.width / 3).max(40);
        let modal_height = (terminal_size.height / 2).max(16);

        let modal_x = (terminal_size.width - modal_width) / 2;
        let modal_y = (terminal_size.height - modal_height) / 2;

        let modal_rect = Rect::new(modal_x, modal_y, modal_width, modal_height);

        let theme_colors = self.get_theme_colors();
        let border_color = theme_colors[1];

        let settings_items = [
            ("Performance", self.config.performance.to_str()),
            ("Theme", self.config.theme.as_str()),
            ("Data Dir", self.config.data_dir.as_str()),
            ("Auto Save", &format!("{}s", self.config.auto_save_interval)),
            ("Spawn", self.config.spawn_location.to_str()),
        ];

        let items: Vec<ListItem> = settings_items
            .iter()
            .enumerate()
            .map(|(i, (label, value))| {
                let is_selected = i == self.settings_selected_index;
                let theme_color = theme_colors[i % 12];

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(theme_color),
                        Span::raw(format!("{}: {}", label, value)).fg(Color::Black),
                    ]))
                    .style(Style::default().bg(theme_color).fg(Color::Black))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(theme_color),
                        Span::raw(format!("{}: {}", label, value)).fg(Color::White),
                    ]))
                }
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.settings_selected_index));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(Span::styled(
                        "settings",
                        Style::default().fg(Color::White),
                    )))
                    .title_alignment(Alignment::Center)
                    .border_style(Style::default().fg(border_color)),
            )
            .highlight_style(Style::default());

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_stateful_widget(list, modal_rect, &mut list_state);
    }

    fn render_warnings(&self, f: &mut Frame, terminal_size: Rect) {
        if self.startup_warnings.is_empty() && self.category_conflict_warning.is_none() {
            return;
        }

        let mut warnings: Vec<String> = Vec::new();

        for warning in &self.startup_warnings {
            warnings.push(warning.clone());
        }

        if let Some(ref cat_warning) = self.category_conflict_warning {
            warnings.push(format!(
                "⚠ Category '{}' is active in another window",
                cat_warning
            ));
            warnings.push("Continue anyway? (y/n)".to_string());
        }

        let warning_text = warnings.join("\n");
        let warning_lines = warning_text.lines().count();

        let warn_width = (terminal_size.width / 2).max(50);
        let warn_height = (warning_lines as u16 + 2).max(6);

        let warn_x = (terminal_size.width - warn_width) / 2;
        let warn_y = terminal_size.height.saturating_sub(warn_height + 2);

        let warn_rect = Rect::new(warn_x, warn_y, warn_width, warn_height);

        let paragraph = Paragraph::new(warning_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(Span::styled(
                        "WARNING",
                        Style::default().fg(Color::Yellow),
                    )))
                    .title_alignment(Alignment::Center)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(ratatui::widgets::Clear, warn_rect);
        f.render_widget(paragraph, warn_rect);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.settings_open {
            self.handle_settings_key(key);
            false
        } else if self.modal_open {
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

                    let new_cat_idx = self.selected_index;
                    if new_cat_idx > 0 {
                        let cat_name = self.time_tracker.categories[new_cat_idx].name.clone();

                        if let Some((started_at, _)) = get_other_instance_session(&cat_name) {
                            self.category_conflict_warning = Some(cat_name);
                            self.pending_category_index = Some(new_cat_idx);
                            self.render_needed = true;
                            return;
                        }
                    }

                    self.select_category(new_cat_idx);
                    self.close_modal();
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(cat_name) = self.category_conflict_warning.clone() {
                    if let Some(idx) = self.pending_category_index {
                        if let Some((started_at, _)) = get_other_instance_session(&cat_name) {
                            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(
                                &started_at,
                                "%Y-%m-%dT%H:%M:%S",
                            ) {
                                let now = Local::now().naive_local();
                                let elapsed = now.signed_duration_since(dt);
                                let offset =
                                    Duration::from_secs(elapsed.num_seconds().max(0) as u64);
                                self.select_category_synced(idx, offset);
                            } else {
                                self.select_category(idx);
                            }
                        } else {
                            self.select_category(idx);
                        }
                    }
                    self.category_conflict_warning = None;
                    self.pending_category_index = None;
                    self.close_modal();
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if self.category_conflict_warning.is_some() {
                    self.category_conflict_warning = None;
                    self.pending_category_index = None;
                    self.render_needed = true;
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

    fn handle_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.close_settings();
            }
            KeyCode::Up => {
                if self.settings_selected_index > 0 {
                    self.settings_selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.settings_selected_index < 4 {
                    self.settings_selected_index += 1;
                }
            }
            KeyCode::Left => {
                match self.settings_selected_index {
                    0 => {
                        self.config.performance = match self.config.performance {
                            PerformancePreset::Low => PerformancePreset::Ultra,
                            PerformancePreset::Medium => PerformancePreset::Low,
                            PerformancePreset::High => PerformancePreset::Medium,
                            PerformancePreset::Ultra => PerformancePreset::High,
                        };
                    }
                    1 => {
                        let idx = self.get_current_theme_index();
                        let new_idx = if idx == 0 {
                            themes::THEMES.len() - 1
                        } else {
                            idx - 1
                        };
                        self.config.theme = themes::THEMES[new_idx].name.to_string();
                    }
                    2 => {}
                    3 => {
                        if self.config.auto_save_interval > 10 {
                            self.config.auto_save_interval -= 10;
                        }
                    }
                    4 => {
                        self.config.spawn_location = match self.config.spawn_location {
                            SpawnLocation::Random => SpawnLocation::Centered,
                            SpawnLocation::Centered => SpawnLocation::Random,
                        };
                    }
                    _ => {}
                }
                save_config(&self.config);
            }
            KeyCode::Right => {
                match self.settings_selected_index {
                    0 => {
                        self.config.performance = match self.config.performance {
                            PerformancePreset::Low => PerformancePreset::Medium,
                            PerformancePreset::Medium => PerformancePreset::High,
                            PerformancePreset::High => PerformancePreset::Ultra,
                            PerformancePreset::Ultra => PerformancePreset::Low,
                        };
                    }
                    1 => {
                        let idx = self.get_current_theme_index();
                        let new_idx = (idx + 1) % themes::THEMES.len();
                        self.config.theme = themes::THEMES[new_idx].name.to_string();
                    }
                    2 => {}
                    3 => {
                        self.config.auto_save_interval += 10;
                    }
                    4 => {
                        self.config.spawn_location = match self.config.spawn_location {
                            SpawnLocation::Random => SpawnLocation::Centered,
                            SpawnLocation::Centered => SpawnLocation::Random,
                        };
                    }
                    _ => {}
                }
                save_config(&self.config);
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
            KeyCode::Char('/') => {
                self.open_settings();
                false
            }
            KeyCode::Enter => {
                self.open_modal();
                false
            }
            KeyCode::Esc => {
                if let Some(idx) = self.time_tracker.active_category_index {
                    if idx > 0 {
                        let cat_name = self.time_tracker.categories[idx].name.clone();
                        unregister_active_category(&cat_name);
                    }
                }
                self.time_tracker.end_session();
                self.time_tracker.active_category_index = Some(0);
                register_active_category("none");
                self.time_tracker.start_session();
                false
            }
            _ => false,
        }
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height);

    let physics_rate = Duration::from_millis(app.config.performance.physics_ms());
    let tick_rate = Duration::from_millis(TIME_SETTINGS.tick_ms);
    let render_rate = Duration::from_millis(1000 / TIME_SETTINGS.target_fps);
    let save_rate = Duration::from_secs(app.config.auto_save_interval);
    let gravity_freq = app.config.performance.gravity_freq();
    let spawn_centered = matches!(app.config.spawn_location, SpawnLocation::Centered);
    let mut last_frame_count: usize = 0;
    let mut last_spawn = Instant::now();
    let mut last_physics = Instant::now();
    let mut last_render = Instant::now();
    let mut last_save = Instant::now();
    let mut last_session_check = Instant::now();
    let session_check_rate = Duration::from_secs(5);

    loop {
        if last_spawn.elapsed() >= tick_rate {
            let should_spawn = app.time_tracker.current_session_start.is_some()
                && app.time_tracker.active_category_index.is_some();

            if should_spawn {
                let cat_idx = app.time_tracker.active_category_index.unwrap_or(0);
                app.sand_engine.spawn(cat_idx, spawn_centered);
                app.render_needed = true;
            }

            last_spawn = Instant::now();
        }

        if last_physics.elapsed() >= physics_rate {
            last_frame_count += 1;
            if last_frame_count % gravity_freq == 0 {
                app.sand_engine.update(gravity_freq);
            }
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

        if last_session_check.elapsed() >= session_check_rate {
            if let Some(idx) = app.time_tracker.active_category_index {
                if idx > 0 {
                    let cat_name = app.time_tracker.categories[idx].name.clone();
                    if !is_my_category_still_mine(&cat_name) {
                        unregister_active_category(&cat_name);
                        app.time_tracker.end_session();
                        app.time_tracker.active_category_index = Some(0);
                        register_active_category("none");
                        app.time_tracker.start_session();
                        app.blink_state = app.next_blink_interval();
                        app.render_needed = true;
                    }
                }
            }
            let warnings = check_other_instances_running();
            if warnings.len() != app.startup_warnings.len() {
                app.startup_warnings = warnings;
                app.render_needed = true;
            }
            last_session_check = Instant::now();
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

                let theme_colors = app.get_theme_colors();
                let category_colors: Vec<Color> = app
                    .time_tracker
                    .categories
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        if c.name == "none" {
                            Color::White
                        } else {
                            theme_colors[i % 12]
                        }
                    })
                    .collect();
                let sand = app.sand_engine.render(&category_colors);

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
                } else if app.time_tracker.current_session_start.is_some() {
                    let elapsed = app.time_tracker.get_elapsed();
                    app.format_time(elapsed.as_secs() as usize)
                } else {
                    Local::now().format("%H:%M:%S").to_string()
                };

                let effective_time_str = if app.time_tracker.active_category_index == Some(0) {
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
                    if app.time_tracker.current_session_start.is_some() {
                        total += app.time_tracker.get_elapsed().as_secs() as usize;
                    }
                    app.format_time(total)
                } else {
                    app.format_time(app.get_effective_time_today())
                };

                let effective_time_str = if app.time_tracker.active_category_index == Some(0) {
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

                if app.settings_open {
                    app.render_settings(f, size);
                }

                app.render_warnings(f, size);
            })?;
            app.render_needed = false;
            last_render = Instant::now();
        }

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }
                if app.handle_key(key) {
                    break;
                }
            } else if let Event::Resize(_, _) = event::read()? {
                // Handle resize
            }
        }
    }

    if let Some(idx) = app.time_tracker.active_category_index {
        if idx > 0 {
            let cat_name = app.time_tracker.categories[idx].name.clone();
            unregister_active_category(&cat_name);
        } else {
            unregister_active_category("none");
        }
    }

    app.time_tracker.end_session();
    app.time_tracker.save_sessions();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
