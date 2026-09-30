use std::collections::{BTreeSet, HashSet};

use chrono::{
    DateTime, Duration as ChronoDuration, FixedOffset, NaiveDate, NaiveTime, TimeZone, Timelike,
    Utc,
};
use ratatui::{prelude::Line, style::Color};

use crate::domain::{
    BalanceReportEntry, BalanceReportSummary, Category, CategoryId, CategoryLogEntry,
    DRIFT_CATEGORY_ID,
    LiveSessionPreview, OperationalDayPolicy, ReportPeriod, ReportWindow,
    build_balance_report_with_live_for_window, build_category_logs_for_window, day_boundary_config,
    operational_day_key_now, report_period_window_with_offset, session_slices,
};
use crate::sand::{
    ClassicProductionEngine, DailySedimentSlice, SedimentSnapshot, daily_contribution_from_slices,
    derived_preview_from_slices, select_historical_visual_artifact,
};
use crate::temporal;
use crate::runtime_identity::transition_identity;

use super::{
    App, HistoricalPreviewState, LedgerSelectionIdentity, PersistenceOperation, RecoveryAction,
    ReportRangeBoundary, ReportTagFacet, tagging, ui_helpers,
};

fn build_historical_preview(
    snapshot: &SedimentSnapshot,
    width: u16,
    height: u16,
    categories: &[Category],
) -> Result<HistoricalPreviewState, String> {
    let mut valid_category_ids = categories
        .iter()
        .map(|category| category.id)
        .collect::<HashSet<CategoryId>>();
    valid_category_ids.insert(DRIFT_CATEGORY_ID);

    let mut engine = ClassicProductionEngine::new_production(width, height);
    engine.restore_state(&snapshot.state, &valid_category_ids)?;
    Ok(HistoricalPreviewState {
        source_key: snapshot.source_revision.clone(),
        engine,
        physics_accumulator: std::time::Duration::ZERO,
    })
}

fn parse_report_range(from: &str, to: &str) -> Result<ReportWindow, String> {
    let parse = |label: &str, value: &str| {
        NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
            .map_err(|_| format!("{label} must use YYYY-MM-DD"))
    };
    let start = parse("From", from)?;
    let end_exclusive = parse("To", to)?;
    if start >= end_exclusive {
        return Err("From must be before To".to_string());
    }
    let end = end_exclusive
        .checked_sub_signed(ChronoDuration::days(1))
        .ok_or_else(|| "To boundary is outside the supported date range".to_string())?;
    ReportWindow::new(start, end)
}

fn shifted_report_boundary_month(
    window: &ReportWindow,
    boundary: ReportRangeBoundary,
    direction: i8,
    today: NaiveDate,
) -> Option<ReportWindow> {
    if direction == 0 {
        return Some(window.clone());
    }
    let sign = i64::from(direction.signum());
    match boundary {
        ReportRangeBoundary::Start => {
            let candidate = crate::temporal::shift_civil_month(window.start, sign)?;
            let start = if sign > 0 {
                candidate.min(window.end)
            } else {
                candidate
            };
            ReportWindow::new(start, window.end).ok()
        }
        ReportRangeBoundary::End => {
            let end_exclusive = ui_helpers::report_window_end_exclusive(window)?;
            let shifted_exclusive = crate::temporal::shift_civil_month(end_exclusive, sign)?;
            let candidate_end = shifted_exclusive.checked_sub_signed(ChronoDuration::days(1))?;
            let end = if sign > 0 {
                candidate_end.min(today)
            } else {
                candidate_end.max(window.start)
            };
            ReportWindow::new(window.start, end).ok()
        }
    }
}

fn shifted_report_boundary(
    window: &ReportWindow,
    boundary: ReportRangeBoundary,
    direction: i8,
    today: NaiveDate,
) -> Option<ReportWindow> {
    match (boundary, direction) {
        (ReportRangeBoundary::Start, -1) => {
            let start = window.start.checked_sub_signed(ChronoDuration::days(1))?;
            ReportWindow::new(start, window.end).ok()
        }
        (ReportRangeBoundary::Start, 1) if window.start < window.end => {
            let start = window.start.checked_add_signed(ChronoDuration::days(1))?;
            ReportWindow::new(start, window.end).ok()
        }
        (ReportRangeBoundary::End, -1) if window.end > window.start => {
            let end = window.end.checked_sub_signed(ChronoDuration::days(1))?;
            ReportWindow::new(window.start, end).ok()
        }
        (ReportRangeBoundary::End, 1) if window.end < today => {
            let end = window.end.checked_add_signed(ChronoDuration::days(1))?;
            ReportWindow::new(window.start, end).ok()
        }
        _ => None,
    }
}

