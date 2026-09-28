use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
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
    App, LedgerEntryEditState, LedgerEntryField, ReportRangeBoundary, balance_instrument,
    overlay_layout, ui_helpers, view_style,
};

const LEDGER_INLINE_EDITOR_MIN_WIDTH: usize = 84;

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

#[derive(Clone, Copy)]
struct LedgerRowPresentation<'a> {
    selected_text: Option<Color>,
    filter_focus: Option<usize>,
    is_none_category: bool,
    balance_effect: i8,
    marker_color: Color,
    fallback_tag: &'a str,
    deemphasized: bool,
}

fn balance_marker(balance_effect: i8) -> &'static str {
    if balance_effect < 0 { "◯" } else { "●" }
}

fn ledger_edit_effective_end_date(edit: &LedgerEntryEditState) -> String {
    if edit.dates_linked
        && let (Ok(start), Ok(end), Ok(date)) = (
            super::ledger_state::parse_ledger_time_input(&edit.start_time).ok_or(()),
            super::ledger_state::parse_ledger_time_input(&edit.end_time).ok_or(()),
            NaiveDate::parse_from_str(&edit.start_date, "%Y-%m-%d"),
        )
        && end < start
    {
        date.checked_add_signed(ChronoDuration::days(1))
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| edit.end_date.clone())
    } else if edit.dates_linked {
        edit.start_date.clone()
    } else {
        edit.end_date.clone()
    }
}

fn ledger_edit_token(value: &str, active: bool) -> String {
    if !active {
        return value.to_string();
    }
    if value.is_empty() {
        "[        ]".to_string()
    } else {
        format!("[{value}]")
    }
}

fn ledger_edit_time_token(value: &str, active: bool, select_all: bool) -> String {
    let display = if active && !select_all {
        value.to_string()
    } else {
        super::ledger_state::format_ledger_time_input(value)
    };
    ledger_edit_token(&display, active)
}

fn ledger_display_tag(description: &str, fallback_tag: &str) -> String {
    let description = description.trim();
    if description.is_empty() {
        fallback_tag.to_string()
    } else {
        description.to_string()
    }
}

