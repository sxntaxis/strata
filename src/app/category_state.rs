use ratatui::style::Color;

use crate::{
    constants::{CATEGORY_SETTINGS, COLORS},
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

    pub(super) fn remember_selected_tag(&mut self) {
        let Some(category_id) = self.selected_category_id() else {
            return;
        };

        let tag = self.modal_description.trim();
        if tag.is_empty() {
            return;
        }

        let tags = self
            .category_tags
            .tags_by_category
            .entry(category_id.0)
            .or_default();
        tags.retain(|existing| existing != tag);
        tags.insert(0, tag.to_string());
        tags.truncate(CATEGORY_SETTINGS.max_tags_per_category);

        self.modal_tag_index = Some(0);
        self.persist_category_tags();
    }

    pub(super) fn cycle_selected_tag(&mut self, direction: isize) {
        let Some(category_id) = self.selected_category_id() else {
            return;
        };

        let Some(tags) = self.category_tags.tags_by_category.get(&category_id.0) else {
            return;
        };

        if tags.is_empty() {
            return;
        }

        let len = tags.len();
        let next_index = if let Some(current_index) = self.modal_tag_index {
            if direction < 0 {
                (current_index + len - 1) % len
            } else {
                (current_index + 1) % len
            }
        } else if !self.modal_description.trim().is_empty() {
            if let Some(existing_index) = tags
                .iter()
                .position(|tag| tag == self.modal_description.trim())
            {
                if direction < 0 {
                    (existing_index + len - 1) % len
                } else {
                    (existing_index + 1) % len
                }
            } else if direction < 0 {
                len - 1
            } else {
                0
            }
        } else if direction < 0 {
            len - 1
        } else {
            0
        };

        self.modal_tag_index = Some(next_index);
        self.modal_description = tags[next_index].clone();
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
            self.time_tracker.add_category(
                requested_name.to_string(),
                String::new(),
                Some(self.color_index),
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
            COLORS[self.color_index]
        } else if let Some(category) = self.time_tracker.category_by_index(self.selected_index) {
            category.color
        } else {
            Color::White
        }
    }

    pub(super) fn get_active_color(&self) -> Color {
        if let Some(idx) = self.time_tracker.active_category_index()
            && let Some(category) = self.time_tracker.category_by_index(idx)
        {
            return category.color;
        }
        Color::White
    }
}
