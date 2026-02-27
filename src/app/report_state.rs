use chrono::Local;
use ratatui::style::Color;

use crate::domain::{
    build_category_logs_for_period, build_period_karma_report_with_live, CategoryId,
    CategoryLogEntry, KarmaReportSummary, LiveSessionPreview, ReportPeriod,
};

use super::App;

impl App {
    pub(super) fn category_name_for_id(&self, category_id: CategoryId) -> String {
        self.time_tracker
            .category_name_by_id(category_id)
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub(super) fn category_color_for_id(&self, category_id: CategoryId) -> Color {
        self.time_tracker
            .category_color_by_id(category_id)
            .unwrap_or(Color::White)
    }

    pub(super) fn report_rows(&self) -> KarmaReportSummary {
        let categories = self.time_tracker.categories_for_storage();
        let live_preview = self.live_session_preview();

        build_period_karma_report_with_live(
            &self.time_tracker.sessions,
            &categories,
            self.report_period,
            live_preview.as_ref(),
        )
    }

    pub(super) fn report_logs_for_category(
        &self,
        category_id: CategoryId,
    ) -> Vec<CategoryLogEntry> {
        let categories = self.time_tracker.categories_for_storage();
        let live_preview = self.live_session_preview();

        build_category_logs_for_period(
            &self.time_tracker.sessions,
            &categories,
            category_id,
            self.report_period,
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
        let description = self
            .time_tracker
            .category_description_by_id(category_id)
            .map(ToString::to_string)
            .unwrap_or_default();

        Some(LiveSessionPreview {
            category_id,
            description,
            elapsed_seconds,
            now_local: Local::now(),
        })
    }

    pub(super) fn set_report_period(&mut self, period: ReportPeriod) {
        self.report_period = period;
        if self.report_logs_category_id.is_some() {
            let row_count = self.report_current_logs().len();
            self.clamp_report_log_selection(row_count);
        } else {
            let row_count = self.report_rows().entries.len();
            self.clamp_report_selection(row_count);
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