fn ledger_display_tag_with_focus(
    description: &str,
    fallback_tag: &str,
    focus: Option<usize>,
) -> String {
    let Some(focus) = focus else {
        return ledger_display_tag(description, fallback_tag);
    };
    let tags = super::tagging::parse_tags(description);
    if tags.is_empty() {
        return format!("[{fallback_tag}]");
    }
    tags.into_iter()
        .enumerate()
        .map(|(index, tag)| {
            if index == focus {
                format!("[{tag}]")
            } else {
                tag
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn tag_completion_suffix(completion: &super::tagging::TagCompletion) -> String {
    completion
        .tag
        .chars()
        .skip(completion.typed_chars)
        .collect()
}

impl App {
    pub(super) fn render_report_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let summary = self.report_rows();
        let logs_for_view = self
            .report_logs_category_id
            .map(|category_id| self.report_logs_for_category(category_id));

        let default_summary =
            self.report_logs_category_id.is_none() && self.report_range_edit.is_none();
        let layer_detail = self.report_logs_category_id.is_some();
        let mut body_row_count = if layer_detail {
            self.report_ledger_row_count()
        } else {
            logs_for_view
                .as_ref()
                .map_or(summary.entries.len(), |logs| logs.len())
        };
        if layer_detail {
            if logs_for_view.as_ref().is_some_and(|logs| !logs.is_empty())
                && self
                    .report_logs_category_id
                    .is_some_and(|category_id| self.report_layer_can_add(category_id))
            {
                body_row_count = body_row_count.saturating_add(1);
            }
            body_row_count = body_row_count.saturating_add(self.ledger_confirmation_extra_height());
        }

        let preferred_inner_width = self
            .preferred_report_inner_width(&summary, logs_for_view.as_deref())
            .max(if self.report_range_edit.is_some() {
                if self.report_logs_category_id.is_some() {
                    REPORT_MODAL_SETTINGS
                        .range_editor_min_width
                        .saturating_add(12)
                } else {
                    REPORT_MODAL_SETTINGS.range_editor_min_width
                }
            } else if self.ledger_entry_edit.is_some() {
                LEDGER_INLINE_EDITOR_MIN_WIDTH
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
        let selected_summary_index = self.report_selected_summary_index(&summary);
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
        let interaction_bottom_title = if let Some(edit) = self.report_range_edit.as_ref() {
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

        if default_summary || (layer_detail && self.report_range_edit.is_none()) {
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
            let fallback_tag = self
                .report_logs_category_id
                .map(|category_id| self.report_layer_display_name(category_id))
                .unwrap_or_default();
            let needs_expanded_detail = logs.iter().any(|row| {
                let display_tag = if row.description.trim().is_empty() {
                    fallback_tag.as_str()
                } else {
                    row.description.trim()
                };
                row.date != row.end_date
                    || display_tag.chars().count()
                        > REPORT_MODAL_SETTINGS.log_detail_compact_max_width / 2
            });
            let detail_max_width = if needs_expanded_detail {
                REPORT_MODAL_SETTINGS.log_detail_max_width
            } else {
                REPORT_MODAL_SETTINGS.log_detail_compact_max_width
            };
            let max_log_detail = logs
                .iter()
                .map(|row| {
                    if row.description.trim().is_empty() {
                        format!(
                            "{} · {} {}-{} {}",
                            fallback_tag, row.date, row.start_time, row.end_date, row.end_time
                        )
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
                .min(detail_max_width);
            let filter_detail = self
                .report_filter_label()
                .map(|label| {
                    "filter  "
                        .chars()
                        .count()
                        .saturating_add(label.chars().count())
                })
                .unwrap_or(0)
                .min(REPORT_MODAL_SETTINGS.log_detail_max_width);
            let max_detail = max_log_detail.max(filter_detail);

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

    fn ledger_confirmation_card_height(&self) -> u16 {
        let row_count = self
            .ledger_entry_edit
            .as_ref()
            .and_then(|edit| edit.confirmation.as_ref())
            .map(|confirmation| {
                confirmation
                    .changes
                    .iter()
                    .map(|change| change.after.len().max(1))
                    .sum::<usize>()
            })
            .unwrap_or(0);
        if row_count == 0 {
            0
        } else {
            row_count.saturating_add(6).min(u16::MAX as usize) as u16
        }
    }

    fn ledger_confirmation_extra_height(&self) -> usize {
        usize::from(self.ledger_confirmation_card_height().saturating_sub(4))
    }

    fn ledger_confirmation_interval(
        &self,
        started_at_utc: DateTime<Utc>,
        ended_at_utc: DateTime<Utc>,
        policy: crate::domain::OperationalDayPolicy,
        show_date: bool,
    ) -> String {
        let Ok(start) = crate::temporal::civil_from_policy(started_at_utc, policy) else {
            return "?".to_string();
        };
        let Ok(end) = crate::temporal::civil_from_policy(ended_at_utc, policy) else {
            return "?".to_string();
        };
        if !show_date && start.date_naive() == end.date_naive() {
            return format!("{}–{}", start.format("%H:%M"), end.format("%H:%M"));
        }
        if start.date_naive() == end.date_naive() {
            return format!(
                "{} · {}–{}",
                start.format("%b %-d"),
                start.format("%H:%M"),
                end.format("%H:%M")
            );
        }
        format!(
            "{} {}–{} {}",
            start.format("%b %-d"),
            start.format("%H:%M"),
            end.format("%b %-d"),
            end.format("%H:%M")
        )
    }

    fn render_ledger_confirmation_card(&self, f: &mut Frame, area: Rect) {
        let Some(confirmation) = self
            .ledger_entry_edit
            .as_ref()
            .and_then(|edit| edit.confirmation.as_ref())
        else {
            return;
        };
        if confirmation.changes.is_empty() || area.width < 4 || area.height < 3 {
            return;
        }

        let desired_width = 62u16.min(area.width);
        let x = area.x + area.width.saturating_sub(desired_width) / 2;
        let card = Rect::new(x, area.y, desired_width, area.height);
        let title = if confirmation.changes.len() == 1 {
            "Another entry will change".to_string()
        } else {
            format!("{} other entries will change", confirmation.changes.len())
        };
        let block = Block::default()
            .title(Line::from(Span::styled(
                title,
                Style::default()
                    .fg(self.theme_warning())
                    .add_modifier(Modifier::BOLD),
            )))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.theme_warning()))
            .style(Style::default().bg(self.theme_background()));
        f.render_widget(block.clone(), card);
        let inner = block.inner(card);
        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let show_date = {
            let window = self.current_report_window();
            window.start != window.end
                || confirmation.changes.iter().any(|change| {
                    let start = crate::temporal::civil_from_policy(
                        change.started_at_utc,
                        change.operational_day_policy,
                    );
                    let end = crate::temporal::civil_from_policy(
                        change.ended_at_utc,
                        change.operational_day_policy,
                    );
                    start.ok().map(|value| value.date_naive())
                        != end.ok().map(|value| value.date_naive())
                })
        };

        let columns_total = usize::from(inner.width);
        let widths = Self::ledger_weighted_widths(columns_total, &[1, 2, 2]);
        let header = Line::from(vec![
            Span::raw(self.ledger_left_cell("", widths[0])),
            Span::styled(
                self.ledger_center_cell("BEFORE", widths[1]),
                Style::default()
                    .fg(self.theme_status())
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                self.ledger_right_cell("AFTER", widths[2]),
                Style::default()
                    .fg(self.theme_status())
                    .add_modifier(Modifier::DIM),
            ),
        ]);

        let mut lines = vec![header, Line::default()];
        for change in &confirmation.changes {
            let name = self.report_layer_display_name(change.category_id);
            let before = self.ledger_confirmation_interval(
                change.started_at_utc,
                change.ended_at_utc,
                change.operational_day_policy,
                show_date,
            );
            let after = if change.after.is_empty() {
                vec!["removed".to_string()]
            } else {
                change
                    .after
                    .iter()
                    .map(|interval| {
                        self.ledger_confirmation_interval(
                            interval.started_at_utc,
                            interval.ended_at_utc,
                            change.operational_day_policy,
                            show_date,
                        )
                    })
                    .collect::<Vec<_>>()
            };
            for (index, after_value) in after.iter().enumerate() {
                let name_value = if index == 0 { name.as_str() } else { "" };
                let before_value = if index == 0 { before.as_str() } else { "" };
                let after_color = if after_value == "removed" {
                    self.theme_error()
                } else {
                    self.theme_foreground()
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        self.ledger_left_cell(name_value, widths[0]),
                        Style::default().fg(self.category_color_for_id(change.category_id)),
                    ),
                    Span::styled(
                        self.ledger_center_cell(before_value, widths[1]),
                        Style::default().fg(self.theme_foreground()),
                    ),
                    Span::styled(
                        self.ledger_right_cell(after_value, widths[2]),
                        Style::default().fg(after_color),
                    ),
                ]));
            }
        }
        lines.push(Line::default());
        lines.push(
            Line::from(vec![
                Span::styled(
                    "Enter apply",
                    Style::default()
                        .fg(self.theme_accent())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" · Esc back", Style::default().fg(self.theme_status())),
            ])
            .alignment(Alignment::Center),
        );
        f.render_widget(Paragraph::new(lines), inner);
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
        if self
            .ledger_entry_edit
            .as_ref()
            .and_then(|edit| edit.confirmation.as_ref())
            .is_some()
        {
            let desired_card_height = self.ledger_confirmation_card_height();
            let card_height = desired_card_height
                .min(area.height.saturating_sub(3))
                .max(3.min(area.height));
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(card_height),
                    Constraint::Length(u16::from(area.height > card_height)),
                    Constraint::Min(0),
                ])
                .split(area);
            self.render_ledger_confirmation_card(f, rows[0]);
            self.render_report_logs_view(f, rows[2], logs, category_id, border_color);
            return;
        }

        let filter_active = self.report_tag_filter_active();
        let (total_area, meter_area, filter_area, list_area) = if area.height >= 6 {
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
            (
                Some(rows[1]),
                Some(rows[2]),
                filter_active.then_some(rows[3]),
                rows[4],
            )
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
            (
                Some(rows[1]),
                Some(rows[2]),
                filter_active.then_some(rows[3]),
                rows[4],
            )
        } else if area.height >= 3 {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(area);
            (Some(rows[0]), Some(rows[1]), None, rows[2])
        } else {
            (None, None, None, area)
        };

        if let (Some(total_area), Some(meter_area)) = (total_area, meter_area) {
            let unfiltered_contribution = summary
                .entries
                .iter()
                .find(|entry| entry.category_id == category_id)
                .map(|entry| entry.balance_seconds)
                .unwrap_or(0);
            let contribution = if self.report_tag_filter_active() {
                self.report_filtered_layer_balance(logs)
            } else {
                unfiltered_contribution
            };
            self.render_layer_influence_instrument(
                f,
                total_area,
                meter_area,
                filter_area,
                summary,
                contribution,
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

    fn ledger_left_cell(&self, text: &str, width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        let text = self.truncate_label(text, width);
        let remaining = width.saturating_sub(text.chars().count());
        format!("{}{}", text, " ".repeat(remaining))
    }

    fn ledger_right_cell(&self, text: &str, width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        let text = self.truncate_label(text, width);
        let remaining = width.saturating_sub(text.chars().count());
        format!("{}{}", " ".repeat(remaining), text)
    }

    fn ledger_date_label(&self, raw: &str) -> String {
        ui_helpers::format_report_interval_label(raw)
    }

    fn ledger_minute_time(raw: &str) -> String {
        raw.get(..5).unwrap_or(raw).to_string()
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
        presentation: LedgerRowPresentation<'_>,
    ) -> Line<'static> {
        let tag = ledger_display_tag_with_focus(
            &row.description,
            presentation.fallback_tag,
            presentation.filter_focus,
        );
        let start_time = Self::ledger_minute_time(&row.start_time);
        let end_time = Self::ledger_minute_time(&row.end_time);
        let metric = self.ledger_metric_value(row, presentation.is_none_category);
        let metric_color = presentation
            .selected_text
            .unwrap_or_else(|| self.ledger_metric_color(row, presentation.is_none_category));
        let tag_color = presentation
            .selected_text
            .unwrap_or_else(|| self.theme_foreground());
        let marker_color = presentation
            .selected_text
            .unwrap_or(presentation.marker_color);
        let temporal_color = presentation
            .selected_text
            .unwrap_or_else(|| self.theme_status());
        let (tag_color, marker_color, temporal_color, metric_color) = if presentation.deemphasized {
            let subdued = self.theme_status();
            (subdued, subdued, subdued, subdued)
        } else {
            (tag_color, marker_color, temporal_color, metric_color)
        };
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
                vec![
                    (tag, tag_color),
                    (temporal, temporal_color),
                    (metric, metric_color),
                ],
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

        let last = values.len().saturating_sub(1);
        let mut spans = Vec::with_capacity(values.len().saturating_add(1));
        for (idx, ((value, color), width)) in values.into_iter().zip(widths).enumerate() {
            if idx == 0 {
                let marker = format!("{} ", balance_marker(presentation.balance_effect));
                let marker_width = marker.chars().count().min(width);
                spans.push(Span::styled(
                    self.truncate_label(&marker, marker_width),
                    Style::default().fg(marker_color),
                ));
                spans.push(Span::styled(
                    self.ledger_left_cell(&value, width.saturating_sub(marker_width)),
                    Style::default().fg(tag_color),
                ));
            } else {
                let aligned = if idx == last {
                    self.ledger_right_cell(&value, width)
                } else {
                    self.ledger_center_cell(&value, width)
                };
                spans.push(Span::styled(aligned, Style::default().fg(color)));
            }
        }
        Line::from(spans)
    }

    fn ledger_edit_line(
        &self,
        edit: &LedgerEntryEditState,
        row_width: usize,
        show_date_column: bool,
        marker: &str,
        fallback_tag: &str,
    ) -> Line<'static> {
        if row_width < 52 {
            let effective_end_date = ledger_edit_effective_end_date(edit);
            if edit.active_field == LedgerEntryField::Description
                && let Some(completion) =
                    self.tag_completion_for_category(edit.category_id, &edit.description)
            {
                let suffix = tag_completion_suffix(&completion);
                let prefix = format!("{marker} Tag [{}", edit.description.trim());
                let full_width = prefix
                    .chars()
                    .count()
                    .saturating_add(suffix.chars().count())
                    .saturating_add(1);
                if full_width <= row_width {
                    let padding = row_width.saturating_sub(full_width);
                    return Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(suffix, Style::default().add_modifier(Modifier::DIM)),
                        Span::raw(format!("]{}", " ".repeat(padding))),
                    ]);
                }
            }
            let active_value = match edit.active_field {
                LedgerEntryField::Description => {
                    format!("Tag {}", ledger_edit_token(edit.description.trim(), true))
                }
                LedgerEntryField::StartDate => {
                    format!("From {}", ledger_edit_token(&edit.start_date, true))
                }
                LedgerEntryField::StartTime => format!(
                    "From {} {}",
                    self.ledger_date_label(&edit.start_date),
                    ledger_edit_time_token(&edit.start_time, true, edit.select_all)
                ),
                LedgerEntryField::EndDate => {
                    format!("To {}", ledger_edit_token(&effective_end_date, true))
                }
                LedgerEntryField::EndTime => format!(
                    "To {} {}",
                    self.ledger_date_label(&effective_end_date),
                    ledger_edit_time_token(&edit.end_time, true, edit.select_all)
                ),
            };
            return Line::from(Span::raw(
                self.ledger_left_cell(&format!("{marker} {active_value}"), row_width),
            ));
        }

        let description = edit.description.trim();
        let tag = if edit.active_field == LedgerEntryField::Description {
            ledger_edit_token(description, true)
        } else {
            ledger_display_tag(description, fallback_tag)
        };

        let effective_end_date = ledger_edit_effective_end_date(edit);
        let cross_day = edit.start_date != effective_end_date;

        let start_date_label = if edit.active_field == LedgerEntryField::StartDate {
            edit.start_date.clone()
        } else {
            self.ledger_date_label(&edit.start_date)
        };
        let start_date = ledger_edit_token(
            &start_date_label,
            edit.active_field == LedgerEntryField::StartDate,
        );
        let end_date_label = if edit.active_field == LedgerEntryField::EndDate {
            effective_end_date.clone()
        } else {
            self.ledger_date_label(&effective_end_date)
        };
        let end_date = if !edit.dates_linked && edit.active_field == LedgerEntryField::EndDate {
            ledger_edit_token(&end_date_label, true)
        } else {
            end_date_label
        };
        let start_time = ledger_edit_time_token(
            &edit.start_time,
            edit.active_field == LedgerEntryField::StartTime,
            edit.select_all,
        );
        let end_time = ledger_edit_time_token(
            &edit.end_time,
            edit.active_field == LedgerEntryField::EndTime,
            edit.select_all,
        );

        let (widths, values): (Vec<usize>, Vec<String>) = if cross_day {
            (
                Self::ledger_weighted_widths(row_width, &[1, 2, 1]),
                vec![
                    tag,
                    format!("{start_date} {start_time} → {end_date} {end_time}"),
                    String::new(),
                ],
            )
        } else if show_date_column {
            (
                Self::ledger_weighted_widths(row_width, &[1, 1, 1, 1]),
                vec![
                    tag,
                    start_date,
                    format!("{start_time}–{end_time}"),
                    String::new(),
                ],
            )
        } else {
            (
                Self::ledger_weighted_widths(row_width, &[1, 1, 1]),
                vec![tag, format!("{start_time}–{end_time}"), String::new()],
            )
        };

        let last = values.len().saturating_sub(1);
        let mut spans = Vec::with_capacity(values.len().saturating_add(1));
        for (idx, (value, width)) in values.into_iter().zip(widths).enumerate() {
            if idx == 0 {
                let marker = format!("{marker} ");
                let marker_width = marker.chars().count().min(width);
                spans.push(Span::raw(self.truncate_label(&marker, marker_width)));
                let value_width = width.saturating_sub(marker_width);
                if edit.active_field == LedgerEntryField::Description
                    && let Some(completion) =
                        self.tag_completion_for_category(edit.category_id, &edit.description)
                {
                    let suffix = tag_completion_suffix(&completion);
                    let prefix = format!("[{}", edit.description.trim());
                    let autocomplete_width = prefix
                        .chars()
                        .count()
                        .saturating_add(suffix.chars().count())
                        .saturating_add(1);
                    if autocomplete_width <= value_width {
                        spans.push(Span::raw(prefix));
                        spans.push(Span::styled(
                            suffix,
                            Style::default().add_modifier(Modifier::DIM),
                        ));
                        spans.push(Span::raw(format!(
                            "]{}",
                            " ".repeat(value_width.saturating_sub(autocomplete_width))
                        )));
                        continue;
                    }
                }
                spans.push(Span::raw(self.ledger_left_cell(&value, value_width)));
            } else {
                let aligned = if idx == last {
                    self.ledger_right_cell(&value, width)
                } else {
                    self.ledger_center_cell(&value, width)
                };
                spans.push(Span::raw(aligned));
            }
        }
        Line::from(spans)
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
        let selected_log_index = (row_count > 0).then(|| {
            self.report_log_selected_index
                .min(row_count.saturating_sub(1))
        });
        let is_none_category = category_id == DRIFT_CATEGORY_ID;
        let layer_display_name = self.report_layer_display_name(category_id);
        let balance_effect = self
            .report_rows()
            .entries
            .iter()
            .find(|entry| entry.category_id == category_id)
            .map(|entry| entry.balance_effect)
            .unwrap_or(0);
        let row_width = list_area.width as usize;
        let show_date_column = {
            let window = self.current_report_window();
            window.start != window.end
        };
        let text_color =
            crate::appearance::contrasting_text_color(border_color, self.theme_foreground());

        let mut items = logs
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let is_selected = selected_log_index == Some(idx);
                let editing_this_row = self
                    .ledger_entry_edit
                    .as_ref()
                    .is_some_and(|edit| edit.session_id() == row.session_id);
                let filtered_out = self.report_tag_filter_active()
                    && !editing_this_row
                    && !self.report_log_matches_filter(row);
                let item = if editing_this_row {
                    ListItem::new(
                        self.ledger_edit_line(
                            self.ledger_entry_edit
                                .as_ref()
                                .expect("editing row has ledger edit state"),
                            row_width,
                            show_date_column,
                            balance_marker(balance_effect),
                            &layer_display_name,
                        ),
                    )
                } else {
                    ListItem::new(self.ledger_normal_line(
                        row,
                        row_width,
                        show_date_column,
                        LedgerRowPresentation {
                            selected_text: is_selected.then_some(text_color),
                            filter_focus: self.report_filter_focus_for_row(idx),
                            is_none_category,
                            balance_effect,
                            marker_color: border_color,
                            fallback_tag: &layer_display_name,
                            deemphasized: filtered_out && !is_selected,
                        },
                    ))
                };
                if is_selected {
                    let style = Style::default().fg(text_color).bg(border_color);
                    item.style(if filtered_out {
                        style.add_modifier(Modifier::DIM)
                    } else {
                        style
                    })
                } else if filtered_out {
                    item.style(Style::default().add_modifier(Modifier::DIM))
                } else {
                    item
                }
            })
            .collect::<Vec<_>>();

        let add_separator =
            can_add && !logs.is_empty() && usize::from(list_area.height) > row_count;
        if can_add {
            if add_separator {
                items.push(ListItem::new(Line::default()));
            }
            let add_index = logs.len();
            let is_selected = selected_log_index == Some(add_index);
            let editing_add = self
                .ledger_entry_edit
                .as_ref()
                .is_some_and(|edit| edit.is_add());
            let item = if editing_add {
                ListItem::new(
                    self.ledger_edit_line(
                        self.ledger_entry_edit
                            .as_ref()
                            .expect("adding row has ledger edit state"),
                        row_width,
                        show_date_column,
                        "+",
                        &layer_display_name,
                    ),
                )
            } else {
                let label = "+ Add entry…";
                let padded = self.ledger_left_cell(label, row_width);
                ListItem::new(Line::from(Span::styled(
                    padded,
                    Style::default().fg(if is_selected {
                        text_color
                    } else {
                        self.theme_status()
                    }),
                )))
            };
            if is_selected {
                items.push(item.style(Style::default().fg(text_color).bg(border_color)));
            } else {
                items.push(item);
            }
        }

        if items.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "No entries for this layer in this period.",
                Style::default().fg(self.theme_status()),
            ))));
        }

        let mut list_state = ListState::default();
        let visual_selected_index = selected_log_index.map(|index| {
            if add_separator && index == logs.len() {
                index.saturating_add(1)
            } else {
                index
            }
        });
        list_state.select(visual_selected_index);
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
                let dot = format!("{} ", balance_marker(entry.balance_effect));
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

    use super::super::{LedgerEntryEditState, LedgerEntryField};
    use super::{
        App, balance_marker, ledger_display_tag, ledger_edit_effective_end_date, ledger_edit_token,
        summary_border_color,
    };
    use crate::domain::{BalanceReportEntry, BalanceReportSummary, CategoryId};

    #[test]
    fn ledger_data_regions_split_evenly_for_three_and_four_bit_rows() {
        assert_eq!(
            App::ledger_weighted_widths(60, &[1, 1, 1]),
            vec![20, 20, 20]
        );
        assert_eq!(
            App::ledger_weighted_widths(60, &[1, 1, 1, 1]),
            vec![15, 15, 15, 15]
        );
        assert_eq!(
            App::ledger_weighted_widths(60, &[1, 2, 1]),
            vec![15, 30, 15]
        );
    }

    #[test]
    fn balance_marker_matches_layer_modal_and_keeps_idle_filled() {
        assert_eq!(balance_marker(-1), "◯");
        assert_eq!(balance_marker(0), "●");
        assert_eq!(balance_marker(1), "●");
    }

    #[test]
    fn empty_ledger_tag_falls_back_to_layer_name_without_persisting_it() {
        assert_eq!(ledger_display_tag("", "Cinema"), "Cinema");
        assert_eq!(ledger_display_tag("   ", "Cinema"), "Cinema");
        assert_eq!(ledger_display_tag("Anibal", "Cinema"), "Anibal");
    }

    #[test]
    fn ledger_edit_tokens_keep_overnight_end_date_and_active_field_visible() {
        let mut edit = LedgerEntryEditState::existing(
            7,
            CategoryId::new(2),
            "UX detail smoke".to_string(),
            "2026-09-27".to_string(),
            "23:40:00".to_string(),
            "2026-09-27".to_string(),
            "00:20:00".to_string(),
        );
        edit.active_field = LedgerEntryField::EndTime;

        assert_eq!(ledger_edit_effective_end_date(&edit), "2026-09-28");
        assert_eq!(
            ledger_edit_token("UX detail smoke", false),
            "UX detail smoke"
        );
        assert_eq!(ledger_edit_token("00:20:00", true), "[00:20:00]");
        assert_eq!(ledger_edit_token("", true), "[        ]");
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
