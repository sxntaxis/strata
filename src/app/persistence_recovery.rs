use std::{
    fmt, fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use chrono::{DateTime, Local, SecondsFormat, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use serde::Serialize;

use crate::{
    domain::{DRIFT_CATEGORY_ID, operational_day_key_now},
    sqlite, storage,
};

use super::{App, RecoveryStatement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PersistenceOperation {
    RuntimeWrite,
    StateReload,
    ActiveStart,
    ActiveFinish,
    ActiveSwitch,
    ActiveReset,
    ActiveDescription,
    CategorySync,
    CategoryArchive,
    CategoryDelete,
    CategoryTagsSync,
    SessionSync,
    SessionDelete,
    SessionCorrection,
    SandStateSave,
    DailySnapshotSave,
    DailySnapshotDelete,
    CheckpointSave,
    CheckpointClear,
    CheckpointRecovery,
}

impl fmt::Display for PersistenceOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RuntimeWrite => "runtime persistence",
            Self::StateReload => "authoritative state reload",
            Self::ActiveStart => "active-session start",
            Self::ActiveFinish => "active-session finish",
            Self::ActiveSwitch => "active-session switch",
            Self::ActiveReset => "active-session reset",
            Self::ActiveDescription => "active-session description",
            Self::CategorySync => "category synchronization",
            Self::CategoryArchive => "category archive",
            Self::CategoryDelete => "category deletion",
            Self::CategoryTagsSync => "category-tag synchronization",
            Self::SessionSync => "session synchronization",
            Self::SessionDelete => "session deletion",
            Self::SessionCorrection => "historical session correction",
            Self::SandStateSave => "sediment-state save",
            Self::DailySnapshotSave => "daily sediment snapshot save",
            Self::DailySnapshotDelete => "daily sediment snapshot deletion",
            Self::CheckpointSave => "detached checkpoint save",
            Self::CheckpointClear => "checkpoint cleanup",
            Self::CheckpointRecovery => "checkpoint recovery commit",
        })
    }
}

