use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::{Line, Span},
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};

use crate::domain::BalanceReportSummary;

use super::{App, view_style};

const MIN_PREFERRED_WIDTH: u16 = 45;
const INSTRUMENT_SIDE_PADDING: u16 = 2;
const SUMMARY_VERTICAL_CHROME_ROWS: u16 = 5;
const MIN_THREE_COLUMN_WIDTH: u16 = 27;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InstrumentTotals {
    negative: isize,
    net: isize,
    positive: isize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MeterRole {
    Inactive,
    Negative,
    Positive,
    Equilibrium,
    DotNegative,
    DotPositive,
    DotNeutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MeterCell {
    glyph: char,
    role: MeterRole,
}

fn instrument_totals(summary: &BalanceReportSummary) -> InstrumentTotals {
    let negative = summary
        .entries
        .iter()
        .map(|entry| entry.balance_seconds)
        .filter(|seconds| *seconds < 0)
        .sum();
    let positive = summary
        .entries
        .iter()
        .map(|entry| entry.balance_seconds)
        .filter(|seconds| *seconds > 0)
        .sum();

    InstrumentTotals {
        negative,
        net: summary.total_balance_seconds,
        positive,
    }
}

fn format_hms_magnitude(seconds: isize) -> String {
    let abs_secs = seconds.unsigned_abs();
    format!(
        "{:02}:{:02}:{:02}",
        abs_secs / 3600,
        (abs_secs % 3600) / 60,
        abs_secs % 60
    )
}

#[cfg(test)]
fn format_negative_total(seconds: isize) -> String {
    format!("-{}", format_hms_magnitude(seconds))
}

#[cfg(test)]
fn format_positive_total(seconds: isize) -> String {
    format!("+{}", format_hms_magnitude(seconds))
}

fn format_net_total(seconds: isize) -> String {
    if seconds < 0 {
        format!("-{}", format_hms_magnitude(seconds))
    } else if seconds > 0 {
        format!("+{}", format_hms_magnitude(seconds))
    } else {
        format_hms_magnitude(seconds)
    }
}

pub(super) fn preferred_summary_inner_width() -> u16 {
    MIN_PREFERRED_WIDTH.saturating_add(INSTRUMENT_SIDE_PADDING)
}

pub(super) fn preferred_summary_inner_height(row_count: usize) -> u16 {
    let rows = row_count.min(u16::MAX as usize) as u16;
    rows.saturating_add(SUMMARY_VERTICAL_CHROME_ROWS)
}

fn side_total_line(
    seconds: isize,
    sign: char,
    sign_color: Color,
    digits_color: Color,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(sign.to_string(), Style::default().fg(sign_color)),
        Span::styled(
            format_hms_magnitude(seconds),
            Style::default().fg(digits_color),
        ),
    ])
}

fn instrument_width(available: u16) -> u16 {
    let cap = available;
    if cap >= MIN_THREE_COLUMN_WIDTH {
        for width in (MIN_THREE_COLUMN_WIDTH..=cap).rev() {
            if width % 6 == 3 {
                return width;
            }
        }
    }

    if cap >= 5 {
        if cap.is_multiple_of(2) { cap - 1 } else { cap }
    } else {
        cap
    }
}

fn rounded_meter_offset(signed_seconds: i128, total_seconds: i128, radius: i128) -> i128 {
    if total_seconds == 0 || signed_seconds == 0 || radius == 0 {
        return 0;
    }
    let scaled = signed_seconds.saturating_mul(radius);
    let rounded = if scaled > 0 {
        (scaled + total_seconds / 2) / total_seconds
    } else {
        (scaled - total_seconds / 2) / total_seconds
    };
    if rounded == 0 {
        signed_seconds.signum()
    } else {
        rounded
    }
}

fn meter_radius(width: usize) -> i128 {
    if width < 3 {
        return 0;
    }
    (width / 2).saturating_sub(1).max(1) as i128
}

