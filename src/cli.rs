use std::{io, path::PathBuf};

use chrono::{DateTime, Duration as ChronoDuration, Local, Utc};
use clap::{CommandFactory, Parser, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::{
    constants::COLORS,
    domain::{
        CategoryId, ReportPeriod, Session, build_period_report, operational_day_key_for_local,
    },
    storage,
};

#[derive(Parser, Debug)]
#[command(name = "strata")]
#[command(about = "Time tracking with falling sand", long_about = None)]
pub enum Cli {
    #[command(about = "Start a new tracking session")]
    Start {
        #[arg(help = "Project name")]
        project: String,

        #[arg(long, help = "Session description")]
        desc: Option<String>,

        #[arg(long, short, help = "Category name or ID")]
        category: Option<String>,
    },

    #[command(about = "Stop the current tracking session")]
    Stop,

    #[command(about = "Show a time report")]
    Report {
        #[arg(
            long,
            help = "Show today's time",
            conflicts_with_all = ["week", "month"]
        )]
        today: bool,

        #[arg(
            long,
            help = "Show last 7 days",
            conflicts_with_all = ["today", "month"]
        )]
        week: bool,

        #[arg(
            long,
            help = "Show last 30 days",
            conflicts_with_all = ["today", "week"]
        )]
        month: bool,
    },

    #[command(about = "Export sessions")]
    Export {
        #[arg(long, value_enum, help = "Export format")]
        format: ExportFormat,

        #[arg(long, short, help = "Output path")]
        out: Option<PathBuf>,
    },

    #[command(about = "Generate shell completions")]
    Completions {
        #[arg(help = "Shell type (bash, zsh, fish)")]
        shell: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ExportFormat {
    Json,
    Ics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    pub project: String,
    pub description: String,
    pub category_id: u64,
    pub category_name: String,
    pub start_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExport {
    pub id: usize,
    pub date: String,
    pub category_id: u64,
    pub category_name: String,
    pub project: Option<String>,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryExport {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub color_index: usize,
    pub karma_effect: i8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExport {
    pub schema_version: u32,
    pub exported_at: DateTime<Utc>,
    pub categories: Vec<CategoryExport>,
    pub sessions: Vec<SessionExport>,
}

pub fn start_session(
    project: String,
    description: Option<String>,
    category_name: Option<String>,
) -> Result<(), String> {
    let data_dir = storage::get_data_dir();
    let categories_path = data_dir.join("categories.csv");
    let categories = storage::load_categories_from_csv(&categories_path).categories;

    let cat_name = category_name.unwrap_or_else(|| "none".to_string());
    let category = categories
        .iter()
        .find(|c| c.name == cat_name || c.id.0.to_string() == cat_name)
        .ok_or_else(|| format!("Category '{}' not found", cat_name))?;

    let session = ActiveSession {
        project: project.clone(),
        description: description.unwrap_or_default(),
        category_id: category.id.0,
        category_name: category.name.clone(),
        start_time: Utc::now(),
    };

    let session_path = storage::get_active_session_path();
    storage::write_json_atomic(&session_path, &session)?;

    println!(
        "Started session for project '{}' in category '{}'",
        project, category.name
    );
    Ok(())
}

pub fn stop_session() -> Result<usize, String> {
    let session_path = storage::get_active_session_path();
    if !storage::file_exists(&session_path) {
        return Err("No active session to stop".to_string());
    }

    let active_session: ActiveSession = storage::read_json(&session_path)?;

    let elapsed = (Utc::now() - active_session.start_time).num_seconds() as usize;

    let data_dir = storage::get_data_dir();
    let sessions_path = data_dir.join("time_log.csv");
    let categories_path = data_dir.join("categories.csv");

    let categories = storage::load_categories_from_csv(&categories_path).categories;
    let mut sessions = storage::load_sessions_from_csv(&sessions_path, &categories).sessions;

    let now = Local::now();
    let today = operational_day_key_for_local(&now)
        .format("%Y-%m-%d")
        .to_string();
    let start_time = now - ChronoDuration::seconds(elapsed as i64);

    let new_id = sessions.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    sessions.push(Session {
        id: new_id,
        date: today,
        category_id: CategoryId::new(active_session.category_id),
        description: active_session.description.clone(),
        start_time: start_time.format("%H:%M:%S").to_string(),
        end_time: now.format("%H:%M:%S").to_string(),
        elapsed_seconds: elapsed,
    });

    storage::save_sessions_to_csv(&sessions_path, &sessions, &categories)?;

    storage::delete_file_if_exists(&session_path)?;

    println!(
        "Stopped session. Elapsed time: {:02}:{:02}:{:02}",
        elapsed / 3600,
        (elapsed % 3600) / 60,
        elapsed % 60
    );
    Ok(elapsed)
}

pub fn report(period: ReportPeriod) -> Result<(), String> {
    let data_dir = storage::get_data_dir();
    let sessions_path = data_dir.join("time_log.csv");
    let categories_path = data_dir.join("categories.csv");

    let categories = storage::load_categories_from_csv(&categories_path).categories;
    let sessions = storage::load_sessions_from_csv(&sessions_path, &categories).sessions;

    let summary = build_period_report(&sessions, &categories, period);

    let title = match period {
        ReportPeriod::Today => "Today's Report",
        ReportPeriod::Week => "Weekly Report",
        ReportPeriod::Month => "Monthly Report",
    };

    println!("{} ({})", title, summary.date);
    println!("{}", "-".repeat(40));
    for entry in &summary.entries {
        println!(
            "{:20} {:02}:{:02}:{:02}",
            entry.category_name,
            entry.elapsed_seconds / 3600,
            (entry.elapsed_seconds % 3600) / 60,
            entry.elapsed_seconds % 60
        );
    }
    println!("{}", "-".repeat(40));
    println!(
        "{:20} {:02}:{:02}:{:02}",
        "TOTAL",
        summary.total_seconds / 3600,
        (summary.total_seconds % 3600) / 60,
        summary.total_seconds % 60
    );

    Ok(())
}

pub fn export_data(format: ExportFormat, out_path: Option<PathBuf>) -> Result<(), String> {
    let data_dir = storage::get_data_dir();
    let sessions_path = data_dir.join("time_log.csv");
    let categories_path = data_dir.join("categories.csv");

    let categories = storage::load_categories_from_csv(&categories_path).categories;
    let sessions = storage::load_sessions_from_csv(&sessions_path, &categories).sessions;

    let export = DataExport {
        schema_version: 1,
        exported_at: Utc::now(),
        categories: categories
            .iter()
            .skip(1)
            .map(|c| {
                let color_pos = COLORS.iter().position(|&col| col == c.color).unwrap_or(0);
                CategoryExport {
                    id: c.id.0,
                    name: c.name.clone(),
                    description: c.description.clone(),
                    color_index: color_pos,
                    karma_effect: c.karma_effect,
                }
            })
            .collect(),
        sessions: sessions
            .iter()
            .map(|s| {
                let cat_name = categories
                    .iter()
                    .find(|c| c.id == s.category_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("none")
                    .to_string();
                SessionExport {
                    id: s.id,
                    date: s.date.clone(),
                    category_id: s.category_id.0,
                    category_name: cat_name,
                    project: None,
                    description: s.description.clone(),
                    start_time: s.start_time.clone(),
                    end_time: s.end_time.clone(),
                    elapsed_seconds: s.elapsed_seconds,
                }
            })
            .collect(),
    };

    match format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
            if let Some(path) = out_path {
                storage::write_text_file(&path, &json)?;
                println!("Exported to {}", path.display());
            } else {
                println!("{}", json);
            }
        }
        ExportFormat::Ics => {
            let mut ics = String::new();
            ics.push_str("BEGIN:VCALENDAR\r\n");
            ics.push_str("VERSION:2.0\r\n");
            ics.push_str("PRODID:-//strata//time tracking//EN\r\n");

            for session in &export.sessions {
                if session.category_name == "none" || session.elapsed_seconds == 0 {
                    continue;
                }
                let dt_start = format_ics_datetime(&session.date, &session.start_time);
                let dt_end = format_ics_datetime(&session.date, &session.end_time);
                let uid = format!("strata-session-{}", session.id);

                ics.push_str("BEGIN:VEVENT\r\n");
                ics.push_str(&format!("UID:{}\r\n", uid));
                ics.push_str(&format!("DTSTAMP:{}\r\n", format_ics_timestamp(Utc::now())));
                ics.push_str(&format!("DTSTART:{}\r\n", dt_start));
                ics.push_str(&format!("DTEND:{}\r\n", dt_end));
                ics.push_str(&format!(
                    "SUMMARY:{} - {}\r\n",
                    session.project.as_deref().unwrap_or("Project"),
                    session.category_name
                ));
                if !session.description.is_empty() {
                    ics.push_str(&format!("DESCRIPTION:{}\r\n", session.description));
                }
                ics.push_str(&format!("CATEGORIES:{}\r\n", session.category_name));
                ics.push_str("END:VEVENT\r\n");
            }

            ics.push_str("END:VCALENDAR\r\n");

            if let Some(path) = out_path {
                storage::write_text_file(&path, &ics)?;
                println!("Exported to {}", path.display());
            } else {
                println!("{}", ics);
            }
        }
    }

    Ok(())
}

fn format_ics_datetime(date: &str, time: &str) -> String {
    format!("{}T{}00", date.replace('-', ""), time.replace(':', ""))
}

fn format_ics_timestamp(dt: DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}

pub fn print_completions(shell: &str) -> Result<(), String> {
    use clap_complete::Shell;
    match shell {
        "bash" => {
            clap_complete::generate(
                Shell::Bash,
                &mut Cli::command(),
                "strata",
                &mut io::stdout(),
            );
        }
        "zsh" => {
            clap_complete::generate(Shell::Zsh, &mut Cli::command(), "strata", &mut io::stdout());
        }
        "fish" => {
            clap_complete::generate(
                Shell::Fish,
                &mut Cli::command(),
                "strata",
                &mut io::stdout(),
            );
        }
        _ => {
            return Err(format!(
                "Unsupported shell: {}. Use bash, zsh, or fish.",
                shell
            ));
        }
    }
    Ok(())
}

pub fn run_cli() {
    let cli = Cli::parse();
    match cli {
        Cli::Start {
            project,
            desc,
            category,
        } => {
            if let Err(e) = start_session(project, desc, category) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Cli::Stop => {
            if let Err(e) = stop_session() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Cli::Report { week, month, .. } => {
            let period = if month {
                ReportPeriod::Month
            } else if week {
                ReportPeriod::Week
            } else {
                ReportPeriod::Today
            };

            if let Err(e) = report(period) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Cli::Export { format, out } => {
            if let Err(e) = export_data(format, out) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Cli::Completions { shell } => {
            if let Err(e) = print_completions(&shell) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
