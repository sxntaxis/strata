use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::constants::REPORT_MODAL_SETTINGS;
use crate::domain::{BalanceReportSummary, CategoryId, CategoryLogEntry, DRIFT_CATEGORY_ID};

use super::{
    App, LedgerEntryField, ReportRangeBoundary, balance_instrument, overlay_layout, ui_helpers,
    view_style,
};

fn summary_border_color(
    summary: &BalanceReportSummary,
    selected_summary_index: Option<usize>,
    fallback: Color,
) -> Color {
    selected_summary_index
        .and_then(|idx| summary.entries.get(idx))
        .map(|entry| entry.color)
        .unwrap_or(fallback)
}

impl App {
    pub(super) fn render_report_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let summary = self.report_rows();
        let logs_for_view = self
            .report_logs_category_id
            .map(|category_id| self.report_logs_for_category(category_id));

        let default_summary = self.report_logs_category_id.is_none()
            && self.historical_activity_edit.is_none()
            && self.report_range_edit.is_none();
        let layer_detail = self.report_logs_category_id.is_some()
            && self.historical_activity_edit.is_none()
            && self.report_range_edit.is_none();
        let body_row_count = if layer_detail {
            self.report_ledger_row_count()
        } else {
            logs_for_view
                .as_ref()
                .map_or(summary.entries.len(), |logs| logs.len())
        };

        let preferred_inner_width = self
            .preferred_report_inner_width(&summary, logs_for_view.as_deref())
            .max(if self.historical_activity_edit.is_some() {
                REPORT_MODAL_SETTINGS.historical_activity_editor_min_width
            } else if self.report_range_edit.is_some() {
                REPORT_MODAL_SETTINGS.range_editor_min_width
            } else {
                0
            });

        let summary_content_width = preferred_inner_width
            .max(usize::from(
                balance_instrument::preferred_summary_inner_width(),
            ))
            .min(u16::MAX as usize) as u16;
        let summary_content_height =
            balance_instrument::preferred_summary_inner_height(body_row_count);

        let modal_rect = if default_summary || layer_detail {
            overlay_layout::centered_overlay_rect(
                terminal_size,
                summary_content_width.saturating_add(2),
                summary_content_height,
                1,
                3,
                crate::constants::APP_LAYOUT_SETTINGS.frame_margin,
            )
        } else {
            self.report_modal_rect(
                terminal_size,
                body_row_count,
                preferred_inner_width.saturating_add(REPORT_MODAL_SETTINGS.expanded_inner_padding),
            )
        };
        let selected_summary_index = if summary.entries.is_empty() {
            None
        } else {
            Some(self.report_selected_index.min(summary.entries.len() - 1))
        };
        let report_window = self.current_report_window();
        let (interval_start, interval_end) =
            ui_helpers::format_report_window_boundary_parts(&report_window);
        let interval_label = ui_helpers::format_report_window_boundaries(&report_window);

        let border_color = if let Some(category_id) = self.report_logs_category_id {
            self.category_color_for_id(category_id)
        } else {
            summary_border_color(&summary, selected_summary_index, self.theme_foreground())
        };

        let interval_title = Line::from(Span::styled(
            interval_label.clone(),
            Style::default().fg(self.theme_foreground()),
        ))
        .alignment(Alignment::Left);

