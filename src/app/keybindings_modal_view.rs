use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{
    constants::ATLAS_LAYOUT_SETTINGS,
    domain::FirstDayOfWeek,
    keybindings::{Action, ActionCategory},
};

use super::{App, AtlasOverlay, AtlasSelectable, view_style};

#[derive(Clone)]
struct AtlasRow {
    selectable: Option<AtlasSelectable>,
    line: Line<'static>,
}

impl App {
    pub(super) fn render_keybindings_modal(&mut self, f: &mut Frame, terminal_size: Rect) {
        let selected_item = self.selected_atlas_item();
        let border_color = self.atlas_item_color(selected_item);
        let bottom_description = self.atlas_item_description(selected_item);

        let modal_rect = self.modal_rect_ratio(terminal_size, 5, 6);
        let title = Line::from(Span::styled(
            "atlas",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);

        let bottom_left = Line::from(Span::styled(
            "Esc/F1/? close",
            Style::default().fg(Color::DarkGray),
        ))
        .alignment(Alignment::Left);
        let bottom_center = Line::from(Span::styled(
            bottom_description,
            Style::default().fg(Color::Gray),
        ))
        .alignment(Alignment::Center);
        let bottom_right = Line::from(Span::styled(
            "↑↓ move · Home/End jump",
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

        let rows = self.command_atlas_rows(selected_item);
        let selected_row = rows
            .iter()
            .position(|row| row.selectable == Some(selected_item))
            .unwrap_or(0);

        let viewport_rows = body_rect.height as usize;
        if self.keybindings_scroll > selected_row {
            self.keybindings_scroll = selected_row;
        } else if selected_row >= self.keybindings_scroll.saturating_add(viewport_rows)
            && viewport_rows > 0
        {
            self.keybindings_scroll = selected_row + 1 - viewport_rows;
        }
        let max_scroll = rows.len().saturating_sub(viewport_rows);
        self.keybindings_scroll = self.keybindings_scroll.min(max_scroll);

        let body_lines: Vec<Line<'static>> = rows.into_iter().map(|row| row.line).collect();
        let body = Paragraph::new(body_lines).scroll((self.keybindings_scroll as u16, 0));
        f.render_widget(body, body_rect);

        if let Some(overlay) = self.atlas_overlay.as_ref() {
            self.render_atlas_overlay(f, terminal_size, overlay);
        }
    }

    fn command_atlas_rows(&self, selected_item: AtlasSelectable) -> Vec<AtlasRow> {
        let value_col = ATLAS_LAYOUT_SETTINGS.value_col_width;
        let action_col = ATLAS_LAYOUT_SETTINGS.action_col_width;

        let mut rows = vec![
            AtlasRow {
                selectable: None,
                line: Line::from(vec![
                    Span::styled(
                        pad_column("binding/value", value_col),
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "target",
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            },
            AtlasRow {
                selectable: None,
                line: Line::from(Span::styled(
                    "Atlas Settings",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
            },
            self.selectable_row(
                AtlasSelectable::TimeLogPath,
                selected_item,
                self.atlas_item_color(AtlasSelectable::TimeLogPath),
                pad_column(
                    &self.truncate_label(
                        &crate::storage::get_time_log_path().display().to_string(),
                        value_col,
                    ),
                    value_col,
                ),
                "time_log_path".to_string(),
                action_col,
            ),
            self.selectable_row(
                AtlasSelectable::WeekStartDay,
                selected_item,
                self.atlas_item_color(AtlasSelectable::WeekStartDay),
                pad_column(&self.first_day_of_week_label(), value_col),
                "week_start".to_string(),
                action_col,
            ),
            AtlasRow {
                selectable: None,
                line: Line::from(""),
            },
        ];

        for category in ActionCategory::all() {
            let section_color = self.atlas_item_color(AtlasSelectable::Action(
                Action::all()
                    .iter()
                    .copied()
                    .find(|action| action.category() == category)
                    .unwrap_or(Action::Quit),
            ));

            rows.push(AtlasRow {
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
                let keys = self.effective_keys_for_action(action);
                let key_text = if keys.is_empty() {
                    "(unbound)".to_string()
                } else {
                    keys.into_iter()
                        .map(|key| key.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                rows.push(self.selectable_row(
                    AtlasSelectable::Action(action),
                    selected_item,
                    self.atlas_item_color(AtlasSelectable::Action(action)),
                    pad_column(&key_text, value_col),
                    action.config_name().to_string(),
                    action_col,
                ));
            }

            rows.push(AtlasRow {
                selectable: None,
                line: Line::from(""),
            });
        }

        rows
    }

    fn selectable_row(
        &self,
        selectable: AtlasSelectable,
        selected_item: AtlasSelectable,
        accent: Color,
        left_text: String,
        right_text: String,
        right_width: usize,
    ) -> AtlasRow {
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

        AtlasRow {
            selectable: Some(selectable),
            line,
        }
    }

    fn render_atlas_overlay(&self, f: &mut Frame, terminal_size: Rect, overlay: &AtlasOverlay) {
        match overlay {
            AtlasOverlay::CaptureKey { action } => {
                let rect = self.modal_rect_ratio(terminal_size, 1, 2);
                let block = Block::default()
                    .title(Line::from(Span::styled(
                        format!("rebind {}", action.config_name()),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )))
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(
                        Style::default()
                            .fg(self.atlas_item_color(AtlasSelectable::Action(*action))),
                    );

                let body = vec![
                    Line::from(Span::styled(
                        "Press the new keybinding now.",
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        "Esc: cancel · Backspace/Delete: unbind",
                        Style::default().fg(Color::Gray),
                    )),
                ];

                f.render_widget(ratatui::widgets::Clear, rect);
                f.render_widget(Paragraph::new(body).block(block), rect);
            }
            AtlasOverlay::EditTimeLogPath { input } => {
                let rect = self.modal_rect_ratio(terminal_size, 2, 3);
                let block = Block::default()
                    .title(Line::from(Span::styled(
                        "time log path",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )))
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(
                        Style::default().fg(self.atlas_item_color(AtlasSelectable::TimeLogPath)),
                    );

                let body = vec![
                    Line::from(Span::styled(
                        "Type a file path (.csv) or a directory.",
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        "If directory is given, time_log.csv is appended.",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        input.to_string(),
                        Style::default().fg(Color::Cyan),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Enter: save · Esc: cancel",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];

                f.render_widget(ratatui::widgets::Clear, rect);
                f.render_widget(Paragraph::new(body).block(block), rect);
            }
            AtlasOverlay::SelectWeekStartDay { selected } => {
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
                        Style::default().fg(self.atlas_item_color(AtlasSelectable::WeekStartDay)),
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
                                    .bg(self.atlas_item_color(AtlasSelectable::WeekStartDay))
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
