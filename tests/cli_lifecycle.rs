#![cfg(target_os = "linux")]

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

    let stop = profile.run(&["stop"]);
    assert!(stop.status.success(), "stop failed: {}", stderr(&stop));
    assert!(!profile.active_session_path().exists());
    assert_exists(&profile.time_log_path());

    let log = fs::read_to_string(profile.time_log_path()).expect("time log should be readable");
    assert!(log.contains("Work"));
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
fn corrupt_categories_fail_start_without_creating_active_state() {
    let profile = TestProfile::new("corrupt-categories");
    let corrupt = "name,description\nWork,focus\n";
    fs::write(profile.categories_path(), corrupt).expect("corrupt categories should be written");

    let start = profile.run(&["start", "unsafe-default"]);

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
