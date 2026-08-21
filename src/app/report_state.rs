use std::collections::BTreeSet;

use chrono::{Duration as ChronoDuration, NaiveDate};
use ratatui::{prelude::Line, style::Color};

use crate::domain::{
    Category, CategoryId, CategoryLogEntry, DRIFT_CATEGORY_ID, KarmaReportSummary,
    LiveSessionPreview, OperationalDayPolicy, ReportPeriod,
    build_category_logs_for_period_with_offset, build_period_karma_report_with_live_and_offset,
    day_boundary_config, operational_day_key_now, report_period_date_bounds_with_offset,
    session_slices,
};
use crate::sand::{
    DailySedimentSlice, SedimentSnapshot, daily_contribution_from_slices,
    derived_preview_from_slices, select_daily_artifact,
};

use super::{App, PersistenceOperation, RecoveryAction};

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

    pub(super) fn report_rows(&self) -> KarmaReportSummary {
        let categories = self.report_categories();
        let live_preview = self.live_session_preview();

        build_period_karma_report_with_live_and_offset(
            &self.time_tracker.sessions,
            &categories,
            self.report_period,
            self.report_period_offset,
            live_preview.as_ref(),
        )
    }

    pub(super) fn report_logs_for_category(
        &self,
        category_id: CategoryId,
    ) -> Vec<CategoryLogEntry> {
        let categories = self.report_categories();
        let live_preview = self.live_session_preview();

        build_category_logs_for_period_with_offset(
            &self.time_tracker.sessions,
            &categories,
            category_id,
            self.report_period,
            self.report_period_offset,
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

    pub(super) fn set_report_period(&mut self, period: ReportPeriod) {
        self.report_period = period;
        self.report_period_offset = 0;
        self.clear_report_snapshot_cache();
        self.sync_report_selection_for_interval();
    }

    pub(super) fn shift_report_interval_older(&mut self) {
        self.report_period_offset = self.report_period_offset.saturating_add(1);
        self.clear_report_snapshot_cache();
        self.sync_report_selection_for_interval();
    }

    pub(super) fn shift_report_interval_newer(&mut self) {
        if self.report_period_offset > 0 {
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
        report_period_date_bounds_with_offset(self.report_period, self.report_period_offset).1
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

        let persisted = self.load_daily_sediment_snapshot(end_day);
        let derived = self.synthetic_snapshot_from_time_log(end_day);

        self.report_snapshot_end_day = Some(key.clone());
        self.report_snapshot_artifact = select_daily_artifact(&key, persisted, derived);
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_lines = None;
    }

    pub(super) fn daily_contribution_from_time_log(
        &self,
        day: NaiveDate,
    ) -> Option<SedimentSnapshot> {
        let slices = self.daily_sediment_slices(day);
        let day_key = day.format("%Y-%m-%d").to_string();
        daily_contribution_from_slices(
            &day_key,
            self.sand_engine.grid_width_dots,
            self.sand_engine.grid_height_dots,
            &slices,
        )
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
    use super::retain_report_edit_after_commit;
    use crate::app::ReportLogEditState;

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
}
