#![cfg(target_os = "linux")]

use std::{
    fs,
    path::{Path, PathBuf},
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
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "strata-{name}-{}-{nonce}",
            std::process::id()
        ));
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

    fn active_session_path(&self) -> PathBuf {
        self.state_home.join("strata/active_session.json")
    }

    fn time_log_path(&self) -> PathBuf {
        self.data_home.join("strata/time_log.csv")
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
    assert!(first.status.success(), "first start failed: {}", stderr(&first));

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

    let after = fs::read_to_string(profile.time_log_path()).expect("time log should remain readable");
    assert_eq!(after, before);
}
