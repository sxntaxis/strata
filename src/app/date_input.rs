use chrono::{Datelike, Duration as ChronoDuration, NaiveDate};

const MAX_DATE_DRAFT_CHARS: usize = 32;

pub(super) fn date_candidate_accepts_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-' || character == ' '
}

pub(super) fn date_candidate_is_well_shaped(value: &str) -> bool {
    value.chars().count() <= MAX_DATE_DRAFT_CHARS
        && value.chars().all(date_candidate_accepts_character)
}

fn month_number(token: &str) -> Option<i64> {
    let lower = token.to_ascii_lowercase();
    match lower.as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

fn normalized_ymd(year: i64, month: i64, day: i64) -> Result<NaiveDate, String> {
    let month_index = year
        .checked_mul(12)
        .and_then(|value| value.checked_add(month.saturating_sub(1)))
        .ok_or_else(|| "date is outside the supported range".to_string())?;
    let normalized_year = month_index.div_euclid(12);
    let normalized_month = month_index.rem_euclid(12).saturating_add(1);
    let normalized_year = i32::try_from(normalized_year)
        .map_err(|_| "date is outside the supported range".to_string())?;
    let normalized_month = u32::try_from(normalized_month)
        .map_err(|_| "date is outside the supported range".to_string())?;
    let base = NaiveDate::from_ymd_opt(normalized_year, normalized_month, 1)
        .ok_or_else(|| "date is outside the supported range".to_string())?;
    base.checked_add_signed(ChronoDuration::days(day.saturating_sub(1)))
        .ok_or_else(|| "date is outside the supported range".to_string())
}

pub(super) fn normalize_date_input(value: &str, current: NaiveDate) -> Result<NaiveDate, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("date cannot be empty".to_string());
    }
    if !date_candidate_is_well_shaped(value) {
        return Err("date contains unsupported characters".to_string());
    }

    let normalized_separators = value.replace('-', " ");
    let tokens = normalized_separators.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() || tokens.len() > 3 {
        return Err("date could not be understood".to_string());
    }

    let month_positions = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| month_number(token).map(|month| (index, month)))
        .collect::<Vec<_>>();

    let (year, month, day) = if month_positions.is_empty() {
        let numbers = tokens
            .iter()
            .map(|token| {
                token
                    .parse::<i64>()
                    .map_err(|_| "date could not be understood".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        match numbers.as_slice() {
            [only] if tokens[0].len() == 4 => (*only, 1, 1),
            [month] => (i64::from(current.year()), *month, 1),
            [first, second] if tokens[0].len() == 4 => (*first, *second, 1),
            [month, day] => (i64::from(current.year()), *month, *day),
            [year, month, day] => (*year, *month, *day),
            _ => return Err("date could not be understood".to_string()),
        }
    } else if month_positions.len() == 1 {
        let (month_index, month) = month_positions[0];
        let numeric = tokens
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != month_index)
            .map(|(index, token)| {
                token
                    .parse::<i64>()
                    .map(|value| (index, token.len(), value))
                    .map_err(|_| "date could not be understood".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        match numeric.as_slice() {
            [] => (i64::from(current.year()), month, 1),
            [(_, len, value)] if *len == 4 => (*value, month, 1),
            [(_, _, day)] => (i64::from(current.year()), month, *day),
            [first, second] => {
                let (year_part, day_part) = if first.1 == 4 && second.1 != 4 {
                    (first, second)
                } else if second.1 == 4 && first.1 != 4 {
                    (second, first)
                } else {
                    return Err("date with a named month needs one four-digit year".to_string());
                };
                (year_part.2, month, day_part.2)
            }
            _ => return Err("date could not be understood".to_string()),
        }
    } else {
        return Err("date contains more than one month name".to_string());
    };

    normalized_ymd(year, month, day)
}

pub(super) fn normalize_date_text(value: &str, current: NaiveDate) -> Result<String, String> {
    normalize_date_input(value, current).map(|date| date.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::{normalize_date_input, normalize_date_text};
    use chrono::NaiveDate;

    fn current() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
    }

    #[test]
    fn accepts_named_months_and_deterministic_defaults() {
        assert_eq!(normalize_date_text("Sep", current()).unwrap(), "2026-09-01");
        assert_eq!(
            normalize_date_text("September", current()).unwrap(),
            "2026-09-01"
        );
        assert_eq!(
            normalize_date_text("Sep 8", current()).unwrap(),
            "2026-09-08"
        );
        assert_eq!(
            normalize_date_text("8 Sep", current()).unwrap(),
            "2026-09-08"
        );
        assert_eq!(
            normalize_date_text("Sep 8 2025", current()).unwrap(),
            "2025-09-08"
        );
        assert_eq!(
            normalize_date_text("8 Sep 2025", current()).unwrap(),
            "2025-09-08"
        );
        assert_eq!(
            normalize_date_text("2026 Sep 8", current()).unwrap(),
            "2026-09-08"
        );
    }

    #[test]
    fn accepts_numeric_shorthand_with_month_first_missing_year() {
        assert_eq!(
            normalize_date_text("2026-9-8", current()).unwrap(),
            "2026-09-08"
        );
        assert_eq!(
            normalize_date_text("2026-9", current()).unwrap(),
            "2026-09-01"
        );
        assert_eq!(
            normalize_date_text("2027", current()).unwrap(),
            "2027-01-01"
        );
        assert_eq!(normalize_date_text("9-8", current()).unwrap(), "2026-09-08");
        assert_eq!(normalize_date_text("9", current()).unwrap(), "2026-09-01");
    }

    #[test]
    fn carries_overflow_like_time_input() {
        assert_eq!(
            normalize_date_text("2026-09-31", current()).unwrap(),
            "2026-10-01"
        );
        assert_eq!(
            normalize_date_text("Sep 31", current()).unwrap(),
            "2026-10-01"
        );
        assert_eq!(
            normalize_date_text("2026-13-02", current()).unwrap(),
            "2027-01-02"
        );
        assert_eq!(
            normalize_date_text("2026-00-15", current()).unwrap(),
            "2025-12-15"
        );
        assert_eq!(
            normalize_date_text("Sep 0", current()).unwrap(),
            "2026-08-31"
        );
    }

    #[test]
    fn empty_and_unresolved_named_month_input_are_not_silently_defaulted() {
        assert!(normalize_date_input("", current()).is_err());
        assert!(normalize_date_input("Sep x", current()).is_err());
    }
}
