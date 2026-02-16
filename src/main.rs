use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::Path,
    time::{Duration, Instant},
};

use chrono::{Duration as ChronoDuration, Local};

use crossterm::{
    event::{self, Event, KeyCode},
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

struct TimeSettings {
    tick_ms: u64,
    physics_ms: u64,
}

const TIME_SETTINGS: TimeSettings = TimeSettings {
    tick_ms: 1000,
    physics_ms: 16,
};

const SAND_ENGINE: SandEngineSettings = SandEngineSettings {
    settle_passes: 80,
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

struct SandEngineSettings {
    settle_passes: usize,
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

struct App {
    grid: Vec<Vec<Option<usize>>>,
    width: u16,
    height: u16,
    total_mass: usize,
    overflow: usize,
    categories: Vec<Category>,
    lost_dots: Vec<usize>,
    active_category_index: Option<usize>,
    frame_count: usize,
    blink_state: i32,
    modal_open: bool,
    selected_index: usize,
    new_category_name: String,
    color_index: usize,
    modal_description: String,
    sessions: Vec<Session>,
    current_session_start: Option<Instant>,
    session_id_counter: usize,
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        let mut app = Self {
            grid: vec![],
            width,
            height,
            total_mass: 0,
            overflow: 0,
            categories: vec![Category {
                name: "none".to_string(),
                color: Color::White,
                description: String::new(),
            }],
            lost_dots: vec![0],
            active_category_index: Some(0),
            frame_count: 0,
            blink_state: 0,
            modal_open: false,
            selected_index: 0,
            new_category_name: String::new(),
            color_index: 0,
            modal_description: String::new(),
            sessions: Vec::new(),
            current_session_start: None,
            session_id_counter: 0,
        };

        app.load_sessions();
        app.load_categories();
        app.calculate_todays_mass();
        app.resize(width, height);
        app.start_session();

        app
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
                        let id: usize = parts[6].trim().parse().unwrap_or(0);
                        max_id = max_id.max(id);

                        self.sessions.push(Session {
                            id,
                            date: parts[0].trim().to_string(),
                            category: parts[1].trim().to_string(),
                            description: parts[2].trim().to_string(),
                            start_time: parts[3].trim().to_string(),
                            end_time: parts[4].trim().to_string(),
                            elapsed_seconds: parts[5].trim().parse().unwrap_or(0),
                        });
                    }
                }
            }
            self.session_id_counter = max_id + 1;
        }
    }

    fn save_sessions(&self) {
        let path = Path::new(FILE_PATHS.time_log);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .ok();

        if let Some(ref mut f) = file {
            let _ = writeln!(
                f,
                "Date,Category,Description,Start Time,End Time,Elapsed Time,ID"
            );
            for session in &self.sessions {
                let _ = writeln!(
                    f,
                    "{},{},{},{},{},{},{}",
                    session.date,
                    session.category,
                    session.description,
                    session.start_time,
                    session.end_time,
                    session.elapsed_seconds,
                    session.id
                );
            }
        }
    }

    fn save_categories(&self) {
        let path = Path::new(FILE_PATHS.categories);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .ok();

        if let Some(ref mut f) = file {
            let _ = writeln!(f, "name,r,g,b");
            for cat in &self.categories {
                if let Color::Rgb(r, g, b) = cat.color {
                    let _ = writeln!(f, "{},{},{},{}", cat.name, r, g, b);
                }
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
                    if parts.len() >= 4 {
                        let name = parts[0].trim().to_string();
                        if name != "none" {
                            let r: u8 = parts[1].trim().parse().unwrap_or(0);
                            let g: u8 = parts[2].trim().parse().unwrap_or(0);
                            let b: u8 = parts[3].trim().parse().unwrap_or(0);
                            self.categories.push(Category {
                                name,
                                color: Color::Rgb(r, g, b),
                                description: String::new(),
                            });
                            self.lost_dots.push(0);
                        }
                    }
                }
            }
        }
    }

    fn calculate_todays_mass(&mut self) {
        let today = Local::now().format("%Y-%m-%d").to_string();

        self.total_mass = self
            .sessions
            .iter()
            .filter(|s| s.date == today && s.category != "none")
            .map(|s| s.elapsed_seconds)
            .sum();
    }

    fn start_session(&mut self) {
        if let Some(cat_idx) = self.active_category_index {
            let _cat = &self.categories[cat_idx];
            let _now = Local::now();

            self.current_session_start = Some(Instant::now());
            self.session_id_counter += 1;

            if cat_idx == 0 {
                self.blink_state = self.next_blink_interval();
            }
        }
    }

    fn end_session(&mut self) {
        if let Some(start_instant) = self.current_session_start {
            let elapsed = start_instant.elapsed().as_secs() as usize;

            if let Some(cat_idx) = self.active_category_index {
                let cat_name = self.categories[cat_idx].name.clone();
                let cat_description = self.categories[cat_idx].description.clone();

                self.record_session(&cat_name, &cat_description, elapsed);
                self.categories[cat_idx].description.clear();
                self.calculate_todays_mass();
            }

            self.current_session_start = None;
        }
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
            .find(|s| s.id == self.session_id_counter)
        {
            session.end_time = now.format("%H:%M:%S").to_string();
            session.elapsed_seconds = elapsed;
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

    fn update_current_session(&mut self) {
        if let Some(start_instant) = self.current_session_start {
            if let Some(cat_idx) = self.active_category_index {
                let cat_name = self.categories[cat_idx].name.clone();
                let cat_description = self.categories[cat_idx].description.clone();
                let elapsed = start_instant.elapsed().as_secs() as usize;
                self.record_session(&cat_name, &cat_description, elapsed);
            }
        }
    }

    fn capacity(&self) -> usize {
        self.grid.len() * self.grid[0].len()
    }

    fn resize(&mut self, width: u16, height: u16) {
        let old_w = self.grid.first().map_or(0, |row| row.len());
        let old_h = self.grid.len();

        self.width = width;
        self.height = height * SAND_ENGINE.dot_height as u16;

        let new_w = width as usize * SAND_ENGINE.dot_width;
        let new_h = self.height as usize;

        if old_w == 0 || old_h == 0 {
            self.grid = vec![vec![None; new_w]; new_h];
            self.rebuild_from_mass();
        } else {
            let mut new_grid = vec![vec![None; new_w]; new_h];
            let mut particles_kept = 0;

            for y in 0..old_h.min(new_h) {
                for x in 0..old_w.min(new_w) {
                    new_grid[y][x] = self.grid[y][x];
                    if self.grid[y][x].is_some() {
                        particles_kept += 1;
                    }
                }
            }

            let lost_particles = self
                .total_mass
                .saturating_sub(self.overflow)
                .saturating_sub(particles_kept);
            self.overflow = self.overflow.saturating_add(lost_particles);

            self.grid = new_grid;
        }
    }

    fn rebuild_from_mass(&mut self) {
        let capacity = self.capacity();

        let visible = self.total_mass.min(capacity);
        self.overflow = self.total_mass.saturating_sub(capacity);

        for row in &mut self.grid {
            for cell in row {
                *cell = None;
            }
        }

        let mut grains = visible;
        let h = self.grid.len();
        let w = self.grid[0].len();

        for y in (0..h).rev() {
            for x in 0..w {
                if grains > 0 {
                    self.grid[y][x] = Some(0);
                    grains -= 1;
                }
            }
        }

        self.settle(SAND_ENGINE.settle_passes);
    }

    fn spawn_one(&mut self) {
        let capacity = self.capacity();

        if self.total_mass > capacity {
            self.overflow += 1;
            return;
        }

        let mut rng = rand::thread_rng();
        let w = self.grid[0].len();

        let category_idx = self.active_category_index.unwrap_or(0);

        let x = rng.gen_range(0..w);

        if x < w && self.grid[0][x].is_none() {
            self.grid[0][x] = Some(category_idx);
        } else {
            let fallback_x = rng.gen_range(0..w);
            if self.grid[0][fallback_x].is_none() {
                self.grid[0][fallback_x] = Some(category_idx);
            } else {
                self.overflow += 1;
            }
        }
    }

    fn apply_gravity(&mut self) {
        let h = self.grid.len();
        let w = self.grid[0].len();

        for y in (0..h - 1).rev() {
            for x in 0..w {
                if let Some(_) = self.grid[y][x] {
                    if self.grid[y + 1][x].is_none() {
                        self.grid[y + 1][x] = self.grid[y][x];
                        self.grid[y][x] = None;
                    } else {
                        let dir = if rand::random() { 1 } else { -1 };
                        let nx = x as isize + dir;

                        if nx >= 0 && nx < w as isize && self.grid[y + 1][nx as usize].is_none() {
                            if let Some(existing_idx) = self.grid[y + 1][nx as usize] {
                                self.lost_dots[existing_idx] += 1;
                            }
                            self.grid[y + 1][nx as usize] = self.grid[y][x];
                            self.grid[y][x] = None;
                        }
                    }
                }
            }
        }
    }

    fn settle(&mut self, passes: usize) {
        for _ in 0..passes {
            self.apply_gravity();
        }
    }

    fn render(&self) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        let cell_w = self.width as usize;
        let cell_h = (self.height / SAND_ENGINE.dot_height as u16) as usize;

        for cy in 0..cell_h {
            let mut spans: Vec<Span<'static>> = Vec::new();

            for cx in 0..cell_w {
                let mut dots = 0u8;
                let mut cat_counts: Vec<(usize, usize)> = Vec::new();

                for dy in 0..SAND_ENGINE.dot_height {
                    for dx in 0..SAND_ENGINE.dot_width {
                        let gx = cx * SAND_ENGINE.dot_width + dx;
                        let gy = cy * SAND_ENGINE.dot_height + dy;

                        if gy < self.grid.len() && gx < self.grid[0].len() {
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

                                if let Some(pos) =
                                    cat_counts.iter().position(|(c, _)| *c == cat_idx)
                                {
                                    cat_counts[pos].1 += 1;
                                } else {
                                    cat_counts.push((cat_idx, 1));
                                }
                            }
                        }
                    }
                }

                let color = if let Some(&(dominant_cat, _)) =
                    cat_counts.iter().max_by_key(|&&(_, count)| count)
                {
                    self.get_category_color(dominant_cat)
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

    fn open_modal(&mut self) {
        self.modal_open = true;
        self.selected_index = self.active_category_index.unwrap_or(0);
        self.new_category_name = String::new();
        self.color_index = 0;
        self.modal_description = if let Some(idx) = self.active_category_index {
            self.categories[idx].description.clone()
        } else {
            String::new()
        };
    }

    fn close_modal(&mut self) {
        self.modal_open = false;
        self.modal_description = String::new();
    }

    fn is_on_insert_space(&self) -> bool {
        self.selected_index == self.categories.len()
    }

    fn add_category(&mut self) {
        if !self.new_category_name.is_empty() {
            let color = COLORS[self.color_index];
            let index = self.categories.len();
            self.categories.push(Category {
                name: self.new_category_name.clone(),
                color,
                description: String::new(),
            });
            self.lost_dots.push(0);
            self.save_categories();
            self.active_category_index = Some(index);
            self.start_session();
        }
    }

    fn delete_category(&mut self) {
        if !self.is_on_insert_space()
            && self.selected_index < self.categories.len()
            && self.selected_index > 0
        {
            self.categories.remove(self.selected_index);
            self.save_categories();
            if self.selected_index > 0 && self.selected_index >= self.categories.len() {
                self.selected_index = self.categories.len();
            }
        }
    }

    fn get_selected_color(&self) -> Color {
        if self.is_on_insert_space() {
            COLORS[self.color_index]
        } else if self.selected_index < self.categories.len() {
            self.categories[self.selected_index].color
        } else {
            Color::White
        }
    }

    fn get_active_color(&self) -> Color {
        if let Some(idx) = self.active_category_index {
            if idx < self.categories.len() {
                return self.categories[idx].color;
            }
        }
        Color::White
    }

    fn get_effective_time_today(&self) -> usize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions
            .iter()
            .filter(|s| s.date == today && s.category != "none")
            .map(|s| s.elapsed_seconds)
            .sum()
    }

    fn get_effective_time_for_category(&self, category_name: &str) -> usize {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.sessions
            .iter()
            .filter(|s| s.date == today && s.category == category_name)
            .map(|s| s.elapsed_seconds)
            .sum()
    }

    fn format_time(&self, seconds: usize) -> String {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    }

    fn get_category_color(&self, idx: usize) -> Color {
        if idx < self.categories.len() {
            self.categories[idx].color
        } else {
            Color::White
        }
    }

    fn get_idle_face(&self) -> String {
        let idle_seconds = self
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
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected_index;

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
                        Span::raw("● ").fg(cat.color),
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
                        Span::raw("● ").fg(cat.color),
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
                    .title("strata")
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .border_style(ratatui::style::Style::default().fg(border_color)),
            )
            .highlight_style(ratatui::style::Style::default());

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_stateful_widget(list, modal_rect, &mut list_state);
    }

    fn handle_key(&mut self, key: KeyCode) -> bool {
        if self.modal_open {
            self.handle_modal_key(key);
            false
        } else {
            self.handle_normal_key(key)
        }
    }

    fn handle_modal_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.close_modal();
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                let max_index = self.categories.len();
                if self.selected_index < max_index {
                    self.selected_index += 1;
                }
            }
            KeyCode::Left => {
                if self.is_on_insert_space() {
                    self.color_index = (self.color_index + COLORS.len() - 1) % COLORS.len();
                }
            }
            KeyCode::Right => {
                if self.is_on_insert_space() {
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
                    if self.selected_index < self.categories.len() {
                        self.categories[self.selected_index].description =
                            self.modal_description.clone();
                    }
                    if self.active_category_index != Some(self.selected_index) {
                        self.end_session();
                        self.active_category_index = Some(self.selected_index);
                        self.start_session();
                    }
                    self.close_modal();
                }
            }
            KeyCode::Char('x') => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.delete_category();
                }
            }
            KeyCode::Char(c) => {
                if self.is_on_insert_space() {
                    self.new_category_name.push(c);
                } else if self.selected_index < self.categories.len() {
                    self.modal_description.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.is_on_insert_space() {
                    self.new_category_name.pop();
                } else if self.selected_index < self.categories.len() {
                    self.modal_description.pop();
                }
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') => true,
            KeyCode::Enter => {
                self.open_modal();
                false
            }
            KeyCode::Esc => {
                self.end_session();
                self.active_category_index = Some(0);
                self.start_session();
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

    let physics_rate = Duration::from_millis(TIME_SETTINGS.physics_ms);
    let tick_rate = Duration::from_millis(TIME_SETTINGS.tick_ms);
    let mut last_spawn = Instant::now();
    let mut last_physics = Instant::now();

    loop {
        if last_spawn.elapsed() >= tick_rate {
            let should_spawn =
                app.current_session_start.is_some() && app.active_category_index.is_some();

            if should_spawn {
                app.spawn_one();
            }

            app.update_current_session();
            app.save_sessions();
            last_spawn = Instant::now();
        }

        if last_physics.elapsed() >= physics_rate {
            app.apply_gravity();
            app.apply_gravity();
            app.frame_count += 1;
            if app.active_category_index == Some(0) {
                app.update_blink();
            }
            last_physics = Instant::now();
        }

        terminal.draw(|f| {
            let size = f.size();

            let inner_width = size.width.saturating_sub(2);
            let inner_height = size.height.saturating_sub(2);

            if app.width != inner_width || app.height / 4 != inner_height {
                app.resize(inner_width, inner_height);
            }

            let sand = app.render();

            let category_name = if app.active_category_index == Some(0) {
                app.get_idle_face()
            } else if let Some(idx) = app.active_category_index {
                app.categories
                    .get(idx)
                    .map(|c| c.name.clone())
                    .unwrap_or_default()
            } else {
                app.get_idle_face()
            };

            let description = if let Some(idx) = app.active_category_index {
                app.categories
                    .get(idx)
                    .map(|c| c.description.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let session_timer = if app.active_category_index == Some(0) {
                Local::now().format("%H:%M:%S").to_string()
            } else if let Some(start) = app.current_session_start {
                let elapsed = start.elapsed();
                app.format_time(elapsed.as_secs() as usize)
            } else {
                Local::now().format("%H:%M:%S").to_string()
            };

            let effective_time = if app.active_category_index == Some(0) {
                app.get_effective_time_today()
            } else if let Some(idx) = app.active_category_index {
                let cat_name = app
                    .categories
                    .get(idx)
                    .map(|c| c.name.as_str())
                    .unwrap_or("none");
                app.get_effective_time_for_category(cat_name)
            } else {
                app.get_effective_time_today()
            };
            let effective_time_str = app.format_time(effective_time);

            let border_color = app.get_active_color();
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(
                    Line::from(vec![
                        Span::styled(
                            &category_name,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        if description.is_empty() {
                            Span::raw("")
                        } else {
                            Span::styled(
                                format!(" {}", description),
                                Style::default().add_modifier(Modifier::ITALIC),
                            )
                        },
                    ])
                    .alignment(Alignment::Left),
                )
                .title(Line::from(session_timer.as_str()).alignment(Alignment::Center))
                .title(Line::from(effective_time_str.as_str()).alignment(Alignment::Right))
                .border_style(Style::default().fg(border_color));
            let paragraph = Paragraph::new(sand).block(block);
            f.render_widget(paragraph, size);

            if app.modal_open {
                app.render_modal(f, size);
            }
        })?;

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key.code) {
                    break;
                }
            }
        }
    }

    app.end_session();
    app.save_sessions();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
