use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration as ChronoDuration, Local, Utc};
use clap::{CommandFactory, Parser, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::{
    constants::COLORS,
    domain::{
        CategoryId, DRIFT_CATEGORY_CONFIG_NAME, DayBoundaryMode, ReportPeriod, Session,
        build_period_report, operational_day_key_for_local, runtime_settings,
    },
    sqlite, storage,
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

    #[command(about = "Validate and publish a verified SQLite migration candidate")]
    MigrateSqlite {
        #[arg(
            long,
            help = "Validate without creating backups, reports, markers, or a database"
        )]
        dry_run: bool,

        #[arg(
            long,
            help = "Explicitly include an active or detached recovery interval"
        )]
        include_active_recovery: bool,

        #[arg(long, value_name = "PATH", help = "SQLite candidate output path")]
        database: Option<PathBuf>,

        #[arg(
            long,
            value_name = "PATH",
            help = "Machine-readable migration report path"
        )]
        report_out: Option<PathBuf>,

        #[arg(
            long,
            value_name = "SECONDS",
            help = "Fixed UTC offset used to reconstruct legacy wall-clock timestamps"
        )]
        utc_offset_seconds: Option<i32>,

        #[arg(
            long,
            value_name = "MINUTES",
            help = "Operational-day start as minutes after midnight"
        )]
        day_start_minutes: Option<u16>,

        #[arg(
            long,
            default_value_t = 1,
            value_name = "SECONDS",
            help = "Seconds represented by one sediment quantum"
        )]
        quantum_seconds: i64,

        #[arg(long, help = "Print the migration result as JSON")]
        json: bool,
    },

    #[command(about = "Activate a verified SQLite candidate for CLI runtime operations")]
    ActivateSqlite {
        #[arg(long, value_name = "PATH", help = "Verified SQLite candidate path")]
        database: Option<PathBuf>,

        #[arg(long, help = "Confirm the one-way CLI authority switch")]
        confirm: bool,

        #[arg(long, help = "Print the activation result as JSON")]
        json: bool,
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

        #[arg(long, value_name = "PATH", help = "New SQLite database path")]
        database: Option<PathBuf>,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Check SQLite integrity, schema, foreign keys, and authority metadata")]
    SqliteDoctor {
        #[arg(long, value_name = "PATH", help = "SQLite database path")]
        database: Option<PathBuf>,

        #[arg(long, value_name = "PATH", help = "Authority marker to validate")]
        authority_marker: Option<PathBuf>,

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
    match sqlite::resolve_runtime_authority()? {
        sqlite::RuntimeAuthority::LegacyFiles => {
            start_session_legacy(project, description, category_name)
        }
        sqlite::RuntimeAuthority::SqliteCli { database_path } => {
            let started =
                sqlite::start_cli_session(&database_path, project, description, category_name)?;
            println!(
                "Started session for project '{}' in category '{}'",
                started.project, started.category_name
            );
            Ok(())
        }
    }
}

fn start_session_legacy(
    project: String,
    description: Option<String>,
    category_name: Option<String>,
) -> Result<(), String> {
    let categories_path = storage::get_categories_path();
    let categories = storage::try_load_categories_from_csv(&categories_path)
        .map_err(|error| error.to_string())?
        .categories;

    let cat_name = category_name.unwrap_or_else(|| DRIFT_CATEGORY_CONFIG_NAME.to_string());
    let category = categories
        .iter()
        .find(|c| c.name == cat_name || c.id.0.to_string() == cat_name)
        .ok_or_else(|| format!("Category '{}' not found", cat_name))?;

    let session_path = storage::get_active_session_path();
    if storage::file_exists(&session_path) {
        return Err(
            "An active session is already running; stop it before starting another".to_string(),
        );
    }

    let session = ActiveSession {
        project: project.clone(),
        description: description.unwrap_or_default(),
        category_id: category.id.0,
        category_name: category.name.clone(),
        start_time: Utc::now(),
    };

    storage::write_json_atomic(&session_path, &session)?;

    println!(
        "Started session for project '{}' in category '{}'",
        project, category.name
    );
    Ok(())
}

pub fn stop_session() -> Result<usize, String> {
    match sqlite::resolve_runtime_authority()? {
        sqlite::RuntimeAuthority::LegacyFiles => stop_session_legacy(),
        sqlite::RuntimeAuthority::SqliteCli { database_path } => {
            let stopped = sqlite::stop_cli_session(&database_path)?;
            let elapsed = stopped.elapsed_seconds;
            println!(
                "Stopped session. Elapsed time: {:02}:{:02}:{:02}",
                elapsed / 3600,
                (elapsed % 3600) / 60,
                elapsed % 60
            );
            io::stdout().flush().map_err(|error| error.to_string())?;
            sqlite::acknowledge_cli_stop(&database_path, &stopped.operation_id)?;
            Ok(elapsed)
        }
    }
}

fn stop_session_legacy() -> Result<usize, String> {
    let session_path = storage::get_active_session_path();
    if !storage::file_exists(&session_path) {
        return Err("No active session to stop".to_string());
    }

    let active_session: ActiveSession = storage::read_json(&session_path)?;

    let elapsed = (Utc::now() - active_session.start_time).num_seconds() as usize;

    let sessions_path = storage::get_time_log_path();
    let categories_path = storage::get_categories_path();

    let categories = storage::try_load_categories_from_csv(&categories_path)
        .map_err(|error| error.to_string())?
        .categories;
    let mut sessions = storage::try_load_sessions_from_csv(&sessions_path, &categories)
        .map_err(|error| error.to_string())?
        .sessions;

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
    match sqlite::resolve_runtime_authority()? {
        sqlite::RuntimeAuthority::LegacyFiles => report_legacy(period),
        sqlite::RuntimeAuthority::SqliteCli { database_path } => {
            let snapshot = sqlite::read_cli_snapshot(&database_path)?;
            let sessions = snapshot
                .sessions
                .iter()
                .map(|session| session.as_domain_session())
                .collect::<Vec<_>>();
            let summary = build_period_report(&sessions, &snapshot.categories, period);
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
    }
}

fn report_legacy(period: ReportPeriod) -> Result<(), String> {
    let sessions_path = storage::get_time_log_path();
    let categories_path = storage::get_categories_path();

    let categories = storage::try_load_categories_from_csv(&categories_path)
        .map_err(|error| error.to_string())?
        .categories;
    let sessions = storage::try_load_sessions_from_csv(&sessions_path, &categories)
        .map_err(|error| error.to_string())?
        .sessions;

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
    match sqlite::resolve_runtime_authority()? {
        sqlite::RuntimeAuthority::LegacyFiles => export_data_legacy(format, out_path),
        sqlite::RuntimeAuthority::SqliteCli { database_path } => {
            export_data_sqlite(&database_path, format, out_path)
        }
    }
}

fn export_data_sqlite(
    database_path: &Path,
    format: ExportFormat,
    out_path: Option<PathBuf>,
) -> Result<(), String> {
    let snapshot = sqlite::read_cli_snapshot(database_path)?;
    let export = DataExport {
        schema_version: 1,
        exported_at: Utc::now(),
        categories: snapshot
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
                    karma_effect: category.karma_effect,
                }
            })
            .collect(),
        sessions: snapshot
            .sessions
            .iter()
            .map(|session| SessionExport {
                id: session.id,
                date: session.date.clone(),
                category_id: session.category_id,
                category_name: session.category_name.clone(),
                project: (!session.project.is_empty()).then(|| session.project.clone()),
                description: session.description.clone(),
                start_time: session.start_time.clone(),
                end_time: session.end_time.clone(),
                elapsed_seconds: session.elapsed_seconds,
            })
            .collect(),
    };

    match format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&export).map_err(|error| error.to_string())?;
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
                if session.category_name == DRIFT_CATEGORY_CONFIG_NAME
                    || session.elapsed_seconds == 0
                {
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

fn export_data_legacy(format: ExportFormat, out_path: Option<PathBuf>) -> Result<(), String> {
    let sessions_path = storage::get_time_log_path();
    let categories_path = storage::get_categories_path();

    let categories = storage::try_load_categories_from_csv(&categories_path)
        .map_err(|error| error.to_string())?
        .categories;
    let sessions = storage::try_load_sessions_from_csv(&sessions_path, &categories)
        .map_err(|error| error.to_string())?
        .sessions;

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
                    .unwrap_or(DRIFT_CATEGORY_CONFIG_NAME)
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
                if session.category_name == DRIFT_CATEGORY_CONFIG_NAME
                    || session.elapsed_seconds == 0
                {
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

#[allow(clippy::too_many_arguments)]
pub fn migrate_sqlite(
    dry_run: bool,
    include_active_recovery: bool,
    database: Option<PathBuf>,
    report_out: Option<PathBuf>,
    utc_offset_seconds: Option<i32>,
    day_start_minutes: Option<u16>,
    quantum_seconds: i64,
    json: bool,
) -> Result<(), String> {
    if dry_run && report_out.is_some() {
        return Err(
            "--report-out cannot be used with --dry-run; use --json for a machine-readable preview"
                .to_string(),
        );
    }

    let settings = runtime_settings();
    let resolved_day_start = match day_start_minutes {
        Some(value) => value,
        None => match settings.day_boundary.mode {
            DayBoundaryMode::FixedHour => {
                let minutes = settings
                    .day_boundary
                    .fixed_hour
                    .checked_mul(60)
                    .and_then(|value| value.checked_add(settings.day_boundary.fixed_minute))
                    .ok_or_else(|| "Configured operational-day boundary overflows".to_string())?;
                u16::try_from(minutes)
                    .map_err(|_| "Configured operational-day boundary is invalid".to_string())?
            }
            DayBoundaryMode::Sunrise => {
                return Err(
                    "Sunrise day boundaries require explicit --day-start-minutes for migration"
                        .to_string(),
                );
            }
        },
    };

    let report = sqlite::run_controlled_migration(sqlite::ControlledMigrationOptions {
        dry_run,
        include_active_recovery,
        database_path: database,
        report_path: report_out,
        utc_offset_seconds: utc_offset_seconds.unwrap_or(settings.day_boundary.utc_offset_seconds),
        operational_day_start_minutes: resolved_day_start,
        quantum_seconds,
    })
    .map_err(|error| error.to_string())?;

    if json {
        println!("{}", report.to_pretty_json()?);
    } else {
        report.print_human();
    }
    Ok(())
}

pub fn activate_sqlite(database: Option<PathBuf>, confirm: bool, json: bool) -> Result<(), String> {
    let report = sqlite::activate_sqlite_cli(sqlite::SqliteCliActivationOptions {
        database_path: database,
        confirm,
    })?;
    if json {
        println!("{}", report.to_pretty_json()?);
    } else {
        report.print_human();
    }
    Ok(())
}

fn default_sqlite_database_path() -> PathBuf {
    storage::get_data_dir().join("strata.sqlite3")
}

fn default_authority_marker_path() -> PathBuf {
    storage::get_state_dir().join("storage_authority.json")
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

pub fn sqlite_import(bundle: PathBuf, database: Option<PathBuf>, json: bool) -> Result<(), String> {
    let report = sqlite::run_bundle_import(sqlite::BundleImportOptions {
        bundle_directory: bundle,
        database_path: database.unwrap_or_else(default_sqlite_database_path),
    })?;
    print_maintenance_report(report, json)
}

pub fn sqlite_doctor(
    database: Option<PathBuf>,
    authority_marker: Option<PathBuf>,
    json: bool,
) -> Result<(), String> {
    let use_default_marker = database.is_none() && authority_marker.is_none();
    let report = sqlite::run_doctor(sqlite::DoctorOptions {
        database_path: database.unwrap_or_else(default_sqlite_database_path),
        authority_marker_path: authority_marker
            .or_else(|| use_default_marker.then(default_authority_marker_path)),
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
        Cli::MigrateSqlite {
            dry_run,
            include_active_recovery,
            database,
            report_out,
            utc_offset_seconds,
            day_start_minutes,
            quantum_seconds,
            json,
        } => {
            if let Err(e) = migrate_sqlite(
                dry_run,
                include_active_recovery,
                database,
                report_out,
                utc_offset_seconds,
                day_start_minutes,
                quantum_seconds,
                json,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }

        Cli::ActivateSqlite {
            database,
            confirm,
            json,
        } => {
            if let Err(error) = activate_sqlite(database, confirm, json) {
                eprintln!("Error: {}", error);
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
            database,
            json,
        } => {
            if let Err(error) = sqlite_import(bundle, database, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteDoctor {
            database,
            authority_marker,
            json,
        } => {
            if let Err(error) = sqlite_doctor(database, authority_marker, json) {
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
