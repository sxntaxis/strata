use std::{
    collections::{BTreeSet, HashSet},
    io,
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
use crossterm::event::{self, Event};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};

use crate::{
    constants::{APP_LAYOUT_SETTINGS, CATCHUP_SETTINGS, RUNTIME_LOOP_SETTINGS, TIME_SETTINGS},
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_DISPLAY_NAME, DRIFT_CATEGORY_ID, FirstDayOfWeek,
        OperationalDayPolicy, ReportPeriod, RuntimeSettings, TimeTracker, is_drift_category_id,
        operational_day_key_for_utc, set_runtime_settings,
    },
    keybindings::{self, Action, ActionBindingState, KeyBinding},
    runtime_identity::transition_identity,
    sand::{
        RecoveryTiming, SandEngine, SandState, SandStateGrain, SedimentSnapshot,
        recover_detached_sediment, settle_transition_sediment,
    },
    sqlite, storage, temporal,
};

mod category_modal_view;
mod category_state;
mod command_palette_view;
mod event_handlers;
mod keybindings_modal_view;
mod persistence_recovery;
mod recovery_statement;
mod render_views;
mod report_modal_view;
mod report_state;
mod terminal_lifecycle;
mod time_format;
mod ui_helpers;
mod view_style;

use persistence_recovery::{PersistenceOperation, PersistenceRecoveryState, RecoveryAction};
use terminal_lifecycle::{ManagedTerminal, TerminalSession};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UiMode {
    Main,
    CategoryModal,
    BalanceModal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionClockMode {
    LiveMonotonic,
    HistoricalWall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AtlasSelectable {
    WeekStartDay,
    Action(keybindings::Action),
}

#[derive(Clone, Debug)]
enum AtlasOverlay {
    CaptureKey { action: keybindings::Action },
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransitionSedimentSettlement {
    state: SandState,
    spawn_remainder: Duration,
    physics_remainder: Duration,
    added_grains: usize,
    skipped_physics_events: usize,
}

fn settle_transition_sediment_segment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    timing: RecoveryTiming,
) -> Result<TransitionSedimentSettlement, String> {
    let recovered =
        settle_transition_sediment(base_state, valid_category_ids, active_category_id, timing)?;
    Ok(TransitionSedimentSettlement {
        state: recovered.state,
        spawn_remainder: recovered.spawn_remainder,
        physics_remainder: recovered.physics_remainder,
        added_grains: recovered.added_grains,
        skipped_physics_events: recovered.skipped_physics_events,
    })
}

fn valid_category_ids_for_catalog(
    active_categories: impl IntoIterator<Item = Category>,
    archived_categories: &[Category],
) -> HashSet<u64> {
    let mut category_ids = active_categories
        .into_iter()
        .map(|category| category.id.0)
        .collect::<HashSet<_>>();
    category_ids.extend(archived_categories.iter().map(|category| category.id.0));
    category_ids
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReportLogEditState {
    session_id: usize,
    draft: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeMutation {
    SwitchLayer {
        category_id: CategoryId,
        description: String,
    },
    ClearAllSand,
    ClearDriftSand,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DetachedRuntimeCheckpoint {
    schema_version: u8,
    #[serde(default)]
    profile_id: Option<String>,
    detached_at_utc: DateTime<Utc>,
    simulation_time_utc: DateTime<Utc>,
    spawn_accumulator_nanos: u64,
    physics_accumulator_nanos: u64,
    active_category_id: u64,
    active_description: String,
    active_session_started_at_utc: Option<DateTime<Utc>>,
    sand_state: crate::sand::SandState,
    pending_mutations: Vec<serde_json::Value>,
    #[serde(default)]
    recovery_target_utc: Option<DateTime<Utc>>,
}

impl DetachedRuntimeCheckpoint {
    const VERSION: u8 = 2;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum RecoveredIntervalClass {
    Exact,
    Reconstructed,
}

impl RecoveredIntervalClass {
    fn label(self) -> &'static str {
        match self {
            Self::Exact => "EXACT",
            Self::Reconstructed => "RECONSTRUCTED",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum PostTargetClass {
    ProvisionalLiveTime,
}

impl PostTargetClass {
    fn label(self) -> &'static str {
        "PROVISIONAL LIVE TIME"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct RecoveryStatement {
    profile_id: String,
    checkpoint_captured_at_utc: DateTime<Utc>,
    checkpoint_simulation_at_utc: DateTime<Utc>,
    recovery_target_utc: DateTime<Utc>,
    reconstructed_duration_nanos: u64,
    recovered_interval_class: RecoveredIntervalClass,
    post_target_class: PostTargetClass,
    active_stable_id: Option<String>,
    active_category_id: u64,
    active_description: String,
    active_session_started_at_utc: DateTime<Utc>,
    cutoff_policy: String,
}

fn should_use_bounded_catchup(backlog: Duration) -> bool {
    backlog > Duration::from_secs(CATCHUP_SETTINGS.bounded_catchup_after_secs)
}

fn recovery_target_for_claim(
    persisted_target_utc: Option<DateTime<Utc>>,
    claim_time_utc: DateTime<Utc>,
) -> DateTime<Utc> {
    persisted_target_utc.unwrap_or(claim_time_utc)
}

fn initial_tui_stable_id_matches_start(
    active_stable_id: Option<&str>,
    started_at_utc: DateTime<Utc>,
) -> bool {
    let Some(stable_id) = active_stable_id else {
        return false;
    };
    let Some(rest) = stable_id.strip_prefix("tui-") else {
        return false;
    };
    let Some((timestamp, process_id)) = rest.rsplit_once('-') else {
        return false;
    };
    if process_id.parse::<u32>().is_err() {
        return false;
    }
    DateTime::parse_from_rfc3339(timestamp)
        .map(|value| value.with_timezone(&Utc) == started_at_utc)
        .unwrap_or(false)
}

fn repair_initial_checkpoint_simulation_boundary(
    checkpoint: &mut DetachedRuntimeCheckpoint,
    active_stable_id: Option<&str>,
) -> bool {
    let Some(started_at_utc) = checkpoint.active_session_started_at_utc else {
        return false;
    };
    if started_at_utc <= checkpoint.simulation_time_utc
        || started_at_utc > checkpoint.detached_at_utc
        || checkpoint.active_category_id != DRIFT_CATEGORY_ID.0
        || !initial_tui_stable_id_matches_start(active_stable_id, started_at_utc)
    {
        return false;
    }

    checkpoint.simulation_time_utc = started_at_utc;
    true
}

fn build_recovery_statement(
    checkpoint: &DetachedRuntimeCheckpoint,
    active_stable_id: Option<String>,
    target_utc: DateTime<Utc>,
) -> Result<RecoveryStatement, String> {
    if checkpoint.simulation_time_utc > checkpoint.detached_at_utc
        || checkpoint.detached_at_utc > target_utc
    {
        return Err("recovery statement timestamps are not monotonic".to_string());
    }
    let started_at_utc = checkpoint
        .active_session_started_at_utc
        .ok_or_else(|| "recovery statement has no active-session start".to_string())?;
    if started_at_utc > checkpoint.simulation_time_utc {
        return Err(
            "recovery statement active session starts after durable simulation time".to_string(),
        );
    }
    let reconstructed = (target_utc - checkpoint.simulation_time_utc)
        .to_std()
        .map_err(|error| format!("invalid recovery statement interval: {error}"))?;
    let reconstructed_duration_nanos = u64::try_from(reconstructed.as_nanos())
        .map_err(|_| "recovery statement interval exceeds the supported range".to_string())?;
    let recovered_interval_class = if reconstructed_duration_nanos == 0 {
        RecoveredIntervalClass::Exact
    } else {
        RecoveredIntervalClass::Reconstructed
    };
    Ok(RecoveryStatement {
        profile_id: checkpoint
            .profile_id
            .clone()
            .unwrap_or_else(crate::profile::profile_id),
        checkpoint_captured_at_utc: checkpoint.detached_at_utc,
        checkpoint_simulation_at_utc: checkpoint.simulation_time_utc,
        recovery_target_utc: target_utc,
        reconstructed_duration_nanos,
        recovered_interval_class,
        post_target_class: PostTargetClass::ProvisionalLiveTime,
        active_stable_id,
        active_category_id: checkpoint.active_category_id,
        active_description: checkpoint.active_description.clone(),
        active_session_started_at_utc: started_at_utc,
        cutoff_policy: "persisted target; no post-target time is counted as recovered".to_string(),
    })
}

fn clear_all_affected_days_for_interval(
    operation_day: NaiveDate,
    idle_reset: bool,
    previous_started_at_utc: DateTime<Utc>,
    applied_at_utc: DateTime<Utc>,
    previous_elapsed_seconds: usize,
    policy: OperationalDayPolicy,
) -> Result<BTreeSet<NaiveDate>, String> {
    let mut days = BTreeSet::from([operation_day]);
    if idle_reset {
        days.extend(
            temporal::allocate_operational_day_slices(
                previous_started_at_utc,
                applied_at_utc,
                previous_elapsed_seconds,
                policy,
            )?
            .into_iter()
            .map(|slice| slice.operational_day),
        );
    }
    Ok(days)
}

#[derive(Clone)]
struct SessionState {
    active_session_stable_id: Option<String>,
    active_session_started_at_utc: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingDayEndSnapshot {
    operational_day: NaiveDate,
    captured_at_utc: DateTime<Utc>,
    snapshot: SedimentSnapshot,
}

fn stage_pending_day_end_snapshot(
    pending: &mut Vec<PendingDayEndSnapshot>,
    operational_day: NaiveDate,
    captured_at_utc: DateTime<Utc>,
    state: SandState,
) -> Result<(), String> {
    let snapshot =
        SedimentSnapshot::day_end_checkpoint(operational_day.format("%Y-%m-%d").to_string(), state);
    if let Some(existing) = pending
        .iter()
        .find(|pending| pending.operational_day == operational_day)
    {
        if existing.snapshot != snapshot || existing.captured_at_utc != captured_at_utc {
            return Err(format!(
                "operational-day {operational_day} produced conflicting in-memory day-end snapshots"
            ));
        }
        return Ok(());
    }
    pending.push(PendingDayEndSnapshot {
        operational_day,
        captured_at_utc,
        snapshot,
    });
    Ok(())
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
    modal_active_description_dirty: bool,
    modal_editing_category_metadata: bool,
    category_tags: storage::CategoryTagsState,
    modal_tag_index: Option<usize>,
    report_selected_index: usize,
    report_period: ReportPeriod,
    report_period_offset: usize,
    report_logs_category_id: Option<CategoryId>,
    report_log_selected_index: usize,
    report_log_edit: Option<ReportLogEditState>,
    report_snapshot_end_day: Option<String>,
    report_snapshot_artifact: Option<SedimentSnapshot>,
    report_snapshot_preview_key: Option<String>,
    report_snapshot_preview_lines: Option<Vec<ratatui::text::Line<'static>>>,
    pending_day_end_snapshots: Vec<PendingDayEndSnapshot>,
    simulation: SimulationState,
    detach_requested: bool,
    keymap: keybindings::Keymap,
    runtime_settings: RuntimeSettings,
    keymap_error: Option<String>,
    show_command_palette: bool,
    command_palette_query: String,
    command_palette_feedback: Option<String>,
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
    checkpoint_recovery_payload: Option<DetachedRuntimeCheckpoint>,
    recovery_statement: Option<RecoveryStatement>,
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
        } = loaded;
        let keymap_error = None;

        let mut tracker = TimeTracker::new();
        let database_path = sqlite::resolve_runtime_database()?;
        let state = sqlite::load_tui_state(&database_path)?;
        let sqlite_database_path = Some(database_path);
        let loaded_categories = state.loaded_categories;
        let loaded_sessions = state.loaded_sessions;
        let mut category_tags = state.category_tags;
        let archived_categories = state.archived_categories;
        let sqlite_active_session = state.active_session;
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );

        let valid_category_ids =
            valid_category_ids_for_catalog(tracker.categories_for_storage(), &archived_categories);
        category_tags
            .tags_by_category
            .retain(|category_id, _| valid_category_ids.contains(category_id));

        let mut app = Self {
            time_tracker: tracker,
            sand_engine: SandEngine::new(width, height),
            session: SessionState {
                active_session_stable_id: None,
                active_session_started_at_utc: None,
            },
            ui_mode: UiMode::Main,
            selected_index: 0,
            new_category_name: String::new(),
            color_index: 0,
            modal_description: String::new(),
            modal_active_description_dirty: false,
            modal_editing_category_metadata: false,
            category_tags,
            modal_tag_index: None,
            report_selected_index: 0,
            report_period: ReportPeriod::Today,
            report_period_offset: 0,
            report_logs_category_id: None,
            report_log_selected_index: 0,
            report_log_edit: None,
            report_snapshot_end_day: None,
            report_snapshot_artifact: None,
            report_snapshot_preview_key: None,
            report_snapshot_preview_lines: None,
            pending_day_end_snapshots: Vec::new(),
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
            },
            detach_requested: false,
            keymap,
            runtime_settings,
            keymap_error,
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_feedback: None,
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
            checkpoint_recovery_payload: None,
            recovery_statement: None,
            persistence_recovery: None,
            recovery_exit_requested: false,
            recovery_exit_error: None,
        };

        app.persist_category_tags();

        let had_sqlite_active_session = sqlite_active_session.is_some();
        let mut initial_checkpoint_published = false;
        if !app.restore_from_detached_checkpoint() && !app.has_persistence_recovery() {
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
                app.time_tracker.set_active_description(active.description);
                app.session.active_session_stable_id = Some(active.stable_id);
                app.begin_active_session_at(active.started_at_utc, false)?;
            } else {
                app.begin_active_session_now();
            }
            app.restore_sand_state();
            if !had_sqlite_active_session
                && app.sqlite_database_path.is_some()
                && !app.has_persistence_recovery()
            {
                initial_checkpoint_published = app.persist_initial_active_generation();
            }
        }

        app.commit_checkpoint_recovery_if_ready();
        if !app.has_persistence_recovery() && !initial_checkpoint_published {
            app.persist_runtime_checkpoint();
        }
        if let Some(recovery) = app.persistence_recovery.take() {
            return Err(recovery.failure.summary());
        }

        Ok(app)
    }

    fn persist_initial_active_generation(&mut self) -> bool {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return true;
        };
        let category_id = self.time_tracker.active_category_id();
        let description = self.time_tracker.active_description().to_string();
        let Some(started_at_utc) = self.session.active_session_started_at_utc else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err("initial active generation has no UTC start timestamp".to_string()),
            );
            return false;
        };
        let stable_id = sqlite::initial_tui_active_stable_id(started_at_utc);
        self.session.active_session_stable_id = Some(stable_id.clone());
        let checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.session.active_session_stable_id = None;
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return false;
            }
        };
        let result = sqlite::start_tui_active_session_with_checkpoint(
            &database_path,
            sqlite::TuiInitialActiveGenerationRequest {
                active_stable_id: &stable_id,
                category_id,
                description: &description,
                started_at_utc,
                detached_at_utc: checkpoint.detached_at_utc,
                simulation_time_utc: checkpoint.simulation_time_utc,
                checkpoint: &checkpoint,
            },
        );
        if self
            .record_storage_result_for(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                result,
            )
            .is_none()
        {
            self.session.active_session_stable_id = None;
            return false;
        }
        true
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

    fn open_modal(&mut self) {
        self.ui_mode = UiMode::CategoryModal;
        self.selected_index = self.time_tracker.active_category_index().unwrap_or(0);
        self.new_category_name = String::new();
        self.color_index = 0;
        self.modal_active_description_dirty = false;
        self.sync_modal_description_from_selection();
        self.render_needed = true;
    }

    fn persist_modal_active_description(&mut self) -> bool {
        if !self.modal_active_description_dirty {
            return true;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveDescription,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return false;
        };
        let Some(stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveDescription,
                RecoveryAction::ReloadAuthority,
                Err("active session has no stable identity".to_string()),
            );
            return false;
        };
        let description = self.time_tracker.active_description().to_string();
        let result =
            sqlite::update_tui_active_description(&database_path, &stable_id, &description);
        if self
            .record_storage_result_for(
                PersistenceOperation::ActiveDescription,
                RecoveryAction::ReloadAuthority,
                result,
            )
            .is_none()
        {
            return false;
        }
        self.modal_active_description_dirty = false;
        self.refresh_active_runtime_checkpoint();
        !self.has_persistence_recovery()
    }

    fn close_modal(&mut self) {
        if !self.persist_modal_active_description() {
            self.render_needed = true;
            return;
        }
        self.ui_mode = UiMode::Main;
        self.modal_description = String::new();
        self.modal_editing_category_metadata = false;
        self.modal_tag_index = None;
        self.render_needed = true;
    }

    fn open_report_modal(&mut self) {
        self.ui_mode = UiMode::BalanceModal;
        self.report_selected_index = 0;
        self.report_period = ReportPeriod::Today;
        self.report_period_offset = 0;
        self.report_logs_category_id = None;
        self.report_log_selected_index = 0;
        self.report_log_edit = None;
        self.report_snapshot_end_day = None;
        self.report_snapshot_artifact = None;
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_lines = None;
        self.focus_none_report_row();
        self.render_needed = true;
    }

    fn close_report_modal(&mut self) {
        self.ui_mode = UiMode::Main;
        self.report_logs_category_id = None;
        self.report_log_selected_index = 0;
        self.report_log_edit = None;
        self.report_snapshot_end_day = None;
        self.report_snapshot_artifact = None;
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_lines = None;
        self.render_needed = true;
    }

    fn in_category_modal(&self) -> bool {
        matches!(self.ui_mode, UiMode::CategoryModal)
    }

    fn in_balance_modal(&self) -> bool {
        matches!(self.ui_mode, UiMode::BalanceModal)
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

    fn atlas_items(&self) -> Vec<AtlasSelectable> {
        let mut items = vec![AtlasSelectable::WeekStartDay];
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

    pub(super) fn effective_keys_for_action(&self, action: Action) -> Vec<KeyBinding> {
        let mut keys = self.keymap.keys_for_action(action);
        keys.extend(self.keymap.mandatory_keys_for_action(action));
        keys.sort_by_key(|key| key.to_string());
        keys.dedup();
        keys
    }

    pub(super) fn keymap_state_for_action(&self, action: Action) -> ActionBindingState {
        self.keymap.action_state(action)
    }

    pub(super) fn contextual_labels_for_action(&self, action: Action) -> Vec<String> {
        self.keymap
            .aliases_for_action(action)
            .into_iter()
            .map(|alias| alias.display_label())
            .collect()
    }
    fn atlas_item_description(&self, item: AtlasSelectable) -> String {
        match item {
            AtlasSelectable::WeekStartDay => {
                "First weekday used by Week range in Balance pop-up.".to_string()
            }
            AtlasSelectable::Action(action) => action.description().to_string(),
        }
    }

    fn atlas_item_color(&self, item: AtlasSelectable) -> ratatui::style::Color {
        use ratatui::style::Color;

        match item {
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
            self.command_palette_feedback = None;
            self.command_palette_selected_index = 0;
            self.command_palette_scroll = 0;
            self.close_keybindings_modal();
        }
        self.render_needed = true;
    }

    fn close_command_palette(&mut self) {
        self.show_command_palette = false;
        self.command_palette_query.clear();
        self.command_palette_feedback = None;
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

    fn begin_active_session_now(&mut self) {
        let now = Utc::now();
        self.time_tracker.start_session();
        self.session.active_session_started_at_utc = Some(now);
        if now > self.simulation.simulation_time_utc {
            self.simulation.simulation_time_utc = now;
        }
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

    fn prepare_active_finish_for_exit(&mut self) -> Option<usize> {
        let finished_at_utc = Utc::now();
        if let Err(error) = self.settle_transition_boundary(finished_at_utc) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::FinishAndExit,
                Err(error),
            );
            return None;
        }
        self.end_active_session_at(finished_at_utc, SessionClockMode::LiveMonotonic)
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
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::ReloadAuthority,
                Err("SQLite runtime has no active stable identity to finish".to_string()),
            );
            return None;
        };
        let database_path = self.sqlite_database_path.clone()?;
        let operational_day = operational_day_key_for_utc(interval.ended_at_utc)
            .format("%Y-%m-%d")
            .to_string();
        let operation_id = transition_identity(
            "finish",
            &expected_stable_id,
            interval.ended_at_utc,
            "completed",
        )
        .operation_id;
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
        self.time_tracker.set_active_description(String::new());
        self.time_tracker.current_session_start = None;
        self.session.active_session_stable_id = None;
        self.session.active_session_started_at_utc = None;
        self.reload_sqlite_sessions();
        Some(elapsed)
    }

    fn switch_active_category_at(
        &mut self,
        category_id: CategoryId,
        next_description: String,
        switched_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> bool {
        if self.time_tracker.active_category_id() == category_id
            || self.time_tracker.category_by_id(category_id).is_none()
        {
            return false;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return false;
        };
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
        let identity = transition_identity(
            "switch",
            &expected_stable_id,
            interval.ended_at_utc,
            &category_id.0.to_string(),
        );
        let next_stable_id = identity.tui_active_stable_id();
        let result = sqlite::switch_tui_active_session(
            &database_path,
            &expected_stable_id,
            &identity.operation_id,
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
        if !self.time_tracker.set_active_category_by_id(category_id) {
            return false;
        }
        self.time_tracker.set_active_description(next_description);
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
        self.refresh_active_runtime_checkpoint();
        !self.has_persistence_recovery()
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
    }

    fn clear_all_effect(
        &self,
        applied_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Result<(BTreeSet<NaiveDate>, usize), String> {
        let interval = self.reconciled_active_interval(applied_at_utc, clock_mode)?;
        let previous_started_at_utc = self
            .session
            .active_session_started_at_utc
            .ok_or_else(|| "active session is missing its UTC start timestamp".to_string())?;
        let days = clear_all_affected_days_for_interval(
            operational_day_key_for_utc(applied_at_utc),
            is_drift_category_id(self.time_tracker.active_category_id()),
            previous_started_at_utc,
            interval.ended_at_utc,
            interval.elapsed_seconds,
            OperationalDayPolicy::from_config(self.runtime_settings.day_boundary),
        )?;
        Ok((days, interval.elapsed_seconds))
    }

    fn apply_clear_all_at(&mut self, applied_at_utc: DateTime<Utc>, clock_mode: SessionClockMode) {
        let (affected_days, _previous_elapsed_seconds) =
            match self.clear_all_effect(applied_at_utc, clock_mode) {
                Ok(effect) => effect,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::SandStateSave,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return;
                }
            };
        let previous_tracker = self.time_tracker.clone();
        let previous_session = self.session.clone();
        let previous_sand = self.sand_engine.snapshot_state();
        let rollback = |app: &mut Self| {
            app.time_tracker = previous_tracker.clone();
            app.session = previous_session.clone();
            app.sand_engine
                .restore_state(
                    &previous_sand,
                    &app.time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(app.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                )
                .expect("captured rollback sediment must remain valid");
        };

        let idle_reset = is_drift_category_id(self.time_tracker.active_category_id());
        self.sand_engine.clear();
        if idle_reset && let Err(error) = self.begin_transition_session(applied_at_utc, clock_mode)
        {
            rollback(self);
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }

        let checkpoint = match self.build_runtime_checkpoint() {
            Ok(value) => value,
            Err(error) => {
                rollback(self);
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        };
        let Some(database_path) = self.sqlite_database_path.clone() else {
            rollback(self);
            self.record_storage_result_for::<()>(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let Some(expected_stable_id) = previous_session.active_session_stable_id.clone() else {
            rollback(self);
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                Err("SQLite clear-all has no active stable identity".to_string()),
            );
            return;
        };
        let resulting_stable_id = if idle_reset {
            transition_identity(
                "clear-all",
                &expected_stable_id,
                applied_at_utc,
                "idle-reset",
            )
            .tui_active_stable_id()
        } else {
            expected_stable_id.clone()
        };
        let Some(resulting_started_at_utc) = self.session.active_session_started_at_utc else {
            rollback(self);
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                Err("SQLite clear-all has no resulting active start timestamp".to_string()),
            );
            return;
        };
        let daily_updates = affected_days
            .iter()
            .map(|day| {
                (
                    day.format("%Y-%m-%d").to_string(),
                    self.daily_contribution_from_time_log(*day),
                )
            })
            .collect::<Vec<_>>();
        let result = sqlite::clear_tui_state(
            &database_path,
            sqlite::TuiClearAllStateRequest {
                expected_active_stable_id: &expected_stable_id,
                resulting_active_stable_id: &resulting_stable_id,
                resulting_started_at_utc,
                state: &checkpoint.sand_state,
                daily_updates: &daily_updates,
                detached_at_utc: checkpoint.detached_at_utc,
                simulation_time_utc: checkpoint.simulation_time_utc,
                checkpoint: &checkpoint,
            },
        );
        if self
            .record_storage_result_for(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                result,
            )
            .is_none()
        {
            rollback(self);
            return;
        }
        self.session.active_session_stable_id = Some(resulting_stable_id);
    }

    fn settle_simulation_segment_to(&mut self, target_utc: DateTime<Utc>) -> Result<(), String> {
        if target_utc <= self.simulation.simulation_time_utc {
            return Ok(());
        }
        let elapsed = (target_utc - self.simulation.simulation_time_utc)
            .to_std()
            .map_err(|error| error.to_string())?;
        let mut valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        valid_category_ids.insert(DRIFT_CATEGORY_ID);
        let settlement = settle_transition_sediment_segment(
            &self.sand_engine.snapshot_state(),
            &valid_category_ids,
            self.time_tracker.active_category_id(),
            RecoveryTiming {
                elapsed,
                spawn_accumulator: self.simulation.spawn_accumulator,
                physics_accumulator: self.simulation.physics_accumulator,
                spawn_period: Duration::from_millis(TIME_SETTINGS.tick_ms),
                physics_period: Duration::from_millis(TIME_SETTINGS.physics_ms),
            },
        )?;
        self.sand_engine
            .restore_state(&settlement.state, &valid_category_ids)?;
        self.simulation.spawn_accumulator = settlement.spawn_remainder;
        self.simulation.physics_accumulator = settlement.physics_remainder;
        self.simulation.simulation_time_utc = target_utc;
        if settlement.added_grains > 0 || settlement.skipped_physics_events > 0 {
            self.render_needed = true;
        }
        Ok(())
    }

    fn settle_transition_boundary(&mut self, boundary_utc: DateTime<Utc>) -> Result<(), String> {
        self.settle_simulation_segment_to(boundary_utc)
    }

    fn apply_runtime_mutation(&mut self, mutation: RuntimeMutation) {
        let scheduled_at_utc = Utc::now();
        let clock_mode = if self.is_catching_up() {
            SessionClockMode::HistoricalWall
        } else {
            SessionClockMode::LiveMonotonic
        };

        if let Err(error) = self.settle_transition_boundary(scheduled_at_utc) {
            if !self.has_persistence_recovery() {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::SandStateSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
            }
        } else {
            self.apply_mutation_at(mutation, scheduled_at_utc, clock_mode);
        }
        self.render_needed = true;
    }

    fn apply_mutation_at(
        &mut self,
        mutation: RuntimeMutation,
        scheduled_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) {
        match mutation {
            RuntimeMutation::SwitchLayer {
                category_id,
                description,
            } => {
                self.apply_switch_layer_at(category_id, description, scheduled_at_utc, clock_mode);
            }
            RuntimeMutation::ClearAllSand => {
                self.apply_clear_all_at(scheduled_at_utc, clock_mode);
            }
            RuntimeMutation::ClearDriftSand => {
                self.sand_engine.clear_category(DRIFT_CATEGORY_ID);
                self.persist_sand_state();
                self.persist_daily_sand_snapshot();
            }
        }
    }

    fn apply_switch_layer_at(
        &mut self,
        category_id: CategoryId,
        description: String,
        scheduled_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) {
        self.switch_active_category_at(category_id, description, scheduled_at_utc, clock_mode);
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

        let target_utc = Utc::now();
        let backlog = self.simulation_backlog_duration_at(target_utc);

        if should_use_bounded_catchup(backlog) {
            if let Err(error) = self.settle_transition_boundary(target_utc) {
                if !self.has_persistence_recovery() {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::SandStateSave,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                }
            } else {
                self.simulation.catchup_cadence_accumulator = Duration::ZERO;
                self.render_needed = true;
            }
        } else if backlog.is_zero() {
            if let Err(error) = self.advance_simulation_by(Duration::ZERO, tick_rate, physics_rate)
            {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::DailySnapshotSave,
                    RecoveryAction::FlushCurrentState,
                    Err(error),
                );
                return;
            }
            self.simulation.catchup_cadence_accumulator =
                self.simulation.catchup_cadence_accumulator.min(cadence);
        } else {
            while self.simulation.catchup_cadence_accumulator >= cadence {
                self.simulation.catchup_cadence_accumulator = self
                    .simulation
                    .catchup_cadence_accumulator
                    .saturating_sub(cadence);

                let step_target_utc = Utc::now();
                let step_backlog = self.simulation_backlog_duration_at(step_target_utc);
                if step_backlog.is_zero() {
                    if let Err(error) =
                        self.advance_simulation_by(Duration::ZERO, tick_rate, physics_rate)
                    {
                        self.record_storage_result_for::<()>(
                            PersistenceOperation::DailySnapshotSave,
                            RecoveryAction::FlushCurrentState,
                            Err(error),
                        );
                        return;
                    }
                    break;
                }

                let speed = self.current_catchup_multiplier(step_backlog);
                let step_budget = cadence.saturating_mul(speed);
                let advance_by = step_backlog.min(step_budget);
                if advance_by.is_zero() {
                    break;
                }

                if let Err(error) = self.advance_simulation_by(advance_by, tick_rate, physics_rate)
                {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::DailySnapshotSave,
                        RecoveryAction::FlushCurrentState,
                        Err(error),
                    );
                    return;
                }
                self.render_needed = true;
            }
        }

        if let Err(error) = self.persist_pending_day_end_snapshots() {
            self.record_storage_result_for::<()>(
                PersistenceOperation::DailySnapshotSave,
                RecoveryAction::FlushCurrentState,
                Err(error),
            );
            return;
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

        let visual_cadence = Duration::from_millis(CATCHUP_SETTINGS.visual_refresh_ms);

        let should_recreate = self
            .simulation
            .catchup_visual_engine
            .as_ref()
            .map(|engine| engine.cell_width != cell_width || engine.cell_height != cell_height)
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
                engine
                    .restore_state(&projected_state, &valid_category_ids)
                    .ok()?;
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
            pending_grains: state.pending_grains.clone(),
            pending_runs: state.pending_runs.clone(),
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
    ) -> Result<(), String> {
        let target_time = self.simulation.simulation_time_utc
            + ChronoDuration::from_std(delta).unwrap_or(ChronoDuration::zero());

        if delta.is_zero() {
            return Ok(());
        }

        while self.simulation.simulation_time_utc < target_time {
            let config = crate::domain::day_boundary_config();
            let (ending_day, next_boundary) = temporal::next_operational_day_boundary_after(
                self.simulation.simulation_time_utc,
                &config,
            )?;
            let segment_target = target_time.min(next_boundary);
            let remaining = (segment_target - self.simulation.simulation_time_utc)
                .to_std()
                .map_err(|error| error.to_string())?;

            self.process_simulation_delta(remaining, tick_rate, physics_rate);
            self.simulation.simulation_time_utc = segment_target;

            if segment_target == next_boundary {
                stage_pending_day_end_snapshot(
                    &mut self.pending_day_end_snapshots,
                    ending_day,
                    next_boundary,
                    self.sand_engine.snapshot_state(),
                )?;
            }
        }
        Ok(())
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
    }

    fn catchup_progress_ratio(&mut self) -> Option<f64> {
        let target_utc = Utc::now();
        let backlog = self.simulation_backlog_duration_at(target_utc);

        if backlog <= self.catchup_visibility_threshold()
            && let Some(until) = self.simulation.catchup_gauge_hold_until
        {
            if Instant::now() < until {
                return Some(1.0);
            }
            self.simulation.catchup_gauge_hold_until = None;
        }

        if backlog <= self.catchup_visibility_threshold() {
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

    fn prepare_detach_boundary(&mut self) -> Result<(), String> {
        self.settle_transition_boundary(Utc::now())
    }

    fn build_runtime_checkpoint(&self) -> Result<DetachedRuntimeCheckpoint, String> {
        if self.checkpoint_recovery_active {
            return Err("checkpoint recovery is still active".to_string());
        }
        let active_category_id = self.time_tracker.active_category_id();
        let active_description = self.time_tracker.active_description().to_string();
        let spawn_accumulator_nanos =
            u64::try_from(self.simulation.spawn_accumulator.as_nanos())
                .map_err(|_| "spawn accumulator exceeds checkpoint range".to_string())?;
        let physics_accumulator_nanos =
            u64::try_from(self.simulation.physics_accumulator.as_nanos())
                .map_err(|_| "physics accumulator exceeds checkpoint range".to_string())?;

        Ok(DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            profile_id: Some(crate::profile::profile_id()),
            detached_at_utc: Utc::now(),
            simulation_time_utc: self.simulation.simulation_time_utc,
            spawn_accumulator_nanos,
            physics_accumulator_nanos,
            active_category_id: active_category_id.0,
            active_description,
            active_session_started_at_utc: self.session.active_session_started_at_utc,
            sand_state: self.sand_engine.snapshot_state(),
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
        })
    }

    pub(super) fn try_write_runtime_checkpoint(&self) -> Result<(), String> {
        let checkpoint = self.build_runtime_checkpoint()?;
        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
        let expected_stable_id = self
            .session
            .active_session_stable_id
            .as_deref()
            .ok_or_else(|| {
                "SQLite runtime has no active stable identity to checkpoint".to_string()
            })?;
        sqlite::save_tui_checkpoint(
            &database_path,
            expected_stable_id,
            checkpoint.detached_at_utc,
            checkpoint.simulation_time_utc,
            &checkpoint,
        )
    }

    fn try_emergency_runtime_checkpoint(&self) -> Result<(), String> {
        self.try_write_runtime_checkpoint()
    }

    fn persist_runtime_checkpoint(&mut self) {
        if self.checkpoint_recovery_active {
            return;
        }
        let result = self.try_write_runtime_checkpoint();
        self.record_storage_result_for(
            PersistenceOperation::CheckpointSave,
            RecoveryAction::DetachAndExit,
            result,
        );
    }

    fn refresh_active_runtime_checkpoint(&mut self) {
        if self.session.active_session_started_at_utc.is_some() && !self.has_persistence_recovery()
        {
            self.persist_runtime_checkpoint();
        }
    }

    fn clear_detached_checkpoint(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return;
        };
        let result = sqlite::clear_tui_checkpoint(&database_path);
        self.record_storage_result_for(
            PersistenceOperation::CheckpointClear,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    fn restore_from_detached_checkpoint(&mut self) -> bool {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return false;
        };
        let mut checkpoint: DetachedRuntimeCheckpoint =
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
                    claimed.payload
                }
                Ok(None) => return false,
                Err(error) => {
                    self.record_storage_result::<()>(Err(error));
                    return false;
                }
            };

        if let Err(error) = crate::profile::validate_artifact_profile(
            checkpoint.profile_id.as_deref(),
            "detached runtime checkpoint",
        ) {
            if let Some(database_path) = self.sqlite_database_path.clone() {
                let _ = sqlite::quarantine_tui_checkpoint(&database_path);
            }
            self.record_storage_result::<()>(Err(error));
            return false;
        }
        checkpoint.profile_id = Some(crate::profile::profile_id());

        self.checkpoint_recovery_active = true;

        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION {
            if let Some(database_path) = self.sqlite_database_path.clone() {
                let _ = sqlite::quarantine_tui_checkpoint(&database_path);
            }
            self.record_storage_result::<()>(Err(format!(
                "unsupported detached checkpoint schema {}",
                checkpoint.schema_version
            )));
            return false;
        }
        if !checkpoint.pending_mutations.is_empty() {
            self.record_storage_result::<()>(Err(
                "detached checkpoint contains queued mutations that cannot be recovered without a stable receipt identity; evidence retained"
                    .to_string(),
            ));
            return false;
        }

        repair_initial_checkpoint_simulation_boundary(
            &mut checkpoint,
            self.session.active_session_stable_id.as_deref(),
        );

        let now_utc = Utc::now();
        let target_utc = recovery_target_for_claim(checkpoint.recovery_target_utc, now_utc);
        if target_utc > now_utc {
            self.record_storage_result::<()>(Err(format!(
                "detached recovery target {target_utc} is in the future"
            )));
            return false;
        }
        if checkpoint.simulation_time_utc > checkpoint.detached_at_utc
            || checkpoint.detached_at_utc > target_utc
        {
            self.record_storage_result::<()>(Err(
                "detached checkpoint timestamps are not monotonic".to_string(),
            ));
            return false;
        }

        checkpoint.schema_version = DetachedRuntimeCheckpoint::VERSION;
        checkpoint.recovery_target_utc = Some(target_utc);

        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result::<()>(Err("SQLite authority is unavailable".to_string()));
            return false;
        };
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result::<()>(Err(
                "SQLite recovery checkpoint has no stable identity".to_string(),
            ));
            return false;
        };
        let claim_persisted = self
            .record_storage_result_for(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                sqlite::replace_tui_recovering_checkpoint(
                    &database_path,
                    &expected_stable_id,
                    &checkpoint,
                ),
            )
            .is_some();
        if !claim_persisted {
            return false;
        }

        let active_category_id = CategoryId::new(checkpoint.active_category_id);
        if self
            .time_tracker
            .category_by_id(active_category_id)
            .is_none()
        {
            self.record_storage_result::<()>(Err(format!(
                "detached checkpoint references unavailable active category {}",
                checkpoint.active_category_id
            )));
            return false;
        }
        let Some(started_at_utc) = checkpoint.active_session_started_at_utc else {
            self.record_storage_result::<()>(Err(
                "detached checkpoint has no active-session start timestamp".to_string(),
            ));
            return false;
        };
        if started_at_utc > target_utc {
            self.record_storage_result::<()>(Err(
                "detached checkpoint active session starts after its recovery target".to_string(),
            ));
            return false;
        }

        let recovery_statement = match build_recovery_statement(
            &checkpoint,
            self.session.active_session_stable_id.clone(),
            target_utc,
        ) {
            Ok(statement) => statement,
            Err(error) => {
                self.record_storage_result::<()>(Err(error));
                return false;
            }
        };

        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        let elapsed = match (target_utc - checkpoint.simulation_time_utc).to_std() {
            Ok(elapsed) => elapsed,
            Err(error) => {
                self.record_storage_result::<()>(Err(format!(
                    "invalid detached recovery interval: {error}"
                )));
                return false;
            }
        };
        let recovered = match recover_detached_sediment(
            &checkpoint.sand_state,
            &valid_category_ids,
            active_category_id,
            RecoveryTiming {
                elapsed,
                spawn_accumulator: Duration::from_nanos(checkpoint.spawn_accumulator_nanos),
                physics_accumulator: Duration::from_nanos(checkpoint.physics_accumulator_nanos),
                spawn_period: Duration::from_millis(TIME_SETTINGS.tick_ms),
                physics_period: Duration::from_millis(TIME_SETTINGS.physics_ms),
            },
        ) {
            Ok(recovered) => recovered,
            Err(error) => {
                self.record_storage_result::<()>(Err(error));
                return false;
            }
        };

        if let Err(error) = self
            .sand_engine
            .restore_state(&recovered.state, &valid_category_ids)
        {
            self.record_storage_result::<()>(Err(error));
            return false;
        }
        if !self
            .time_tracker
            .set_active_category_by_id(active_category_id)
        {
            self.record_storage_result::<()>(Err(
                "detached recovery could not select its active category".to_string(),
            ));
            return false;
        }
        self.time_tracker
            .set_active_description(checkpoint.active_description.clone());
        if let Err(error) = self.begin_active_session_at(started_at_utc, true) {
            self.record_storage_result::<()>(Err(error));
            return false;
        }

        self.simulation.simulation_time_utc = target_utc;
        self.simulation.spawn_accumulator = recovered.spawn_remainder;
        self.simulation.physics_accumulator = recovered.physics_remainder;
        self.simulation.catchup_cadence_accumulator = Duration::ZERO;
        self.simulation.catchup_visual_engine = None;
        self.simulation.catchup_progress_anchor = None;
        self.simulation.catchup_was_active = false;
        self.checkpoint_recovery_payload = Some(checkpoint);
        self.recovery_statement = Some(recovery_statement);
        true
    }

    fn commit_checkpoint_recovery_if_ready(&mut self) {
        if !self.checkpoint_recovery_active {
            return;
        }
        let Some(_checkpoint) = self.checkpoint_recovery_payload.clone() else {
            self.record_storage_result::<()>(Err(
                "checkpoint recovery payload is unavailable for commit".to_string(),
            ));
            return;
        };
        let state = self.sand_engine.snapshot_state();
        let operational_day_date = operational_day_key_for_utc(self.simulation.simulation_time_utc);
        let operational_day = operational_day_date.format("%Y-%m-%d").to_string();
        let Some(daily_contribution) = self.daily_contribution_from_time_log(operational_day_date)
        else {
            self.record_storage_result::<()>(Err(
                "checkpoint recovery produced no daily contribution for its active session"
                    .to_string(),
            ));
            return;
        };
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result::<()>(Err("SQLite authority is unavailable".to_string()));
            return;
        };
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result::<()>(Err(
                "SQLite recovery has no active stable identity to commit".to_string(),
            ));
            return;
        };
        if self
            .record_storage_result_for(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                sqlite::commit_tui_checkpoint_recovery(
                    &database_path,
                    &expected_stable_id,
                    &operational_day,
                    &state,
                    &daily_contribution,
                ),
            )
            .is_none()
        {
            return;
        }
        self.checkpoint_recovery_active = false;
        self.checkpoint_recovery_payload = None;
        self.reconcile_all_daily_contributions();
    }
}

