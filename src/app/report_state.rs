use std::collections::BTreeSet;

use chrono::{
    DateTime, Duration as ChronoDuration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone,
    Timelike, Utc,
};
use ratatui::{prelude::Line, style::Color};

use crate::domain::{
    BalanceReportSummary, Category, CategoryId, CategoryLogEntry, DRIFT_CATEGORY_ID,
    LiveSessionPreview, OperationalDayPolicy, ReportPeriod, ReportWindow,
    build_balance_report_with_live_for_window, build_category_logs_for_window, day_boundary_config,
    operational_day_key_now, report_period_window_with_offset, session_slices,
};
use crate::sand::{
    DailySedimentSlice, SedimentSnapshot, daily_contribution_from_slices,
    derived_preview_from_slices, select_historical_visual_artifact,
};
use crate::temporal;

use super::{App, PersistenceOperation, RecoveryAction};

fn parse_report_range(from: &str, to: &str) -> Result<ReportWindow, String> {
    let parse = |label: &str, value: &str| {
        NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
            .map_err(|_| format!("{label} must use YYYY-MM-DD"))
    };
    let start = parse("From", from)?;
    let end = parse("To", to)?;
    if start > end {
        return Err("From must be on or before To".to_string());
    }
    ReportWindow::new(start, end)
}

fn parse_historical_activity_timestamp(
    value: &str,
    policy: OperationalDayPolicy,
) -> Result<DateTime<Utc>, String> {
    let civil = NaiveDateTime::parse_from_str(value.trim(), "%Y-%m-%d %H:%M:%S")
        .map_err(|_| "time must use YYYY-MM-DD HH:MM:SS".to_string())?;
    let offset = FixedOffset::east_opt(policy.utc_offset_seconds)
        .ok_or_else(|| "historical activity has an invalid UTC offset".to_string())?;
    let local = offset
        .from_local_datetime(&civil)
        .single()
        .ok_or_else(|| "historical civil time is not unique".to_string())?;
    Ok(local.with_timezone(&Utc))
}

fn shifted_custom_window_older(window: &ReportWindow) -> Option<ReportWindow> {
    let width_days = (window.end - window.start).num_days().saturating_add(1);
    let delta = ChronoDuration::days(width_days);
    let start = window.start.checked_sub_signed(delta)?;
    let end = window.end.checked_sub_signed(delta)?;
    ReportWindow::new(start, end).ok()
}

fn shifted_custom_window_newer(window: &ReportWindow, today: NaiveDate) -> Option<ReportWindow> {
    if window.end >= today {
        return None;
    }
    let width_days = (window.end - window.start).num_days().saturating_add(1);
    let delta = ChronoDuration::days(width_days);
    let candidate_end = window.end.checked_add_signed(delta)?;
    let end = candidate_end.min(today);
    let start = end.checked_sub_signed(ChronoDuration::days(width_days.saturating_sub(1)))?;
    ReportWindow::new(start, end).ok()
}

impl App {
    pub(super) fn focus_none_report_row(&mut self) {
        let summary = self.report_rows();
        self.report_selected_index = summary
            .entries
            .iter()
            .position(|entry| entry.category_id == DRIFT_CATEGORY_ID)
            .unwrap_or(0);
        self.clamp_report_selection(summary.entries.len());
    }

    fn report_categories(&self) -> Vec<Category> {
        let mut categories = self.time_tracker.categories_for_storage();
        categories.extend(self.archived_categories.iter().cloned());
        categories
    }

    pub(super) fn category_color_for_id(&self, category_id: CategoryId) -> Color {
        self.time_tracker
            .category_color_by_id(category_id)
            .or_else(|| {
                self.archived_categories
                    .iter()
                    .find(|category| category.id == category_id)
                    .map(|category| category.color)
            })
            .unwrap_or(Color::White)
    }

    pub(super) fn current_report_window(&self) -> ReportWindow {
        self.report_custom_window.clone().unwrap_or_else(|| {
            report_period_window_with_offset(self.report_period, self.report_period_offset)
        })
    }

