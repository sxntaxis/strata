use chrono::{Datelike, NaiveDate};

use crate::domain::ReportPeriod;

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
    if len == 0 {
        0
    } else if current + 1 >= len {
        0
    } else {
        current + 1
    }
}

#[cfg(test)]
mod tests {
    use super::{format_report_interval_label, wrap_next_index, wrap_prev_index};

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
}
