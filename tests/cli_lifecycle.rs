#![cfg(target_os = "linux")]

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
        self.command()
            .args(args)
            .output()
            .expect("Strata should run")
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

    let connection = Connection::open(profile.database_path()).unwrap();
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
    let start = profile.run(&["start", "Work", "--desc", "Read chapter 4"]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    profile.backdate_active(2);

    let stop = profile.run(&["stop"]);
    assert!(stop.status.success(), "stop failed: {}", stderr(&stop));

    let connection = Connection::open(profile.database_path()).unwrap();
    let completed: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(completed, 1);
    let active_category_id: i64 = connection
        .query_row("SELECT category_id FROM active_session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(active_category_id, 0);

    let report = profile.run(&["report", "--today"]);
    assert!(
        report.status.success(),
        "report failed: {}",
        stderr(&report)
    );
    assert!(String::from_utf8_lossy(&report.stdout).contains("Work"));

    let export = profile.run(&["export", "--format", "json"]);
    assert!(
        export.status.success(),
        "export failed: {}",
        stderr(&export)
    );
    let value: serde_json::Value = serde_json::from_slice(&export.stdout).unwrap();
    assert!(value["sessions"][0]
        .as_object()
        .expect("session export should be an object")
        .get("project")
        .is_none());
}

#[test]
fn repeated_start_updates_live_layer_and_stop_returns_to_idle() {
    let profile = TestProfile::new("continuous-ledger");
    profile.insert_work_category();

    let first = profile.run(&["start", "Work", "--desc", "first"]);
    assert!(
        first.status.success(),
        "first start failed: {}",
        stderr(&first)
    );
    let second = profile.run(&["start", "Work", "--desc", "second"]);
    assert!(
        second.status.success(),
        "second start failed: {}",
        stderr(&second)
    );

    let connection = Connection::open(profile.database_path()).unwrap();
    let (category_id, description): (i64, String) = connection
        .query_row(
            "SELECT category_id, description FROM active_session",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(category_id, 1);
    assert_eq!(description, "second");
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
    let idle_category_id: i64 = connection
        .query_row("SELECT category_id FROM active_session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(idle_category_id, 0);
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
