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
    data_home: PathBuf,
    state_home: PathBuf,
    config_home: PathBuf,
}

impl TestProfile {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "strata-temporal-{name}-{}-{nonce}",
            std::process::id()
        ));
        let data_home = root.join("data");
        let state_home = root.join("state");
        let config_home = root.join("config");
        fs::create_dir_all(data_home.join("strata")).unwrap();
        fs::create_dir_all(state_home.join("strata")).unwrap();
        fs::create_dir_all(config_home.join("strata")).unwrap();
        let profile = Self {
            root,
            data_home,
            state_home,
            config_home,
        };
        let initialized = profile.run(&["report", "--today"]);
        assert!(initialized.status.success(), "{}", stderr(&initialized));
        Connection::open(profile.database_path())
            .unwrap()
            .execute(
                "INSERT INTO categories(id, name, description, color_index, balance_effect, sort_order)
                 VALUES (1, 'Work', '', 0, 1, 1)",
                [],
            )
            .unwrap();
        profile
    }

    fn database_path(&self) -> PathBuf {
        self.data_home.join("strata/strata.sqlite3")
    }

    fn seed_active_start(&self, started_at: chrono::DateTime<Utc>) {
        let started = self.run(&["start", "Work"]);
        assert!(started.status.success(), "{}", stderr(&started));
        Connection::open(self.database_path())
            .unwrap()
            .execute(
                "UPDATE active_session SET started_at_utc = ?1 WHERE singleton = 1",
                [started_at.to_rfc3339()],
            )
            .unwrap();
    }

    fn row_count(&self, table: &str) -> i64 {
        Connection::open(self.database_path())
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_strata"))
            .args(args)
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_PROFILE")
            .env_remove("STRATA_DATA_DIR")
            .output()
            .unwrap()
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
fn future_start_is_rejected_without_consuming_active_state() {
    let profile = TestProfile::new("future");
    profile.seed_active_start(Utc::now() + ChronoDuration::hours(2));

    let output = profile.run(&["stop"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("starts in the future"));
    assert_eq!(profile.row_count("active_session"), 1);
    assert_eq!(profile.row_count("sessions"), 0);
}

#[test]
fn large_wall_interval_requires_explicit_clock_jump_acceptance() {
    let profile = TestProfile::new("forward");
    profile.seed_active_start(Utc::now() - ChronoDuration::days(8));

    let blocked = profile.run(&["stop"]);
    assert!(!blocked.status.success());
    assert!(stderr(&blocked).contains("--accept-clock-jump"));
    assert_eq!(profile.row_count("active_session"), 1);
    assert_eq!(profile.row_count("sessions"), 0);

    let accepted = profile.run(&["stop", "--accept-clock-jump"]);
    assert!(accepted.status.success(), "{}", stderr(&accepted));
    assert_eq!(profile.row_count("active_session"), 1);
    assert_eq!(profile.row_count("sessions"), 1);
    let active_category: i64 = Connection::open(profile.database_path())
        .unwrap()
        .query_row("SELECT category_id FROM active_session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(active_category, 0);
}
