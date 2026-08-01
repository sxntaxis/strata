#![cfg(target_os = "linux")]

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use serde_json::Value;

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
            "strata-sqlite006-{name}-{}-{nonce}",
            std::process::id()
        ));
        let data_home = root.join("data");
        let state_home = root.join("state");
        let config_home = root.join("config");
        fs::create_dir_all(data_home.join("strata")).expect("data directory should be created");
        fs::create_dir_all(&state_home).expect("state directory should be created");
        fs::create_dir_all(&config_home).expect("config directory should be created");
        fs::write(
            data_home.join("strata/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,Focused work,0,1\n",
        )
        .expect("categories should be written");
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

    fn database_path(&self) -> PathBuf {
        self.data_home.join("strata/strata.sqlite3")
    }

    fn marker_path(&self) -> PathBuf {
        self.state_home.join("strata/storage_authority.json")
    }

    fn categories_path(&self) -> PathBuf {
        self.data_home.join("strata/categories.csv")
    }

    fn legacy_active_path(&self) -> PathBuf {
        self.state_home.join("strata/active_session.json")
    }

    fn legacy_time_log_path(&self) -> PathBuf {
        self.data_home.join("strata/time_log.csv")
    }

    fn migrate(&self) {
        let output = self.run(&["migrate-sqlite"]);
        assert!(
            output.status.success(),
            "migration failed: {}",
            stderr(&output)
        );
    }

    fn activate(&self) -> Output {
        self.run(&["activate-sqlite", "--confirm", "--json"])
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn read_marker(profile: &TestProfile) -> Value {
    let bytes = fs::read(profile.marker_path()).expect("authority marker should be readable");
    serde_json::from_slice(&bytes).expect("authority marker should be valid JSON")
}

#[test]
fn activation_requires_confirmation_and_preserves_legacy_authority() {
    let profile = TestProfile::new("confirmation");
    profile.migrate();

    let output = profile.run(&["activate-sqlite"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("requires --confirm"));
    assert_eq!(read_marker(&profile)["active_authority"], "legacy-files");
    let connection = Connection::open(profile.database_path()).expect("database should open");
    let authority: String = connection
        .query_row(
            "SELECT value FROM database_metadata WHERE key = 'storage_authority'",
            [],
            |row| row.get(0),
        )
        .expect("authority metadata should exist");
    assert_eq!(authority, "sqlite-candidate");
}

#[test]
fn stale_candidate_is_rejected_before_authority_changes() {
    let profile = TestProfile::new("stale-candidate");
    profile.migrate();
    fs::OpenOptions::new()
        .append(true)
        .open(profile.categories_path())
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(b"2,Changed after migration,,1,1\n")
        })
        .expect("legacy source should be changed");

    let output = profile.activate();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("legacy authority changed"));
    assert_eq!(read_marker(&profile)["active_authority"], "legacy-files");
    let connection = Connection::open(profile.database_path()).expect("database should open");
    let authority: String = connection
        .query_row(
            "SELECT value FROM database_metadata WHERE key = 'storage_authority'",
            [],
            |row| row.get(0),
        )
        .expect("authority metadata should exist");
    assert_eq!(authority, "sqlite-candidate");
}

#[test]
fn interrupted_activation_is_recovered_idempotently() {
    let profile = TestProfile::new("activation-recovery");
    profile.migrate();
    let marker_path = profile.marker_path();
    let marker = fs::read_to_string(&marker_path).expect("marker should be readable");
    fs::write(
        &marker_path,
        marker.replacen(
            "\"active_authority\": \"legacy-files\"",
            "\"active_authority\": \"activating-sqlite-cli\"",
            1,
        ),
    )
    .expect("interrupted marker should be written");

    let output = profile.activate();

    assert!(
        output.status.success(),
        "activation failed: {}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("recovered-activation"));
    let marker = read_marker(&profile);
    assert_eq!(marker["active_authority"], "sqlite-cli");
    assert_eq!(marker["sqlite_cli_activation"]["status"], "active");

    let repeated = profile.activate();
    assert!(
        repeated.status.success(),
        "repeat failed: {}",
        stderr(&repeated)
    );
    assert!(stdout(&repeated).contains("already-active"));
}

#[test]
fn activated_cli_uses_sqlite_without_legacy_dual_writes() {
    let profile = TestProfile::new("cli-cutover");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let start = profile.run(&[
        "start",
        "sqlite-project",
        "--category",
        "Work",
        "--desc",
        "SQLite authority",
    ]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    assert!(!profile.legacy_active_path().exists());
    assert!(!profile.legacy_time_log_path().exists());

    {
        let connection = Connection::open(profile.database_path()).expect("database should open");
        let active: (String, String) = connection
            .query_row(
                "SELECT project, description FROM active_session WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("SQLite active session should exist");
        assert_eq!(active.0, "sqlite-project");
        assert_eq!(active.1, "SQLite authority");
    }

    let stop = profile.run(&["stop"]);
    assert!(stop.status.success(), "stop failed: {}", stderr(&stop));
    assert!(!profile.legacy_active_path().exists());
    assert!(!profile.legacy_time_log_path().exists());

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let completed: (i64, String, String) = connection
        .query_row(
            "SELECT count(*), max(project), max(source) FROM sessions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("completed SQLite session should exist");
    assert_eq!(completed.0, 1);
    assert_eq!(completed.1, "sqlite-project");
    assert_eq!(completed.2, "cli-runtime");
    drop(connection);

    let report = profile.run(&["report", "--today"]);
    assert!(
        report.status.success(),
        "report failed: {}",
        stderr(&report)
    );
    assert!(stdout(&report).contains("Work"));

    let export = profile.run(&["export", "--format", "json"]);
    assert!(
        export.status.success(),
        "export failed: {}",
        stderr(&export)
    );
    let exported: Value = serde_json::from_slice(&export.stdout).expect("export should be JSON");
    assert_eq!(exported["sessions"][0]["project"], "sqlite-project");
}
