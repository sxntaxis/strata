use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{
    constants::SETTINGS_LAYOUT,
    domain::FirstDayOfWeek,
    keybindings::{Action, ActionBindingState, ActionCategory},
};

use super::{App, SettingsOverlay, SettingsSelectable, view_style};

#[derive(Clone)]
struct SettingsRow {
    selectable: Option<SettingsSelectable>,
    line: Line<'static>,
}

impl App {
    pub(super) fn render_settings(&mut self, f: &mut Frame, terminal_size: Rect) {
        let selected_item = self.selected_settings_item();
        let border_color = self.settings_item_color(selected_item);
        let bottom_description = self.settings_item_description(selected_item);
        let close_hint =
            self.settings_control_hint(&[Action::Cancel, Action::ToggleSettings], "close");
        let movement_hint = self.settings_control_hint(
            &[Action::Up, Action::Down, Action::Left, Action::Right],
            "move",
        );
        let jump_hint =
            self.settings_control_hint(&[Action::SettingsTop, Action::SettingsBottom], "jump");

        let modal_rect = self.modal_rect_ratio(terminal_size, 5, 6);
        let title = Line::from(Span::styled(
            "settings",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);

        let bottom_left = Line::from(Span::styled(
            close_hint,
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Left);
        let bottom_center = Line::from(Span::styled(
            bottom_description,
            Style::default().fg(Color::Gray),
        ))
        .alignment(Alignment::Center);
        let bottom_right = Line::from(Span::styled(
            format!("{movement_hint} · {jump_hint}"),
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Right);

        let frame_block = Block::default()
            .title(title)
            .title_bottom(bottom_left)
            .title_bottom(bottom_center)
            .title_bottom(bottom_right)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_widget(frame_block.clone(), modal_rect);

        let inner = frame_block.inner(modal_rect);
        let has_error = self.keymap_error.is_some();
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints(if has_error {
                vec![Constraint::Length(1), Constraint::Min(1)]
            } else {
                vec![Constraint::Min(1)]
            })
            .split(inner);

        let body_rect = if has_error {
            if let Some(err) = self.keymap_error.as_ref() {
                let error_line = Line::from(vec![
                    Span::styled(
                        "config error: ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(err.to_string(), Style::default().fg(Color::Gray)),
                ]);
                f.render_widget(Paragraph::new(vec![error_line]), vertical[0]);
            }
            vertical[1]
        } else {
            vertical[0]
        };

        let rows = self.settings_rows(selected_item);
        let selected_row = rows
            .iter()
            .position(|row| row.selectable == Some(selected_item))
            .unwrap_or(0);

        let viewport_rows = body_rect.height as usize;
        if self.settings_scroll > selected_row {
            self.settings_scroll = selected_row;
        } else if selected_row >= self.settings_scroll.saturating_add(viewport_rows)
            && viewport_rows > 0
        {
            self.settings_scroll = selected_row + 1 - viewport_rows;
        }
        let max_scroll = rows.len().saturating_sub(viewport_rows);
        self.settings_scroll = self.settings_scroll.min(max_scroll);

        let body_lines: Vec<Line<'static>> = rows.into_iter().map(|row| row.line).collect();
        let body = Paragraph::new(body_lines).scroll((self.settings_scroll as u16, 0));
        f.render_widget(body, body_rect);

        if let Some(overlay) = self.settings_overlay.as_ref() {
            self.render_settings_overlay(f, terminal_size, overlay);
        }
    }

    fn settings_control_hint(&self, actions: &[Action], label: &str) -> String {
        format_settings_control_hint(
            actions
                .iter()
                .flat_map(|action| self.effective_keys_for_action(*action))
                .map(|key| key.to_string()),
            label,
        )
    }

    fn settings_rows(&self, selected_item: SettingsSelectable) -> Vec<SettingsRow> {
        let value_col = SETTINGS_LAYOUT.value_col_width;
        let action_col = SETTINGS_LAYOUT.action_col_width;

        let mut rows = vec![
            SettingsRow {
                selectable: None,
                line: Line::from(vec![
                    Span::styled(
                        pad_column("binding / value", value_col),
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "action",
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            },
            SettingsRow {
                selectable: None,
                line: Line::from(Span::styled(
                    "General",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
            },
            self.selectable_row(
                SettingsSelectable::WeekStartDay,
                selected_item,
                self.settings_item_color(SettingsSelectable::WeekStartDay),
                pad_column(&self.first_day_of_week_label(), value_col),
                "First day of week".to_string(),
                action_col,
            ),
            SettingsRow {
                selectable: None,
                line: Line::from(""),
            },
        ];

        for category in ActionCategory::all() {
            let section_color = self.settings_item_color(SettingsSelectable::Action(
                Action::all()
                    .iter()
                    .copied()
                    .find(|action| action.category() == category)
                    .unwrap_or(Action::Quit),
            ));

            rows.push(SettingsRow {
                selectable: None,
                line: Line::from(Span::styled(
                    category.title(),
                    Style::default()
                        .fg(section_color)
                        .add_modifier(Modifier::BOLD),
                )),
            });

            for action in Action::all()
                .iter()
                .copied()
                .filter(|action| action.category() == category)
            {
                let direct = self.keymap.keys_for_action(action);
                let mandatory = self.keymap.mandatory_keys_for_action(action);
                let state = self.keymap_state_for_action(action);
                let mut parts = Vec::new();
                match state {
                    ActionBindingState::Bound => parts.push(
                        direct
                            .into_iter()
                            .map(|key| key.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    ActionBindingState::Unbound => parts.push("(unbound)".to_string()),
                    ActionBindingState::Disabled => parts.push("(disabled)".to_string()),
                }
                if !mandatory.is_empty() {
                    parts.push(format!(
                        "{} [mandatory]",
                        mandatory
                            .into_iter()
                            .map(|key| key.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                parts.extend(self.contextual_labels_for_action(action));
                let key_text = parts.join(" · ");

                rows.push(self.selectable_row(
                    SettingsSelectable::Action(action),
                    selected_item,
                    self.settings_item_color(SettingsSelectable::Action(action)),
                    pad_column(&key_text, value_col),
                    action.settings_label().to_string(),
                    action_col,
                ));
            }

            rows.push(SettingsRow {
                selectable: None,
                line: Line::from(""),
            });
        }

        rows
    }

    fn selectable_row(
        &self,
        selectable: SettingsSelectable,
        selected_item: SettingsSelectable,
        accent: Color,
        left_text: String,
        right_text: String,
        right_width: usize,
    ) -> SettingsRow {
        let is_selected = selectable == selected_item;

        let line = if is_selected {
            let text_color = view_style::text_color_for_bg(accent);
            Line::from(vec![
                Span::styled(left_text, Style::default().fg(text_color).bg(accent)),
                Span::styled(
                    pad_column(&right_text, right_width),
                    Style::default()
                        .fg(text_color)
                        .bg(accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(left_text, Style::default().fg(Color::White)),
                Span::styled(
                    pad_column(&right_text, right_width),
                    Style::default().fg(accent),
                ),
            ])
        };

        SettingsRow {
            selectable: Some(selectable),
            line,
        }
    }

    fn render_settings_overlay(
        &self,
        f: &mut Frame,
        terminal_size: Rect,
        overlay: &SettingsOverlay,
    ) {
        match overlay {
            SettingsOverlay::CaptureKey { action } => {
                let rect = self.modal_rect_ratio(terminal_size, 1, 2);
                let block = Block::default()
                    .title(Line::from(Span::styled(
                        format!("rebind {}", action.settings_label()),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )))
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(
                        Style::default()
                            .fg(self.settings_item_color(SettingsSelectable::Action(*action))),
                    );

                let body = vec![
                    Line::from(Span::styled(
                        "Press the new keybinding.",
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        "Esc: cancel · Backspace: disable action · Delete: unbind",
                        Style::default().fg(Color::Gray),
                    )),
                ];

                f.render_widget(ratatui::widgets::Clear, rect);
                f.render_widget(Paragraph::new(body).block(block), rect);
            }
            SettingsOverlay::SelectWeekStartDay { selected } => {
                let rect = self.modal_rect_ratio(terminal_size, 1, 3);
                let block = Block::default()
                    .title(Line::from(Span::styled(
                        "week start",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )))
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(
                        Style::default()
                            .fg(self.settings_item_color(SettingsSelectable::WeekStartDay)),
                    );

                let options = Self::week_start_options();
                let lines: Vec<Line<'static>> = options
                    .iter()
                    .enumerate()
                    .map(|(idx, day)| {
                        let label = week_day_label(*day);
                        if idx == *selected {
                            Line::from(Span::styled(
                                format!("> {}", label),
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(self.settings_item_color(SettingsSelectable::WeekStartDay))
                                    .add_modifier(Modifier::BOLD),
                            ))
                        } else {
                            Line::from(Span::styled(
                                format!("  {}", label),
                                Style::default().fg(Color::White),
                            ))
                        }
                    })
                    .collect();

                f.render_widget(ratatui::widgets::Clear, rect);
                f.render_widget(Paragraph::new(lines).block(block), rect);
            }
        }
    }
}

fn format_settings_control_hint(keys: impl IntoIterator<Item = String>, label: &str) -> String {
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
    match day {
        FirstDayOfWeek::Monday => "Monday",
        FirstDayOfWeek::Tuesday => "Tuesday",
        FirstDayOfWeek::Wednesday => "Wednesday",
        FirstDayOfWeek::Thursday => "Thursday",
        FirstDayOfWeek::Friday => "Friday",
        FirstDayOfWeek::Saturday => "Saturday",
        FirstDayOfWeek::Sunday => "Sunday",
    }
}

fn pad_column(value: &str, width: usize) -> String {
    let mut chars = value.chars();
    let mut out = String::new();
    for _ in 0..width {
        let Some(ch) = chars.next() else {
            break;
        };
        out.push(ch);
    }

    let current_width = out.chars().count();
    if current_width < width {
        out.push_str(&" ".repeat(width - current_width));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::format_settings_control_hint;

    #[test]
    fn settings_control_hint_preserves_runtime_key_order_and_deduplicates() {
        let hint = format_settings_control_hint(
            ["Esc", "F1", "?", "F1"].into_iter().map(str::to_string),
            "close",
        );
        assert_eq!(hint, "Esc/F1/? close");
    }

    #[test]
    fn settings_control_hint_exposes_unreachable_control_groups() {
        assert_eq!(
            format_settings_control_hint(std::iter::empty(), "jump"),
            "(unbound) jump"
        );
    }
}
