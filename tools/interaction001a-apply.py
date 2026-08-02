from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


# Stable draft state belongs to the application, not the selected row's current position.
app = Path("src/app.rs")
text = app.read_text()
anchor = '''#[derive(Clone, Debug)]
struct PaletteEntry {
    command: PaletteCommand,
    title: String,
    search_text: String,
    hint: String,
}
'''
addition = anchor + '''
#[derive(Clone, Debug, PartialEq, Eq)]
struct ReportLogEditState {
    session_id: usize,
    draft: String,
}
'''
if text.count(anchor) != 1:
    raise SystemExit("palette entry anchor not found")
text = text.replace(anchor, addition, 1)
text = text.replace(
    "    report_log_selected_index: usize,\n    report_snapshot_end_day: Option<String>,",
    "    report_log_selected_index: usize,\n    report_log_edit: Option<ReportLogEditState>,\n    report_snapshot_end_day: Option<String>,",
    1,
)
text = text.replace(
    "            report_log_selected_index: 0,\n            report_snapshot_end_day: None,",
    "            report_log_selected_index: 0,\n            report_log_edit: None,\n            report_snapshot_end_day: None,",
    1,
)
text = text.replace(
    "        self.report_log_selected_index = 0;\n        self.report_snapshot_end_day = None;",
    "        self.report_log_selected_index = 0;\n        self.report_log_edit = None;\n        self.report_snapshot_end_day = None;",
    2,
)
app.write_text(text)

# Explicit input intent: plain characters are draft text only in edit mode; modified Quit remains deliberate.
events = Path("src/app/event_handlers.rs")
text = events.read_text()
text = text.replace(
    "use super::{App, PaletteCommand, QueuedMutation, ui_helpers};",
    "use super::{App, PaletteCommand, QueuedMutation, ui_helpers};",
    1,
)
impl_anchor = "impl App {\n"
intent = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReportEditKeyIntent {
    Append(char),
    Backspace,
    Commit,
    Cancel,
    EmergencyQuit,
    Ignore,
}

