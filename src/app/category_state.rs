use ratatui::style::Color;

use crate::{
    constants::CATEGORY_SETTINGS,
    domain::{CategoryId, DRIFT_CATEGORY_ID},
    sqlite,
};

use super::{App, PersistenceOperation, RecoveryAction};
use chrono::NaiveDate;

impl App {
    pub(super) fn persist_categories(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CategorySync,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let categories = self.time_tracker.categories_for_storage();
        let result = sqlite::sync_tui_categories(
            &database_path,
            &categories,
            self.time_tracker.active_category_id(),
            self.session.active_session_stable_id.as_deref(),
        );
        if let Some(archived) = self.record_storage_result_for(
            PersistenceOperation::CategorySync,
            RecoveryAction::FlushCurrentState,
            result,
        ) {
            self.archived_categories = archived;
        }
    }

    pub(super) fn persist_sessions(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::SessionSync,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let result = sqlite::sync_tui_sessions(&database_path, &self.time_tracker.sessions);
        self.record_storage_result_for(
            PersistenceOperation::SessionSync,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn persist_sand_state(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let state = self.sand_engine.snapshot_state();
        let result = sqlite::save_tui_sand_state(&database_path, &state);
        self.record_storage_result_for(
            PersistenceOperation::SandStateSave,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn persist_daily_sand_snapshot(&mut self) {
        let pending = self.persist_pending_day_end_snapshots();
        if self
            .record_storage_result_for(
                PersistenceOperation::DailySnapshotSave,
                RecoveryAction::FlushCurrentState,
                pending,
            )
            .is_none()
        {
            return;
        }
        let day = crate::domain::operational_day_key_now();
        let day_key = day.format("%Y-%m-%d").to_string();
        let latest = crate::sand::SedimentSnapshot::latest_daily_checkpoint(
            day_key.clone(),
            self.sand_engine.snapshot_state(),
        );
        let latest_result = self
            .sqlite_database_path
            .as_deref()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())
            .and_then(|path| {
                sqlite::save_tui_latest_day_checkpoint(path, &day_key, &latest, chrono::Utc::now())
            });
        if self
            .record_storage_result_for(
                PersistenceOperation::DailySnapshotSave,
                RecoveryAction::FlushCurrentState,
                latest_result,
            )
            .is_none()
        {
            return;
        }
        self.reconcile_all_daily_contributions();
    }

    pub(super) fn persist_category_tags(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CategoryTagsSync,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id)
            .collect::<Vec<_>>();
        let result =
            sqlite::sync_tui_category_tags(&database_path, &self.category_tags, &category_ids);
        self.record_storage_result_for(
            PersistenceOperation::CategoryTagsSync,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn restore_sand_state(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::StateReload,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let state = match sqlite::load_tui_sand_state(&database_path) {
            Ok(value) => value,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::StateReload,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        };
        let Some(state) = state else {
            return;
        };
        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<std::collections::HashSet<_>>();
        if let Err(error) = self.sand_engine.restore_state(&state, &valid_category_ids) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::StateReload,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
        }
    }

    pub(super) fn load_daily_sediment_snapshot(
        &mut self,
        day: NaiveDate,
    ) -> Option<crate::sand::SedimentSnapshot> {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::StateReload,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return None;
        };
        let day = day.format("%Y-%m-%d").to_string();
        match sqlite::load_tui_daily_snapshot(&database_path, &day) {
            Ok(value) => value,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::StateReload,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                None
            }
        }
    }

    pub(super) fn load_day_end_sediment_snapshot(
        &mut self,
        day: NaiveDate,
    ) -> Option<crate::sand::SedimentSnapshot> {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::StateReload,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return None;
        };
        let day = day.format("%Y-%m-%d").to_string();
        match sqlite::load_tui_day_end_snapshot(&database_path, &day) {
            Ok(value) => value,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::StateReload,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                None
            }
        }
    }

    pub(super) fn persist_pending_day_end_snapshots(&mut self) -> Result<(), String> {
        let mut wrote_any = false;
        while let Some(pending) = self.pending_day_end_snapshots.first().cloned() {
            let database_path = self
                .sqlite_database_path
                .clone()
                .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
            let day = pending.operational_day.format("%Y-%m-%d").to_string();
            sqlite::save_tui_day_end_snapshot(
                &database_path,
                &day,
                &pending.snapshot,
                pending.captured_at_utc,
            )?;
            self.pending_day_end_snapshots.remove(0);
            wrote_any = true;
        }
        if wrote_any {
            self.clear_report_snapshot_cache();
        }
        Ok(())
    }

    pub(super) fn save_daily_sediment_snapshot(
        &mut self,
        day: NaiveDate,
        snapshot: &crate::sand::SedimentSnapshot,
    ) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::DailySnapshotSave,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let day = day.format("%Y-%m-%d").to_string();
        let result = sqlite::save_tui_daily_snapshot(&database_path, &day, snapshot);
        self.record_storage_result_for(
            PersistenceOperation::DailySnapshotSave,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn delete_daily_sediment_snapshot(&mut self, day: NaiveDate) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::DailySnapshotDelete,
                RecoveryAction::ReloadAuthority,
                Err("SQLite authority is unavailable".to_string()),
            );
            return;
        };
        let day = day.format("%Y-%m-%d").to_string();
        let result = sqlite::delete_tui_daily_snapshot(&database_path, &day);
        self.record_storage_result_for(
            PersistenceOperation::DailySnapshotDelete,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    pub(super) fn sync_modal_description_from_selection(&mut self) {
        self.modal_editing_category_metadata = false;
        if self.is_on_insert_space() {
            self.modal_description.clear();
        } else if self.time_tracker.active_category_index() == Some(self.selected_index) {
            self.modal_description = self.time_tracker.active_description().to_string();
        } else {
            self.modal_description.clear();
        }
        self.modal_tag_index = None;
        self.modal_tag_cycle_prefix = None;
    }

    pub(super) fn preview_active_description_from_modal(&mut self) {
        if self.modal_editing_category_metadata
            || self.is_on_insert_space()
            || self.time_tracker.active_category_index() != Some(self.selected_index)
        {
            return;
        }
        if self.time_tracker.active_description() != self.modal_description {
            self.time_tracker
                .set_active_description(self.modal_description.clone());
            self.modal_active_description_dirty = true;
        }
    }

    pub(super) fn toggle_category_metadata_edit(&mut self) {
        if self.is_on_insert_space() {
            return;
        }
        self.modal_editing_category_metadata = !self.modal_editing_category_metadata;
        self.modal_description = if self.modal_editing_category_metadata {
            self.time_tracker
                .category_description_by_index(self.selected_index)
                .unwrap_or_default()
        } else if self.time_tracker.active_category_index() == Some(self.selected_index) {
            self.time_tracker.active_description().to_string()
        } else {
            String::new()
        };
        self.modal_tag_index = None;
        self.modal_tag_cycle_prefix = None;
    }

    fn selected_category_id(&self) -> Option<CategoryId> {
        if self.is_on_insert_space() {
            None
        } else {
            self.time_tracker
                .category_by_index(self.selected_index)
                .map(|category| category.id)
        }
    }

    fn known_tags_for_category(&self, category_id: CategoryId) -> Vec<String> {
        let mut known = Vec::new();
        if let Some(stored) = self.category_tags.tags_by_category.get(&category_id.0) {
            for stored_value in stored {
                for tag in super::tagging::parse_tags(stored_value) {
                    if !known
                        .iter()
                        .any(|candidate: &String| candidate.eq_ignore_ascii_case(&tag))
                    {
                        known.push(tag);
                    }
                }
            }
        }
        for session in self
            .time_tracker
            .sessions
            .iter()
            .filter(|session| session.category_id == category_id)
        {
            for tag in super::tagging::parse_tags(&session.description) {
                if !known
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(&tag))
                {
                    known.push(tag);
                }
            }
        }
        if self.time_tracker.active_category_id() == category_id {
            for tag in super::tagging::parse_tags(self.time_tracker.active_description()) {
                if !known
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(&tag))
                {
                    known.push(tag);
                }
            }
        }
        known
    }

    pub(super) fn canonicalize_description_for_category(
        &self,
        category_id: CategoryId,
        description: &str,
    ) -> String {
        let known_tags = self.known_tags_for_category(category_id);
        super::tagging::canonicalize_description(description, &known_tags)
    }

    pub(super) fn tag_completion_for_category(
        &self,
        category_id: CategoryId,
        description: &str,
    ) -> Option<super::tagging::TagCompletion> {
        let known_tags = self.known_tags_for_category(category_id);
        super::tagging::tag_completion(description, &known_tags)
    }

    pub(super) fn remember_description_tags_for_category(
        &mut self,
        category_id: CategoryId,
        description: &str,
    ) {
        let parsed = super::tagging::parse_tags(description);
        if parsed.is_empty() {
            return;
        }
        let tags = self
            .category_tags
            .tags_by_category
            .entry(category_id.0)
            .or_default();
        let mut merged = parsed;
        for existing_value in tags.drain(..) {
            for existing in super::tagging::parse_tags(&existing_value) {
                if !merged
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(&existing))
                {
                    merged.push(existing);
                }
            }
        }
        merged.truncate(CATEGORY_SETTINGS.max_tags_per_category);
        *tags = merged;
        self.persist_category_tags();
    }

    pub(super) fn remember_selected_tag(&mut self) {
        let Some(category_id) = self.selected_category_id() else {
            return;
        };

        let canonical =
            self.canonicalize_description_for_category(category_id, &self.modal_description);
        if canonical != self.modal_description {
            self.modal_description = canonical.clone();
            self.preview_active_description_from_modal();
        }
        self.remember_description_tags_for_category(category_id, &canonical);
        self.modal_tag_index = (!super::tagging::parse_tags(&canonical).is_empty()).then_some(0);
        self.modal_tag_cycle_prefix = None;
    }

    pub(super) fn cycle_selected_tag(&mut self, direction: isize) {
        let Some(category_id) = self.selected_category_id() else {
            return;
        };
        let tags = self.known_tags_for_category(category_id);
        let Some(cycle) = super::tagging::cycle_tag(
            &self.modal_description,
            &tags,
            self.modal_tag_cycle_prefix.as_deref(),
            direction,
        ) else {
            return;
        };
        self.modal_description = cycle.value;
        self.modal_tag_cycle_prefix = Some(cycle.prefix);
        let current = self
            .modal_description
            .rsplit(';')
            .next()
            .unwrap_or_default()
            .trim();
        self.modal_tag_index = tags
            .iter()
            .position(|tag| tag.eq_ignore_ascii_case(current));
        self.preview_active_description_from_modal();
    }

    pub(super) fn is_on_insert_space(&self) -> bool {
        self.selected_index == self.time_tracker.category_count()
    }

    pub(super) fn add_category(&mut self) {
        let requested_name = self.new_category_name.trim();
        if requested_name.is_empty() {
            return;
        }

        let restored = self
            .archived_categories
            .iter()
            .position(|category| category.name.eq_ignore_ascii_case(requested_name))
            .and_then(|index| {
                let category = self.archived_categories[index].clone();
                self.time_tracker
                    .restore_category(category)
                    .then_some((index, self.archived_categories[index].id))
            });

        let added_id = if let Some((archived_index, category_id)) = restored {
            self.archived_categories.remove(archived_index);
            Some(category_id)
        } else {
            self.time_tracker.add_category_with_color(
                requested_name.to_string(),
                String::new(),
                self.appearance
                    .sand_color_at(self.new_category_color_cursor),
            )
        };

        if let Some(added_id) = added_id {
            if !self.persist_modal_active_description() {
                return;
            }
            self.persist_categories();
            self.switch_active_category_at(
                added_id,
                String::new(),
                chrono::Utc::now(),
                super::SessionClockMode::LiveMonotonic,
            );
            self.sync_modal_description_from_selection();
        }
    }

    pub(super) fn delete_category(&mut self) {
        if !self.is_on_insert_space()
            && self.selected_index < self.time_tracker.category_count()
            && self.selected_index > 0
        {
            let removed_category = self
                .time_tracker
                .category_by_index(self.selected_index)
                .cloned();
            let removed_id = removed_category.as_ref().map(|category| category.id);

            let was_active = removed_id
                .map(|category_id| category_id == self.time_tracker.active_category_id())
                .unwrap_or(false);

            if was_active {
                if !self.persist_modal_active_description() {
                    return;
                }
                self.switch_active_category_at(
                    DRIFT_CATEGORY_ID,
                    String::new(),
                    chrono::Utc::now(),
                    super::SessionClockMode::LiveMonotonic,
                );
            }

            if let Some(category_id) = removed_id
                && let Some(database_path) = self.sqlite_database_path.clone()
            {
                let result = sqlite::archive_tui_category(&database_path, category_id);
                if self
                    .record_storage_result_for(
                        PersistenceOperation::CategoryArchive,
                        RecoveryAction::ReloadAuthority,
                        result,
                    )
                    .is_none()
                {
                    return;
                }
            }

            if self.time_tracker.delete_category(self.selected_index) {
                if self.sqlite_database_path.is_none()
                    && let Some(category) = removed_category
                    && !self
                        .archived_categories
                        .iter()
                        .any(|archived| archived.id == category.id)
                {
                    self.archived_categories.push(category);
                }

                if self.selected_index > 0
                    && self.selected_index >= self.time_tracker.category_count()
                {
                    self.selected_index = self.time_tracker.category_count();
                }
                self.persist_categories();
                self.sync_modal_description_from_selection();
            }
        }
    }

    pub(super) fn get_selected_color(&self) -> Color {
        if self.is_on_insert_space() {
            self.appearance
                .sand_color_at(self.new_category_color_cursor)
        } else if let Some(category) = self.time_tracker.category_by_index(self.selected_index) {
            self.resolved_category_color(category.id, category.color)
        } else {
            self.theme_foreground()
        }
    }

    pub(super) fn get_active_color(&self) -> Color {
        if let Some(idx) = self.time_tracker.active_category_index()
            && let Some(category) = self.time_tracker.category_by_index(idx)
        {
            return self.resolved_category_color(category.id, category.color);
        }
        self.theme_foreground()
    }
}
