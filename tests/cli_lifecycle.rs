#![cfg(target_os = "linux")]

use chrono::{Duration as ChronoDuration, Utc};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const EMPTY_TIME_LOG: &str =
    "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n";

struct TestProfile {
    root: PathBuf,
    data_home: PathBuf,
    state_home: PathBuf,
    config_home: PathBuf,
}

impl TestProfile {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("strata-{name}-{}-{nonce}", std::process::id()));
        let data_home = root.join("data");
        let state_home = root.join("state");
        let config_home = root.join("config");

        fs::create_dir_all(data_home.join("strata"))
            .expect("test data directory should be created");
        fs::create_dir_all(&state_home).expect("test state directory should be created");
        fs::create_dir_all(&config_home).expect("test config directory should be created");

        fs::write(
            data_home.join("strata/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .expect("test categories should be written");

        Self {
            root,
            data_home,
            state_home,
            config_home,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_strata"));
        command
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command()
            .args(args)
            .output()
            .expect("Strata subprocess should run")
    }

    fn data_dir(&self) -> PathBuf {
        self.data_home.join("strata")
    }

    fn categories_path(&self) -> PathBuf {
        self.data_dir().join("categories.csv")
    }

    fn active_session_path(&self) -> PathBuf {
        self.state_home.join("strata/active_session.json")
    }

    fn time_log_path(&self) -> PathBuf {
        self.data_dir().join("time_log.csv")
    }

    fn backdate_active_session(&self, seconds: i64) {
        let path = self.active_session_path();
        let content = fs::read_to_string(&path).expect("active session should be readable");
        let mut value: serde_json::Value =
            serde_json::from_str(&content).expect("active session JSON should be valid");
        value["start_time"] =
            serde_json::Value::String((Utc::now() - ChronoDuration::seconds(seconds)).to_rfc3339());
        fs::write(
            path,
            serde_json::to_string_pretty(&value).expect("active session JSON should serialize"),
        )
        .expect("active session should be backdated");
    }
}

impl Drop for TestProfile {
    fn drop(&mut self) {
        let _ = fs::set_permissions(self.data_dir(), fs::Permissions::from_mode(0o755));
        let _ = fs::set_permissions(self.time_log_path(), fs::Permissions::from_mode(0o644));
        fs::remove_dir_all(&self.root).ok();
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_exists(path: &Path) {
    assert!(path.exists(), "expected {} to exist", path.display());
}

#[test]
fn start_stop_report_round_trip_uses_an_isolated_profile() {
    let profile = TestProfile::new("round-trip");

    let start = profile.run(&[
        "start",
        "study-session",
        "--category",
        "Work",
        "--desc",
        "Read chapter 4",
    ]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    assert_exists(&profile.active_session_path());
    profile.backdate_active_session(2);

    let stop = profile.run(&["stop"]);
    assert!(stop.status.success(), "stop failed: {}", stderr(&stop));
    assert!(!profile.active_session_path().exists());
    assert_exists(&profile.time_log_path());

    let log = fs::read_to_string(profile.time_log_path()).expect("time log should be readable");
    assert!(log.contains("Work"));
    assert!(log.contains("study-session"));
    assert!(log.contains("Read chapter 4"));

    let report = profile.run(&["report", "--today"]);
    assert!(
        report.status.success(),
        "report failed: {}",
        stderr(&report)
    );
    let stdout = String::from_utf8_lossy(&report.stdout);
    assert!(stdout.contains("Today's Report"));
    assert!(stdout.contains("Work"));

    let json = profile.run(&["export", "--format", "json"]);
    assert!(
        json.status.success(),
        "JSON export failed: {}",
        stderr(&json)
    );
    let exported: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("JSON export should parse");
    assert_eq!(exported["sessions"][0]["project"], "study-session");

    let ics = profile.run(&["export", "--format", "ics"]);
    assert!(ics.status.success(), "ICS export failed: {}", stderr(&ics));
    assert!(String::from_utf8_lossy(&ics.stdout).contains("SUMMARY:study-session - Work"));
}

#[test]
fn second_start_is_rejected_without_replacing_the_active_session() {
    let profile = TestProfile::new("duplicate-start");

    let first = profile.run(&["start", "first-project", "--category", "Work"]);
    assert!(
        first.status.success(),
        "first start failed: {}",
        stderr(&first)
    );

    let active_path = profile.active_session_path();
    let before = fs::read_to_string(&active_path).expect("active session should be readable");
    assert!(before.contains("first-project"));

    let second = profile.run(&["start", "replacement-project", "--category", "Work"]);
    assert!(!second.status.success());
    assert!(stderr(&second).contains("active session is already running"));

    let after = fs::read_to_string(&active_path).expect("active session should remain readable");
    assert_eq!(after, before);
    assert!(!after.contains("replacement-project"));
}

#[test]
fn repeated_stop_fails_without_adding_a_duplicate_session() {
    let profile = TestProfile::new("duplicate-stop");

    let start = profile.run(&["start", "single-project", "--category", "Work"]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    profile.backdate_active_session(2);

    let first_stop = profile.run(&["stop"]);
    assert!(
        first_stop.status.success(),
        "first stop failed: {}",
        stderr(&first_stop)
    );

    let before = fs::read_to_string(profile.time_log_path()).expect("time log should be readable");
    let second_stop = profile.run(&["stop"]);
    assert!(!second_stop.status.success());
    assert!(stderr(&second_stop).contains("No active session to stop"));

    let after =
        fs::read_to_string(profile.time_log_path()).expect("time log should remain readable");
    assert_eq!(after, before);
}

#[test]
fn project_only_start_is_rejected_and_idle_is_explicit() {
    let profile = TestProfile::new("explicit-classification");

    let omitted = profile.run(&["start", "project-only"]);
    assert!(!omitted.status.success());
    let omitted_error = stderr(&omitted);
    assert!(omitted_error.contains("--category <CATEGORY>"));
    assert!(!profile.active_session_path().exists());

    let idle = profile.run(&["start", "break", "--category", "idle"]);
    assert!(
        idle.status.success(),
        "idle start failed: {}",
        stderr(&idle)
    );
    assert!(String::from_utf8_lossy(&idle.stdout).contains("category 'idle'"));
    profile.backdate_active_session(2);
    let stop = profile.run(&["stop"]);
    assert!(stop.status.success(), "idle stop failed: {}", stderr(&stop));

    let report = profile.run(&["report", "--today"]);
    assert!(
        report.status.success(),
        "report failed: {}",
        stderr(&report)
    );
    let stdout = String::from_utf8_lossy(&report.stdout);
    assert!(stdout.contains("TOTAL"));
    assert!(stdout.contains("00:00:00"));
}

#[test]
fn corrupt_categories_fail_start_without_creating_active_state() {
    let profile = TestProfile::new("corrupt-categories");
    let corrupt = "name,description\nWork,focus\n";
    fs::write(profile.categories_path(), corrupt).expect("corrupt categories should be written");

    let start = profile.run(&["start", "unsafe-default", "--category", "Work"]);

    assert!(!start.status.success());
    assert!(stderr(&start).contains("Invalid CSV schema"));
    assert!(!profile.active_session_path().exists());
    assert_eq!(
        fs::read_to_string(profile.categories_path()).expect("categories should remain readable"),
        corrupt
    );
}

#[test]
fn corrupt_time_log_fails_stop_without_erasing_source_or_active_state() {
    let profile = TestProfile::new("corrupt-time-log");
    let start = profile.run(&["start", "recoverable-project", "--category", "Work"]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));

    let active_path = profile.active_session_path();
    let active_before =
        fs::read_to_string(&active_path).expect("active session should be readable");
    let corrupt = "date,category,elapsed\n2026-08-01,Work,120\n";
    fs::write(profile.time_log_path(), corrupt).expect("corrupt time log should be written");

    let stop = profile.run(&["stop"]);

    assert!(!stop.status.success());
    assert!(stderr(&stop).contains("Invalid CSV schema"));
    assert_eq!(
        fs::read_to_string(&active_path).expect("active session should remain readable"),
        active_before
    );
    assert_eq!(
        fs::read_to_string(profile.time_log_path()).expect("time log should remain readable"),
        corrupt
    );
}

#[test]
fn report_and_export_fail_on_corrupt_time_log_instead_of_showing_empty_history() {
    let profile = TestProfile::new("corrupt-readers");
    let corrupt = "broken,header\nvalue,value\n";
    fs::write(profile.time_log_path(), corrupt).expect("corrupt time log should be written");

    let report = profile.run(&["report", "--today"]);
    assert!(!report.status.success());
    assert!(stderr(&report).contains("Invalid CSV schema"));

    let export = profile.run(&["export", "--format", "json"]);
    assert!(!export.status.success());
    assert!(stderr(&export).contains("Invalid CSV schema"));

    assert_eq!(
        fs::read_to_string(profile.time_log_path()).expect("time log should remain readable"),
        corrupt
    );
}

#[test]
fn unwritable_time_log_fails_stop_without_consuming_active_state() {
    let profile = TestProfile::new("unwritable-time-log");
    let start = profile.run(&["start", "retryable-project", "--category", "Work"]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    profile.backdate_active_session(2);

    fs::write(profile.time_log_path(), EMPTY_TIME_LOG).expect("time log should be initialized");
    let active_path = profile.active_session_path();
    let active_before =
        fs::read_to_string(&active_path).expect("active session should be readable");
    let log_before =
        fs::read_to_string(profile.time_log_path()).expect("time log should be readable");

    fs::set_permissions(profile.time_log_path(), fs::Permissions::from_mode(0o444))
        .expect("time log should become read only");
    fs::set_permissions(profile.data_dir(), fs::Permissions::from_mode(0o555))
        .expect("data directory should become read only");

    let stop = profile.run(&["stop"]);

    fs::set_permissions(profile.data_dir(), fs::Permissions::from_mode(0o755))
        .expect("data directory permissions should be restored");
    fs::set_permissions(profile.time_log_path(), fs::Permissions::from_mode(0o644))
        .expect("time log permissions should be restored");

    assert!(!stop.status.success());
    assert_eq!(
        fs::read_to_string(&active_path).expect("active session should remain readable"),
        active_before
    );
    assert_eq!(
        fs::read_to_string(profile.time_log_path()).expect("time log should remain readable"),
        log_before
    );
}

#[test]
fn custom_ranges_and_active_projection_are_explicit() {
    let profile = TestProfile::new("report-projection");
    let start = profile.run(&[
        "start",
        "client-a",
        "--category",
        "Work",
        "--desc",
        "active work",
    ]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    profile.backdate_active_session(3);

    let today_date = Utc::now().date_naive();
    let from = (today_date - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let to = today_date.format("%Y-%m-%d").to_string();
    let active = profile.run(&["report", "--from", &from, "--to", &to]);
    assert!(
        active.status.success(),
        "custom report failed: {}",
        stderr(&active)
    );
    let active_stdout = String::from_utf8_lossy(&active.stdout);
    assert!(active_stdout.contains("Custom Report"));
    assert!(active_stdout.contains("Includes provisional active time"));
    assert!(
        active_stdout.contains("Work"),
        "custom report output:\n{active_stdout}"
    );

    let committed = profile.run(&["report", "--from", &from, "--to", &to, "--completed-only"]);
    assert!(
        committed.status.success(),
        "completed-only report failed: {}",
        stderr(&committed)
    );
    let committed_stdout = String::from_utf8_lossy(&committed.stdout);
    assert!(!committed_stdout.contains("Includes provisional active time"));
    assert!(committed_stdout.contains("00:00:00"));

    let json = profile.run(&["export", "--format", "json"]);
    assert!(
        json.status.success(),
        "active JSON export failed: {}",
        stderr(&json)
    );
    let exported: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(exported["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(exported["sessions"][0]["provisional"], true);
    assert_eq!(exported["sessions"][0]["project"], "client-a");

    let completed_json = profile.run(&["export", "--format", "json", "--completed-only"]);
    assert!(
        completed_json.status.success(),
        "completed JSON export failed: {}",
        stderr(&completed_json)
    );
    let completed_export: serde_json::Value =
        serde_json::from_slice(&completed_json.stdout).unwrap();
    assert!(completed_export["sessions"].as_array().unwrap().is_empty());

    let reversed = profile.run(&["report", "--from", "2026-07-15", "--to", "2026-07-01"]);
    assert!(!reversed.status.success());
    assert!(stderr(&reversed).contains("later than"));

    let incomplete = profile.run(&["report", "--from", "2026-07-01"]);
    assert!(!incomplete.status.success());
    assert!(stderr(&incomplete).contains("--to"));
}
