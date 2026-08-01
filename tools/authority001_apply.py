from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"missing patch anchor in {path}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


# CLI: parse one invocation with an explicit deliberate-default bypass.
replace_once(
    "src/cli.rs",
    "use clap::{CommandFactory, Parser, ValueEnum};",
    "use clap::{CommandFactory, Parser, Subcommand, ValueEnum};",
)
replace_once(
    "src/cli.rs",
    "#[derive(Parser, Debug)]\n#[command(name = \"strata\")]\n#[command(about = \"Time tracking with falling sand\", long_about = None)]\npub enum Cli {",
    "#[derive(Parser, Debug)]\n#[command(name = \"strata\")]\n#[command(about = \"Time tracking with falling sand\", long_about = None)]\npub struct Invocation {\n    #[arg(\n        long,\n        global = true,\n        help = \"Deliberately ignore keymap.json and use built-in defaults\"\n    )]\n    pub ignore_config: bool,\n\n    #[command(subcommand)]\n    pub command: Option<Cli>,\n}\n\n#[derive(Subcommand, Debug)]\npub enum Cli {",
)
replace_once(
    "src/cli.rs",
    "pub fn print_completions(shell: &str) -> Result<(), String> {",
    "pub fn parse_invocation() -> Invocation {\n    Invocation::parse()\n}\n\npub fn print_completions(shell: &str) -> Result<(), String> {",
)
text = Path("src/cli.rs").read_text()
text = text.replace("&mut Cli::command()", "&mut Invocation::command()")
replace_once(
    "src/cli.rs",
    "pub fn run_cli() {\n    let cli = Cli::parse();\n    match cli {",
    "pub fn run_command(cli: Cli) {\n    match cli {",
)
Path("src/cli.rs").write_text(text)

# Keybindings/settings: validate every authority-critical value and expose deliberate defaults.
replace_once(
    "src/keybindings.rs",
    "use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};\nuse serde::{Deserialize, Serialize};",
    "use chrono::FixedOffset;\nuse crossterm::event::{KeyCode, KeyEvent, KeyModifiers};\nuse serde::{Deserialize, Serialize};",
)
replace_once(
    "src/keybindings.rs",
    "    if let Some(offset) = config.utc_offset_seconds {\n        settings.day_boundary.utc_offset_seconds = offset;\n    }",
    "    if let Some(offset) = config.utc_offset_seconds {\n        if FixedOffset::east_opt(offset).is_none() {\n            return Err(format!(\n                \"Invalid utc_offset_seconds '{}' in {}. Expected an offset between -86399 and 86399\",\n                offset,\n                path.display()\n            ));\n        }\n        settings.day_boundary.utc_offset_seconds = offset;\n    }",
)
replace_once(
    "src/keybindings.rs",
    "fn parse_time_log_path(config: &KeymapConfig) -> Option<PathBuf> {\n    crate::storage::normalize_time_log_path_input(config.time_log_path.as_deref()?)\n}",
    "fn parse_time_log_path(\n    config: &KeymapConfig,\n    config_path: &Path,\n) -> Result<Option<PathBuf>, String> {\n    let Some(raw) = config.time_log_path.as_deref() else {\n        return Ok(None);\n    };\n    if raw.contains('\\0') {\n        return Err(format!(\n            \"Invalid time_log_path in {}: paths cannot contain NUL bytes\",\n            config_path.display()\n        ));\n    }\n\n    let Some(path) = crate::storage::normalize_time_log_path_input(raw) else {\n        return Ok(None);\n    };\n    if path.file_name().is_none() {\n        return Err(format!(\n            \"Invalid time_log_path '{}' in {}: expected a file or directory path\",\n            raw,\n            config_path.display()\n        ));\n    }\n    if path.exists() && !path.is_file() {\n        return Err(format!(\n            \"Invalid time_log_path '{}' in {}: resolved path {} is not a regular file\",\n            raw,\n            config_path.display(),\n            path.display()\n        ));\n    }\n    if let Some(parent) = path.parent()\n        && parent.exists()\n        && !parent.is_dir()\n    {\n        return Err(format!(\n            \"Invalid time_log_path '{}' in {}: parent {} is not a directory\",\n            raw,\n            config_path.display(),\n            parent.display()\n        ));\n    }\n\n    Ok(Some(path))\n}",
)
replace_once(
    "src/keybindings.rs",
    "    let time_log_path = parse_time_log_path(&config);",
    "    let time_log_path = parse_time_log_path(&config, path)?;",
)
replace_once(
    "src/keybindings.rs",
    "pub(crate) fn default_runtime_settings() -> RuntimeSettings {\n    RuntimeSettings::default()\n}",
    "pub(crate) fn default_runtime_settings() -> RuntimeSettings {\n    RuntimeSettings::default()\n}\n\npub(crate) fn default_loaded_keybindings() -> LoadedKeybindings {\n    LoadedKeybindings {\n        keymap: default_keymap(),\n        runtime_settings: default_runtime_settings(),\n        time_log_path: None,\n    }\n}",
)

