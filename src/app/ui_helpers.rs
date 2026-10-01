use chrono::{Datelike, Duration as ChronoDuration, NaiveDate};

use crate::domain::{ReportPeriod, ReportWindow};

pub fn report_period_prev(period: ReportPeriod) -> ReportPeriod {
    match period {
        ReportPeriod::Today => ReportPeriod::Month,
        ReportPeriod::Week => ReportPeriod::Today,
        ReportPeriod::Month => ReportPeriod::Week,
    }
}

pub fn report_period_next(period: ReportPeriod) -> ReportPeriod {
    match period {
        ReportPeriod::Today => ReportPeriod::Week,
        ReportPeriod::Week => ReportPeriod::Month,
        ReportPeriod::Month => ReportPeriod::Today,
    }
}

pub fn format_report_interval_label(raw: &str) -> String {
    let parse = |value: &str| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok();

    if let Some((start_raw, end_raw)) = raw.split_once("..") {
        let (Some(start), Some(end)) = (parse(start_raw), parse(end_raw)) else {
            return raw.to_string();
        };

        if start.year() == end.year() && start.month() == end.month() {
            return format!("{}-{}", start.format("%b %-d"), end.format("%-d"));
        }

        if start.year() == end.year() {
            return format!("{}-{}", start.format("%b %-d"), end.format("%b %-d"));
        }

        return format!(
            "{}-{}",
            start.format("%b %-d, %Y"),
            end.format("%b %-d, %Y")
        );
    }

    parse(raw)
        .map(|date| date.format("%b %-d").to_string())
        .unwrap_or_else(|| raw.to_string())
}

pub fn report_window_end_exclusive(window: &ReportWindow) -> Option<NaiveDate> {
    window.end.checked_add_signed(ChronoDuration::days(1))
}

pub fn format_report_window_boundary_parts(window: &ReportWindow) -> (String, String) {
    let end_exclusive = report_window_end_exclusive(window).unwrap_or(window.end);

    if window.start.year() == end_exclusive.year() {
        (
            window.start.format("%b %-d").to_string(),
            end_exclusive.format("%b %-d").to_string(),
        )
    } else {
        (
            window.start.format("%b %-d, %Y").to_string(),
            end_exclusive.format("%b %-d, %Y").to_string(),
        )
    }
}

pub fn format_report_window_boundaries(window: &ReportWindow) -> String {
    let (start, end) = format_report_window_boundary_parts(window);
    format!("{start} – {end}")
}

pub fn wrap_prev_index(current: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else if current == 0 {
        len - 1
    } else {
        current - 1
    }
}

pub fn wrap_next_index(current: usize, len: usize) -> usize {
    if len == 0 || current + 1 >= len {
        0
    } else {
        current + 1
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{
        format_report_interval_label, format_report_window_boundaries, wrap_next_index,
        wrap_prev_index,
    };
    use crate::domain::ReportWindow;

    #[test]
    fn test_wrap_prev_index_wraps_to_end() {
        assert_eq!(wrap_prev_index(0, 5), 4);
        assert_eq!(wrap_prev_index(3, 5), 2);
        assert_eq!(wrap_prev_index(0, 0), 0);
    }

    #[test]
    fn test_wrap_next_index_wraps_to_start() {
        assert_eq!(wrap_next_index(4, 5), 0);
        assert_eq!(wrap_next_index(1, 5), 2);
        assert_eq!(wrap_next_index(0, 0), 0);
    }

    #[test]
    fn test_format_report_interval_same_month() {
        assert_eq!(
            format_report_interval_label("2026-02-09..2026-02-15"),
            "Feb 9-15"
        );
    }

    #[test]
    fn report_window_chrome_uses_explicit_exclusive_boundaries() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 21).unwrap();
        assert_eq!(
            format_report_window_boundaries(&ReportWindow::new(day, day).unwrap()),
            "Sep 21 – Sep 22"
        );

        let week_end = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        assert_eq!(
            format_report_window_boundaries(&ReportWindow::new(day, week_end).unwrap()),
            "Sep 21 – Sep 28"
        );
    }

    #[test]
    fn report_window_chrome_repeats_month_and_disambiguates_years() {
        let september_end = ReportWindow::new(
            NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(),
        )
        .unwrap();
        assert_eq!(
            format_report_window_boundaries(&september_end),
            "Sep 30 – Oct 1"
        );

        let year_end = ReportWindow::new(
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
        )
        .unwrap();
        assert_eq!(
            format_report_window_boundaries(&year_end),
            "Dec 31, 2026 – Jan 1, 2027"
        );
    }

    #[test]
    fn report_window_chrome_does_not_panic_at_maximum_date() {
        let window = ReportWindow::new(NaiveDate::MAX, NaiveDate::MAX).unwrap();
        assert_eq!(
            format_report_window_boundaries(&window),
            format!(
                "{} – {}",
                NaiveDate::MAX.format("%b %-d"),
                NaiveDate::MAX.format("%b %-d")
            )
        );
    }
}
