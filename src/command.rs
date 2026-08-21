use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CommandIntent {
    Status,
    Start {
        layer: String,
        tag: Option<String>,
    },
    Stop,
    Karma {
        period: Option<String>,
    },
    DeleteLastSession {
        layer: String,
    },
    DataDir,
    ConfigDir,
    Timer {
        duration_seconds: u64,
    },
    #[cfg(debug_assertions)]
    TestingCheatsHalfFull,
}

pub(crate) fn parse(input: &str) -> Result<CommandIntent, String> {
    let tokens = tokenize(input)?;
    let Some((command, args)) = tokens.split_first() else {
        return Err(
            "Missing command. Try: status, start, stop, karma, x, datadir, configdir, timer".into(),
        );
    };

    match command.trim_end_matches('=').to_ascii_lowercase().as_str() {
        "status" if args.is_empty() => Ok(CommandIntent::Status),
        "start" if (1..=2).contains(&args.len()) => Ok(CommandIntent::Start {
            layer: args[0].clone(),
            tag: args.get(1).cloned(),
        }),
        "stop" if args.is_empty() => Ok(CommandIntent::Stop),
        "karma" if args.len() <= 1 => Ok(CommandIntent::Karma {
            period: args.first().cloned(),
        }),
        "x" if args.len() == 2 && args[1].eq_ignore_ascii_case("lastsession") => {
            Ok(CommandIntent::DeleteLastSession {
                layer: args[0].clone(),
            })
        }
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
        let unit = &compact[unit_start..index];
        let multiplier = match unit {
            "s" | "sec" | "secs" | "second" | "seconds" => 1,
            "m" | "min" | "mins" | "minute" | "minutes" => 60,
            "h" | "hr" | "hrs" | "hour" | "hours" => 3600,
            _ => return Err(format!("Unsupported duration unit '{unit}'")),
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
    let _ = Duration::from_secs(total);
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_start_and_duration() {
        assert_eq!(
            parse("start Work \"deep focus\"").unwrap(),
            CommandIntent::Start {
                layer: "Work".into(),
                tag: Some("deep focus".into()),
            }
        );
        assert_eq!(
            parse("timer 1h30m").unwrap(),
            CommandIntent::Timer {
                duration_seconds: 5400
            }
        );
    }
}
