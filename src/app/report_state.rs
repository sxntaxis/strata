use std::collections::HashSet;

use chrono::{Duration as ChronoDuration, NaiveDate};
use ratatui::{prelude::Line, style::Color};

use crate::domain::{
    Category, CategoryId, CategoryLogEntry, DRIFT_CATEGORY_ID, KarmaReportSummary,
    LiveSessionPreview, OperationalDayPolicy, ReportPeriod,
    build_category_logs_for_period_with_offset, build_period_karma_report_with_live_and_offset,
    day_boundary_config, operational_day_key_now, report_period_date_bounds_with_offset,
    session_slices,
};
use crate::sand::{SandEngine, SandState, SandStateGrain};

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
        let description = self
            .time_tracker
            .category_description_by_id(category_id)
            .map(ToString::to_string)
            .unwrap_or_default();

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
        let state = self.report_snapshot_state.clone()?;

        let categories = self.report_categories();
        let valid_category_ids: HashSet<CategoryId> =
            categories.iter().map(|category| category.id).collect();

        let cache_key = format!(
            "{}:{}:{}:{}",
            self.report_snapshot_end_day.as_deref().unwrap_or_default(),
            width,
            height,
            state.grains.len()
        );

        let should_rebuild_preview = self
            .report_snapshot_preview_key
            .as_deref()
            .map(|key| key != cache_key.as_str())
            .unwrap_or(true)
            || self.report_snapshot_preview_engine.is_none();

        if should_rebuild_preview {
            let mut preview_engine = SandEngine::new(width, height);
            preview_engine.restore_state(&state, &valid_category_ids);
            self.report_snapshot_preview_engine = Some(preview_engine);
            self.report_snapshot_preview_key = Some(cache_key);
        }

        let preview_engine = self.report_snapshot_preview_engine.as_mut()?;
        preview_engine.update();
        Some(preview_engine.render(&categories))
    }

    pub(super) fn clear_report_snapshot_cache(&mut self) {
        self.report_snapshot_end_day = None;
        self.report_snapshot_state = None;
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_engine = None;
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

        if self.sqlite_database_path.is_none() {
            self.persist_sessions();
        }
        self.rebuild_report_snapshot_for_interval_end_day();

        let refreshed = self.report_current_logs();
        self.clamp_report_log_selection(refreshed.len());
        true
    }

    pub(super) fn append_to_selected_report_session_tag(&mut self, ch: char) -> bool {
        self.update_selected_report_session_tag(|description| description.push(ch))
    }

    pub(super) fn backspace_selected_report_session_tag(&mut self) -> bool {
        self.update_selected_report_session_tag(|description| {
            description.pop();
        })
    }

    fn update_selected_report_session_tag<F>(&mut self, mutator: F) -> bool
    where
        F: FnOnce(&mut String),
    {
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

        let mut description = row.description.clone();
        mutator(&mut description);

        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = crate::sqlite::update_tui_session_description(
                &database_path,
                session_id,
                &description,
            );
            if self
                .record_storage_result_for(
                    PersistenceOperation::SessionEdit,
                    RecoveryAction::ReloadAuthority,
                    result,
                )
                .is_none()
            {
                return false;
            }
        }
        if !self
            .time_tracker
            .set_session_description_by_id(session_id, description)
        {
            return false;
        }

        if self.sqlite_database_path.is_none() {
            self.persist_sessions();
        }
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

        self.report_snapshot_end_day = Some(key);
        self.report_snapshot_state = self
            .load_daily_sand_snapshot(end_day)
            .or_else(|| self.synthetic_snapshot_from_time_log(end_day));
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_engine = None;
    }

    pub(super) fn rebuild_report_snapshot_for_interval_end_day(&mut self) {
        let end_day = self.report_interval_end_day();
        let key = end_day.format("%Y-%m-%d").to_string();
        if let Some(state) = self.synthetic_snapshot_from_time_log(end_day) {
            self.save_daily_sand_snapshot(end_day, &state);
            self.report_snapshot_state = Some(state);
        } else {
            self.delete_daily_sand_snapshot(end_day);
            self.report_snapshot_state = None;
        }

        self.report_snapshot_end_day = Some(key);
        self.report_snapshot_preview_key = None;
        self.report_snapshot_preview_engine = None;
    }

    fn synthetic_snapshot_from_time_log(&self, day: NaiveDate) -> Option<SandState> {
        let mut day_sessions: Vec<(u64, usize, String, String, usize)> = self
            .time_tracker
            .sessions
            .iter()
            .flat_map(|session| {
                session_slices(session)
                    .into_iter()
                    .filter(move |slice| slice.operational_day == day)
                    .map(move |slice| {
                        (
                            session.category_id.0,
                            slice.elapsed_seconds,
                            slice.start_time,
                            slice.end_time,
                            session.id,
                        )
                    })
            })
            .collect();

        if day == operational_day_key_now()
            && let Some(live) = self.live_session_preview()
        {
            let preview = crate::domain::Session {
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
            };
            day_sessions.extend(
                session_slices(&preview)
                    .into_iter()
                    .filter(|slice| slice.operational_day == day)
                    .map(|slice| {
                        (
                            preview.category_id.0,
                            slice.elapsed_seconds,
                            slice.start_time,
                            slice.end_time,
                            usize::MAX,
                        )
                    }),
            );
        }

        if day_sessions.is_empty() {
            return None;
        }

        day_sessions.sort_by(|a, b| a.2.cmp(&b.2).then(a.3.cmp(&b.3)).then(a.4.cmp(&b.4)));

        let grid_width = self.sand_engine.width as usize;
        let grid_height = self.sand_engine.height as usize;
        let capacity = grid_width.saturating_mul(grid_height);
        if capacity == 0 {
            return None;
        }

        let total_seconds: usize = day_sessions
            .iter()
            .map(|(_, seconds, _, _, _)| *seconds)
            .sum();
        let target_grains = total_seconds.min(capacity);
        if target_grains == 0 {
            return None;
        }

        let mut allocations: Vec<(u64, usize, usize, usize)> = day_sessions
            .iter()
            .enumerate()
            .map(|(order, (category_id, seconds, _, _, _))| {
                let weighted = (*seconds as u128) * (target_grains as u128);
                let base = (weighted / (total_seconds as u128)) as usize;
                let remainder = (weighted % (total_seconds as u128)) as usize;
                (*category_id, base, remainder, order)
            })
            .collect();

        let mut assigned: usize = allocations.iter().map(|(_, count, _, _)| *count).sum();
        if assigned < target_grains {
            let mut ranked: Vec<usize> = (0..allocations.len()).collect();
            ranked.sort_by(|a, b| {
                allocations[*b]
                    .2
                    .cmp(&allocations[*a].2)
                    .then(allocations[*a].3.cmp(&allocations[*b].3))
            });

            let mut idx = 0usize;
            while assigned < target_grains && !ranked.is_empty() {
                let allocation_index = ranked[idx];
                allocations[allocation_index].1 += 1;
                assigned += 1;
                idx = (idx + 1) % ranked.len();
            }
        }

        allocations.sort_by_key(|(_, _, _, order)| *order);

        let mut grains = Vec::with_capacity(target_grains);
        for (category_id, count, _, _) in allocations {
            for _ in 0..count {
                let grain_index = grains.len();
                let x = grain_index % grid_width;
                let row = grain_index / grid_width;
                if row >= grid_height {
                    break;
                }

                let y = grid_height - 1 - row;
                grains.push(SandStateGrain { x, y, category_id });
            }
            if grains.len() >= target_grains {
                break;
            }
        }

        Some(SandState {
            version: SandState::VERSION,
            grid_width,
            grid_height,
            grains,
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 0,
        })
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
