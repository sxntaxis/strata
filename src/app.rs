use std::{
    collections::{HashSet, VecDeque},
    io,
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};
use serde::{Deserialize, Serialize};

use crate::{
    constants::{
        APP_LAYOUT_SETTINGS, BLINK_SETTINGS, CATCHUP_SETTINGS, FACE_SETTINGS,
        RUNTIME_LOOP_SETTINGS, SAND_ENGINE, TIME_SETTINGS,
    },
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_DISPLAY_NAME, DRIFT_CATEGORY_ID, FirstDayOfWeek,
        ReportPeriod, RuntimeSettings, TimeTracker, civil_time_for_utc, is_drift_category_id,
        operational_day_key_for_utc, set_runtime_settings,
    },
    keybindings,
    sand::{SandEngine, SandState, SandStateGrain},
    sqlite, storage, temporal,
};

mod category_modal_view;
mod category_state;
mod command_palette_view;
mod event_handlers;
mod keybindings_modal_view;
mod persistence_recovery;
mod render_views;
mod report_modal_view;
mod report_state;
mod time_format;
mod ui_helpers;
mod view_style;

use persistence_recovery::{PersistenceOperation, PersistenceRecoveryState, RecoveryAction};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiMode {
    Main,
    CategoryModal,
    KarmaModal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionClockMode {
    LiveMonotonic,
    HistoricalWall,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QueuedMutation {
    SwitchLayer(CategoryId),
    ClearAllSand,
    ClearDriftSand,
}

#[derive(Clone, Debug)]
struct QueuedMutationEvent {
    execute_at_utc: DateTime<Utc>,
    mutation: QueuedMutation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum QueuedMutationRecord {
    SwitchLayer { category_id: u64 },
    ClearAllSand,
    ClearDriftSand,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct QueuedMutationEventRecord {
    execute_at_utc: DateTime<Utc>,
    mutation: QueuedMutationRecord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DetachedRuntimeCheckpoint {
    schema_version: u8,
    detached_at_utc: DateTime<Utc>,
    simulation_time_utc: DateTime<Utc>,
    spawn_accumulator_nanos: u64,
    physics_accumulator_nanos: u64,
    active_category_id: u64,
    active_description: String,
    active_session_started_at_utc: Option<DateTime<Utc>>,
    sand_state: crate::sand::SandState,
    pending_mutations: Vec<QueuedMutationEventRecord>,
}

struct SessionState {
    blink_state: i32,
    active_session_stable_id: Option<String>,
    active_session_started_at_utc: Option<DateTime<Utc>>,
    none_entry_time: Option<Instant>,
}

struct SimulationState {
    simulation_time_utc: DateTime<Utc>,
    spawn_accumulator: Duration,
    physics_accumulator: Duration,
    catchup_cadence_accumulator: Duration,
    catchup_visual_engine: Option<SandEngine>,
    catchup_visual_last_refresh: Instant,
    catchup_progress_anchor: Option<Duration>,
    catchup_gauge_hold_until: Option<Instant>,
    catchup_was_active: bool,
    pending_mutations: VecDeque<QueuedMutationEvent>,
}

struct App {
    time_tracker: TimeTracker,
    sand_engine: SandEngine,
    session: SessionState,
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
    simulation: SimulationState,
    detach_requested: bool,
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
    sqlite_database_path: Option<PathBuf>,
    archived_categories: Vec<Category>,
    checkpoint_recovery_active: bool,
    persistence_recovery: Option<PersistenceRecoveryState>,
    recovery_exit_requested: bool,
    recovery_exit_error: Option<String>,
}

impl App {
    fn new(
        width: u16,
        height: u16,
        loaded: keybindings::LoadedKeybindings,
    ) -> Result<Self, String> {
        let keymap_path = storage::get_keymap_path();
        let keymap_last_modified = std::fs::metadata(&keymap_path)
            .and_then(|metadata| metadata.modified())
            .ok();
        let keybindings::LoadedKeybindings {
            keymap,
            runtime_settings,
            time_log_path: _,
        } = loaded;
        let keymap_error = None;

        let mut tracker = TimeTracker::new();
        let authority = sqlite::resolve_runtime_authority()?;
        let (
            sqlite_database_path,
            loaded_categories,
            loaded_sessions,
            mut category_tags,
            archived_categories,
            sqlite_active_session,
        ) = match authority {
            sqlite::RuntimeAuthority::LegacyFiles => {
                let categories_path = storage::get_categories_path();
                let sessions_path = storage::get_time_log_path();
                let loaded_categories = storage::try_load_categories_from_csv(&categories_path)
                    .map_err(|error| error.to_string())?;
                let loaded_sessions = storage::try_load_sessions_from_csv(
                    &sessions_path,
                    &loaded_categories.categories,
                )
                .map_err(|error| error.to_string())?;
                let tags = storage::load_category_tags(&storage::get_category_tags_path());
                (
                    None,
                    loaded_categories,
                    loaded_sessions,
                    tags,
                    Vec::new(),
                    None,
                )
            }
            sqlite::RuntimeAuthority::SqliteCli { database_path } => {
                let state = sqlite::load_tui_state(&database_path)?;
                (
                    Some(database_path),
                    state.loaded_categories,
                    state.loaded_sessions,
                    state.category_tags,
                    state.archived_categories,
                    state.active_session,
                )
            }
        };
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );

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
            session: SessionState {
                blink_state: 0,
                active_session_stable_id: None,
                active_session_started_at_utc: None,
                none_entry_time: None,
            },
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
            simulation: SimulationState {
                simulation_time_utc: Utc::now(),
                spawn_accumulator: Duration::ZERO,
                physics_accumulator: Duration::ZERO,
                catchup_cadence_accumulator: Duration::ZERO,
                catchup_visual_engine: None,
                catchup_visual_last_refresh: Instant::now(),
                catchup_progress_anchor: None,
                catchup_gauge_hold_until: None,
                catchup_was_active: false,
                pending_mutations: VecDeque::new(),
            },
            detach_requested: false,
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
            sqlite_database_path,
            archived_categories,
            checkpoint_recovery_active: false,
            persistence_recovery: None,
            recovery_exit_requested: false,
            recovery_exit_error: None,
        };

        app.persist_category_tags();

        if !app.restore_from_detached_checkpoint() {
            if let Some(active) = sqlite_active_session {
                if !app
                    .time_tracker
                    .set_active_category_by_id(active.category_id)
                {
                    return Err(format!(
                        "SQLite active session references unavailable category {}",
                        active.category_id.0
                    ));
                }
                let _ = app
                    .time_tracker
                    .set_category_description_by_id(active.category_id, active.description);
                app.session.active_session_stable_id = Some(active.stable_id);
                app.begin_active_session_at(active.started_at_utc, false)?;
            } else {
                app.begin_active_session_now();
                app.persist_active_session_start();
            }
            app.restore_sand_state();
        }

        app.sync_drift_idle_state();
        app.commit_checkpoint_recovery_if_ready();
        if let Some(recovery) = app.persistence_recovery.take() {
            return Err(recovery.failure.summary());
        }

        Ok(app)
    }

    fn persist_active_session_start(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return;
        };
        let category_id = self.time_tracker.active_category_id();
        let description = self
            .time_tracker
            .category_description_by_id(category_id)
            .unwrap_or_default()
            .to_string();
        let started_at = self
            .session
            .active_session_started_at_utc
            .unwrap_or_else(Utc::now);
        let result = sqlite::ensure_tui_active_session(
            &database_path,
            category_id,
            &description,
            started_at,
        );
        if let Some(stable_id) = self.record_storage_result_for(
            PersistenceOperation::ActiveStart,
            RecoveryAction::ReloadAuthority,
            result,
        ) {
            self.session.active_session_stable_id = Some(stable_id);
        }
    }

    fn reload_sqlite_sessions(&mut self) -> bool {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return true;
        };
        let reload_result = sqlite::inject_tui_test_fault("session-reload", "before-read")
            .and_then(|()| sqlite::load_tui_state(&database_path));
        let Some(state) = self.record_storage_result_for(
            PersistenceOperation::StateReload,
            RecoveryAction::ReloadAuthority,
            reload_result,
        ) else {
            return false;
        };
        self.time_tracker.sessions = state.loaded_sessions.sessions;
        self.time_tracker.session_id_counter = state.loaded_sessions.next_session_id;
        self.archived_categories = state.archived_categories;
        true
    }

    fn reset_active_session_at(
        &mut self,
        started_at_utc: DateTime<Utc>,
        accept_large_wall_interval: bool,
    ) {
        if let Err(error) =
            temporal::checked_wall_interval(started_at_utc, Utc::now(), accept_large_wall_interval)
        {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveReset,
                    RecoveryAction::ReloadAuthority,
                    Err("SQLite runtime has no active stable identity to reset".to_string()),
                );
                return;
            };
            let operation_id = self.transition_operation_id(
                "reset",
                &expected_stable_id,
                started_at_utc,
                "active",
            );
            let next_stable_id = format!("tui-active:{operation_id}");
            let result = sqlite::reset_tui_active_session(
                &database_path,
                &expected_stable_id,
                &operation_id,
                &next_stable_id,
                started_at_utc,
            );
            let Some(receipt) = self.record_storage_result_for(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                result,
            ) else {
                return;
            };
            self.session.active_session_stable_id = receipt.resulting_active_stable_id;
        }
        if let Err(error) = self.begin_active_session_at(started_at_utc, accept_large_wall_interval)
        {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
        }
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
        crate::domain::is_drift_name(name)
    }

    fn display_layer_name(&self, name: &str) -> String {
        if Self::is_drift_name(name) {
            DRIFT_CATEGORY_DISPLAY_NAME.to_string()
        } else {
            name.to_string()
        }
    }

    fn sync_drift_idle_state(&mut self) {
        if is_drift_category_id(self.time_tracker.active_category_id()) {
            self.session.blink_state = self.next_blink_interval();
            self.session.none_entry_time = self.time_tracker.current_session_start;
        } else {
            self.session.none_entry_time = None;
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

    fn week_start_options() -> [FirstDayOfWeek; 7] {
        [
            FirstDayOfWeek::Monday,
            FirstDayOfWeek::Tuesday,
            FirstDayOfWeek::Wednesday,
            FirstDayOfWeek::Thursday,
            FirstDayOfWeek::Friday,
            FirstDayOfWeek::Saturday,
            FirstDayOfWeek::Sunday,
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
        let abs_offset = boundary.utc_offset_seconds.unsigned_abs();
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
        let poll_interval = Duration::from_millis(RUNTIME_LOOP_SETTINGS.keymap_poll_ms);
        if self.keymap_last_poll.elapsed() < poll_interval {
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
        let target_height = (terminal_size.height.saturating_mul(numerator) / denominator)
            .max(APP_LAYOUT_SETTINGS.modal_min_height);

        let frame_padding = APP_LAYOUT_SETTINGS.frame_margin;
        let max_width = terminal_size
            .width
            .saturating_sub(frame_padding)
            .saturating_sub(frame_padding)
            .max(1);
        let max_height = terminal_size
            .height
            .saturating_sub(frame_padding)
            .saturating_sub(frame_padding)
            .max(1);

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

        let breathing_room = APP_LAYOUT_SETTINGS.report_breathing_room;
        let width_is_cramped = inner_width <= min_inner_width.saturating_add(breathing_room);
        let rows_are_cramped = row_count > visible_rows;

        let content_is_cramped = width_is_cramped || rows_are_cramped;
        if content_is_cramped {
            let target_width = terminal_size.width.saturating_mul(2) / 3;
            let frame_padding = APP_LAYOUT_SETTINGS.frame_margin;
            let max_width = terminal_size
                .width
                .saturating_sub(frame_padding)
                .saturating_sub(frame_padding)
                .max(1);
            let modal_width = target_width.clamp(1, max_width);
            let modal_x = (terminal_size.width.saturating_sub(modal_width)) / 2;

            Rect::new(modal_x, compact.y, modal_width, compact.height)
        } else {
            compact
        }
    }

    fn get_idle_face(&self) -> String {
        let idle_seconds = self
            .session
            .none_entry_time
            .map_or(0, |t| t.elapsed().as_secs() as usize);

        if self.session.blink_state < 0 {
            "(-_-)".to_string()
        } else if self.session.blink_state > 0 {
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
        if self.session.blink_state < 0 {
            self.session.blink_state -= 1;
            let blink_duration = BLINK_SETTINGS.duration_min_frames
                + (rand::random::<i32>()
                    % (BLINK_SETTINGS.duration_max_frames - BLINK_SETTINGS.duration_min_frames));
            if self.session.blink_state < -blink_duration {
                self.session.blink_state = self.next_blink_interval();
            }
        } else if self.session.blink_state > 0 {
            self.session.blink_state -= 1;
            if self.session.blink_state == 0 {
                self.session.blink_state = -1;
            }
        }
    }

    fn begin_active_session_now(&mut self) {
        let now = Utc::now();
        self.time_tracker.start_session();
        self.session.active_session_started_at_utc = Some(now);
    }

    fn begin_active_session_at(
        &mut self,
        started_at_utc: DateTime<Utc>,
        accept_large_wall_interval: bool,
    ) -> Result<(), String> {
        let interval = temporal::checked_wall_interval(
            started_at_utc,
            Utc::now(),
            accept_large_wall_interval,
        )?;
        self.time_tracker
            .start_session_with_elapsed(interval.elapsed_seconds)?;
        self.session.active_session_started_at_utc = Some(started_at_utc);
        Ok(())
    }

    fn begin_transition_session(
        &mut self,
        started_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Result<(), String> {
        match clock_mode {
            SessionClockMode::LiveMonotonic => {
                self.time_tracker.start_session();
                self.session.active_session_started_at_utc = Some(started_at_utc);
                Ok(())
            }
            SessionClockMode::HistoricalWall => self.begin_active_session_at(started_at_utc, true),
        }
    }

    fn reconciled_active_interval(
        &self,
        observed_end_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Result<temporal::ReconciledInterval, String> {
        let started_at_utc = self
            .session
            .active_session_started_at_utc
            .ok_or_else(|| "active session is missing its UTC start timestamp".to_string())?;
        match clock_mode {
            SessionClockMode::LiveMonotonic => temporal::reconcile_live_interval(
                started_at_utc,
                observed_end_utc,
                self.time_tracker.current_elapsed().unwrap_or_default(),
            ),
            SessionClockMode::HistoricalWall => {
                temporal::checked_wall_interval(started_at_utc, observed_end_utc, true)
            }
        }
    }

    fn end_active_session_now(&mut self) -> Option<usize> {
        self.end_active_session_at(Utc::now(), SessionClockMode::LiveMonotonic)
    }

    fn end_active_session_at(
        &mut self,
        observed_end_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Option<usize> {
        let interval = match self.reconciled_active_interval(observed_end_utc, clock_mode) {
            Ok(interval) => interval,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveFinish,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return None;
            }
        };
        let elapsed = interval.elapsed_seconds;
        let ended_civil = civil_time_for_utc(interval.ended_at_utc);

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveFinish,
                    RecoveryAction::ReloadAuthority,
                    Err("SQLite runtime has no active stable identity to finish".to_string()),
                );
                return None;
            };
            let operational_day = operational_day_key_for_utc(interval.ended_at_utc)
                .format("%Y-%m-%d")
                .to_string();
            let operation_id = format!("finish:{expected_stable_id}");
            self.record_storage_result_for(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::ReloadAuthority,
                sqlite::finish_tui_active_session(
                    &database_path,
                    &expected_stable_id,
                    &operation_id,
                    interval.ended_at_utc,
                    &operational_day,
                    elapsed,
                ),
            )?;
            let active_category_id = self.time_tracker.active_category_id();
            let _ = self
                .time_tracker
                .set_category_description_by_id(active_category_id, String::new());
            self.time_tracker.current_session_start = None;
            self.session.active_session_stable_id = None;
            self.session.active_session_started_at_utc = None;
            self.reload_sqlite_sessions();
            self.persist_categories();
            return Some(elapsed);
        }

        let result = self
            .time_tracker
            .end_session_with_elapsed_at_local(elapsed, ended_civil);
        self.session.active_session_started_at_utc = None;
        result
    }

    fn switch_active_category_at(
        &mut self,
        category_id: CategoryId,
        switched_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> bool {
        if self.time_tracker.active_category_id() == category_id {
            return false;
        }

        if self.time_tracker.category_by_id(category_id).is_none() {
            return false;
        }

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveSwitch,
                    RecoveryAction::ReloadAuthority,
                    Err("SQLite runtime has no active stable identity to switch".to_string()),
                );
                return false;
            };
            let interval = match self.reconciled_active_interval(switched_at_utc, clock_mode) {
                Ok(interval) => interval,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::ActiveSwitch,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return false;
                }
            };
            let elapsed = interval.elapsed_seconds;
            let operational_day = operational_day_key_for_utc(interval.ended_at_utc)
                .format("%Y-%m-%d")
                .to_string();
            let next_description = self
                .time_tracker
                .category_description_by_id(category_id)
                .unwrap_or_default()
                .to_string();
            let operation_id = self.transition_operation_id(
                "switch",
                &expected_stable_id,
                interval.ended_at_utc,
                &category_id.0.to_string(),
            );
            let next_stable_id = format!("tui-active:{operation_id}");
            let result = sqlite::switch_tui_active_session(
                &database_path,
                &expected_stable_id,
                &operation_id,
                &next_stable_id,
                category_id,
                &next_description,
                interval.ended_at_utc,
                &operational_day,
                elapsed,
            );
            let Some(receipt) = self.record_storage_result_for(
                PersistenceOperation::ActiveSwitch,
                RecoveryAction::ReloadAuthority,
                result,
            ) else {
                return false;
            };
            let previous_category_id = self.time_tracker.active_category_id();
            let _ = self
                .time_tracker
                .set_category_description_by_id(previous_category_id, String::new());
            if !self.time_tracker.set_active_category_by_id(category_id) {
                return false;
            }
            self.session.active_session_stable_id = receipt.resulting_active_stable_id;
            if let Err(error) = self.begin_transition_session(interval.ended_at_utc, clock_mode) {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveStart,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return false;
            }
            self.reload_sqlite_sessions();
            self.persist_categories();
            self.sync_drift_idle_state();
            return true;
        }

        if self
            .end_active_session_at(switched_at_utc, clock_mode)
            .is_none()
        {
            return false;
        }
        self.persist_sessions();

        if !self.time_tracker.set_active_category_by_id(category_id) {
            return false;
        }

        if let Err(error) = self.begin_transition_session(switched_at_utc, clock_mode) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }
        self.sync_drift_idle_state();

        true
    }

    fn transition_operation_id(
        &self,
        kind: &str,
        expected_stable_id: &str,
        at_utc: DateTime<Utc>,
        discriminator: &str,
    ) -> String {
        format!(
            "{}:{}:{}:{}",
            kind,
            expected_stable_id,
            at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true),
            discriminator
        )
    }

    fn simulation_backlog_duration_at(&self, now_utc: DateTime<Utc>) -> Duration {
        if now_utc <= self.simulation.simulation_time_utc {
            Duration::ZERO
        } else {
            (now_utc - self.simulation.simulation_time_utc)
                .to_std()
                .unwrap_or(Duration::ZERO)
        }
    }

    fn simulation_backlog_duration(&self) -> Duration {
        self.simulation_backlog_duration_at(Utc::now())
    }

    fn catchup_target_utc(&self, now_utc: DateTime<Utc>) -> DateTime<Utc> {
        if let Some(next) = self.simulation.pending_mutations.front()
            && next.execute_at_utc > now_utc
        {
            next.execute_at_utc
        } else {
            now_utc
        }
    }

    fn current_catchup_multiplier(&self, backlog: Duration) -> u32 {
        if backlog.is_zero() {
            return 1;
        }

        CATCHUP_SETTINGS.accelerated_multiplier.max(2)
    }

    fn catchup_visibility_threshold(&self) -> Duration {
        Duration::from_millis(CATCHUP_SETTINGS.cadence_ms)
    }

    fn is_catching_up(&self) -> bool {
        self.simulation_backlog_duration() > self.catchup_visibility_threshold()
            || !self.simulation.pending_mutations.is_empty()
    }

    fn queue_or_apply_mutation(&mut self, mutation: QueuedMutation) {
        if self.is_catching_up() || !self.simulation.pending_mutations.is_empty() {
            self.simulation
                .pending_mutations
                .push_back(QueuedMutationEvent {
                    execute_at_utc: Utc::now(),
                    mutation,
                });
        } else {
            self.apply_mutation_at(mutation, Utc::now(), SessionClockMode::LiveMonotonic);
        }
        self.render_needed = true;
    }

    fn apply_mutation_at(
        &mut self,
        mutation: QueuedMutation,
        scheduled_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) {
        match mutation {
            QueuedMutation::SwitchLayer(category_id) => {
                self.apply_switch_layer_at(category_id, scheduled_at_utc, clock_mode);
            }
            QueuedMutation::ClearAllSand => {
                self.sand_engine.clear();

                let scheduled_day = operational_day_key_for_utc(scheduled_at_utc);
                if let Some(database_path) = self.sqlite_database_path.clone() {
                    let day = scheduled_day.format("%Y-%m-%d").to_string();
                    let result = sqlite::delete_tui_drift_sessions_for_day(&database_path, &day);
                    if self
                        .record_storage_result_for(
                            PersistenceOperation::DriftSessionDelete,
                            RecoveryAction::ReloadAuthority,
                            result,
                        )
                        .is_none()
                    {
                        return;
                    }
                }
                self.time_tracker
                    .clear_drift_sessions_for_day(scheduled_day);

                if is_drift_category_id(self.time_tracker.active_category_id()) {
                    self.reset_active_session_at(
                        scheduled_at_utc,
                        clock_mode == SessionClockMode::HistoricalWall,
                    );
                    self.sync_drift_idle_state();
                }

                self.persist_sessions();
                self.persist_sand_state();
                self.persist_daily_sand_snapshot();
            }
            QueuedMutation::ClearDriftSand => {
                self.sand_engine.clear_category(DRIFT_CATEGORY_ID);
                self.persist_sand_state();
                self.persist_daily_sand_snapshot();
            }
        }
    }

    fn apply_switch_layer_at(
        &mut self,
        category_id: CategoryId,
        scheduled_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) {
        self.switch_active_category_at(category_id, scheduled_at_utc, clock_mode);
    }

    fn advance_runtime(
        &mut self,
        wall_delta: Duration,
        tick_rate: Duration,
        physics_rate: Duration,
    ) {
        let was_catching = self.simulation.catchup_was_active;
        let cadence = Duration::from_millis(CATCHUP_SETTINGS.cadence_ms);
        self.simulation.catchup_cadence_accumulator = self
            .simulation
            .catchup_cadence_accumulator
            .saturating_add(wall_delta);

        let target_utc = self.catchup_target_utc(Utc::now());
        let backlog = self.simulation_backlog_duration_at(target_utc);

        if backlog.is_zero() {
            self.advance_simulation_by(Duration::ZERO, tick_rate, physics_rate);
            self.simulation.catchup_cadence_accumulator =
                self.simulation.catchup_cadence_accumulator.min(cadence);
        } else {
            while self.simulation.catchup_cadence_accumulator >= cadence {
                self.simulation.catchup_cadence_accumulator = self
                    .simulation
                    .catchup_cadence_accumulator
                    .saturating_sub(cadence);

                let step_target_utc = self.catchup_target_utc(Utc::now());
                let step_backlog = self.simulation_backlog_duration_at(step_target_utc);
                if step_backlog.is_zero() {
                    self.advance_simulation_by(Duration::ZERO, tick_rate, physics_rate);
                    break;
                }

                let speed = self.current_catchup_multiplier(step_backlog);
                let step_budget = cadence.saturating_mul(speed);
                let advance_by = step_backlog.min(step_budget);
                if advance_by.is_zero() {
                    break;
                }

                self.advance_simulation_by(advance_by, tick_rate, physics_rate);
                self.render_needed = true;
            }
        }

        let now_catching = self.is_catching_up();
        if was_catching && !now_catching {
            self.finalize_catchup_transition();
            self.simulation.catchup_gauge_hold_until =
                Some(Instant::now() + Duration::from_millis(CATCHUP_SETTINGS.gauge_hold_ms));
        }
        self.simulation.catchup_was_active = now_catching;
        self.commit_checkpoint_recovery_if_ready();
    }

    fn finalize_catchup_transition(&mut self) {
        let settled = self.build_catchup_projection_state(&self.sand_engine.snapshot_state());
        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        self.sand_engine
            .restore_state(&settled, &valid_category_ids);
        self.simulation.catchup_visual_engine = None;
        self.simulation.catchup_progress_anchor = None;
        self.simulation.catchup_visual_last_refresh = Instant::now();
        self.simulation.catchup_cadence_accumulator = Duration::ZERO;
        self.render_needed = true;
    }

    fn catchup_visual_lines(
        &mut self,
        cell_width: u16,
        cell_height: u16,
        categories: &[Category],
    ) -> Option<Vec<ratatui::prelude::Line<'static>>> {
        if !self.is_catching_up() {
            self.simulation.catchup_visual_engine = None;
            return None;
        }

        let expected_grid_width = cell_width * SAND_ENGINE.dot_width as u16;
        let expected_grid_height = cell_height * SAND_ENGINE.dot_height as u16;
        let visual_cadence = Duration::from_millis(CATCHUP_SETTINGS.visual_refresh_ms);

        let should_recreate = self
            .simulation
            .catchup_visual_engine
            .as_ref()
            .map(|engine| {
                engine.width != expected_grid_width || engine.height != expected_grid_height
            })
            .unwrap_or(true);
        if should_recreate {
            self.simulation.catchup_visual_engine = Some(SandEngine::new(cell_width, cell_height));
            self.simulation.catchup_visual_last_refresh = Instant::now() - visual_cadence;
        }

        let should_refresh =
            self.simulation.catchup_visual_last_refresh.elapsed() >= visual_cadence;
        if should_refresh {
            let state = self.sand_engine.snapshot_state();
            let projected_state = self.build_catchup_projection_state(&state);

            let valid_category_ids = categories
                .iter()
                .map(|category| category.id)
                .collect::<HashSet<_>>();

            if let Some(engine) = self.simulation.catchup_visual_engine.as_mut() {
                engine.restore_state(&projected_state, &valid_category_ids);
            }
            self.simulation.catchup_visual_last_refresh = Instant::now();
        }

        self.simulation
            .catchup_visual_engine
            .as_ref()
            .map(|engine| engine.render(categories))
    }

    fn build_catchup_projection_state(&self, state: &SandState) -> SandState {
        if state.grid_width == 0 || state.grid_height == 0 {
            return state.clone();
        }

        let mut columns: Vec<Vec<u64>> = vec![Vec::new(); state.grid_width];
        let mut grains = state.grains.clone();
        grains.sort_by(|a, b| a.x.cmp(&b.x).then(b.y.cmp(&a.y)));

        for grain in grains {
            if grain.x < state.grid_width && grain.y < state.grid_height {
                columns[grain.x].push(grain.category_id);
            }
        }

        self.relax_projection_columns(&mut columns, state.grid_height);

        let mut projected_grains = Vec::with_capacity(state.grains.len());
        for (x, column) in columns.into_iter().enumerate() {
            for (index, category_id) in column.into_iter().take(state.grid_height).enumerate() {
                let y = state.grid_height - 1 - index;
                projected_grains.push(SandStateGrain { x, y, category_id });
            }
        }

        SandState {
            version: state.version,
            grid_width: state.grid_width,
            grid_height: state.grid_height,
            grains: projected_grains,
            frame_count: state.frame_count,
            sweep_left_to_right: state.sweep_left_to_right,
            rng_state: state.rng_state,
        }
    }

    fn relax_projection_columns(&self, columns: &mut [Vec<u64>], max_height: usize) {
        if columns.len() < 2 {
            return;
        }

        let threshold = CATCHUP_SETTINGS.repose_threshold.max(1);
        for _ in 0..CATCHUP_SETTINGS.relax_passes {
            let mut moved = false;

            for idx in 0..(columns.len() - 1) {
                moved |= Self::relax_projection_pair(columns, idx, idx + 1, threshold, max_height);
            }

            for idx in (1..columns.len()).rev() {
                moved |= Self::relax_projection_pair(columns, idx, idx - 1, threshold, max_height);
            }

            if !moved {
                break;
            }
        }
    }

    fn relax_projection_pair(
        columns: &mut [Vec<u64>],
        from: usize,
        to: usize,
        threshold: usize,
        max_height: usize,
    ) -> bool {
        let from_height = columns[from].len();
        let to_height = columns[to].len();

        if from_height <= to_height + threshold || to_height >= max_height {
            return false;
        }

        if let Some(grain) = columns[from].pop() {
            columns[to].push(grain);
            true
        } else {
            false
        }
    }

    fn advance_simulation_by(
        &mut self,
        delta: Duration,
        tick_rate: Duration,
        physics_rate: Duration,
    ) {
        let target_time = self.simulation.simulation_time_utc
            + ChronoDuration::from_std(delta).unwrap_or(ChronoDuration::zero());

        loop {
            let Some(next) = self.simulation.pending_mutations.front().cloned() else {
                break;
            };

            if next.execute_at_utc > target_time {
                break;
            }

            let mut pre_delta = Duration::ZERO;
            if next.execute_at_utc > self.simulation.simulation_time_utc {
                pre_delta = (next.execute_at_utc - self.simulation.simulation_time_utc)
                    .to_std()
                    .unwrap_or(Duration::ZERO);
            }

            self.process_simulation_delta(pre_delta, tick_rate, physics_rate);
            self.simulation.simulation_time_utc = next.execute_at_utc;
            self.simulation.pending_mutations.pop_front();
            self.apply_mutation_at(
                next.mutation,
                next.execute_at_utc,
                SessionClockMode::HistoricalWall,
            );
        }

        if delta.is_zero() {
            return;
        }

        let mut remaining = Duration::ZERO;
        if target_time > self.simulation.simulation_time_utc {
            remaining = (target_time - self.simulation.simulation_time_utc)
                .to_std()
                .unwrap_or(Duration::ZERO);
        }

        self.process_simulation_delta(remaining, tick_rate, physics_rate);
        self.simulation.simulation_time_utc = target_time;
    }

    fn process_simulation_delta(
        &mut self,
        mut delta: Duration,
        tick_rate: Duration,
        physics_rate: Duration,
    ) {
        while !delta.is_zero() {
            let spawn_left = tick_rate.saturating_sub(self.simulation.spawn_accumulator);
            let physics_left = physics_rate.saturating_sub(self.simulation.physics_accumulator);
            let next_event = spawn_left.min(physics_left);

            let step = delta.min(next_event);
            self.simulation.spawn_accumulator += step;
            self.simulation.physics_accumulator += step;
            delta = delta.saturating_sub(step);

            let spawn_due = self.simulation.spawn_accumulator >= tick_rate;
            let physics_due = self.simulation.physics_accumulator >= physics_rate;

            if spawn_due {
                self.simulation.spawn_accumulator =
                    self.simulation.spawn_accumulator.saturating_sub(tick_rate);
                self.run_spawn_tick();
            }

            if physics_due {
                self.simulation.physics_accumulator = self
                    .simulation
                    .physics_accumulator
                    .saturating_sub(physics_rate);
                self.run_physics_tick();
            }

            if step.is_zero() && !spawn_due && !physics_due {
                break;
            }
        }
    }

    fn run_spawn_tick(&mut self) {
        let should_spawn = self.time_tracker.current_session_start.is_some()
            && self.time_tracker.active_category_index().is_some();

        if should_spawn {
            let cat_id = self.time_tracker.active_category_id();
            self.sand_engine.spawn(cat_id);
        }
    }

    fn run_physics_tick(&mut self) {
        self.sand_engine.update();
        if is_drift_category_id(self.time_tracker.active_category_id()) && !self.is_catching_up() {
            self.update_blink();
        }
    }

    fn catchup_progress_ratio(&mut self) -> Option<f64> {
        let target_utc = self.catchup_target_utc(Utc::now());
        let backlog = self.simulation_backlog_duration_at(target_utc);

        if backlog <= self.catchup_visibility_threshold()
            && self.simulation.pending_mutations.is_empty()
            && let Some(until) = self.simulation.catchup_gauge_hold_until
        {
            if Instant::now() < until {
                return Some(1.0);
            }
            self.simulation.catchup_gauge_hold_until = None;
        }

        if backlog <= self.catchup_visibility_threshold()
            && self.simulation.pending_mutations.is_empty()
        {
            if self.simulation.catchup_progress_anchor.is_some() {
                self.simulation.catchup_progress_anchor = None;
                return Some(1.0);
            }
            return None;
        }

        self.simulation.catchup_gauge_hold_until = None;

        if backlog <= self.catchup_visibility_threshold() {
            return Some(1.0);
        }

        let effective_backlog = backlog;
        let min_anchor = Duration::from_millis(APP_LAYOUT_SETTINGS.catchup_progress_min_anchor_ms);
        let anchor = self
            .simulation
            .catchup_progress_anchor
            .get_or_insert(effective_backlog.max(min_anchor));
        if effective_backlog > *anchor {
            *anchor = effective_backlog;
        }

        let denom = anchor.as_secs_f64();
        if denom <= f64::EPSILON {
            return Some(1.0);
        }

        let ratio = 1.0 - (effective_backlog.as_secs_f64() / denom);
        Some(ratio.clamp(0.0, 1.0))
    }

    fn persist_detached_checkpoint(&mut self) {
        if self.checkpoint_recovery_active {
            return;
        }
        let active_category_id = self.time_tracker.active_category_id();
        let active_description = self
            .time_tracker
            .category_description_by_id(active_category_id)
            .unwrap_or_default()
            .to_string();

        let checkpoint = DetachedRuntimeCheckpoint {
            schema_version: 1,
            detached_at_utc: Utc::now(),
            simulation_time_utc: self.simulation.simulation_time_utc,
            spawn_accumulator_nanos: self
                .simulation
                .spawn_accumulator
                .as_nanos()
                .min(u64::MAX as u128) as u64,
            physics_accumulator_nanos: self
                .simulation
                .physics_accumulator
                .as_nanos()
                .min(u64::MAX as u128) as u64,
            active_category_id: active_category_id.0,
            active_description,
            active_session_started_at_utc: self.session.active_session_started_at_utc,
            sand_state: self.sand_engine.snapshot_state(),
            pending_mutations: self
                .simulation
                .pending_mutations
                .iter()
                .map(|event| QueuedMutationEventRecord {
                    execute_at_utc: event.execute_at_utc,
                    mutation: match event.mutation {
                        QueuedMutation::SwitchLayer(category_id) => {
                            QueuedMutationRecord::SwitchLayer {
                                category_id: category_id.0,
                            }
                        }
                        QueuedMutation::ClearAllSand => QueuedMutationRecord::ClearAllSand,
                        QueuedMutation::ClearDriftSand => QueuedMutationRecord::ClearDriftSand,
                    },
                })
                .collect(),
        };

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result::<()>(Err(
                    "SQLite runtime has no active stable identity to checkpoint".to_string(),
                ));
                return;
            };
            let result = sqlite::save_tui_checkpoint(
                &database_path,
                &expected_stable_id,
                checkpoint.detached_at_utc,
                checkpoint.simulation_time_utc,
                &checkpoint,
            );
            self.record_storage_result_for(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::DetachAndExit,
                result,
            );
        } else {
            let path = storage::get_detached_runtime_path();
            if let Err(error) = storage::write_json_atomic(&path, &checkpoint) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    fn clear_detached_checkpoint(&mut self) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::clear_tui_checkpoint(&database_path);
            self.record_storage_result_for(
                PersistenceOperation::CheckpointClear,
                RecoveryAction::FlushCurrentState,
                result,
            );
        } else {
            let path = storage::get_detached_runtime_path();
            if let Err(error) = storage::delete_file_if_exists(&path) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    fn restore_from_detached_checkpoint(&mut self) -> bool {
        let checkpoint: DetachedRuntimeCheckpoint = if let Some(database_path) =
            self.sqlite_database_path.clone()
        {
            match sqlite::load_tui_checkpoint(&database_path) {
                Ok(Some(claimed)) => {
                    let Some(active_stable_id) = claimed.active_session_stable_id else {
                        let _ = sqlite::quarantine_tui_checkpoint(&database_path);
                        self.record_storage_result::<()>(Err(
                            "SQLite recovery checkpoint has no active stable identity".to_string(),
                        ));
                        return false;
                    };
                    self.session.active_session_stable_id = Some(active_stable_id);
                    self.checkpoint_recovery_active = true;
                    claimed.payload
                }
                Ok(None) => return false,
                Err(error) => {
                    self.record_storage_result::<()>(Err(error));
                    return false;
                }
            }
        } else {
            let path = storage::get_detached_runtime_path();
            if !storage::file_exists(&path) {
                return false;
            }
            match storage::read_json(&path) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result::<()>(Err(error));
                    return false;
                }
            }
        };

        if checkpoint.schema_version != 1 {
            if let Some(database_path) = self.sqlite_database_path.clone() {
                let _ = sqlite::quarantine_tui_checkpoint(&database_path);
                self.checkpoint_recovery_active = false;
            }
            self.record_storage_result::<()>(Err(format!(
                "unsupported detached checkpoint schema {}",
                checkpoint.schema_version
            )));
            return false;
        }

        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        self.sand_engine
            .restore_state(&checkpoint.sand_state, &valid_category_ids);

        let active_category_id = CategoryId::new(checkpoint.active_category_id);
        if !self
            .time_tracker
            .set_active_category_by_id(active_category_id)
        {
            let _ = self
                .time_tracker
                .set_active_category_by_id(DRIFT_CATEGORY_ID);
        }
        let active_id = self.time_tracker.active_category_id();
        let _ = self
            .time_tracker
            .set_category_description_by_id(active_id, checkpoint.active_description);

        if let Some(started_at) = checkpoint.active_session_started_at_utc {
            if let Err(error) = self.begin_active_session_at(started_at, false) {
                self.record_storage_result::<()>(Err(error));
                return false;
            }
        } else {
            self.begin_active_session_now();
        }

        self.simulation.simulation_time_utc = checkpoint.simulation_time_utc;
        self.simulation.spawn_accumulator =
            Duration::from_nanos(checkpoint.spawn_accumulator_nanos);
        self.simulation.physics_accumulator =
            Duration::from_nanos(checkpoint.physics_accumulator_nanos);
        self.simulation.pending_mutations = checkpoint
            .pending_mutations
            .into_iter()
            .map(|event| QueuedMutationEvent {
                execute_at_utc: event.execute_at_utc,
                mutation: match event.mutation {
                    QueuedMutationRecord::SwitchLayer { category_id } => {
                        QueuedMutation::SwitchLayer(CategoryId::new(category_id))
                    }
                    QueuedMutationRecord::ClearAllSand => QueuedMutation::ClearAllSand,
                    QueuedMutationRecord::ClearDriftSand => QueuedMutation::ClearDriftSand,
                },
            })
            .collect();

        self.simulation
            .pending_mutations
            .make_contiguous()
            .sort_by(|a, b| a.execute_at_utc.cmp(&b.execute_at_utc));

        if self.sqlite_database_path.is_none() {
            self.clear_detached_checkpoint();
        }
        true
    }

    fn commit_checkpoint_recovery_if_ready(&mut self) {
        if !self.checkpoint_recovery_active
            || self.is_catching_up()
            || !self.simulation.pending_mutations.is_empty()
        {
            return;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.checkpoint_recovery_active = false;
            return;
        };
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result::<()>(Err(
                "SQLite recovery has no active stable identity to commit".to_string(),
            ));
            return;
        };
        let state = self.sand_engine.snapshot_state();
        let operational_day = crate::domain::operational_day_key_now()
            .format("%Y-%m-%d")
            .to_string();
        if self
            .record_storage_result_for(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                sqlite::commit_tui_checkpoint_recovery(
                    &database_path,
                    &expected_stable_id,
                    &operational_day,
                    &state,
                ),
            )
            .is_none()
        {
            return;
        }
        if self
            .record_storage_result_for(
                PersistenceOperation::CheckpointClear,
                RecoveryAction::FlushCurrentState,
                sqlite::clear_tui_checkpoint(&database_path),
            )
            .is_some()
        {
            self.checkpoint_recovery_active = false;
        }
    }

    fn next_blink_interval(&self) -> i32 {
        BLINK_SETTINGS.interval_min_frames
            + (rand::random::<i32>()
                % (BLINK_SETTINGS.interval_max_frames - BLINK_SETTINGS.interval_min_frames))
    }
}

