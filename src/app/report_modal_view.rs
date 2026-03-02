use chrono::NaiveDate;
use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::domain::{CategoryId, ReportPeriod};

use super::{App, ui_helpers, view_style};

impl App {
    pub(super) fn render_report_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let summary = self.report_rows();
        let logs_for_view = self
            .report_logs_category_id
            .map(|category_id| self.report_logs_for_category(category_id));

        let body_row_count = logs_for_view
            .as_ref()
            .map_or(summary.entries.len(), |logs| logs.len());

        let preferred_inner_width = if let Some(logs) = logs_for_view.as_ref() {
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
                .unwrap_or(16)
                .min(40);

            let is_none = self.report_logs_category_id == Some(CategoryId::new(0));
            let metric_width = if is_none { 8 } else { 9 };
            let date_width = 7usize;
            date_width + 1 + max_detail + 1 + metric_width
        } else {
            let max_name = summary
                .entries
                .iter()
                .map(|entry| entry.category_name.chars().count())
                .max()
                .unwrap_or(12)
                .min(28);

            2 + max_name + 1 + 9
        };

        let modal_rect = self.report_modal_rect(
            terminal_size,
            body_row_count,
            preferred_inner_width.saturating_add(4),
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

        let center_label = "Karma".to_string();

        let center_title = Line::from(Span::styled(
            center_label,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);

        let total_title = Line::from(Span::styled(
            self.format_karma_time(summary.total_karma_seconds),
            Style::default().fg(view_style::karma_color(summary.total_karma_seconds)),
        ))
        .alignment(Alignment::Right);

        let period_bottom_title = Line::from(vec![
            view_style::report_period_label_span("day", self.report_period == ReportPeriod::Today),
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span("week", self.report_period == ReportPeriod::Week),
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            view_style::report_period_label_span(
                "month",
                self.report_period == ReportPeriod::Month,
            ),
        ])
        .alignment(Alignment::Center);

        let frame_block = Block::default()
            .title(interval_title)
            .title(center_title)
            .title(total_title)
            .title_bottom(period_bottom_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_widget(frame_block.clone(), modal_rect);

        if modal_rect.width > 2 && modal_rect.height > 2 {
            let mid_y = modal_rect.y + (modal_rect.height / 2);
            let left_arrow = Paragraph::new(Line::from(Span::styled(
                "←",
                Style::default().fg(Color::Gray),
            )));
            let right_arrow = Paragraph::new(Line::from(Span::styled(
                "→",
                if self.report_period_offset == 0 {
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

        let list_area = frame_block.inner(modal_rect);

        if let Some(category_id) = self.report_logs_category_id {
            let empty_logs = Vec::new();
            let logs = logs_for_view.as_ref().unwrap_or(&empty_logs);
            let selected_log_index = if logs.is_empty() {
                None
            } else {
                Some(self.report_log_selected_index.min(logs.len() - 1))
            };
            let is_none_category = category_id == CategoryId::new(0);

            let row_width = list_area.width as usize;
            let metric_width = if is_none_category { 8 } else { 9 };
            let time_width = 17;
            let show_date_column = self.report_period != ReportPeriod::Today;
            let date_width = if show_date_column { 6 } else { 0 };
            let separator_count = if show_date_column { 3 } else { 2 };
            let tag_width = row_width
                .saturating_sub(date_width + time_width + metric_width + separator_count)
                .max(4);

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

                    let tag = self.truncate_label(row.description.trim(), tag_width);
                    let tag_cell = format!("{tag:<width$}", width = tag_width);

                    let time_text = self.truncate_label(
                        &format!("{}-{}", row.start_time, row.end_time),
                        time_width,
                    );
                    let time_cell = format!("{time_text:<width$}", width = time_width);

                    let metric_value = if is_none_category {
                        self.format_time(row.elapsed_seconds)
                    } else if row.karma_seconds == 0 && row.karma_effect < 0 {
                        "-00:00:00".to_string()
                    } else {
                        self.format_karma_time(row.karma_seconds)
                    };
                    let metric_cell = format!("{metric_value:>width$}", width = metric_width);

                    let metric_color = if is_none_category {
                        Color::Gray
                    } else if row.karma_seconds == 0 {
                        if row.karma_effect < 0 {
                            Color::Red
                        } else if row.karma_effect > 0 {
                            Color::Green
                        } else {
                            Color::Gray
                        }
                    } else {
                        view_style::karma_color(row.karma_seconds)
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
        } else {
            let row_width = list_area.width as usize;
            let metric_width = 9;
            let name_width = row_width.saturating_sub(metric_width + 4).max(4);

            let items: Vec<ListItem> = summary
                .entries
                .iter()
                .enumerate()
                .map(|(idx, entry)| {
                    let is_selected = selected_summary_index == Some(idx);
                    let dot = if entry.karma_effect < 0 {
                        "◯ "
                    } else if entry.karma_effect == 0 {
                        "· "
                    } else {
                        "● "
                    };
                    let branded_name = self.display_layer_name(&entry.category_name);
                    let name = self.truncate_label(&branded_name, name_width);
                    let pad = name_width.saturating_sub(name.chars().count()) + 1;
                    let is_none_row = entry.category_id == CategoryId::new(0);
                    let metric_value = if is_none_row {
                        self.format_time(entry.elapsed_seconds)
                    } else if entry.karma_seconds == 0 && entry.karma_effect < 0 {
                        "-00:00:00".to_string()
                    } else {
                        self.format_karma_time(entry.karma_seconds)
                    };
                    let metric_color = if is_none_row {
                        Color::Gray
                    } else if entry.karma_seconds == 0 {
                        if entry.karma_effect < 0 {
                            Color::Red
                        } else if entry.karma_effect > 0 {
                            Color::Green
                        } else {
                            Color::Gray
                        }
                    } else {
                        view_style::karma_color(entry.karma_seconds)
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
    }

    fn format_log_date_label(&self, date: &str) -> String {
        match self.report_period {
            ReportPeriod::Today => String::new(),
            ReportPeriod::Week => NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map(|parsed| parsed.format("%a %-d").to_string())
                .unwrap_or_else(|_| date.to_string()),
            ReportPeriod::Month => ui_helpers::format_report_interval_label(date),
        }
    }
}