    pub(super) fn report_range_is_custom(&self) -> bool {
        self.report_custom_window.is_some()
    }

    pub(super) fn report_rows(&self) -> BalanceReportSummary {
        let categories = self.report_categories();
        let live_preview = self.live_session_preview();
        let window = self.current_report_window();

        build_balance_report_with_live_for_window(
            &self.time_tracker.sessions,
            &categories,
            &window,
            live_preview.as_ref(),
        )
    }

    pub(super) fn report_logs_for_category(
        &self,
        category_id: CategoryId,
    ) -> Vec<CategoryLogEntry> {
        let categories = self.report_categories();
        let live_preview = self.live_session_preview();

        let window = self.current_report_window();
        build_category_logs_for_window(
            &self.time_tracker.sessions,
            &categories,
            category_id,
            &window,
            live_preview.as_ref(),
        )
    }

    pub(super) fn report_current_logs(&self) -> Vec<CategoryLogEntry> {
        let Some(category_id) = self.report_logs_category_id else {
            return Vec::new();
        };
        self.report_logs_for_category(category_id)
    }

    fn live_session_preview(&self) -> Option<LiveSessionPreview> {
        let start = self.time_tracker.current_session_start?;
        let elapsed_seconds = start.elapsed().as_secs() as usize;
        if elapsed_seconds == 0 {
            return None;
        }

        let category_id = self.time_tracker.active_category_id();
        let description = self.time_tracker.active_description().to_string();

        let started_at_utc = self.session.active_session_started_at_utc?;
        let ended_at_utc = started_at_utc + ChronoDuration::seconds(elapsed_seconds as i64);
        Some(LiveSessionPreview {
            category_id,
            description,
            elapsed_seconds,
            started_at_utc,
            ended_at_utc,
            operational_day_policy: OperationalDayPolicy::from_config(day_boundary_config()),
        })
    }

    fn historical_activity_targets(&self) -> Vec<Category> {
        self.time_tracker.categories_ordered()
    }

    pub(super) fn historical_activity_target_name(&self) -> Option<String> {
        let edit = self.historical_activity_edit.as_ref()?;
        self.time_tracker
            .category_by_id(edit.target_category_id)
            .map(|category| self.display_layer_name(&category.name))
    }

    pub(super) fn historical_activity_conflict_labels(&self) -> Vec<String> {
        let Some(confirmation) = self
            .historical_activity_edit
            .as_ref()
            .and_then(|edit| edit.confirmation.as_ref())
        else {
            return Vec::new();
        };
        let policy = OperationalDayPolicy::from_config(day_boundary_config());
        confirmation
            .conflicts
            .iter()
            .map(|conflict| {
                let name = self
                    .time_tracker
                    .category_by_id(conflict.category_id)
                    .map(|category| self.display_layer_name(&category.name))
                    .or_else(|| {
                        self.archived_categories
                            .iter()
                            .find(|category| category.id == conflict.category_id)
                            .map(|category| self.display_layer_name(&category.name))
                    })
                    .unwrap_or_else(|| format!("layer {}", conflict.category_id.0));
                let start = temporal::civil_from_policy(conflict.started_at_utc, policy).ok();
                let end = temporal::civil_from_policy(conflict.ended_at_utc, policy).ok();
                let interval = match (start, end) {
                    (Some(start), Some(end)) if start.date_naive() == end.date_naive() => format!(
                        "{} {}-{}",
                        start.format("%Y-%m-%d"),
                        start.format("%H:%M:%S"),
                        end.format("%H:%M:%S")
                    ),
                    (Some(start), Some(end)) => format!(
                        "{}-{}",
                        start.format("%Y-%m-%d %H:%M:%S"),
                        end.format("%Y-%m-%d %H:%M:%S")
                    ),
                    _ => "?".to_string(),
                };
                if conflict.active {
                    format!("{name} {interval} (current)")
                } else {
                    format!("{name} {interval}")
                }
            })
            .collect()
    }