fn meter_offset(width: u16, signed_seconds: isize, total_seconds: usize) -> i128 {
    let width = usize::from(width);
    let radius = meter_radius(width);
    rounded_meter_offset(signed_seconds as i128, total_seconds as i128, radius)
        .clamp(-radius, radius)
}

fn meter_cells_for_offset(width: u16, offset: i128) -> Vec<MeterCell> {
    let width = usize::from(width);
    if width == 0 {
        return Vec::new();
    }
    if width == 1 {
        return vec![MeterCell {
            glyph: '●',
            role: MeterRole::DotNeutral,
        }];
    }

    let mut cells = vec![
        MeterCell {
            glyph: '─',
            role: MeterRole::Inactive,
        };
        width
    ];
    cells[0].glyph = '└';
    cells[width - 1].glyph = '┘';

    let center = width / 2;
    if width < 3 {
        return cells;
    }

    cells[center] = MeterCell {
        glyph: '┼',
        role: MeterRole::Equilibrium,
    };

    let radius = meter_radius(width);
    let offset = offset.clamp(-radius, radius);
    let dot = (center as i128 + offset) as usize;

    if dot < center {
        for cell in cells.iter_mut().take(center).skip(dot + 1) {
            *cell = MeterCell {
                glyph: '━',
                role: MeterRole::Negative,
            };
        }
        cells[dot] = MeterCell {
            glyph: '●',
            role: MeterRole::DotNegative,
        };
    } else if dot > center {
        for cell in cells.iter_mut().take(dot).skip(center + 1) {
            *cell = MeterCell {
                glyph: '━',
                role: MeterRole::Positive,
            };
        }
        cells[dot] = MeterCell {
            glyph: '●',
            role: MeterRole::DotPositive,
        };
    } else {
        cells[center] = MeterCell {
            glyph: '●',
            role: MeterRole::DotNeutral,
        };
    }

    cells
}

fn meter_cells(width: u16, signed_seconds: isize, total_seconds: usize) -> Vec<MeterCell> {
    meter_cells_for_offset(width, meter_offset(width, signed_seconds, total_seconds))
}

fn meter_line(
    cells: Vec<MeterCell>,
    inactive: Color,
    negative: Color,
    positive: Color,
) -> Line<'static> {
    let inactive_style = Style::default().fg(inactive).add_modifier(Modifier::DIM);
    Line::from(
        cells
            .into_iter()
            .map(|cell| {
                let style = match cell.role {
                    MeterRole::Inactive | MeterRole::Equilibrium => inactive_style,
                    MeterRole::Negative | MeterRole::DotNegative => Style::default().fg(negative),
                    MeterRole::Positive | MeterRole::DotPositive => Style::default().fg(positive),
                    MeterRole::DotNeutral => Style::default().fg(inactive),
                };
                Span::styled(cell.glyph.to_string(), style)
            })
            .collect::<Vec<_>>(),
    )
}

