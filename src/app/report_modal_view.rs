use chrono::NaiveDate;
use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::constants::REPORT_MODAL_SETTINGS;
use crate::domain::{
    BalanceReportSummary, CategoryId, CategoryLogEntry, DRIFT_CATEGORY_ID, ReportPeriod,
};
use crate::keybindings::Action;

use super::{App, ui_helpers, view_style};

fn balance_key_hint(key: impl ToString) -> String {
    let raw = key.to_string();
    let label = if raw.chars().count() == 1 {
        raw.to_uppercase()
    } else {
        raw
    };
    format!("[{label}]")
}

impl App {
    pub(super) fn render_report_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let summary = self.report_rows();
        let logs_for_view = self
            .report_logs_category_id
            .map(|category_id| self.report_logs_for_category(category_id));

        let body_row_count = logs_for_view
            .as_ref()
            .map_or(summary.entries.len(), |logs| logs.len());

        let preferred_inner_width = self
            .preferred_report_inner_width(&summary, logs_for_view.as_deref())
            .max(if self.historical_activity_edit.is_some() {
                REPORT_MODAL_SETTINGS.historical_activity_editor_min_width
            } else if self.report_range_edit.is_some() {
                REPORT_MODAL_SETTINGS.range_editor_min_width
            } else {
                0
            });

        let modal_rect = self.report_modal_rect(
            terminal_size,
            body_row_count,
            preferred_inner_width.saturating_add(REPORT_MODAL_SETTINGS.expanded_inner_padding),
        );
        let selected_summary_index = if summary.entries.is_empty() {
            None
        } else {
            Some(self.report_selected_index.min(summary.entries.len() - 1))
        };
        let interval_label = ui_helpers::format_report_interval_label(&summary.date);

        let border_color = if let Some(category_id) = self.report_logs_category_id {
            self.category_color_for_id(category_id)
        } else {
            selected_summary_index
                .and_then(|idx| summary.entries.get(idx))
                .map(|entry| entry.color)
                .unwrap_or(Color::White)
        };

        let interval_title = Line::from(Span::styled(
            interval_label,
            Style::default().fg(Color::White),
        ))
        .alignment(Alignment::Left);

