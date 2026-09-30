use chrono::{DateTime, Utc};
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

const CONFIRMATION_CARD_WIDTH: u16 = 62;
const EMPTY_TAG_DISPLAY: &str = "—";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LedgerColumnWidths {
    tag: usize,
    date: usize,
    time: usize,
    metric: usize,
    show_date: bool,
}

impl LedgerColumnWidths {
    #[allow(dead_code)]
    fn total(self) -> usize {
        self.tag
            .saturating_add(self.date)
            .saturating_add(self.time)
            .saturating_add(self.metric)
            .saturating_add(if self.show_date { 3 } else { 2 })
    }

    fn required_proportional_total(self) -> usize {
        // Alignment owns geometry. Three-cell rows are outer/middle/outer so Time
        // stays exactly centered; four-cell rows are outer/inner/inner/outer so
        // Date and Time remain a symmetric inner pair. Content only grows its
        // mirrored region instead of forcing every cell to the widest value.
        let outer = self.tag.max(self.metric);
        if self.show_date {
            let inner = self.date.max(self.time);
            outer
                .saturating_mul(2)
                .saturating_add(inner.saturating_mul(2))
                .saturating_add(3)
        } else {
            outer
                .saturating_mul(2)
                .saturating_add(self.time)
                .saturating_add(2)
        }
    }

    fn fit(self, row_width: usize) -> Self {
        let gaps = if self.show_date { 3 } else { 2 };
        let available = row_width.saturating_sub(gaps);
        if self.show_date {
            let natural_outer = self.tag.max(self.metric);
            let natural_inner = self.date.max(self.time);
            let natural = natural_outer
                .saturating_mul(2)
                .saturating_add(natural_inner.saturating_mul(2));
            let (outer, inner) = if available >= natural {
                let pair_surplus = (available - natural) / 2;
                let outer_extra = pair_surplus / 2;
                let inner_extra = pair_surplus.saturating_sub(outer_extra);
                (
                    natural_outer.saturating_add(outer_extra),
                    natural_inner.saturating_add(inner_extra),
                )
            } else {
                let pair_budget = available / 2;
                let natural_pair = natural_outer.saturating_add(natural_inner).max(1);
                let outer = pair_budget
                    .saturating_mul(natural_outer)
                    .checked_div(natural_pair)
                    .unwrap_or(0);
                (outer, pair_budget.saturating_sub(outer))
            };
            Self {
                tag: outer,
                date: inner,
                time: inner,
                metric: outer,
                show_date: true,
            }
        } else {
            let natural_outer = self.tag.max(self.metric);
            let natural = natural_outer.saturating_mul(2).saturating_add(self.time);
            let (outer, middle) = if available >= natural {
                let surplus = available - natural;
                (
                    natural_outer.saturating_add(surplus / 2),
                    self.time.saturating_add(surplus % 2),
                )
            } else {
                let middle = self.time.min(available);
                ((available.saturating_sub(middle)) / 2, middle)
            };
            Self {
                tag: outer,
                date: 0,
                time: middle,
                metric: outer,
                show_date: false,
            }
        }
    }
}

fn balance_marker(balance_effect: i8) -> &'static str {
    if balance_effect < 0 { "◯" } else { "●" }
}

fn ledger_date_span_label(start: &str, end: &str) -> String {
    if start == end {
        ui_helpers::format_report_interval_label(start)
    } else {
        ui_helpers::format_report_interval_label(&format!("{start}..{end}"))
    }
}

fn ledger_time_span_label(start: &str, end: &str) -> String {
    format!(
        "{}–{}",
        start.get(..5).unwrap_or(start),
        end.get(..5).unwrap_or(end)
    )
}

fn ledger_edit_effective_end_date(edit: &LedgerEntryEditState) -> String {
    edit.preview_naive_bounds()
        .map(|(_, end)| end.date().format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| {
            if edit.dates_linked {
                edit.start_date.clone()
            } else {
                edit.end_date.clone()
            }
        })
}

