from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


def sub_once(text: str, pattern: str, replacement: str, label: str) -> str:
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return result


# One profile-local database authority. No marker, activation, or fallback.
Path("src/sqlite/authority.rs").write_text('''use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{OptionalExtension, params};

use crate::{profile, storage};

use super::SqliteRepository;

const STORAGE_AUTHORITY: &str = "sqlite";
const PROFILE_ID_KEY: &str = "profile_id";

pub(crate) fn profile_database_path() -> PathBuf {
    storage::get_data_dir().join("strata.sqlite3")
}

pub(crate) fn resolve_runtime_database() -> Result<PathBuf, String> {
    let path = profile_database_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "cannot create SQLite profile directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let repository = SqliteRepository::open(&path).map_err(|error| error.to_string())?;
    validate_and_bind_profile(&repository)?;
    Ok(path)
}

pub(crate) fn open_cli_repository(path: &Path) -> Result<SqliteRepository, String> {
    let repository = SqliteRepository::open(path).map_err(|error| error.to_string())?;
    validate_and_bind_profile(&repository)?;
    Ok(repository)
}

fn validate_and_bind_profile(repository: &SqliteRepository) -> Result<(), String> {
    let authority = repository
        .metadata_value("storage_authority")
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| "missing".to_string());
    if authority != STORAGE_AUTHORITY {
        return Err(format!(
            "database storage authority is {authority}, expected {STORAGE_AUTHORITY}; remove or rebuild this development database"
        ));
    }

    let expected_profile = profile::profile_id();
    let existing: Option<String> = repository
        .connection
        .query_row(
            "SELECT value FROM database_metadata WHERE key = ?1",
            params![PROFILE_ID_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match existing {
        Some(found) if found != expected_profile => Err(format!(
            "SQLite database belongs to profile {found}, current profile is {expected_profile}"
        )),
        Some(_) => Ok(()),
        None => {
            repository
                .connection
                .execute(
                    "INSERT INTO database_metadata(key, value) VALUES (?1, ?2)",
                    params![PROFILE_ID_KEY, expected_profile],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        }
    }
}
''')

# SQLite module exports and new-database authority.
path = Path("src/sqlite.rs")
text = path.read_text()
text = sub_once(
    text,
    r"pub\(crate\) use authority::\{.*?\};",
    "pub(crate) use authority::{open_cli_repository, profile_database_path, resolve_runtime_database};",
    "authority exports",
)
text = replace_once(
    text,
    "VALUES ('storage_authority', 'sqlite-candidate');",
    "VALUES ('storage_authority', 'sqlite');",
    "new database authority",
)
path.write_text(text)

