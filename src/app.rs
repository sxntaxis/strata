use std::{
    collections::{BTreeSet, HashSet, VecDeque},
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, SecondsFormat, Utc};
use crossterm::event::{self, Event};
use ratatui::layout::Rect;
use serde::{Deserialize, Serialize};

use crate::{
    constants::{
        APP_LAYOUT_SETTINGS, BLINK_SETTINGS, CATCHUP_SETTINGS, FACE_SETTINGS,
        RUNTIME_LOOP_SETTINGS, TIME_SETTINGS,
    },
    domain::{
        Category, CategoryId, DRIFT_CATEGORY_DISPLAY_NAME, DRIFT_CATEGORY_ID, FirstDayOfWeek,
        OperationalDayPolicy, ReportPeriod, RuntimeSettings, TimeTracker, civil_time_for_utc,
        is_drift_category_id, operational_day_key_for_utc, set_runtime_settings,
    },
    keybindings::{self, Action, ActionBindingState, KeyBinding},
    legacy_transition::{
        ClearAllReceipt, LegacyActiveReceipt, LegacyFinishReceipt, LegacySessionReceipt,
        LegacyTransitionKind, LegacyTransitionReceipt, reconcile_completed_session,
    },
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
    WeekStartDay,
    Action(keybindings::Action),
}

#[derive(Clone, Debug)]
enum AtlasOverlay {
    CaptureKey { action: keybindings::Action },
    EditTimeLogPath { input: String },
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
    #[serde(default)]
    recovery_target_utc: Option<DateTime<Utc>>,
    #[serde(default)]
    legacy_recovery_committed: bool,
    #[serde(default)]
    legacy_transition: Option<LegacyTransitionReceipt>,
    #[serde(default)]
    legacy_finish: Option<LegacyFinishReceipt>,
    #[serde(default)]
    clear_all: Option<ClearAllReceipt>,
}

