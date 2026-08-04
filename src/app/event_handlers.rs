use crate::{
    constants::COLORS,
    domain::{CategoryId, DRIFT_CATEGORY_ID, ReportPeriod},
    keybindings::{Action, InputContext},
    sqlite,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{
    App, PaletteCommand, PersistenceOperation, QueuedMutation, RecoveryAction, ui_helpers,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReportEditKeyIntent {
    Append(char),
    Backspace,
    Commit,
    Cancel,
    EmergencyQuit,
    Ignore,
}

fn resolve_report_edit_key(
    key: KeyEvent,
    keymap: &crate::keybindings::Keymap,
) -> ReportEditKeyIntent {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return if keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
            ReportEditKeyIntent::EmergencyQuit
        } else {
            ReportEditKeyIntent::Ignore
        };
    }

    match key.code {
        KeyCode::Esc => ReportEditKeyIntent::Cancel,
        KeyCode::Enter => ReportEditKeyIntent::Commit,
        KeyCode::Backspace | KeyCode::Delete => ReportEditKeyIntent::Backspace,
        KeyCode::Char(character) => ReportEditKeyIntent::Append(character),
        _ => ReportEditKeyIntent::Ignore,
    }
}

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Release {
            return false;
        }

        if self.has_persistence_recovery() {
            if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
                return self.request_persistence_recovery_quit();
            }
            return self.handle_persistence_recovery_key(key);
        }

        if self.recovery_statement.is_some() {
            if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
                return true;
            }
            return self.handle_recovery_statement_key(key);
        }

        if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
            return true;
        }

        if self.category_lifecycle_overlay.is_some() {
            return self.handle_category_lifecycle_key(key);
        }

        if self.report_log_edit.is_some() {
            return self.handle_report_log_edit_key(key);
        }

        if self.show_command_palette {
            return self.handle_command_palette_key(key);
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
        if self.in_category_modal()
            && !self.show_keybindings_modal
            && matches!(key.code, KeyCode::Char('?'))
        {
            return None;
        }

        let context = if self.in_karma_modal() {
            InputContext::Report
        } else if self.in_category_modal() || self.show_keybindings_modal {
            InputContext::Other
        } else {
            InputContext::Main
        };
        self.keymap
            .resolve_key_event(context, key)
            .map(|resolved| resolved.action)
    }

    fn route_action(&mut self, action: Action, key: KeyEvent) -> bool {
        if action == Action::ToggleCommandPalette {
            self.toggle_command_palette();
            return false;
        }

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
            return self.handle_report_modal_action(action);
        }

        self.handle_main_action(action)
    }

    fn handle_command_palette_key(&mut self, key: KeyEvent) -> bool {
        if self
            .keymap
            .resolve_key_event(InputContext::Other, key)
            .is_some_and(|resolved| resolved.action == Action::ToggleCommandPalette)
        {
            self.close_command_palette();
            return false;
        }

        let entries = self.filtered_command_palette_entries();
        self.clamp_command_palette_selection(entries.len());

        match key.code {
            KeyCode::Esc => {
                self.close_command_palette();
            }
            KeyCode::Enter => {
                if let Some(entry) = entries.get(self.command_palette_selected_index) {
                    return self.execute_palette_command(entry.command);
                }
                self.close_command_palette();
            }
            KeyCode::Up => {
                if !entries.is_empty() {
                    self.command_palette_selected_index = ui_helpers::wrap_prev_index(
                        self.command_palette_selected_index,
                        entries.len(),
                    );
                    self.render_needed = true;
                }
            }
            KeyCode::Down => {
                if !entries.is_empty() {
                    self.command_palette_selected_index = ui_helpers::wrap_next_index(
                        self.command_palette_selected_index,
                        entries.len(),
                    );
                    self.render_needed = true;
                }
            }
            KeyCode::Home => {
                self.command_palette_selected_index = 0;
                self.command_palette_scroll = 0;
                self.render_needed = true;
            }
            KeyCode::End => {
                if !entries.is_empty() {
                    self.command_palette_selected_index = entries.len() - 1;
                    self.render_needed = true;
                }
            }
            KeyCode::Backspace => {
                self.command_palette_query.pop();
                let updated = self.filtered_command_palette_entries();
                self.clamp_command_palette_selection(updated.len());
                self.render_needed = true;
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.command_palette_query.push(c);
                let updated = self.filtered_command_palette_entries();
                self.clamp_command_palette_selection(updated.len());
                self.render_needed = true;
            }
            _ => {}
        }

        false
    }

    fn execute_palette_command(&mut self, command: PaletteCommand) -> bool {
        self.close_command_palette();

        match command {
            PaletteCommand::Action(Action::ToggleCommandPalette) => false,
            PaletteCommand::Action(Action::ToggleKeybindingsHelp) => {
                self.toggle_keybindings_modal();
                false
            }
            PaletteCommand::Action(action) => self.handle_main_action(action),
            PaletteCommand::SetReportPeriod(period) => {
                if !self.in_karma_modal() {
                    self.open_report_modal();
                }
                self.set_report_period(period);
                self.render_needed = true;
                false
            }
            PaletteCommand::SwitchLayer(category_id) => {
                self.switch_to_layer_from_palette(category_id);
                false
            }
        }
    }

    fn switch_to_layer_from_palette(&mut self, category_id: CategoryId) {
        self.queue_or_apply_mutation(QueuedMutation::SwitchLayer {
            category_id,
            description: String::new(),
        });
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
            KeyCode::Backspace => {
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
            KeyCode::Delete => {
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_action_unbound(&keymap_path, action) {
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
                    if self.has_persistence_recovery() {
                        return true;
                    }
                }
            }
            Action::ShiftDown => {
                if self.time_tracker.move_category_down(self.selected_index) {
                    self.selected_index += 1;
                    self.persist_categories();
                    if self.has_persistence_recovery() {
                        return true;
                    }
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
                } else if self.modal_editing_category_metadata {
                    if self.time_tracker.set_category_description_by_index(
                        self.selected_index,
                        self.modal_description.clone(),
                    ) {
                        self.persist_categories();
                    }
                    if !self.has_persistence_recovery() {
                        self.close_modal();
                    }
                } else {
                    self.remember_selected_tag();
                    if self.has_persistence_recovery() {
                        self.render_needed = true;
                        return true;
                    }
                    let selected = self
                        .time_tracker
                        .category_by_index(self.selected_index)
                        .map(|category| category.id);
                    if let Some(category_id) = selected {
                        if self.time_tracker.active_category_id() == category_id {
                            self.time_tracker
                                .set_active_description(self.modal_description.clone());
                            if let Some(database_path) = self.sqlite_database_path.clone() {
                                let Some(stable_id) = self.session.active_session_stable_id.clone()
                                else {
                                    self.render_needed = true;
                                    return true;
                                };
                                let result = sqlite::update_tui_active_description(
                                    &database_path,
                                    &stable_id,
                                    &self.modal_description,
                                );
                                if self
                                    .record_storage_result_for(
                                        PersistenceOperation::ActiveDescription,
                                        RecoveryAction::ReloadAuthority,
                                        result,
                                    )
                                    .is_none()
                                {
                                    self.render_needed = true;
                                    return true;
                                }
                            }
                            self.refresh_active_runtime_checkpoint();
                        } else {
                            self.queue_or_apply_mutation(QueuedMutation::SwitchLayer {
                                category_id,
                                description: self.modal_description.clone(),
                            });
                        }
                    }
                    self.close_modal();
                }
            }
            Action::EditCategoryDescription => {
                self.toggle_category_metadata_edit();
            }
            Action::DeleteCategory => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.delete_category();
                }
            }
            Action::CategoryLifecycle => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.open_category_lifecycle_for_selected();
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

    fn handle_report_modal_action(&mut self, action: Action) -> bool {
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
                    handled = self.begin_report_log_edit();
                } else if let Some(entry) = summary.entries.get(self.report_selected_index) {
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
            Action::Left => {
                self.shift_report_interval_older();
            }
            Action::Right => {
                self.shift_report_interval_newer();
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
            Action::DeleteCategory => {
                if in_logs_view {
                    handled = self.delete_selected_report_session();
                } else {
                    handled = false;
                }
            }
            Action::Quit => return true,
            _ => handled = false,
        }

        if handled {
            self.render_needed = true;
        }
        false
    }

    fn handle_main_action(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => true,
            Action::ClearAllSand => {
                self.queue_or_apply_mutation(QueuedMutation::ClearAllSand);
                false
            }
            Action::ClearNoneSand => {
                self.queue_or_apply_mutation(QueuedMutation::ClearDriftSand);
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
            Action::CategoryLifecycle => {
                self.open_category_lifecycle_for_active();
                false
            }
            Action::Confirm => false,
            Action::SwitchToNone => {
                self.queue_or_apply_mutation(QueuedMutation::SwitchLayer {
                    category_id: DRIFT_CATEGORY_ID,
                    description: String::new(),
                });
                false
            }
            Action::Detach => {
                self.detach_requested = true;
                true
            }
            Action::Cancel => false,
            Action::ReportToday => {
                self.open_report_modal();
                self.set_report_period(ReportPeriod::Today);
                false
            }
            _ => false,
        }
    }

    fn handle_report_log_edit_key(&mut self, key: KeyEvent) -> bool {
        match resolve_report_edit_key(key, &self.keymap) {
            ReportEditKeyIntent::Append(character) => {
                if let Some(edit) = self.report_log_edit.as_mut() {
                    edit.draft.push(character);
                    self.render_needed = true;
                }
            }
            ReportEditKeyIntent::Backspace => {
                if let Some(edit) = self.report_log_edit.as_mut() {
                    edit.draft.pop();
                    self.render_needed = true;
                }
            }
            ReportEditKeyIntent::Commit => {
                self.commit_report_log_edit();
            }
            ReportEditKeyIntent::Cancel => {
                self.cancel_report_log_edit();
            }
            ReportEditKeyIntent::EmergencyQuit => return true,
            ReportEditKeyIntent::Ignore => {}
        }
        false
    }
}