impl App {
    pub(super) fn render_balance_instrument(
        &self,
        f: &mut Frame,
        totals_area: Rect,
        meter_area: Rect,
        summary: &BalanceReportSummary,
    ) {
        let totals = instrument_totals(summary);
        let available = totals_area.width.saturating_sub(2);
        let width = instrument_width(available);
        if width == 0 {
            return;
        }

        let instrument_x = totals_area.x + totals_area.width.saturating_sub(width) / 2;
        let totals_rect = Rect::new(instrument_x, totals_area.y, width, 1);
        let meter_rect = Rect::new(instrument_x, meter_area.y, width, 1);

        let net_style = Style::default().fg(view_style::balance_color(
            totals.net,
            self.theme_error(),
            self.theme_success(),
            self.theme_status(),
        ));

        if width >= MIN_THREE_COLUMN_WIDTH && width.is_multiple_of(3) {
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(totals_rect);

            f.render_widget(
                Paragraph::new(side_total_line(
                    totals.negative,
                    '-',
                    self.theme_error(),
                    self.theme_status(),
                ))
                .alignment(Alignment::Left),
                columns[0],
            );
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format_net_total(totals.net),
                    net_style,
                )))
                .alignment(Alignment::Center),
                columns[1],
            );
            f.render_widget(
                Paragraph::new(side_total_line(
                    totals.positive,
                    '+',
                    self.theme_success(),
                    self.theme_status(),
                ))
                .alignment(Alignment::Right),
                columns[2],
            );
        } else {
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format_net_total(totals.net),
                    net_style,
                )))
                .alignment(Alignment::Center),
                totals_rect,
            );
        }

        f.render_widget(
            Paragraph::new(meter_line(
                meter_cells(width, summary.total_balance_seconds, summary.total_seconds),
                self.theme_status(),
                self.theme_error(),
                self.theme_success(),
            )),
            meter_rect,
        );
    }

    pub(super) fn render_layer_influence_instrument(
        &self,
        f: &mut Frame,
        total_area: Rect,
        meter_area: Rect,
        filter_area: Option<Rect>,
        summary: &BalanceReportSummary,
        contribution: isize,
    ) {
        let available = total_area.width.saturating_sub(2);
        let width = instrument_width(available);
        if width == 0 {
            return;
        }

        let instrument_x = total_area.x + total_area.width.saturating_sub(width) / 2;
        let total_rect = Rect::new(instrument_x, total_area.y, width, 1);
        let meter_rect = Rect::new(instrument_x, meter_area.y, width, 1);

        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format_net_total(contribution),
                Style::default().fg(view_style::balance_color(
                    contribution,
                    self.theme_error(),
                    self.theme_success(),
                    self.theme_status(),
                )),
            )))
            .alignment(Alignment::Center),
            total_rect,
        );

        f.render_widget(
            Paragraph::new(meter_line(
                meter_cells(width, contribution, summary.total_seconds),
                self.theme_status(),
                self.theme_error(),
                self.theme_success(),
            )),
            meter_rect,
        );

        if let (Some(filter_area), Some(filter_label)) = (filter_area, self.report_filter_label()) {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(
                        "filter  ",
                        Style::default()
                            .fg(self.theme_status())
                            .add_modifier(Modifier::DIM),
                    ),
                    Span::styled(filter_label, Style::default().fg(self.theme_foreground())),
                ]))
                .alignment(Alignment::Center),
                filter_area,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use crate::domain::{BalanceReportEntry, BalanceReportSummary, CategoryId, DRIFT_CATEGORY_ID};

    use super::{
        InstrumentTotals, MIN_THREE_COLUMN_WIDTH, MeterRole, format_negative_total,
        format_net_total, format_positive_total, instrument_totals, instrument_width, meter_cells,
        meter_cells_for_offset, meter_offset, preferred_summary_inner_height,
        preferred_summary_inner_width, side_total_line,
    };

    fn entry(
        id: u64,
        name: &str,
        elapsed_seconds: usize,
        balance_effect: i8,
        balance_seconds: isize,
    ) -> BalanceReportEntry {
        BalanceReportEntry {
            category_id: CategoryId::new(id),
            category_name: name.to_string(),
            color: Color::White,
            elapsed_seconds,
            balance_effect,
            balance_seconds,
        }
    }

    #[test]
    fn preferred_summary_geometry_reserves_instrument_spacing_and_content_height() {
        assert_eq!(preferred_summary_inner_width(), 47);
        assert_eq!(preferred_summary_inner_height(8), 13);
    }

    #[test]
    fn side_total_styles_only_the_sign_with_polarity() {
        let negative = side_total_line(-3665, '-', Color::Red, Color::Gray);
        assert_eq!(negative.spans.len(), 2);
        assert_eq!(negative.spans[0].content.as_ref(), "-");
        assert_eq!(negative.spans[0].style.fg, Some(Color::Red));
        assert_eq!(negative.spans[1].content.as_ref(), "01:01:05");
        assert_eq!(negative.spans[1].style.fg, Some(Color::Gray));

        let positive = side_total_line(861, '+', Color::Green, Color::Gray);
        assert_eq!(positive.spans.len(), 2);
        assert_eq!(positive.spans[0].content.as_ref(), "+");
        assert_eq!(positive.spans[0].style.fg, Some(Color::Green));
        assert_eq!(positive.spans[1].content.as_ref(), "00:14:21");
        assert_eq!(positive.spans[1].style.fg, Some(Color::Gray));
    }

    #[test]
    fn aggregate_totals_exclude_idle_from_polarity() {
        let summary = BalanceReportSummary {
            date: "2026-09-27".to_string(),
            entries: vec![
                entry(1, "positive", 861, 1, 861),
                BalanceReportEntry {
                    category_id: DRIFT_CATEGORY_ID,
                    category_name: "idle".to_string(),
                    color: Color::White,
                    elapsed_seconds: 218,
                    balance_effect: 0,
                    balance_seconds: 0,
                },
                entry(2, "negative", 3665, -1, -3665),
            ],
            total_seconds: 4744,
            total_balance_seconds: -2804,
        };

        assert_eq!(
            instrument_totals(&summary),
            InstrumentTotals {
                negative: -3665,
                net: -2804,
                positive: 861,
            }
        );
    }

    #[test]
    fn totals_format_polarity_and_zero_net_explicitly() {
        assert_eq!(format_negative_total(0), "-00:00:00");
        assert_eq!(format_negative_total(-3665), "-01:01:05");
        assert_eq!(format_positive_total(0), "+00:00:00");
        assert_eq!(format_positive_total(861), "+00:14:21");
        assert_eq!(format_net_total(-2804), "-00:46:44");
        assert_eq!(format_net_total(2804), "+00:46:44");
        assert_eq!(format_net_total(0), "00:00:00");
    }

    #[test]
    fn responsive_width_keeps_total_center_aligned_with_meter_center() {
        for (available, expected) in [
            (117, 117),
            (80, 75),
            (66, 63),
            (45, 45),
            (44, 39),
            (38, 33),
            (32, 27),
        ] {
            let width = instrument_width(available);
            assert_eq!(width, expected);
            assert!(width >= MIN_THREE_COLUMN_WIDTH);
            assert_eq!(width % 3, 0);
            assert_eq!(width % 2, 1);

            let third = width / 3;
            let center_column_center = third + third / 2;
            assert_eq!(center_column_center, width / 2);
        }

        assert_eq!(instrument_width(26), 25);
        assert_eq!(instrument_width(0), 0);
    }

    #[test]
    fn nonzero_meter_values_always_leave_equilibrium_by_at_least_one_cell() {
        let width = 117;
        let total_recorded = 30 * 24 * 60 * 60;

        assert_eq!(meter_offset(width, 0, total_recorded), 0);
        assert_eq!(meter_offset(width, 1, total_recorded), 1);
        assert_eq!(meter_offset(width, -1, total_recorded), -1);

        assert_eq!(meter_offset(3, 1, 100), 1);
        assert_eq!(meter_offset(3, -1, 100), -1);
        assert_eq!(
            meter_cells_for_offset(3, -1)[0].role,
            MeterRole::DotNegative
        );
        assert_eq!(meter_cells_for_offset(3, 1)[2].role, MeterRole::DotPositive);
    }

    #[test]
    fn meter_places_extremes_and_equilibrium_symmetrically() {
        let width = 45;
        let center = usize::from(width / 2);
        let left_usable = 1;
        let right_usable = usize::from(width - 2);

        let all_negative = meter_cells(width, -100, 100);
        assert_eq!(all_negative[left_usable].role, MeterRole::DotNegative);

        let negative_dominant = meter_cells(width, -50, 100);
        let negative_dot = negative_dominant
            .iter()
            .position(|cell| cell.role == MeterRole::DotNegative)
            .expect("negative dot");
        assert!(negative_dot < center);

        let equal = meter_cells(width, 0, 100);
        assert_eq!(equal[center].role, MeterRole::DotNeutral);
        assert_eq!(equal[center].glyph, '●');

        let positive_dominant = meter_cells(width, 50, 100);
        let positive_dot = positive_dominant
            .iter()
            .position(|cell| cell.role == MeterRole::DotPositive)
            .expect("positive dot");
        assert!(positive_dot > center);

        let all_positive = meter_cells(width, 100, 100);
        assert_eq!(all_positive[right_usable].role, MeterRole::DotPositive);

        let empty = meter_cells(width, 0, 0);
        assert_eq!(empty[center].role, MeterRole::DotNeutral);
    }

    #[test]
    fn meter_keeps_endcaps_center_and_directional_heavy_segment() {
        let width = 45;
        let center = usize::from(width / 2);

        let negative = meter_cells(width, -50, 100);
        assert_eq!(negative.len(), usize::from(width));
        assert_eq!(negative[0].glyph, '└');
        assert_eq!(negative[usize::from(width) - 1].glyph, '┘');
        assert_eq!(negative[center].glyph, '┼');
        let negative_dot = negative
            .iter()
            .position(|cell| cell.role == MeterRole::DotNegative)
            .expect("negative dot");
        assert!(
            negative[negative_dot + 1..center]
                .iter()
                .all(|cell| cell.glyph == '━' && cell.role == MeterRole::Negative)
        );

        let positive = meter_cells(width, 50, 100);
        assert_eq!(positive[0].glyph, '└');
        assert_eq!(positive[usize::from(width) - 1].glyph, '┘');
        assert_eq!(positive[center].glyph, '┼');
        let positive_dot = positive
            .iter()
            .position(|cell| cell.role == MeterRole::DotPositive)
            .expect("positive dot");
        assert!(
            positive[center + 1..positive_dot]
                .iter()
                .all(|cell| cell.glyph == '━' && cell.role == MeterRole::Positive)
        );
    }

    #[test]
    fn shared_meter_scale_uses_total_recorded_time_for_global_and_layer_values() {
        let width = 45;
        let total_recorded = 100;
        assert_eq!(meter_offset(width, 20, total_recorded), 4);
        assert_eq!(meter_offset(width, 60, total_recorded), 13);
        assert_eq!(meter_offset(width, -40, total_recorded), -8);
    }

    #[test]
    fn layer_can_pull_opposite_the_global_result_on_the_same_scale() {
        let width = 45;
        let total_recorded = 10 * 60 * 60;
        let global = meter_offset(width, 2 * 60 * 60, total_recorded);
        let negative_layer = meter_offset(width, -4 * 60 * 60, total_recorded);
        assert!(global > 0);
        assert!(negative_layer < 0);
    }

    #[test]
    fn balanced_global_does_not_collapse_individual_layer_positions() {
        let width = 45;
        let total_recorded = 16 * 60 * 60;
        assert_eq!(meter_offset(width, 0, total_recorded), 0);
        assert!(meter_offset(width, 5 * 60 * 60, total_recorded) > 0);
        assert!(meter_offset(width, -7 * 60 * 60, total_recorded) < 0);
    }

    #[test]
    fn idle_time_remains_part_of_the_shared_denominator() {
        let width = 45;
        let explicit_positive = 100;
        let idle = 100;
        let without_idle = meter_offset(width, explicit_positive, explicit_positive as usize);
        let with_idle = meter_offset(
            width,
            explicit_positive,
            (explicit_positive + idle) as usize,
        );
        assert!(with_idle > 0);
        assert!(with_idle < without_idle);
    }
}