impl DetachedRuntimeCheckpoint {
    const VERSION: u8 = 3;
    const PREVIOUS_VERSION: u8 = 2;
    const LEGACY_VERSION: u8 = 1;
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

fn recovery_target_for_claim(
    persisted_target_utc: Option<DateTime<Utc>>,
    claim_time_utc: DateTime<Utc>,
) -> DateTime<Utc> {
    persisted_target_utc.unwrap_or(claim_time_utc)
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
    if started_at_utc > target_utc {
        return Err("recovery statement active session starts after its target".to_string());
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

fn transition_operation_id(
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

fn clear_all_operation_id(
    previous_active: &LegacyActiveReceipt,
    applied_at_utc: DateTime<Utc>,
    idle_reset: bool,
    previous_elapsed_seconds: usize,
    affected_operational_days: &[String],
) -> String {
    let description = &previous_active.description;
    let identity = format!(
        "{}:{}:{}:{}:{}:{}",
        previous_active.category_id,
        description.len(),
        description,
        previous_active
            .started_at_utc
            .to_rfc3339_opts(SecondsFormat::Nanos, true),
        previous_elapsed_seconds,
        affected_operational_days.join(",")
    );
    transition_operation_id(
        "clear-all",
        &identity,
        applied_at_utc,
        if idle_reset {
            "idle-reset"
        } else {
            "active-preserved"
        },
    )
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

fn stage_clear_all_active_state(
    tracker: &mut TimeTracker,
    active_session_started_at_utc: &mut Option<DateTime<Utc>>,
    receipt: &ClearAllReceipt,
) -> Result<(), String> {
    let resulting_category_id = CategoryId::new(receipt.resulting_active.category_id);
    if !tracker.set_active_category_by_id(resulting_category_id) {
        return Err(format!(
            "clear-all receipt {} references unavailable resulting category {}",
            receipt.operation_id, receipt.resulting_active.category_id
        ));
    }
    if !tracker.set_category_description_by_id(
        resulting_category_id,
        receipt.resulting_active.description.clone(),
    ) {
        return Err(format!(
            "clear-all receipt {} cannot restore its resulting description",
            receipt.operation_id
        ));
    }
    let resulting_elapsed_seconds = if receipt.idle_reset {
        0
    } else {
        receipt.previous_elapsed_seconds
    };
    tracker.start_session_with_elapsed(resulting_elapsed_seconds)?;
    *active_session_started_at_utc = Some(receipt.resulting_active.started_at_utc);
    Ok(())
}

fn validate_legacy_switch_checkpoint(
    checkpoint: &DetachedRuntimeCheckpoint,
    receipt: &LegacyTransitionReceipt,
) -> Result<(), String> {
    if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION {
        return Err(format!(
            "legacy transition receipt requires checkpoint schema {}, found {}; evidence retained",
            DetachedRuntimeCheckpoint::VERSION,
            checkpoint.schema_version
        ));
    }
    receipt.validate_switch_boundaries()?;
    let expected_identity = format!(
        "legacy:{}:{}",
        receipt.expected_previous_category_id,
        receipt
            .expected_previous_started_at_utc
            .to_rfc3339_opts(SecondsFormat::Nanos, true)
    );
    let expected_operation_id = transition_operation_id(
        "legacy-switch",
        &expected_identity,
        receipt.transition_at_utc,
        &receipt.resulting_active.category_id.to_string(),
    );
    if receipt.operation_id != expected_operation_id {
        return Err(format!(
            "legacy switch receipt operation ID {} is inconsistent; evidence retained",
            receipt.operation_id
        ));
    }
    if checkpoint.active_category_id != receipt.resulting_active.category_id
        || checkpoint.active_description != receipt.resulting_active.description
        || checkpoint.active_session_started_at_utc != Some(receipt.resulting_active.started_at_utc)
    {
        return Err(format!(
            "legacy switch receipt {} does not match its resulting checkpoint generation",
            receipt.operation_id
        ));
    }
    Ok(())
}

fn publish_legacy_switch_replay(
    tracker: &TimeTracker,
    archived_categories: &[Category],
    checkpoint: &mut DetachedRuntimeCheckpoint,
    receipt: &LegacyTransitionReceipt,
    sessions_path: &Path,
    categories_path: &Path,
    checkpoint_path: &Path,
) -> Result<TimeTracker, String> {
    let mut staged_tracker = tracker.clone();
    reconcile_completed_session(
        &mut staged_tracker.sessions,
        &mut staged_tracker.session_id_counter,
        receipt.completed_session.as_ref(),
    )?;
    let previous_category_id = CategoryId::new(receipt.expected_previous_category_id);
    if !staged_tracker.set_category_description_by_id(previous_category_id, String::new()) {
        return Err(format!(
            "legacy switch receipt {} references unavailable previous category {}",
            receipt.operation_id, receipt.expected_previous_category_id
        ));
    }
    let resulting_category_id = CategoryId::new(receipt.resulting_active.category_id);
    if !staged_tracker.set_category_description_by_id(
        resulting_category_id,
        receipt.resulting_active.description.clone(),
    ) {
        return Err(format!(
            "legacy switch receipt {} references unavailable resulting category {}",
            receipt.operation_id, receipt.resulting_active.category_id
        ));
    }

    let mut catalog = staged_tracker.categories_for_storage();
    catalog.extend(archived_categories.iter().cloned());
    storage::save_sessions_to_csv(sessions_path, &staged_tracker.sessions, &catalog)?;
    storage::save_category_catalog_to_csv(
        categories_path,
        &staged_tracker.categories_for_storage(),
        archived_categories,
    )?;

    checkpoint.legacy_transition = None;
    checkpoint.schema_version = DetachedRuntimeCheckpoint::VERSION;
    storage::write_json_atomic(checkpoint_path, checkpoint)?;
    Ok(staged_tracker)
}

fn validate_legacy_finish_checkpoint(
    checkpoint: &DetachedRuntimeCheckpoint,
    receipt: &LegacyFinishReceipt,
) -> Result<(), String> {
    if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION {
        return Err(format!(
            "legacy finish receipt requires checkpoint schema {}, found {}; evidence retained",
            DetachedRuntimeCheckpoint::VERSION,
            checkpoint.schema_version
        ));
    }
    if checkpoint.legacy_transition.is_some() {
        return Err(
            "checkpoint contains both switch and finish receipts; evidence retained".to_string(),
        );
    }
    receipt.validate_boundaries()?;
    let expected_identity = format!(
        "legacy:{}:{}",
        receipt.expected_previous_category_id,
        receipt
            .expected_previous_started_at_utc
            .to_rfc3339_opts(SecondsFormat::Nanos, true)
    );
    let expected_operation_id = transition_operation_id(
        "legacy-finish",
        &expected_identity,
        receipt.finished_at_utc,
        "complete",
    );
    if receipt.operation_id != expected_operation_id {
        return Err(format!(
            "legacy finish receipt operation ID {} is inconsistent; evidence retained",
            receipt.operation_id
        ));
    }
    if checkpoint.active_category_id != receipt.expected_previous_category_id
        || checkpoint.active_description != receipt.expected_previous_description
        || checkpoint.active_session_started_at_utc
            != Some(receipt.expected_previous_started_at_utc)
    {
        return Err(format!(
            "legacy finish receipt {} does not match its prior checkpoint generation",
            receipt.operation_id
        ));
    }
    Ok(())
}

fn publish_legacy_finish_replay(
    tracker: &TimeTracker,
    archived_categories: &[Category],
    checkpoint: &DetachedRuntimeCheckpoint,
    receipt: &LegacyFinishReceipt,
    sessions_path: &Path,
    categories_path: &Path,
    sand_path: &Path,
) -> Result<TimeTracker, String> {
    let mut staged_tracker = tracker.clone();
    reconcile_completed_session(
        &mut staged_tracker.sessions,
        &mut staged_tracker.session_id_counter,
        receipt.completed_session.as_ref(),
    )?;
    let previous_category_id = CategoryId::new(receipt.expected_previous_category_id);
    if !staged_tracker.set_category_description_by_id(previous_category_id, String::new()) {
        return Err(format!(
            "legacy finish receipt {} references unavailable previous category {}",
            receipt.operation_id, receipt.expected_previous_category_id
        ));
    }
    let mut catalog = staged_tracker.categories_for_storage();
    catalog.extend(archived_categories.iter().cloned());
    storage::save_sessions_to_csv(sessions_path, &staged_tracker.sessions, &catalog)?;
    storage::save_category_catalog_to_csv(
        categories_path,
        &staged_tracker.categories_for_storage(),
        archived_categories,
    )?;
    storage::save_sand_state(sand_path, &checkpoint.sand_state)?;
    Ok(staged_tracker)
}

fn sand_state_is_empty(state: &SandState) -> bool {
    state.grains.is_empty() && state.pending_grains.is_empty() && state.pending_runs.is_empty()
}

fn validate_clear_all_checkpoint(
    checkpoint: &DetachedRuntimeCheckpoint,
    receipt: &ClearAllReceipt,
) -> Result<(), String> {
    if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION {
        return Err(format!(
            "clear-all receipt requires checkpoint schema {}, found {}; evidence retained",
            DetachedRuntimeCheckpoint::VERSION,
            checkpoint.schema_version
        ));
    }
    if checkpoint.legacy_transition.is_some() || checkpoint.legacy_finish.is_some() {
        return Err(
            "checkpoint contains overlapping transition receipts; evidence retained".to_string(),
        );
    }
    receipt.validate_boundaries()?;
    let expected_operation_id = clear_all_operation_id(
        &receipt.previous_active,
        receipt.applied_at_utc,
        receipt.idle_reset,
        receipt.previous_elapsed_seconds,
        &receipt.affected_operational_days,
    );
    if receipt.operation_id != expected_operation_id {
        return Err(format!(
            "clear-all receipt operation ID {} is inconsistent; evidence retained",
            receipt.operation_id
        ));
    }
    if checkpoint.active_category_id != receipt.resulting_active.category_id
        || checkpoint.active_description != receipt.resulting_active.description
        || checkpoint.active_session_started_at_utc != Some(receipt.resulting_active.started_at_utc)
    {
        return Err(format!(
            "clear-all receipt {} does not match its resulting checkpoint generation",
            receipt.operation_id
        ));
    }
    if !sand_state_is_empty(&checkpoint.sand_state) {
        return Err(format!(
            "clear-all receipt {} carries non-empty sediment",
            receipt.operation_id
        ));
    }
    Ok(())
}

#[derive(Clone)]
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
    report_log_edit: Option<ReportLogEditState>,
    report_snapshot_end_day: Option<String>,
    report_snapshot_artifact: Option<SedimentSnapshot>,
    report_snapshot_preview_key: Option<String>,
    report_snapshot_preview_lines: Option<Vec<ratatui::text::Line<'static>>>,
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
                let mut session_categories = loaded_categories.categories.clone();
                session_categories.extend(loaded_categories.archived_categories.iter().cloned());
                let loaded_sessions =
                    storage::try_load_sessions_from_csv(&sessions_path, &session_categories)
                        .map_err(|error| error.to_string())?;
                let tags = storage::load_category_tags(&storage::get_category_tags_path());
                let archived_categories = loaded_categories.archived_categories.clone();
                (
                    None,
                    loaded_categories,
                    loaded_sessions,
                    tags,
                    archived_categories,
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

        let valid_category_ids =
            valid_category_ids_for_catalog(tracker.categories_for_storage(), &archived_categories);
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
            report_log_edit: None,
            report_snapshot_end_day: None,
            report_snapshot_artifact: None,
            report_snapshot_preview_key: None,
            report_snapshot_preview_lines: None,
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
                let _ = app
                    .time_tracker
                    .set_category_description_by_id(active.category_id, active.description);
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
                app.sync_drift_idle_state();
                initial_checkpoint_published = app.persist_initial_active_generation();
            }
        }

        app.sync_drift_idle_state();
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
        let description = self
            .time_tracker
            .category_description_by_id(category_id)
            .unwrap_or_default()
            .to_string();
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
        let mut items = vec![AtlasSelectable::TimeLogPath, AtlasSelectable::WeekStartDay];
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
            AtlasSelectable::TimeLogPath => {
                "Path where session rows are written (time_log.csv).".to_string()
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
        if self.sqlite_database_path.is_some() {
            return self.end_active_session_at(finished_at_utc, SessionClockMode::LiveMonotonic);
        }
        let interval = match self
            .reconciled_active_interval(finished_at_utc, SessionClockMode::LiveMonotonic)
        {
            Ok(interval) => interval,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveFinish,
                    RecoveryAction::FinishAndExit,
                    Err(error),
                );
                return None;
            }
        };
        let previous_tracker = self.time_tracker.clone();
        let previous_session = self.session.clone();
        let previous_category_id = self.time_tracker.active_category_id();
        let previous_description = self
            .time_tracker
            .category_description_by_id(previous_category_id)
            .unwrap_or_default()
            .to_string();
        let Some(previous_started_at_utc) = self.session.active_session_started_at_utc else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::FinishAndExit,
                Err("legacy runtime has no active UTC start timestamp to finish".to_string()),
            );
            return None;
        };
        let mut prepared_checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::FinishAndExit,
                    Err(error),
                );
                return None;
            }
        };
        let previous_session_count = self.time_tracker.sessions.len();
        let ended_civil = civil_time_for_utc(interval.ended_at_utc);
        let result = self
            .time_tracker
            .end_session_with_elapsed_at_local(interval.elapsed_seconds, ended_civil);
        self.session.active_session_started_at_utc = None;
        let completed_session = self
            .time_tracker
            .sessions
            .get(previous_session_count)
            .map(LegacySessionReceipt::from_session);
        let expected_identity = format!(
            "legacy:{}:{}",
            previous_category_id.0,
            previous_started_at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true)
        );
        let receipt = LegacyFinishReceipt {
            version: LegacyFinishReceipt::VERSION,
            operation_id: transition_operation_id(
                "legacy-finish",
                &expected_identity,
                interval.ended_at_utc,
                "complete",
            ),
            expected_previous_category_id: previous_category_id.0,
            expected_previous_description: previous_description,
            expected_previous_started_at_utc: previous_started_at_utc,
            finished_at_utc: interval.ended_at_utc,
            completed_session,
        };
        prepared_checkpoint.legacy_finish = Some(receipt);
        if let Err(error) =
            storage::write_json_atomic(&storage::get_detached_runtime_path(), &prepared_checkpoint)
        {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::FinishAndExit,
                Err(error),
            );
            return None;
        }
        result
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
            let operation_id = transition_operation_id(
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
            self.refresh_active_runtime_checkpoint();
            return !self.has_persistence_recovery();
        }

        let previous_tracker = self.time_tracker.clone();
        let previous_session = self.session.clone();
        let previous_category_id = self.time_tracker.active_category_id();
        let Some(previous_started_at_utc) = self.session.active_session_started_at_utc else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveSwitch,
                RecoveryAction::ReloadAuthority,
                Err("legacy runtime has no active UTC start timestamp to switch".to_string()),
            );
            return false;
        };
        let previous_session_count = self.time_tracker.sessions.len();

        if self
            .end_active_session_at(switched_at_utc, clock_mode)
            .is_none()
        {
            return false;
        }
        let completed_session = self
            .time_tracker
            .sessions
            .get(previous_session_count)
            .map(LegacySessionReceipt::from_session);

        if !self.time_tracker.set_active_category_by_id(category_id) {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            return false;
        }
        if let Err(error) = self.begin_transition_session(switched_at_utc, clock_mode) {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }

        let resulting_description = self
            .time_tracker
            .category_description_by_id(category_id)
            .unwrap_or_default()
            .to_string();
        let expected_identity = format!(
            "legacy:{}:{}",
            previous_category_id.0,
            previous_started_at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true)
        );
        let operation_id = transition_operation_id(
            "legacy-switch",
            &expected_identity,
            switched_at_utc,
            &category_id.0.to_string(),
        );
        let receipt = LegacyTransitionReceipt {
            version: LegacyTransitionReceipt::VERSION,
            operation_id,
            kind: LegacyTransitionKind::Switch,
            expected_previous_category_id: previous_category_id.0,
            expected_previous_started_at_utc: previous_started_at_utc,
            transition_at_utc: switched_at_utc,
            completed_session,
            resulting_active: LegacyActiveReceipt {
                category_id: category_id.0,
                description: resulting_description,
                started_at_utc: switched_at_utc,
            },
        };
        let mut prepared_checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.time_tracker = previous_tracker;
                self.session = previous_session;
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return false;
            }
        };
        prepared_checkpoint.legacy_transition = Some(receipt);
        if let Err(error) =
            storage::write_json_atomic(&storage::get_detached_runtime_path(), &prepared_checkpoint)
        {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }

        self.persist_sessions();
        if self.has_persistence_recovery() {
            return false;
        }
        self.persist_categories();
        if self.has_persistence_recovery() {
            return false;
        }
        self.sync_drift_idle_state();
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

    fn reconcile_clear_all_receipt(
        &mut self,
        checkpoint: &mut DetachedRuntimeCheckpoint,
    ) -> Result<(), String> {
        let Some(receipt) = checkpoint.clear_all.clone() else {
            return Ok(());
        };
        validate_clear_all_checkpoint(checkpoint, &receipt)?;
        if self.sqlite_database_path.is_none() {
            let valid_category_ids = self
                .time_tracker
                .categories_for_storage()
                .into_iter()
                .chain(self.archived_categories.iter().cloned())
                .map(|category| category.id)
                .collect::<HashSet<_>>();
            self.sand_engine
                .restore_state(&checkpoint.sand_state, &valid_category_ids);
            stage_clear_all_active_state(
                &mut self.time_tracker,
                &mut self.session.active_session_started_at_utc,
                &receipt,
            )?;
            storage::save_sand_state(&storage::get_sand_state_path(), &checkpoint.sand_state)?;
            for value in &receipt.affected_operational_days {
                let day = NaiveDate::parse_from_str(value, "%Y-%m-%d")
                    .map_err(|error| error.to_string())?;
                self.reconcile_daily_contribution(day);
                if let Some(recovery) = self.persistence_recovery.as_ref() {
                    return Err(recovery.failure.summary());
                }
            }
            checkpoint.clear_all = None;
            storage::write_json_atomic(&storage::get_detached_runtime_path(), checkpoint)?;
        } else if let Some(database_path) = self.sqlite_database_path.clone() {
            let expected_stable_id = self
                .session
                .active_session_stable_id
                .as_deref()
                .ok_or_else(|| {
                    "SQLite clear-all recovery has no active stable identity".to_string()
                })?;
            checkpoint.clear_all = None;
            sqlite::replace_tui_recovering_checkpoint(
                &database_path,
                expected_stable_id,
                checkpoint,
            )?;
        }
        Ok(())
    }

    fn apply_clear_all_at(&mut self, applied_at_utc: DateTime<Utc>, clock_mode: SessionClockMode) {
        let (affected_days, previous_elapsed_seconds) =
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
        let previous_active = LegacyActiveReceipt {
            category_id: self.time_tracker.active_category_id().0,
            description: self
                .time_tracker
                .category_description_by_id(self.time_tracker.active_category_id())
                .unwrap_or_default()
                .to_string(),
            started_at_utc: match self.session.active_session_started_at_utc {
                Some(value) => value,
                None => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::ActiveReset,
                        RecoveryAction::ReloadAuthority,
                        Err("runtime has no active UTC start timestamp to clear".to_string()),
                    );
                    return;
                }
            },
        };
        let idle_reset = is_drift_category_id(self.time_tracker.active_category_id());

        self.sand_engine.clear();
        if idle_reset && let Err(error) = self.begin_transition_session(applied_at_utc, clock_mode)
        {
            self.sand_engine.restore_state(
                &previous_sand,
                &self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect(),
            );
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }
        let resulting_active = LegacyActiveReceipt {
            category_id: previous_active.category_id,
            description: previous_active.description.clone(),
            started_at_utc: if idle_reset {
                applied_at_utc
            } else {
                previous_active.started_at_utc
            },
        };
        let affected_operational_days = affected_days
            .iter()
            .map(|day| day.format("%Y-%m-%d").to_string())
            .collect::<Vec<_>>();
        let operation_id = clear_all_operation_id(
            &previous_active,
            applied_at_utc,
            idle_reset,
            previous_elapsed_seconds,
            &affected_operational_days,
        );
        let receipt = ClearAllReceipt {
            version: ClearAllReceipt::VERSION,
            operation_id,
            applied_at_utc,
            previous_active,
            resulting_active,
            idle_reset,
            previous_elapsed_seconds,
            affected_operational_days,
        };
        let mut checkpoint = match self.build_runtime_checkpoint() {
            Ok(value) => value,
            Err(error) => {
                self.time_tracker = previous_tracker;
                self.session = previous_session;
                self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        };
        checkpoint.clear_all = Some(receipt.clone());

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = previous_session.active_session_stable_id.clone() else {
                self.time_tracker = previous_tracker;
                self.session = previous_session;
                self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveReset,
                    RecoveryAction::ReloadAuthority,
                    Err("SQLite clear-all has no active stable identity".to_string()),
                );
                return;
            };
            let resulting_stable_id = if idle_reset {
                format!("tui-active:{}", receipt.operation_id)
            } else {
                expected_stable_id.clone()
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
                    resulting_started_at_utc: receipt.resulting_active.started_at_utc,
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
                self.time_tracker = previous_tracker;
                self.session = previous_session;
                self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );
                return;
            }
            self.session.active_session_stable_id = Some(resulting_stable_id);
            self.sync_drift_idle_state();
            return;
        }

        if let Err(error) =
            storage::write_json_atomic(&storage::get_detached_runtime_path(), &checkpoint)
        {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.sand_engine.restore_state(
                &previous_sand,
                &self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect(),
            );
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }
        self.persist_sand_state();
        if self.has_persistence_recovery() {
            return;
        }
        for day in affected_days {
            self.reconcile_daily_contribution(day);
            if self.has_persistence_recovery() {
                return;
            }
        }
        self.refresh_active_runtime_checkpoint();
        self.sync_drift_idle_state();
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
            .restore_state(&settlement.state, &valid_category_ids);
        self.simulation.spawn_accumulator = settlement.spawn_remainder;
        self.simulation.physics_accumulator = settlement.physics_remainder;
        self.simulation.simulation_time_utc = target_utc;
        if settlement.added_grains > 0 || settlement.skipped_physics_events > 0 {
            self.render_needed = true;
        }
        Ok(())
    }

    fn settle_transition_boundary(&mut self, boundary_utc: DateTime<Utc>) -> Result<(), String> {
        loop {
            let Some(next) = self.simulation.pending_mutations.front().cloned() else {
                break;
            };
            if next.execute_at_utc > boundary_utc {
                break;
            }
            self.settle_simulation_segment_to(next.execute_at_utc)?;
            self.simulation.pending_mutations.pop_front();
            self.apply_mutation_at(
                next.mutation,
                next.execute_at_utc,
                SessionClockMode::HistoricalWall,
            );
            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
        }
        self.settle_simulation_segment_to(boundary_utc)
    }

    fn queue_or_apply_mutation(&mut self, mutation: QueuedMutation) {
        let scheduled_at_utc = Utc::now();
        if self.is_catching_up() || !self.simulation.pending_mutations.is_empty() {
            self.simulation
                .pending_mutations
                .push_back(QueuedMutationEvent {
                    execute_at_utc: scheduled_at_utc,
                    mutation,
                });
        } else if let Err(error) = self.settle_transition_boundary(scheduled_at_utc) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
        } else {
            self.apply_mutation_at(mutation, scheduled_at_utc, SessionClockMode::LiveMonotonic);
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
                self.apply_clear_all_at(scheduled_at_utc, clock_mode);
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

    fn build_runtime_checkpoint(&self) -> Result<DetachedRuntimeCheckpoint, String> {
        if self.checkpoint_recovery_active {
            return Err("checkpoint recovery is still active".to_string());
        }
        if !self.simulation.pending_mutations.is_empty() {
            return Err(
                "runtime checkpoint cannot be written while mutations are pending".to_string(),
            );
        }

        let active_category_id = self.time_tracker.active_category_id();
        let active_description = self
            .time_tracker
            .category_description_by_id(active_category_id)
            .unwrap_or_default()
            .to_string();
        let spawn_accumulator_nanos =
            u64::try_from(self.simulation.spawn_accumulator.as_nanos())
                .map_err(|_| "spawn accumulator exceeds checkpoint range".to_string())?;
        let physics_accumulator_nanos =
            u64::try_from(self.simulation.physics_accumulator.as_nanos())
                .map_err(|_| "physics accumulator exceeds checkpoint range".to_string())?;

        Ok(DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
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
            legacy_recovery_committed: false,
            legacy_transition: None,
            legacy_finish: None,
            clear_all: None,
        })
    }

    fn try_write_runtime_checkpoint(&self) -> Result<(), String> {
        let checkpoint = self.build_runtime_checkpoint()?;
        if let Some(database_path) = self.sqlite_database_path.clone() {
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
        } else {
            storage::write_json_atomic(&storage::get_detached_runtime_path(), &checkpoint)
        }
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

    fn reconcile_legacy_transition_receipt(
        &mut self,
        checkpoint: &mut DetachedRuntimeCheckpoint,
    ) -> Result<bool, String> {
        if self.sqlite_database_path.is_some()
            && (checkpoint.legacy_transition.is_some() || checkpoint.legacy_finish.is_some())
        {
            return Err(
                "legacy transition receipt appeared under SQLite authority; evidence retained"
                    .to_string(),
            );
        }
        if let Some(receipt) = checkpoint.legacy_finish.clone() {
            validate_legacy_finish_checkpoint(checkpoint, &receipt)?;
            let staged_tracker = publish_legacy_finish_replay(
                &self.time_tracker,
                &self.archived_categories,
                checkpoint,
                &receipt,
                &storage::get_time_log_path(),
                &storage::get_categories_path(),
                &storage::get_sand_state_path(),
            )?;
            self.time_tracker = staged_tracker;
            let valid_category_ids = self
                .time_tracker
                .categories_for_storage()
                .into_iter()
                .chain(self.archived_categories.iter().cloned())
                .map(|category| category.id)
                .collect::<HashSet<_>>();
            self.sand_engine
                .restore_state(&checkpoint.sand_state, &valid_category_ids);
            self.reconcile_all_daily_contributions();
            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
            storage::delete_file_if_exists(&storage::get_detached_runtime_path())?;
            return Ok(true);
        }
        let Some(receipt) = checkpoint.legacy_transition.clone() else {
            return Ok(false);
        };
        validate_legacy_switch_checkpoint(checkpoint, &receipt)?;
        let staged_tracker = publish_legacy_switch_replay(
            &self.time_tracker,
            &self.archived_categories,
            checkpoint,
            &receipt,
            &storage::get_time_log_path(),
            &storage::get_categories_path(),
            &storage::get_detached_runtime_path(),
        )?;
        self.time_tracker = staged_tracker;
        Ok(false)
    }

    fn restore_from_detached_checkpoint(&mut self) -> bool {
        let mut checkpoint: DetachedRuntimeCheckpoint = if let Some(database_path) =
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
            match storage::read_json::<DetachedRuntimeCheckpoint>(&path) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result::<()>(Err(error));
                    return false;
                }
            }
        };

        if checkpoint.clear_all.is_some()
            && let Err(error) = self.reconcile_clear_all_receipt(&mut checkpoint)
        {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }

        if self.sqlite_database_path.is_none() {
            match self.reconcile_legacy_transition_receipt(&mut checkpoint) {
                Ok(true) => return false,
                Ok(false) => {}
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::CheckpointRecovery,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return false;
                }
            }
        }

        self.checkpoint_recovery_active = true;

        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION
            && checkpoint.schema_version != DetachedRuntimeCheckpoint::PREVIOUS_VERSION
            && checkpoint.schema_version != DetachedRuntimeCheckpoint::LEGACY_VERSION
        {
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
                "detached checkpoint contains queued mutations that cannot be recovered without a stable legacy receipt identity; evidence retained"
                    .to_string(),
            ));
            return false;
        }

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
        checkpoint.legacy_recovery_committed = false;

        let claim_persisted = if let Some(database_path) = self.sqlite_database_path.clone() {
            let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
                self.record_storage_result::<()>(Err(
                    "SQLite recovery checkpoint has no stable identity".to_string(),
                ));
                return false;
            };
            self.record_storage_result_for(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                sqlite::replace_tui_recovering_checkpoint(
                    &database_path,
                    &expected_stable_id,
                    &checkpoint,
                ),
            )
            .is_some()
        } else {
            let path = storage::get_detached_runtime_path();
            self.record_storage_result_for(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                storage::write_json_atomic(&path, &checkpoint),
            )
            .is_some()
        };
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

        self.sand_engine
            .restore_state(&recovered.state, &valid_category_ids);
        if !self
            .time_tracker
            .set_active_category_by_id(active_category_id)
        {
            self.record_storage_result::<()>(Err(
                "detached recovery could not select its active category".to_string(),
            ));
            return false;
        }
        let _ = self.time_tracker.set_category_description_by_id(
            active_category_id,
            checkpoint.active_description.clone(),
        );
        if let Err(error) = self.begin_active_session_at(started_at_utc, true) {
            self.record_storage_result::<()>(Err(error));
            return false;
        }

        self.simulation.simulation_time_utc = target_utc;
        self.simulation.spawn_accumulator = recovered.spawn_remainder;
        self.simulation.physics_accumulator = recovered.physics_remainder;
        self.simulation.pending_mutations.clear();
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
        let Some(mut checkpoint) = self.checkpoint_recovery_payload.clone() else {
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

        if let Some(database_path) = self.sqlite_database_path.clone() {
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
        } else {
            if let Err(error) = self.try_flush_current_state() {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointRecovery,
                    RecoveryAction::CommitCheckpointRecovery,
                    Err(error),
                );
                return;
            }
            checkpoint.legacy_recovery_committed = true;
            let path = storage::get_detached_runtime_path();
            if self
                .record_storage_result_for(
                    PersistenceOperation::CheckpointRecovery,
                    RecoveryAction::CommitCheckpointRecovery,
                    storage::write_json_atomic(&path, &checkpoint),
                )
                .is_none()
            {
                return;
            }
        }

        self.checkpoint_recovery_active = false;
        self.checkpoint_recovery_payload = None;
        self.reconcile_all_daily_contributions();
    }

    fn next_blink_interval(&self) -> i32 {
        BLINK_SETTINGS.interval_min_frames
            + (rand::random::<i32>()
                % (BLINK_SETTINGS.interval_max_frames - BLINK_SETTINGS.interval_min_frames))
    }
}