# Shared startup: parse once, load once, apply once, then choose CLI or TUI.
Path("src/lib.rs").write_text('''#![forbid(unsafe_code)]

use std::io;

#[allow(clippy::unnecessary_sort_by, clippy::while_let_loop)]
mod app;
mod cli;
mod constants;
#[allow(clippy::unnecessary_sort_by)]
mod domain;
mod keybindings;
#[allow(clippy::manual_checked_ops)]
mod sand;
#[allow(dead_code)]
mod sqlite;
mod storage;

fn load_startup_configuration(
    ignore_config: bool,
) -> Result<keybindings::LoadedKeybindings, io::Error> {
    if ignore_config {
        return Ok(keybindings::default_loaded_keybindings());
    }

    let path = storage::get_keymap_path();
    keybindings::load_keybindings(&path).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Configuration error: {error}. Fix {} or rerun with --ignore-config to deliberately use built-in defaults",
                path.display()
            ),
        )
    })
}

fn apply_startup_configuration(loaded: &keybindings::LoadedKeybindings) {
    domain::set_runtime_settings(loaded.runtime_settings);
    storage::set_runtime_storage_settings(storage::RuntimeStorageSettings {
        time_log_path: loaded.time_log_path.clone(),
    });
}

pub fn run() -> Result<(), io::Error> {
    let invocation = cli::parse_invocation();
    let loaded = load_startup_configuration(invocation.ignore_config)?;
    apply_startup_configuration(&loaded);

    match invocation.command {
        Some(command) => {
            cli::run_command(command);
            Ok(())
        }
        None => app::run_ui(loaded),
    }
}
''')

# TUI consumes the validated startup object rather than independently falling back.
replace_once(
    "src/app.rs",
    "    fn new(width: u16, height: u16) -> Result<Self, String> {\n        let keymap_path = storage::get_keymap_path();\n        let keymap_last_modified = std::fs::metadata(&keymap_path)\n            .and_then(|metadata| metadata.modified())\n            .ok();\n        let (keymap, runtime_settings, loaded_time_log_path, keymap_error) =\n            match keybindings::load_keybindings(&keymap_path) {\n                Ok(loaded) => (\n                    loaded.keymap,\n                    loaded.runtime_settings,\n                    loaded.time_log_path,\n                    None,\n                ),\n                Err(err) => (\n                    keybindings::default_keymap(),\n                    keybindings::default_runtime_settings(),\n                    None,\n                    Some(err),\n                ),\n            };\n\n        set_runtime_settings(runtime_settings);\n        storage::set_runtime_storage_settings(storage::RuntimeStorageSettings {\n            time_log_path: loaded_time_log_path.clone(),\n        });",
    "    fn new(\n        width: u16,\n        height: u16,\n        loaded: keybindings::LoadedKeybindings,\n    ) -> Result<Self, String> {\n        let keymap_path = storage::get_keymap_path();\n        let keymap_last_modified = std::fs::metadata(&keymap_path)\n            .and_then(|metadata| metadata.modified())\n            .ok();\n        let keybindings::LoadedKeybindings {\n            keymap,\n            runtime_settings,\n            time_log_path: loaded_time_log_path,\n        } = loaded;\n        let keymap_error = None;",
)
replace_once(
    "src/app.rs",
    "pub fn run_ui() -> Result<(), io::Error> {\n    let (width, height) = crossterm::terminal::size()?;\n    let mut app = App::new(width, height).map_err(io::Error::other)?;",
    "pub fn run_ui(loaded: keybindings::LoadedKeybindings) -> Result<(), io::Error> {\n    let (width, height) = crossterm::terminal::size()?;\n    let mut app = App::new(width, height, loaded).map_err(io::Error::other)?;",
)