fn run_application_loop(
    app: &mut App,
    terminal: &mut ManagedTerminal,
) -> Result<Option<String>, io::Error> {
    #[cfg(unix)]
    let command_server = crate::ipc::CommandServer::bind().map_err(io::Error::other)?;
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
            #[cfg(unix)]
            if let Err(error) =
                command_server.process_pending(|command| app.execute_command(command))
            {
                app.keymap_error = Some(format!("Remote control error: {error}"));
                app.render_needed = true;
            }

            if !app.has_persistence_recovery() {
                let now = Instant::now();
                let wall_delta = now.saturating_duration_since(last_simulation_update);
                last_simulation_update = now;
                app.advance_runtime(wall_delta, tick_rate, physics_rate);

                if last_save.elapsed() >= save_rate && !app.is_catching_up() {
                    app.persist_sessions();
                    if !app.has_persistence_recovery() {
                        app.persist_sand_state();
                    }
                    if !app.has_persistence_recovery() {
                        app.persist_daily_sand_snapshot();
                    }
                    if !app.has_persistence_recovery() {
                        app.persist_runtime_checkpoint();
                    }
                    last_save = Instant::now();
                }

                app.refresh_keymap_if_changed();
            }

            if last_render.elapsed() >= render_rate && app.render_needed {
                terminal_lifecycle::maybe_inject_runtime_io_fault("draw")?;
                terminal.draw(|frame| {
                    app.draw_frame(frame);
                })?;
                app.render_needed = false;
                last_render = Instant::now();
            }

            terminal_lifecycle::maybe_inject_runtime_io_fault("poll")?;
            if event::poll(Duration::from_millis(RUNTIME_LOOP_SETTINGS.input_poll_ms))? {
                terminal_lifecycle::maybe_inject_runtime_io_fault("read")?;
                if let Event::Key(key) = event::read()? {
                    if app.handle_key(key) {
                        break;
                    }
                    if app.detach_requested {
                        break;
                    }
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

        if app.detach_requested {
            if let Err(error) = app.prepare_detach_boundary()
                && !app.has_persistence_recovery()
            {
                app.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::DetachAndExit,
                    Err(error),
                );
            }
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
                app.persist_runtime_checkpoint();
            }
            if app.has_persistence_recovery() {
                app.promote_recovery_action(RecoveryAction::DetachAndExit);
                app.detach_requested = false;
                continue 'runtime;
            }
        } else {
            app.prepare_active_finish_for_exit();
            if !app.has_persistence_recovery() {
                app.persist_sessions();
            }
            if !app.has_persistence_recovery() {
                app.persist_categories();
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

    Ok(runtime_error)
}

pub fn run_ui(loaded: keybindings::LoadedKeybindings) -> Result<(), io::Error> {
    let (width, height) = crossterm::terminal::size()?;
    let mut app = App::new(width, height, loaded).map_err(io::Error::other)?;
    let mut terminal_session = TerminalSession::enter()?;
    terminal_lifecycle::maybe_inject_runtime_panic();

    match run_application_loop(&mut app, terminal_session.terminal_mut()) {
        Ok(application_error) => {
            let cleanup_result = terminal_session.restore();
            terminal_lifecycle::finish_normal_run(application_error, cleanup_result)
        }
        Err(primary) => {
            let checkpoint_result = app.try_emergency_runtime_checkpoint();
            let cleanup_result = terminal_session.restore();
            Err(terminal_lifecycle::compose_runtime_failure(
                primary,
                checkpoint_result,
                cleanup_result,
            ))
        }
    }
}

#[cfg(test)]
mod recovery_statement_tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};

    use super::{
        DetachedRuntimeCheckpoint, PostTargetClass, RecoveredIntervalClass,
        build_recovery_statement, recovery_target_for_claim,
        repair_initial_checkpoint_simulation_boundary, should_use_bounded_catchup,
    };
    use crate::sand::SandState;

    fn timestamp(second: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 3, 18, 0, second).unwrap()
    }

    fn checkpoint(simulation_second: u32, capture_second: u32) -> DetachedRuntimeCheckpoint {
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            profile_id: Some(crate::profile::profile_id()),
            detached_at_utc: timestamp(capture_second),
            simulation_time_utc: timestamp(simulation_second),
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: 1,
            active_description: "Focused".to_string(),
            active_session_started_at_utc: Some(timestamp(0)),
            sand_state: SandState {
                version: SandState::VERSION,
                grid_width: 2,
                grid_height: 2,
                grains: Vec::new(),
                frame_count: 0,
                sweep_left_to_right: true,
                rng_state: 1,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
        }
    }

    #[test]
    fn exact_and_reconstructed_statements_are_distinct() {
        let exact_checkpoint = checkpoint(2, 2);
        let exact =
            build_recovery_statement(&exact_checkpoint, Some("stable".to_string()), timestamp(2))
                .unwrap();
        assert_eq!(exact.reconstructed_duration_nanos, 0);
        assert_eq!(
            exact.recovered_interval_class,
            RecoveredIntervalClass::Exact
        );
        assert_eq!(
            exact.post_target_class,
            PostTargetClass::ProvisionalLiveTime
        );

        let reconstructed_checkpoint = checkpoint(2, 3);
        let reconstructed = build_recovery_statement(
            &reconstructed_checkpoint,
            Some("stable".to_string()),
            timestamp(7),
        )
        .unwrap();
        assert_eq!(reconstructed.reconstructed_duration_nanos, 5_000_000_000);
        assert_eq!(
            reconstructed.recovered_interval_class,
            RecoveredIntervalClass::Reconstructed
        );
    }

    #[test]
    fn long_live_backlog_uses_bounded_settlement_after_eight_seconds() {
        assert!(!should_use_bounded_catchup(Duration::from_secs(8)));
        assert!(should_use_bounded_catchup(Duration::from_secs(9)));
        assert!(should_use_bounded_catchup(Duration::from_secs(60 * 60)));
    }

    #[test]
    fn persisted_target_is_reused_after_wall_time_advances() {
        let persisted = timestamp(5);
        assert_eq!(
            recovery_target_for_claim(Some(persisted), timestamp(30)),
            persisted
        );
        assert_eq!(
            recovery_target_for_claim(None, timestamp(30)),
            timestamp(30)
        );
    }

    #[test]
    fn initial_tui_checkpoint_repairs_bootstrap_start_after_simulation_boundary() {
        let simulation = timestamp(0);
        let started = simulation + chrono::Duration::milliseconds(8);
        let mut checkpoint = checkpoint(0, 1);
        checkpoint.active_category_id = crate::domain::DRIFT_CATEGORY_ID.0;
        checkpoint.active_description.clear();
        checkpoint.active_session_started_at_utc = Some(started);
        let stable_id = format!(
            "tui-{}-42",
            started.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        );

        assert!(repair_initial_checkpoint_simulation_boundary(
            &mut checkpoint,
            Some(&stable_id)
        ));
        assert_eq!(checkpoint.simulation_time_utc, started);
        assert!(build_recovery_statement(&checkpoint, Some(stable_id), timestamp(2)).is_ok());
    }

    #[test]
    fn non_initial_checkpoint_start_after_simulation_still_fails_closed() {
        let mut checkpoint = checkpoint(0, 1);
        checkpoint.active_session_started_at_utc = Some(timestamp(1));

        assert!(!repair_initial_checkpoint_simulation_boundary(
            &mut checkpoint,
            Some("tui-active:switch")
        ));
        assert!(
            build_recovery_statement(&checkpoint, None, timestamp(2))
                .unwrap_err()
                .contains("starts after durable simulation time")
        );
    }

    #[test]
    fn non_monotonic_statement_fails_closed() {
        let invalid = checkpoint(4, 3);
        assert!(
            build_recovery_statement(&invalid, None, timestamp(5))
                .unwrap_err()
                .contains("not monotonic")
        );

        let mut invalid_start = checkpoint(2, 3);
        invalid_start.active_session_started_at_utc = Some(timestamp(4));
        assert!(
            build_recovery_statement(&invalid_start, None, timestamp(5))
                .unwrap_err()
                .contains("starts after durable simulation time")
        );
    }
}

