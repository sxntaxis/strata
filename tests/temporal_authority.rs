#![cfg(target_os = "linux")]

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use serde_json::json;

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
        fs::write(
            data_home.join("strata/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .unwrap();
        Self {
            root,
            data_home,
            state_home,
            config_home,
        }
    }

    fn active_path(&self) -> PathBuf {
        self.state_home.join("strata/active_session.json")
    }

    fn time_log_path(&self) -> PathBuf {
        self.data_home.join("strata/time_log.csv")
    }

    fn write_active_start(&self, started_at: chrono::DateTime<Utc>) {
        fs::write(
            self.active_path(),
            serde_json::to_vec_pretty(&json!({
                "project": "clock-test",
                "description": "",
                "category_id": 1,
                "category_name": "Work",
                "start_time": started_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_strata"))
            .args(args)
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
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
fn future_legacy_start_is_rejected_without_consuming_active_state() {
    let profile = TestProfile::new("future");
    profile.write_active_start(Utc::now() + ChronoDuration::hours(2));

    let output = profile.run(&["stop"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("starts in the future"));
    assert!(profile.active_path().exists());
    assert!(!profile.time_log_path().exists());
}

#[test]
fn large_wall_interval_requires_explicit_clock_jump_acceptance() {
    let profile = TestProfile::new("forward");
    profile.write_active_start(Utc::now() - ChronoDuration::days(8));

    let blocked = profile.run(&["stop"]);
    assert!(!blocked.status.success());
    assert!(stderr(&blocked).contains("--accept-clock-jump"));
    assert!(profile.active_path().exists());
    assert!(!profile.time_log_path().exists());

    let accepted = profile.run(&["stop", "--accept-clock-jump"]);
    assert!(accepted.status.success(), "{}", stderr(&accepted));
    assert!(!profile.active_path().exists());
    let log = fs::read_to_string(profile.time_log_path()).unwrap();
    assert!(
        log.lines().count() >= 2,
        "expected header plus a committed session row"
    );
}