impl PersistenceOperation {
    fn records_durable_progress(self) -> bool {
        !matches!(self, Self::StateReload)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecoveryAction {
    FlushCurrentState,
    ReloadAuthority,
    FinishAndExit,
    DetachAndExit,
    CommitCheckpointRecovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum PersistenceFailureClass {
    Busy,
    ReadOnly,
    Corrupt,
    Constraint,
    Conflict,
    Commit,
    Io,
    InvalidData,
    Unknown,
}

impl fmt::Display for PersistenceFailureClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Busy => "database busy or locked",
            Self::ReadOnly => "read-only authority",
            Self::Corrupt => "database corruption",
            Self::Constraint => "integrity constraint",
            Self::Conflict => "concurrent authority conflict",
            Self::Commit => "transaction commit failure",
            Self::Io => "storage I/O failure",
            Self::InvalidData => "invalid persisted data",
            Self::Unknown => "unclassified persistence failure",
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct PersistenceFailure {
    pub operation: PersistenceOperation,
    pub class: PersistenceFailureClass,
    pub detail: String,
    pub authority_path: Option<PathBuf>,
    pub occurred_at_utc: String,
}

impl PersistenceFailure {
    fn new(app: &App, operation: PersistenceOperation, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        Self {
            operation,
            class: classify_failure(&detail),
            detail,
            authority_path: app.sqlite_database_path.clone(),
            occurred_at_utc: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }

    pub(super) fn summary(&self) -> String {
        let path = self
            .authority_path
            .as_ref()
            .map(|path| format!(" at {}", path.display()))
            .unwrap_or_default();
        format!(
            "{} failed{}: {} ({})",
            self.operation, path, self.detail, self.class
        )
    }
}

#[derive(Clone, Debug)]
pub(super) struct PersistenceRecoveryState {
    pub failure: PersistenceFailure,
    pub action: RecoveryAction,
    pub exported_path: Option<PathBuf>,
    pub export_error: Option<String>,
    pub dialog_open: bool,
    pub dismissed: bool,
    pub last_durable_at_utc: DateTime<Utc>,
    pub automatic_retries_remaining: u8,
    pub retry_stage: u8,
    pub next_retry_at: Instant,
}

fn emergency_categories(
    active_categories: impl IntoIterator<Item = crate::domain::Category>,
    archived_categories: &[crate::domain::Category],
) -> Vec<EmergencyCategory> {
    active_categories
        .into_iter()
        .map(|category| EmergencyCategory {
            id: category.id.0,
            name: category.name,
            description: category.description,
            color: format!("{:?}", category.color),
            balance_effect: category.balance_effect,
            archived: false,
        })
        .chain(
            archived_categories
                .iter()
                .cloned()
                .map(|category| EmergencyCategory {
                    id: category.id.0,
                    name: category.name,
                    description: category.description,
                    color: format!("{:?}", category.color),
                    balance_effect: category.balance_effect,
                    archived: true,
                }),
        )
        .collect()
}

impl App {
    pub(super) fn record_storage_result<T>(&mut self, result: Result<T, String>) -> Option<T> {
        self.record_storage_result_for(
            PersistenceOperation::RuntimeWrite,
            RecoveryAction::FlushCurrentState,
            result,
        )
    }

    pub(super) fn record_storage_result_for<T>(
        &mut self,
        operation: PersistenceOperation,
        action: RecoveryAction,
        result: Result<T, String>,
    ) -> Option<T> {
        match result {
            Ok(value) => {
                if operation.records_durable_progress() {
                    self.last_durable_save_utc = Utc::now();
                }
                Some(value)
            }
            Err(detail) => {
                if self.persistence_recovery.is_none() {
                    let last_durable_at_utc = self.last_durable_save_utc;
                    self.persistence_recovery = Some(PersistenceRecoveryState {
                        failure: PersistenceFailure::new(self, operation, detail),
                        action,
                        exported_path: None,
                        export_error: None,
                        dialog_open: false,
                        dismissed: false,
                        last_durable_at_utc,
                        automatic_retries_remaining: 3,
                        retry_stage: 0,
                        next_retry_at: Instant::now() + Duration::from_millis(100),
                    });
                    self.render_needed = true;
                }
                None
            }
        }
    }

    pub(super) fn has_persistence_recovery(&self) -> bool {
        self.persistence_recovery.is_some()
    }

    pub(super) fn promote_recovery_action(&mut self, action: RecoveryAction) {
        if let Some(recovery) = self.persistence_recovery.as_mut() {
            let retryable_finish = recovery.failure.operation == PersistenceOperation::ActiveFinish
                && !matches!(
                    recovery.failure.class,
                    PersistenceFailureClass::Conflict
                        | PersistenceFailureClass::Constraint
                        | PersistenceFailureClass::Corrupt
                        | PersistenceFailureClass::InvalidData
                );
            if recovery.action == RecoveryAction::FlushCurrentState || retryable_finish {
                recovery.action = action;
            }
        }
    }

    pub(super) fn persistence_recovery_dialog_open(&self) -> bool {
        self.persistence_recovery
            .as_ref()
            .is_some_and(|recovery| recovery.dialog_open)
    }

    pub(super) fn reopen_persistence_recovery_dialog(&mut self) {
        if let Some(recovery) = self.persistence_recovery.as_mut() {
            recovery.dialog_open = true;
            self.system_dialog_selected_index = 0;
            self.render_needed = true;
        }
    }

    pub(super) fn request_persistence_recovery_quit(&mut self) -> bool {
        self.export_current_recovery(true);
        self.recovery_exit_requested
    }

    fn continue_with_degraded_persistence(&mut self) {
        if let Some(recovery) = self.persistence_recovery.as_mut() {
            recovery.dialog_open = false;
            recovery.dismissed = true;
            recovery.automatic_retries_remaining = 0;
            recovery.retry_stage = 0;
            recovery.next_retry_at = Instant::now() + Duration::from_secs(5);
            recovery.action = RecoveryAction::FlushCurrentState;
        }
        self.render_needed = true;
    }

    pub(super) fn handle_persistence_recovery_key(&mut self, key: KeyEvent) -> bool {
        if !self.persistence_recovery_dialog_open() {
            return false;
        }
        match key.code {
            KeyCode::Up => self.move_system_dialog_selection(-1, 4),
            KeyCode::Down => self.move_system_dialog_selection(1, 4),
            KeyCode::Esc => self.continue_with_degraded_persistence(),
            KeyCode::Enter => match self.system_dialog_selected_index.min(3) {
                0 => self.continue_with_degraded_persistence(),
                1 => self.retry_persistence_failure(true),
                2 => self.export_current_recovery(false),
                3 => {
                    self.recovery_exit_requested = true;
                    self.recovery_exit_error = Some(
                        "exited without saving after an acknowledged persistence failure"
                            .to_string(),
                    );
                    return true;
                }
                _ => unreachable!(),
            },
            _ => {}
        }
        self.recovery_exit_requested
    }

    fn retry_delay_for_stage(stage: u8) -> Duration {
        match stage {
            0 => Duration::from_millis(100),
            1 => Duration::from_millis(300),
            2 => Duration::from_millis(700),
            3 => Duration::from_secs(5),
            4 => Duration::from_secs(15),
            _ => Duration::from_secs(60),
        }
    }

    pub(super) fn service_persistence_recovery(&mut self) {
        let Some(recovery) = self.persistence_recovery.as_ref() else {
            return;
        };
        if recovery.dialog_open || Instant::now() < recovery.next_retry_at {
            return;
        }
        self.retry_persistence_failure(false);
    }

    fn retry_persistence_failure(&mut self, manual: bool) {
        let Some(mut state) = self.persistence_recovery.take() else {
            return;
        };
        let was_dismissed = state.dismissed;
        let result = self.try_flush_current_state();
        if result.is_ok() {
            self.last_durable_save_utc = Utc::now();
            self.persistence_recovery = None;
            self.render_needed = true;
            return;
        }
        let detail = result
            .err()
            .unwrap_or_else(|| "persistence retry failed".to_string());
        state.failure = PersistenceFailure::new(self, state.failure.operation, detail);
        state.action = RecoveryAction::FlushCurrentState;
        state.export_error = None;

        if manual {
            state.dialog_open = true;
            state.dismissed = false;
            state.automatic_retries_remaining = 0;
            state.next_retry_at = Instant::now() + Duration::from_secs(5);
        } else if state.automatic_retries_remaining > 0 {
            state.automatic_retries_remaining -= 1;
            state.retry_stage = state.retry_stage.saturating_add(1);
            if state.automatic_retries_remaining == 0 {
                state.dialog_open = true;
                state.dismissed = false;
                self.system_dialog_selected_index = 0;
            } else {
                state.next_retry_at =
                    Instant::now() + Self::retry_delay_for_stage(state.retry_stage);
            }
        } else if was_dismissed {
            state.dismissed = true;
            state.dialog_open = false;
            state.retry_stage = state.retry_stage.saturating_add(1);
            state.next_retry_at =
                Instant::now() + Self::retry_delay_for_stage(3 + state.retry_stage);
        } else {
            state.dialog_open = true;
            self.system_dialog_selected_index = 0;
        }
        self.persistence_recovery = Some(state);
        self.render_needed = true;
    }

    pub(super) fn try_flush_current_state(&mut self) -> Result<(), String> {
        self.persist_pending_day_end_snapshots()?;
        let categories = self.time_tracker.categories_for_storage();
        let operational_day_date = operational_day_key_now();
        let operational_day = operational_day_date.format("%Y-%m-%d").to_string();
        let daily_contribution = self.daily_contribution_from_time_log(operational_day_date);

        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;

        self.archived_categories = sqlite::sync_tui_categories(
            &database_path,
            &categories,
            self.time_tracker.active_category_id(),
            self.session.active_session_stable_id.as_deref(),
        )?;
        if let Some(stable_id) = self.session.active_session_stable_id.as_deref() {
            sqlite::update_tui_active_description(
                &database_path,
                stable_id,
                self.time_tracker.active_description(),
            )?;
            self.modal_active_description_dirty = false;
        }
        let category_ids = categories
            .iter()
            .map(|category| category.id)
            .collect::<Vec<_>>();
        sqlite::sync_tui_category_tags(&database_path, &self.category_tags, &category_ids)?;
        sqlite::sync_tui_sessions(&database_path, &self.time_tracker.sessions)?;
        sqlite::save_tui_sand_state(&database_path, &self.sand_engine.snapshot_state())?;
        if let Some(snapshot) = daily_contribution.as_ref() {
            sqlite::save_tui_daily_snapshot(&database_path, &operational_day, snapshot)?;
        } else {
            sqlite::delete_tui_daily_snapshot(&database_path, &operational_day)?;
        }
        self.try_write_runtime_checkpoint()?;
        Ok(())
    }

    pub(super) fn try_reload_authority(&mut self) -> Result<(), String> {
        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;

        let state = sqlite::load_tui_state(&database_path)?;
        self.time_tracker.apply_loaded_state(
            state.loaded_categories.categories,
            state.loaded_categories.next_category_id,
            state.loaded_sessions.sessions,
            state.loaded_sessions.next_session_id,
        );
        self.category_tags = state.category_tags;
        self.archived_categories = state.archived_categories;

        self.modal_active_description_dirty = false;
        if let Some(active) = state.active_session {
            if !self
                .time_tracker
                .set_active_category_by_id(active.category_id)
            {
                return Err(format!(
                    "SQLite active session references unavailable category {}",
                    active.category_id.0
                ));
            }
            self.time_tracker.set_active_description(active.description);
            self.session.active_session_stable_id = Some(active.stable_id);
            self.begin_active_session_at(active.started_at_utc, false)?;
        } else {
            let _ = self
                .time_tracker
                .set_active_category_by_id(DRIFT_CATEGORY_ID);
            self.begin_active_session_now();
            let category_id = self.time_tracker.active_category_id();
            let description = self.time_tracker.active_description().to_string();
            let started_at = self
                .session
                .active_session_started_at_utc
                .unwrap_or_else(Utc::now);
            let stable_id = sqlite::ensure_tui_active_session(
                &database_path,
                category_id,
                &description,
                started_at,
            )?;
            self.session.active_session_stable_id = Some(stable_id);
        }

        if let Some(state) = sqlite::load_tui_sand_state(&database_path)? {
            let valid_category_ids = self
                .time_tracker
                .categories_for_storage()
                .into_iter()
                .chain(self.archived_categories.iter().cloned())
                .map(|category| category.id)
                .collect();
            self.sand_engine
                .restore_state(&state, &valid_category_ids)?;
        }
        self.pending_day_end_snapshots.clear();
        self.clear_report_snapshot_cache();
        Ok(())
    }

    #[allow(dead_code)]
    fn try_finish_and_exit(&mut self) -> Result<(), String> {
        if self.checkpoint_recovery_active {
            return Err(
                "recovery catch-up is not durably committed; checkpoint retained".to_string(),
            );
        }
        let has_active = self.session.active_session_stable_id.is_some();
        if has_active {
            self.prepare_active_finish_for_exit();
            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
        }
        self.try_flush_current_state()?;
        self.reconcile_all_daily_contributions();
        if let Some(recovery) = self.persistence_recovery.as_ref() {
            return Err(recovery.failure.summary());
        }
        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
        sqlite::clear_tui_checkpoint(&database_path)?;
        Ok(())
    }

    #[allow(dead_code)]
    fn try_detach_and_exit(&mut self) -> Result<(), String> {
        self.prepare_detach_boundary()?;
        self.try_flush_current_state()?;
        self.persist_runtime_checkpoint();
        if let Some(recovery) = self.persistence_recovery.as_ref() {
            return Err(recovery.failure.summary());
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn try_commit_checkpoint_recovery(&mut self) -> Result<(), String> {
        self.commit_checkpoint_recovery_if_ready();
        if let Some(recovery) = self.persistence_recovery.as_ref() {
            return Err(recovery.failure.summary());
        }
        if self.checkpoint_recovery_active {
            return Err(
                "checkpoint recovery remains active after retry; catch-up is not settled"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn export_current_recovery(&mut self, exit_after_export: bool) {
        let result = self.export_emergency_recovery();
        match result {
            Ok(path) => {
                if let Some(recovery) = self.persistence_recovery.as_mut() {
                    recovery.exported_path = Some(path);
                    recovery.export_error = None;
                }
                if exit_after_export {
                    self.recovery_exit_requested = true;
                    self.recovery_exit_error = None;
                }
            }
            Err(error) => {
                if let Some(recovery) = self.persistence_recovery.as_mut() {
                    recovery.export_error = Some(error);
                    recovery.dialog_open = true;
                }
            }
        }
        self.render_needed = true;
    }

    fn export_emergency_recovery(&self) -> Result<PathBuf, String> {
        let recovery = self
            .persistence_recovery
            .as_ref()
            .ok_or_else(|| "there is no persistence failure to export".to_string())?;
        let recovery_dir = storage::get_state_dir().join("recovery");
        fs::create_dir_all(&recovery_dir).map_err(|error| error.to_string())?;
        let now = Utc::now();
        let filename = format!("strata-emergency-{}.json", now.format("%Y%m%dT%H%M%S%.3fZ"));
        let path = recovery_dir.join(filename);

        let categories = emergency_categories(
            self.time_tracker.categories_for_storage(),
            &self.archived_categories,
        );
        let sessions = self
            .time_tracker
            .sessions
            .iter()
            .map(|session| EmergencySession {
                id: session.id,
                date: session.date.clone(),
                category_id: session.category_id.0,
                description: session.description.clone(),
                start_time: session.start_time.clone(),
                end_time: session.end_time.clone(),
                elapsed_seconds: session.elapsed_seconds,
            })
            .collect();
        let active = self
            .session
            .active_session_started_at_utc
            .map(|started_at| EmergencyActiveSession {
                stable_id: self.session.active_session_stable_id.clone(),
                category_id: self.time_tracker.active_category_id().0,
                description: self.time_tracker.active_description().to_string(),
                started_at_utc: started_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            });
        let pending_mutations = Vec::new();
        let bundle = EmergencyRecoveryBundle {
            schema_version: 3,
            created_at_utc: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            failure: EmergencyFailure {
                operation: recovery.failure.operation.to_string(),
                class: recovery.failure.class,
                detail: recovery.failure.detail.clone(),
                authority_path: recovery
                    .failure
                    .authority_path
                    .as_ref()
                    .map(|path| path.display().to_string()),
                occurred_at_utc: recovery.failure.occurred_at_utc.clone(),
            },
            categories,
            category_tags: self.category_tags.clone(),
            sessions,
            active_session: active,
            sand_state: self.sand_engine.snapshot_state(),
            simulation_time_utc: self
                .simulation
                .simulation_time_utc
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            pending_mutations,
            checkpoint_recovery_active: self.checkpoint_recovery_active,
            recovery_statement: self.recovery_statement.clone(),
        };
        write_private_json_atomic(&path, &bundle)?;
        Ok(path)
    }

    pub(super) fn render_persistence_status(&self, frame: &mut Frame, size: Rect) {
        let Some(recovery) = self.persistence_recovery.as_ref() else {
            return;
        };
        if recovery.dialog_open || size.width < 18 || size.height < 3 {
            return;
        }
        let label = "Saving paused";
        let width = u16::try_from(label.chars().count()).unwrap_or(13);
        let area = Rect::new(
            size.x.saturating_add(2),
            size.y.saturating_add(size.height.saturating_sub(2)),
            width.min(size.width.saturating_sub(4)),
            1,
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                label,
                Style::default()
                    .fg(self.theme_warning())
                    .add_modifier(Modifier::BOLD),
            ))),
            area,
        );
    }

    pub(super) fn render_persistence_recovery(&self, frame: &mut Frame, size: Rect) {
        let Some(recovery) = self.persistence_recovery.as_ref() else {
            return;
        };
        if !recovery.dialog_open {
            return;
        }
        let last_local = recovery.last_durable_at_utc.with_timezone(&Local);
        let last_label = last_local.format("%H:%M").to_string();
        let at_risk_seconds = (Utc::now() - recovery.last_durable_at_utc)
            .num_seconds()
            .max(0);
        let at_risk = self.format_time(usize::try_from(at_risk_seconds).unwrap_or(usize::MAX));
        let timeline = Line::from(Span::styled(
            "●────────────────?",
            Style::default().fg(self.theme_error()),
        ));
        let values = Line::from(format!("{last_label}    {at_risk}    now"));
        let mut body = vec![
            timeline,
            values,
            Line::default(),
            Line::from(format!(
                "Changes since {last_label} are still held in memory."
            )),
        ];
        if let Some(path) = recovery.exported_path.as_ref() {
            body.push(Line::from(format!("Recovery exported: {}", path.display())));
        }
        if let Some(error) = recovery.export_error.as_ref() {
            body.push(Line::from(Span::styled(
                format!("Recovery export failed: {error}"),
                Style::default().fg(self.theme_error()),
            )));
        }
        self.render_system_dialog(
            frame,
            size,
            super::SystemDialogSeverity::Error,
            Line::from(Span::styled(
                "Saving failed",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            body,
            vec![
                Line::from("Continue"),
                Line::from("Retry now"),
                Line::from("Export recovery"),
                Line::from("Exit without saving"),
            ],
            self.system_dialog_selected_index,
            42,
        );
    }
}

fn write_private_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|error| error.to_string())?;
    }

    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(&json).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())?;
        Ok(())
    })();

    if result.is_err() {
        fs::remove_file(&temporary).ok();
    }
    result
}

fn classify_failure(detail: &str) -> PersistenceFailureClass {
    let normalized = detail.to_ascii_lowercase();
    if normalized.contains("locked") || normalized.contains("busy") {
        PersistenceFailureClass::Busy
    } else if normalized.contains("readonly") || normalized.contains("read-only") {
        PersistenceFailureClass::ReadOnly
    } else if normalized.contains("malformed")
        || normalized.contains("corrupt")
        || normalized.contains("not a database")
    {
        PersistenceFailureClass::Corrupt
    } else if normalized.contains("constraint")
        || normalized.contains("foreign key")
        || normalized.contains("unique")
    {
        PersistenceFailureClass::Constraint
    } else if normalized.contains("changed concurrently")
        || normalized.contains("authority conflict")
        || normalized.contains("expected") && normalized.contains("found")
    {
        PersistenceFailureClass::Conflict
    } else if normalized.contains("commit") {
        PersistenceFailureClass::Commit
    } else if normalized.contains("i/o")
        || normalized.contains("disk")
        || normalized.contains("permission")
        || normalized.contains("denied")
    {
        PersistenceFailureClass::Io
    } else if normalized.contains("invalid")
        || normalized.contains("unsupported")
        || normalized.contains("outside")
        || normalized.contains("no active stable identity")
    {
        PersistenceFailureClass::InvalidData
    } else {
        PersistenceFailureClass::Unknown
    }
}

#[derive(Serialize)]
struct EmergencyRecoveryBundle {
    schema_version: u8,
    created_at_utc: String,
    failure: EmergencyFailure,
    categories: Vec<EmergencyCategory>,
    category_tags: storage::CategoryTagsState,
    sessions: Vec<EmergencySession>,
    active_session: Option<EmergencyActiveSession>,
    sand_state: crate::sand::SandState,
    simulation_time_utc: String,
    pending_mutations: Vec<serde_json::Value>,
    checkpoint_recovery_active: bool,
    recovery_statement: Option<RecoveryStatement>,
}

#[derive(Serialize)]
struct EmergencyFailure {
    operation: String,
    class: PersistenceFailureClass,
    detail: String,
    authority_path: Option<String>,
    occurred_at_utc: String,
}

#[derive(Serialize)]
struct EmergencyCategory {
    id: u64,
    name: String,
    description: String,
    color: String,
    balance_effect: i8,
    archived: bool,
}

#[derive(Serialize)]
struct EmergencySession {
    id: usize,
    date: String,
    category_id: u64,
    description: String,
    start_time: String,
    end_time: String,
    elapsed_seconds: usize,
}

#[derive(Serialize)]
struct EmergencyActiveSession {
    stable_id: Option<String>,
    category_id: u64,
    description: String,
    started_at_utc: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn recovery_category(id: u64, name: &str, description: &str) -> crate::domain::Category {
        crate::domain::Category {
            id: crate::domain::CategoryId::new(id),
            name: name.to_string(),
            color: if id == 0 {
                Color::White
            } else {
                crate::constants::COLORS[((id - 1) as usize) % crate::constants::COLORS.len()]
            },
            description: description.to_string(),
            balance_effect: if id == 0 { 0 } else { 1 },
        }
    }

    #[test]
    fn emergency_export_categories_preserve_archived_state() {
        let active = vec![
            recovery_category(0, "idle", ""),
            recovery_category(1, "Active", "current"),
        ];
        let archived = vec![recovery_category(7, "Archived", "historical")];
        let exported = emergency_categories(active, &archived);
        assert_eq!(exported.len(), 3);
        assert!(
            exported
                .iter()
                .any(|category| category.id == 7 && category.archived)
        );
        assert!(
            exported
                .iter()
                .filter(|category| category.id != 7)
                .all(|category| !category.archived)
        );
    }

    #[test]
    fn emergency_export_schema_three_carries_exact_recovery_statement() {
        let captured = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 8, 3, 18, 0, 2).unwrap();
        let target = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 8, 3, 18, 0, 7).unwrap();
        let statement = RecoveryStatement {
            profile_id: crate::profile::profile_id(),
            checkpoint_captured_at_utc: captured,
            checkpoint_simulation_at_utc: captured,
            recovery_target_utc: target,
            reconstructed_duration_nanos: 5_000_000_000,
            recovered_interval_class: super::super::RecoveredIntervalClass::Reconstructed,
            post_target_class: super::super::PostTargetClass::ProvisionalLiveTime,
            active_stable_id: Some("stable-1".to_string()),
            active_category_id: 1,
            active_description: "Focused".to_string(),
            active_session_started_at_utc: captured,
            cutoff_policy: "persisted target; no post-target time is counted as recovered"
                .to_string(),
        };
        let bundle = EmergencyRecoveryBundle {
            schema_version: 3,
            created_at_utc: target.to_rfc3339(),
            failure: EmergencyFailure {
                operation: "checkpoint recovery".to_string(),
                class: PersistenceFailureClass::Commit,
                detail: "injected".to_string(),
                authority_path: None,
                occurred_at_utc: target.to_rfc3339(),
            },
            categories: Vec::new(),
            category_tags: storage::CategoryTagsState::default(),
            sessions: Vec::new(),
            active_session: Some(EmergencyActiveSession {
                stable_id: Some("stable-1".to_string()),
                category_id: 1,
                description: "Focused".to_string(),
                started_at_utc: captured.to_rfc3339(),
            }),
            sand_state: crate::sand::SandState {
                version: crate::sand::SandState::VERSION,
                grid_width: 2,
                grid_height: 2,
                grains: Vec::new(),
                frame_count: 0,
                sweep_left_to_right: true,
                rng_state: 1,
                ingress_focus_x: None,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
                active_avalanche_columns: Vec::new(),
                mobilized_grains: Vec::new(),
                classic_runtime: None,
            },
            simulation_time_utc: captured.to_rfc3339(),
            pending_mutations: Vec::new(),
            checkpoint_recovery_active: true,
            recovery_statement: Some(statement),
        };
        let value = serde_json::to_value(bundle).unwrap();
        assert_eq!(value["schema_version"], 3);
        let exported_target = chrono::DateTime::parse_from_rfc3339(
            value["recovery_statement"]["recovery_target_utc"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
        .with_timezone(&Utc);
        assert_eq!(exported_target, target);
        assert_eq!(
            value["recovery_statement"]["recovered_interval_class"],
            "reconstructed"
        );
        assert_eq!(
            value["recovery_statement"]["post_target_class"],
            "provisional-live-time"
        );
    }

    #[test]
    fn persistence_failure_classes_are_actionable() {
        assert_eq!(
            classify_failure("database is locked"),
            PersistenceFailureClass::Busy
        );
        assert_eq!(
            classify_failure("attempt to write a readonly database"),
            PersistenceFailureClass::ReadOnly
        );
        assert_eq!(
            classify_failure("database disk image is malformed"),
            PersistenceFailureClass::Corrupt
        );
        assert_eq!(
            classify_failure("injected commit failure"),
            PersistenceFailureClass::Commit
        );
        assert_eq!(
            classify_failure("active session changed concurrently; expected a, found b"),
            PersistenceFailureClass::Conflict
        );
        assert_eq!(
            classify_failure("FOREIGN KEY constraint failed"),
            PersistenceFailureClass::Constraint
        );
        assert_eq!(
            classify_failure("database or disk is full"),
            PersistenceFailureClass::Io
        );
        assert_eq!(
            classify_failure("invalid runtime transition"),
            PersistenceFailureClass::InvalidData
        );
        assert_eq!(
            classify_failure("unrecognized persistence response"),
            PersistenceFailureClass::Unknown
        );
    }
}
