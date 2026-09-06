use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use crate::domain::{FirstDayOfWeek, ReportWindow};

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
    Balance {
        selector: BalanceSelector,
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
    TestingCheatsHelp,
    #[cfg(debug_assertions)]
    TestingCheatsFallSpeed {
        multiplier: Option<u32>,
    },
    #[cfg(debug_assertions)]
    TestingCheatsAdvance {
        duration_seconds: u64,
    },
    #[cfg(debug_assertions)]
    TestingCheatsModel {
        model: String,
    },
    #[cfg(debug_assertions)]
    TestingCheatsClear,
    #[cfg(debug_assertions)]
    TestingCheatsFill,
    #[cfg(debug_assertions)]
    TestingCheatsFillHalf,
    #[cfg(debug_assertions)]
    TestingCheatsClassicTexture {
        profile: Option<String>,
    },
    #[cfg(debug_assertions)]
    TestingCheatsClassicExperiment {
        profile: Option<String>,
    },
    #[cfg(debug_assertions)]
    TestingCheatsStatus,
    #[cfg(debug_assertions)]
    TestingCheatsProvenance,
    #[cfg(debug_assertions)]
    TestingCheatsReset,
}

impl CommandIntent {
    pub(crate) fn keeps_palette_open(&self) -> bool {
        let ordinary = matches!(
            self,
            Self::Status
                | Self::Balance { .. }
                | Self::DataDir
                | Self::ConfigDir
                | Self::Timer { .. }
        );
        #[cfg(debug_assertions)]
        {
            ordinary
                || matches!(
                    self,
                    Self::TestingCheatsHelp
                        | Self::TestingCheatsFallSpeed { .. }
                        | Self::TestingCheatsAdvance { .. }
                        | Self::TestingCheatsModel { .. }
                        | Self::TestingCheatsClear
                        | Self::TestingCheatsFill
                        | Self::TestingCheatsFillHalf
                        | Self::TestingCheatsClassicTexture { .. }
                        | Self::TestingCheatsClassicExperiment { .. }
                        | Self::TestingCheatsStatus
                        | Self::TestingCheatsProvenance
                        | Self::TestingCheatsReset
                )
        }
        #[cfg(not(debug_assertions))]
        {
            ordinary
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum BalanceSelector {
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

pub(crate) fn parse(input: &str) -> Result<CommandIntent, String> {
    let tokens = tokenize(input)?;
    let Some((command, args)) = tokens.split_first() else {
        return Err(
            "Missing command. Try: status, start, stop, balance, x, datadir, configdir, timer"
                .to_string(),
        );
    };

    match command.trim_end_matches('=').to_ascii_lowercase().as_str() {
        "status" if args.is_empty() => Ok(CommandIntent::Status),
        "start" => parse_start(args),
        "stop" => parse_stop(args),
        "balance" => parse_balance(args),
        "x" => parse_delete_last_session(args),
        "datadir" if args.is_empty() => Ok(CommandIntent::DataDir),
        "configdir" if args.is_empty() => Ok(CommandIntent::ConfigDir),
        "timer" => Ok(CommandIntent::Timer {
            duration_seconds: parse_duration(args)?,
        }),
        #[cfg(debug_assertions)]
        "testingcheats" => parse_testing_cheats(args),
        _ => Err(format!("Invalid command or arguments: {input}")),
    }
}

#[cfg(debug_assertions)]
fn parse_testing_cheats(args: &[String]) -> Result<CommandIntent, String> {
    let Some((subcommand, rest)) = args.split_first() else {
        return Err("Usage: testingcheats help | model <h4|classic|hybrid|oslo-zero|oslo-box|oslo-vessel|oslo-vessel-momentum|oslo-vessel-front|oslo-vessel-front-flowviz|oslo-vessel-front-parcels|oslo-vessel-front-grains|oslo-vessel-fluid> | classic texture [baseline|textured|rugged|terraced] | classic experiment [rugged|memory|slope|memory-slope|anchored|momentum|momentum-repose|momentum-tangent|momentum-soft] | fallspeed [1x|4x|16x|64x|128x] | advance <duration> | fill | fillhalf | clear | status | provenance | reset".to_string());
    };

    match subcommand.to_ascii_lowercase().as_str() {
        "help" if rest.is_empty() => Ok(CommandIntent::TestingCheatsHelp),
        "fallspeed" if rest.is_empty() => {
            Ok(CommandIntent::TestingCheatsFallSpeed { multiplier: None })
        }
        "fallspeed" if rest.len() == 1 => {
            let raw = rest[0].trim().to_ascii_lowercase();
            let number = raw.strip_suffix('x').unwrap_or(&raw);
            let multiplier = number
                .parse::<u32>()
                .map_err(|_| "Usage: testingcheats fallspeed [1x|4x|16x|64x|128x]".to_string())?;
            if !matches!(multiplier, 1 | 4 | 16 | 64 | 128) {
                return Err("fallspeed must be one of 1x, 4x, 16x, 64x, or 128x".to_string());
            }
            Ok(CommandIntent::TestingCheatsFallSpeed {
                multiplier: Some(multiplier),
            })
        }
        "advance" if !rest.is_empty() => {
            let duration_seconds = parse_duration(rest)?;
            Ok(CommandIntent::TestingCheatsAdvance { duration_seconds })
        }
        "model" if rest.len() == 1 => {
            let model = rest[0].trim().to_ascii_lowercase();
            if !matches!(
                model.as_str(),
                "h4"
                    | "classic"
                    | "hybrid"
                    | "oslo-zero"
                    | "oslo-box"
                    | "oslo-vessel"
                    | "oslo-vessel-momentum"
                    | "oslo-vessel-front"
                    | "oslo-vessel-front-flowviz"
                    | "oslo-vessel-front-parcels"
                    | "oslo-vessel-front-grains"
                    | "oslo-vessel-fluid"
            ) {
                return Err(
                    "testingcheats model must be h4, classic, hybrid, oslo-zero, oslo-box, oslo-vessel, oslo-vessel-momentum, oslo-vessel-front, oslo-vessel-front-flowviz, oslo-vessel-front-parcels, oslo-vessel-front-grains, or oslo-vessel-fluid"
                        .to_string(),
                );
            }
            Ok(CommandIntent::TestingCheatsModel { model })
        }
        "classic" if rest.len() == 1 && rest[0].eq_ignore_ascii_case("texture") => {
            Ok(CommandIntent::TestingCheatsClassicTexture { profile: None })
        }
        "classic" if rest.len() == 2 && rest[0].eq_ignore_ascii_case("texture") => {
            let profile = rest[1].trim().to_ascii_lowercase();
            if !matches!(
                profile.as_str(),
                "baseline" | "textured" | "rugged" | "terraced"
            ) {
                return Err(
                    "classic texture profile must be baseline, textured, rugged, or terraced"
                        .to_string(),
                );
            }
            Ok(CommandIntent::TestingCheatsClassicTexture {
                profile: Some(profile),
            })
        }
        "classic" if rest.first().is_some_and(|token| token.eq_ignore_ascii_case("experiment")) => {
            let experiment = &rest[1..];
            if experiment.is_empty() {
                return Ok(CommandIntent::TestingCheatsClassicExperiment { profile: None });
            }
            if experiment.len() != 1 {
                return Err(
                    "Usage: testingcheats classic experiment [rugged|memory|slope|memory-slope|anchored|momentum|momentum-repose|momentum-tangent|momentum-soft]"
                        .to_string(),
                );
            }
            let profile = experiment[0].to_ascii_lowercase();
            if !matches!(
                profile.as_str(),
                "rugged"
                    | "memory"
                    | "slope"
                    | "memory-slope"
                    | "anchored"
                    | "momentum"
                    | "momentum-repose"
                    | "momentum-tangent"
                    | "momentum-soft"
            ) {
                return Err(
                    "classic experiment profile must be rugged, memory, slope, memory-slope, anchored, momentum, momentum-repose, momentum-tangent, or momentum-soft"
                        .to_string(),
                );
            }
            Ok(CommandIntent::TestingCheatsClassicExperiment {
                profile: Some(profile),
            })
        }
        "fill" if rest.is_empty() => Ok(CommandIntent::TestingCheatsFill),
        "fillhalf" if rest.is_empty() => Ok(CommandIntent::TestingCheatsFillHalf),
        "clear" if rest.is_empty() => Ok(CommandIntent::TestingCheatsClear),
        "status" if rest.is_empty() => Ok(CommandIntent::TestingCheatsStatus),
        "provenance" if rest.is_empty() => Ok(CommandIntent::TestingCheatsProvenance),
        "reset" if rest.is_empty() => Ok(CommandIntent::TestingCheatsReset),
        _ => Err("Usage: testingcheats help | model <h4|classic|hybrid|oslo-zero|oslo-box|oslo-vessel|oslo-vessel-momentum|oslo-vessel-front|oslo-vessel-front-flowviz|oslo-vessel-front-parcels|oslo-vessel-front-grains|oslo-vessel-fluid> | classic texture [baseline|textured|rugged|terraced] | classic experiment [rugged|memory|slope|memory-slope|anchored|momentum|momentum-repose|momentum-tangent|momentum-soft] | fallspeed [1x|4x|16x|64x|128x] | advance <duration> | fill | fillhalf | clear | status | provenance | reset".to_string()),
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

fn parse_balance(args: &[String]) -> Result<CommandIntent, String> {
    if args.is_empty() {
        return Ok(CommandIntent::Balance {
            selector: BalanceSelector::Today,
            layer: None,
            tag: None,
        });
    }

    let mut index = 0usize;
    let mut selector = BalanceSelector::Today;
    if let Some((parsed, consumed)) = parse_balance_selector(args) {
        selector = parsed;
        index = consumed;
    }
    let (layer, tag) = parse_balance_scope(&args[index..])?;
    Ok(CommandIntent::Balance {
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

fn parse_balance_scope(tokens: &[String]) -> Result<(Option<String>, Option<String>), String> {
    if tokens.is_empty() {
        return Ok((None, None));
    }
    let index = usize::from(tokens[0].eq_ignore_ascii_case("layer"));
    let scoped = &tokens[index..];
    if scoped.is_empty() {
        return Err("Expected layer name after 'layer'".to_string());
    }
    if scoped.len() > 2 {
        return Err("Usage: balance [time] [layer [tag]]".to_string());
    }
    Ok((scoped.first().cloned(), scoped.get(1).cloned()))
}

fn parse_balance_selector(tokens: &[String]) -> Option<(BalanceSelector, usize)> {
    let first = tokens.first()?.trim_end_matches('=').to_ascii_lowercase();
    let selector = match first.as_str() {
        "today" => Some((BalanceSelector::Today, 1)),
        "yesterday" => Some((BalanceSelector::Yesterday, 1)),
        "week" => Some((BalanceSelector::CurrentWeek, 1)),
        "lastweek" => Some((BalanceSelector::LastWeek, 1)),
        "month" => Some((BalanceSelector::CurrentMonth, 1)),
        "lastmonth" => Some((BalanceSelector::LastMonth, 1)),
        "monday" => Some((BalanceSelector::Weekday(Weekday::Mon), 1)),
        "tuesday" => Some((BalanceSelector::Weekday(Weekday::Tue), 1)),
        "wednesday" => Some((BalanceSelector::Weekday(Weekday::Wed), 1)),
        "thursday" => Some((BalanceSelector::Weekday(Weekday::Thu), 1)),
        "friday" => Some((BalanceSelector::Weekday(Weekday::Fri), 1)),
        "saturday" => Some((BalanceSelector::Weekday(Weekday::Sat), 1)),
        "sunday" => Some((BalanceSelector::Weekday(Weekday::Sun), 1)),
        _ => None,
    };
    if selector.is_some() {
        return selector;
    }
    if let Some(week) = parse_week_token(&first) {
        return Some((BalanceSelector::IsoWeek(week), 1));
    }
    if let Ok(date) = NaiveDate::parse_from_str(&first, "%Y-%m-%d") {
        return Some((BalanceSelector::Date(date), 1));
    }
    if let Some((month, day)) = parse_month_day_compact(&first) {
        return Some((BalanceSelector::MonthDay { month, day }, 1));
    }
    if tokens.len() >= 2
        && let Some(month) = parse_month_name(&first)
        && let Ok(day) = tokens[1].parse::<u32>()
        && (1..=31).contains(&day)
    {
        return Some((BalanceSelector::MonthDay { month, day }, 2));
    }
    None
}

pub(crate) fn resolve_balance_window(
    selector: &BalanceSelector,
    today: NaiveDate,
    first_day_of_week: FirstDayOfWeek,
) -> Result<ReportWindow, String> {
    match selector {
        BalanceSelector::Today => Ok(single_day(today)),
        BalanceSelector::Yesterday => Ok(single_day(today - ChronoDuration::days(1))),
        BalanceSelector::Weekday(target) => {
            let today_index = today.weekday().num_days_from_monday() as i64;
            let target_index = target.num_days_from_monday() as i64;
            Ok(single_day(
                today - ChronoDuration::days((7 + today_index - target_index) % 7),
            ))
        }
        BalanceSelector::CurrentWeek => {
            let start = start_of_week(today, first_day_of_week);
            Ok(window(start, today))
        }
        BalanceSelector::LastWeek => {
            let start = start_of_week(today, first_day_of_week) - ChronoDuration::days(7);
            Ok(window(start, start + ChronoDuration::days(6)))
        }
        BalanceSelector::CurrentMonth => Ok(window(today.with_day(1).unwrap_or(today), today)),
        BalanceSelector::LastMonth => {
            let previous_end = today.with_day(1).unwrap_or(today) - ChronoDuration::days(1);
            Ok(window(
                previous_end.with_day(1).unwrap_or(previous_end),
                previous_end,
            ))
        }
        BalanceSelector::IsoWeek(week) => {
            let start = NaiveDate::from_isoywd_opt(today.year(), *week, Weekday::Mon)
                .ok_or_else(|| format!("Invalid ISO week '{week}'"))?;
            Ok(window(start, start + ChronoDuration::days(6)))
        }
        BalanceSelector::MonthDay { month, day } => {
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
        BalanceSelector::Date(date) => Ok(single_day(*date)),
    }
}

fn single_day(day: NaiveDate) -> ReportWindow {
    ReportWindow::single_day(day)
}

fn window(start: NaiveDate, end: NaiveDate) -> ReportWindow {
    ReportWindow::new(start, end).expect("resolved balance window must be chronological")
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

    #[cfg(debug_assertions)]
    #[test]
    fn parses_testing_cheats_commands() {
        assert_eq!(
            parse("testingcheats help").unwrap(),
            CommandIntent::TestingCheatsHelp
        );
        assert_eq!(
            parse("testingcheats fallspeed").unwrap(),
            CommandIntent::TestingCheatsFallSpeed { multiplier: None }
        );
        assert_eq!(
            parse("testingcheats fallspeed 16x").unwrap(),
            CommandIntent::TestingCheatsFallSpeed {
                multiplier: Some(16)
            }
        );
        assert_eq!(
            parse("testingcheats fallspeed 128x").unwrap(),
            CommandIntent::TestingCheatsFallSpeed {
                multiplier: Some(128)
            }
        );
        assert_eq!(
            parse("testingcheats advance 30m").unwrap(),
            CommandIntent::TestingCheatsAdvance {
                duration_seconds: 1800
            }
        );
        assert_eq!(
            parse("testingcheats model classic").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "classic".into()
            }
        );
        assert_eq!(
            parse("testingcheats model hybrid").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "hybrid".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-zero").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-zero".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-box").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-box".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-vessel").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-vessel".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-vessel-momentum").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-vessel-momentum".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-vessel-front").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-vessel-front".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-vessel-front-flowviz").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-vessel-front-flowviz".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-vessel-front-parcels").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-vessel-front-parcels".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-vessel-front-grains").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-vessel-front-grains".into()
            }
        );
        assert_eq!(
            parse("testingcheats model oslo-vessel-fluid").unwrap(),
            CommandIntent::TestingCheatsModel {
                model: "oslo-vessel-fluid".into()
            }
        );
        assert_eq!(
            parse("testingcheats classic texture").unwrap(),
            CommandIntent::TestingCheatsClassicTexture { profile: None }
        );
        assert_eq!(
            parse("testingcheats classic texture baseline").unwrap(),
            CommandIntent::TestingCheatsClassicTexture {
                profile: Some("baseline".to_string())
            }
        );
        assert_eq!(
            parse("testingcheats classic texture textured").unwrap(),
            CommandIntent::TestingCheatsClassicTexture {
                profile: Some("textured".to_string())
            }
        );
        assert_eq!(
            parse("testingcheats classic texture rugged").unwrap(),
            CommandIntent::TestingCheatsClassicTexture {
                profile: Some("rugged".to_string())
            }
        );
        assert_eq!(
            parse("testingcheats classic texture terraced").unwrap(),
            CommandIntent::TestingCheatsClassicTexture {
                profile: Some("terraced".to_string())
            }
        );
        assert_eq!(
            parse("testingcheats classic experiment").unwrap(),
            CommandIntent::TestingCheatsClassicExperiment { profile: None }
        );
        for profile in [
            "rugged",
            "memory",
            "slope",
            "memory-slope",
            "anchored",
            "momentum",
            "momentum-repose",
            "momentum-tangent",
            "momentum-soft",
        ] {
            assert_eq!(
                parse(&format!("testingcheats classic experiment {profile}")).unwrap(),
                CommandIntent::TestingCheatsClassicExperiment {
                    profile: Some(profile.to_string())
                }
            );
        }
        assert!(parse("testingcheats classic experiment custom").is_err());
        assert!(parse("testingcheats classic texture custom").is_err());
        assert!(parse("testingcheats classic repose 20").is_err());
        assert_eq!(
            parse("testingcheats fill").unwrap(),
            CommandIntent::TestingCheatsFill
        );
        assert_eq!(
            parse("testingcheats fillhalf").unwrap(),
            CommandIntent::TestingCheatsFillHalf
        );
        assert_eq!(
            parse("testingcheats clear").unwrap(),
            CommandIntent::TestingCheatsClear
        );
        assert_eq!(
            parse("testingcheats status").unwrap(),
            CommandIntent::TestingCheatsStatus
        );
        assert_eq!(
            parse("testingcheats provenance").unwrap(),
            CommandIntent::TestingCheatsProvenance
        );
        assert_eq!(
            parse("testingcheats reset").unwrap(),
            CommandIntent::TestingCheatsReset
        );
        assert!(parse("testingcheats fallspeed 3x").is_err());
        assert_eq!(
            parse("testingcheats advance 24h").unwrap(),
            CommandIntent::TestingCheatsAdvance {
                duration_seconds: 24 * 60 * 60
            }
        );
        assert!(parse("testingcheats model h3").is_err());
        assert!(parse("testingcheats model oslo").is_err());
    }

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
    fn legacy_karma_command_is_not_part_of_balance_vocabulary() {
        assert!(parse("karma").is_err());
    }

    #[test]
    fn parses_balance_selectors_and_scope() {
        assert_eq!(
            parse("balance lastweek Work focus").unwrap(),
            CommandIntent::Balance {
                selector: BalanceSelector::LastWeek,
                layer: Some("Work".into()),
                tag: Some("focus".into()),
            }
        );
        assert_eq!(
            parse("balance 2026-08-20").unwrap(),
            CommandIntent::Balance {
                selector: BalanceSelector::Date(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()),
                layer: None,
                tag: None,
            }
        );
    }
}
