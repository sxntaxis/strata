use ratatui::style::Color;

use crate::{constants::COLORS, domain::CategoryId, storage};

use super::App;

impl App {
    pub(super) fn persist_categories(&self) {
        let categories = self.time_tracker.categories_for_storage();
        let path = storage::get_data_dir().join("categories.csv");
        let _ = storage::save_categories_to_csv(&path, &categories);
    }

    pub(super) fn persist_sessions(&self) {
        let categories = self.time_tracker.categories_for_storage();
        let path = storage::get_data_dir().join("time_log.csv");
        let _ = storage::save_sessions_to_csv(&path, &self.time_tracker.sessions, &categories);
    }

    pub(super) fn persist_sand_state(&self) {
        let state = self.sand_engine.snapshot_state();
        let path = storage::get_sand_state_path();
        let _ = storage::save_sand_state(&path, &state);
    }

    pub(super) fn persist_category_tags(&self) {
        let path = storage::get_category_tags_path();
        let _ = storage::save_category_tags(&path, &self.category_tags);
    }

    pub(super) fn restore_sand_state(&mut self) {
        let path = storage::get_sand_state_path();
        let Some(state) = storage::load_sand_state(&path) else {
            return;
        };

        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id)
            .collect::<std::collections::HashSet<_>>();

        self.sand_engine.restore_state(&state, &valid_category_ids);
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
        const MAX_TAGS_PER_CATEGORY: usize = 24;
        tags.truncate(MAX_TAGS_PER_CATEGORY);

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
        if !self.new_category_name.is_empty() {
            let added = self.time_tracker.add_category(
                self.new_category_name.clone(),
                String::new(),
                Some(self.color_index),
            );
            if added.is_some() {
                let index = self.time_tracker.category_count().saturating_sub(1);
                let _ = self.time_tracker.set_active_category_by_index(index);
                self.time_tracker.start_session();
                self.persist_categories();
                self.sync_modal_description_from_selection();
            }
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
        if let Some(idx) = self.time_tracker.active_category_index() {
            if let Some(category) = self.time_tracker.category_by_index(idx) {
                return category.color;
            }
        }
        Color::White
    }
}
