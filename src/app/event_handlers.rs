use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Instant;

use crate::{
    constants::COLORS,
    domain::{CategoryId, ReportPeriod},
    keybindings::Action,
};

use super::{App, ui_helpers};

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Release {
            return false;
        }

        if self.show_keybindings_modal && self.atlas_overlay.is_some() {
            return self.handle_atlas_overlay_key(key);
        }

        if let Some(action) = self.resolve_action(key) {
            return self.route_action(action, key);
        }

        if self.in_category_modal() {
            self.handle_modal_text_input(key);
        }

        false
    }

    fn resolve_action(&self, key: KeyEvent) -> Option<Action> {
        if matches!(key.code, KeyCode::F(1)) {
            return Some(Action::ToggleKeybindingsHelp);
        }

        if self.in_category_modal()
            && !self.show_keybindings_modal
            && matches!(key.code, KeyCode::Char('?'))
        {
            return None;
        }

        self.keymap.action_for_key_event(key)
    }

    fn route_action(&mut self, action: Action, key: KeyEvent) -> bool {
        if action == Action::ToggleKeybindingsHelp {
            self.toggle_keybindings_modal();
            return false;
        }

        if self.show_keybindings_modal {
            return self.handle_keybindings_modal_action(action);
        }

        if self.in_category_modal() {
            let handled = self.handle_modal_action(action);
            if !handled {
                self.handle_modal_text_input(key);
            }
            return false;
        }

        if self.in_karma_modal() {
            self.handle_report_modal_action(action);
            return false;
        }

        self.handle_main_action(action)
    }

    fn handle_keybindings_modal_action(&mut self, action: Action) -> bool {
        match action {
            Action::Cancel => self.close_keybindings_modal(),
            Action::Up | Action::Left => self.select_previous_keybinding_action(),
            Action::Down | Action::Right => self.select_next_keybinding_action(),
            Action::Confirm => self.open_atlas_editor_for_selection(),
            Action::HelpTop => self.jump_keybindings_top(),
            Action::HelpBottom => self.jump_keybindings_bottom(),
            Action::Quit => return true,
            _ => {}
        }

        false
    }

    fn handle_atlas_overlay_key(&mut self, key: KeyEvent) -> bool {
        let Some(overlay) = self.atlas_overlay.clone() else {
            return false;
        };

        match overlay {
            super::AtlasOverlay::CaptureKey { action } => {
                self.handle_atlas_capture_key_input(action, key);
            }
            super::AtlasOverlay::EditTimeLogPath { .. } => {
                self.handle_atlas_time_log_input(key);
            }
            super::AtlasOverlay::SelectDayStartMode { .. } => {
                self.handle_atlas_day_start_dropdown(key);
            }
            super::AtlasOverlay::SelectWeekStartDay { .. } => {
                self.handle_atlas_week_start_dropdown(key);
            }
        }

        false
    }

    fn handle_atlas_capture_key_input(&mut self, action: Action, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.close_atlas_overlay();
            }
            KeyCode::Backspace | KeyCode::Delete => {
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_action_binding(&keymap_path, action, None) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_atlas_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_atlas_overlay();
                    }
                }
            }
            _ => {
                if let Some(binding) = crate::keybindings::KeyBinding::from_key_event(key) {
                    let keymap_path = crate::storage::get_keymap_path();
                    match crate::keybindings::set_action_binding(
                        &keymap_path,
                        action,
                        Some(binding),
                    ) {
                        Ok(loaded) => {
                            self.apply_loaded_keybindings(loaded);
                            self.close_atlas_overlay();
                        }
                        Err(err) => {
                            self.keymap_error = Some(err);
                            self.close_atlas_overlay();
                        }
                    }
                }
            }
        }
    }

    fn handle_atlas_time_log_input(&mut self, key: KeyEvent) {
        let Some(super::AtlasOverlay::EditTimeLogPath { mut input }) = self.atlas_overlay.take()
        else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                self.close_atlas_overlay();
                return;
            }
            KeyCode::Enter => {
                let trimmed = input.trim();
                let value = if trimmed.is_empty() {
                    None
                } else {
                    let candidate = std::path::PathBuf::from(trimmed);
                    let normalized = if candidate
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
                    {
                        candidate
                    } else {
                        candidate.join("time_log.csv")
                    };
                    Some(normalized.display().to_string())
                };

                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_time_log_path(&keymap_path, value) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_atlas_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_atlas_overlay();
                    }
                }
                return;
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                input.push(c);
            }
            _ => {}
        }

        self.atlas_overlay = Some(super::AtlasOverlay::EditTimeLogPath { input });
        self.render_needed = true;
    }

    fn handle_atlas_day_start_dropdown(&mut self, key: KeyEvent) {
        let Some(super::AtlasOverlay::SelectDayStartMode { mut selected }) =
            self.atlas_overlay.take()
        else {
            return;
        };

        let options = Self::day_start_mode_options();
        match key.code {
            KeyCode::Esc => {
                self.close_atlas_overlay();
                return;
            }
            KeyCode::Up | KeyCode::Left => {
                selected = if selected == 0 {
                    options.len().saturating_sub(1)
                } else {
                    selected - 1
                };
            }
            KeyCode::Down | KeyCode::Right => {
                selected = (selected + 1) % options.len().max(1);
            }
            KeyCode::Enter => {
                let mode = options.get(selected).copied().unwrap_or(options[0]);
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_day_start_mode(&keymap_path, mode) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_atlas_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_atlas_overlay();
                    }
                }
                return;
            }
            _ => {}
        }

        self.atlas_overlay = Some(super::AtlasOverlay::SelectDayStartMode { selected });
        self.render_needed = true;
    }

    fn handle_atlas_week_start_dropdown(&mut self, key: KeyEvent) {
        let Some(super::AtlasOverlay::SelectWeekStartDay { mut selected }) =
            self.atlas_overlay.take()
        else {
            return;
        };

        let options = Self::week_start_options();
        match key.code {
            KeyCode::Esc => {
                self.close_atlas_overlay();
                return;
            }
            KeyCode::Up | KeyCode::Left => {
                selected = if selected == 0 {
                    options.len().saturating_sub(1)
                } else {
                    selected - 1
                };
            }
            KeyCode::Down | KeyCode::Right => {
                selected = (selected + 1) % options.len().max(1);
            }
            KeyCode::Enter => {
                let week_start = options.get(selected).copied().unwrap_or(options[0]);
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_first_day_of_week(&keymap_path, week_start) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_atlas_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_atlas_overlay();
                    }
                }
                return;
            }
            _ => {}
        }

        self.atlas_overlay = Some(super::AtlasOverlay::SelectWeekStartDay { selected });
        self.render_needed = true;
    }

    fn handle_modal_action(&mut self, action: Action) -> bool {
        let mut handled = true;

        match action {
            Action::Cancel => self.close_modal(),
            Action::Up => {
                let total_rows = self.time_tracker.category_count() + 1;
                if total_rows > 0 {
                    self.selected_index =
                        ui_helpers::wrap_prev_index(self.selected_index, total_rows);
                    self.sync_modal_description_from_selection();
                }
            }
            Action::Down => {
                let total_rows = self.time_tracker.category_count() + 1;
                if total_rows > 0 {
                    self.selected_index =
                        ui_helpers::wrap_next_index(self.selected_index, total_rows);
                    self.sync_modal_description_from_selection();
                }
            }
            Action::Left => {
                if self.is_on_insert_space() {
                    self.color_index = (self.color_index + COLORS.len() - 1) % COLORS.len();
                } else {
                    self.cycle_selected_tag(-1);
                }
            }
            Action::Right => {
                if self.is_on_insert_space() {
                    self.color_index = (self.color_index + 1) % COLORS.len();
                } else {
                    self.cycle_selected_tag(1);
                }
            }
            Action::ShiftUp => {
                if self.time_tracker.move_category_up(self.selected_index) {
                    self.selected_index = self.selected_index.saturating_sub(1);
                    self.persist_categories();
                }
            }
            Action::ShiftDown => {
                if self.time_tracker.move_category_down(self.selected_index) {
                    self.selected_index += 1;
                    self.persist_categories();
                }
            }
            Action::ShiftLeft => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    let Some(current_color) = self
                        .time_tracker
                        .category_by_index(self.selected_index)
                        .map(|category| category.color)
                    else {
                        self.render_needed = true;
                        return true;
                    };
                    let current_pos = COLORS
                        .iter()
                        .position(|&color| color == current_color)
                        .unwrap_or(0);
                    let new_pos = (current_pos + COLORS.len() - 1) % COLORS.len();
                    if self
                        .time_tracker
                        .set_category_color_by_index(self.selected_index, COLORS[new_pos])
                    {
                        self.persist_categories();
                    }
                } else if self.is_on_insert_space() {
                    self.color_index = (self.color_index + COLORS.len() - 1) % COLORS.len();
                }
            }
            Action::ShiftRight => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    let Some(current_color) = self
                        .time_tracker
                        .category_by_index(self.selected_index)
                        .map(|category| category.color)
                    else {
                        self.render_needed = true;
                        return true;
                    };
                    let current_pos = COLORS
                        .iter()
                        .position(|&color| color == current_color)
                        .unwrap_or(0);
                    let new_pos = (current_pos + 1) % COLORS.len();
                    if self
                        .time_tracker
                        .set_category_color_by_index(self.selected_index, COLORS[new_pos])
                    {
                        self.persist_categories();
                    }
                } else if self.is_on_insert_space() {
                    self.color_index = (self.color_index + 1) % COLORS.len();
                }
            }
            Action::Confirm => {
                if self.is_on_insert_space() {
                    if !self.new_category_name.is_empty() {
                        self.add_category();
                        self.close_modal();
                    }
                } else {
                    if self.selected_index < self.time_tracker.category_count() {
                        if self.time_tracker.set_category_description_by_index(
                            self.selected_index,
                            self.modal_description.clone(),
                        ) {
                            self.persist_categories();
                        }
                        self.remember_selected_tag();
                    }
                    if self.time_tracker.active_category_index() != Some(self.selected_index) {
                        self.time_tracker.end_session();
                        self.persist_sessions();
                        let _ = self
                            .time_tracker
                            .set_active_category_by_index(self.selected_index);
                        self.time_tracker.start_session();
                        if self.selected_index != 0 {
                            self.none_entry_time = None;
                        }
                    }
                    self.close_modal();
                }
            }
            Action::DeleteCategory => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.delete_category();
                }
            }
            Action::IncreaseKarma => {
                if !self.is_on_insert_space()
                    && self.selected_index > 0
                    && self.selected_index < self.time_tracker.category_count()
                    && self
                        .time_tracker
                        .set_category_karma_by_index(self.selected_index, 1)
                {
                    self.persist_categories();
                }
            }
            Action::DecreaseKarma => {
                if !self.is_on_insert_space()
                    && self.selected_index > 0
                    && self.selected_index < self.time_tracker.category_count()
                    && self
                        .time_tracker
                        .set_category_karma_by_index(self.selected_index, -1)
                {
                    self.persist_categories();
                }
            }
            Action::Backspace => {
                if self.is_on_insert_space() {
                    self.new_category_name.pop();
                } else if self.selected_index < self.time_tracker.category_count() {
                    self.modal_tag_index = None;
                    self.modal_description.pop();
                }
            }
            _ => handled = false,
        }

        if handled {
            self.render_needed = true;
        }

        handled
    }

    fn handle_modal_text_input(&mut self, key: KeyEvent) {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }

        if let KeyCode::Char(c) = key.code {
            if self.is_on_insert_space() {
                self.new_category_name.push(c);
                self.render_needed = true;
            } else if self.selected_index < self.time_tracker.category_count() {
                self.modal_tag_index = None;
                self.modal_description.push(c);
                self.render_needed = true;
            }
        }
    }

    fn handle_report_modal_action(&mut self, action: Action) {
        let summary = self.report_rows();
        self.clamp_report_selection(summary.entries.len());
        let logs = self.report_current_logs();
        self.clamp_report_log_selection(logs.len());
        let in_logs_view = self.report_logs_category_id.is_some();

        let mut handled = true;

        match action {
            Action::Cancel => {
                if in_logs_view {
                    self.report_logs_category_id = None;
                    self.report_log_selected_index = 0;
                } else {
                    self.close_report_modal();
                }
            }
            Action::Confirm => {
                if in_logs_view {
                    self.report_logs_category_id = None;
                    self.report_log_selected_index = 0;
                } else if let Some(entry) = summary.entries.get(self.report_selected_index)
                    && entry.category_id != CategoryId::new(0)
                {
                    self.report_logs_category_id = Some(entry.category_id);
                    self.report_log_selected_index = 0;
                }
            }
            Action::Up => {
                if in_logs_view {
                    if !logs.is_empty() {
                        self.report_log_selected_index =
                            ui_helpers::wrap_prev_index(self.report_log_selected_index, logs.len());
                    }
                } else if !summary.entries.is_empty() {
                    self.report_selected_index = ui_helpers::wrap_prev_index(
                        self.report_selected_index,
                        summary.entries.len(),
                    );
                }
            }
            Action::Down => {
                if in_logs_view {
                    if !logs.is_empty() {
                        self.report_log_selected_index =
                            ui_helpers::wrap_next_index(self.report_log_selected_index, logs.len());
                    }
                } else if !summary.entries.is_empty() {
                    self.report_selected_index = ui_helpers::wrap_next_index(
                        self.report_selected_index,
                        summary.entries.len(),
                    );
                }
            }
            Action::ShiftLeft => {
                self.set_report_period(ui_helpers::report_period_prev(self.report_period));
            }
            Action::ShiftRight => {
                self.set_report_period(ui_helpers::report_period_next(self.report_period));
            }
            Action::ReportToday => {
                self.set_report_period(ReportPeriod::Today);
            }
            Action::ReportWeek => {
                self.set_report_period(ReportPeriod::Week);
            }
            Action::ReportMonth => {
                self.set_report_period(ReportPeriod::Month);
            }
            _ => handled = false,
        }

        if handled {
            self.render_needed = true;
        }
    }

    fn handle_main_action(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => true,
            Action::ClearAllSand => {
                self.sand_engine.clear();
                self.time_tracker.reset_none_counter_today();
                self.persist_sessions();
                self.persist_sand_state();
                self.render_needed = true;
                false
            }
            Action::ClearNoneSand => {
                self.sand_engine.clear_category(CategoryId::new(0));
                self.persist_sand_state();
                self.render_needed = true;
                false
            }
            Action::OpenReportModal => {
                self.open_report_modal();
                false
            }
            Action::OpenCategoryModal => {
                self.open_modal();
                false
            }
            Action::Confirm => {
                if self
                    .keymap
                    .keys_for_action(Action::OpenCategoryModal)
                    .is_empty()
                {
                    self.open_modal();
                }
                false
            }
            Action::SwitchToNone => {
                self.time_tracker.end_session();
                self.persist_sessions();
                let _ = self.time_tracker.set_active_category_by_index(0);
                self.time_tracker.start_session();
                self.none_entry_time = Some(Instant::now());
                self.render_needed = true;
                false
            }
            Action::Cancel => {
                if self.keymap.keys_for_action(Action::SwitchToNone).is_empty() {
                    self.time_tracker.end_session();
                    self.persist_sessions();
                    let _ = self.time_tracker.set_active_category_by_index(0);
                    self.time_tracker.start_session();
                    self.none_entry_time = Some(Instant::now());
                    self.render_needed = true;
                }
                false
            }
            _ => false,
        }
    }
}
