use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, SecondsFormat, Utc};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::{
    constants::COLORS,
    domain::{
        CategoryId, DRIFT_CATEGORY_CONFIG_NAME, OperationalDayPolicy, ReportPeriod, Session,
        build_period_report, build_report_for_date_range, civil_time_for_utc, day_boundary_config,
        operational_day_key_for_utc, runtime_settings,
    },
    sqlite, storage, temporal,
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

    #[command(subcommand)]
    pub command: Option<Cli>,
}

#[derive(Subcommand, Debug)]
pub enum Cli {
    #[command(about = "Start a new tracking session")]
    Start {
        #[arg(help = "Project name")]
        project: String,

        #[arg(long, help = "Session description")]
        desc: Option<String>,

        #[arg(
            long,
            short,
            help = "Required category name or ID; use 'idle' explicitly for baseline time"
        )]
        category: String,
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

    #[command(about = "Inventory verified legacy migration evidence")]
    SqliteLegacyInventory {
        #[arg(long, value_name = "PATH", help = "Storage authority marker path")]
        authority_marker: Option<PathBuf>,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Archive verified legacy migration evidence")]
    SqliteLegacyArchive {
        #[arg(long, value_name = "DIRECTORY", help = "New archive directory")]
        out: PathBuf,

        #[arg(long, value_name = "PATH", help = "Storage authority marker path")]
        authority_marker: Option<PathBuf>,

        #[arg(long, help = "Confirm archive publication")]
        confirm: bool,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Remove legacy files after a verified archive exists")]
    SqliteLegacyRemove {
        #[arg(
            long,
            value_name = "DIRECTORY",
            help = "Verified legacy evidence archive"
        )]
        archive: PathBuf,

        #[arg(long, value_name = "PATH", help = "Storage authority marker path")]
        authority_marker: Option<PathBuf>,

        #[arg(
            long,
            value_name = "FINGERPRINT",
            help = "Exact migration fingerprint confirming irreversible removal"
        )]
        confirm_fingerprint: String,

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
    pub id: Option<usize>,
    pub uid: String,
    pub provisional: bool,
    pub date: String,
    pub category_id: u64,
    pub category_name: String,
    pub project: Option<String>,
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
    category_name: String,
) -> Result<(), String> {
    if project.trim().is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    if category_name.trim().is_empty() {
        return Err("Category is required; use --category idle for baseline time".to_string());
    }
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
    category_name: String,
) -> Result<(), String> {
    let categories_path = storage::get_categories_path();
    let categories = storage::try_load_categories_from_csv(&categories_path)
        .map_err(|error| error.to_string())?
        .categories;

    let requested = category_name.trim();
    let category = if crate::domain::is_drift_name(requested) || requested == "0" {
        categories
            .iter()
            .find(|category| category.id == CategoryId::new(0))
    } else {
        categories.iter().find(|category| {
            category.name.eq_ignore_ascii_case(requested) || category.id.0.to_string() == requested
        })
    }
    .ok_or_else(|| format!("Category '{requested}' not found"))?;

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

pub fn stop_session(accept_clock_jump: bool) -> Result<usize, String> {
    match sqlite::resolve_runtime_authority()? {
        sqlite::RuntimeAuthority::LegacyFiles => stop_session_legacy(accept_clock_jump),
        sqlite::RuntimeAuthority::SqliteCli { database_path } => {
            let stopped = sqlite::stop_cli_session(&database_path, accept_clock_jump)?;
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

fn stop_session_legacy(accept_clock_jump: bool) -> Result<usize, String> {
    let session_path = storage::get_active_session_path();
    if !storage::file_exists(&session_path) {
        return Err("No active session to stop".to_string());
    }

    let active_session: ActiveSession = storage::read_json(&session_path)?;

    let now_utc = Utc::now();
    let interval =
        temporal::checked_wall_interval(active_session.start_time, now_utc, accept_clock_jump)?;
    let elapsed = interval.elapsed_seconds;

    let sessions_path = storage::get_time_log_path();
    let categories_path = storage::get_categories_path();

    let categories = storage::try_load_categories_from_csv(&categories_path)
        .map_err(|error| error.to_string())?
        .categories;
    let mut sessions = storage::try_load_sessions_from_csv(&sessions_path, &categories)
        .map_err(|error| error.to_string())?
        .sessions;

    let now = civil_time_for_utc(interval.ended_at_utc);
    let today = operational_day_key_for_utc(interval.ended_at_utc)
        .format("%Y-%m-%d")
        .to_string();
    let start_time = now - ChronoDuration::seconds(elapsed as i64);

    if elapsed > 0 {
        let new_id = sessions.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        sessions.push(Session {
            id: new_id,
            date: today,
            category_id: CategoryId::new(active_session.category_id),
            project: active_session.project.clone(),
            description: active_session.description.clone(),
            start_time: start_time.format("%H:%M:%S").to_string(),
            end_time: now.format("%H:%M:%S").to_string(),
            elapsed_seconds: elapsed,
            started_at_utc: Some(active_session.start_time),
            ended_at_utc: Some(interval.ended_at_utc),
            operational_day_policy: Some(OperationalDayPolicy::from_config(day_boundary_config())),
        });
        storage::save_sessions_to_csv(&sessions_path, &sessions, &categories)?;
    }

    storage::delete_file_if_exists(&session_path)?;

    println!(
        "Stopped session. Elapsed time: {:02}:{:02}:{:02}",
        elapsed / 3600,
        (elapsed % 3600) / 60,
        elapsed % 60
    );
    Ok(elapsed)
}

#[derive(Clone, Copy)]
pub enum ReportSelection {
    Preset(ReportPeriod),
    Custom { start: NaiveDate, end: NaiveDate },
}

pub fn report(selection: ReportSelection, completed_only: bool) -> Result<(), String> {
    match sqlite::resolve_runtime_authority()? {
        sqlite::RuntimeAuthority::LegacyFiles => report_legacy(selection, completed_only),
        sqlite::RuntimeAuthority::SqliteCli { database_path } => {
            report_sqlite(&database_path, selection, completed_only)
        }
    }
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
            &active.project,
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

fn report_legacy(selection: ReportSelection, completed_only: bool) -> Result<(), String> {
    let sessions_path = storage::get_time_log_path();
    let categories_path = storage::get_categories_path();
    let categories = storage::try_load_categories_from_csv(&categories_path)
        .map_err(|error| error.to_string())?
        .categories;
    let mut sessions = storage::try_load_sessions_from_csv(&sessions_path, &categories)
        .map_err(|error| error.to_string())?
        .sessions;
    if !completed_only {
        let active_path = storage::get_active_session_path();
        if storage::file_exists(&active_path) {
            let active: ActiveSession = storage::read_json(&active_path)?;
            if let Some(provisional) = provisional_session(
                &active.project,
                active.category_id,
                &active.description,
                active.start_time,
                Utc::now(),
                sessions.iter().map(|session| session.id).max().unwrap_or(0) + 1,
            )? {
                sessions.push(provisional);
            }
        }
    }
    print_report(&sessions, &categories, selection, !completed_only)
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
            if start > end {
                return Err(format!(
                    "Invalid report range: --from {start} is later than --to {end}"
                ));
            }
            let label = format!("{start}..{end}");
            (
                "Custom Report".to_string(),
                build_report_for_date_range(sessions, categories, start, end, label),
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
    project: &str,
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
        project: project.to_string(),
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
    match sqlite::resolve_runtime_authority()? {
        sqlite::RuntimeAuthority::LegacyFiles => {
            export_data_legacy(format, out_path, completed_only)
        }
        sqlite::RuntimeAuthority::SqliteCli { database_path } => {
            export_data_sqlite(&database_path, format, out_path, completed_only)
        }
    }
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
            project: (!session.project.is_empty()).then(|| session.project.clone()),
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
            &active.project,
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
                karma_effect: category.karma_effect,
            }
        })
        .collect::<Vec<_>>();
    sort_exports(&mut categories, &mut sessions);
    write_export(
        DataExport {
            schema_version: 2,
            exported_at: snapshot_at,
            categories,
            sessions,
        },
        format,
        out_path,
    )
}

fn export_data_legacy(
    format: ExportFormat,
    out_path: Option<PathBuf>,
    completed_only: bool,
) -> Result<(), String> {
    let categories_path = storage::get_categories_path();
    let categories = storage::try_load_categories_from_csv(&categories_path)
        .map_err(|error| error.to_string())?
        .categories;
    let sessions_path = storage::get_time_log_path();
    let completed = storage::try_load_sessions_from_csv(&sessions_path, &categories)
        .map_err(|error| error.to_string())?
        .sessions;
    let snapshot_at = Utc::now();
    let mut sessions = completed
        .into_iter()
        .map(|session| {
            let category_name = category_name(&categories, session.category_id);
            let uid = format!("legacy-session-{}@strata", session.id);
            session_export_from_domain(session, category_name, uid, false)
        })
        .collect::<Vec<_>>();
    if !completed_only {
        let active_path = storage::get_active_session_path();
        if storage::file_exists(&active_path) {
            let active: ActiveSession = storage::read_json(&active_path)?;
            if let Some(session) = provisional_session(
                &active.project,
                active.category_id,
                &active.description,
                active.start_time,
                snapshot_at,
                0,
            )? {
                sessions.push(session_export_from_domain(
                    session,
                    active.category_name,
                    format!(
                        "legacy-active-{}@strata",
                        active
                            .start_time
                            .to_rfc3339_opts(SecondsFormat::Nanos, true)
                    ),
                    true,
                ));
            }
        }
    }
    let mut category_exports = categories
        .iter()
        .filter(|category| category.id.0 != 0)
        .map(|category| CategoryExport {
            id: category.id.0,
            name: category.name.clone(),
            description: category.description.clone(),
            color_index: COLORS
                .iter()
                .position(|&color| color == category.color)
                .unwrap_or(0),
            karma_effect: category.karma_effect,
        })
        .collect::<Vec<_>>();
    sort_exports(&mut category_exports, &mut sessions);
    write_export(
        DataExport {
            schema_version: 2,
            exported_at: snapshot_at,
            categories: category_exports,
            sessions,
        },
        format,
        out_path,
    )
}

fn category_name(categories: &[crate::domain::Category], category_id: CategoryId) -> String {
    categories
        .iter()
        .find(|category| category.id == category_id)
        .map(|category| category.name.clone())
        .unwrap_or_else(|| DRIFT_CATEGORY_CONFIG_NAME.to_string())
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
        project: (!session.project.is_empty()).then_some(session.project),
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
        let summary = match session.project.as_deref() {
            Some(project) => format!("{project} - {}", session.category_name),
            None => session.category_name.clone(),
        };
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
        None => {
            let minutes = settings
                .day_boundary
                .fixed_hour
                .checked_mul(60)
                .and_then(|value| value.checked_add(settings.day_boundary.fixed_minute))
                .ok_or_else(|| "Configured operational-day boundary overflows".to_string())?;
            u16::try_from(minutes)
                .map_err(|_| "Configured operational-day boundary is invalid".to_string())?
        }
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

fn print_legacy_evidence_report(
    report: sqlite::LegacyEvidenceReport,
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
        Err("legacy evidence differs from the verified migration backup".to_string())
    }
}

pub fn sqlite_legacy_inventory(
    authority_marker: Option<PathBuf>,
    json: bool,
) -> Result<(), String> {
    let report = sqlite::run_legacy_evidence_inventory(sqlite::LegacyEvidenceInventoryOptions {
        authority_marker_path: authority_marker.unwrap_or_else(default_authority_marker_path),
    })?;
    print_legacy_evidence_report(report, json)
}

pub fn sqlite_legacy_archive(
    out: PathBuf,
    authority_marker: Option<PathBuf>,
    confirm: bool,
    json: bool,
) -> Result<(), String> {
    let report = sqlite::run_legacy_evidence_archive(sqlite::LegacyEvidenceArchiveOptions {
        authority_marker_path: authority_marker.unwrap_or_else(default_authority_marker_path),
        output_directory: out,
        confirm,
    })?;
    print_legacy_evidence_report(report, json)
}

pub fn sqlite_legacy_remove(
    archive: PathBuf,
    authority_marker: Option<PathBuf>,
    confirm_fingerprint: String,
    json: bool,
) -> Result<(), String> {
    let report = sqlite::run_legacy_evidence_remove(sqlite::LegacyEvidenceRemoveOptions {
        authority_marker_path: authority_marker.unwrap_or_else(default_authority_marker_path),
        archive_directory: archive,
        confirm_fingerprint,
    })?;
    print_legacy_evidence_report(report, json)
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
            dry_run,
            database,
            json,
        } => {
            if let Err(error) = sqlite_import(bundle, database, dry_run, json) {
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

        Cli::SqliteLegacyInventory {
            authority_marker,
            json,
        } => {
            if let Err(error) = sqlite_legacy_inventory(authority_marker, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteLegacyArchive {
            out,
            authority_marker,
            confirm,
            json,
        } => {
            if let Err(error) = sqlite_legacy_archive(out, authority_marker, confirm, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteLegacyRemove {
            archive,
            authority_marker,
            confirm_fingerprint,
            json,
        } => {
            if let Err(error) =
                sqlite_legacy_remove(archive, authority_marker, confirm_fingerprint, json)
            {
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