#[cfg(test)]
mod transition_edge_tests {
    use std::{collections::HashSet, time::Duration};

    use super::{RecoveryTiming, settle_transition_sediment_segment};
    use crate::{
        domain::CategoryId,
        sand::{SandState, SandStateGrain},
    };

    fn categories() -> HashSet<CategoryId> {
        HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)])
    }

    fn empty_state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: Vec::new(),
            frame_count: 7,
            sweep_left_to_right: true,
            rng_state: 11,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        }
    }

    #[test]
    fn exact_boundary_grain_belongs_to_outgoing_category() {
        let outgoing = settle_transition_sediment_segment(
            &empty_state(),
            &categories(),
            CategoryId::new(1),
            RecoveryTiming {
                elapsed: Duration::from_millis(100),
                spawn_accumulator: Duration::from_millis(900),
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();
        assert_eq!(outgoing.added_grains, 1);
        assert_eq!(outgoing.spawn_remainder, Duration::ZERO);
        assert_eq!(outgoing.state.pending_runs.len(), 1);
        assert_eq!(outgoing.state.pending_runs[0].category_id, 1);
        assert_eq!(outgoing.state.pending_runs[0].count, 1);

        let resulting = settle_transition_sediment_segment(
            &outgoing.state,
            &categories(),
            CategoryId::new(2),
            RecoveryTiming {
                elapsed: Duration::from_secs(1),
                spawn_accumulator: outgoing.spawn_remainder,
                physics_accumulator: outgoing.physics_remainder,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();
        assert_eq!(resulting.state.pending_runs.len(), 2);
        assert_eq!(resulting.state.pending_runs[0].category_id, 1);
        assert_eq!(resulting.state.pending_runs[0].count, 1);
        assert_eq!(resulting.state.pending_runs[1].category_id, 2);
        assert_eq!(resulting.state.pending_runs[1].count, 1);
    }

    #[test]
    fn cleared_pre_boundary_mass_cannot_reappear_after_clear() {
        let settled = settle_transition_sediment_segment(
            &empty_state(),
            &categories(),
            CategoryId::new(1),
            RecoveryTiming {
                elapsed: Duration::from_millis(100),
                spawn_accumulator: Duration::from_millis(900),
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();
        assert_eq!(settled.added_grains, 1);

        let mut cleared = settled.state;
        cleared.grains.clear();
        cleared.pending_runs.clear();
        let before_next_tick = settle_transition_sediment_segment(
            &cleared,
            &categories(),
            CategoryId::new(2),
            RecoveryTiming {
                elapsed: Duration::from_millis(999),
                spawn_accumulator: settled.spawn_remainder,
                physics_accumulator: settled.physics_remainder,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();
        assert_eq!(before_next_tick.added_grains, 0);
        assert!(before_next_tick.state.grains.is_empty());
        assert!(before_next_tick.state.pending_runs.is_empty());
    }

    #[test]
    fn large_transition_gap_is_bounded_and_preserves_topology() {
        let mut state = empty_state();
        state.grains.push(SandStateGrain {
            x: 0,
            y: 1,
            category_id: 1,
        });
        let settled = settle_transition_sediment_segment(
            &state,
            &categories(),
            CategoryId::new(2),
            RecoveryTiming {
                elapsed: Duration::from_secs(1_000_000_000),
                spawn_accumulator: Duration::ZERO,
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
        .unwrap();
        assert_eq!(settled.added_grains, 1_000_000_000);
        assert_eq!(settled.state.grains, state.grains);
        assert_eq!(settled.state.pending_runs.len(), 1);
        assert_eq!(settled.state.pending_runs[0].category_id, 2);
        assert_eq!(settled.state.pending_runs[0].count, 1_000_000_000);
    }
}

#[cfg(test)]
mod category_catalog_tests {
    use super::valid_category_ids_for_catalog;
    use crate::domain::{Category, CategoryId, DRIFT_CATEGORY_ID};
    use ratatui::style::Color;

    fn category(id: u64, name: &str) -> Category {
        Category {
            id: CategoryId::new(id),
            name: name.to_string(),
            color: Color::White,
            description: String::new(),
            balance_effect: 0,
        }
    }

    #[test]
    fn archived_category_ids_remain_valid_for_tag_retention() {
        let active = vec![
            category(DRIFT_CATEGORY_ID.0, "idle"),
            category(1, "Current"),
        ];
        let archived = vec![category(7, "Historical")];
        let ids = valid_category_ids_for_catalog(active, &archived);
        assert!(ids.contains(&DRIFT_CATEGORY_ID.0));
        assert!(ids.contains(&1));
        assert!(ids.contains(&7));
    }
}

#[cfg(test)]
mod clear_all_temporal_tests {
    use super::clear_all_affected_days_for_interval;
    use crate::domain::OperationalDayPolicy;
    use chrono::{Duration as ChronoDuration, TimeZone, Utc};

    #[test]
    fn idle_clear_all_conserves_fractional_cross_boundary_interval() {
        let policy = OperationalDayPolicy {
            utc_offset_seconds: -21600,
            start_minutes: 360,
        };
        let start = Utc
            .with_ymd_and_hms(2026, 8, 21, 7, 14, 55)
            .single()
            .unwrap()
            + ChronoDuration::nanoseconds(773_810_532);
        let elapsed_seconds = 32_937;
        let end = start + ChronoDuration::seconds(elapsed_seconds as i64);
        let operation_day = chrono::NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();

        let days = clear_all_affected_days_for_interval(
            operation_day,
            true,
            start,
            end,
            elapsed_seconds,
            policy,
        )
        .unwrap();

        assert_eq!(days.len(), 2);
        assert!(days.contains(&chrono::NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()));
        assert!(days.contains(&operation_day));
    }
}

#[cfg(test)]
mod day_end_snapshot_tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use super::{PendingDayEndSnapshot, stage_pending_day_end_snapshot};
    use crate::sand::{PendingGrainRun, SandState, SandStateGrain};

    fn state(x: usize) -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 9,
            grid_height: 6,
            grains: vec![SandStateGrain {
                x,
                y: 5,
                category_id: 1,
            }],
            frame_count: 41,
            sweep_left_to_right: false,
            rng_state: 12345,
            pending_grains: Vec::new(),
            pending_runs: vec![PendingGrainRun {
                category_id: 0,
                count: 2,
            }],
        }
    }

    #[test]
    fn staged_day_end_is_exact_and_first_write_wins_in_memory() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
        let boundary = Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).unwrap();
        let original = state(4);
        let mut pending = Vec::<PendingDayEndSnapshot>::new();

        stage_pending_day_end_snapshot(&mut pending, day, boundary, original.clone()).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].operational_day, day);
        assert_eq!(pending[0].captured_at_utc, boundary);
        assert_eq!(pending[0].snapshot.state, original);
        assert!(pending[0].snapshot.is_authentic_day_end_for("2026-08-01"));

        stage_pending_day_end_snapshot(&mut pending, day, boundary, state(4)).unwrap();
        assert_eq!(pending.len(), 1);

        let error =
            stage_pending_day_end_snapshot(&mut pending, day, boundary, state(5)).unwrap_err();
        assert!(error.contains("conflicting in-memory day-end snapshots"));
        assert_eq!(pending[0].snapshot.state, state(4));
    }
}
