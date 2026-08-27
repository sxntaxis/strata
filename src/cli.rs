use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, NaiveDate, Utc};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::{
    command::CommandIntent,
    constants::COLORS,
    domain::{
        CategoryId, OperationalDayPolicy, ReportPeriod, ReportWindow, Session, build_period_report,
        build_report_for_window, civil_time_for_utc, day_boundary_config,
        operational_day_key_for_utc,
    },
    profile, sqlite, storage,
};

#[derive(Parser, Debug)]
#[command(name = "strata")]
#[command(about = "Time tracking with falling sand", long_about = None)]
pub struct Invocation {
    #[arg(
        long,
        global = true,
        help = "Deliberately ignore keymap.json and use built-in defaults"
    )]
    pub ignore_config: bool,

    #[arg(
        long,
        global = true,
        value_name = "DIRECTORY",
        help = "Select one complete process-lifetime profile root"
    )]
    pub profile: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Cli>,
}

#[derive(Subcommand, Debug)]
pub enum Cli {
    #[command(about = "Show the selected profile identity and complete authority paths")]
    Profile {
        #[arg(long, help = "Print the profile description as JSON")]
        json: bool,
    },

    #[command(about = "Show the active runtime status")]
    Status,

    #[command(about = "Start or switch to a tracking layer")]
    Start {
        #[arg(help = "Layer name or ID; use 'idle' explicitly for baseline time")]
        layer: String,

        #[arg(long, help = "Session description/tag")]
        desc: Option<String>,
    },