fn report_range_edit_state(window: &ReportWindow) -> super::ReportRangeEditState {
    let (to, error) = match ui_helpers::report_window_end_exclusive(window) {
        Some(end) => (end.format("%Y-%m-%d").to_string(), None),
        None => (
            String::new(),
            Some("To boundary is outside the supported date range".to_string()),
        ),
    };
    super::ReportRangeEditState {
        from: window.start.format("%Y-%m-%d").to_string(),
        to,
        active_field: super::ReportRangeField::From,
        select_all: true,
        error,
    }
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

fn report_tag_facets(description: &str) -> Vec<ReportTagFacet> {
    let tags = tagging::parse_tags(description);
    if tags.is_empty() {
        vec![ReportTagFacet::Untagged]
    } else {
        tags.into_iter().map(ReportTagFacet::Tag).collect()
    }
}

fn report_tag_facet_matches(left: &ReportTagFacet, right: &ReportTagFacet) -> bool {
    match (left, right) {
        (ReportTagFacet::Untagged, ReportTagFacet::Untagged) => true,
        (ReportTagFacet::Tag(left), ReportTagFacet::Tag(right)) => left.eq_ignore_ascii_case(right),
        _ => false,
    }
}

fn report_log_matches_tag_filter(row: &CategoryLogEntry, filter: &[ReportTagFacet]) -> bool {
    filter.is_empty()
        || report_tag_facets(&row.description).iter().any(|facet| {
            filter
                .iter()
                .any(|selected| report_tag_facet_matches(facet, selected))
        })
}

fn filtered_layer_balance(logs: &[CategoryLogEntry], filter: &[ReportTagFacet]) -> isize {
    logs.iter()
        .filter(|row| report_log_matches_tag_filter(row, filter))
        .map(|row| row.balance_seconds)
        .sum()
}

fn report_entry_is_visible(entry: &BalanceReportEntry) -> bool {
    if entry.category_id == DRIFT_CATEGORY_ID {
        entry.elapsed_seconds != 0
    } else {
        entry.balance_seconds != 0
    }
}

impl App {
    pub(super) fn focus_none_report_row(&mut self) {
        let summary = self.report_visible_rows();
        let index = summary
            .entries
            .iter()
            .position(|entry| entry.category_id == DRIFT_CATEGORY_ID)
            .unwrap_or(0);
        self.select_report_summary_index(&summary, index);
    }

    pub(super) fn report_selected_summary_index(
        &self,
        summary: &BalanceReportSummary,
    ) -> Option<usize> {
        if summary.entries.is_empty() {
            return None;
        }
        self.report_selected_category_id
            .and_then(|category_id| {
                summary
                    .entries
                    .iter()
                    .position(|entry| entry.category_id == category_id)
            })
            .or_else(|| Some(self.report_selected_index.min(summary.entries.len() - 1)))
    }

    pub(super) fn sync_report_selection_to_summary(&mut self, summary: &BalanceReportSummary) {
        let Some(index) = self.report_selected_summary_index(summary) else {
            self.report_selected_index = 0;
            self.report_selected_category_id = None;
            return;
        };
        self.report_selected_index = index;
        self.report_selected_category_id = Some(summary.entries[index].category_id);
    }

    pub(super) fn select_report_summary_index(
        &mut self,
        summary: &BalanceReportSummary,
        index: usize,
    ) {
        if summary.entries.is_empty() {
            self.report_selected_index = 0;
            self.report_selected_category_id = None;
            return;
        }
        let index = index.min(summary.entries.len() - 1);
        self.report_selected_index = index;
        self.report_selected_category_id = Some(summary.entries[index].category_id);
    }

    fn report_categories(&self) -> Vec<Category> {
        let mut categories = self.time_tracker.categories_for_storage();
        categories.extend(self.archived_categories.iter().cloned());
        categories
            .into_iter()
            .map(|mut category| {
                category.color = self.resolved_category_color(category.id, category.color);
                category
            })
            .collect()
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
            .map(|anchor| self.resolved_category_color(category_id, anchor))
            .unwrap_or_else(|| self.theme_foreground())
    }

    pub(super) fn current_report_window(&self) -> ReportWindow {
        self.report_custom_window.clone().unwrap_or_else(|| {
            report_period_window_with_offset(self.report_period, self.report_period_offset)
        })
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

    pub(super) fn report_visible_rows(&self) -> BalanceReportSummary {
        let mut summary = self.report_rows();
        summary.entries.retain(report_entry_is_visible);
        summary
    }

    pub(super) fn begin_report_layer_delete_confirmation(
        &mut self,
        summary: &BalanceReportSummary,
    ) -> bool {
        let Some(index) = self.report_selected_summary_index(summary) else {
            return false;
        };
        let Some(entry) = summary.entries.get(index) else {
            return false;
        };
        if entry.category_id == DRIFT_CATEGORY_ID {
            return false;
        }
        self.clear_report_range_boundary();
        self.report_layer_delete_confirmation = Some(entry.category_id);
        self.system_dialog_selected_index = 1;
        self.render_needed = true;
        true
    }

    pub(super) fn cancel_report_layer_delete_confirmation(&mut self) {
        if self.report_layer_delete_confirmation.take().is_some() {
            self.render_needed = true;
        }
    }

    pub(super) fn confirm_report_layer_delete(&mut self) -> bool {
        let Some(category_id) = self.report_layer_delete_confirmation else {
            return false;
        };
        let fallback_index = self.report_selected_index;

        if self.time_tracker.active_category_id() == category_id {
            self.apply_runtime_mutation(super::RuntimeMutation::SwitchLayer {
                category_id: DRIFT_CATEGORY_ID,
                description: String::new(),
            });
            if self.has_persistence_recovery()
                || self.time_tracker.active_category_id() == category_id
            {
                return false;
            }
        }

        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CategoryDelete,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return false;
        };
        let Some(()) = self.record_storage_result_for(
            PersistenceOperation::CategoryDelete,
            RecoveryAction::ReloadAuthority,
            crate::sqlite::delete_tui_category(&database_path, category_id),
        ) else {
            return false;
        };

        let Some(state) = self.record_storage_result_for(
            PersistenceOperation::StateReload,
            RecoveryAction::ReloadAuthority,
            crate::sqlite::load_tui_state(&database_path),
        ) else {
            return false;
        };
        self.time_tracker.apply_loaded_state(
            state.loaded_categories.categories,
            state.loaded_categories.next_category_id,
            state.loaded_sessions.sessions,
            state.loaded_sessions.next_session_id,
        );
        self.archived_categories = state.archived_categories;
        self.category_tags = state.category_tags;

        // Permanent deletion reclassifies every retained representation of the
        // deleted Layer as Idle. Keep staged day-end photos aligned with the
        // same identity rewrite before they can be persisted later.
        for pending in &mut self.pending_day_end_snapshots {
            if crate::sand::recolor_state_category_mass(
                &mut pending.snapshot.state,
                category_id,
                DRIFT_CATEGORY_ID,
                usize::MAX,
            ) > 0
            {
                let day = pending
                    .snapshot
                    .operational_day
                    .clone()
                    .unwrap_or_else(|| pending.operational_day.format("%Y-%m-%d").to_string());
                pending.snapshot = crate::sand::SedimentSnapshot::day_end_checkpoint(
                    day,
                    pending.snapshot.state.clone(),
                );
            }
        }
        self.restore_sand_state();
        if self.has_persistence_recovery() {
            return false;
        }

        self.report_layer_delete_confirmation = None;
        self.report_tag_filter.clear();
        self.report_filter_tag_index = None;
        self.ledger_entry_edit = None;
        self.clear_report_snapshot_cache();
        let summary = self.report_visible_rows();
        if summary.entries.is_empty() {
            self.report_selected_index = 0;
            self.report_selected_category_id = None;
        } else {
            self.select_report_summary_index(
                &summary,
                fallback_index.min(summary.entries.len().saturating_sub(1)),
            );
        }
        self.render_needed = true;
        true
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

    fn ledger_identity_for_row(row: &CategoryLogEntry) -> Option<LedgerSelectionIdentity> {
        if let Some(session_id) = row.session_id {
            Some(LedgerSelectionIdentity::Session(session_id))
        } else {
            row.active_stable_id
                .as_ref()
                .map(|stable_id| LedgerSelectionIdentity::Active(stable_id.clone()))
        }
    }

    pub(super) fn remember_report_log_selection(&mut self) {
        let logs = self.report_current_logs();
        let can_add = self
            .report_logs_category_id
            .is_some_and(|category_id| self.report_layer_can_add(category_id));
        self.report_log_selected_identity = if self.report_log_selected_index < logs.len() {
            logs.get(self.report_log_selected_index)
                .and_then(Self::ledger_identity_for_row)
        } else if can_add && self.report_log_selected_index == logs.len() {
            Some(LedgerSelectionIdentity::Add)
        } else {
            None
        };
    }

    pub(super) fn sync_report_log_selection_to_identity(&mut self) {
        let logs = self.report_current_logs();
        let can_add = self
            .report_logs_category_id
            .is_some_and(|category_id| self.report_layer_can_add(category_id));
        if let Some(identity) = self.report_log_selected_identity.clone() {
            let found = match identity {
                LedgerSelectionIdentity::Session(session_id) => logs
                    .iter()
                    .position(|row| row.session_id == Some(session_id)),
                LedgerSelectionIdentity::Active(stable_id) => logs.iter().position(|row| {
                    row.active_stable_id.as_deref() == Some(stable_id.as_str())
                }),
                LedgerSelectionIdentity::Add if can_add => Some(logs.len()),
                LedgerSelectionIdentity::Add => None,
            };
            if let Some(index) = found {
                self.report_log_selected_index = index;
                return;
            }
        }

        let row_count = logs.len().saturating_add(usize::from(can_add));
        self.clamp_report_log_selection(row_count);
        self.remember_report_log_selection();
    }

    pub(super) fn report_tag_facets_for_log(&self, row: &CategoryLogEntry) -> Vec<ReportTagFacet> {
        report_tag_facets(&row.description)
    }

    pub(super) fn report_tag_filter_active(&self) -> bool {
        !self.report_tag_filter.is_empty()
    }

    pub(super) fn report_filter_focus_active(&self) -> bool {
        self.report_filter_tag_index.is_some()
    }

    pub(super) fn report_log_matches_filter(&self, row: &CategoryLogEntry) -> bool {
        report_log_matches_tag_filter(row, &self.report_tag_filter)
    }

    pub(super) fn report_filtered_layer_balance(&self, logs: &[CategoryLogEntry]) -> isize {
        filtered_layer_balance(logs, &self.report_tag_filter)
    }

    pub(super) fn report_filter_label(&self) -> Option<String> {
        if self.report_tag_filter.is_empty() {
            return None;
        }
        Some(
            self.report_tag_filter
                .iter()
                .map(|facet| match facet {
                    ReportTagFacet::Tag(tag) => tag.clone(),
                    ReportTagFacet::Untagged => "—".to_string(),
                })
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    pub(super) fn report_filter_focus_for_row(&self, row_index: usize) -> Option<usize> {
        if self.report_log_selected_index == row_index {
            self.report_filter_tag_index
        } else {
            None
        }
    }

    pub(super) fn toggle_selected_report_filter(&mut self) -> bool {
        let logs = self.report_current_logs();
        let Some(row) = logs.get(self.report_log_selected_index) else {
            return false;
        };
        let facets = self.report_tag_facets_for_log(row);
        if facets.is_empty() {
            return false;
        }
        self.clear_report_range_boundary();

        if facets.len() > 1 && self.report_filter_tag_index.is_none() {
            self.report_filter_tag_index = Some(0);
            self.render_needed = true;
            return true;
        }

        let focus = self
            .report_filter_tag_index
            .unwrap_or(0)
            .min(facets.len().saturating_sub(1));
        let facet = facets[focus].clone();
        if let Some(index) = self
            .report_tag_filter
            .iter()
            .position(|selected| report_tag_facet_matches(selected, &facet))
        {
            self.report_tag_filter.remove(index);
        } else {
            self.report_tag_filter.push(facet);
        }
        self.report_filter_tag_index = None;
        self.render_needed = true;
        true
    }

    pub(super) fn move_report_filter_focus(&mut self, delta: isize) -> bool {
        let Some(current) = self.report_filter_tag_index else {
            return false;
        };
        let logs = self.report_current_logs();
        let Some(row) = logs.get(self.report_log_selected_index) else {
            self.report_filter_tag_index = None;
            return false;
        };
        let facets = self.report_tag_facets_for_log(row);
        if facets.len() <= 1 {
            self.report_filter_tag_index = None;
            return false;
        }
        let len = facets.len();
        let current = current.min(len - 1);
        let next = if delta < 0 {
            (current + len - 1) % len
        } else {
            (current + 1) % len
        };
        self.report_filter_tag_index = Some(next);
        self.render_needed = true;
        true
    }

    pub(super) fn leave_report_filter_focus(&mut self) -> bool {
        if self.report_filter_tag_index.take().is_some() {
            self.render_needed = true;
            true
        } else {
            false
        }
    }

    pub(super) fn clear_report_tag_filter(&mut self) -> bool {
        let changed = !self.report_tag_filter.is_empty() || self.report_filter_tag_index.is_some();
        self.report_tag_filter.clear();
        self.report_filter_tag_index = None;
        if changed {
            self.render_needed = true;
        }
        changed
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
            active_stable_id: self.session.active_session_stable_id.clone()?,
            category_id,
            description,
            elapsed_seconds,
            started_at_utc,
            ended_at_utc,
            operational_day_policy: OperationalDayPolicy::from_config(day_boundary_config()),
        })
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

    fn install_historical_correction_receipt(
        &mut self,
        receipt: crate::sqlite::TuiHistoricalCorrectionReceipt,
    ) -> Result<(), String> {
        if let Some(state) = receipt.resulting_sand_state.as_ref() {
            let valid_category_ids = self
                .report_categories()
                .into_iter()
                .map(|category| category.id)
                .collect();
            self.sand_engine
                .restore_state(state, &valid_category_ids)
                .map_err(|error| {
                    format!("history committed but sediment refresh failed: {error}")
                })?;
        }
        let active_start_changed = self.session.active_session_started_at_utc
            != Some(receipt.resulting_active_started_at_utc);
        self.session.active_session_stable_id = Some(receipt.resulting_active_stable_id.clone());
        if active_start_changed {
            self.begin_active_session_at(receipt.resulting_active_started_at_utc, true)?;
        }
        if !self.reload_sqlite_sessions() {
            return Err("history committed but session reload failed".to_string());
        }
        self.clear_report_snapshot_cache();
        Ok(())
    }

    pub(super) fn set_report_period(&mut self, period: ReportPeriod) {
        self.report_filter_tag_index = None;
        self.report_period = period;
        self.report_period_offset = 0;
        self.report_custom_window = None;
        self.report_range_boundary = None;
        self.report_range_boundary_original = None;
        self.report_range_edit = None;
        self.ledger_entry_edit = None;
        self.clear_report_snapshot_cache();
        self.sync_report_selection_for_interval();
    }

    pub(super) fn begin_report_range_edit(&mut self) {
        self.report_filter_tag_index = None;
        let window = self.current_report_window();
        self.report_range_boundary = None;
        self.report_range_boundary_original = None;
        self.ledger_entry_edit = None;
        self.report_range_edit = Some(report_range_edit_state(&window));
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

    pub(super) fn select_report_range_boundary(&mut self, boundary: ReportRangeBoundary) {
        self.report_filter_tag_index = None;
        if self.report_range_boundary.is_none() {
            self.report_range_boundary_original = Some(super::ReportRangeBoundarySnapshot {
                period: self.report_period,
                period_offset: self.report_period_offset,
                custom_window: self.report_custom_window.clone(),
                selected_category_id: self.report_selected_category_id,
                selected_index: self.report_selected_index,
            });
        }
        self.report_range_boundary = Some(boundary);
        self.render_needed = true;
    }

    pub(super) fn clear_report_range_boundary(&mut self) {
        let changed = self.report_range_boundary.take().is_some()
            || self.report_range_boundary_original.take().is_some();
        if changed {
            self.render_needed = true;
        }
    }

    pub(super) fn cancel_report_range_boundary(&mut self) {
        let snapshot = self.report_range_boundary_original.take();
        let changed = self.report_range_boundary.take().is_some() || snapshot.is_some();
        if let Some(snapshot) = snapshot {
            self.report_period = snapshot.period;
            self.report_period_offset = snapshot.period_offset;
            self.report_custom_window = snapshot.custom_window;
            self.report_selected_category_id = snapshot.selected_category_id;
            self.report_selected_index = snapshot.selected_index;
            self.clear_report_snapshot_cache();
            self.sync_report_selection_for_interval();
        }
        if changed {
            self.render_needed = true;
        }
    }

    pub(super) fn move_report_range_boundary(&mut self, direction: i8) -> bool {
        let Some(boundary) = self.report_range_boundary else {
            return false;
        };
        let original = self.current_report_window();
        let today = operational_day_key_now();
        let Some(shifted) = shifted_report_boundary(&original, boundary, direction, today) else {
            return false;
        };
        self.install_shifted_report_boundary(original, shifted)
    }

    pub(super) fn move_report_range_boundary_month(&mut self, direction: i8) -> bool {
        let Some(boundary) = self.report_range_boundary else {
            return false;
        };
        let original = self.current_report_window();
        let today = operational_day_key_now();
        let Some(shifted) = shifted_report_boundary_month(&original, boundary, direction, today)
        else {
            return false;
        };
        self.install_shifted_report_boundary(original, shifted)
    }

    fn install_shifted_report_boundary(
        &mut self,
        original: ReportWindow,
        shifted: ReportWindow,
    ) -> bool {
        if shifted == original {
            return false;
        }

        self.report_custom_window = Some(shifted);
        self.report_period_offset = 0;
        self.clear_report_snapshot_cache();
        self.sync_report_selection_for_interval();
        self.render_needed = true;
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
        let categories = self.report_categories();
        let source_key = snapshot.source_revision.clone();
        let should_rebuild_physics_preview = self
            .report_snapshot_physics_preview
            .as_ref()
            .map(|preview| preview.source_key != source_key)
            .unwrap_or(true);

        if should_rebuild_physics_preview {
            self.report_snapshot_physics_preview =
                build_historical_preview(&snapshot, width, height, &categories).ok();
        }

        if let Some(preview) = self.report_snapshot_physics_preview.as_mut() {
            if preview.engine.dimensions() != (width, height) {
                preview.engine.resize_physics_preview(width, height);
            }
            return Some(preview.engine.render(&categories));
        }

        let cache_key = snapshot.render_cache_key(width, height);
        let should_rebuild_fallback = self
            .report_snapshot_preview_key
            .as_deref()
            .map(|key| key != cache_key.as_str())
            .unwrap_or(true)
            || self.report_snapshot_preview_lines.is_none();
        if should_rebuild_fallback {
            self.report_snapshot_preview_lines =
                Some(snapshot.render_immutable(width, height, &categories));
            self.report_snapshot_preview_key = Some(cache_key);
        }
        self.report_snapshot_preview_lines.clone()
    }

    pub(super) fn clear_report_snapshot_cache(&mut self) {
        self.report_snapshot_end_day = None;
        self.report_snapshot_artifact = None;
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_lines = None;
        self.report_snapshot_physics_preview = None;
    }

    pub(super) fn report_interval_end_day(&self) -> NaiveDate {
        self.current_report_window().end
    }

    pub(super) fn should_use_report_snapshot(&self) -> bool {
        self.report_interval_end_day() < operational_day_key_now()
    }

    fn fallback_identity_after_removing(
        logs: &[CategoryLogEntry],
        removed_index: usize,
    ) -> Option<LedgerSelectionIdentity> {
        logs.get(removed_index.saturating_add(1))
            .and_then(Self::ledger_identity_for_row)
            .or_else(|| {
                removed_index
                    .checked_sub(1)
                    .and_then(|index| logs.get(index))
                    .and_then(Self::ledger_identity_for_row)
            })
    }

    pub(super) fn begin_report_entry_delete_confirmation(&mut self) -> bool {
        let logs = self.report_current_logs();
        let Some(row) = logs.get(self.report_log_selected_index) else {
            return false;
        };
        let Some(identity) = Self::ledger_identity_for_row(row) else {
            return false;
        };
        self.report_entry_delete_confirmation = Some(identity);
        self.system_dialog_selected_index = 1;
        self.render_needed = true;
        true
    }

    pub(super) fn cancel_report_entry_delete_confirmation(&mut self) {
        if self.report_entry_delete_confirmation.take().is_some() {
            self.render_needed = true;
        }
    }

    fn delete_report_session_by_id(&mut self, session_id: usize) -> bool {
        let Some(category_id) = self.report_logs_category_id else {
            return false;
        };
        let Some(session) = self
            .time_tracker
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .cloned()
        else {
            return false;
        };
        let removed_seconds = session.elapsed_seconds;
        let affected_days = session_slices(&session)
            .into_iter()
            .map(|slice| slice.operational_day)
            .collect::<BTreeSet<_>>();

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
        true
    }

    fn delete_active_report_generation(&mut self, expected_stable_id: &str) -> bool {
        if self.session.active_session_stable_id.as_deref() != Some(expected_stable_id) {
            return false;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return false;
        };
        let now = Utc::now();
        if let Err(error) = self.settle_transition_boundary(now) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::SessionDelete,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }
        let previous_category = self.time_tracker.active_category_id();
        let removed_seconds = usize::try_from(
            self.time_tracker.current_elapsed().unwrap_or_default().as_secs(),
        )
        .unwrap_or(usize::MAX);
        let affected_days = self
            .live_preview_session()
            .map(|session| {
                session_slices(&session)
                    .into_iter()
                    .map(|slice| slice.operational_day)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let identity = transition_identity(
            "delete-active",
            expected_stable_id,
            now,
            "idle",
        );
        let next_stable_id = identity.tui_active_stable_id();
        let Some(receipt) = self.record_storage_result_for(
            PersistenceOperation::SessionDelete,
            RecoveryAction::ReloadAuthority,
            crate::sqlite::reset_active_session_to(
                &database_path,
                expected_stable_id,
                &identity.operation_id,
                &next_stable_id,
                DRIFT_CATEGORY_ID,
                "",
                now,
                now,
            ),
        ) else {
            return false;
        };

        if !self.time_tracker.set_active_category_by_id(DRIFT_CATEGORY_ID) {
            return false;
        }
        self.time_tracker.set_active_description(String::new());
        self.time_tracker.start_session();
        self.session.active_session_stable_id = receipt.resulting_active_stable_id;
        self.session.active_session_started_at_utc = Some(now);
        if previous_category != DRIFT_CATEGORY_ID && removed_seconds > 0 {
            self.sand_engine
                .remove_category_grains(previous_category, removed_seconds);
        }
        for day in affected_days {
            self.reconcile_daily_contribution(day);
        }
        self.refresh_active_runtime_checkpoint();
        self.clear_report_snapshot_cache();
        !self.has_persistence_recovery()
    }

    pub(super) fn confirm_report_entry_delete(&mut self) -> bool {
        let Some(identity) = self.report_entry_delete_confirmation.clone() else {
            return false;
        };
        let logs = self.report_current_logs();
        let removed_index = logs.iter().position(|row| match &identity {
            LedgerSelectionIdentity::Session(session_id) => row.session_id == Some(*session_id),
            LedgerSelectionIdentity::Active(stable_id) => {
                row.active_stable_id.as_deref() == Some(stable_id.as_str())
            }
            LedgerSelectionIdentity::Add => false,
        });
        let fallback = removed_index
            .and_then(|index| Self::fallback_identity_after_removing(&logs, index));
        let deleted = match &identity {
            LedgerSelectionIdentity::Session(session_id) => {
                self.delete_report_session_by_id(*session_id)
            }
            LedgerSelectionIdentity::Active(stable_id) => {
                self.delete_active_report_generation(stable_id)
            }
            LedgerSelectionIdentity::Add => false,
        };
        if !deleted {
            return false;
        }
        self.report_entry_delete_confirmation = None;
        self.report_log_selected_identity = fallback.or_else(|| {
            self.report_logs_category_id
                .filter(|category_id| self.report_layer_can_add(*category_id))
                .map(|_| LedgerSelectionIdentity::Add)
        });
        self.sync_report_log_selection_to_identity();
        self.render_needed = true;
        true
    }

    pub(super) fn report_layer_display_name(&self, category_id: CategoryId) -> String {
        self.time_tracker
            .category_by_id(category_id)
            .map(|category| self.display_layer_name(&category.name))
            .or_else(|| {
                self.archived_categories
                    .iter()
                    .find(|category| category.id == category_id)
                    .map(|category| self.display_layer_name(&category.name))
            })
            .unwrap_or_else(|| format!("Layer {}", category_id.0))
    }

    pub(super) fn report_layer_can_add(&self, category_id: CategoryId) -> bool {
        self.time_tracker.category_by_id(category_id).is_some()
    }

    pub(super) fn report_ledger_row_count(&self) -> usize {
        let logs = self.report_current_logs();
        let add = self
            .report_logs_category_id
            .is_some_and(|category_id| self.report_layer_can_add(category_id));
        logs.len().saturating_add(usize::from(add))
    }

    pub(super) fn begin_ledger_entry_edit(&mut self) -> bool {
        let Some(category_id) = self.report_logs_category_id else {
            return false;
        };
        let logs = self.report_current_logs();
        let add_index = logs.len();
        if self.report_log_selected_index == add_index && self.report_layer_can_add(category_id) {
            return self.begin_ledger_add_edit(category_id);
        }
        let Some(row) = logs.get(
            self.report_log_selected_index
                .min(logs.len().saturating_sub(1)),
        ) else {
            return false;
        };
        if row.active_stable_id.as_deref() == self.session.active_session_stable_id.as_deref()
            && row.active_stable_id.is_some()
        {
            let Some(started_at_utc) = self.session.active_session_started_at_utc else {
                return false;
            };
            let policy = OperationalDayPolicy::from_config(day_boundary_config());
            let Ok(start) = temporal::civil_from_policy(started_at_utc, policy) else {
                return false;
            };
            let Ok(end) = temporal::civil_from_policy(Utc::now(), policy) else {
                return false;
            };
            self.report_filter_tag_index = None;
            self.ledger_entry_edit = Some(super::LedgerEntryEditState::active(
                category_id,
                self.time_tracker.active_description().to_string(),
                start.format("%Y-%m-%d").to_string(),
                start.format("%H:%M:%S").to_string(),
                end.format("%Y-%m-%d").to_string(),
                end.format("%H:%M:%S").to_string(),
            ));
            self.report_range_boundary = None;
            self.report_range_boundary_original = None;
            self.render_needed = true;
            return true;
        }
        let Some(session_id) = row.session_id else {
            return false;
        };
        let Some(session) = self
            .time_tracker
            .sessions
            .iter()
            .find(|session| session.id == session_id)
        else {
            return false;
        };
        let (Some(started_at_utc), Some(ended_at_utc), Some(policy)) = (
            session.started_at_utc,
            session.ended_at_utc,
            session.operational_day_policy,
        ) else {
            return false;
        };
        let Ok(start) = temporal::civil_from_policy(started_at_utc, policy) else {
            return false;
        };
        let Ok(end) = temporal::civil_from_policy(ended_at_utc, policy) else {
            return false;
        };
        self.report_filter_tag_index = None;
        self.ledger_entry_edit = Some(super::LedgerEntryEditState::existing(
            session_id,
            category_id,
            session.description.clone(),
            start.format("%Y-%m-%d").to_string(),
            start.format("%H:%M:%S").to_string(),
            end.format("%Y-%m-%d").to_string(),
            end.format("%H:%M:%S").to_string(),
        ));
        self.report_range_boundary = None;
        self.report_range_boundary_original = None;
        self.render_needed = true;
        true
    }

    pub(super) fn begin_selected_layer_ledger_add(&mut self) -> bool {
        if let Some(category_id) = self.report_logs_category_id {
            return self.begin_ledger_add_edit(category_id);
        }
        let summary = self.report_visible_rows();
        let Some(index) = self.report_selected_summary_index(&summary) else {
            return false;
        };
        let Some(entry) = summary.entries.get(index) else {
            return false;
        };
        let category_id = entry.category_id;
        if !self.report_layer_can_add(category_id) {
            return false;
        }
        self.clear_report_tag_filter();
        self.report_logs_category_id = Some(category_id);
        self.report_log_selected_index = 0;
        self.begin_ledger_add_edit(category_id)
    }

    pub(super) fn begin_ledger_add_edit(&mut self, category_id: CategoryId) -> bool {
        if !self.report_layer_can_add(category_id) {
            return false;
        }
        let active_preview = match self.historical_correction_active_preview() {
            Ok(preview) => preview,
            Err(_) => return false,
        };
        let (from, to) = match self.default_ledger_add_bounds(&active_preview) {
            Ok(bounds) => bounds,
            Err(_) => return false,
        };
        let Ok(from_civil) =
            temporal::civil_from_policy(from, active_preview.operational_day_policy)
        else {
            return false;
        };
        let Ok(to_civil) = temporal::civil_from_policy(to, active_preview.operational_day_policy)
        else {
            return false;
        };
        self.report_filter_tag_index = None;
        self.ledger_entry_edit = Some(super::LedgerEntryEditState::add(
            category_id,
            from_civil.format("%Y-%m-%d").to_string(),
            from_civil.format("%H:%M:%S").to_string(),
            to_civil.format("%Y-%m-%d").to_string(),
            to_civil.format("%H:%M:%S").to_string(),
        ));
        self.report_log_selected_index = self.report_current_logs().len();
        self.report_log_selected_identity = Some(LedgerSelectionIdentity::Add);
        self.report_range_boundary = None;
        self.report_range_boundary_original = None;
        self.render_needed = true;
        true
    }

    fn default_ledger_add_bounds(
        &self,
        active_preview: &crate::sqlite::TuiHistoricalActivePreview,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
        let window = self.current_report_window();
        let policy = active_preview.operational_day_policy;
        if window.end == operational_day_key_now() {
            let to = active_preview.ended_at_utc;
            let from = to
                .checked_sub_signed(ChronoDuration::minutes(15))
                .ok_or_else(|| "default ledger interval underflowed".to_string())?;
            if temporal::operational_day_from_policy(from, policy)? >= window.start {
                return Ok((from, to));
            }
        }

        let offset = FixedOffset::east_opt(policy.utc_offset_seconds)
            .ok_or_else(|| "ledger interval has an invalid UTC offset".to_string())?;
        let seconds = u32::from(policy.start_minutes) * 60;
        let time = NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0)
            .ok_or_else(|| "ledger interval has an invalid day boundary".to_string())?;
        let local = window.end.and_time(time);
        let from = offset
            .from_local_datetime(&local)
            .single()
            .ok_or_else(|| "ledger interval boundary is not unique".to_string())?
            .with_timezone(&Utc);
        let to = from
            .checked_add_signed(ChronoDuration::minutes(15))
            .ok_or_else(|| "default ledger interval overflowed".to_string())?
            .min(active_preview.ended_at_utc);
        if to <= from {
            return Err(
                "selected period has no completed time available for a new entry".to_string(),
            );
        }
        Ok((from, to))
    }

    pub(super) fn cycle_ledger_tag(&mut self, direction: isize) -> bool {
        let Some(edit) = self.ledger_entry_edit.as_ref() else {
            return false;
        };
        if edit.active_field != super::LedgerEntryField::Description {
            return false;
        }
        let category_id = edit.category_id;
        let description = edit.description.clone();
        let retained_prefix = edit.tag_cycle_prefix.clone();
        let known_tags = self.known_tags_for_category(category_id);
        let Some(cycle) = tagging::cycle_tag(
            &description,
            &known_tags,
            retained_prefix.as_deref(),
            direction,
        ) else {
            return false;
        };
        let Some(edit) = self.ledger_entry_edit.as_mut() else {
            return false;
        };
        edit.description = cycle.value;
        // Plain Right accepts/cycles the current ghost-completion token. Keep the
        // caret at the end of that accepted token instead of stranded at the
        // length of the typed prefix.
        edit.caret = edit.description.chars().count();
        edit.tag_cycle_prefix = Some(cycle.prefix);
        edit.select_all = false;
        edit.error = None;
        edit.confirmation = None;
        self.render_needed = true;
        true
    }

    pub(super) fn cancel_ledger_entry_edit(&mut self) {
        self.ledger_entry_edit = None;
        self.render_needed = true;
    }

    pub(super) fn dismiss_ledger_entry_confirmation(&mut self) {
        if let Some(edit) = self.ledger_entry_edit.as_mut() {
            edit.confirmation = None;
            edit.error = None;
            self.render_needed = true;
        }
    }

    fn ledger_edit_bounds(
        edit: &super::LedgerEntryEditState,
        policy: OperationalDayPolicy,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
        let (start, mut end) = edit
            .preview_naive_bounds()
            .ok_or_else(|| "date/time could not be normalized".to_string())?;
        if edit.is_active() {
            end = temporal::civil_from_policy(Utc::now(), policy)
                .map_err(|error| format!("active end cannot be represented: {error}"))?
                .naive_local();
        }
        let offset = FixedOffset::east_opt(policy.utc_offset_seconds)
            .ok_or_else(|| "ledger entry has an invalid UTC offset".to_string())?;
        let from = offset
            .from_local_datetime(&start)
            .single()
            .ok_or_else(|| "ledger entry start is not unique".to_string())?
            .with_timezone(&Utc);
        let to = offset
            .from_local_datetime(&end)
            .single()
            .ok_or_else(|| "ledger entry end is not unique".to_string())?
            .with_timezone(&Utc);
        if from >= to {
            return Err("start must be before end".to_string());
        }
        Ok((from, to))
    }

    pub(super) fn ledger_edit_policy(
        &self,
        edit: &super::LedgerEntryEditState,
    ) -> OperationalDayPolicy {
        if let Some(session_id) = edit.session_id()
            && let Some(policy) = self
                .time_tracker
                .sessions
                .iter()
                .find(|session| session.id == session_id)
                .and_then(|session| session.operational_day_policy)
        {
            return policy;
        }
        OperationalDayPolicy::from_config(day_boundary_config())
    }

    pub(super) fn ledger_edit_report_contribution(
        &self,
        edit: &super::LedgerEntryEditState,
    ) -> Option<(usize, isize)> {
        let policy = self.ledger_edit_policy(edit);
        let (from, to) = Self::ledger_edit_bounds(edit, policy).ok()?;
        let elapsed = usize::try_from((to - from).num_seconds()).ok()?;
        if elapsed == 0 {
            return Some((0, 0));
        }
        let window = self.current_report_window();
        let visible = temporal::allocate_operational_day_slices(from, to, elapsed, policy)
            .ok()?
            .into_iter()
            .filter(|slice| {
                slice.operational_day >= window.start && slice.operational_day <= window.end
            })
            .map(|slice| slice.elapsed_seconds)
            .sum::<usize>();
        let effect = self
            .report_categories()
            .into_iter()
            .find(|category| category.id == edit.category_id)
            .map(|category| category.balance_effect)
            .unwrap_or(0);
        Some((visible, visible as isize * effect as isize))
    }

    fn preserve_hidden_subseconds(
        requested: DateTime<Utc>,
        original: DateTime<Utc>,
        policy: OperationalDayPolicy,
    ) -> DateTime<Utc> {
        let (Ok(requested_civil), Ok(original_civil)) = (
            temporal::civil_from_policy(requested, policy),
            temporal::civil_from_policy(original, policy),
        ) else {
            return requested;
        };
        if requested_civil.format("%Y-%m-%d %H:%M:%S").to_string()
            == original_civil.format("%Y-%m-%d %H:%M:%S").to_string()
        {
            original
        } else {
            requested
        }
    }

    fn commit_active_ledger_entry_edit(
        &mut self,
        edit: &super::LedgerEntryEditState,
        from: DateTime<Utc>,
        policy: OperationalDayPolicy,
    ) -> bool {
        let now = Utc::now();
        if from >= now {
            if let Some(current) = self.ledger_entry_edit.as_mut() {
                current.error = Some("active start must be before now".to_string());
            }
            self.render_needed = true;
            return false;
        }
        // Re-basing a live generation across completed history would require the
        // collateral-history planner. Fail closed rather than silently overlap it.
        if self.time_tracker.sessions.iter().any(|session| {
            session.ended_at_utc.is_some_and(|ended| ended > from)
                && session.started_at_utc.is_some_and(|started| started < now)
        }) {
            if let Some(current) = self.ledger_entry_edit.as_mut() {
                current.error = Some("active start overlaps completed history".to_string());
            }
            self.render_needed = true;
            return false;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return false;
        };
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            return false;
        };
        let old_start = self.session.active_session_started_at_utc.unwrap_or(from);
        let old_elapsed = usize::try_from(
            self.time_tracker.current_elapsed().unwrap_or_default().as_secs(),
        )
        .unwrap_or(usize::MAX);
        let old_days = self
            .live_preview_session()
            .map(|session| {
                session_slices(&session)
                    .into_iter()
                    .map(|slice| slice.operational_day)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let identity = transition_identity(
            "rebase-active",
            &expected_stable_id,
            now,
            &edit.category_id.0.to_string(),
        );
        let next_stable_id = identity.tui_active_stable_id();
        let Some(receipt) = self.record_storage_result_for(
            PersistenceOperation::ActiveReset,
            RecoveryAction::ReloadAuthority,
            crate::sqlite::reset_active_session_to(
                &database_path,
                &expected_stable_id,
                &identity.operation_id,
                &next_stable_id,
                edit.category_id,
                &edit.description,
                from,
                now,
            ),
        ) else {
            return false;
        };

        self.time_tracker.set_active_description(edit.description.clone());
        if let Err(error) = self.begin_active_session_at(from, true) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveReset,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }
        self.session.active_session_stable_id = receipt.resulting_active_stable_id.clone();
        self.session.active_session_started_at_utc = Some(from);
        let new_elapsed = usize::try_from((now - from).num_seconds().max(0)).unwrap_or(usize::MAX);
        if old_elapsed > new_elapsed {
            self.sand_engine
                .remove_category_grains(edit.category_id, old_elapsed - new_elapsed);
        }
        let mut affected_days = old_days;
        if let Some(new_preview) = self.live_preview_session() {
            affected_days.extend(
                session_slices(&new_preview)
                    .into_iter()
                    .map(|slice| slice.operational_day),
            );
        }
        // A backward start extends ledger truth into a true gap; consistent with
        // historical correction, that added time does not synthesize current grains.
        let _ = old_start;
        let _ = policy;
        for day in affected_days {
            self.reconcile_daily_contribution(day);
        }
        self.remember_description_tags_for_category(edit.category_id, &edit.description);
        self.report_log_selected_identity = receipt
            .resulting_active_stable_id
            .map(LedgerSelectionIdentity::Active);
        self.ledger_entry_edit = None;
        self.clear_report_snapshot_cache();
        self.refresh_active_runtime_checkpoint();
        self.sync_report_selection_for_interval();
        !self.has_persistence_recovery()
    }

    pub(super) fn commit_ledger_entry_edit(&mut self) -> bool {
        let Some(mut edit) = self.ledger_entry_edit.clone() else {
            return false;
        };
        edit.description =
            self.canonicalize_description_for_category(edit.category_id, &edit.description);
        if let Err(error) = self.settle_transition_boundary(Utc::now()) {
            if let Some(current) = self.ledger_entry_edit.as_mut() {
                current.error = Some(error);
            }
            self.render_needed = true;
            return false;
        }
        let active_preview = match self.historical_correction_active_preview() {
            Ok(preview) => preview,
            Err(error) => {
                if let Some(current) = self.ledger_entry_edit.as_mut() {
                    current.error = Some(error);
                }
                self.render_needed = true;
                return false;
            }
        };
        let policy = match edit.kind {
            super::LedgerEntryEditKind::Existing { session_id } => self
                .time_tracker
                .sessions
                .iter()
                .find(|session| session.id == session_id)
                .and_then(|session| session.operational_day_policy)
                .unwrap_or(active_preview.operational_day_policy),
            super::LedgerEntryEditKind::Active | super::LedgerEntryEditKind::Add => {
                active_preview.operational_day_policy
            }
        };
        let (mut from, mut to) = match Self::ledger_edit_bounds(&edit, policy) {
            Ok(bounds) => bounds,
            Err(error) => {
                if let Some(current) = self.ledger_entry_edit.as_mut() {
                    current.error = Some(error);
                    current.confirmation = None;
                }
                self.render_needed = true;
                return false;
            }
        };
        if let super::LedgerEntryEditKind::Existing { session_id } = edit.kind
            && let Some(session) = self
                .time_tracker
                .sessions
                .iter()
                .find(|session| session.id == session_id)
        {
            if let Some(original) = session.started_at_utc {
                from = Self::preserve_hidden_subseconds(from, original, policy);
            }
            if let Some(original) = session.ended_at_utc {
                to = Self::preserve_hidden_subseconds(to, original, policy);
            }
        }
        if to > active_preview.ended_at_utc {
            if let Some(current) = self.ledger_entry_edit.as_mut() {
                current.error = Some("end cannot be later than now".to_string());
                current.confirmation = None;
            }
            self.render_needed = true;
            return false;
        }
        if edit.is_active() {
            return self.commit_active_ledger_entry_edit(&edit, from, policy);
        }
        if edit.is_add() && self.time_tracker.category_by_id(edit.category_id).is_none() {
            if let Some(current) = self.ledger_entry_edit.as_mut() {
                current.error = Some("archived layer cannot receive new entries".to_string());
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
                if let Some(current) = self.ledger_entry_edit.as_mut() {
                    current.error = Some(error);
                }
                self.render_needed = true;
                return false;
            }
        };
        let checkpoint_json = match serde_json::to_string(&checkpoint) {
            Ok(value) => value,
            Err(error) => {
                if let Some(current) = self.ledger_entry_edit.as_mut() {
                    current.error = Some(error.to_string());
                }
                self.render_needed = true;
                return false;
            }
        };
        let confirmed_plan_token = edit
            .confirmation
            .as_ref()
            .map(|confirmation| confirmation.plan_token.clone());
        let committed_description = edit.description.clone();
        let result = crate::sqlite::apply_tui_historical_correction(
            &database_path,
            crate::sqlite::TuiHistoricalCorrectionRequest {
                source_session_id: edit.session_id(),
                target_category_id: edit.category_id,
                started_at_utc: from,
                ended_at_utc: to,
                description: edit.description,
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
            crate::sqlite::TuiHistoricalCorrectionOutcome::NeedsConfirmation {
                plan_token,
                changes,
            } => {
                if let Some(current) = self.ledger_entry_edit.as_mut() {
                    current.confirmation = Some(super::LedgerCorrectionConfirmation {
                        plan_token,
                        changes,
                    });
                    current.error = None;
                }
                self.system_dialog_selected_index = 1;
                self.render_needed = true;
                false
            }
            crate::sqlite::TuiHistoricalCorrectionOutcome::Applied(receipt) => {
                if let Err(error) = self.install_historical_correction_receipt(receipt) {
                    if let Some(current) = self.ledger_entry_edit.as_mut() {
                        current.error = Some(error);
                    }
                    self.render_needed = true;
                    return false;
                }
                self.remember_description_tags_for_category(
                    edit.category_id,
                    &committed_description,
                );
                self.ledger_entry_edit = None;
                self.sync_report_selection_for_interval();
                true
            }
        }
    }

    fn sync_report_selection_for_interval(&mut self) {
        if self.report_logs_category_id.is_some() {
            self.sync_report_log_selection_to_identity();
        } else {
            let summary = self.report_visible_rows();
            self.sync_report_selection_to_summary(&summary);
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
        self.report_snapshot_physics_preview = None;
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
        let (grid_width_dots, grid_height_dots) = self.sand_engine.canonical_dimensions();
        derived_preview_from_slices(&day_key, grid_width_dots, grid_height_dots, &slices)
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

    pub(super) fn clamp_report_log_selection(&mut self, row_count: usize) {
        if row_count == 0 {
            self.report_log_selected_index = 0;
        } else if self.report_log_selected_index >= row_count {
            self.report_log_selected_index = row_count - 1;
        }
    }
}

#[cfg(test)]
mod report_edit_state_tests {
    use chrono::{Duration as ChronoDuration, NaiveDate, TimeZone, Utc};
    use ratatui::style::Color;

    use super::{
        build_historical_preview, filtered_layer_balance, parse_report_range,
        report_entry_is_visible, report_log_matches_tag_filter, report_range_edit_state,
        shifted_custom_window_newer,
        shifted_custom_window_older, shifted_report_boundary, shifted_report_boundary_month,
    };
    use crate::{
        app::{ReportRangeBoundary, ReportTagFacet, ui_helpers},
        domain::{
            BalanceReportEntry, Category, CategoryId, CategoryLogEntry, OperationalDayPolicy,
            ReportWindow, DRIFT_CATEGORY_ID,
        },
        sand::{SandState, SandStateGrain, SedimentSnapshot},
    };

    fn preview_category(id: CategoryId) -> Category {
        Category {
            id,
            name: "preview".to_string(),
            color: Color::White,
            description: String::new(),
            balance_effect: 0,
        }
    }

    fn tagged_log(description: &str, balance_seconds: isize) -> CategoryLogEntry {
        CategoryLogEntry {
            session_id: Some(1),
            active_stable_id: None,
            date: "2026-09-28".to_string(),
            end_date: "2026-09-28".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            description: description.to_string(),
            elapsed_seconds: balance_seconds.unsigned_abs(),
            balance_effect: balance_seconds.signum() as i8,
            balance_seconds,
        }
    }

    #[test]
    fn layer_tag_filter_is_or_and_counts_each_matching_row_once() {
        let logs = vec![
            tagged_log("Renzo; Anibal", 3600),
            tagged_log("Personal", 1800),
        ];
        let renzo = vec![ReportTagFacet::Tag("Renzo".to_string())];
        let anibal = vec![ReportTagFacet::Tag("Anibal".to_string())];
        let both = vec![
            ReportTagFacet::Tag("Renzo".to_string()),
            ReportTagFacet::Tag("Anibal".to_string()),
        ];

        assert_eq!(filtered_layer_balance(&logs, &renzo), 3600);
        assert_eq!(filtered_layer_balance(&logs, &anibal), 3600);
        assert_eq!(filtered_layer_balance(&logs, &both), 3600);
        assert!(report_log_matches_tag_filter(&logs[0], &both));
        assert!(!report_log_matches_tag_filter(&logs[1], &both));
    }

    #[test]
    fn untagged_filter_matches_only_empty_tag_rows() {
        let untagged = tagged_log("", 1200);
        let tagged = tagged_log("Renzo", 1200);
        let filter = vec![ReportTagFacet::Untagged];

        assert!(report_log_matches_tag_filter(&untagged, &filter));
        assert!(!report_log_matches_tag_filter(&tagged, &filter));
    }

    #[test]
    fn balance_summary_visibility_omits_exact_zero_rows_only() {
        let ordinary_zero = BalanceReportEntry {
            category_id: CategoryId::new(1),
            category_name: "Work".to_string(),
            color: Color::White,
            elapsed_seconds: 3600,
            balance_effect: 1,
            balance_seconds: 0,
        };
        let ordinary_nonzero = BalanceReportEntry {
            balance_seconds: 1,
            ..ordinary_zero.clone()
        };
        let idle_zero = BalanceReportEntry {
            category_id: DRIFT_CATEGORY_ID,
            category_name: "Idle".to_string(),
            color: Color::White,
            elapsed_seconds: 0,
            balance_effect: 0,
            balance_seconds: 0,
        };
        let idle_nonzero = BalanceReportEntry {
            elapsed_seconds: 1,
            ..idle_zero.clone()
        };

        assert!(!report_entry_is_visible(&ordinary_zero));
        assert!(report_entry_is_visible(&ordinary_nonzero));
        assert!(!report_entry_is_visible(&idle_zero));
        assert!(report_entry_is_visible(&idle_nonzero));
    }

    #[test]
    fn custom_range_editor_uses_exclusive_to_boundary() {
        let window = parse_report_range("2026-08-01", "2026-08-28").unwrap();
        assert_eq!(window.label, "2026-08-01..2026-08-27");
        assert_eq!(
            ui_helpers::report_window_end_exclusive(&window),
            NaiveDate::from_ymd_opt(2026, 8, 28)
        );
        assert!(parse_report_range("08/01/2026", "2026-08-28").is_err());
        assert!(parse_report_range("2026-08-27", "2026-08-27").is_err());
        assert!(parse_report_range("2026-08-28", "2026-08-27").is_err());
    }

    #[test]
    fn range_editor_starts_from_visible_exclusive_boundaries() {
        let window = ReportWindow::new(
            NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
        )
        .unwrap();
        let edit = report_range_edit_state(&window);
        assert_eq!(edit.from, "2026-09-21");
        assert_eq!(edit.to, "2026-09-22");
        assert_eq!(edit.active_field, crate::app::ReportRangeField::From);
        assert!(edit.select_all);
        assert_eq!(edit.error, None);
    }

    #[test]
    fn report_exclusive_end_is_checked_at_date_extremes() {
        let window = ReportWindow::new(NaiveDate::MAX, NaiveDate::MAX).unwrap();
        assert_eq!(ui_helpers::report_window_end_exclusive(&window), None);
    }

    #[test]
    fn unchanged_visible_boundary_second_preserves_hidden_subseconds() {
        let policy = OperationalDayPolicy {
            utc_offset_seconds: -6 * 60 * 60,
            start_minutes: 0,
        };
        let original =
            Utc.with_ymd_and_hms(2026, 8, 2, 5, 45, 0).unwrap() + ChronoDuration::milliseconds(500);
        let requested = Utc.with_ymd_and_hms(2026, 8, 2, 5, 45, 0).unwrap();
        assert_eq!(
            crate::app::App::preserve_hidden_subseconds(requested, original, policy),
            original
        );
        assert_eq!(
            crate::app::App::preserve_hidden_subseconds(
                requested + ChronoDuration::seconds(1),
                original,
                policy,
            ),
            requested + ChronoDuration::seconds(1)
        );
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

    #[test]
    fn bracket_boundaries_move_one_operational_day_without_crossing() {
        let start = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 8, 14).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let window = ReportWindow::new(start, end).unwrap();

        let expanded_left =
            shifted_report_boundary(&window, ReportRangeBoundary::Start, -1, today).unwrap();
        assert_eq!(
            expanded_left.start,
            NaiveDate::from_ymd_opt(2026, 8, 9).unwrap()
        );
        assert_eq!(expanded_left.end, end);

        let contracted_left =
            shifted_report_boundary(&window, ReportRangeBoundary::Start, 1, today).unwrap();
        assert_eq!(
            contracted_left.start,
            NaiveDate::from_ymd_opt(2026, 8, 11).unwrap()
        );
        assert_eq!(contracted_left.end, end);

        let contracted_right =
            shifted_report_boundary(&window, ReportRangeBoundary::End, -1, today).unwrap();
        assert_eq!(contracted_right.start, start);
        assert_eq!(
            contracted_right.end,
            NaiveDate::from_ymd_opt(2026, 8, 13).unwrap()
        );

        let expanded_right =
            shifted_report_boundary(&window, ReportRangeBoundary::End, 1, today).unwrap();
        assert_eq!(expanded_right.start, start);
        assert_eq!(
            expanded_right.end,
            NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()
        );
    }

    #[test]
    fn bracket_boundary_shift_uses_month_scale_and_visible_end_boundary() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 28).unwrap();
        let window = ReportWindow::new(
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        )
        .unwrap();

        let start_shift =
            shifted_report_boundary_month(&window, ReportRangeBoundary::Start, 1, today).unwrap();
        assert_eq!(
            start_shift.start,
            NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()
        );

        let end_shift =
            shifted_report_boundary_month(&window, ReportRangeBoundary::End, 1, today).unwrap();
        assert_eq!(end_shift.end, NaiveDate::from_ymd_opt(2026, 7, 31).unwrap());

        let reaches_present = ReportWindow::new(
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
        )
        .unwrap();
        assert_eq!(
            shifted_report_boundary_month(&reaches_present, ReportRangeBoundary::End, 1, today,)
                .unwrap()
                .end,
            today
        );
    }

    #[test]
    fn bracket_boundaries_keep_a_one_day_minimum_and_never_extend_past_today() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let one_day = ReportWindow::new(today, today).unwrap();

        assert!(shifted_report_boundary(&one_day, ReportRangeBoundary::Start, 1, today).is_none());
        assert!(shifted_report_boundary(&one_day, ReportRangeBoundary::End, -1, today).is_none());
        assert!(shifted_report_boundary(&one_day, ReportRangeBoundary::End, 1, today).is_none());

        let past = ReportWindow::new(
            NaiveDate::from_ymd_opt(2026, 8, 15).unwrap(),
            NaiveDate::from_ymd_opt(2026, 8, 16).unwrap(),
        )
        .unwrap();
        let reaches_today =
            shifted_report_boundary(&past, ReportRangeBoundary::End, 1, today).unwrap();
        assert_eq!(reaches_today.end, today);
        assert!(
            shifted_report_boundary(&reaches_today, ReportRangeBoundary::End, 1, today).is_none()
        );
    }

    #[test]
    fn historical_preview_build_does_not_mutate_source_and_rebuilds_from_original_topology() {
        let category_id = CategoryId::new(1);
        let state = SandState {
            version: SandState::VERSION,
            grid_width: 4,
            grid_height: 8,
            grains: vec![SandStateGrain {
                x: 1,
                y: 1,
                category_id: category_id.0,
            }],
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 7,
            ingress_focus_x: None,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
            active_avalanche_columns: Vec::new(),
            mobilized_grains: Vec::new(),
            classic_runtime: None,
        };
        let snapshot = SedimentSnapshot::derived_preview(
            "2026-09-22".to_string(),
            "source-preview".to_string(),
            state.clone(),
        );
        let original = snapshot.clone();
        let categories = vec![preview_category(category_id)];

        let mut first = build_historical_preview(&snapshot, 2, 2, &categories)
            .expect("build first historical preview");
        first.engine.update_physics_only();
        first.engine.update_physics_only();

        let second = build_historical_preview(&snapshot, 2, 2, &categories)
            .expect("rebuild historical preview");

        assert_eq!(snapshot, original);
        assert_eq!(second.source_key, snapshot.source_revision);
        assert_eq!(second.engine.snapshot_state().grains, state.grains);
    }
}
