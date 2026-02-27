use std::{
    collections::HashSet,
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};

use crate::{
    constants::{BLINK_SETTINGS, FACE_SETTINGS, TIME_SETTINGS},
    domain::{CategoryId, ReportPeriod, TimeTracker},
    sand::SandEngine,
    storage,
};

mod category_modal_view;
mod category_state;
mod event_handlers;
mod render_views;
mod report_modal_view;
mod report_state;
mod time_format;
mod ui_helpers;
mod view_style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiMode {
    Main,
    CategoryModal,
    KarmaModal,
}

struct App {
    time_tracker: TimeTracker,
    sand_engine: SandEngine,
    blink_state: i32,
    ui_mode: UiMode,
    selected_index: usize,
    new_category_name: String,
    color_index: usize,
    modal_description: String,
    category_tags: storage::CategoryTagsState,
    modal_tag_index: Option<usize>,
    report_selected_index: usize,
    report_period: ReportPeriod,
    report_logs_category_id: Option<CategoryId>,
    report_log_selected_index: usize,
    report_show_help: bool,
    render_needed: bool,
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        let mut tracker = TimeTracker::new();
        let data_dir = storage::get_data_dir();
        let categories_path = data_dir.join("categories.csv");
        let sessions_path = data_dir.join("time_log.csv");

        let loaded_categories = storage::load_categories_from_csv(&categories_path);
        let loaded_sessions =
            storage::load_sessions_from_csv(&sessions_path, &loaded_categories.categories);
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );

        let mut category_tags = storage::load_category_tags(&storage::get_category_tags_path());
        let valid_category_ids: HashSet<u64> = tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id.0)
            .collect();
        category_tags
            .tags_by_category
            .retain(|category_id, _| valid_category_ids.contains(category_id));

        let mut app = Self {
            time_tracker: tracker,
            sand_engine: SandEngine::new(width, height),
            blink_state: 0,
            ui_mode: UiMode::Main,
            selected_index: 0,
            new_category_name: String::new(),
            color_index: 0,
            modal_description: String::new(),
            category_tags,
            modal_tag_index: None,
            report_selected_index: 0,
            report_period: ReportPeriod::Today,
            report_logs_category_id: None,
            report_log_selected_index: 0,
            report_show_help: false,
            render_needed: true,
        };

        app.persist_category_tags();

        app.time_tracker.start_session();
        if app.time_tracker.active_category_index() == Some(0) {
            app.blink_state = app.next_blink_interval();
        }

        app
    }

    fn open_modal(&mut self) {
        self.ui_mode = UiMode::CategoryModal;
        self.selected_index = self.time_tracker.active_category_index().unwrap_or(0);
        self.new_category_name = String::new();
        self.color_index = 0;
        self.sync_modal_description_from_selection();
        self.render_needed = true;
    }

    fn close_modal(&mut self) {
        self.ui_mode = UiMode::Main;
        self.modal_description = String::new();
        self.modal_tag_index = None;
        self.render_needed = true;
    }

    fn open_report_modal(&mut self) {
        self.ui_mode = UiMode::KarmaModal;
        self.report_selected_index = 0;
        self.report_period = ReportPeriod::Today;
        self.report_logs_category_id = None;
        self.report_log_selected_index = 0;
        self.report_show_help = false;
        self.render_needed = true;
    }

    fn close_report_modal(&mut self) {
        self.ui_mode = UiMode::Main;
        self.report_logs_category_id = None;
        self.report_log_selected_index = 0;
        self.report_show_help = false;
        self.render_needed = true;
    }

    fn in_category_modal(&self) -> bool {
        matches!(self.ui_mode, UiMode::CategoryModal)
    }

    fn in_karma_modal(&self) -> bool {
        matches!(self.ui_mode, UiMode::KarmaModal)
    }

    fn modal_rect(&self, terminal_size: Rect) -> Rect {
        self.modal_rect_ratio(terminal_size, 1, 3)
    }

    fn modal_rect_ratio(&self, terminal_size: Rect, numerator: u16, denominator: u16) -> Rect {
        let target_width = terminal_size.width.saturating_mul(numerator) / denominator;
        let target_height = (terminal_size.height.saturating_mul(numerator) / denominator).max(10);

        let max_width = terminal_size.width.saturating_sub(2).max(1);
        let max_height = terminal_size.height.saturating_sub(2).max(1);

        let modal_width = target_width.clamp(1, max_width);
        let modal_height = target_height.clamp(1, max_height);

        let modal_x = (terminal_size.width.saturating_sub(modal_width)) / 2;
        let modal_y = (terminal_size.height.saturating_sub(modal_height)) / 2;

        Rect::new(modal_x, modal_y, modal_width, modal_height)
    }

    fn report_modal_rect(
        &self,
        terminal_size: Rect,
        row_count: usize,
        min_inner_width: usize,
    ) -> Rect {
        let compact = self.modal_rect(terminal_size);
        let inner_width = compact.width.saturating_sub(2) as usize;
        let inner_height = compact.height.saturating_sub(2);
        let footer_height = if self.report_show_help { 1 } else { 0 };
        let visible_rows = inner_height.saturating_sub(footer_height) as usize;

        let breathing_room = 2usize;
        let width_is_cramped = inner_width < min_inner_width.saturating_add(breathing_room);
        let rows_are_cramped = row_count > visible_rows;

        let content_is_cramped = width_is_cramped || rows_are_cramped;
        if content_is_cramped {
            self.modal_rect_ratio(terminal_size, 2, 3)
        } else {
            compact
        }
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
}

pub fn run_ui() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height);
    app.restore_sand_state();

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
                && app.time_tracker.active_category_index().is_some();

            if should_spawn {
                let cat_id = app.time_tracker.active_category_id();
                app.sand_engine.spawn(cat_id);
                app.render_needed = true;
            }

            last_spawn = Instant::now();
        }

        if last_physics.elapsed() >= physics_rate {
            app.sand_engine.update();
            app.render_needed = true;
            if app.time_tracker.active_category_index() == Some(0) {
                app.update_blink();
            }
            last_physics = Instant::now();
        }

        if last_save.elapsed() >= save_rate {
            app.persist_sessions();
            last_save = Instant::now();
        }

        if last_render.elapsed() >= render_rate && app.render_needed {
            terminal.draw(|f| {
                app.draw_frame(f);
            })?;
            app.render_needed = false;
            last_render = Instant::now();
        }

        if event::poll(Duration::from_millis(1))?
            && let Event::Key(key) = event::read()?
            && app.handle_key(key)
        {
            break;
        }
    }

    app.time_tracker.end_session();
    app.persist_sessions();
    app.persist_sand_state();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
