use std::{
    fmt, fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use serde::Serialize;

use crate::{
    domain::{DRIFT_CATEGORY_ID, is_drift_category_id, operational_day_key_now},
    sqlite, storage,
};

use super::{App, QueuedMutation, QueuedMutationEventRecord, QueuedMutationRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PersistenceOperation {
    RuntimeWrite,
    StateReload,
    ActiveStart,
    ActiveFinish,
    ActiveSwitch,
    ActiveReset,
    CategorySync,
    CategoryArchive,
    CategoryTagsSync,
    SessionSync,
    SessionEdit,
    SessionDelete,
    DriftSessionDelete,
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
            Self::CategorySync => "category synchronization",
            Self::CategoryArchive => "category archive",
            Self::CategoryTagsSync => "category-tag synchronization",
            Self::SessionSync => "session synchronization",
            Self::SessionEdit => "session edit",
            Self::SessionDelete => "session deletion",
            Self::DriftSessionDelete => "drift-session deletion",
            Self::SandStateSave => "sediment-state save",
            Self::DailySnapshotSave => "daily sediment snapshot save",
            Self::DailySnapshotDelete => "daily sediment snapshot deletion",
            Self::CheckpointSave => "detached checkpoint save",
            Self::CheckpointClear => "checkpoint cleanup",
            Self::CheckpointRecovery => "checkpoint recovery commit",
        })
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

impl RecoveryAction {
    fn label(self) -> &'static str {
        match self {
            Self::FlushCurrentState => "retry current state",
            Self::ReloadAuthority => "reload SQLite authority",
            Self::FinishAndExit => "retry finish and exit",
            Self::DetachAndExit => "retry detach and exit",
            Self::CommitCheckpointRecovery => "retry checkpoint completion",
        }
    }
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
    pub exit_without_saving_armed: bool,
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
            Ok(value) => Some(value),
            Err(detail) => {
                if self.persistence_recovery.is_none() {
                    self.persistence_recovery = Some(PersistenceRecoveryState {
                        failure: PersistenceFailure::new(self, operation, detail),
                        action,
                        exported_path: None,
                        export_error: None,
                        exit_without_saving_armed: false,
                    });
                    self.render_needed = true;
                }
                None
            }
        }
    }

    pub(super) fn begin_manual_persistence_failure(
        &mut self,
        operation: PersistenceOperation,
        action: RecoveryAction,
        detail: impl Into<String>,
    ) {
        if self.persistence_recovery.is_none() {
            self.persistence_recovery = Some(PersistenceRecoveryState {
                failure: PersistenceFailure::new(self, operation, detail),
                action,
                exported_path: None,
                export_error: None,
                exit_without_saving_armed: false,
            });
            self.render_needed = true;
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

    pub(super) fn handle_persistence_recovery_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('r' | 'R') => {
                self.retry_persistence_failure();
                self.recovery_exit_requested
            }
            KeyCode::Char('e' | 'E') => {
                self.export_current_recovery(false);
                false
            }
            KeyCode::Char('q' | 'Q') => {
                self.export_current_recovery(true);
                self.recovery_exit_requested
            }
            KeyCode::Char('x' | 'X') => {
                let armed = self
                    .persistence_recovery
                    .as_ref()
                    .is_some_and(|recovery| recovery.exit_without_saving_armed);
                if armed {
                    self.recovery_exit_requested = true;
                    self.recovery_exit_error = Some(
                        "exited without confirming authoritative persistence after explicit user confirmation"
                            .to_string(),
                    );
                    true
                } else {
                    if let Some(recovery) = self.persistence_recovery.as_mut() {
                        recovery.exit_without_saving_armed = true;
                        recovery.export_error = None;
                    }
                    self.render_needed = true;
                    false
                }
            }
            _ => false,
        }
    }

    fn retry_persistence_failure(&mut self) {
        let Some(previous) = self.persistence_recovery.take() else {
            return;
        };
        let operation = previous.failure.operation;
        let action = previous.action;
        let exported_path = previous.exported_path;
        let result = match action {
            RecoveryAction::FlushCurrentState => self.try_flush_current_state(),
            RecoveryAction::ReloadAuthority => self.try_reload_authority(),
            RecoveryAction::FinishAndExit => self.try_finish_and_exit(),
            RecoveryAction::DetachAndExit => self.try_detach_and_exit(),
            RecoveryAction::CommitCheckpointRecovery => self.try_commit_checkpoint_recovery(),
        };

        if let Some(recovery) = self.persistence_recovery.as_mut() {
            if recovery.exported_path.is_none() {
                recovery.exported_path = exported_path;
            }
            recovery.export_error = None;
            recovery.exit_without_saving_armed = false;
            self.render_needed = true;
            return;
        }

        match result {
            Ok(()) => {
                self.persistence_recovery = None;
                self.render_needed = true;
                if matches!(
                    action,
                    RecoveryAction::FinishAndExit | RecoveryAction::DetachAndExit
                ) {
                    self.recovery_exit_requested = true;
                    self.recovery_exit_error = None;
                }
            }
            Err(detail) => {
                self.persistence_recovery = Some(PersistenceRecoveryState {
                    failure: PersistenceFailure::new(self, operation, detail),
                    action,
                    exported_path,
                    export_error: None,
                    exit_without_saving_armed: false,
                });
                self.render_needed = true;
            }
        }
    }

    fn try_flush_current_state(&mut self) -> Result<(), String> {
        let categories = self.time_tracker.categories_for_storage();
        let mut state = self.sand_engine.snapshot_state();
        if is_drift_category_id(self.time_tracker.active_category_id()) {
            state.grains.retain(|grain| grain.category_id != 0);
        }
        let operational_day = operational_day_key_now().format("%Y-%m-%d").to_string();

        if let Some(database_path) = self.sqlite_database_path.clone() {
            self.archived_categories = sqlite::sync_tui_categories(
                &database_path,
                &categories,
                self.time_tracker.active_category_id(),
                self.session.active_session_stable_id.as_deref(),
            )?;
            let category_ids = categories
                .iter()
                .map(|category| category.id)
                .collect::<Vec<_>>();
            sqlite::sync_tui_category_tags(&database_path, &self.category_tags, &category_ids)?;
            sqlite::sync_tui_sessions(&database_path, &self.time_tracker.sessions)?;
            sqlite::save_tui_sand_state(&database_path, &self.sand_engine.snapshot_state())?;
            sqlite::save_tui_daily_snapshot(&database_path, &operational_day, &state)?;
        } else {
            storage::save_categories_to_csv(&storage::get_categories_path(), &categories)
                .map_err(|error| error.to_string())?;
            storage::save_category_tags(&storage::get_category_tags_path(), &self.category_tags)?;
            storage::save_sessions_to_csv(
                &storage::get_time_log_path(),
                &self.time_tracker.sessions,
                &categories,
            )
            .map_err(|error| error.to_string())?;
            storage::save_sand_state(
                &storage::get_sand_state_path(),
                &self.sand_engine.snapshot_state(),
            )?;
            storage::save_sand_state(
                &storage::get_sand_history_path_for_day(operational_day_key_now()),
                &state,
            )?;
        }
        Ok(())
    }

    fn try_reload_authority(&mut self) -> Result<(), String> {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let state = sqlite::load_tui_state(&database_path)?;
            self.time_tracker.apply_loaded_state(
                state.loaded_categories.categories,
                state.loaded_categories.next_category_id,
                state.loaded_sessions.sessions,
                state.loaded_sessions.next_session_id,
            );
            self.category_tags = state.category_tags;
            self.archived_categories = state.archived_categories;

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
                let _ = self
                    .time_tracker
                    .set_category_description_by_id(active.category_id, active.description);
                self.session.active_session_stable_id = Some(active.stable_id);
                self.begin_active_session_at(active.started_at_utc);
            } else {
                let _ = self
                    .time_tracker
                    .set_active_category_by_id(DRIFT_CATEGORY_ID);
                self.begin_active_session_now();
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
                self.sand_engine.restore_state(&state, &valid_category_ids);
            }
        } else {
            let categories = storage::try_load_categories_from_csv(&storage::get_categories_path())
                .map_err(|error| error.to_string())?;
            let sessions = storage::try_load_sessions_from_csv(
                &storage::get_time_log_path(),
                &categories.categories,
            )
            .map_err(|error| error.to_string())?;
            self.time_tracker.apply_loaded_state(
                categories.categories,
                categories.next_category_id,
                sessions.sessions,
                sessions.next_session_id,
            );
            self.category_tags = storage::load_category_tags(&storage::get_category_tags_path());
            if let Some(state) = storage::load_sand_state(&storage::get_sand_state_path()) {
                let valid_category_ids = self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .map(|category| category.id)
                    .collect();
                self.sand_engine.restore_state(&state, &valid_category_ids);
            }
        }
        self.sync_drift_idle_state();
        Ok(())
    }

    fn try_finish_and_exit(&mut self) -> Result<(), String> {
        if self.checkpoint_recovery_active {
            return Err(
                "recovery catch-up is not durably committed; checkpoint retained".to_string(),
            );
        }
        if self.session.active_session_stable_id.is_some() {
            self.end_active_session_now();
            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
        }
        self.try_flush_current_state()?;
        if let Some(database_path) = self.sqlite_database_path.clone() {
            sqlite::clear_tui_checkpoint(&database_path)?;
        } else {
            storage::delete_file_if_exists(&storage::get_detached_runtime_path())?;
        }
        Ok(())
    }

    fn try_detach_and_exit(&mut self) -> Result<(), String> {
        self.try_flush_current_state()?;
        self.persist_detached_checkpoint();
        if let Some(recovery) = self.persistence_recovery.as_ref() {
            return Err(recovery.failure.summary());
        }
        Ok(())
    }

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
                    recovery.exit_without_saving_armed = false;
                }
                if exit_after_export {
                    self.recovery_exit_requested = true;
                    self.recovery_exit_error = None;
                }
            }
            Err(error) => {
                if let Some(recovery) = self.persistence_recovery.as_mut() {
                    recovery.export_error = Some(error);
                    recovery.exit_without_saving_armed = false;
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

        let categories = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| EmergencyCategory {
                id: category.id.0,
                name: category.name,
                description: category.description,
                color: format!("{:?}", category.color),
                balance_effect: category.karma_effect,
            })
            .collect();
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
                description: self
                    .time_tracker
                    .category_description_by_id(self.time_tracker.active_category_id())
                    .unwrap_or_default()
                    .to_string(),
                started_at_utc: started_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            });
        let pending_mutations = self
            .simulation
            .pending_mutations
            .iter()
            .map(|event| QueuedMutationEventRecord {
                execute_at_utc: event.execute_at_utc,
                mutation: match event.mutation {
                    QueuedMutation::SwitchLayer(category_id) => QueuedMutationRecord::SwitchLayer {
                        category_id: category_id.0,
                    },
                    QueuedMutation::ClearAllSand => QueuedMutationRecord::ClearAllSand,
                    QueuedMutation::ClearDriftSand => QueuedMutationRecord::ClearDriftSand,
                },
            })
            .collect();
        let bundle = EmergencyRecoveryBundle {
            schema_version: 1,
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
        };
        write_private_json_atomic(&path, &bundle)?;
        Ok(path)
    }

    pub(super) fn render_persistence_recovery(&self, frame: &mut Frame, size: Rect) {
        let Some(recovery) = self.persistence_recovery.as_ref() else {
            return;
        };
        let width = size.width.saturating_sub(4).clamp(36, 96);
        let height = size.height.saturating_sub(4).clamp(14, 22);
        let area = centered_rect(width, height, size);
        frame.render_widget(Clear, area);

        let mut lines = vec![
            Line::from(Span::styled(
                "AUTHORITATIVE PERSISTENCE FAILED",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Operation: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(recovery.failure.operation.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Class: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(recovery.failure.class.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Authority: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(
                    recovery
                        .failure
                        .authority_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "legacy file authority".to_string()),
                ),
            ]),
            Line::from(""),
            Line::from(recovery.failure.detail.clone()),
            Line::from(""),
            Line::from(Span::styled(
                "Normal controls are disabled. The visible state is not claimed durable.",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(format!("[R] {}", recovery.action.label())),
            Line::from("[E] write emergency recovery JSON and remain open"),
            Line::from("[Q] write emergency recovery JSON and exit"),
        ];
        if recovery.exit_without_saving_armed {
            lines.push(Line::from(Span::styled(
                "[X] press again to exit without a recovery export",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(Line::from("[X] arm exit without saving"));
        }
        if let Some(path) = &recovery.exported_path {
            lines.push(Line::from(Span::styled(
                format!("Exported: {}", path.display()),
                Style::default().fg(Color::Green),
            )));
        }
        if let Some(error) = &recovery.export_error {
            lines.push(Line::from(Span::styled(
                format!("Export failed: {error}"),
                Style::default().fg(Color::Red),
            )));
        }

        let block = Block::default()
            .title(" Persistence recovery ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Red));
        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
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

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(area.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
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
    pending_mutations: Vec<QueuedMutationEventRecord>,
    checkpoint_recovery_active: bool,
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
    }
}