fn run_application_loop(
    app: &mut App,
    terminal: &mut ManagedTerminal,
) -> Result<Option<String>, io::Error> {
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
    use chrono::{TimeZone, Utc};

    use super::{
        DetachedRuntimeCheckpoint, PostTargetClass, RecoveredIntervalClass,
        build_recovery_statement, recovery_target_for_claim,
    };
    use crate::sand::SandState;

    fn timestamp(second: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 3, 18, 0, second).unwrap()
    }

    fn checkpoint(simulation_second: u32, capture_second: u32) -> DetachedRuntimeCheckpoint {
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
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
            legacy_recovery_committed: false,
            legacy_transition: None,
            legacy_finish: None,
            clear_all: None,
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
    fn non_monotonic_statement_fails_closed() {
        let invalid = checkpoint(4, 3);
        assert!(
            build_recovery_statement(&invalid, None, timestamp(5))
                .unwrap_err()
                .contains("not monotonic")
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
mod clear_all_replay_tests {
    use chrono::{TimeZone, Utc};
    use ratatui::style::Color;

    use super::{
        ClearAllReceipt, DetachedRuntimeCheckpoint, LegacyActiveReceipt,
        clear_all_affected_days_for_interval, clear_all_operation_id, stage_clear_all_active_state,
        validate_clear_all_checkpoint,
    };
    use crate::{
        domain::{Category, CategoryId, OperationalDayPolicy, TimeTracker},
        sand::{SandState, SandStateGrain},
    };

    fn categories() -> Vec<Category> {
        vec![
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
                color: Color::Blue,
                description: "focus".to_string(),
                karma_effect: 1,
            },
        ]
    }

    fn receipt(idle_reset: bool) -> ClearAllReceipt {
        let previous_started_at_utc = Utc.with_ymd_and_hms(2026, 8, 1, 23, 0, 0).unwrap();
        let applied_at_utc = Utc.with_ymd_and_hms(2026, 8, 2, 1, 0, 0).unwrap();
        let previous_active = LegacyActiveReceipt {
            category_id: if idle_reset { 0 } else { 1 },
            description: if idle_reset { "" } else { "focus" }.to_string(),
            started_at_utc: previous_started_at_utc,
        };
        let affected_operational_days = if idle_reset {
            vec!["2026-08-01".to_string(), "2026-08-02".to_string()]
        } else {
            vec!["2026-08-02".to_string()]
        };
        let previous_elapsed_seconds = 7_200;
        ClearAllReceipt {
            version: ClearAllReceipt::VERSION,
            operation_id: clear_all_operation_id(
                &previous_active,
                applied_at_utc,
                idle_reset,
                previous_elapsed_seconds,
                &affected_operational_days,
            ),
            applied_at_utc,
            resulting_active: LegacyActiveReceipt {
                category_id: previous_active.category_id,
                description: previous_active.description.clone(),
                started_at_utc: if idle_reset {
                    applied_at_utc
                } else {
                    previous_started_at_utc
                },
            },
            previous_active,
            idle_reset,
            previous_elapsed_seconds,
            affected_operational_days,
        }
    }

    fn checkpoint(receipt: ClearAllReceipt) -> DetachedRuntimeCheckpoint {
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: receipt.applied_at_utc,
            simulation_time_utc: receipt.applied_at_utc,
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: receipt.resulting_active.category_id,
            active_description: receipt.resulting_active.description.clone(),
            active_session_started_at_utc: Some(receipt.resulting_active.started_at_utc),
            sand_state: SandState {
                version: SandState::VERSION,
                grid_width: 3,
                grid_height: 5,
                grains: Vec::new(),
                frame_count: 9,
                sweep_left_to_right: true,
                rng_state: 7,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
            legacy_transition: None,
            legacy_finish: None,
            clear_all: Some(receipt),
        }
    }

    #[test]
    fn idle_cross_day_effect_names_every_touched_day() {
        let days = clear_all_affected_days_for_interval(
            Utc.with_ymd_and_hms(2026, 8, 3, 1, 0, 0)
                .unwrap()
                .date_naive(),
            true,
            Utc.with_ymd_and_hms(2026, 8, 1, 23, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 8, 3, 1, 0, 0).unwrap(),
            93_600,
            OperationalDayPolicy {
                utc_offset_seconds: 0,
                start_minutes: 0,
            },
        )
        .unwrap();
        assert_eq!(
            days.into_iter().collect::<Vec<_>>(),
            vec![
                Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0)
                    .unwrap()
                    .date_naive(),
                Utc.with_ymd_and_hms(2026, 8, 2, 0, 0, 0)
                    .unwrap()
                    .date_naive(),
                Utc.with_ymd_and_hms(2026, 8, 3, 0, 0, 0)
                    .unwrap()
                    .date_naive(),
            ]
        );
    }

    #[test]
    fn non_idle_effect_names_only_operation_day() {
        let operation_day = Utc
            .with_ymd_and_hms(2026, 8, 3, 0, 0, 0)
            .unwrap()
            .date_naive();
        let days = clear_all_affected_days_for_interval(
            operation_day,
            false,
            Utc.with_ymd_and_hms(2026, 8, 1, 23, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 8, 3, 1, 0, 0).unwrap(),
            93_600,
            OperationalDayPolicy {
                utc_offset_seconds: 0,
                start_minutes: 0,
            },
        )
        .unwrap();
        assert_eq!(days.into_iter().collect::<Vec<_>>(), vec![operation_day]);
    }

    #[test]
    fn receipt_identity_binds_elapsed_days_and_empty_sediment() {
        let receipt = receipt(false);
        let checkpoint = checkpoint(receipt.clone());
        validate_clear_all_checkpoint(&checkpoint, &receipt).unwrap();

        let mut changed_days = receipt.clone();
        changed_days.affected_operational_days = vec!["2026-08-01".to_string()];
        assert!(
            validate_clear_all_checkpoint(&checkpoint, &changed_days)
                .unwrap_err()
                .contains("operation ID")
        );

        let mut changed_elapsed = receipt.clone();
        changed_elapsed.previous_elapsed_seconds += 1;
        assert!(
            validate_clear_all_checkpoint(&checkpoint, &changed_elapsed)
                .unwrap_err()
                .contains("operation ID")
        );

        let mut non_empty = checkpoint;
        non_empty.sand_state.grains.push(SandStateGrain {
            x: 0,
            y: 0,
            category_id: 1,
        });
        assert!(
            validate_clear_all_checkpoint(&non_empty, &receipt)
                .unwrap_err()
                .contains("non-empty")
        );
    }

    #[test]
    fn replay_stages_exact_resulting_active_interval() {
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(categories(), 2, Vec::new(), 1);
        let mut started_at_utc = None;
        let non_idle = receipt(false);
        stage_clear_all_active_state(&mut tracker, &mut started_at_utc, &non_idle).unwrap();
        assert_eq!(tracker.active_category_id(), CategoryId::new(1));
        assert_eq!(
            started_at_utc,
            Some(non_idle.resulting_active.started_at_utc)
        );
        assert!(
            tracker.current_elapsed().unwrap().as_secs() as usize
                >= non_idle.previous_elapsed_seconds
        );
        assert!(
            tracker.current_elapsed().unwrap().as_secs() as usize
                <= non_idle.previous_elapsed_seconds.saturating_add(1)
        );

        let idle = receipt(true);
        stage_clear_all_active_state(&mut tracker, &mut started_at_utc, &idle).unwrap();
        assert_eq!(tracker.active_category_id(), CategoryId::new(0));
        assert_eq!(started_at_utc, Some(idle.applied_at_utc));
        assert!(tracker.current_elapsed().unwrap().as_secs() <= 1);
    }
}

#[cfg(test)]
mod bounded_checkpoint_tests {
    use super::DetachedRuntimeCheckpoint;
    use crate::sand::SandState;
    use chrono::{TimeZone, Utc};

    fn checkpoint() -> DetachedRuntimeCheckpoint {
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).unwrap(),
            simulation_time_utc: Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).unwrap(),
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: 0,
            active_description: String::new(),
            active_session_started_at_utc: Some(
                Utc.with_ymd_and_hms(2026, 8, 2, 11, 0, 0).unwrap(),
            ),
            sand_state: SandState {
                version: SandState::VERSION,
                grid_width: 2,
                grid_height: 4,
                grains: Vec::new(),
                frame_count: 0,
                sweep_left_to_right: true,
                rng_state: 1,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
            legacy_transition: None,
            legacy_finish: None,
            clear_all: None,
        }
    }

    #[test]
    fn new_checkpoint_fields_are_backward_compatible() {
        let value = serde_json::json!({
            "schema_version": 1,
            "detached_at_utc": "2026-08-02T12:00:00Z",
            "simulation_time_utc": "2026-08-02T12:00:00Z",
            "spawn_accumulator_nanos": 0,
            "physics_accumulator_nanos": 0,
            "active_category_id": 0,
            "active_description": "",
            "active_session_started_at_utc": "2026-08-02T11:00:00Z",
            "sand_state": checkpoint().sand_state,
            "pending_mutations": []
        });
        let decoded: DetachedRuntimeCheckpoint = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.recovery_target_utc, None);
        assert!(!decoded.legacy_recovery_committed);
    }

    #[test]
    fn committed_legacy_evidence_remains_explicit_in_payload() {
        let mut value = checkpoint();
        value.recovery_target_utc = Some(Utc.with_ymd_and_hms(2026, 8, 2, 13, 0, 0).unwrap());
        value.legacy_recovery_committed = true;
        let encoded = serde_json::to_string(&value).unwrap();
        let decoded: DetachedRuntimeCheckpoint = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.legacy_recovery_committed);
        assert_eq!(decoded.recovery_target_utc, value.recovery_target_utc);
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
            karma_effect: 0,
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
mod legacy_switch_replay_tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use chrono::{TimeZone, Utc};
    use ratatui::style::Color;

    use super::{
        DetachedRuntimeCheckpoint, publish_legacy_switch_replay, transition_operation_id,
        validate_legacy_switch_checkpoint,
    };
    use crate::{
        domain::{
            Category, CategoryId, DRIFT_CATEGORY_ID, OperationalDayPolicy, Session, TimeTracker,
        },
        legacy_transition::{
            LegacyActiveReceipt, LegacySessionReceipt, LegacyTransitionKind,
            LegacyTransitionReceipt,
        },
        sand::SandState,
        storage,
    };

    fn unique_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("strata-{label}-{}-{stamp}", std::process::id()))
    }

    fn category(id: u64, name: &str, description: &str) -> Category {
        Category {
            id: CategoryId::new(id),
            name: name.to_string(),
            color: if id == DRIFT_CATEGORY_ID.0 {
                Color::White
            } else {
                crate::constants::COLORS[((id - 1) as usize) % crate::constants::COLORS.len()]
            },
            description: description.to_string(),
            karma_effect: if id == DRIFT_CATEGORY_ID.0 { 0 } else { 1 },
        }
    }

    fn categories(before_switch: bool) -> Vec<Category> {
        vec![
            category(DRIFT_CATEGORY_ID.0, "idle", ""),
            category(1, "Previous", if before_switch { "focus" } else { "" }),
            category(2, "Next", "next task"),
        ]
    }

    fn completed_session() -> Session {
        Session {
            id: 1,
            date: "2026-08-02".to_string(),
            category_id: CategoryId::new(1),
            project: String::new(),
            description: "focus".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap()),
            ended_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap()),
            operational_day_policy: Some(OperationalDayPolicy {
                utc_offset_seconds: -21600,
                start_minutes: 360,
            }),
        }
    }

    fn receipt() -> LegacyTransitionReceipt {
        let previous_start = Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap();
        let transition = Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap();
        let expected_identity = format!(
            "legacy:1:{}",
            previous_start.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        );
        LegacyTransitionReceipt {
            version: LegacyTransitionReceipt::VERSION,
            operation_id: transition_operation_id(
                "legacy-switch",
                &expected_identity,
                transition,
                "2",
            ),
            kind: LegacyTransitionKind::Switch,
            expected_previous_category_id: 1,
            expected_previous_started_at_utc: previous_start,
            transition_at_utc: transition,
            completed_session: Some(LegacySessionReceipt::from_session(&completed_session())),
            resulting_active: LegacyActiveReceipt {
                category_id: 2,
                description: "next task".to_string(),
                started_at_utc: transition,
            },
        }
    }

    fn checkpoint(receipt: LegacyTransitionReceipt) -> DetachedRuntimeCheckpoint {
        let transition = receipt.transition_at_utc;
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: transition,
            simulation_time_utc: transition,
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: 2,
            active_description: "next task".to_string(),
            active_session_started_at_utc: Some(transition),
            sand_state: SandState {
                version: SandState::VERSION,
                grid_width: 2,
                grid_height: 4,
                grains: Vec::new(),
                frame_count: 0,
                sweep_left_to_right: true,
                rng_state: 1,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
            legacy_transition: Some(receipt),
            legacy_finish: None,
            clear_all: None,
        }
    }

    fn load_tracker(
        categories_path: &std::path::Path,
        sessions_path: &std::path::Path,
    ) -> TimeTracker {
        let loaded_categories = storage::try_load_categories_from_csv(categories_path).unwrap();
        let mut catalog = loaded_categories.categories.clone();
        catalog.extend(loaded_categories.archived_categories.iter().cloned());
        let loaded_sessions = storage::try_load_sessions_from_csv(sessions_path, &catalog).unwrap();
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );
        tracker
    }

    fn assert_converged(
        categories_path: &std::path::Path,
        sessions_path: &std::path::Path,
        checkpoint_path: &std::path::Path,
    ) {
        let loaded_categories = storage::try_load_categories_from_csv(categories_path).unwrap();
        let previous = loaded_categories
            .categories
            .iter()
            .find(|category| category.id == CategoryId::new(1))
            .unwrap();
        let next = loaded_categories
            .categories
            .iter()
            .find(|category| category.id == CategoryId::new(2))
            .unwrap();
        assert_eq!(previous.description, "");
        assert_eq!(next.description, "next task");

        let mut catalog = loaded_categories.categories.clone();
        catalog.extend(loaded_categories.archived_categories.iter().cloned());
        let loaded_sessions = storage::try_load_sessions_from_csv(sessions_path, &catalog).unwrap();
        assert_eq!(loaded_sessions.sessions.len(), 1);
        assert_eq!(loaded_sessions.sessions[0].id, 1);
        assert_eq!(loaded_sessions.sessions[0].elapsed_seconds, 3600);

        let checkpoint: DetachedRuntimeCheckpoint = storage::read_json(checkpoint_path).unwrap();
        assert!(checkpoint.legacy_transition.is_none());
    }

    #[test]
    fn every_persisted_switch_kill_point_converges_without_duplicate_time() {
        for phase in 0..3 {
            let dir = unique_dir(&format!("legacy-switch-phase-{phase}"));
            fs::create_dir_all(&dir).unwrap();
            let categories_path = dir.join("categories.csv");
            let sessions_path = dir.join("time_log.csv");
            let checkpoint_path = dir.join("detached_runtime.json");
            let receipt = receipt();
            let checkpoint = checkpoint(receipt.clone());

            let seeded_categories = categories(phase < 2);
            storage::save_category_catalog_to_csv(&categories_path, &seeded_categories, &[])
                .unwrap();
            if phase >= 1 {
                storage::save_sessions_to_csv(
                    &sessions_path,
                    &[completed_session()],
                    &seeded_categories,
                )
                .unwrap();
            }
            storage::write_json_atomic(&checkpoint_path, &checkpoint).unwrap();

            let tracker = load_tracker(&categories_path, &sessions_path);
            let mut loaded_checkpoint: DetachedRuntimeCheckpoint =
                storage::read_json(&checkpoint_path).unwrap();
            validate_legacy_switch_checkpoint(&loaded_checkpoint, &receipt).unwrap();
            let replayed = publish_legacy_switch_replay(
                &tracker,
                &[],
                &mut loaded_checkpoint,
                &receipt,
                &sessions_path,
                &categories_path,
                &checkpoint_path,
            )
            .unwrap();
            assert_eq!(replayed.sessions.len(), 1);
            assert_converged(&categories_path, &sessions_path, &checkpoint_path);
            fs::remove_dir_all(dir).ok();
        }
    }

    #[test]
    fn failed_catalog_publication_retains_receipt_after_session_converges() {
        let dir = unique_dir("legacy-switch-catalog-failure");
        fs::create_dir_all(&dir).unwrap();
        let categories_path = dir.join("categories-as-directory");
        let sessions_path = dir.join("time_log.csv");
        let checkpoint_path = dir.join("detached_runtime.json");
        fs::create_dir_all(&categories_path).unwrap();

        let receipt = receipt();
        let mut checkpoint = checkpoint(receipt.clone());
        storage::write_json_atomic(&checkpoint_path, &checkpoint).unwrap();
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(categories(true), 3, Vec::new(), 1);

        let error = match publish_legacy_switch_replay(
            &tracker,
            &[],
            &mut checkpoint,
            &receipt,
            &sessions_path,
            &categories_path,
            &checkpoint_path,
        ) {
            Ok(_) => panic!("catalog publication unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(!error.is_empty());

        let disk_checkpoint: DetachedRuntimeCheckpoint =
            storage::read_json(&checkpoint_path).unwrap();
        assert!(disk_checkpoint.legacy_transition.is_some());
        let loaded_sessions =
            storage::try_load_sessions_from_csv(&sessions_path, &categories(true)).unwrap();
        assert_eq!(loaded_sessions.sessions.len(), 1);
        fs::remove_dir_all(dir).ok();
    }
}

#[cfg(test)]
mod legacy_finish_replay_tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use chrono::{TimeZone, Utc};
    use ratatui::style::Color;

    use super::{
        DetachedRuntimeCheckpoint, publish_legacy_finish_replay, transition_operation_id,
        validate_legacy_finish_checkpoint,
    };
    use crate::{
        domain::{
            Category, CategoryId, DRIFT_CATEGORY_ID, OperationalDayPolicy, Session, TimeTracker,
        },
        legacy_transition::{LegacyFinishReceipt, LegacySessionReceipt},
        sand::SandState,
        storage,
    };

    fn unique_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("strata-{label}-{}-{stamp}", std::process::id()))
    }

    fn category(id: u64, name: &str, description: &str) -> Category {
        Category {
            id: CategoryId::new(id),
            name: name.to_string(),
            color: if id == DRIFT_CATEGORY_ID.0 {
                Color::White
            } else {
                crate::constants::COLORS[((id - 1) as usize) % crate::constants::COLORS.len()]
            },
            description: description.to_string(),
            karma_effect: if id == DRIFT_CATEGORY_ID.0 { 0 } else { 1 },
        }
    }

    fn categories(before_finish: bool) -> Vec<Category> {
        vec![
            category(DRIFT_CATEGORY_ID.0, "idle", ""),
            category(1, "Work", if before_finish { "focus" } else { "" }),
        ]
    }

    fn completed_session() -> Session {
        Session {
            id: 1,
            date: "2026-08-02".to_string(),
            category_id: CategoryId::new(1),
            project: String::new(),
            description: "focus".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap()),
            ended_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap()),
            operational_day_policy: Some(OperationalDayPolicy {
                utc_offset_seconds: -21600,
                start_minutes: 360,
            }),
        }
    }

    fn receipt() -> LegacyFinishReceipt {
        let previous_start = Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap();
        let finished = Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap();
        let expected_identity = format!(
            "legacy:1:{}",
            previous_start.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        );
        LegacyFinishReceipt {
            version: LegacyFinishReceipt::VERSION,
            operation_id: transition_operation_id(
                "legacy-finish",
                &expected_identity,
                finished,
                "complete",
            ),
            expected_previous_category_id: 1,
            expected_previous_description: "focus".to_string(),
            expected_previous_started_at_utc: previous_start,
            finished_at_utc: finished,
            completed_session: Some(LegacySessionReceipt::from_session(&completed_session())),
        }
    }

    fn sand_state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 4,
            grains: Vec::new(),
            frame_count: 17,
            sweep_left_to_right: true,
            rng_state: 19,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        }
    }

    fn checkpoint(receipt: LegacyFinishReceipt) -> DetachedRuntimeCheckpoint {
        let finished = receipt.finished_at_utc;
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: finished,
            simulation_time_utc: finished,
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: 1,
            active_description: "focus".to_string(),
            active_session_started_at_utc: Some(receipt.expected_previous_started_at_utc),
            sand_state: sand_state(),
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
            legacy_transition: None,
            legacy_finish: Some(receipt),
            clear_all: None,
        }
    }

    fn load_tracker(
        categories_path: &std::path::Path,
        sessions_path: &std::path::Path,
    ) -> TimeTracker {
        let loaded_categories = storage::try_load_categories_from_csv(categories_path).unwrap();
        let mut catalog = loaded_categories.categories.clone();
        catalog.extend(loaded_categories.archived_categories.iter().cloned());
        let loaded_sessions = storage::try_load_sessions_from_csv(sessions_path, &catalog).unwrap();
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );
        tracker
    }

    fn assert_converged(
        categories_path: &std::path::Path,
        sessions_path: &std::path::Path,
        sand_path: &std::path::Path,
    ) {
        let loaded_categories = storage::try_load_categories_from_csv(categories_path).unwrap();
        let work = loaded_categories
            .categories
            .iter()
            .find(|category| category.id == CategoryId::new(1))
            .unwrap();
        assert_eq!(work.description, "");
        let mut catalog = loaded_categories.categories.clone();
        catalog.extend(loaded_categories.archived_categories.iter().cloned());
        let loaded_sessions = storage::try_load_sessions_from_csv(sessions_path, &catalog).unwrap();
        assert_eq!(loaded_sessions.sessions.len(), 1);
        assert_eq!(loaded_sessions.sessions[0].id, 1);
        assert_eq!(loaded_sessions.sessions[0].elapsed_seconds, 3600);
        let persisted_sand = storage::load_sand_state(sand_path).unwrap();
        assert_eq!(persisted_sand.frame_count, 17);
        assert_eq!(persisted_sand.rng_state, 19);
    }

    #[test]
    fn every_persisted_finish_kill_point_converges_without_duplicate_time() {
        for phase in 0..4 {
            let dir = unique_dir(&format!("legacy-finish-phase-{phase}"));
            fs::create_dir_all(&dir).unwrap();
            let categories_path = dir.join("categories.csv");
            let sessions_path = dir.join("time_log.csv");
            let sand_path = dir.join("sand_state.json");
            let checkpoint_path = dir.join("detached_runtime.json");
            let receipt = receipt();
            let checkpoint = checkpoint(receipt.clone());

            let seeded_categories = categories(phase < 2);
            storage::save_category_catalog_to_csv(&categories_path, &seeded_categories, &[])
                .unwrap();
            if phase >= 1 {
                storage::save_sessions_to_csv(
                    &sessions_path,
                    &[completed_session()],
                    &seeded_categories,
                )
                .unwrap();
            }
            if phase >= 3 {
                storage::save_sand_state(&sand_path, &sand_state()).unwrap();
            }
            storage::write_json_atomic(&checkpoint_path, &checkpoint).unwrap();

            let tracker = load_tracker(&categories_path, &sessions_path);
            validate_legacy_finish_checkpoint(&checkpoint, &receipt).unwrap();
            let replayed = publish_legacy_finish_replay(
                &tracker,
                &[],
                &checkpoint,
                &receipt,
                &sessions_path,
                &categories_path,
                &sand_path,
            )
            .unwrap();
            assert_eq!(replayed.sessions.len(), 1);
            assert_converged(&categories_path, &sessions_path, &sand_path);
            let retained: DetachedRuntimeCheckpoint = storage::read_json(&checkpoint_path).unwrap();
            assert!(retained.legacy_finish.is_some());
            fs::remove_dir_all(dir).ok();
        }
    }

    #[test]
    fn failed_finish_catalog_publication_retains_receipt_after_session_converges() {
        let dir = unique_dir("legacy-finish-catalog-failure");
        fs::create_dir_all(&dir).unwrap();
        let categories_path = dir.join("categories-as-directory");
        let sessions_path = dir.join("time_log.csv");
        let sand_path = dir.join("sand_state.json");
        let checkpoint_path = dir.join("detached_runtime.json");
        fs::create_dir_all(&categories_path).unwrap();
        let receipt = receipt();
        let checkpoint = checkpoint(receipt.clone());
        storage::write_json_atomic(&checkpoint_path, &checkpoint).unwrap();
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(categories(true), 2, Vec::new(), 1);

        let error = match publish_legacy_finish_replay(
            &tracker,
            &[],
            &checkpoint,
            &receipt,
            &sessions_path,
            &categories_path,
            &sand_path,
        ) {
            Ok(_) => panic!("catalog publication unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(!error.is_empty());
        let retained: DetachedRuntimeCheckpoint = storage::read_json(&checkpoint_path).unwrap();
        assert!(retained.legacy_finish.is_some());
        let loaded_sessions =
            storage::try_load_sessions_from_csv(&sessions_path, &categories(true)).unwrap();
        assert_eq!(loaded_sessions.sessions.len(), 1);
        assert!(!sand_path.exists());
        fs::remove_dir_all(dir).ok();
    }
}
