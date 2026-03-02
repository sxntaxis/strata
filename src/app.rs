use std::{
    collections::HashSet,
    io,
    time::{Duration, Instant, SystemTime},
};

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};

use crate::{
    constants::{BLINK_SETTINGS, FACE_SETTINGS, TIME_SETTINGS},
    domain::{CategoryId, ReportPeriod, RuntimeSettings, TimeTracker, set_runtime_settings},
    keybindings,
    sand::SandEngine,
    storage,
};

mod category_modal_view;
mod category_state;
mod command_palette_view;
mod event_handlers;
mod keybindings_modal_view;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AtlasSelectable {
    TimeLogPath,
    DayStartMode,
    WeekStartDay,
    Action(keybindings::Action),
}

#[derive(Clone, Debug)]
enum AtlasOverlay {
    CaptureKey { action: keybindings::Action },
    EditTimeLogPath { input: String },
    SelectDayStartMode { selected: usize },
    SelectWeekStartDay { selected: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PaletteCommand {
    Action(keybindings::Action),
    SetReportPeriod(ReportPeriod),
    SwitchLayer(CategoryId),
}

#[derive(Clone, Debug)]
struct PaletteEntry {
    command: PaletteCommand,
    title: String,
    search_text: String,
    hint: String,
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
    report_period_offset: usize,
    report_logs_category_id: Option<CategoryId>,
    report_log_selected_index: usize,
    report_snapshot_end_day: Option<String>,
    report_snapshot_state: Option<crate::sand::SandState>,
    report_snapshot_preview_key: Option<String>,
    report_snapshot_preview_engine: Option<SandEngine>,
    keymap: keybindings::Keymap,
    runtime_settings: RuntimeSettings,
    keymap_error: Option<String>,
    show_command_palette: bool,
    command_palette_query: String,
    command_palette_selected_index: usize,
    command_palette_scroll: usize,
    show_keybindings_modal: bool,
    keybindings_scroll: usize,
    atlas_selected_index: usize,
    atlas_overlay: Option<AtlasOverlay>,
    keymap_last_modified: Option<SystemTime>,
    keymap_last_poll: Instant,
    render_needed: bool,
    none_entry_time: Option<Instant>,
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        let keymap_path = storage::get_keymap_path();
        let keymap_last_modified = std::fs::metadata(&keymap_path)
            .and_then(|metadata| metadata.modified())
            .ok();
        let (keymap, runtime_settings, loaded_time_log_path, keymap_error) =
            match keybindings::load_keybindings(&keymap_path) {
                Ok(loaded) => (
                    loaded.keymap,
                    loaded.runtime_settings,
                    loaded.time_log_path,
                    None,
                ),
                Err(err) => (
                    keybindings::default_keymap(),
                    keybindings::default_runtime_settings(),
                    None,
                    Some(err),
                ),
            };

        set_runtime_settings(runtime_settings);
        storage::set_runtime_storage_settings(storage::RuntimeStorageSettings {
            time_log_path: loaded_time_log_path.clone(),
        });

        let mut tracker = TimeTracker::new();
        let categories_path = storage::get_categories_path();
        let sessions_path = storage::get_time_log_path();

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
            report_period_offset: 0,
            report_logs_category_id: None,
            report_log_selected_index: 0,
            report_snapshot_end_day: None,
            report_snapshot_state: None,
            report_snapshot_preview_key: None,
            report_snapshot_preview_engine: None,
            keymap,
            runtime_settings,
            keymap_error,
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_selected_index: 0,
            command_palette_scroll: 0,
            show_keybindings_modal: false,
            keybindings_scroll: 0,
            atlas_selected_index: 0,
            atlas_overlay: None,
            keymap_last_modified,
            keymap_last_poll: Instant::now(),
            render_needed: true,
            none_entry_time: None,
        };

        app.persist_category_tags();

        app.time_tracker.start_session();
        if app.time_tracker.active_category_index() == Some(0) {
            app.blink_state = app.next_blink_interval();
            app.none_entry_time = Some(Instant::now());
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
        self.report_period_offset = 0;
        self.report_logs_category_id = None;
        self.report_log_selected_index = 0;
        self.report_snapshot_end_day = None;
        self.report_snapshot_state = None;
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_engine = None;
        self.focus_none_report_row();
        self.render_needed = true;
    }

    fn close_report_modal(&mut self) {
        self.ui_mode = UiMode::Main;
        self.report_logs_category_id = None;
        self.report_log_selected_index = 0;
        self.report_snapshot_end_day = None;
        self.report_snapshot_state = None;
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_engine = None;
        self.render_needed = true;
    }

    fn in_category_modal(&self) -> bool {
        matches!(self.ui_mode, UiMode::CategoryModal)
    }

    fn in_karma_modal(&self) -> bool {
        matches!(self.ui_mode, UiMode::KarmaModal)
    }

    fn is_drift_name(name: &str) -> bool {
        name.eq_ignore_ascii_case("none") || name.eq_ignore_ascii_case("drift")
    }

    fn display_layer_name(&self, name: &str) -> String {
        if Self::is_drift_name(name) {
            "drift".to_string()
        } else {
            name.to_string()
        }
    }

    fn atlas_items(&self) -> Vec<AtlasSelectable> {
        let mut items = vec![
            AtlasSelectable::TimeLogPath,
            AtlasSelectable::DayStartMode,
            AtlasSelectable::WeekStartDay,
        ];
        items.extend(
            keybindings::Action::all()
                .iter()
                .copied()
                .map(AtlasSelectable::Action),
        );
        items
    }

    fn selected_atlas_item(&self) -> AtlasSelectable {
        let items = self.atlas_items();
        items
            .get(self.atlas_selected_index)
            .copied()
            .unwrap_or(AtlasSelectable::Action(keybindings::Action::all()[0]))
    }

    fn total_atlas_items(&self) -> usize {
        self.atlas_items().len()
    }

    fn effective_keys_for_action(
        &self,
        action: keybindings::Action,
    ) -> Vec<keybindings::KeyBinding> {
        let direct = self.keymap.keys_for_action(action);
        if !direct.is_empty() {
            return direct;
        }

        match action {
            keybindings::Action::OpenCategoryModal => {
                self.keymap.keys_for_action(keybindings::Action::Confirm)
            }
            keybindings::Action::SwitchToNone => {
                self.keymap.keys_for_action(keybindings::Action::Cancel)
            }
            _ => direct,
        }
    }

    fn atlas_item_description(&self, item: AtlasSelectable) -> String {
        match item {
            AtlasSelectable::TimeLogPath => {
                "Path where session rows are written (time_log.csv).".to_string()
            }
            AtlasSelectable::DayStartMode => {
                "Operational day boundary mode used for day rollover.".to_string()
            }
            AtlasSelectable::WeekStartDay => {
                "First weekday used by Week range in Karma pop-up.".to_string()
            }
            AtlasSelectable::Action(action) => action.description().to_string(),
        }
    }

    fn atlas_item_color(&self, item: AtlasSelectable) -> ratatui::style::Color {
        use ratatui::style::Color;

        match item {
            AtlasSelectable::TimeLogPath => Color::Cyan,
            AtlasSelectable::DayStartMode => Color::Yellow,
            AtlasSelectable::WeekStartDay => Color::Green,
            AtlasSelectable::Action(action) => match action.category() {
                keybindings::ActionCategory::Global => Color::Cyan,
                keybindings::ActionCategory::Navigation => Color::Yellow,
                keybindings::ActionCategory::CategoryModal => Color::Green,
                keybindings::ActionCategory::ReportModal => Color::Magenta,
                keybindings::ActionCategory::HelpModal => Color::Blue,
            },
        }
    }

    fn day_start_mode_options() -> [crate::domain::DayBoundaryMode; 2] {
        [
            crate::domain::DayBoundaryMode::FixedHour,
            crate::domain::DayBoundaryMode::Sunrise,
        ]
    }

    fn week_start_options() -> [crate::domain::FirstDayOfWeek; 7] {
        [
            crate::domain::FirstDayOfWeek::Monday,
            crate::domain::FirstDayOfWeek::Tuesday,
            crate::domain::FirstDayOfWeek::Wednesday,
            crate::domain::FirstDayOfWeek::Thursday,
            crate::domain::FirstDayOfWeek::Friday,
            crate::domain::FirstDayOfWeek::Saturday,
            crate::domain::FirstDayOfWeek::Sunday,
        ]
    }

    fn day_start_setting_label(&self) -> String {
        let boundary = self.runtime_settings.day_boundary;
        let mode = match boundary.mode {
            crate::domain::DayBoundaryMode::FixedHour => "fixed",
            crate::domain::DayBoundaryMode::Sunrise => "sunrise",
        };

        let sign = if boundary.utc_offset_seconds < 0 {
            "-"
        } else {
            "+"
        };
        let abs_offset = boundary.utc_offset_seconds.unsigned_abs() as u32;
        let offset_hours = abs_offset / 3600;
        let offset_minutes = (abs_offset % 3600) / 60;

        format!(
            "{} {:02}:{:02} (UTC{}{:02}:{:02})",
            mode, boundary.fixed_hour, boundary.fixed_minute, sign, offset_hours, offset_minutes
        )
    }

    fn first_day_of_week_label(&self) -> String {
        let raw = self.runtime_settings.first_day_of_week.as_config_name();
        let mut chars = raw.chars();
        let Some(first) = chars.next() else {
            return "Monday".to_string();
        };

        format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
    }

    fn toggle_keybindings_modal(&mut self) {
        self.show_keybindings_modal = !self.show_keybindings_modal;
        if self.show_keybindings_modal {
            self.atlas_selected_index = 0;
            self.keybindings_scroll = 0;
            self.atlas_overlay = None;
            self.close_command_palette();
        }
        self.render_needed = true;
    }

    fn close_keybindings_modal(&mut self) {
        self.show_keybindings_modal = false;
        self.atlas_overlay = None;
        self.render_needed = true;
    }

    fn toggle_command_palette(&mut self) {
        self.show_command_palette = !self.show_command_palette;
        if self.show_command_palette {
            self.command_palette_query.clear();
            self.command_palette_selected_index = 0;
            self.command_palette_scroll = 0;
            self.close_keybindings_modal();
        }
        self.render_needed = true;
    }

    fn close_command_palette(&mut self) {
        self.show_command_palette = false;
        self.command_palette_query.clear();
        self.command_palette_selected_index = 0;
        self.command_palette_scroll = 0;
        self.render_needed = true;
    }

    fn clamp_command_palette_selection(&mut self, row_count: usize) {
        if row_count == 0 {
            self.command_palette_selected_index = 0;
        } else if self.command_palette_selected_index >= row_count {
            self.command_palette_selected_index = row_count - 1;
        }
    }

    fn select_previous_keybinding_action(&mut self) {
        let total = self.total_atlas_items();
        if total == 0 {
            return;
        }
        self.atlas_selected_index = (self.atlas_selected_index + total - 1) % total;
        self.render_needed = true;
    }

    fn select_next_keybinding_action(&mut self) {
        let total = self.total_atlas_items();
        if total == 0 {
            return;
        }
        self.atlas_selected_index = (self.atlas_selected_index + 1) % total;
        self.render_needed = true;
    }

    fn jump_keybindings_top(&mut self) {
        self.atlas_selected_index = 0;
        self.keybindings_scroll = 0;
        self.render_needed = true;
    }

    fn jump_keybindings_bottom(&mut self) {
        let total = self.total_atlas_items();
        if total == 0 {
            return;
        }
        self.atlas_selected_index = total - 1;
        self.render_needed = true;
    }

    fn open_atlas_editor_for_selection(&mut self) {
        match self.selected_atlas_item() {
            AtlasSelectable::Action(action) => {
                self.atlas_overlay = Some(AtlasOverlay::CaptureKey { action });
            }
            AtlasSelectable::TimeLogPath => {
                self.atlas_overlay = Some(AtlasOverlay::EditTimeLogPath {
                    input: storage::get_time_log_path().display().to_string(),
                });
            }
            AtlasSelectable::DayStartMode => {
                let selected = Self::day_start_mode_options()
                    .iter()
                    .position(|mode| *mode == self.runtime_settings.day_boundary.mode)
                    .unwrap_or(0);
                self.atlas_overlay = Some(AtlasOverlay::SelectDayStartMode { selected });
            }
            AtlasSelectable::WeekStartDay => {
                let selected = Self::week_start_options()
                    .iter()
                    .position(|day| *day == self.runtime_settings.first_day_of_week)
                    .unwrap_or(0);
                self.atlas_overlay = Some(AtlasOverlay::SelectWeekStartDay { selected });
            }
        }

        self.render_needed = true;
    }

    fn close_atlas_overlay(&mut self) {
        self.atlas_overlay = None;
        self.render_needed = true;
    }

    fn apply_loaded_keybindings(&mut self, loaded: keybindings::LoadedKeybindings) {
        self.keymap = loaded.keymap;
        self.runtime_settings = loaded.runtime_settings;
        set_runtime_settings(self.runtime_settings);
        storage::set_runtime_storage_settings(storage::RuntimeStorageSettings {
            time_log_path: loaded.time_log_path,
        });
        self.keymap_error = None;
        self.render_needed = true;
    }

    fn refresh_keymap_if_changed(&mut self) {
        const POLL_INTERVAL: Duration = Duration::from_millis(300);
        if self.keymap_last_poll.elapsed() < POLL_INTERVAL {
            return;
        }
        self.keymap_last_poll = Instant::now();

        let keymap_path = storage::get_keymap_path();
        let modified = std::fs::metadata(&keymap_path)
            .and_then(|metadata| metadata.modified())
            .ok();

        if modified == self.keymap_last_modified {
            return;
        }

        self.keymap_last_modified = modified;

        match keybindings::load_keybindings(&keymap_path) {
            Ok(loaded) => {
                self.apply_loaded_keybindings(loaded);
            }
            Err(err) => {
                self.keymap_error = Some(err);
            }
        }

        self.render_needed = true;
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
        let visible_rows = inner_height as usize;

        let breathing_room = 5usize;
        let width_is_cramped = inner_width <= min_inner_width.saturating_add(breathing_room);
        let rows_are_cramped = row_count > visible_rows;

        let content_is_cramped = width_is_cramped || rows_are_cramped;
        if content_is_cramped {
            let target_width = terminal_size.width.saturating_mul(2) / 3;
            let max_width = terminal_size.width.saturating_sub(2).max(1);
            let modal_width = target_width.clamp(1, max_width);
            let modal_x = (terminal_size.width.saturating_sub(modal_width)) / 2;

            Rect::new(modal_x, compact.y, modal_width, compact.height)
        } else {
            compact
        }
    }

    fn get_idle_face(&self) -> String {
        let idle_seconds = self
            .none_entry_time
            .map_or(0, |t| t.elapsed().as_secs() as usize);

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
            app.persist_sand_state();
            app.persist_daily_sand_snapshot();
            last_save = Instant::now();
        }

        app.refresh_keymap_if_changed();

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
    app.persist_daily_sand_snapshot();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