fn resolve_report_edit_key(key: KeyEvent, keymap: &crate::keybindings::Keymap) -> ReportEditKeyIntent {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return if keymap.action_for_key_event(key) == Some(Action::Quit) {
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

'''
if text.count(impl_anchor) != 1:
    raise SystemExit("event handler impl anchor not found")
text = text.replace(impl_anchor, intent + impl_anchor, 1)
old_dispatch = '''        if self.show_command_palette {
            return self.handle_command_palette_key(key);
        }

        if self.show_keybindings_modal && self.atlas_overlay.is_some() {
            return self.handle_atlas_overlay_key(key);
        }

        if self.in_karma_modal() && self.report_logs_category_id.is_some() {
            let is_text_like = matches!(
                key.code,
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
            ) && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

            if is_text_like && self.keymap.action_for_key_event(key) != Some(Action::DeleteCategory)
            {
                self.handle_report_logs_text_input(key);
                return false;
            }
        }
'''
new_dispatch = '''        if self.report_log_edit.is_some() {
            return self.handle_report_log_edit_key(key);
        }

        if self.show_command_palette {
            return self.handle_command_palette_key(key);
        }

        if self.show_keybindings_modal && self.atlas_overlay.is_some() {
            return self.handle_atlas_overlay_key(key);
        }
'''
if text.count(old_dispatch) != 1:
    raise SystemExit("implicit report text dispatch block not found")
text = text.replace(old_dispatch, new_dispatch, 1)
text = text.replace(
    "        if self.in_karma_modal() {\n            self.handle_report_modal_action(action);\n            return false;\n        }",
    "        if self.in_karma_modal() {\n            return self.handle_report_modal_action(action);\n        }",
    1,
)
# Convert report action routing to an exit decision and use Confirm to enter editing.
text = text.replace(
    "    fn handle_report_modal_action(&mut self, action: Action) {",
    "    fn handle_report_modal_action(&mut self, action: Action) -> bool {",
    1,
)
text = text.replace(
    "            Action::Confirm => {\n                if in_logs_view {\n                    self.report_logs_category_id = None;\n                    self.report_log_selected_index = 0;\n                } else if let Some(entry) = summary.entries.get(self.report_selected_index) {",
    "            Action::Confirm => {\n                if in_logs_view {\n                    handled = self.begin_report_log_edit();\n                } else if let Some(entry) = summary.entries.get(self.report_selected_index) {",
    1,
)
old_backspace = '''            Action::Backspace => {
                if in_logs_view {
                    handled = self.backspace_selected_report_session_tag();
                } else {
                    handled = false;
                }
            }
            _ => handled = false,
'''
new_backspace = '''            Action::Quit => return true,
            _ => handled = false,
'''
if text.count(old_backspace) != 1:
    raise SystemExit("report backspace action block not found")
text = text.replace(old_backspace, new_backspace, 1)
text = text.replace(
    "        if handled {\n            self.render_needed = true;\n        }\n    }\n\n    fn handle_main_action",
    "        if handled {\n            self.render_needed = true;\n        }\n        false\n    }\n\n    fn handle_main_action",
    1,
)
# Replace implicit per-character persistence with explicit draft handling.
replace_start = "    fn handle_report_logs_text_input(&mut self, key: KeyEvent) {\n"
start = text.index(replace_start)
end = text.index("\n}\n", start)
replacement = '''    fn handle_report_log_edit_key(&mut self, key: KeyEvent) -> bool {
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
'''
text = text[:start] + replacement + text[end:]
# Add focused input-policy tests.
text += r'''

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
'''
events.write_text(text)

# Draft lifecycle and one-write commit boundary.
report = Path("src/app/report_state.rs")
text = report.read_text()
start = text.index("    pub(super) fn append_to_selected_report_session_tag")
end = text.index("    fn sync_report_selection_for_interval", start)
replacement = '''    pub(super) fn begin_report_log_edit(&mut self) -> bool {
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

        if let Some(database_path) = self.sqlite_database_path.clone() {
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
        } else {
            let mut sessions = self.time_tracker.sessions.clone();
            let Some(session) = sessions
                .iter_mut()
                .find(|session| session.id == edit.session_id)
            else {
                return false;
            };
            session.description = edit.draft.clone();
            let categories = self.time_tracker.categories_for_storage();
            let result = crate::storage::save_sessions_to_csv(
                &crate::storage::get_time_log_path(),
                &sessions,
                &categories,
            )
            .map_err(|error| error.to_string());
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
            self.time_tracker.sessions = sessions;
        }

        retain_report_edit_after_commit(&mut self.report_log_edit, true);
        self.render_needed = true;
        true
    }

'''
text = text[:start] + replacement + text[end:]
# Pure state transition proves failed commit retains full draft.
text += r'''

fn retain_report_edit_after_commit(
    edit: &mut Option<super::ReportLogEditState>,
    committed: bool,
) {
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
'''
report.write_text(text)

# Visible mode, draft row, and cursor marker.
modal = Path("src/app/report_modal_view.rs")
text = modal.read_text()
old_snapshot_title = '''        let snapshot_bottom_title = Line::from(Span::styled(
            self.report_snapshot_status_label(),
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Left);

        let frame_block = Block::default()
            .title(interval_title)
            .title(center_title)
            .title(total_title)
            .title_bottom(snapshot_bottom_title)
            .title_bottom(period_bottom_title)
'''
new_snapshot_title = '''        let snapshot_bottom_title = Line::from(Span::styled(
            self.report_snapshot_status_label(),
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Left);
        let interaction_bottom_title = if self.report_logs_category_id.is_some() {
            let label = if self.report_log_edit.is_some() {
                "EDIT DESCRIPTION · Enter commit · Esc cancel"
            } else {
                "VIEW · Enter edit · Esc back"
            };
            Some(
                Line::from(Span::styled(label, Style::default().fg(Color::Gray)))
                    .alignment(Alignment::Right),
            )
        } else {
            None
        };

        let mut frame_block = Block::default()
            .title(interval_title)
            .title(center_title)
            .title(total_title)
            .title_bottom(snapshot_bottom_title)
            .title_bottom(period_bottom_title)
'''
if text.count(old_snapshot_title) != 1:
    raise SystemExit("report frame title block not found")
text = text.replace(old_snapshot_title, new_snapshot_title, 1)
old_frame_end = '''            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        f.render_widget(ratatui::widgets::Clear, modal_rect);
'''
new_frame_end = '''            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        if let Some(interaction_bottom_title) = interaction_bottom_title {
            frame_block = frame_block.title_bottom(interaction_bottom_title);
        }

        f.render_widget(ratatui::widgets::Clear, modal_rect);
'''
if text.count(old_frame_end) != 1:
    raise SystemExit("report frame completion block not found")
text = text.replace(old_frame_end, new_frame_end, 1)
old_tag = '''                let tag = self.truncate_label(row.description.trim(), tag_width);
                let tag_cell = format!("{tag:<width$}", width = tag_width);
'''
new_tag = '''                let displayed_description = self
                    .report_log_edit
                    .as_ref()
                    .filter(|edit| row.session_id == Some(edit.session_id))
                    .map(|edit| format!("{}▏", edit.draft))
                    .unwrap_or_else(|| row.description.trim().to_string());
                let tag = self.truncate_label(&displayed_description, tag_width);
                let tag_cell = format!("{tag:<width$}", width = tag_width);
'''
if text.count(old_tag) != 1:
    raise SystemExit("report log tag block not found")
modal.write_text(text.replace(old_tag, new_tag, 1))

for temporary in [
    ".github/workflows/interaction001a-apply.yml",
    "tools/interaction001a-apply.py",
    "tools/interaction001a.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