        let center_title = Line::from(Span::styled(
            "Balance",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);

        let total_title = Line::from(Span::styled(
            self.format_balance_time(summary.total_balance_seconds),
            Style::default().fg(view_style::balance_color(summary.total_balance_seconds)),
        ))
        .alignment(Alignment::Right);

        let custom_range_active = self.report_range_is_custom() || self.report_range_edit.is_some();
        let period_bottom_title = Line::from(vec![
            view_style::report_period_label_span(
                "Day",
                !custom_range_active && self.report_period == ReportPeriod::Today,
            ),
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span(
                "Week",
                !custom_range_active && self.report_period == ReportPeriod::Week,
            ),
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span(
                "Month",
                !custom_range_active && self.report_period == ReportPeriod::Month,
            ),
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span("Range", custom_range_active),
        ])
        .alignment(Alignment::Center);
        let snapshot_bottom_title = self.should_use_report_snapshot().then(|| {
            Line::from(Span::styled(
                self.report_snapshot_status_label(),
                Style::default().fg(Color::DarkGray),
            ))
            .alignment(Alignment::Left)
        });
        let interaction_bottom_title = if let Some(edit) = self.historical_activity_edit.as_ref() {
            if edit.confirmation.is_some() {
                let labels = self.historical_activity_conflict_labels();
                let preview = if labels.is_empty() {
                    "recorded activity".to_string()
                } else {
                    let remaining = labels.len().saturating_sub(3);
                    let mut preview = labels.into_iter().take(3).collect::<Vec<_>>().join("; ");
                    if remaining > 0 {
                        preview.push_str(&format!("; +{remaining} more"));
                    }
                    preview
                };
                let mut spans = vec![
                    Span::styled("collision · ", Style::default().fg(Color::Yellow)),
                    Span::styled(preview, Style::default().fg(Color::White)),
                ];
                if edit.confirmation.as_ref().is_some_and(|confirmation| {
                    confirmation.conflicts.iter().any(|item| item.active)
                }) {
                    let active_name = self
                        .time_tracker
                        .category_by_id(self.time_tracker.active_category_id())
                        .map(|category| self.display_layer_name(&category.name))
                        .unwrap_or_else(|| "current layer".to_string());
                    spans.push(Span::styled(
                        format!(" · current stays {active_name}"),
                        Style::default().fg(Color::Gray),
                    ));
                }
                spans.push(Span::styled(
                    " · Enter replace · Esc back",
                    Style::default().fg(Color::Gray),
                ));
                Some(Line::from(spans).alignment(Alignment::Right))
            } else {
                let active_style = Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
                let inactive_style = Style::default().fg(Color::White);
                let target = self
                    .historical_activity_target_name()
                    .unwrap_or_else(|| "unavailable".to_string());
                let mut spans = vec![
                    Span::styled("log past · layer ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        target,
                        if edit.active_field == super::HistoricalActivityField::Layer {
                            active_style
                        } else {
                            inactive_style
                        },
                    ),
                    Span::styled(" · from ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        edit.from.clone(),
                        if edit.active_field == super::HistoricalActivityField::From {
                            active_style
                        } else {
                            inactive_style
                        },
                    ),
                    Span::styled(" · to ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        edit.to.clone(),
                        if edit.active_field == super::HistoricalActivityField::To {
                            active_style
                        } else {
                            inactive_style
                        },
                    ),
                ];
                if let Some(error) = edit.error.as_ref() {
                    spans.push(Span::styled(
                        format!(" · {error}"),
                        Style::default().fg(Color::Red),
                    ));
                    spans.push(Span::styled(
                        " · Enter retry · Esc cancel",
                        Style::default().fg(Color::Gray),
                    ));
                } else {
                    spans.push(Span::styled(
                        " · ←/→ layer · Tab next · Enter save · Esc cancel",
                        Style::default().fg(Color::Gray),
                    ));
                }
                Some(Line::from(spans).alignment(Alignment::Right))
            }
        } else if let Some(edit) = self.report_range_edit.as_ref() {
            let active_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            let inactive_style = Style::default().fg(Color::White);
            let mut spans = vec![
                Span::styled("from ", Style::default().fg(Color::Gray)),
                Span::styled(
                    edit.from.clone(),
                    if edit.active_field == super::ReportRangeField::From {
                        active_style
                    } else {
                        inactive_style
                    },
                ),
                Span::styled(" · to ", Style::default().fg(Color::Gray)),
                Span::styled(
                    edit.to.clone(),
                    if edit.active_field == super::ReportRangeField::To {
                        active_style
                    } else {
                        inactive_style
                    },
                ),
            ];
            if let Some(error) = edit.error.as_ref() {
                spans.push(Span::styled(
                    format!(" · {error}"),
                    Style::default().fg(Color::Red),
                ));
                spans.push(Span::styled(
                    " · Enter retry · Esc cancel",
                    Style::default().fg(Color::Gray),
                ));
            } else {
                spans.push(Span::styled(
                    " · Tab next · Enter apply · Esc cancel",
                    Style::default().fg(Color::Gray),
                ));
            }
            Some(Line::from(spans).alignment(Alignment::Right))
        } else if self.report_logs_category_id.is_some() {
            let log_key = self
                .keymap
                .keys_for_action(Action::LogActivity)
                .first()
                .map(ToString::to_string);
            let label = if self.report_log_edit.is_some() {
                "Enter save · Esc cancel".to_string()
            } else {
                log_key.map_or_else(
                    || "Enter edit · Esc back".to_string(),
                    |key| format!("{} Log past · Enter edit · Esc back", balance_key_hint(key)),
                )
            };
            Some(
                Line::from(Span::styled(label, Style::default().fg(Color::Gray)))
                    .alignment(Alignment::Right),
            )
        } else {
            self.keymap
                .keys_for_action(Action::LogActivity)
                .first()
                .map(|key| {
                    Line::from(Span::styled(
                        format!("{} Log past", balance_key_hint(key)),
                        Style::default().fg(Color::Gray),
                    ))
                    .alignment(Alignment::Right)
                })
        };

        let mut frame_block = Block::default()
            .title(interval_title)
            .title(center_title)
            .title(total_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        let default_summary = self.report_logs_category_id.is_none()
            && self.historical_activity_edit.is_none()
            && self.report_range_edit.is_none();
        if default_summary {
            if let Some(snapshot_bottom_title) = snapshot_bottom_title {
                frame_block = frame_block.title_bottom(snapshot_bottom_title);
            }
            frame_block = frame_block.title_bottom(period_bottom_title);
        }
        if let Some(interaction_bottom_title) = interaction_bottom_title {
            frame_block = frame_block.title_bottom(interaction_bottom_title);
        }

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_widget(frame_block.clone(), modal_rect);
        self.render_report_navigation_arrows(f, modal_rect);

        let list_area = frame_block.inner(modal_rect);

        if let Some(category_id) = self.report_logs_category_id {
            let empty_logs = Vec::new();
            self.render_report_logs_view(
                f,
                list_area,
                logs_for_view.as_deref().unwrap_or(&empty_logs),
                category_id,
                border_color,
            );
        } else {
            self.render_report_summary_view(f, list_area, &summary, selected_summary_index);
        }
    }

    fn preferred_report_inner_width(
        &self,
        summary: &BalanceReportSummary,
        logs_for_view: Option<&[CategoryLogEntry]>,
    ) -> usize {
        if let Some(logs) = logs_for_view {
            let max_detail = logs
                .iter()
                .map(|row| {
                    if row.description.trim().is_empty() {
                        format!("{}-{}", row.start_time, row.end_time)
                    } else {
                        format!("{} · {}-{}", row.description, row.start_time, row.end_time)
                    }
                })
                .map(|text| text.chars().count())
                .max()
                .unwrap_or(REPORT_MODAL_SETTINGS.log_detail_fallback_width)
                .min(REPORT_MODAL_SETTINGS.log_detail_max_width);

            let is_none = self.report_logs_category_id == Some(DRIFT_CATEGORY_ID);
            let metric_width = if is_none {
                REPORT_MODAL_SETTINGS.detail_metric_width_drift
            } else {
                REPORT_MODAL_SETTINGS.detail_metric_width_default
            };

            REPORT_MODAL_SETTINGS.detail_date_preview_width + 1 + max_detail + 1 + metric_width
        } else {
            let max_name = summary
                .entries
                .iter()
                .map(|entry| entry.category_name.chars().count())
                .max()
                .unwrap_or(REPORT_MODAL_SETTINGS.summary_name_fallback_width)
                .min(REPORT_MODAL_SETTINGS.summary_name_max_width);

            2 + max_name + 1 + REPORT_MODAL_SETTINGS.summary_metric_width
        }
    }

    fn render_report_navigation_arrows(&self, f: &mut Frame, modal_rect: Rect) {
        if self.report_range_edit.is_some() || modal_rect.width <= 2 || modal_rect.height <= 2 {
            return;
        }

        let mid_y = modal_rect.y + (modal_rect.height / 2);
        let left_arrow = Paragraph::new(Line::from(Span::styled(
            "←",
            Style::default().fg(Color::Gray),
        )));
        let right_arrow = Paragraph::new(Line::from(Span::styled(
            "→",
            if !self.can_shift_report_interval_newer() {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(Color::Gray)
            },
        )));

        let left_rect = Rect::new(modal_rect.x, mid_y, 1, 1);
        let right_rect = Rect::new(
            modal_rect.x + modal_rect.width.saturating_sub(1),
            mid_y,
            1,
            1,
        );

        f.render_widget(left_arrow, left_rect);
        f.render_widget(right_arrow, right_rect);
    }

    fn render_report_logs_view(
        &self,
        f: &mut Frame,
        list_area: Rect,
        logs: &[CategoryLogEntry],
        category_id: CategoryId,
        border_color: Color,
    ) {
        let selected_log_index = if logs.is_empty() {
            None
        } else {
            Some(self.report_log_selected_index.min(logs.len() - 1))
        };
        let is_none_category = category_id == DRIFT_CATEGORY_ID;

        let row_width = list_area.width as usize;
        let metric_width = if is_none_category {
            REPORT_MODAL_SETTINGS.detail_metric_width_drift
        } else {
            REPORT_MODAL_SETTINGS.detail_metric_width_default
        };
        let window = self.current_report_window();
        let show_date_column = window.start != window.end;
        let date_width = if show_date_column {
            REPORT_MODAL_SETTINGS.detail_date_width
        } else {
            0
        };
        let separator_count = if show_date_column { 3 } else { 2 };
        let tag_width = row_width
            .saturating_sub(
                date_width
                    + REPORT_MODAL_SETTINGS.detail_time_width
                    + metric_width
                    + separator_count,
            )
            .max(REPORT_MODAL_SETTINGS.min_tag_width);

        let items: Vec<ListItem> = logs
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let is_selected = selected_log_index == Some(idx);
                let date_label = if show_date_column {
                    self.format_log_date_label(&row.date)
                } else {
                    String::new()
                };
                let date = if show_date_column {
                    self.truncate_label(&date_label, date_width)
                } else {
                    String::new()
                };
                let date_cell = if show_date_column {
                    format!("{date:<width$}", width = date_width)
                } else {
                    String::new()
                };

                let displayed_description = self
                    .report_log_edit
                    .as_ref()
                    .filter(|edit| row.session_id == Some(edit.session_id))
                    .map(|edit| format!("{}▏", edit.draft))
                    .unwrap_or_else(|| row.description.trim().to_string());
                let tag = self.truncate_label(&displayed_description, tag_width);
                let tag_cell = format!("{tag:<width$}", width = tag_width);

                let time_text = self.truncate_label(
                    &format!("{}-{}", row.start_time, row.end_time),
                    REPORT_MODAL_SETTINGS.detail_time_width,
                );
                let time_cell = format!(
                    "{time_text:<width$}",
                    width = REPORT_MODAL_SETTINGS.detail_time_width
                );

                let metric_value = if is_none_category {
                    self.format_time(row.elapsed_seconds)
                } else if row.balance_seconds == 0 && row.balance_effect < 0 {
                    "-00:00:00".to_string()
                } else {
                    self.format_balance_time(row.balance_seconds)
                };
                let metric_cell = format!("{metric_value:>width$}", width = metric_width);

                let metric_color = if is_none_category {
                    Color::Gray
                } else if row.balance_seconds == 0 {
                    if row.balance_effect < 0 {
                        Color::Red
                    } else if row.balance_effect > 0 {
                        Color::Green
                    } else {
                        Color::Gray
                    }
                } else {
                    view_style::balance_color(row.balance_seconds)
                };

                if is_selected {
                    let text_color = view_style::text_color_for_bg(border_color);
                    let mut spans = Vec::new();
                    if show_date_column {
                        spans.push(Span::raw(date_cell.clone()).fg(text_color));
                        spans.push(Span::raw(" ").fg(text_color));
                    }
                    spans.push(Span::raw(tag_cell.clone()).fg(text_color));
                    spans.push(Span::raw(" ").fg(text_color));
                    spans.push(Span::raw(time_cell.clone()).fg(text_color));
                    spans.push(Span::raw(" ").fg(text_color));
                    spans.push(Span::raw(metric_cell.clone()).fg(text_color));

                    ListItem::new(Line::from(spans))
                        .style(Style::default().fg(text_color).bg(border_color))
                } else {
                    let mut spans = Vec::new();
                    if show_date_column {
                        spans.push(Span::raw(date_cell).fg(Color::Gray));
                        spans.push(Span::raw(" ").fg(Color::Gray));
                    }
                    spans.push(Span::raw(tag_cell).fg(Color::White));
                    spans.push(Span::raw(" ").fg(Color::White));
                    spans.push(Span::raw(time_cell).fg(Color::Gray));
                    spans.push(Span::raw(" ").fg(Color::White));
                    spans.push(Span::raw(metric_cell).fg(metric_color));

                    ListItem::new(Line::from(spans))
                }
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(selected_log_index);

        let list = if logs.is_empty() {
            List::new(vec![ListItem::new(Line::from(vec![Span::styled(
                "No logs for this layer in this period.",
                Style::default().fg(Color::Gray),
            )]))])
        } else {
            List::new(items)
        };

        f.render_stateful_widget(list, list_area, &mut list_state);
    }

    fn render_report_summary_view(
        &self,
        f: &mut Frame,
        list_area: Rect,
        summary: &BalanceReportSummary,
        selected_summary_index: Option<usize>,
    ) {
        let row_width = list_area.width as usize;
        let name_width = row_width
            .saturating_sub(
                REPORT_MODAL_SETTINGS.summary_metric_width + REPORT_MODAL_SETTINGS.summary_name_gap,
            )
            .max(REPORT_MODAL_SETTINGS.min_tag_width);

        let items: Vec<ListItem> = summary
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = selected_summary_index == Some(idx);
                let dot = if entry.balance_effect < 0 {
                    "◯ "
                } else if entry.balance_effect == 0 {
                    "· "
                } else {
                    "● "
                };
                let branded_name = self.display_layer_name(&entry.category_name);
                let name = self.truncate_label(&branded_name, name_width);
                let pad = name_width.saturating_sub(name.chars().count()) + 1;
                let is_none_row = entry.category_id == DRIFT_CATEGORY_ID;
                let metric_value = if is_none_row {
                    self.format_time(entry.elapsed_seconds)
                } else if entry.balance_seconds == 0 && entry.balance_effect < 0 {
                    "-00:00:00".to_string()
                } else {
                    self.format_balance_time(entry.balance_seconds)
                };
                let metric_color = if is_none_row {
                    Color::Gray
                } else if entry.balance_seconds == 0 {
                    if entry.balance_effect < 0 {
                        Color::Red
                    } else if entry.balance_effect > 0 {
                        Color::Green
                    } else {
                        Color::Gray
                    }
                } else {
                    view_style::balance_color(entry.balance_seconds)
                };

                if is_selected {
                    let text_color = if is_none_row {
                        Color::Black
                    } else {
                        view_style::text_color_for_bg(entry.color)
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(text_color),
                        Span::raw(name).fg(text_color),
                        Span::raw(" ".repeat(pad)).fg(text_color),
                        Span::raw(metric_value).fg(text_color),
                    ]))
                    .style(Style::default().fg(text_color).bg(entry.color))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(entry.color),
                        Span::raw(name).fg(Color::White),
                        Span::raw(" ".repeat(pad)).fg(Color::White),
                        Span::raw(metric_value).fg(metric_color),
                    ]))
                }
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(selected_summary_index);

        let list = if summary.entries.is_empty() {
            List::new(vec![ListItem::new(Line::from(vec![Span::styled(
                "No tracked sessions for this period.",
                Style::default().fg(Color::Gray),
            )]))])
        } else {
            List::new(items)
        };

        f.render_stateful_widget(list, list_area, &mut list_state);
    }

    fn format_log_date_label(&self, date: &str) -> String {
        if self.report_range_is_custom() {
            return ui_helpers::format_report_interval_label(date);
        }
        match self.report_period {
            ReportPeriod::Today => String::new(),
            ReportPeriod::Week => NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map(|parsed| parsed.format("%a %-d").to_string())
                .unwrap_or_else(|_| date.to_string()),
            ReportPeriod::Month => ui_helpers::format_report_interval_label(date),
        }
    }
}

#[cfg(test)]
mod hardening_tests {
    use super::balance_key_hint;

    #[test]
    fn balance_key_hint_makes_single_letter_actions_legible() {
        assert_eq!(balance_key_hint("l"), "[L]");
        assert_eq!(balance_key_hint("Ctrl+L"), "[Ctrl+L]");
    }
}