fn ledger_display_tag(description: &str, _fallback_tag: &str) -> String {
    let description = description.trim();
    if description.is_empty() {
        EMPTY_TAG_DISPLAY.to_string()
    } else {
        description.to_string()
    }
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
        let layer_detail = self.report_logs_category_id.is_some();
        let summary = if layer_detail {
            self.report_rows()
        } else {
            self.report_visible_rows()
        };
        let logs_for_view = self
            .report_logs_category_id
            .map(|category_id| self.report_logs_for_category(category_id));

        let default_summary =
            self.report_logs_category_id.is_none() && self.report_range_edit.is_none();
        let mut body_row_count = if layer_detail {
            self.report_ledger_row_count()
        } else {
            logs_for_view
                .as_ref()
                .map_or(summary.entries.len(), |logs| logs.len())
        };
        if layer_detail
            && logs_for_view.as_ref().is_some_and(|logs| !logs.is_empty())
            && self
                .report_logs_category_id
                .is_some_and(|category_id| self.report_layer_can_add(category_id))
        {
            body_row_count = body_row_count.saturating_add(1);
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
        let selected_boundary_style = Style::default()
            .fg(self.theme_background())
            .bg(self.theme_foreground())
            .add_modifier(Modifier::BOLD);
        let period_bottom_title = Line::from(vec![
            Span::styled("← ", Style::default().fg(self.theme_status())),
            Span::styled(
                interval_start,
                if start_selected {
                    selected_boundary_style
                } else {
                    Style::default().fg(self.theme_foreground())
                },
            ),
            Span::styled(" – ", Style::default().fg(self.theme_foreground())),
            Span::styled(
                interval_end,
                if end_selected {
                    selected_boundary_style
                } else {
                    Style::default().fg(self.theme_foreground())
                },
            ),
            Span::styled(" →", newer_chevron_style),
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

        // System dialogs are rendered last so they are true overlays over the
        // current surface, including over Balance/Layer Detail itself.
        if self.report_layer_delete_confirmation.is_some() {
            self.render_report_layer_delete_confirmation(f, terminal_size);
        } else if self.report_entry_delete_confirmation.is_some() {
            self.render_report_entry_delete_confirmation(f, terminal_size);
        } else if self
            .ledger_entry_edit
            .as_ref()
            .and_then(|edit| edit.confirmation.as_ref())
            .is_some()
        {
            self.render_ledger_confirmation_dialog(f, terminal_size);
        }
    }

    fn preferred_report_inner_width(
        &self,
        summary: &BalanceReportSummary,
        logs_for_view: Option<&[CategoryLogEntry]>,
    ) -> usize {
        if let Some(logs) = logs_for_view {
            // Geometry is owned by the ledger values themselves. Transient chrome
            // such as filter labels/focus brackets must never resize the overlay.
            self.preferred_ledger_columns(logs)
                .required_proportional_total()
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

    fn preferred_ledger_columns(&self, logs: &[CategoryLogEntry]) -> LedgerColumnWidths {
        let category_id = self.report_logs_category_id.unwrap_or(DRIFT_CATEGORY_ID);
        let fallback_tag = self.report_layer_display_name(category_id);
        let is_none = category_id == DRIFT_CATEGORY_ID;
        let report_window = self.current_report_window();
        let mut show_date = report_window.start != report_window.end;
        let mut tag_width = EMPTY_TAG_DISPLAY.chars().count().saturating_add(2);
        let mut date_width = 0;
        let mut time_width = 11;
        let mut metric_width = if is_none {
            REPORT_MODAL_SETTINGS.detail_metric_width_drift
        } else {
            REPORT_MODAL_SETTINGS.detail_metric_width_default
        };

        for row in logs {
            // Geometry is measured only from durable/display data. Focus, cursor,
            // autocomplete, and editor chrome are deliberately absent here.
            let tag = ledger_display_tag(&row.description, &fallback_tag);
            tag_width = tag_width.max(tag.chars().count().saturating_add(2));
            show_date |= row.date != row.end_date;
            let date = ledger_date_span_label(&row.date, &row.end_date);
            date_width = date_width.max(date.chars().count());
            let time = if row.active_stable_id.is_some() {
                format!("{}–now", row.start_time.get(..5).unwrap_or(&row.start_time))
            } else {
                ledger_time_span_label(&row.start_time, &row.end_time)
            };
            time_width = time_width.max(time.chars().count());
            metric_width = metric_width.max(self.ledger_metric_value(row, is_none).chars().count());
        }

        if !show_date {
            date_width = 0;
        }
        LedgerColumnWidths {
            tag: tag_width,
            date: date_width,
            time: time_width,
            metric: metric_width,
            show_date,
        }
    }

    fn ledger_edit_columns(
        &self,
        edit: &LedgerEntryEditState,
        base: LedgerColumnWidths,
        row_width: usize,
    ) -> LedgerColumnWidths {
        if base.show_date {
            return base.fit(row_width);
        }
        // A single-day ledger stays three-cell in ordinary presentation. Only the
        // active edit row exposes Date, using a symmetric four-cell projection
        // inside the already-established modal width.
        let fallback_tag = self.report_layer_display_name(edit.category_id);
        let tag = ledger_display_tag(edit.description.trim(), &fallback_tag);
        let end_date = ledger_edit_effective_end_date(edit);
        let date = ledger_date_span_label(&edit.start_date, &end_date);
        let start_time = super::ledger_state::format_ledger_time_input(&edit.start_time);
        let time = if edit.is_active() {
            format!("{}–now", start_time)
        } else {
            let end_time = super::ledger_state::format_ledger_time_input(&edit.end_time);
            format!("{start_time}–{end_time}")
        };
        LedgerColumnWidths {
            tag: base.tag.max(tag.chars().count().saturating_add(2)),
            date: date.chars().count(),
            time: base.time.max(time.chars().count()),
            metric: base.metric,
            show_date: true,
        }
        .fit(row_width)
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

    fn render_ledger_confirmation_dialog(&self, f: &mut Frame, terminal: Rect) {
        let Some(confirmation) = self
            .ledger_entry_edit
            .as_ref()
            .and_then(|edit| edit.confirmation.as_ref())
        else {
            return;
        };
        if confirmation.changes.is_empty() {
            return;
        }
        let subtitle = if confirmation.changes.len() == 1 {
            "Another entry will change".to_string()
        } else {
            format!("{} other entries will change", confirmation.changes.len())
        };
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
        let mut body = Vec::new();
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
                let mut spans = Vec::new();
                if index == 0 {
                    spans.push(Span::styled(
                        name.clone(),
                        Style::default().fg(self.category_color_for_id(change.category_id)),
                    ));
                    spans.push(Span::raw("  "));
                    spans.push(Span::raw(before.clone()));
                    spans.push(Span::raw("  →  "));
                } else {
                    spans.push(Span::raw("→  "));
                }
                spans.push(Span::styled(
                    after_value.clone(),
                    Style::default().fg(if after_value == "removed" {
                        self.theme_warning()
                    } else {
                        self.theme_foreground()
                    }),
                ));
                body.push(Line::from(spans));
            }
        }
        self.render_system_dialog(
            f,
            terminal,
            super::SystemDialogSeverity::Warning,
            Line::from(Span::styled(
                subtitle,
                Style::default().add_modifier(Modifier::BOLD),
            )),
            body,
            vec![Line::from("Apply change"), Line::from("Go back")],
            self.system_dialog_selected_index,
            CONFIRMATION_CARD_WIDTH,
        );
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn ledger_date_label(&self, raw: &str) -> String {
        ui_helpers::format_report_interval_label(raw)
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

    fn ledger_tag_cell_spans(
        &self,
        description: &str,
        fallback_tag: &str,
        focus: Option<usize>,
        width: usize,
        color: Color,
    ) -> Vec<Span<'static>> {
        if width == 0 {
            return Vec::new();
        }
        let Some(focus) = focus else {
            return vec![Span::styled(
                self.ledger_left_cell(&ledger_display_tag(description, fallback_tag), width),
                Style::default().fg(color),
            )];
        };

        let tags = super::tagging::parse_tags(description);
        let values = if tags.is_empty() {
            vec![EMPTY_TAG_DISPLAY.to_string()]
        } else {
            tags
        };
        let mut spans = Vec::new();
        let mut remaining = width;
        for (index, value) in values.iter().enumerate() {
            if index > 0 && remaining > 0 {
                let separator = self.truncate_label("; ", remaining);
                remaining = remaining.saturating_sub(separator.chars().count());
                spans.push(Span::styled(separator, Style::default().fg(color)));
            }
            if remaining == 0 {
                break;
            }
            let visible = self.truncate_label(value, remaining);
            remaining = remaining.saturating_sub(visible.chars().count());
            let style = if index == focus {
                Style::default().fg(color).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(color)
            };
            spans.push(Span::styled(visible, style));
        }
        if remaining > 0 {
            spans.push(Span::styled(
                " ".repeat(remaining),
                Style::default().fg(color),
            ));
        }
        spans
    }

    fn ledger_normal_line(
        &self,
        row: &CategoryLogEntry,
        widths: LedgerColumnWidths,
        presentation: LedgerRowPresentation<'_>,
    ) -> Line<'static> {
        let time = if row.active_stable_id.is_some() {
            format!("{}–now", row.start_time.get(..5).unwrap_or(&row.start_time))
        } else {
            ledger_time_span_label(&row.start_time, &row.end_time)
        };
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

        let marker = format!("{} ", balance_marker(presentation.balance_effect));
        let marker_width = marker.chars().count().min(widths.tag);
        let value_width = widths.tag.saturating_sub(marker_width);
        let mut spans = vec![Span::styled(
            self.truncate_label(&marker, marker_width),
            Style::default().fg(marker_color),
        )];
        spans.extend(self.ledger_tag_cell_spans(
            &row.description,
            presentation.fallback_tag,
            presentation.filter_focus,
            value_width,
            tag_color,
        ));

        if widths.show_date {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                self.ledger_center_cell(
                    &ledger_date_span_label(&row.date, &row.end_date),
                    widths.date,
                ),
                Style::default().fg(temporal_color),
            ));
        }
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            self.ledger_center_cell(&time, widths.time),
            Style::default().fg(temporal_color),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            self.ledger_right_cell(&metric, widths.metric),
            Style::default().fg(metric_color),
        ));
        Line::from(spans)
    }

    fn ledger_edit_metric(
        &self,
        edit: &LedgerEntryEditState,
        is_none_category: bool,
    ) -> (String, Color) {
        let Some((elapsed, balance)) = self.ledger_edit_report_contribution(edit) else {
            return ("—".to_string(), self.theme_status());
        };
        if is_none_category {
            return (self.format_time(elapsed), self.theme_status());
        }
        let value = if balance == 0 {
            let effect = self
                .report_rows()
                .entries
                .iter()
                .find(|entry| entry.category_id == edit.category_id)
                .map(|entry| entry.balance_effect)
                .unwrap_or(0);
            if effect < 0 {
                "-00:00:00".to_string()
            } else {
                self.format_balance_time(balance)
            }
        } else {
            self.format_balance_time(balance)
        };
        (
            value,
            view_style::balance_color(
                balance,
                self.theme_error(),
                self.theme_success(),
                self.theme_status(),
            ),
        )
    }

    fn ledger_edit_line(
        &self,
        edit: &LedgerEntryEditState,
        widths: LedgerColumnWidths,
        marker: &str,
        fallback_tag: &str,
    ) -> Line<'static> {
        let description = edit.description.trim();
        let tag = if edit.active_field == LedgerEntryField::Description {
            description.to_string()
        } else {
            ledger_display_tag(description, fallback_tag)
        };
        let effective_end_date = ledger_edit_effective_end_date(edit);
        let date_value = match edit.active_field {
            LedgerEntryField::StartDate => edit.start_date.clone(),
            LedgerEntryField::EndDate => effective_end_date.clone(),
            _ => ledger_date_span_label(&edit.start_date, &effective_end_date),
        };
        let start_time = if edit.active_field == LedgerEntryField::StartTime {
            edit.start_time.clone()
        } else {
            super::ledger_state::format_ledger_time_input(&edit.start_time)
        };
        let time_value = if edit.is_active() {
            format!("{start_time}–now")
        } else {
            let end_time = if edit.active_field == LedgerEntryField::EndTime {
                edit.end_time.clone()
            } else {
                super::ledger_state::format_ledger_time_input(&edit.end_time)
            };
            format!("{start_time}–{end_time}")
        };
        let is_none = edit.category_id == DRIFT_CATEGORY_ID;
        let (metric, metric_color) = self.ledger_edit_metric(edit, is_none);

        let marker_text = format!("{marker} ");
        let marker_width = marker_text.chars().count().min(widths.tag);
        let value_width = widths.tag.saturating_sub(marker_width);
        let mut spans = vec![Span::styled(
            self.truncate_label(&marker_text, marker_width),
            Style::default().fg(self.category_color_for_id(edit.category_id)),
        )];
        if edit.active_field == LedgerEntryField::Description {
            let visible = self.truncate_label(&tag, value_width);
            let used = visible.chars().count();
            spans.push(Span::styled(
                visible,
                Style::default().fg(self.theme_foreground()),
            ));
            let mut remaining = value_width.saturating_sub(used);
            if remaining > 0
                && let Some(completion) =
                    self.tag_completion_for_category(edit.category_id, &edit.description)
            {
                let suffix = self.truncate_label(&tag_completion_suffix(&completion), remaining);
                remaining = remaining.saturating_sub(suffix.chars().count());
                spans.push(Span::styled(
                    suffix,
                    Style::default()
                        .fg(self.theme_status())
                        .add_modifier(Modifier::DIM),
                ));
            }
            if remaining > 0 {
                spans.push(Span::raw(" ".repeat(remaining)));
            }
        } else {
            spans.push(Span::styled(
                self.ledger_left_cell(&tag, value_width),
                Style::default().fg(self.theme_foreground()),
            ));
        }

        if widths.show_date {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                self.ledger_center_cell(&date_value, widths.date),
                Style::default().fg(self.theme_status()),
            ));
        }
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            self.ledger_center_cell(&time_value, widths.time),
            Style::default().fg(self.theme_status()),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            self.ledger_right_cell(&metric, widths.metric),
            Style::default().fg(metric_color),
        ));
        Line::from(spans)
    }

    fn ledger_edit_cursor_column(
        &self,
        edit: &LedgerEntryEditState,
        widths: LedgerColumnWidths,
        marker: &str,
    ) -> usize {
        let marker_width = format!("{marker} ").chars().count().min(widths.tag);
        if edit.active_field == LedgerEntryField::Description {
            return marker_width
                .saturating_add(edit.caret.min(widths.tag.saturating_sub(marker_width)));
        }

        if edit.active_field == LedgerEntryField::StartDate
            || edit.active_field == LedgerEntryField::EndDate
        {
            let value = if edit.active_field == LedgerEntryField::StartDate {
                edit.start_date.as_str()
            } else {
                edit.end_date.as_str()
            };
            let cell_start = widths.tag.saturating_add(1);
            let visible_len = value.chars().count().min(widths.date);
            let left_pad = widths.date.saturating_sub(visible_len) / 2;
            return cell_start
                .saturating_add(left_pad)
                .saturating_add(edit.caret.min(visible_len));
        }

        if edit.active_field == LedgerEntryField::StartTime
            || edit.active_field == LedgerEntryField::EndTime
        {
            let cell_start = widths
                .tag
                .saturating_add(1)
                .saturating_add(if widths.show_date {
                    widths.date.saturating_add(1)
                } else {
                    0
                });
            let start_display = if edit.active_field == LedgerEntryField::StartTime {
                edit.start_time.clone()
            } else {
                super::ledger_state::format_ledger_time_input(&edit.start_time)
            };
            let time_value = if edit.is_active() {
                format!("{start_display}–now")
            } else {
                let end_display = if edit.active_field == LedgerEntryField::EndTime {
                    edit.end_time.clone()
                } else {
                    super::ledger_state::format_ledger_time_input(&edit.end_time)
                };
                format!("{start_display}–{end_display}")
            };
            let visible_len = time_value.chars().count().min(widths.time);
            let left_pad = widths.time.saturating_sub(visible_len) / 2;
            let within = if edit.active_field == LedgerEntryField::StartTime {
                edit.caret.min(start_display.chars().count())
            } else {
                start_display
                    .chars()
                    .count()
                    .saturating_add(1)
                    .saturating_add(edit.caret.min(edit.end_time.chars().count()))
            };
            return cell_start
                .saturating_add(left_pad)
                .saturating_add(within.min(visible_len));
        }
        0
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
        let columns = self.preferred_ledger_columns(logs).fit(row_width);
        let text_color =
            crate::appearance::contrasting_text_color(border_color, self.theme_foreground());

        let mut items = logs
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let is_selected = selected_log_index == Some(idx);
                let editing_this_row = self.ledger_entry_edit.as_ref().is_some_and(|edit| {
                    edit.session_id() == row.session_id && row.session_id.is_some()
                        || edit.is_active()
                            && row.active_stable_id.as_deref()
                                == self.session.active_session_stable_id.as_deref()
                });
                let filtered_out = self.report_tag_filter_active()
                    && !editing_this_row
                    && !self.report_log_matches_filter(row);
                let item = if editing_this_row {
                    ListItem::new({
                        let edit = self
                            .ledger_entry_edit
                            .as_ref()
                            .expect("editing row has ledger edit state");
                        let edit_columns = self.ledger_edit_columns(edit, columns, row_width);
                        self.ledger_edit_line(
                            edit,
                            edit_columns,
                            balance_marker(balance_effect),
                            &layer_display_name,
                        )
                    })
                } else {
                    ListItem::new(self.ledger_normal_line(
                        row,
                        columns,
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
                ListItem::new({
                    let edit = self
                        .ledger_entry_edit
                        .as_ref()
                        .expect("adding row has ledger edit state");
                    let edit_columns = self.ledger_edit_columns(edit, columns, row_width);
                    self.ledger_edit_line(edit, edit_columns, "+", &layer_display_name)
                })
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

        if let (Some(edit), Some(visual_index)) =
            (self.ledger_entry_edit.as_ref(), visual_selected_index)
            && edit.confirmation.is_none()
            && self.report_layer_delete_confirmation.is_none()
            && self.report_entry_delete_confirmation.is_none()
        {
            let visible_row = visual_index.saturating_sub(list_state.offset());
            if visible_row < usize::from(list_area.height) && list_area.width > 0 {
                let marker = if edit.is_add() {
                    "+"
                } else {
                    balance_marker(balance_effect)
                };
                let edit_columns = self.ledger_edit_columns(edit, columns, row_width);
                let relative_x = self.ledger_edit_cursor_column(edit, edit_columns, marker);
                let cursor_x = list_area.x.saturating_add(
                    u16::try_from(relative_x)
                        .unwrap_or(u16::MAX)
                        .min(list_area.width.saturating_sub(1)),
                );
                let cursor_y = list_area.y.saturating_add(visible_row as u16);
                #[allow(deprecated)]
                f.set_cursor(cursor_x, cursor_y);
            }
        }
    }

    fn render_report_layer_delete_confirmation(&self, f: &mut Frame, terminal: Rect) {
        let Some(category_id) = self.report_layer_delete_confirmation else {
            return;
        };
        let name = self.report_layer_display_name(category_id);
        self.render_system_dialog(
            f,
            terminal,
            super::SystemDialogSeverity::Warning,
            Line::from(vec![
                Span::raw("Delete "),
                Span::styled(
                    name,
                    Style::default().fg(self.category_color_for_id(category_id)),
                ),
                Span::raw("?"),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
            vec![
                Line::from("Balance history will be permanently removed."),
                Line::from("Existing sediment becomes Idle."),
            ],
            vec![Line::from("Delete Layer"), Line::from("Go back")],
            self.system_dialog_selected_index,
            48,
        );
    }

    fn render_report_entry_delete_confirmation(&self, f: &mut Frame, terminal: Rect) {
        let Some(identity) = self.report_entry_delete_confirmation.as_ref() else {
            return;
        };
        let Some(category_id) = self.report_logs_category_id else {
            return;
        };
        let logs = self.report_current_logs();
        let row = logs.iter().find(|row| match identity {
            super::LedgerSelectionIdentity::Session(session_id) => {
                row.session_id == Some(*session_id)
            }
            super::LedgerSelectionIdentity::Active(stable_id) => {
                row.active_stable_id.as_deref() == Some(stable_id.as_str())
            }
            super::LedgerSelectionIdentity::Add => false,
        });
        let Some(row) = row else {
            return;
        };
        let base = self.preferred_ledger_columns(std::slice::from_ref(row));
        let natural_width = base.required_proportional_total().max(24);
        let widths = base.fit(natural_width);
        let balance_effect = self
            .report_rows()
            .entries
            .iter()
            .find(|entry| entry.category_id == category_id)
            .map(|entry| entry.balance_effect)
            .unwrap_or(0);
        let line = self.ledger_normal_line(
            row,
            widths,
            LedgerRowPresentation {
                selected_text: None,
                filter_focus: None,
                is_none_category: category_id == DRIFT_CATEGORY_ID,
                balance_effect,
                marker_color: self.category_color_for_id(category_id),
                fallback_tag: &self.report_layer_display_name(category_id),
                deemphasized: false,
            },
        );
        self.render_system_dialog(
            f,
            terminal,
            super::SystemDialogSeverity::Warning,
            Line::from(Span::styled(
                "Delete entry?",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            vec![line],
            vec![Line::from("Delete entry"), Line::from("Go back")],
            self.system_dialog_selected_index,
            u16::try_from(natural_width.saturating_add(2)).unwrap_or(u16::MAX),
        );
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
                "No non-zero layers for this period.",
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
        App, LedgerColumnWidths, balance_marker, ledger_date_span_label, ledger_display_tag,
        ledger_edit_effective_end_date, ledger_time_span_label, summary_border_color,
    };
    use crate::domain::{BalanceReportEntry, BalanceReportSummary, CategoryId};

    #[test]
    fn ledger_data_regions_keep_symmetric_three_and_four_cell_anchors() {
        assert_eq!(
            App::ledger_weighted_widths(60, &[1, 1, 1]),
            vec![20, 20, 20]
        );
        assert_eq!(
            App::ledger_weighted_widths(60, &[1, 1, 1, 1]),
            vec![15, 15, 15, 15]
        );
    }

    #[test]
    fn ledger_detail_keeps_cross_day_dates_and_times_in_separate_sized_columns() {
        assert_eq!(
            ledger_date_span_label("2026-09-10", "2026-09-11"),
            "Sep 10-11"
        );
        assert_eq!(
            ledger_date_span_label("2026-09-30", "2026-10-01"),
            "Sep 30-Oct 1"
        );
        assert_eq!(
            ledger_time_span_label("23:50:00", "06:00:00"),
            "23:50–06:00"
        );

        let natural = LedgerColumnWidths {
            tag: "Renzo; Anibal y Renzo".chars().count() + 2,
            date: "Sep 10-11".chars().count(),
            time: "23:50–06:00".chars().count(),
            metric: 9,
            show_date: true,
        };
        assert_eq!(natural.total(), 55);
        assert_eq!(natural.required_proportional_total(), 71);
        let fitted = natural.fit(natural.required_proportional_total());
        assert_eq!(fitted.total(), 71);
        assert_eq!(fitted.tag, 23);
        assert_eq!(fitted.date, 11);
        assert_eq!(fitted.time, 11);
        assert_eq!(fitted.metric, 23);
        assert!(natural.fit(24).total() <= 24);
    }

    #[test]
    fn single_day_ledger_keeps_time_centered_without_widening_every_cell() {
        let natural = LedgerColumnWidths {
            tag: 40,
            date: 0,
            time: 11,
            metric: 9,
            show_date: false,
        };
        assert_eq!(natural.required_proportional_total(), 93);
        let fitted = natural.fit(natural.required_proportional_total());
        assert_eq!(fitted.total(), 93);
        assert_eq!(fitted.tag, 40);
        assert_eq!(fitted.time, 11);
        assert_eq!(fitted.metric, 40);
    }

    #[test]
    fn balance_marker_matches_layer_modal_and_keeps_idle_filled() {
        assert_eq!(balance_marker(-1), "◯");
        assert_eq!(balance_marker(0), "●");
        assert_eq!(balance_marker(1), "●");
    }

    #[test]
    fn empty_ledger_tag_uses_neutral_dash_without_persisting_a_fallback() {
        assert_eq!(ledger_display_tag("", "Cinema"), "—");
        assert_eq!(ledger_display_tag("   ", "Cinema"), "—");
        assert_eq!(ledger_display_tag("Anibal", "Cinema"), "Anibal");
    }

    #[test]
    fn ledger_edit_keeps_overnight_end_date_without_bracket_chrome() {
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
