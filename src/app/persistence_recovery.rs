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
    CategoryTagsSync,
    SessionSync,
    SessionEdit,
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
            Self::CategoryTagsSync => "category-tag synchronization",
            Self::SessionSync => "session synchronization",
            Self::SessionEdit => "session edit",
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

    pub(super) fn request_persistence_recovery_quit(&mut self) -> bool {
        self.export_current_recovery(true);
        self.recovery_exit_requested
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
            KeyCode::Char('q' | 'Q') => self.request_persistence_recovery_quit(),
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

    fn try_detach_and_exit(&mut self) -> Result<(), String> {
        self.prepare_detach_boundary()?;
        self.try_flush_current_state()?;
        self.persist_runtime_checkpoint();
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
                        .unwrap_or_else(|| "SQLite database".to_string()),
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