        let center_title_text = self
            .report_logs_category_id
            .map(|category_id| self.report_layer_display_name(category_id))
            .unwrap_or_else(|| "Balance".to_string());
        let center_title = Line::from(Span::styled(
            center_title_text,
            Style::default()
                .fg(self.theme_foreground())
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);

        let total_title = Line::from(Span::styled(
            self.format_balance_time(summary.total_balance_seconds),
            Style::default().fg(view_style::balance_color(
                summary.total_balance_seconds,
                self.theme_error(),
                self.theme_success(),
                self.theme_status(),
            )),
        ))
        .alignment(Alignment::Right);

        let newer_chevron_style = if self.can_shift_report_interval_newer() {
            Style::default().fg(self.theme_status())
        } else {
            Style::default()
                .fg(self.theme_status())
                .add_modifier(Modifier::DIM)
        };
        let start_selected = self.report_range_boundary == Some(ReportRangeBoundary::Start);
        let end_selected = self.report_range_boundary == Some(ReportRangeBoundary::End);
        let start_text = if start_selected {
            format!("[{interval_start}]")
        } else {
            interval_start
        };
        let end_text = if end_selected {
            format!("[{interval_end}]")
        } else {
            interval_end
        };
        let period_bottom_title = Line::from(vec![
            Span::styled("< ", Style::default().fg(self.theme_status())),
            Span::styled(start_text, Style::default().fg(self.theme_foreground())),
            Span::styled(" – ", Style::default().fg(self.theme_foreground())),
            Span::styled(end_text, Style::default().fg(self.theme_foreground())),
            Span::styled(" >", newer_chevron_style),
        ])
        .alignment(Alignment::Center);
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
                    Span::styled("collision · ", Style::default().fg(self.theme_warning())),
                    Span::styled(preview, Style::default().fg(self.theme_foreground())),
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
                        Style::default().fg(self.theme_status()),
                    ));
                }
                spans.push(Span::styled(
                    " · Enter replace · Esc back",
                    Style::default().fg(self.theme_status()),
                ));
                Some(Line::from(spans).alignment(Alignment::Right))
            } else {
                let active_style = Style::default()
                    .fg(self.theme_accent())
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
                let inactive_style = Style::default().fg(self.theme_foreground());
                let target = self
                    .historical_activity_target_name()
                    .unwrap_or_else(|| "unavailable".to_string());
                let mut spans = vec![
                    Span::styled(
                        "log past · layer ",
                        Style::default().fg(self.theme_status()),
                    ),
                    Span::styled(
                        target,
                        if edit.active_field == super::HistoricalActivityField::Layer {
                            active_style
                        } else {
                            inactive_style
                        },
                    ),
                    Span::styled(" · from ", Style::default().fg(self.theme_status())),
                    Span::styled(
                        edit.from.clone(),
                        if edit.active_field == super::HistoricalActivityField::From {
                            active_style
                        } else {
                            inactive_style
                        },
                    ),
                    Span::styled(" · to ", Style::default().fg(self.theme_status())),
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
                        Style::default().fg(self.theme_error()),
                    ));
                    spans.push(Span::styled(
                        " · Enter retry · Esc cancel",
                        Style::default().fg(self.theme_status()),
                    ));
                } else {
                    spans.push(Span::styled(
                        " · ←/→ layer · Tab next · Enter save · Esc cancel",
                        Style::default().fg(self.theme_status()),
                    ));
                }
                Some(Line::from(spans).alignment(Alignment::Right))
            }
        } else if let Some(edit) = self.report_range_edit.as_ref() {
            let active_style = Style::default()
                .fg(self.theme_accent())
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            let inactive_style = Style::default().fg(self.theme_foreground());
            let mut spans = vec![
                Span::styled("from ", Style::default().fg(self.theme_status())),
                Span::styled(
                    edit.from.clone(),
                    if edit.active_field == super::ReportRangeField::From {
                        active_style
                    } else {
                        inactive_style
                    },
                ),
                Span::styled(" · to ", Style::default().fg(self.theme_status())),
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
                    Style::default().fg(self.theme_error()),
                ));
                spans.push(Span::styled(
                    " · Enter retry · Esc cancel",
                    Style::default().fg(self.theme_status()),
                ));
            } else {
                spans.push(Span::styled(
                    " · Tab next · Enter apply · Esc cancel",
                    Style::default().fg(self.theme_status()),
                ));
            }
            Some(Line::from(spans).alignment(Alignment::Right))
        } else if let Some(edit) = self.ledger_entry_edit.as_ref() {
            if let Some(confirmation) = edit.confirmation.as_ref() {
                let labels = self.ledger_entry_conflict_labels();
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
                    Span::styled("collision · ", Style::default().fg(self.theme_warning())),
                    Span::styled(preview, Style::default().fg(self.theme_foreground())),
                ];
                if confirmation.conflicts.iter().any(|item| item.active) {
                    spans.push(Span::styled(
                        " · current activity is protected",
                        Style::default().fg(self.theme_status()),
                    ));
                }
                spans.push(Span::styled(
                    " · Enter replace · Esc back",
                    Style::default().fg(self.theme_status()),
                ));
                Some(Line::from(spans).alignment(Alignment::Right))
            } else {
                edit.error.as_ref().map(|error| {
                    Line::from(vec![
                        Span::styled(error.clone(), Style::default().fg(self.theme_error())),
                        Span::styled(
                            " · Enter retry · Esc cancel",
                            Style::default().fg(self.theme_status()),
                        ),
                    ])
                    .alignment(Alignment::Right)
                })
            }
        } else {
            None
        };

        let mut frame_block = if default_summary || layer_detail {
            Block::default()
                .style(Style::default().bg(self.theme_background()))
                .title(center_title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
        } else {
            Block::default()
                .style(Style::default().bg(self.theme_background()))
                .title(interval_title)
                .title(center_title)
                .title(total_title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
        };

        if default_summary || layer_detail {
            frame_block = frame_block.title_bottom(period_bottom_title);
        }
        if let Some(interaction_bottom_title) = interaction_bottom_title {
            frame_block = frame_block.title_bottom(interaction_bottom_title);
        }

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_widget(frame_block.clone(), modal_rect);
        if !default_summary && !layer_detail {
            self.render_report_navigation_arrows(f, modal_rect);
        }

        let frame_inner = frame_block.inner(modal_rect);
        let list_area = if default_summary || layer_detail {
            overlay_layout::modal_content_rect(frame_inner, summary_content_height, false)
        } else {
            frame_inner
        };

        if layer_detail {
            if let Some(category_id) = self.report_logs_category_id {
                let empty_logs = Vec::new();
                self.render_layer_detail_with_instrument(
                    f,
                    list_area,
                    &summary,
                    logs_for_view.as_deref().unwrap_or(&empty_logs),
                    category_id,
                    border_color,
                );
            }
        } else if let Some(category_id) = self.report_logs_category_id {
            let empty_logs = Vec::new();
            self.render_report_logs_view(
                f,
                list_area,
                logs_for_view.as_deref().unwrap_or(&empty_logs),
                category_id,
                border_color,
            );
        } else if default_summary {
            self.render_report_summary_with_instrument(
                f,
                list_area,
                &summary,
                selected_summary_index,
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
                        format!("{} {}-{} {}", row.date, row.start_time, row.end_date, row.end_time)
                    } else {
                        format!(
                            "{} · {} {}-{} {}",
                            row.description, row.date, row.start_time, row.end_date, row.end_time
                        )
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
            Style::default().fg(self.theme_status()),
        )));
        let right_arrow = Paragraph::new(Line::from(Span::styled(
            "→",
            if !self.can_shift_report_interval_newer() {
                Style::default()
                    .fg(self.theme_status())
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(self.theme_status())
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

    fn render_layer_detail_with_instrument(
        &self,
        f: &mut Frame,
        area: Rect,
        summary: &BalanceReportSummary,
        logs: &[CategoryLogEntry],
        category_id: CategoryId,
        border_color: Color,
    ) {
        let (total_area, meter_area, list_area) = if area.height >= 6 {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(area);
            (Some(rows[1]), Some(rows[2]), rows[4])
        } else if area.height >= 5 {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            (Some(rows[1]), Some(rows[2]), rows[4])
        } else if area.height >= 3 {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            (Some(rows[0]), Some(rows[1]), rows[2])
        } else {
            (None, None, area)
        };

        if let (Some(total_area), Some(meter_area)) = (total_area, meter_area) {
            self.render_layer_influence_instrument(
                f,
                total_area,
                meter_area,
                summary,
                category_id,
            );
        }

        self.render_report_logs_view(f, list_area, logs, category_id, border_color);
    }

    fn ledger_weighted_widths(total: usize, weights: &[usize]) -> Vec<usize> {
        let weight_sum = weights.iter().sum::<usize>().max(1);
        let mut widths = weights
            .iter()
            .map(|weight| total.saturating_mul(*weight) / weight_sum)
            .collect::<Vec<_>>();
        let used = widths.iter().sum::<usize>();
        let mut remaining = total.saturating_sub(used);
        let mut cursor = 0;
        while remaining > 0 && !widths.is_empty() {
            widths[cursor] = widths[cursor].saturating_add(1);
            remaining -= 1;
            cursor = (cursor + 1) % widths.len();
        }
        widths
    }

    fn ledger_center_cell(&self, text: &str, width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        let text = self.truncate_label(text, width);
        let text_width = text.chars().count();
        let remaining = width.saturating_sub(text_width);
        let left = remaining / 2;
        let right = remaining.saturating_sub(left);
        format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
    }

    fn ledger_date_label(&self, raw: &str) -> String {
        ui_helpers::format_report_interval_label(raw)
    }

    fn ledger_minute_time(raw: &str) -> String {
        raw.get(..5).unwrap_or(raw).to_string()
    }

    fn ledger_active_token(value: String, active: bool) -> String {
        if active { format!("[{value}]") } else { value }
    }

    fn ledger_metric_value(&self, row: &CategoryLogEntry, is_none_category: bool) -> String {
        if is_none_category {
            self.format_time(row.elapsed_seconds)
        } else if row.balance_seconds == 0 && row.balance_effect < 0 {
            "-00:00:00".to_string()
        } else {
            self.format_balance_time(row.balance_seconds)
        }
    }

    fn ledger_metric_color(&self, row: &CategoryLogEntry, is_none_category: bool) -> Color {
        if is_none_category {
            self.theme_status()
        } else if row.balance_seconds == 0 {
            if row.balance_effect < 0 {
                self.theme_error()
            } else if row.balance_effect > 0 {
                self.theme_success()
            } else {
                self.theme_status()
            }
        } else {
            view_style::balance_color(
                row.balance_seconds,
                self.theme_error(),
                self.theme_success(),
                self.theme_status(),
            )
        }
    }

    fn ledger_normal_line(
        &self,
        row: &CategoryLogEntry,
        row_width: usize,
        show_date_column: bool,
        selected_text: Option<Color>,
        is_none_category: bool,
    ) -> Line<'static> {
        let tag = if row.description.trim().is_empty() {
            String::new()
        } else {
            row.description.trim().to_string()
        };
        let start_time = Self::ledger_minute_time(&row.start_time);
        let end_time = Self::ledger_minute_time(&row.end_time);
        let metric = self.ledger_metric_value(row, is_none_category);
        let metric_color = selected_text
            .unwrap_or_else(|| self.ledger_metric_color(row, is_none_category));
        let tag_color = selected_text.unwrap_or_else(|| self.theme_foreground());
        let temporal_color = selected_text.unwrap_or_else(|| self.theme_status());
        let cross_day = row.date != row.end_date;

        let (widths, values): (Vec<usize>, Vec<(String, Color)>) = if cross_day {
            let widths = Self::ledger_weighted_widths(row_width, &[1, 2, 1]);
            let temporal = format!(
                "{} {} → {} {}",
                self.ledger_date_label(&row.date),
                start_time,
                self.ledger_date_label(&row.end_date),
                end_time,
            );
            (
                widths,
                vec![(tag, tag_color), (temporal, temporal_color), (metric, metric_color)],
            )
        } else if show_date_column {
            let widths = Self::ledger_weighted_widths(row_width, &[1, 1, 1, 1]);
            (
                widths,
                vec![
                    (tag, tag_color),
                    (self.ledger_date_label(&row.date), temporal_color),
                    (format!("{start_time}–{end_time}"), temporal_color),
                    (metric, metric_color),
                ],
            )
        } else {
            let widths = Self::ledger_weighted_widths(row_width, &[1, 1, 1]);
            (
                widths,
                vec![
                    (tag, tag_color),
                    (format!("{start_time}–{end_time}"), temporal_color),
                    (metric, metric_color),
                ],
            )
        };

        let spans = values
            .into_iter()
            .zip(widths)
            .map(|((value, color), width)| {
                Span::styled(self.ledger_center_cell(&value, width), Style::default().fg(color))
            })
            .collect::<Vec<_>>();
        Line::from(spans)
    }

    fn ledger_edit_line(
        &self,
        row_width: usize,
        show_date_column: bool,
        selected_text: Color,
    ) -> Line<'static> {
        let Some(edit) = self.ledger_entry_edit.as_ref() else {
            return Line::default();
        };
        let active = edit.active_field;
        let description = Self::ledger_active_token(
            edit.description.clone(),
            active == LedgerEntryField::Description,
        );
        let start_date = Self::ledger_active_token(
            self.ledger_date_label(&edit.start_date),
            active == LedgerEntryField::StartDate,
        );
        let start_time = Self::ledger_active_token(
            edit.start_time.clone(),
            active == LedgerEntryField::StartTime,
        );
        let end_date = Self::ledger_active_token(
            self.ledger_date_label(&edit.end_date),
            active == LedgerEntryField::EndDate,
        );
        let end_time = Self::ledger_active_token(
            edit.end_time.clone(),
            active == LedgerEntryField::EndTime,
        );
        let cross_day = edit.start_date != edit.end_date;
        let placeholder = "…".to_string();

        let (widths, values): (Vec<usize>, Vec<String>) = if cross_day {
            (
                Self::ledger_weighted_widths(row_width, &[1, 2, 1]),
                vec![
                    description,
                    format!("{start_date} {start_time} → {end_date} {end_time}"),
                    placeholder,
                ],
            )
        } else if show_date_column {
            (
                Self::ledger_weighted_widths(row_width, &[1, 1, 1, 1]),
                vec![
                    description,
                    start_date,
                    format!("{start_time}–{end_time}"),
                    placeholder,
                ],
            )
        } else {
            (
                Self::ledger_weighted_widths(row_width, &[1, 1, 1]),
                vec![
                    description,
                    format!("{start_date} · {start_time}–{end_time}"),
                    placeholder,
                ],
            )
        };

        Line::from(
            values
                .into_iter()
                .zip(widths)
                .map(|(value, width)| {
                    Span::styled(
                        self.ledger_center_cell(&value, width),
                        Style::default().fg(selected_text),
                    )
                })
                .collect::<Vec<_>>(),
        )
    }

    fn render_report_logs_view(
        &self,
        f: &mut Frame,
        list_area: Rect,
        logs: &[CategoryLogEntry],
        category_id: CategoryId,
        border_color: Color,
    ) {
        let can_add = self.report_layer_can_add(category_id);
        let row_count = logs.len().saturating_add(usize::from(can_add));
        let selected_log_index = (row_count > 0)
            .then(|| self.report_log_selected_index.min(row_count.saturating_sub(1)));
        let is_none_category = category_id == DRIFT_CATEGORY_ID;
        let row_width = list_area.width as usize;
        let show_date_column = {
            let window = self.current_report_window();
            window.start != window.end
        };
        let text_color = crate::appearance::contrasting_text_color(
            border_color,
            self.theme_foreground(),
        );

        let mut items = logs
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let is_selected = selected_log_index == Some(idx);
                let editing_this_row = self
                    .ledger_entry_edit
                    .as_ref()
                    .is_some_and(|edit| edit.session_id() == row.session_id);
                let line = if editing_this_row {
                    self.ledger_edit_line(row_width, show_date_column, text_color)
                } else {
                    self.ledger_normal_line(
                        row,
                        row_width,
                        show_date_column,
                        is_selected.then_some(text_color),
                        is_none_category,
                    )
                };
                if is_selected {
                    ListItem::new(line).style(Style::default().fg(text_color).bg(border_color))
                } else {
                    ListItem::new(line)
                }
            })
            .collect::<Vec<_>>();

        if can_add {
            let add_index = logs.len();
            let is_selected = selected_log_index == Some(add_index);
            let editing_add = self
                .ledger_entry_edit
                .as_ref()
                .is_some_and(|edit| edit.is_add());
            let line = if editing_add {
                self.ledger_edit_line(row_width, show_date_column, text_color)
            } else {
                let label = "+ Add entry…";
                let padded = format!(
                    "{label}{}",
                    " ".repeat(row_width.saturating_sub(label.chars().count()))
                );
                Line::from(Span::styled(
                    padded,
                    Style::default().fg(if is_selected {
                        text_color
                    } else {
                        self.theme_status()
                    }),
                ))
            };
            if is_selected {
                items.push(
                    ListItem::new(line).style(Style::default().fg(text_color).bg(border_color)),
                );
            } else {
                items.push(ListItem::new(line));
            }
        }

        if items.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "No entries for this layer in this period.",
                Style::default().fg(self.theme_status()),
            ))));
        }

        let mut list_state = ListState::default();
        list_state.select(selected_log_index);
        f.render_stateful_widget(List::new(items), list_area, &mut list_state);
    }

    fn render_report_summary_with_instrument(
        &self,
        f: &mut Frame,
        area: Rect,
        summary: &BalanceReportSummary,
        selected_summary_index: Option<usize>,
    ) {
        let (totals_area, meter_area, list_area) = if area.height >= 6 {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(area);
            (Some(rows[1]), Some(rows[2]), rows[4])
        } else if area.height >= 5 {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            (Some(rows[1]), Some(rows[2]), rows[4])
        } else if area.height >= 3 {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            (Some(rows[0]), Some(rows[1]), rows[2])
        } else {
            (None, None, area)
        };

        if let (Some(totals_area), Some(meter_area)) = (totals_area, meter_area) {
            self.render_balance_instrument(f, totals_area, meter_area, summary);
        }

        self.render_report_summary_view(f, list_area, summary, selected_summary_index);
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
                    self.theme_status()
                } else if entry.balance_seconds == 0 {
                    if entry.balance_effect < 0 {
                        self.theme_error()
                    } else if entry.balance_effect > 0 {
                        self.theme_success()
                    } else {
                        self.theme_status()
                    }
                } else {
                    view_style::balance_color(
                        entry.balance_seconds,
                        self.theme_error(),
                        self.theme_success(),
                        self.theme_status(),
                    )
                };

                if is_selected {
                    let text_color = crate::appearance::contrasting_text_color(
                        entry.color,
                        self.theme_foreground(),
                    );
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
                        Span::raw(name).fg(self.theme_foreground()),
                        Span::raw(" ".repeat(pad)).fg(self.theme_foreground()),
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
                Style::default().fg(self.theme_status()),
            )]))])
        } else {
            List::new(items)
        };

        f.render_stateful_widget(list, list_area, &mut list_state);
    }
}

