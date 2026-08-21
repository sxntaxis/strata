use crate::{
    command::{self, CommandIntent},
    constants::COLORS,
    domain::{Category, CategoryId, DRIFT_CATEGORY_ID, ReportPeriod, is_drift_category_id},
    keybindings::{Action, InputContext},
    sqlite,
};
use chrono::{Duration as ChronoDuration, Local, NaiveDate};
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

fn direct_command_or_fuzzy_fallback(
    query: &str,
    has_fuzzy_result: bool,
) -> Result<Option<CommandIntent>, String> {
    let typed = query.trim();
    if typed.is_empty() {
        return Ok(None);
    }
    match crate::command::parse(typed) {
        Ok(command) => Ok(Some(command)),
        Err(_) if has_fuzzy_result => Ok(None),
        Err(error) => Err(error),
    }
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
            KeyCode::Esc => self.close_command_palette(),
            KeyCode::Enter => {
                match direct_command_or_fuzzy_fallback(
                    &self.command_palette_query,
                    !entries.is_empty(),
                ) {
                    Ok(Some(command)) => {
                        let keep_open = command.keeps_palette_open();
                        match self.execute_command(command) {
                            Ok(message) if keep_open => {
                                self.command_palette_feedback = Some(message);
                                self.render_needed = true;
                            }
                            Ok(_) => self.close_command_palette(),
                            Err(error) => {
                                self.command_palette_feedback = Some(format!("Error: {error}"));
                                self.render_needed = true;
                            }
                        }
                        return false;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        self.command_palette_feedback = Some(format!("Error: {error}"));
                        self.render_needed = true;
                        return false;
                    }
                }
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
                self.command_palette_feedback = None;
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
                self.command_palette_feedback = None;
                let updated = self.filtered_command_palette_entries();
                self.clamp_command_palette_selection(updated.len());
                self.render_needed = true;
            }
            _ => {}
        }

        false
    }

    fn resolve_layer_case_insensitive(&self, layer: &str) -> Option<Category> {
        let trimmed = layer.trim();
        if trimmed.is_empty() {
            return None;
        }
        let categories = self.time_tracker.categories_ordered();
        if let Ok(id) = trimmed.parse::<u64>()
            && let Some(found) = categories.iter().find(|category| category.id.0 == id)
        {
            return Some(found.clone());
        }
        categories
            .into_iter()
            .find(|category| category.name.eq_ignore_ascii_case(trimmed))
    }

    fn layer_suggestions(&self, layer: &str) -> Vec<String> {
        let needle = layer.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        self.time_tracker
            .categories_ordered()
            .into_iter()
            .map(|category| category.name)
            .filter(|name| name.to_ascii_lowercase().contains(&needle))
            .take(3)
            .collect()
    }

    fn canonicalize_tag_for_layer(&self, layer_id: CategoryId, tag: &str) -> String {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        self.category_tags
            .tags_by_category
            .get(&layer_id.0)
            .and_then(|tags| {
                tags.iter()
                    .find(|existing| existing.eq_ignore_ascii_case(trimmed))
            })
            .cloned()
            .unwrap_or_else(|| trimmed.to_string())
    }

    fn remember_tag_for_layer(&mut self, layer_id: CategoryId, tag: &str) {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            return;
        }
        let tags = self
            .category_tags
            .tags_by_category
            .entry(layer_id.0)
            .or_default();
        tags.retain(|existing| !existing.eq_ignore_ascii_case(trimmed));
        tags.insert(0, trimmed.to_string());
        tags.truncate(crate::constants::CATEGORY_SETTINGS.max_tags_per_category);
        self.persist_category_tags();
    }

    pub(super) fn execute_command(&mut self, command: CommandIntent) -> Result<String, String> {
        match command {
            CommandIntent::Status => self.command_status(),
            CommandIntent::Start { layer, tag } => self.command_start(layer, tag),
            CommandIntent::Stop { layer, tag } => self.command_stop(layer, tag),
            CommandIntent::Karma {
                selector,
                layer,
                tag,
            } => self.command_karma(selector, layer, tag),
            CommandIntent::DeleteLastSession { layer, tag } => {
                self.command_delete_last_session(layer, tag)
            }
            CommandIntent::DataDir => Ok(format!(
                "Data dir: {}",
                crate::profile::data_dir().display()
            )),
            CommandIntent::ConfigDir => Ok(format!(
                "Config dir: {}",
                crate::profile::config_dir().display()
            )),
            CommandIntent::Timer { duration_seconds } => {
                let end = Local::now()
                    + ChronoDuration::seconds(i64::try_from(duration_seconds).unwrap_or(i64::MAX));
                Ok(format!(
                    "Timer {} (ends {})",
                    command::format_hms(duration_seconds as usize),
                    end.format("%Y-%m-%d %H:%M:%S")
                ))
            }
            #[cfg(debug_assertions)]
            CommandIntent::TestingCheatsHalfFull => {
                Err("testingcheats is not available in the SQLite runtime".to_string())
            }
        }
    }

    fn command_status(&self) -> Result<String, String> {
        let active_id = self.time_tracker.active_category_id();
        let category_name = self
            .time_tracker
            .category_by_id(active_id)
            .map(|category| self.display_layer_name(&category.name))
            .unwrap_or_else(|| "Idle".to_string());
        let elapsed = self
            .time_tracker
            .current_session_start
            .map(|start| start.elapsed().as_secs() as usize)
            .unwrap_or(0);
        if is_drift_category_id(active_id) {
            return Ok(format!("Status: idle for {}", command::format_hms(elapsed)));
        }
        let description = self.time_tracker.active_description().trim();
        let started = self
            .session
            .active_session_started_at_utc
            .map(|started| {
                started
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());
        if description.is_empty() {
            Ok(format!(
                "Status: active layer '{}' since {} (elapsed {})",
                category_name,
                started,
                command::format_hms(elapsed)
            ))
        } else {
            Ok(format!(
                "Status: active layer '{}' tag '{}' since {} (elapsed {})",
                category_name,
                description,
                started,
                command::format_hms(elapsed)
            ))
        }
    }

    fn command_start(&mut self, layer: String, tag: Option<String>) -> Result<String, String> {
        let Some(category) = self.resolve_layer_case_insensitive(&layer) else {
            let suggestions = self.layer_suggestions(&layer);
            return if suggestions.is_empty() {
                Err(format!("Layer '{layer}' not found"))
            } else {
                Err(format!(
                    "Layer '{layer}' not found. Did you mean: {}",
                    suggestions.join(", ")
                ))
            };
        };
        let canonical_tag = tag
            .as_deref()
            .map(|value| self.canonicalize_tag_for_layer(category.id, value))
            .filter(|value| !value.is_empty());
        if let Some(value) = canonical_tag.as_deref() {
            self.remember_tag_for_layer(category.id, value);
        }
        let description = canonical_tag.clone().unwrap_or_default();
        if self.time_tracker.active_category_id() == category.id {
            if !description.is_empty() {
                let database_path = self
                    .sqlite_database_path
                    .clone()
                    .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
                let stable_id = self
                    .session
                    .active_session_stable_id
                    .clone()
                    .ok_or_else(|| "active session has no stable identity".to_string())?;
                sqlite::update_tui_active_description(&database_path, &stable_id, &description)?;
                self.time_tracker
                    .set_active_description(description.clone());
                self.refresh_active_runtime_checkpoint();
            }
        } else {
            self.queue_or_apply_mutation(QueuedMutation::SwitchLayer {
                category_id: category.id,
                description,
            });
        }
        let display_name = self.display_layer_name(&category.name);
        Ok(match canonical_tag {
            Some(tag) => format!("Started layer '{display_name}' with tag '{tag}'"),
            None => format!("Started layer '{display_name}'"),
        })
    }

    fn command_stop(
        &mut self,
        layer_filter: Option<String>,
        tag_filter: Option<String>,
    ) -> Result<String, String> {
        let active_id = self.time_tracker.active_category_id();
        if is_drift_category_id(active_id) {
            return Err("No active layer session to stop (already idle)".to_string());
        }
        if let Some(layer) = layer_filter {
            let Some(expected) = self.resolve_layer_case_insensitive(&layer) else {
                return Err(format!("Layer '{layer}' not found"));
            };
            if expected.id != active_id {
                let active_name = self
                    .time_tracker
                    .category_by_id(active_id)
                    .map(|category| self.display_layer_name(&category.name))
                    .unwrap_or_else(|| "Idle".to_string());
                return Err(format!(
                    "Active layer is '{}' (not '{}')",
                    active_name,
                    self.display_layer_name(&expected.name)
                ));
            }
        }
        if let Some(tag) = tag_filter {
            let active_tag = self.time_tracker.active_description();
            if !active_tag.eq_ignore_ascii_case(tag.trim()) {
                return Err(format!(
                    "Active tag is '{}' (not '{}')",
                    active_tag,
                    tag.trim()
                ));
            }
        }
        let active_name = self
            .time_tracker
            .category_by_id(active_id)
            .map(|category| self.display_layer_name(&category.name))
            .unwrap_or_else(|| "Idle".to_string());
        self.queue_or_apply_mutation(QueuedMutation::SwitchLayer {
            category_id: DRIFT_CATEGORY_ID,
            description: String::new(),
        });
        Ok(format!("Stopped layer '{active_name}'"))
    }

    fn command_karma(
        &self,
        selector: command::KarmaSelector,
        layer_filter: Option<String>,
        tag_filter: Option<String>,
    ) -> Result<String, String> {
        let window = command::resolve_karma_window(
            &selector,
            crate::domain::operational_day_key_now(),
            self.runtime_settings.first_day_of_week,
        )?;
        let categories = self.time_tracker.categories_ordered();
        let layer = if let Some(layer) = layer_filter {
            Some(
                self.resolve_layer_case_insensitive(&layer)
                    .ok_or_else(|| format!("Layer '{layer}' not found"))?,
            )
        } else {
            None
        };
        let layer_id = layer.as_ref().map(|category| category.id);
        let canonical_tag = tag_filter
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                layer
                    .as_ref()
                    .map(|category| self.canonicalize_tag_for_layer(category.id, value))
                    .unwrap_or_else(|| value.to_string())
            });
        let mut total_elapsed = 0usize;
        let mut total_karma = 0isize;
        for session in &self.time_tracker.sessions {
            let Ok(day) = NaiveDate::parse_from_str(&session.date, "%Y-%m-%d") else {
                continue;
            };
            if day < window.start || day > window.end {
                continue;
            }
            if layer_id.is_some_and(|expected| session.category_id != expected) {
                continue;
            }
            if canonical_tag
                .as_ref()
                .is_some_and(|tag| !session.description.eq_ignore_ascii_case(tag))
            {
                continue;
            }
            let effect = categories
                .iter()
                .find(|category| category.id == session.category_id)
                .map(|category| {
                    if is_drift_category_id(category.id) {
                        0
                    } else {
                        category.karma_effect
                    }
                })
                .unwrap_or(0);
            total_elapsed = total_elapsed.saturating_add(session.elapsed_seconds);
            total_karma = total_karma
                .saturating_add((session.elapsed_seconds as isize).saturating_mul(effect as isize));
        }
        if let Some(start) = self.time_tracker.current_session_start {
            let active_id = self.time_tracker.active_category_id();
            let live_day = crate::domain::operational_day_key_now();
            let layer_matches = layer_id.is_none_or(|expected| expected == active_id);
            let tag_matches = canonical_tag.as_ref().is_none_or(|tag| {
                self.time_tracker
                    .active_description()
                    .eq_ignore_ascii_case(tag)
            });
            if live_day >= window.start && live_day <= window.end && layer_matches && tag_matches {
                let elapsed = start.elapsed().as_secs() as usize;
                let effect = categories
                    .iter()
                    .find(|category| category.id == active_id)
                    .map(|category| {
                        if is_drift_category_id(category.id) {
                            0
                        } else {
                            category.karma_effect
                        }
                    })
                    .unwrap_or(0);
                total_elapsed = total_elapsed.saturating_add(elapsed);
                total_karma =
                    total_karma.saturating_add((elapsed as isize).saturating_mul(effect as isize));
            }
        }
        let mut scope = String::new();
        if let Some(category) = layer {
            scope.push_str(&format!(
                " layer '{}'",
                self.display_layer_name(&category.name)
            ));
        }
        if let Some(tag) = canonical_tag {
            scope.push_str(&format!(" tag '{tag}'"));
        }
        Ok(format!(
            "Karma {}{}: {} (elapsed {})",
            window.label,
            scope,
            command::format_signed_hms(total_karma),
            command::format_hms(total_elapsed)
        ))
    }

    fn command_delete_last_session(
        &mut self,
        layer: String,
        tag: Option<String>,
    ) -> Result<String, String> {
        let category = self
            .resolve_layer_case_insensitive(&layer)
            .ok_or_else(|| format!("Layer '{layer}' not found"))?;
        let canonical_tag = tag
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| self.canonicalize_tag_for_layer(category.id, value));
        let session_id = self
            .time_tracker
            .sessions
            .iter()
            .filter(|session| session.category_id == category.id)
            .filter(|session| {
                canonical_tag
                    .as_ref()
                    .is_none_or(|tag| session.description.eq_ignore_ascii_case(tag))
            })
            .max_by_key(|session| session.id)
            .map(|session| session.id)
            .ok_or_else(|| "No matching session found".to_string())?;
        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
        sqlite::delete_tui_session(&database_path, session_id)?;
        self.reload_sqlite_sessions();
        Ok(format!(
            "Deleted last session for layer '{}'",
            self.display_layer_name(&category.name)
        ))
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
    use super::{ReportEditKeyIntent, direct_command_or_fuzzy_fallback, resolve_report_edit_key};
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
    #[test]
    fn fuzzy_palette_query_falls_back_when_it_is_not_a_direct_command() {
        assert_eq!(
            direct_command_or_fuzzy_fallback("report", true).unwrap(),
            None
        );
    }

    #[test]
    fn valid_direct_command_wins_over_fuzzy_results() {
        let resolved = direct_command_or_fuzzy_fallback("status", true)
            .unwrap()
            .expect("status should resolve as a direct command");
        assert_eq!(resolved, crate::command::CommandIntent::Status);
    }
}