# CLI: remove transitional commands and file-backed runtime.
path = Path("src/cli.rs")
text = path.read_text()
text = sub_once(
    text,
    r"\n    #\[command\(about = \"Validate and publish a verified SQLite migration candidate\"\)\].*?\n    #\[command\(about = \"Export a deterministic portable bundle from SQLite\"\)\]",
    '\n    #[command(about = "Export a deterministic portable bundle from SQLite")]',
    "migration and activation commands",
)
text = sub_once(
    text,
    r"\n    #\[command\(about = \"Inventory verified legacy migration evidence\"\)\].*?\n    #\[command\(about = \"Generate shell completions\"\)\]",
    '\n    #[command(about = "Generate shell completions")]',
    "legacy evidence commands",
)
text = sub_once(
    text,
    r"\n#\[derive\(Debug, Clone, Serialize, Deserialize\)\]\npub struct ActiveSession \{.*?\n\}\n",
    "\n",
    "legacy active session model",
)
text = sub_once(
    text,
    r"pub fn start_session\(.*?\n\}\n\nfn start_session_legacy\(.*?\n\}\n\npub fn stop_session",
    '''pub fn start_session(
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
    let database_path = sqlite::resolve_runtime_database()?;
    let started = sqlite::start_cli_session(
        &database_path,
        project,
        description,
        category_name,
    )?;
    println!(
        "Started session for project '{}' in category '{}'",
        started.project, started.category_name
    );
    Ok(())
}

pub fn stop_session''',
    "start runtime",
)
text = sub_once(
    text,
    r"pub fn stop_session\(accept_clock_jump: bool\) -> Result<usize, String> \{.*?\n\}\n\nfn stop_session_legacy\(.*?\n\}\n\n#\[derive\(Clone, Copy\)\]",
    '''pub fn stop_session(accept_clock_jump: bool) -> Result<usize, String> {
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
    sqlite::acknowledge_cli_stop(&database_path, &stopped.operation_id)?;
    Ok(elapsed)
}

#[derive(Clone, Copy)]''',
    "stop runtime",
)
text = sub_once(
    text,
    r"pub fn report\(selection: ReportSelection, completed_only: bool\) -> Result<\(\), String> \{.*?\n\}\n\nfn report_sqlite",
    '''pub fn report(selection: ReportSelection, completed_only: bool) -> Result<(), String> {
    let database_path = sqlite::resolve_runtime_database()?;
    report_sqlite(&database_path, selection, completed_only)
}

fn report_sqlite''',
    "report runtime",
)
text = sub_once(
    text,
    r"\nfn report_legacy\(.*?\n\}\n\nfn print_report",
    "\nfn print_report",
    "legacy report",
)
text = sub_once(
    text,
    r"pub fn export_data\(.*?\n\}\n\nfn export_data_sqlite",
    '''pub fn export_data(
    format: ExportFormat,
    out_path: Option<PathBuf>,
    completed_only: bool,
) -> Result<(), String> {
    let database_path = sqlite::resolve_runtime_database()?;
    export_data_sqlite(&database_path, format, out_path, completed_only)
}

fn export_data_sqlite''',
    "export runtime",
)
text = sub_once(
    text,
    r"\nfn export_data_legacy\(.*?\n\}\n\nfn category_name\(.*?\n\}\n\nfn session_export_from_domain",
    "\nfn session_export_from_domain",
    "legacy export",
)
text = sub_once(
    text,
    r"\n#\[allow\(clippy::too_many_arguments\)\]\npub fn migrate_sqlite\(.*?\n\}\n\npub fn activate_sqlite\(.*?\n\}\n",
    "\n",
    "migration functions",
)
text = sub_once(
    text,
    r"\nfn default_authority_marker_path\(\) -> PathBuf \{.*?\n\}\n",
    "\n",
    "authority marker path",
)
text = sub_once(
    text,
    r"pub fn sqlite_doctor\(.*?\n\}\n\npub fn sqlite_backup",
    '''pub fn sqlite_doctor(database: Option<PathBuf>, json: bool) -> Result<(), String> {
    let report = sqlite::run_doctor(sqlite::DoctorOptions {
        database_path: database.unwrap_or_else(default_sqlite_database_path),
        authority_marker_path: None,
    })?;
    print_maintenance_report(report, json)
}

pub fn sqlite_backup''',
    "doctor marker removal",
)
text = sub_once(
    text,
    r"\nfn print_legacy_evidence_report\(.*?\n\}\n\npub fn sqlite_legacy_inventory\(.*?\n\}\n\npub fn sqlite_legacy_archive\(.*?\n\}\n\npub fn sqlite_legacy_remove\(.*?\n\}\n",
    "\n",
    "legacy evidence functions",
)
text = sub_once(
    text,
    r"\n        Cli::MigrateSqlite \{.*?\n        \}\n\n        Cli::ActivateSqlite \{.*?\n        \}\n",
    "\n",
    "migration command dispatch",
)
text = sub_once(
    text,
    r"\n        Cli::SqliteLegacyInventory \{.*?\n        \}\n        Cli::SqliteLegacyArchive \{.*?\n        \}\n        Cli::SqliteLegacyRemove \{.*?\n        \}\n",
    "\n",
    "legacy command dispatch",
)
text = replace_once(
    text,
    '''        Cli::SqliteDoctor {
            database,
            authority_marker,
            json,
        } => {
            if let Err(error) = sqlite_doctor(database, authority_marker, json) {''',
    '''        Cli::SqliteDoctor { database, json, .. } => {
            if let Err(error) = sqlite_doctor(database, json) {''',
    "doctor dispatch",
)
# Doctor command no longer exposes an authority marker.
text = sub_once(
    text,
    r"(SqliteDoctor \{\n        #\[arg\(long, value_name = \"PATH\", help = \"SQLite database path\"\)\]\n        database: Option<PathBuf>,)\n\n        #\[arg\(long, value_name = \"PATH\", help = \"Authority marker to validate\"\)\]\n        authority_marker: Option<PathBuf>,",
    r"\1",
    "doctor CLI marker",
)
path.write_text(text)

# TUI startup is always SQLite-backed.
path = Path("src/app.rs")
text = path.read_text()
text = sub_once(
    text,
    r"        let authority = sqlite::resolve_runtime_authority\(\)\?;\n        let \(\n            sqlite_database_path,\n            loaded_categories,\n            loaded_sessions,\n            mut category_tags,\n            archived_categories,\n            sqlite_active_session,\n        \) = match authority \{.*?\n        \};",
    '''        let database_path = sqlite::resolve_runtime_database()?;
        let state = sqlite::load_tui_state(&database_path)?;
        let sqlite_database_path = Some(database_path);
        let loaded_categories = state.loaded_categories;
        let loaded_sessions = state.loaded_sessions;
        let mut category_tags = state.category_tags;
        let archived_categories = state.archived_categories;
        let sqlite_active_session = state.active_session;''',
    "TUI authority load",
)
path.write_text(text)