#[cfg(test)]
mod report_edit_tests {
    use super::{ReportEditKeyIntent, resolve_report_edit_key};
    use crate::keybindings::default_keymap;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn plain_command_letters_are_text_only_in_edit_mode() {
        let keymap = default_keymap();
        for character in ['q', 'w', 'm', 't', 'k', 'd', 'x'] {
            assert_eq!(
                resolve_report_edit_key(
                    KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                    &keymap,
                ),
                ReportEditKeyIntent::Append(character)
            );
        }
    }

    #[test]
    fn unicode_and_spaces_are_supported() {
        let keymap = default_keymap();
        assert_eq!(
            resolve_report_edit_key(
                KeyEvent::new(KeyCode::Char('界'), KeyModifiers::NONE),
                &keymap,
            ),
            ReportEditKeyIntent::Append('界')
        );
        assert_eq!(
            resolve_report_edit_key(
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
                &keymap,
            ),
            ReportEditKeyIntent::Append(' ')
        );
    }

    #[test]
    fn enter_commits_and_escape_cancels() {
        let keymap = default_keymap();
        assert_eq!(
            resolve_report_edit_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &keymap),
            ReportEditKeyIntent::Commit
        );
        assert_eq!(
            resolve_report_edit_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &keymap),
            ReportEditKeyIntent::Cancel
        );
    }

    #[test]
    fn configured_modified_quit_is_the_only_emergency_action() {
        let keymap = default_keymap();
        assert_eq!(
            resolve_report_edit_key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &keymap,
            ),
            ReportEditKeyIntent::EmergencyQuit
        );
        assert_eq!(
            resolve_report_edit_key(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
                &keymap,
            ),
            ReportEditKeyIntent::Ignore
        );
    }
}