    pub(super) fn cycle_historical_activity_target(&mut self, direction: isize) {
        let targets = self.historical_activity_targets();
        if targets.is_empty() {
            return;
        }
        let Some(edit) = self.historical_activity_edit.as_mut() else {
            return;
        };
        let current = targets
            .iter()
            .position(|category| category.id == edit.target_category_id)
            .unwrap_or(0);
        let next = if direction < 0 {
            if current == 0 {
                targets.len() - 1
            } else {
                current - 1
            }
        } else {
            (current + 1) % targets.len()
        };
        edit.target_category_id = targets[next].id;
        edit.error = None;
        edit.confirmation = None;
        self.render_needed = true;
    }

    pub(super) fn begin_historical_activity_edit(&mut self) -> bool {
        let targets = self.historical_activity_targets();
        if targets.is_empty() {
            return false;
        }
        let preview = match self.historical_correction_active_preview() {
            Ok(preview) => preview,
            Err(_) => return false,
        };
        let target_category_id = if preview.category_id != DRIFT_CATEGORY_ID {
            preview.category_id
        } else {
            targets
                .iter()
                .find(|category| category.id != DRIFT_CATEGORY_ID)
                .map(|category| category.id)
                .unwrap_or(DRIFT_CATEGORY_ID)
        };
        let to = preview.ended_at_utc;
        let from = to
            .checked_sub_signed(ChronoDuration::minutes(15))
            .unwrap_or(to);
        let format_civil = |timestamp| {
            temporal::civil_from_policy(timestamp, preview.operational_day_policy)
                .map(|civil| civil.format("%Y-%m-%d %H:%M:%S").to_string())
        };
        let Ok(from) = format_civil(from) else {
            return false;
        };
        let Ok(to) = format_civil(to) else {
            return false;
        };
        self.report_range_edit = None;
        self.report_log_edit = None;
        self.historical_activity_edit = Some(super::HistoricalActivityEditState {
            target_category_id,
            from,
            to,
            active_field: super::HistoricalActivityField::Layer,
            select_all: false,
            error: None,
            confirmation: None,
        });
        self.render_needed = true;
        true
    }

    pub(super) fn cancel_historical_activity_edit(&mut self) {
        self.historical_activity_edit = None;
        self.render_needed = true;
    }

    pub(super) fn dismiss_historical_activity_confirmation(&mut self) {
        if let Some(edit) = self.historical_activity_edit.as_mut() {
            edit.confirmation = None;
            edit.error = None;
            self.render_needed = true;
        }
    }

    fn historical_correction_active_preview(
        &self,
    ) -> Result<crate::sqlite::TuiHistoricalActivePreview, String> {
        let stable_id = self
            .session
            .active_session_stable_id
            .clone()
            .ok_or_else(|| "active session has no stable identity".to_string())?;
        let started_at_utc = self
            .session
            .active_session_started_at_utc
            .ok_or_else(|| "active session has no UTC start".to_string())?;
        let started_at_utc = started_at_utc
            .with_nanosecond(started_at_utc.nanosecond() / 1_000_000 * 1_000_000)
            .ok_or_else(|| "active session start cannot be represented".to_string())?;
        let started = self
            .time_tracker
            .current_session_start
            .ok_or_else(|| "active session has no monotonic start".to_string())?;
        let elapsed_seconds = usize::try_from(started.elapsed().as_secs())
            .map_err(|_| "active session duration exceeds this platform's range".to_string())?;
        let elapsed = i64::try_from(elapsed_seconds)
            .map_err(|_| "active session duration exceeds chrono range".to_string())?;
        let ended_at_utc = started_at_utc
            .checked_add_signed(ChronoDuration::seconds(elapsed))
            .ok_or_else(|| "active session end exceeds chrono range".to_string())?;
        Ok(crate::sqlite::TuiHistoricalActivePreview {
            stable_id,
            category_id: self.time_tracker.active_category_id(),
            started_at_utc,
            ended_at_utc,
            elapsed_seconds,
            operational_day_policy: OperationalDayPolicy::from_config(day_boundary_config()),
        })
    }