    #[command(about = "Stop the current tracking session")]
    Stop {
        #[arg(
            long,
            help = "Explicitly accept a wall-clock interval above the unattended safety limit"
        )]
        accept_clock_jump: bool,
    },

    #[command(about = "Show a time report")]
    Report {
        #[arg(
            long,
            help = "Show today's time",
            conflicts_with_all = ["week", "month", "from", "to"]
        )]
        today: bool,

        #[arg(
            long,
            help = "Show the current operational week",
            conflicts_with_all = ["today", "month", "from", "to"]
        )]
        week: bool,

        #[arg(
            long,
            help = "Show the current calendar month",
            conflicts_with_all = ["today", "week", "from", "to"]
        )]
        month: bool,

        #[arg(
            long,
            value_name = "YYYY-MM-DD",
            requires = "to",
            conflicts_with_all = ["today", "week", "month"],
            help = "Inclusive first operational day"
        )]
        from: Option<NaiveDate>,

        #[arg(
            long,
            value_name = "YYYY-MM-DD",
            requires = "from",
            conflicts_with_all = ["today", "week", "month"],
            help = "Inclusive last operational day"
        )]
        to: Option<NaiveDate>,

        #[arg(long, help = "Exclude the provisional active interval")]
        completed_only: bool,
    },

    #[command(about = "Export completed and provisional sessions")]
    Export {
        #[arg(long, value_enum, help = "Export format")]
        format: ExportFormat,

        #[arg(long, short, help = "Output path")]
        out: Option<PathBuf>,

        #[arg(long, help = "Exclude the provisional active interval")]
        completed_only: bool,
    },

    #[command(about = "Export a deterministic portable bundle from SQLite")]
    SqliteExport {
        #[arg(long, value_name = "PATH", help = "SQLite database path")]
        database: Option<PathBuf>,

        #[arg(long, value_name = "DIRECTORY", help = "New portable bundle directory")]
        out: PathBuf,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Import a validated portable bundle into a new SQLite database")]
    SqliteImport {
        #[arg(long, value_name = "DIRECTORY", help = "Portable bundle directory")]
        bundle: PathBuf,

        #[arg(
            long,
            help = "Validate the complete import without publishing a database"
        )]
        dry_run: bool,

        #[arg(long, value_name = "PATH", help = "New SQLite database path")]
        database: Option<PathBuf>,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Check SQLite integrity, schema, foreign keys, and profile binding")]
    SqliteDoctor {
        #[arg(long, value_name = "PATH", help = "SQLite database path")]
        database: Option<PathBuf>,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Create a verified standalone SQLite backup")]
    SqliteBackup {
        #[arg(long, value_name = "PATH", help = "SQLite database path")]
        database: Option<PathBuf>,

        #[arg(long, value_name = "PATH", help = "New backup database path")]
        out: PathBuf,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Verify and atomically restore a SQLite backup")]
    SqliteRestore {
        #[arg(long, value_name = "PATH", help = "Verified backup database path")]
        backup: PathBuf,

        #[arg(long, value_name = "PATH", help = "SQLite database path")]
        database: Option<PathBuf>,

        #[arg(long, help = "Preserve and replace an existing target database")]
        replace: bool,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
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
pub struct SessionExport {
    pub id: Option<usize>,
    pub uid: String,
    pub provisional: bool,
    pub date: String,
    pub category_id: u64,
    pub category_name: String,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: usize,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub ended_at_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryExport {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub color_index: usize,
    pub balance_effect: i8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExport {
    pub schema_version: u32,
    pub exported_at: DateTime<Utc>,
    pub categories: Vec<CategoryExport>,
    pub sessions: Vec<SessionExport>,
}

pub fn start_session(layer: String, description: Option<String>) -> Result<(), String> {
    if layer.trim().is_empty() {
        return Err("Layer name cannot be empty".to_string());
    }
    #[cfg(unix)]
    if let Some(response) = crate::ipc::send(&CommandIntent::Start {
        layer: layer.clone(),
        tag: description.clone(),
    })? {
        println!("{response}");
        return Ok(());
    }
    let database_path = sqlite::resolve_runtime_database()?;
    let started = sqlite::start_cli_session(&database_path, description, layer)?;
    println!("Started layer '{}'", started.category_name);
    Ok(())
}

pub fn stop_session(accept_clock_jump: bool) -> Result<usize, String> {
    #[cfg(unix)]
    if let Some(response) = crate::ipc::send(&CommandIntent::Stop {
        layer: None,
        tag: None,
    })? {
        println!("{response}");
        return Ok(0);
    }
    let database_path = sqlite::resolve_runtime_database()?;
    let stopped = sqlite::stop_cli_session(&database_path, accept_clock_jump)?;
    let elapsed = stopped.elapsed_seconds;
    println!(
        "Stopped session. Elapsed time: {:02}:{:02}:{:02}",
        elapsed / 3600,
        (elapsed % 3600) / 60,
        elapsed % 60
    );
    io::stdout().flush().map_err(|error| error.to_string())?;
    Ok(elapsed)
}

pub fn status() -> Result<(), String> {
    #[cfg(unix)]
    if let Some(response) = crate::ipc::send(&CommandIntent::Status)? {
        println!("{response}");
        return Ok(());
    }
    let snapshot = sqlite::read_cli_snapshot(&sqlite::resolve_runtime_database()?)?;
    if let Some(active) = snapshot.active_session {
        if active.category_id == 0 {
            println!("Status: idle since {}", active.started_at_utc);
        } else if active.description.trim().is_empty() {
            println!(
                "Status: active layer '{}' since {}",
                active.category_name, active.started_at_utc
            );
        } else {
            println!(
                "Status: active layer '{}' tag '{}' since {}",
                active.category_name, active.description, active.started_at_utc
            );
        }
    } else {
        println!("Status: no active session");
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub enum ReportSelection {
    Preset(ReportPeriod),
    Custom { start: NaiveDate, end: NaiveDate },
}

pub fn report(selection: ReportSelection, completed_only: bool) -> Result<(), String> {
    let database_path = sqlite::resolve_runtime_database()?;
    report_sqlite(&database_path, selection, completed_only)
}

fn report_sqlite(
    database_path: &Path,
    selection: ReportSelection,
    completed_only: bool,
) -> Result<(), String> {
    let snapshot = sqlite::read_cli_snapshot(database_path)?;
    let snapshot_at = Utc::now();
    let mut sessions = snapshot
        .sessions
        .iter()
        .map(|session| session.as_domain_session())
        .collect::<Vec<_>>();
    if !completed_only
        && let Some(active) = snapshot.active_session.as_ref()
        && let Some(provisional) = provisional_session(
            active.category_id,
            &active.description,
            active.started_at_utc,
            snapshot_at,
            sessions.iter().map(|session| session.id).max().unwrap_or(0) + 1,
        )?
    {
        sessions.push(provisional);
    }
    print_report(&sessions, &snapshot.categories, selection, !completed_only)
}

fn print_report(
    sessions: &[Session],
    categories: &[crate::domain::Category],
    selection: ReportSelection,
    includes_provisional: bool,
) -> Result<(), String> {
    let (title, summary) = match selection {
        ReportSelection::Preset(period) => {
            let title = match period {
                ReportPeriod::Today => "Today's Report".to_string(),
                ReportPeriod::Week => "Weekly Report".to_string(),
                ReportPeriod::Month => "Monthly Report".to_string(),
            };
            (title, build_period_report(sessions, categories, period))
        }
        ReportSelection::Custom { start, end } => {
            let window = ReportWindow::new(start, end).map_err(|_| {
                format!("Invalid report range: --from {start} is later than --to {end}")
            })?;
            (
                "Custom Report".to_string(),
                build_report_for_window(sessions, categories, &window),
            )
        }
    };
    println!("{} ({})", title, summary.date);
    if includes_provisional {
        println!("Includes provisional active time; use --completed-only to exclude it.");
    }
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

fn provisional_session(
    category_id: u64,
    description: &str,
    started_at_utc: DateTime<Utc>,
    snapshot_at: DateTime<Utc>,
    id: usize,
) -> Result<Option<Session>, String> {
    let elapsed = snapshot_at
        .signed_duration_since(started_at_utc)
        .num_seconds();
    if elapsed < 0 {
        return Err(
            "Active session starts in the future; provisional projection refused".to_string(),
        );
    }
    if elapsed == 0 {
        return Ok(None);
    }
    let elapsed_seconds = usize::try_from(elapsed)
        .map_err(|_| "Active session duration exceeds this platform's limits".to_string())?;
    let policy = OperationalDayPolicy::from_config(day_boundary_config());
    let start_civil = civil_time_for_utc(started_at_utc);
    let end_civil = civil_time_for_utc(snapshot_at);
    Ok(Some(Session {
        id,
        date: operational_day_key_for_utc(snapshot_at)
            .format("%Y-%m-%d")
            .to_string(),
        category_id: CategoryId::new(category_id),
        description: description.to_string(),
        start_time: start_civil.format("%H:%M:%S").to_string(),
        end_time: end_civil.format("%H:%M:%S").to_string(),
        elapsed_seconds,
        started_at_utc: Some(started_at_utc),
        ended_at_utc: Some(snapshot_at),
        operational_day_policy: Some(policy),
    }))
}

pub fn export_data(
    format: ExportFormat,
    out_path: Option<PathBuf>,
    completed_only: bool,
) -> Result<(), String> {
    let database_path = sqlite::resolve_runtime_database()?;
    export_data_sqlite(&database_path, format, out_path, completed_only)
}

fn export_data_sqlite(
    database_path: &Path,
    format: ExportFormat,
    out_path: Option<PathBuf>,
    completed_only: bool,
) -> Result<(), String> {
    let snapshot = sqlite::read_cli_snapshot(database_path)?;
    let snapshot_at = Utc::now();
    let mut sessions = snapshot
        .sessions
        .iter()
        .map(|session| SessionExport {
            id: Some(session.id),
            uid: format!("{}@strata", session.stable_id),
            provisional: false,
            date: session.date.clone(),
            category_id: session.category_id,
            category_name: session.category_name.clone(),
            description: session.description.clone(),
            start_time: session.start_time.clone(),
            end_time: session.end_time.clone(),
            elapsed_seconds: session.elapsed_seconds,
            started_at_utc: Some(session.started_at_utc),
            ended_at_utc: Some(session.ended_at_utc),
        })
        .collect::<Vec<_>>();
    if !completed_only
        && let Some(active) = snapshot.active_session.as_ref()
        && let Some(session) = provisional_session(
            active.category_id,
            &active.description,
            active.started_at_utc,
            snapshot_at,
            0,
        )?
    {
        sessions.push(session_export_from_domain(
            session,
            active.category_name.clone(),
            format!("{}@strata", active.stable_id),
            true,
        ));
    }
    let mut categories = snapshot
        .categories
        .iter()
        .filter(|category| category.id.0 != 0)
        .map(|category| {
            let color_pos = COLORS
                .iter()
                .position(|&color| color == category.color)
                .unwrap_or(0);
            CategoryExport {
                id: category.id.0,
                name: category.name.clone(),
                description: category.description.clone(),
                color_index: color_pos,
                balance_effect: category.balance_effect,
            }
        })
        .collect::<Vec<_>>();
    sort_exports(&mut categories, &mut sessions);
    write_export(
        DataExport {
            schema_version: 4,
            exported_at: snapshot_at,
            categories,
            sessions,
        },
        format,
        out_path,
    )
}

fn session_export_from_domain(
    session: Session,
    category_name: String,
    uid: String,
    provisional: bool,
) -> SessionExport {
    SessionExport {
        id: (!provisional).then_some(session.id),
        uid,
        provisional,
        date: session.date,
        category_id: session.category_id.0,
        category_name,
        description: session.description,
        start_time: session.start_time,
        end_time: session.end_time,
        elapsed_seconds: session.elapsed_seconds,
        started_at_utc: session.started_at_utc,
        ended_at_utc: session.ended_at_utc,
    }
}

fn sort_exports(categories: &mut [CategoryExport], sessions: &mut [SessionExport]) {
    categories.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    sessions.sort_by(|a, b| {
        a.started_at_utc
            .cmp(&b.started_at_utc)
            .then_with(|| a.ended_at_utc.cmp(&b.ended_at_utc))
            .then_with(|| a.uid.cmp(&b.uid))
    });
}

fn write_export(
    export: DataExport,
    format: ExportFormat,
    out_path: Option<PathBuf>,
) -> Result<(), String> {
    let rendered = match format {
        ExportFormat::Json => {
            serde_json::to_string_pretty(&export).map_err(|error| error.to_string())?
        }
        ExportFormat::Ics => render_ics(&export)?,
    };
    if let Some(path) = out_path {
        storage::write_text_file(&path, &rendered)?;
        println!("Exported to {}", path.display());
    } else {
        print!("{rendered}");
        if !rendered.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}

fn render_ics(export: &DataExport) -> Result<String, String> {
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
        "PRODID:-//sxntaxis//Strata//EN".to_string(),
    ];
    for session in &export.sessions {
        if session.category_id == 0 || session.elapsed_seconds == 0 {
            continue;
        }
        let started = session.started_at_utc.ok_or_else(|| {
            format!(
                "Session {} lacks authoritative UTC chronology and cannot be exported as ICS",
                session.uid
            )
        })?;
        let ended = session.ended_at_utc.ok_or_else(|| {
            format!(
                "Session {} lacks authoritative UTC chronology and cannot be exported as ICS",
                session.uid
            )
        })?;
        let summary = session.category_name.clone();
        lines.push("BEGIN:VEVENT".to_string());
        lines.push(format!("UID:{}", escape_ics_text(&session.uid)));
        lines.push(format!(
            "DTSTAMP:{}",
            format_ics_timestamp(export.exported_at)
        ));
        lines.push(format!("DTSTART:{}", format_ics_timestamp(started)));
        lines.push(format!("DTEND:{}", format_ics_timestamp(ended)));
        lines.push(format!("SUMMARY:{}", escape_ics_text(&summary)));
        if !session.description.is_empty() {
            lines.push(format!(
                "DESCRIPTION:{}",
                escape_ics_text(&session.description)
            ));
        }
        lines.push(format!(
            "CATEGORIES:{}",
            escape_ics_text(&session.category_name)
        ));
        if session.provisional {
            lines.push("X-STRATA-PROVISIONAL:TRUE".to_string());
        }
        lines.push("END:VEVENT".to_string());
    }
    lines.push("END:VCALENDAR".to_string());
    let mut output = String::new();
    for line in lines {
        output.push_str(&fold_ics_line(&line));
        output.push_str("\r\n");
    }
    Ok(output)
}

fn escape_ics_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace("\r\n", "\\n")
        .replace(['\r', '\n'], "\\n")
        .replace(';', "\\;")
        .replace(',', "\\,")
}

fn fold_ics_line(line: &str) -> String {
    const LIMIT: usize = 75;
    if line.len() <= LIMIT {
        return line.to_string();
    }
    let mut output = String::new();
    let mut remaining = line;
    let mut first = true;
    while !remaining.is_empty() {
        let allowance = if first { LIMIT } else { LIMIT - 1 };
        let mut end = remaining.len().min(allowance);
        while !remaining.is_char_boundary(end) {
            end -= 1;
        }
        if !first {
            output.push_str("\r\n ");
        }
        output.push_str(&remaining[..end]);
        remaining = &remaining[end..];
        first = false;
    }
    output
}

fn format_ics_timestamp(dt: DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}

fn default_sqlite_database_path() -> PathBuf {
    storage::get_data_dir().join("strata.sqlite3")
}

fn print_maintenance_report(
    report: sqlite::SqliteMaintenanceReport,
    json: bool,
) -> Result<(), String> {
    let healthy = report.is_healthy();
    if json {
        println!("{}", report.to_pretty_json()?);
    } else {
        report.print_human();
    }
    if healthy {
        Ok(())
    } else {
        Err("SQLite doctor reported an unhealthy database".to_string())
    }
}

pub fn sqlite_export(database: Option<PathBuf>, out: PathBuf, json: bool) -> Result<(), String> {
    let report = sqlite::run_bundle_export(sqlite::BundleExportOptions {
        database_path: database.unwrap_or_else(default_sqlite_database_path),
        output_directory: out,
    })?;
    print_maintenance_report(report, json)
}

pub fn sqlite_import(
    bundle: PathBuf,
    database: Option<PathBuf>,
    dry_run: bool,
    json: bool,
) -> Result<(), String> {
    let database_path = match database {
        Some(path) => path,
        None if dry_run => PathBuf::new(),
        None => default_sqlite_database_path(),
    };
    let report = sqlite::run_bundle_import(sqlite::BundleImportOptions {
        bundle_directory: bundle,
        database_path,
        dry_run,
    })?;
    print_maintenance_report(report, json)
}

pub fn sqlite_doctor(database: Option<PathBuf>, json: bool) -> Result<(), String> {
    let report = sqlite::run_doctor(sqlite::DoctorOptions {
        database_path: database.unwrap_or_else(default_sqlite_database_path),
    })?;
    print_maintenance_report(report, json)
}

pub fn sqlite_backup(database: Option<PathBuf>, out: PathBuf, json: bool) -> Result<(), String> {
    let report = sqlite::run_backup(sqlite::BackupOptions {
        database_path: database.unwrap_or_else(default_sqlite_database_path),
        backup_path: out,
    })?;
    print_maintenance_report(report, json)
}

pub fn sqlite_restore(
    backup: PathBuf,
    database: Option<PathBuf>,
    replace: bool,
    json: bool,
) -> Result<(), String> {
    let report = sqlite::run_restore(sqlite::RestoreOptions {
        backup_path: backup,
        database_path: database.unwrap_or_else(default_sqlite_database_path),
        replace,
    })?;
    print_maintenance_report(report, json)
}

pub fn show_profile(json: bool) -> Result<(), String> {
    let description = profile::describe();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&description).map_err(|error| error.to_string())?
        );
    } else {
        println!(
            "Profile ID: {}",
            description["profile_id"].as_str().unwrap_or("unknown")
        );
        println!(
            "Root: {}",
            description["root"].as_str().unwrap_or("XDG default")
        );
        println!(
            "Data: {}",
            description["data_dir"].as_str().unwrap_or("unknown")
        );
        println!(
            "State: {}",
            description["state_dir"].as_str().unwrap_or("unknown")
        );
        println!(
            "Config: {}",
            description["config_dir"].as_str().unwrap_or("unknown")
        );
        println!("Switching: exit Strata and invoke again with --profile <directory>");
    }
    Ok(())
}

pub fn parse_invocation() -> Invocation {
    Invocation::parse()
}

pub fn print_completions(shell: &str) -> Result<(), String> {
    use clap_complete::Shell;
    match shell {
        "bash" => {
            clap_complete::generate(
                Shell::Bash,
                &mut Invocation::command(),
                "strata",
                &mut io::stdout(),
            );
        }
        "zsh" => {
            clap_complete::generate(
                Shell::Zsh,
                &mut Invocation::command(),
                "strata",
                &mut io::stdout(),
            );
        }
        "fish" => {
            clap_complete::generate(
                Shell::Fish,
                &mut Invocation::command(),
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

pub fn run_command(cli: Cli) {
    match cli {
        Cli::Profile { json } => {
            if let Err(error) = show_profile(json) {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
        }
        Cli::Status => {
            if let Err(error) = status() {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
        }
        Cli::Start { layer, desc } => {
            if let Err(e) = start_session(layer, desc) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Cli::Stop { accept_clock_jump } => {
            if let Err(e) = stop_session(accept_clock_jump) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Cli::Report {
            week,
            month,
            from,
            to,
            completed_only,
            ..
        } => {
            let selection = match (from, to) {
                (Some(start), Some(end)) => ReportSelection::Custom { start, end },
                (None, None) => ReportSelection::Preset(if month {
                    ReportPeriod::Month
                } else if week {
                    ReportPeriod::Week
                } else {
                    ReportPeriod::Today
                }),
                _ => unreachable!("clap requires --from and --to together"),
            };
            if let Err(e) = report(selection, completed_only) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Cli::Export {
            format,
            out,
            completed_only,
        } => {
            if let Err(e) = export_data(format, out, completed_only) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }

        Cli::SqliteExport {
            database,
            out,
            json,
        } => {
            if let Err(error) = sqlite_export(database, out, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteImport {
            bundle,
            dry_run,
            database,
            json,
        } => {
            if let Err(error) = sqlite_import(bundle, database, dry_run, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteDoctor { database, json, .. } => {
            if let Err(error) = sqlite_doctor(database, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteBackup {
            database,
            out,
            json,
        } => {
            if let Err(error) = sqlite_backup(database, out, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteRestore {
            backup,
            database,
            replace,
            json,
        } => {
            if let Err(error) = sqlite_restore(backup, database, replace, json) {
                eprintln!("Error: {}", error);
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

#[cfg(test)]
mod report_export_tests {
    use super::*;

    fn sample_export() -> DataExport {
        DataExport {
            schema_version: 4,
            exported_at: DateTime::parse_from_rfc3339("2026-08-02T03:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            categories: vec![],
            sessions: vec![SessionExport {
                id: Some(7),
                uid: "session-7@strata".to_string(),
                provisional: true,
                date: "2026-08-01".to_string(),
                category_id: 1,
                category_name: "Work, Deep; Focus".to_string(),
                description: "line one\\line two\nthird line with a long Unicode description café música 日本語 that must fold safely without splitting UTF-8".to_string(),
                start_time: "20:30:00".to_string(),
                end_time: "21:30:00".to_string(),
                elapsed_seconds: 3600,
                started_at_utc: Some(
                    DateTime::parse_from_rfc3339("2026-08-02T02:30:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                ),
                ended_at_utc: Some(
                    DateTime::parse_from_rfc3339("2026-08-02T03:30:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                ),
            }],
        }
    }

    #[test]
    fn ics_uses_authoritative_utc_and_escapes_text() {
        let ics = render_ics(&sample_export()).expect("ICS should render");
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        assert!(ics.contains("DTSTART:20260802T023000Z\r\n"));
        assert!(ics.contains("DTEND:20260802T033000Z\r\n"));
        assert!(ics.contains("SUMMARY:Work\\, Deep\\; Focus\r\n"));
        assert!(ics.contains("DESCRIPTION:line one\\\\line two\\nthird line"));
        assert!(ics.contains("X-STRATA-PROVISIONAL:TRUE\r\n"));
        assert!(!ics.contains("SUMMARY:Project"));
        for physical in ics.split("\r\n").filter(|line| !line.is_empty()) {
            assert!(
                physical.len() <= 75,
                "unfolded line exceeds 75 octets: {physical}"
            );
        }
    }

    #[test]
    fn deterministic_export_sort_has_complete_tie_breakers() {
        let mut categories = vec![
            CategoryExport {
                id: 2,
                name: "beta".into(),
                description: String::new(),
                color_index: 0,
                balance_effect: 1,
            },
            CategoryExport {
                id: 1,
                name: "Alpha".into(),
                description: String::new(),
                color_index: 0,
                balance_effect: 1,
            },
        ];
        let mut sessions = vec![
            SessionExport {
                uid: "b@strata".into(),
                ..sample_export().sessions[0].clone()
            },
            SessionExport {
                uid: "a@strata".into(),
                ..sample_export().sessions[0].clone()
            },
        ];
        sort_exports(&mut categories, &mut sessions);
        assert_eq!(
            categories.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(
            sessions.iter().map(|s| s.uid.as_str()).collect::<Vec<_>>(),
            vec!["a@strata", "b@strata"]
        );
    }
}
