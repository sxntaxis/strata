#![cfg(target_os = "linux")]

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
            .expect("system clock should follow Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "strata-config-{name}-{}-{nonce}",
            std::process::id()
        ));
        let data_home = root.join("data");
        let state_home = root.join("state");
        let config_home = root.join("config");
        fs::create_dir_all(data_home.join("strata")).expect("create data profile");
        fs::create_dir_all(state_home.join("strata")).expect("create state profile");
        fs::create_dir_all(config_home.join("strata")).expect("create config profile");
        fs::write(
            data_home.join("strata/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .expect("write categories");
        Self {
            root,
            data_home,
            state_home,
            config_home,
        }
    }

    fn config_path(&self) -> PathBuf {
        self.config_home.join("strata/keymap.json")
    }

    fn active_path(&self) -> PathBuf {
        self.state_home.join("strata/active_session.json")
    }

    fn database_path(&self) -> PathBuf {
        self.data_home.join("strata/strata.sqlite3")
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_strata"));
        command
            .args(args)
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR");
        command.output().expect("Strata process should run")
    }

    fn assert_no_authority_write(&self) {
        assert!(!self.active_path().exists());
        assert!(!self.database_path().exists());
        assert!(
            !self
                .state_home
                .join("strata/storage_authority.json")
                .exists()
        );
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

fn assert_config_failure(profile: &TestProfile, output: &Output, field: &str) {
    assert!(!output.status.success(), "command unexpectedly succeeded");
    let error = stderr(output);
    assert!(error.contains("Configuration error"), "{error}");
    assert!(error.contains(field), "{error}");
    assert!(
        error.contains(&profile.config_path().display().to_string()),
        "{error}"
    );
    assert!(error.contains("--ignore-config"), "{error}");
    profile.assert_no_authority_write();
}

#[test]
fn malformed_json_blocks_mutation_before_authority_open() {
    let profile = TestProfile::new("malformed");
    fs::write(profile.config_path(), "{ broken").expect("write malformed config");

    let output = profile.run(&["start", "unsafe", "--category", "Work"]);
    assert_config_failure(&profile, &output, "Failed parsing keymap JSON");
}

#[test]
fn invalid_keybinding_only_data_blocks_cli_and_tui_equally() {
    let profile = TestProfile::new("keybinding");
    fs::write(profile.config_path(), r#"{"keymap":{"f":"not_real"}}"#)
        .expect("write invalid keybinding config");

    let cli = profile.run(&["report", "--today"]);
    assert_config_failure(&profile, &cli, "Unknown action 'not_real'");

    let tui = profile.run(&[]);
    assert_config_failure(&profile, &tui, "Unknown action 'not_real'");
}

#[test]
fn invalid_timezone_blocks_mutation_before_default_database_creation() {
    let profile = TestProfile::new("timezone");
    fs::write(profile.config_path(), r#"{"utc_offset_seconds":86400}"#)
        .expect("write invalid timezone config");

    let output = profile.run(&["start", "unsafe", "--category", "Work"]);
    assert_config_failure(&profile, &output, "Invalid utc_offset_seconds");
}

#[test]
fn time_log_path_hot_redirect_is_rejected() {
    let profile = TestProfile::new("profile-path");
    fs::write(
        profile.config_path(),
        serde_json::json!({"time_log_path": profile.root.join("elsewhere")}).to_string(),
    )
    .expect("write obsolete partial-path config");

    let output = profile.run(&["start", "unsafe", "--category", "Work"]);
    assert_config_failure(&profile, &output, "time_log_path");
    assert!(stderr(&output).contains("--profile"));
}

#[test]
fn ignore_config_is_an_explicit_deliberate_default_override() {
    let profile = TestProfile::new("override");
    fs::write(profile.config_path(), "{ broken").expect("write malformed config");

    let output = profile.run(&[
        "--ignore-config",
        "start",
        "deliberate-default",
        "--category",
        "Work",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let active = fs::read_to_string(profile.active_path()).expect("active state should exist");
    assert!(active.contains("deliberate-default"));
    assert!(!profile.database_path().exists());
}

#[test]
fn removed_sunrise_mode_is_migrated_visibly_to_fixed_policy() {
    let profile = TestProfile::new("sunrise-migration");
    fs::write(
        profile.config_path(),
        r#"{"day_start_mode":"sunrise","day_start_hour":5,"day_start_minute":45}"#,
    )
    .expect("write legacy sunrise config");

    let output = profile.run(&["report", "--today"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let warning = stderr(&output);
    assert!(
        warning.contains("migrated removed day_start_mode 'sunrise'"),
        "{warning}"
    );
    assert!(
        warning.contains("never implemented solar sunrise"),
        "{warning}"
    );

    let migrated = fs::read_to_string(profile.config_path()).expect("read migrated config");
    assert!(migrated.contains("\"day_start_mode\": \"fixed\""));
    assert!(migrated.contains("\"day_start_hour\": 5"));
    assert!(migrated.contains("\"day_start_minute\": 45"));
    profile.assert_no_authority_write();
}