    pub(super) fn commit_historical_activity_edit(&mut self) -> bool {
        let Some(edit) = self.historical_activity_edit.clone() else {
            return false;
        };
        if let Err(error) = self.settle_transition_boundary(Utc::now()) {
            if let Some(current) = self.historical_activity_edit.as_mut() {
                current.error = Some(error);
            }
            self.render_needed = true;
            return false;
        }
        let active_preview = match self.historical_correction_active_preview() {
            Ok(preview) => preview,
            Err(error) => {
                if let Some(current) = self.historical_activity_edit.as_mut() {
                    current.error = Some(error);
                }
                self.render_needed = true;
                return false;
            }
        };
        let from = match parse_historical_activity_timestamp(
            &edit.from,
            active_preview.operational_day_policy,
        ) {
            Ok(value) => value,
            Err(error) => {
                if let Some(current) = self.historical_activity_edit.as_mut() {
                    current.error = Some(format!("From {error}"));
                }
                self.render_needed = true;
                return false;
            }
        };
        let to = match parse_historical_activity_timestamp(
            &edit.to,
            active_preview.operational_day_policy,
        ) {
            Ok(value) => value,
            Err(error) => {
                if let Some(current) = self.historical_activity_edit.as_mut() {
                    current.error = Some(format!("To {error}"));
                }
                self.render_needed = true;
                return false;
            }
        };
        let validation_error = if from >= to {
            Some("From must be before To".to_string())
        } else if to > active_preview.ended_at_utc {
            Some("To cannot be later than now".to_string())
        } else if self
            .time_tracker
            .category_by_id(edit.target_category_id)
            .is_none()
        {
            Some("choose an active layer".to_string())
        } else {
            None
        };
        if let Some(error) = validation_error {
            if let Some(current) = self.historical_activity_edit.as_mut() {
                current.error = Some(error);
                current.confirmation = None;
            }
            self.render_needed = true;
            return false;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return false;
        };
        let checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                if let Some(current) = self.historical_activity_edit.as_mut() {
                    current.error = Some(error);
                }
                self.render_needed = true;
                return false;
            }
        };
        let checkpoint_json = match serde_json::to_string(&checkpoint) {
            Ok(value) => value,
            Err(error) => {
                if let Some(current) = self.historical_activity_edit.as_mut() {
                    current.error = Some(error.to_string());
                }
                self.render_needed = true;
                return false;
            }
        };
        let description = String::new();
        let confirmed_plan_token = edit
            .confirmation
            .as_ref()
            .map(|confirmation| confirmation.plan_token.clone());
        let result = crate::sqlite::log_tui_historical_activity(
            &database_path,
            crate::sqlite::TuiHistoricalActivityRequest {
                target_category_id: edit.target_category_id,
                started_at_utc: from,
                ended_at_utc: to,
                description,
                active_preview,
                confirmed_plan_token,
                checkpoint_json,
                checkpoint_detached_at_utc: checkpoint.detached_at_utc,
                checkpoint_simulation_time_utc: checkpoint.simulation_time_utc,
            },
        );
        let Some(outcome) = self.record_storage_result_for(
            PersistenceOperation::SessionCorrection,
            RecoveryAction::ReloadAuthority,
            result,
        ) else {
            return false;
        };
        match outcome {
            crate::sqlite::TuiHistoricalActivityOutcome::NeedsConfirmation {
                plan_token,
                conflicts,
            } => {
                if let Some(current) = self.historical_activity_edit.as_mut() {
                    current.confirmation = Some(super::HistoricalActivityConfirmation {
                        plan_token,
                        conflicts,
                    });
                    current.error = None;
                }
                self.render_needed = true;
                false
            }
            crate::sqlite::TuiHistoricalActivityOutcome::Applied(receipt) => {
                let active_start_changed = self.session.active_session_started_at_utc
                    != Some(receipt.resulting_active_started_at_utc);
                self.session.active_session_stable_id =
                    Some(receipt.resulting_active_stable_id.clone());
                if active_start_changed
                    && let Err(error) =
                        self.begin_active_session_at(receipt.resulting_active_started_at_utc, true)
                {
                    if let Some(current) = self.historical_activity_edit.as_mut() {
                        current.error = Some(error);
                    }
                    self.render_needed = true;
                    return false;
                }
                if !self.reload_sqlite_sessions() {
                    return false;
                }
                self.historical_activity_edit = None;
                self.clear_report_snapshot_cache();
                self.sync_report_selection_for_interval();
                true
            }
        }
    }

    pub(super) fn set_report_period(&mut self, period: ReportPeriod) {
        self.report_period = period;
        self.report_period_offset = 0;
        self.report_custom_window = None;
        self.report_range_edit = None;
        self.historical_activity_edit = None;
        self.clear_report_snapshot_cache();
        self.sync_report_selection_for_interval();
    }

    pub(super) fn begin_report_range_edit(&mut self) {
        let window = self.current_report_window();
        self.report_range_edit = Some(super::ReportRangeEditState {
            from: window.start.format("%Y-%m-%d").to_string(),
            to: window.end.format("%Y-%m-%d").to_string(),
            active_field: super::ReportRangeField::From,
            select_all: true,
            error: None,
        });
        self.render_needed = true;
    }

    pub(super) fn cancel_report_range_edit(&mut self) {
        self.report_range_edit = None;
        self.render_needed = true;
    }

    pub(super) fn commit_report_range_edit(&mut self) -> bool {
        let Some(edit) = self.report_range_edit.clone() else {
            return false;
        };
        let window = match parse_report_range(&edit.from, &edit.to) {
            Ok(window) => window,
            Err(error) => {
                if let Some(current) = self.report_range_edit.as_mut() {
                    current.error = Some(error);
                }
                self.render_needed = true;
                return false;
            }
        };
        self.report_custom_window = Some(window);
        self.report_period_offset = 0;
        self.report_range_edit = None;
        self.clear_report_snapshot_cache();
        self.sync_report_selection_for_interval();
        true
    }

    pub(super) fn shift_report_interval_older(&mut self) {
        if let Some(window) = self.report_custom_window.clone() {
            if let Some(shifted) = shifted_custom_window_older(&window) {
                self.report_custom_window = Some(shifted);
            }
        } else {
            self.report_period_offset = self.report_period_offset.saturating_add(1);
        }
        self.clear_report_snapshot_cache();
        self.sync_report_selection_for_interval();
    }

    pub(super) fn can_shift_report_interval_newer(&self) -> bool {
        self.report_custom_window
            .as_ref()
            .map(|window| window.end < operational_day_key_now())
            .unwrap_or(self.report_period_offset > 0)
    }

    pub(super) fn shift_report_interval_newer(&mut self) {
        if let Some(window) = self.report_custom_window.clone() {
            if let Some(shifted) = shifted_custom_window_newer(&window, operational_day_key_now()) {
                self.report_custom_window = Some(shifted);
                self.clear_report_snapshot_cache();
            }
        } else if self.report_period_offset > 0 {
            self.report_period_offset -= 1;
            self.clear_report_snapshot_cache();
        }
        self.sync_report_selection_for_interval();
    }

    pub(super) fn report_snapshot_lines(
        &mut self,
        width: u16,
        height: u16,
        _categories: &[Category],
    ) -> Option<Vec<Line<'static>>> {
        self.refresh_report_snapshot_cache();
        let snapshot = self.report_snapshot_artifact.clone()?;
        let cache_key = snapshot.render_cache_key(width, height);

        let should_rebuild_preview = self
            .report_snapshot_preview_key
            .as_deref()
            .map(|key| key != cache_key.as_str())
            .unwrap_or(true)
            || self.report_snapshot_preview_lines.is_none();

        if should_rebuild_preview {
            let categories = self.report_categories();
            self.report_snapshot_preview_lines =
                Some(snapshot.render_immutable(width, height, &categories));
            self.report_snapshot_preview_key = Some(cache_key);
        }

        self.report_snapshot_preview_lines.clone()
    }

    pub(super) fn report_snapshot_status_label(&self) -> String {
        if !self.should_use_report_snapshot() {
            return "live sediment".to_string();
        }

        self.report_snapshot_artifact
            .as_ref()
            .map(SedimentSnapshot::display_label)
            .unwrap_or_else(|| "historical sediment unavailable".to_string())
    }

    pub(super) fn clear_report_snapshot_cache(&mut self) {
        self.report_snapshot_end_day = None;
        self.report_snapshot_artifact = None;
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_lines = None;
    }

    pub(super) fn report_interval_end_day(&self) -> NaiveDate {
        self.current_report_window().end
    }

    pub(super) fn should_use_report_snapshot(&self) -> bool {
        self.report_interval_end_day() < operational_day_key_now()
    }

    pub(super) fn delete_selected_report_session(&mut self) -> bool {
        let logs = self.report_current_logs();
        if logs.is_empty() {
            return false;
        }

        let selected = self.report_log_selected_index.min(logs.len() - 1);
        let Some(row) = logs.get(selected) else {
            return false;
        };
        let Some(session_id) = row.session_id else {
            return false;
        };
        let removed_seconds = row.elapsed_seconds;
        let Some(category_id) = self.report_logs_category_id else {
            return false;
        };
        let affected_days = self
            .time_tracker
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| {
                session_slices(session)
                    .into_iter()
                    .map(|slice| slice.operational_day)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = crate::sqlite::delete_tui_session(&database_path, session_id);
            if self
                .record_storage_result_for(
                    PersistenceOperation::SessionDelete,
                    RecoveryAction::ReloadAuthority,
                    result,
                )
                .is_none()
            {
                return false;
            }
        }
        if !self.time_tracker.delete_session_by_id(session_id) {
            return false;
        }

        if self.report_interval_end_day() == operational_day_key_now() {
            self.sand_engine
                .remove_category_grains(category_id, removed_seconds);
        }

        for day in affected_days {
            self.reconcile_daily_contribution(day);
        }
        self.clear_report_snapshot_cache();

        let refreshed = self.report_current_logs();
        self.clamp_report_log_selection(refreshed.len());
        true
    }

    pub(super) fn begin_report_log_edit(&mut self) -> bool {
        let logs = self.report_current_logs();
        if logs.is_empty() {
            return false;
        }
        let selected = self.report_log_selected_index.min(logs.len() - 1);
        let Some(row) = logs.get(selected) else {
            return false;
        };
        let Some(session_id) = row.session_id else {
            return false;
        };
        self.report_log_edit = Some(super::ReportLogEditState {
            session_id,
            draft: row.description.clone(),
        });
        self.render_needed = true;
        true
    }

    pub(super) fn cancel_report_log_edit(&mut self) {
        self.report_log_edit = None;
        self.render_needed = true;
    }

    pub(super) fn commit_report_log_edit(&mut self) -> bool {
        let Some(edit) = self.report_log_edit.clone() else {
            return false;
        };
        if !self
            .time_tracker
            .sessions
            .iter()
            .any(|session| session.id == edit.session_id)
        {
            return false;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            retain_report_edit_after_commit(&mut self.report_log_edit, false);
            self.render_needed = true;
            return false;
        };
        let result = crate::sqlite::update_tui_session_description(
            &database_path,
            edit.session_id,
            &edit.draft,
        );
        if self
            .record_storage_result_for(
                PersistenceOperation::SessionEdit,
                RecoveryAction::ReloadAuthority,
                result,
            )
            .is_none()
        {
            retain_report_edit_after_commit(&mut self.report_log_edit, false);
            self.render_needed = true;
            return false;
        }
        if !self
            .time_tracker
            .set_session_description_by_id(edit.session_id, edit.draft)
        {
            return false;
        }
        retain_report_edit_after_commit(&mut self.report_log_edit, true);
        self.render_needed = true;
        true
    }

    fn sync_report_selection_for_interval(&mut self) {
        if self.report_logs_category_id.is_some() {
            let row_count = self.report_current_logs().len();
            self.clamp_report_log_selection(row_count);
        } else {
            let row_count = self.report_rows().entries.len();
            self.clamp_report_selection(row_count);
        }
        self.render_needed = true;
    }

    fn refresh_report_snapshot_cache(&mut self) {
        let end_day = self.report_interval_end_day();
        let key = end_day.format("%Y-%m-%d").to_string();

        if self.report_snapshot_end_day.as_deref() == Some(key.as_str()) {
            return;
        }

        let authentic = self.load_day_end_sediment_snapshot(end_day);
        let derived = self.synthetic_snapshot_from_time_log(end_day);

        self.report_snapshot_end_day = Some(key.clone());
        self.report_snapshot_artifact = select_historical_visual_artifact(&key, authentic, derived);
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_lines = None;
    }

    pub(super) fn daily_contribution_from_time_log(
        &self,
        day: NaiveDate,
    ) -> Option<SedimentSnapshot> {
        let slices = self.daily_sediment_slices(day);
        let day_key = day.format("%Y-%m-%d").to_string();
        daily_contribution_from_slices(&day_key, &slices)
    }

    fn synthetic_snapshot_from_time_log(&self, day: NaiveDate) -> Option<SedimentSnapshot> {
        let slices = self.daily_sediment_slices(day);
        let day_key = day.format("%Y-%m-%d").to_string();
        derived_preview_from_slices(
            &day_key,
            self.sand_engine.grid_width_dots,
            self.sand_engine.grid_height_dots,
            &slices,
        )
    }

    fn daily_sediment_slices(&self, day: NaiveDate) -> Vec<DailySedimentSlice> {
        let mut slices = self
            .time_tracker
            .sessions
            .iter()
            .flat_map(|session| {
                session_slices(session)
                    .into_iter()
                    .filter(move |slice| slice.operational_day == day)
                    .map(move |slice| DailySedimentSlice {
                        category_id: session.category_id.0,
                        elapsed_seconds: slice.elapsed_seconds,
                        start_time: slice.start_time,
                        end_time: slice.end_time,
                        session_id: session.id,
                    })
            })
            .collect::<Vec<_>>();

        if let Some(preview) = self.live_preview_session() {
            slices.extend(
                session_slices(&preview)
                    .into_iter()
                    .filter(|slice| slice.operational_day == day)
                    .map(|slice| DailySedimentSlice {
                        category_id: preview.category_id.0,
                        elapsed_seconds: slice.elapsed_seconds,
                        start_time: slice.start_time,
                        end_time: slice.end_time,
                        session_id: usize::MAX,
                    }),
            );
        }
        slices
    }

    fn live_preview_session(&self) -> Option<crate::domain::Session> {
        let day = operational_day_key_now();
        let live = self.live_session_preview()?;
        Some(crate::domain::Session {
            id: usize::MAX,
            date: day.format("%Y-%m-%d").to_string(),
            category_id: live.category_id,
            description: live.description,
            start_time: String::new(),
            end_time: String::new(),
            elapsed_seconds: live.elapsed_seconds,
            started_at_utc: Some(live.started_at_utc),
            ended_at_utc: Some(live.ended_at_utc),
            operational_day_policy: Some(live.operational_day_policy),
        })
    }

    pub(super) fn daily_contribution_days(&self) -> BTreeSet<NaiveDate> {
        let mut days = self
            .time_tracker
            .sessions
            .iter()
            .flat_map(session_slices)
            .map(|slice| slice.operational_day)
            .collect::<BTreeSet<_>>();
        if let Some(preview) = self.live_preview_session() {
            days.extend(
                session_slices(&preview)
                    .into_iter()
                    .map(|slice| slice.operational_day),
            );
        }
        days
    }

    pub(super) fn reconcile_all_daily_contributions(&mut self) {
        let days = self.daily_contribution_days();
        for day in days {
            self.reconcile_daily_contribution(day);
            if self.has_persistence_recovery() {
                break;
            }
        }
    }

    pub(super) fn reconcile_daily_contribution(&mut self, day: NaiveDate) {
        let expected = self.daily_contribution_from_time_log(day);
        let existing = self.load_daily_sediment_snapshot(day);
        if existing == expected {
            return;
        }
        match expected {
            Some(snapshot) => self.save_daily_sediment_snapshot(day, &snapshot),
            None => self.delete_daily_sediment_snapshot(day),
        }
    }

    pub(super) fn clamp_report_selection(&mut self, row_count: usize) {
        if row_count == 0 {
            self.report_selected_index = 0;
        } else if self.report_selected_index >= row_count {
            self.report_selected_index = row_count - 1;
        }
    }

    pub(super) fn clamp_report_log_selection(&mut self, row_count: usize) {
        if row_count == 0 {
            self.report_log_selected_index = 0;
        } else if self.report_log_selected_index >= row_count {
            self.report_log_selected_index = row_count - 1;
        }
    }
}

