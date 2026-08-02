use ratatui::style::Color;

use crate::{
    constants::{CATEGORY_SETTINGS, COLORS},
    domain::{CategoryId, DRIFT_CATEGORY_ID},
    sqlite, storage,
};

use super::{App, PersistenceOperation, RecoveryAction};
use chrono::NaiveDate;

impl App {
    pub(super) fn persist_categories(&mut self) {
        let categories = self.time_tracker.categories_for_storage();
        if let Some(database_path) = self.sqlite_database_path.clone() {
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
        } else {
            let path = storage::get_categories_path();
            if let Err(error) = storage::save_categories_to_csv(&path, &categories) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn persist_sessions(&mut self) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::sync_tui_sessions(&database_path, &self.time_tracker.sessions);
            self.record_storage_result_for(
                PersistenceOperation::SessionSync,
                RecoveryAction::FlushCurrentState,
                result,
            );
        } else {
            let categories = self.time_tracker.categories_for_storage();
            let path = storage::get_time_log_path();
            if let Err(error) =
                storage::save_sessions_to_csv(&path, &self.time_tracker.sessions, &categories)
            {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn persist_sand_state(&mut self) {
        let state = self.sand_engine.snapshot_state();
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let result = sqlite::save_tui_sand_state(&database_path, &state);
            self.record_storage_result_for(
                PersistenceOperation::SandStateSave,
                RecoveryAction::FlushCurrentState,
                result,
            );
        } else {
            let path = storage::get_sand_state_path();
            if let Err(error) = storage::save_sand_state(&path, &state) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn persist_daily_sand_snapshot(&mut self) {
        self.reconcile_all_daily_contributions();
    }

    pub(super) fn persist_category_tags(&mut self) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
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
        } else {
            let path = storage::get_category_tags_path();
            if let Err(error) = storage::save_category_tags(&path, &self.category_tags) {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }

    pub(super) fn restore_sand_state(&mut self) {
        let state = if let Some(database_path) = self.sqlite_database_path.clone() {
            match sqlite::load_tui_sand_state(&database_path) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::StateReload,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return;
                }
            }
        } else {
            storage::load_sand_state(&storage::get_sand_state_path())
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
        self.sand_engine.restore_state(&state, &valid_category_ids);
    }

    pub(super) fn load_daily_sediment_snapshot(
        &mut self,
        day: NaiveDate,
    ) -> Option<crate::sand::SedimentSnapshot> {
        if let Some(database_path) = self.sqlite_database_path.clone() {
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
        } else {
            let path = storage::get_sand_contribution_path_for_day(day);
            if !storage::file_exists(&path) {
                return None;
            }
            match storage::read_json::<crate::sand::SedimentSnapshot>(&path) {
                Ok(snapshot) => Some(snapshot),
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
    }

    pub(super) fn save_daily_sediment_snapshot(
        &mut self,
        day: NaiveDate,
        snapshot: &crate::sand::SedimentSnapshot,
    ) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let day = day.format("%Y-%m-%d").to_string();
            let result = sqlite::save_tui_daily_snapshot(&database_path, &day, snapshot);
            self.record_storage_result_for(
                PersistenceOperation::DailySnapshotSave,
                RecoveryAction::FlushCurrentState,
                result,
            );
        } else {
            let path = storage::get_sand_contribution_path_for_day(day);
            if let Err(error) = storage::write_json_atomic(&path, snapshot) {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::DailySnapshotSave,
                    RecoveryAction::FlushCurrentState,
                    Err(error),
                );
            }
        }
    }

    pub(super) fn delete_daily_sediment_snapshot(&mut self, day: NaiveDate) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let day = day.format("%Y-%m-%d").to_string();
            let result = sqlite::delete_tui_daily_snapshot(&database_path, &day);
            self.record_storage_result_for(
                PersistenceOperation::DailySnapshotDelete,
                RecoveryAction::FlushCurrentState,
                result,
            );
        } else {
            let path = storage::get_sand_contribution_path_for_day(day);
            if let Err(error) = storage::delete_file_if_exists(&path) {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::DailySnapshotDelete,
                    RecoveryAction::FlushCurrentState,
                    Err(error),
                );
            }
        }
    }

    pub(super) fn sync_modal_description_from_selection(&mut self) {
        if self.is_on_insert_space() {
            self.modal_description.clear();
        } else {
            self.modal_description = self
                .time_tracker
                .category_description_by_index(self.selected_index)
                .unwrap_or_default();
        }
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
                let mut category = self.archived_categories[index].clone();
                category.color = COLORS[self.color_index % COLORS.len()];
                category.description.clear();
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
            self.persist_categories();
            self.switch_active_category_at(
                added_id,
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
            let removed_id = self
                .time_tracker
                .category_by_index(self.selected_index)
                .map(|category| category.id);

            let was_active = removed_id
                .map(|category_id| category_id == self.time_tracker.active_category_id())
                .unwrap_or(false);

            if was_active {
                self.switch_active_category_at(
                    DRIFT_CATEGORY_ID,
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
                if let Some(category_id) = removed_id {
                    self.category_tags.tags_by_category.remove(&category_id.0);
                    self.persist_category_tags();
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
