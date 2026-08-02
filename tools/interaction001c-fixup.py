from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


# Mandatory Quit must use the persistence-recovery custody path when recovery is active.
replace_once(
    "src/app/event_handlers.rs",
    '''        if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
            return true;
        }

        if self.has_persistence_recovery() {
            return self.handle_persistence_recovery_key(key);
        }
''',
    '''        if self.has_persistence_recovery() {
            if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
                return self.request_persistence_recovery_quit();
            }
            return self.handle_persistence_recovery_key(key);
        }

        if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
            return true;
        }
''',
)

replace_once(
    "src/app/persistence_recovery.rs",
    '''    pub(super) fn handle_persistence_recovery_key(&mut self, key: KeyEvent) -> bool {
''',
    '''    pub(super) fn request_persistence_recovery_quit(&mut self) -> bool {
        self.export_current_recovery(true);
        self.recovery_exit_requested
    }

    pub(super) fn handle_persistence_recovery_key(&mut self, key: KeyEvent) -> bool {
''',
)
replace_once(
    "src/app/persistence_recovery.rs",
    '''            KeyCode::Char('q' | 'Q') => {
                self.export_current_recovery(true);
                self.recovery_exit_requested
            }
''',
    '''            KeyCode::Char('q' | 'Q') => self.request_persistence_recovery_quit(),
''',
)

# Contradictory manual JSON must fail closed rather than silently letting a binding
# erase an explicit Disabled marker.
replace_once(
    "src/keybindings.rs",
    '''                overridden_actions.insert(action);
                disabled_actions.remove(&action);
                Some(action)
''',
    '''                if disabled_actions.contains(&action) {
                    return Err(format!(
                        "Action '{}' in {} is both bound and disabled",
                        action.config_name(),
                        path.display()
                    ));
                }
                overridden_actions.insert(action);
                Some(action)
''',
)
replace_once(
    "src/keybindings.rs",
    '''    #[test]
    fn atlas_writer_rejects_mandatory_ctrl_c_before_persisting() {
''',
    '''    #[test]
    fn contradictory_bound_and_disabled_action_is_rejected() {
        let path = unique_path("strata_keymap_contradictory_state");
        fs::write(
            &path,
            r#"{
              "keymap":{"ctrl-r":"open_karma_popup"},
              "unbind_actions":["open_karma_popup"]
            }"#,
        )
        .unwrap();
        let error = load_keymap_for_test(&path).unwrap_err();
        assert!(error.contains("both bound and disabled"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn atlas_writer_rejects_mandatory_ctrl_c_before_persisting() {
''',
)

# The atlas footer is derived from the same configured bindings as runtime.
replace_once(
    "src/app/keybindings_modal_view.rs",
    '''        let bottom_description = self.atlas_item_description(selected_item);

        let modal_rect = self.modal_rect_ratio(terminal_size, 5, 6);
''',
    '''        let bottom_description = self.atlas_item_description(selected_item);
        let close_hint = self.atlas_control_hint(
            &[Action::Cancel, Action::ToggleKeybindingsHelp],
            "close",
        );
        let movement_hint = self.atlas_control_hint(
            &[Action::Up, Action::Down, Action::Left, Action::Right],
            "move",
        );
        let jump_hint =
            self.atlas_control_hint(&[Action::HelpTop, Action::HelpBottom], "jump");

        let modal_rect = self.modal_rect_ratio(terminal_size, 5, 6);
''',
)
replace_once(
    "src/app/keybindings_modal_view.rs",
    '''        let bottom_left = Line::from(Span::styled(
            "Esc/F1/? close",
            Style::default().fg(Color::DarkGray),
        ))
''',
    '''        let bottom_left = Line::from(Span::styled(
            close_hint,
            Style::default().fg(Color::DarkGray),
        ))
''',
)
replace_once(
    "src/app/keybindings_modal_view.rs",
    '''        let bottom_right = Line::from(Span::styled(
            "↑↓ move · Home/End jump",
            Style::default().fg(Color::DarkGray),
        ))
''',
    '''        let bottom_right = Line::from(Span::styled(
            format!("{movement_hint} · {jump_hint}"),
            Style::default().fg(Color::DarkGray),
        ))
''',
)
replace_once(
    "src/app/keybindings_modal_view.rs",
    '''    fn command_atlas_rows(&self, selected_item: AtlasSelectable) -> Vec<AtlasRow> {
''',
    '''    fn atlas_control_hint(&self, actions: &[Action], label: &str) -> String {
        format_atlas_control_hint(
            actions
                .iter()
                .flat_map(|action| self.effective_keys_for_action(*action))
                .map(|key| key.to_string()),
            label,
        )
    }

    fn command_atlas_rows(&self, selected_item: AtlasSelectable) -> Vec<AtlasRow> {
''',
)
replace_once(
    "src/app/keybindings_modal_view.rs",
    '''                        "Esc: cancel · Backspace/Delete: unbind",
''',
    '''                        "Esc: cancel · Backspace: disable · Delete: unbind",
''',
)
replace_once(
    "src/app/keybindings_modal_view.rs",
    '''fn week_day_label(day: FirstDayOfWeek) -> &'static str {
''',
    '''fn format_atlas_control_hint(
    keys: impl IntoIterator<Item = String>,
    label: &str,
) -> String {
    let mut unique = Vec::new();
    for key in keys {
        if !unique.contains(&key) {
            unique.push(key);
        }
    }

    if unique.is_empty() {
        format!("(unbound) {label}")
    } else {
        format!("{} {label}", unique.join("/"))
    }
}

fn week_day_label(day: FirstDayOfWeek) -> &'static str {
''',
)
path = Path("src/app/keybindings_modal_view.rs")
text = path.read_text()
text += r'''

#[cfg(test)]
mod tests {
    use super::format_atlas_control_hint;

    #[test]
    fn atlas_control_hint_preserves_runtime_key_order_and_deduplicates() {
        let hint = format_atlas_control_hint(
            ["Esc", "F1", "?", "F1"].into_iter().map(str::to_string),
            "close",
        );
        assert_eq!(hint, "Esc/F1/? close");
    }

    #[test]
    fn atlas_control_hint_exposes_unreachable_control_groups() {
        assert_eq!(
            format_atlas_control_hint(std::iter::empty(), "jump"),
            "(unbound) jump"
        );
    }
}
'''
path.write_text(text)

for temporary in [
    ".github/workflows/interaction001c-fixup.yml",
    "tools/interaction001c-fixup.py",
]:
    Path(temporary).unlink(missing_ok=True)