#[cfg(test)]
mod hardening_tests {
    use ratatui::style::Color;

    use super::{App, summary_border_color};
    use crate::domain::{BalanceReportEntry, BalanceReportSummary, CategoryId};

    #[test]
    fn ledger_data_regions_split_evenly_for_three_and_four_bit_rows() {
        assert_eq!(App::ledger_weighted_widths(60, &[1, 1, 1]), vec![20, 20, 20]);
        assert_eq!(App::ledger_weighted_widths(60, &[1, 1, 1, 1]), vec![15, 15, 15, 15]);
        assert_eq!(App::ledger_weighted_widths(60, &[1, 2, 1]), vec![15, 30, 15]);
    }

    #[test]
    fn summary_frame_follows_selected_layer_color_with_safe_fallback() {
        let summary = BalanceReportSummary {
            date: "2026-09-27".to_string(),
            entries: vec![
                BalanceReportEntry {
                    category_id: CategoryId::new(1),
                    category_name: "first".to_string(),
                    color: Color::Green,
                    elapsed_seconds: 1,
                    balance_effect: 1,
                    balance_seconds: 1,
                },
                BalanceReportEntry {
                    category_id: CategoryId::new(2),
                    category_name: "idle".to_string(),
                    color: Color::White,
                    elapsed_seconds: 1,
                    balance_effect: 0,
                    balance_seconds: 0,
                },
            ],
            total_seconds: 2,
            total_balance_seconds: 1,
        };

        assert_eq!(
            summary_border_color(&summary, Some(0), Color::Blue),
            Color::Green
        );
        assert_eq!(
            summary_border_color(&summary, Some(1), Color::Blue),
            Color::White
        );
        assert_eq!(
            summary_border_color(&summary, None, Color::Blue),
            Color::Blue
        );
    }
}