fn retain_report_edit_after_commit(edit: &mut Option<super::ReportLogEditState>, committed: bool) {
    if committed {
        *edit = None;
    }
}

#[cfg(test)]
mod report_edit_state_tests {
    use chrono::NaiveDate;

    use super::{
        parse_historical_activity_timestamp, parse_report_range, retain_report_edit_after_commit,
        shifted_custom_window_newer, shifted_custom_window_older,
    };
    use crate::{
        app::ReportLogEditState,
        domain::{OperationalDayPolicy, ReportWindow},
    };

    #[test]
    fn failed_commit_retains_complete_draft() {
        let original = ReportLogEditState {
            session_id: 42,
            draft: "draft 世界".to_string(),
        };
        let mut edit = Some(original.clone());
        retain_report_edit_after_commit(&mut edit, false);
        assert_eq!(edit, Some(original));
    }

    #[test]
    fn successful_commit_closes_edit_mode() {
        let mut edit = Some(ReportLogEditState {
            session_id: 42,
            draft: "done".to_string(),
        });
        retain_report_edit_after_commit(&mut edit, true);
        assert_eq!(edit, None);
    }

    #[test]
    fn custom_range_requires_iso_dates_and_chronological_bounds() {
        let window = parse_report_range("2026-08-01", "2026-08-27").unwrap();
        assert_eq!(window.label, "2026-08-01..2026-08-27");
        assert!(parse_report_range("08/01/2026", "2026-08-27").is_err());
        assert!(parse_report_range("2026-08-28", "2026-08-27").is_err());
    }

    #[test]
    fn historical_activity_timestamp_uses_explicit_civil_second() {
        let policy = OperationalDayPolicy {
            utc_offset_seconds: -6 * 60 * 60,
            start_minutes: 0,
        };
        let parsed = parse_historical_activity_timestamp("2026-08-01 23:45:00", policy).unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-08-02T05:45:00+00:00");
        assert!(parse_historical_activity_timestamp("2026/08/01 23:45", policy).is_err());
    }

    #[test]
    fn custom_range_navigation_preserves_span_and_caps_at_today() {
        let start = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 8, 14).unwrap();
        let window = ReportWindow::new(start, end).unwrap();

        let older = shifted_custom_window_older(&window).unwrap();
        assert_eq!(older.start, NaiveDate::from_ymd_opt(2026, 8, 5).unwrap());
        assert_eq!(older.end, NaiveDate::from_ymd_opt(2026, 8, 9).unwrap());

        let today = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let newer = shifted_custom_window_newer(&window, today).unwrap();
        assert_eq!(newer.start, NaiveDate::from_ymd_opt(2026, 8, 13).unwrap());
        assert_eq!(newer.end, today);
        assert!(shifted_custom_window_newer(&newer, today).is_none());
    }
}