# Unit tests for field/path-specific validation.
keybindings = Path("src/keybindings.rs").read_text()
insert = r'''

    #[test]
    fn test_load_keybindings_malformed_json_identifies_path() {
        let path = unique_path("strata_keymap_malformed_json");
        fs::write(&path, "{ not-json").expect("write config");

        let err = load_keybindings(&path).expect_err("config should fail");
        assert!(err.contains("Failed parsing keymap JSON"));
        assert!(err.contains(&path.display().to_string()));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keybindings_invalid_utc_offset_returns_error() {
        let path = unique_path("strata_keymap_invalid_offset");
        fs::write(&path, r#"{"utc_offset_seconds": 86400}"#).expect("write config");

        let err = load_keybindings(&path).expect_err("config should fail");
        assert!(err.contains("Invalid utc_offset_seconds '86400'"));
        assert!(err.contains(&path.display().to_string()));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_keybindings_invalid_time_log_parent_returns_error() {
        let path = unique_path("strata_keymap_invalid_profile");
        let blocker = unique_path("strata_keymap_profile_blocker");
        fs::write(&blocker, "not a directory").expect("write blocker");
        let configured = blocker.join("history.csv");
        fs::write(
            &path,
            serde_json::json!({"time_log_path": configured}).to_string(),
        )
        .expect("write config");

        let err = load_keybindings(&path).expect_err("config should fail");
        assert!(err.contains("Invalid time_log_path"));
        assert!(err.contains("parent"));
        assert!(err.contains(&path.display().to_string()));

        fs::remove_file(path).ok();
        fs::remove_file(blocker).ok();
    }
'''
position = keybindings.rfind("\n}")
if position < 0:
    raise SystemExit("could not locate keybindings test module end")
keybindings = keybindings[:position] + insert + keybindings[position:]
Path("src/keybindings.rs").write_text(keybindings)

# Process-level authority tests.
Path("tests/config_authority.rs").write_text(r'''#![cfg(target_os = "linux")]

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
        assert!(!self.state_home.join("strata/storage_authority.json").exists());
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
    fs::write(
        profile.config_path(),
        r#"{"keymap":{"f":"not_real"}}"#,
    )
    .expect("write invalid keybinding config");

    let cli = profile.run(&["report", "--today"]);
    assert_config_failure(&profile, &cli, "Unknown action 'not_real'");

    let tui = profile.run(&[]);
    assert_config_failure(&profile, &tui, "Unknown action 'not_real'");
}

#[test]
fn invalid_timezone_blocks_mutation_before_default_database_creation() {
    let profile = TestProfile::new("timezone");
    fs::write(
        profile.config_path(),
        r#"{"utc_offset_seconds":86400}"#,
    )
    .expect("write invalid timezone config");

    let output = profile.run(&["start", "unsafe", "--category", "Work"]);
    assert_config_failure(&profile, &output, "Invalid utc_offset_seconds");
}

#[test]
fn invalid_profile_path_blocks_mutation() {
    let profile = TestProfile::new("profile-path");
    let blocker = profile.root.join("not-a-directory");
    fs::write(&blocker, "blocker").expect("write blocker");
    let configured = blocker.join("time_log.csv");
    fs::write(
        profile.config_path(),
        serde_json::json!({"time_log_path": configured}).to_string(),
    )
    .expect("write invalid profile config");

    let output = profile.run(&["start", "unsafe", "--category", "Work"]);
    assert_config_failure(&profile, &output, "Invalid time_log_path");
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
''')