pub fn run_ui(loaded: keybindings::LoadedKeybindings) -> Result<(), io::Error> {
    let (width, height) = crossterm::terminal::size()?;
    let mut app = App::new(width, height, loaded).map_err(io::Error::other)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let physics_rate = Duration::from_millis(TIME_SETTINGS.physics_ms);
    let tick_rate = Duration::from_millis(TIME_SETTINGS.tick_ms);
    let render_rate = Duration::from_millis(1000 / TIME_SETTINGS.target_fps);
    let save_rate = Duration::from_secs(RUNTIME_LOOP_SETTINGS.autosave_secs);
    let mut last_simulation_update = Instant::now();
    let mut last_render = Instant::now();
    let mut last_save = Instant::now();
    let mut runtime_error = None;

    'runtime: loop {
        loop {
            if !app.has_persistence_recovery() {
                let now = Instant::now();
                let wall_delta = now.saturating_duration_since(last_simulation_update);
                last_simulation_update = now;
                app.advance_runtime(wall_delta, tick_rate, physics_rate);

                if last_save.elapsed() >= save_rate {
                    app.persist_sessions();
                    if !app.has_persistence_recovery() {
                        app.persist_sand_state();
                    }
                    if !app.has_persistence_recovery() {
                        app.persist_daily_sand_snapshot();
                    }
                    last_save = Instant::now();
                }

                app.refresh_keymap_if_changed();
            }

            if last_render.elapsed() >= render_rate && app.render_needed {
                terminal.draw(|f| {
                    app.draw_frame(f);
                })?;
                app.render_needed = false;
                last_render = Instant::now();
            }

            if event::poll(Duration::from_millis(RUNTIME_LOOP_SETTINGS.input_poll_ms))?
                && let Event::Key(key) = event::read()?
            {
                if app.handle_key(key) {
                    break;
                }
                if app.detach_requested {
                    break;
                }
            }
        }

        if app.recovery_exit_requested {
            runtime_error = app.recovery_exit_error.take();
            break 'runtime;
        }

        if app.has_persistence_recovery() {
            continue 'runtime;
        }

        if app.checkpoint_recovery_active {
            app.begin_manual_persistence_failure(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                "recovery catch-up is not durably committed; checkpoint retained",
            );
            app.detach_requested = false;
            continue 'runtime;
        }

        if app.detach_requested {
            app.persist_sessions();
            if !app.has_persistence_recovery() {
                app.persist_sand_state();
            }
            if !app.has_persistence_recovery() {
                app.persist_daily_sand_snapshot();
            }
            if !app.has_persistence_recovery() {
                app.persist_detached_checkpoint();
            }
            if app.has_persistence_recovery() {
                app.promote_recovery_action(RecoveryAction::DetachAndExit);
                app.detach_requested = false;
                continue 'runtime;
            }
        } else {
            app.end_active_session_now();
            if !app.has_persistence_recovery() {
                app.persist_sessions();
            }
            if !app.has_persistence_recovery() {
                app.persist_sand_state();
            }
            if !app.has_persistence_recovery() {
                app.persist_daily_sand_snapshot();
            }
            if !app.has_persistence_recovery() {
                app.clear_detached_checkpoint();
            }
            if app.has_persistence_recovery() {
                app.promote_recovery_action(RecoveryAction::FinishAndExit);
                continue 'runtime;
            }
        }

        break 'runtime;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Some(error) = runtime_error {
        return Err(io::Error::other(error));
    }
    Ok(())
}
