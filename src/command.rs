use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use crate::domain::FirstDayOfWeek;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CommandIntent {
    Status,
    Start {
        layer: String,
        tag: Option<String>,
    },
    Stop {
        layer: Option<String>,
        tag: Option<String>,
    },
    Karma {
        selector: KarmaSelector,
        layer: Option<String>,
        tag: Option<String>,
    },
    DeleteLastSession {
        layer: String,
        tag: Option<String>,
    },
    DataDir,
    ConfigDir,
    Timer {
        duration_seconds: u64,
    },
    #[cfg(debug_assertions)]
    TestingCheatsHalfFull,
}

impl CommandIntent {
    pub(crate) fn keeps_palette_open(&self) -> bool {
        matches!(
            self,
            Self::Status
                | Self::Karma { .. }
                | Self::DataDir
                | Self::ConfigDir
                | Self::Timer { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum KarmaSelector {
    Today,
    Yesterday,
    Weekday(Weekday),
    CurrentWeek,
    LastWeek,
    CurrentMonth,
    LastMonth,
    IsoWeek(u32),
    MonthDay { month: u32, day: u32 },
    Date(NaiveDate),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KarmaWindow {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub label: String,
}

pub(crate) fn parse(input: &str) -> Result<CommandIntent, String> {
    let tokens = tokenize(input)?;
    let Some((command, args)) = tokens.split_first() else {
        return Err(
            "Missing command. Try: status, start, stop, karma, x, datadir, configdir, timer"
                .to_string(),
        );
    };

    match command.trim_end_matches('=').to_ascii_lowercase().as_str() {
        "status" if args.is_empty() => Ok(CommandIntent::Status),
        "start" => parse_start(args),
        "stop" => parse_stop(args),
        "karma" => parse_karma(args),
        "x" => parse_delete_last_session(args),
        "datadir" if args.is_empty() => Ok(CommandIntent::DataDir),
        "configdir" if args.is_empty() => Ok(CommandIntent::ConfigDir),
        "timer" => Ok(CommandIntent::Timer {
            duration_seconds: parse_duration(args)?,
        }),
        #[cfg(debug_assertions)]
        "testingcheats" if args.len() == 1 && args[0].eq_ignore_ascii_case("half") => {
            Ok(CommandIntent::TestingCheatsHalfFull)
        }
        _ => Err(format!("Invalid command or arguments: {input}")),
    }
}

fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            } else {
                current.push(ch);
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }

    if escaped {
        current.push('\\');
    }
    if quote.is_some() {
        return Err("Unterminated quoted string".into());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn parse_start(args: &[String]) -> Result<CommandIntent, String> {
    if args.is_empty() || args.len() > 2 {
        return Err("Usage: start <layer> [tag]".to_string());
    }
    Ok(CommandIntent::Start {
        layer: args[0].clone(),
        tag: args.get(1).cloned(),
    })
}

fn parse_stop(args: &[String]) -> Result<CommandIntent, String> {
    if args.len() > 2 {
        return Err("Usage: stop [layer] [tag]".to_string());
    }
    Ok(CommandIntent::Stop {
        layer: args.first().cloned(),
        tag: args.get(1).cloned(),
    })
}

fn parse_karma(args: &[String]) -> Result<CommandIntent, String> {
    if args.is_empty() {
        return Ok(CommandIntent::Karma {
            selector: KarmaSelector::Today,
            layer: None,
            tag: None,
        });
    }

    let mut index = 0usize;
    let mut selector = KarmaSelector::Today;
    if let Some((parsed, consumed)) = parse_karma_selector(args) {
        selector = parsed;
        index = consumed;
    }
    let (layer, tag) = parse_karma_scope(&args[index..])?;
    Ok(CommandIntent::Karma {
        selector,
        layer,
        tag,
    })
}

fn parse_delete_last_session(args: &[String]) -> Result<CommandIntent, String> {
    if args.len() < 2
        || args.len() > 3
        || !args
            .last()
            .is_some_and(|v| v.eq_ignore_ascii_case("lastsession"))
    {
        return Err("Usage: x <layer> [tag] lastsession".to_string());
    }
    Ok(CommandIntent::DeleteLastSession {
        layer: args[0].clone(),
        tag: (args.len() == 3).then(|| args[1].clone()),
    })
}

fn parse_karma_scope(tokens: &[String]) -> Result<(Option<String>, Option<String>), String> {
    if tokens.is_empty() {
        return Ok((None, None));
    }
    let index = usize::from(tokens[0].eq_ignore_ascii_case("layer"));
    let scoped = &tokens[index..];
    if scoped.is_empty() {
        return Err("Expected layer name after 'layer'".to_string());
    }
    if scoped.len() > 2 {
        return Err("Usage: karma [time] [layer [tag]]".to_string());
    }
    Ok((scoped.first().cloned(), scoped.get(1).cloned()))
}

fn parse_karma_selector(tokens: &[String]) -> Option<(KarmaSelector, usize)> {
    let first = tokens.first()?.trim_end_matches('=').to_ascii_lowercase();
    let selector = match first.as_str() {
        "today" => Some((KarmaSelector::Today, 1)),
        "yesterday" => Some((KarmaSelector::Yesterday, 1)),
        "week" => Some((KarmaSelector::CurrentWeek, 1)),
        "lastweek" => Some((KarmaSelector::LastWeek, 1)),
        "month" => Some((KarmaSelector::CurrentMonth, 1)),
        "lastmonth" => Some((KarmaSelector::LastMonth, 1)),
        "monday" => Some((KarmaSelector::Weekday(Weekday::Mon), 1)),
        "tuesday" => Some((KarmaSelector::Weekday(Weekday::Tue), 1)),
        "wednesday" => Some((KarmaSelector::Weekday(Weekday::Wed), 1)),
        "thursday" => Some((KarmaSelector::Weekday(Weekday::Thu), 1)),
        "friday" => Some((KarmaSelector::Weekday(Weekday::Fri), 1)),
        "saturday" => Some((KarmaSelector::Weekday(Weekday::Sat), 1)),
        "sunday" => Some((KarmaSelector::Weekday(Weekday::Sun), 1)),
        _ => None,
    };
    if selector.is_some() {
        return selector;
    }
    if let Some(week) = parse_week_token(&first) {
        return Some((KarmaSelector::IsoWeek(week), 1));
    }
    if let Ok(date) = NaiveDate::parse_from_str(&first, "%Y-%m-%d") {
        return Some((KarmaSelector::Date(date), 1));
    }
    if let Some((month, day)) = parse_month_day_compact(&first) {
        return Some((KarmaSelector::MonthDay { month, day }, 1));
    }
    if tokens.len() >= 2
        && let Some(month) = parse_month_name(&first)
        && let Ok(day) = tokens[1].parse::<u32>()
        && (1..=31).contains(&day)
    {
        return Some((KarmaSelector::MonthDay { month, day }, 2));
    }
    None
}

pub(crate) fn resolve_karma_window(
    selector: &KarmaSelector,
    today: NaiveDate,
    first_day_of_week: FirstDayOfWeek,
) -> Result<KarmaWindow, String> {
    match selector {
        KarmaSelector::Today => Ok(single_day(today)),
        KarmaSelector::Yesterday => Ok(single_day(today - ChronoDuration::days(1))),
        KarmaSelector::Weekday(target) => {
            let today_index = today.weekday().num_days_from_monday() as i64;
            let target_index = target.num_days_from_monday() as i64;
            Ok(single_day(
                today - ChronoDuration::days((7 + today_index - target_index) % 7),
            ))
        }
        KarmaSelector::CurrentWeek => {
            let start = start_of_week(today, first_day_of_week);
            Ok(window(start, today))
        }
        KarmaSelector::LastWeek => {
            let start = start_of_week(today, first_day_of_week) - ChronoDuration::days(7);
            Ok(window(start, start + ChronoDuration::days(6)))
        }
        KarmaSelector::CurrentMonth => Ok(window(today.with_day(1).unwrap_or(today), today)),
        KarmaSelector::LastMonth => {
            let previous_end = today.with_day(1).unwrap_or(today) - ChronoDuration::days(1);
            Ok(window(
                previous_end.with_day(1).unwrap_or(previous_end),
                previous_end,
            ))
        }
        KarmaSelector::IsoWeek(week) => {
            let start = NaiveDate::from_isoywd_opt(today.year(), *week, Weekday::Mon)
                .ok_or_else(|| format!("Invalid ISO week '{week}'"))?;
            Ok(window(start, start + ChronoDuration::days(6)))
        }
        KarmaSelector::MonthDay { month, day } => {
            let mut year = today.year();
            let mut resolved = NaiveDate::from_ymd_opt(year, *month, *day)
                .ok_or_else(|| format!("Invalid date selector '{}{}'", month_name(*month), day))?;
            if resolved > today {
                year -= 1;
                resolved = NaiveDate::from_ymd_opt(year, *month, *day).ok_or_else(|| {
                    format!("Invalid date selector '{}{}'", month_name(*month), day)
                })?;
            }
            Ok(single_day(resolved))
        }
        KarmaSelector::Date(date) => Ok(single_day(*date)),
    }
}

fn single_day(day: NaiveDate) -> KarmaWindow {
    KarmaWindow {
        start: day,
        end: day,
        label: day.format("%Y-%m-%d").to_string(),
    }
}

fn window(start: NaiveDate, end: NaiveDate) -> KarmaWindow {
    KarmaWindow {
        start,
        end,
        label: format!("{}..{}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d")),
    }
}

fn parse_week_token(value: &str) -> Option<u32> {
    let digits = value.strip_prefix("week")?;
    let week = digits.parse::<u32>().ok()?;
    (1..=53).contains(&week).then_some(week)
}

fn parse_month_day_compact(value: &str) -> Option<(u32, u32)> {
    let mut letters = String::new();
    let mut digits = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphabetic() {
            if !digits.is_empty() {
                return None;
            }
            letters.push(ch);
        } else if ch.is_ascii_digit() {
            digits.push(ch);
        } else if !matches!(ch, '-' | '_' | '/') {
            return None;
        }
    }
    if letters.is_empty() || digits.is_empty() {
        return None;
    }
    let month = parse_month_name(&letters)?;
    let day = digits.parse::<u32>().ok()?;
    (1..=31).contains(&day).then_some((month, day))
}

fn parse_month_name(value: &str) -> Option<u32> {
    match value.to_ascii_lowercase().as_str() {
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

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "jan",
        2 => "feb",
        3 => "mar",
        4 => "apr",
        5 => "may",
        6 => "jun",
        7 => "jul",
        8 => "aug",
        9 => "sep",
        10 => "oct",
        11 => "nov",
        12 => "dec",
        _ => "month",
    }
}

fn start_of_week(day: NaiveDate, first_day: FirstDayOfWeek) -> NaiveDate {
    let day_index = day.weekday().num_days_from_monday() as i64;
    let first_index = match first_day {
        FirstDayOfWeek::Monday => 0,
        FirstDayOfWeek::Tuesday => 1,
        FirstDayOfWeek::Wednesday => 2,
        FirstDayOfWeek::Thursday => 3,
        FirstDayOfWeek::Friday => 4,
        FirstDayOfWeek::Saturday => 5,
        FirstDayOfWeek::Sunday => 6,
    };
    day - ChronoDuration::days((7 + day_index - first_index) % 7)
}

fn parse_duration(tokens: &[String]) -> Result<u64, String> {
    if tokens.is_empty() {
        return Err("Usage: timer <duration> (examples: 5min, 1h30m)".into());
    }
    let compact = tokens.join("").to_ascii_lowercase();
    let chars: Vec<char> = compact.chars().collect();
    let mut index = 0;
    let mut total = 0u64;
    while index < chars.len() {
        let start = index;
        while index < chars.len() && chars[index].is_ascii_digit() {
            index += 1;
        }
        if start == index {
            return Err(format!("Invalid duration '{compact}'"));
        }
        let value: u64 = compact[start..index]
            .parse()
            .map_err(|_| format!("Invalid duration '{compact}'"))?;
        let unit_start = index;
        while index < chars.len() && chars[index].is_ascii_alphabetic() {
            index += 1;
        }
        if unit_start == index {
            return Err(format!("Missing duration unit in '{compact}'"));
        }
        let multiplier = match &compact[unit_start..index] {
            "s" | "sec" | "secs" | "second" | "seconds" => 1,
            "m" | "min" | "mins" | "minute" | "minutes" => 60,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3600,
            unit => return Err(format!("Unsupported duration unit '{unit}'")),
        };
        total = total
            .checked_add(
                value
                    .checked_mul(multiplier)
                    .ok_or("Duration is too large")?,
            )
            .ok_or("Duration is too large")?;
    }
    if total == 0 {
        return Err("Duration must be greater than zero".into());
    }
    Ok(total)
}

pub(crate) fn format_hms(seconds: usize) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

pub(crate) fn format_signed_hms(seconds: isize) -> String {
    if seconds < 0 {
        format!("-{}", format_hms(seconds.unsigned_abs()))
    } else {
        format!("+{}", format_hms(seconds as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_old_expert_command_shapes() {
        assert_eq!(
            parse("start Work \"deep focus\"").unwrap(),
            CommandIntent::Start {
                layer: "Work".into(),
                tag: Some("deep focus".into()),
            }
        );
        assert_eq!(
            parse("stop Work \"deep focus\"").unwrap(),
            CommandIntent::Stop {
                layer: Some("Work".into()),
                tag: Some("deep focus".into()),
            }
        );
        assert_eq!(
            parse("x Work focus lastsession").unwrap(),
            CommandIntent::DeleteLastSession {
                layer: "Work".into(),
                tag: Some("focus".into()),
            }
        );
        assert_eq!(
            parse("timer 1h30m").unwrap(),
            CommandIntent::Timer {
                duration_seconds: 5400
            }
        );
    }

    #[test]
    fn parses_karma_selectors_and_scope() {
        assert_eq!(
            parse("karma lastweek Work focus").unwrap(),
            CommandIntent::Karma {
                selector: KarmaSelector::LastWeek,
                layer: Some("Work".into()),
                tag: Some("focus".into()),
            }
        );
        assert_eq!(
            parse("karma 2026-08-20").unwrap(),
            CommandIntent::Karma {
                selector: KarmaSelector::Date(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()),
                layer: None,
                tag: None,
            }
        );
    }
}