# Transitional CLI evidence tests are replaced by direct SQLite runtime tests.
for obsolete in [
    "tests/legacy_cli_concurrency.rs",
    "tests/legacy_state_custody.rs",
    "tests/sqlite_cli_authority.rs",
]:
    Path(obsolete).unlink(missing_ok=True)

Path("tests/cli_lifecycle.rs").write_text('''#![cfg(target_os = "linux")]

use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::Connection;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct TestProfile {
    root: PathBuf,
}

impl TestProfile {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "strata-sqlite-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_strata"));
        command
            .arg("--profile")
            .arg(&self.root)
            .env_remove("STRATA_PROFILE")
            .env_remove("STRATA_DATA_DIR");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().expect("Strata should run")
    }

    fn database_path(&self) -> PathBuf {
        self.root.join("data/strata.sqlite3")
    }

    fn initialize(&self) {
        let output = self.run(&["report", "--today"]);
        assert!(
            output.status.success(),
            "initialization failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(self.database_path().exists());
    }

    fn insert_work_category(&self) {
        self.initialize();
        Connection::open(self.database_path())
            .unwrap()
            .execute(
                "INSERT INTO categories(id, name, description, color_index, balance_effect, sort_order)
                 VALUES (1, 'Work', '', 0, 1, 1)",
                [],
            )
            .unwrap();
    }

    fn backdate_active(&self, seconds: i64) {
        Connection::open(self.database_path())
            .unwrap()
            .execute(
                "UPDATE active_session SET started_at_utc = ?1 WHERE singleton = 1",
                [(Utc::now() - ChronoDuration::seconds(seconds)).to_rfc3339()],
            )
            .unwrap();
    }
}

impl Drop for TestProfile {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn fresh_profile_creates_only_sqlite_runtime_authority() {
    let profile = TestProfile::new("fresh");
    profile.initialize();
    assert!(profile.database_path().exists());
    assert!(!profile.root.join("data/categories.csv").exists());
    assert!(!profile.root.join("data/time_log.csv").exists());
    assert!(!profile.root.join("state/active_session.json").exists());
    assert!(!profile.root.join("state/storage_authority.json").exists());

    let connection = Connection::open(profile.database_path()).unwrap();
    let authority: String = connection
        .query_row(
            "SELECT value FROM database_metadata WHERE key = 'storage_authority'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(authority, "sqlite");
    let profile_id: String = connection
        .query_row(
            "SELECT value FROM database_metadata WHERE key = 'profile_id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!profile_id.is_empty());
}

#[test]
fn start_stop_report_and_export_use_one_database() {
    let profile = TestProfile::new("round-trip");
    profile.insert_work_category();
    let start = profile.run(&[
        "start",
        "study-session",
        "--category",
        "Work",
        "--desc",
        "Read chapter 4",
    ]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    profile.backdate_active(2);

    let stop = profile.run(&["stop"]);
    assert!(stop.status.success(), "stop failed: {}", stderr(&stop));

    let connection = Connection::open(profile.database_path()).unwrap();
    let completed: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(completed, 1);
    let active: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(active, 0);

    let report = profile.run(&["report", "--today"]);
    assert!(report.status.success(), "report failed: {}", stderr(&report));
    assert!(String::from_utf8_lossy(&report.stdout).contains("Work"));

    let export = profile.run(&["export", "--format", "json"]);
    assert!(export.status.success(), "export failed: {}", stderr(&export));
    let value: serde_json::Value = serde_json::from_slice(&export.stdout).unwrap();
    assert_eq!(value["sessions"][0]["project"], "study-session");
}

#[test]
fn duplicate_start_and_stop_fail_without_replacing_authority() {
    let profile = TestProfile::new("duplicates");
    profile.insert_work_category();
    let first = profile.run(&["start", "first", "--category", "Work"]);
    assert!(first.status.success(), "first start failed: {}", stderr(&first));
    let second = profile.run(&["start", "second", "--category", "Work"]);
    assert!(!second.status.success());

    let connection = Connection::open(profile.database_path()).unwrap();
    let project: String = connection
        .query_row("SELECT project FROM active_session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(project, "first");
    drop(connection);

    profile.backdate_active(2);
    assert!(profile.run(&["stop"]).status.success());
    let repeated = profile.run(&["stop"]);
    assert!(!repeated.status.success());
    let connection = Connection::open(profile.database_path()).unwrap();
    let completed: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(completed, 1);
}

#[test]
fn database_profile_binding_fails_closed_after_copy() {
    let source = TestProfile::new("source");
    source.initialize();
    let target = TestProfile::new("target");
    target.initialize();
    fs::copy(source.database_path(), target.database_path()).unwrap();

    let output = target.run(&["report", "--today"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("database belongs to profile"));
}
''')

print("SQLite-only runtime first pass applied")
