#![cfg(target_os = "linux")]

use rusqlite::Connection;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct TerminalProfile {
    root: PathBuf,
    data_home: PathBuf,
    state_home: PathBuf,
    config_home: PathBuf,
}

impl TerminalProfile {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should follow Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "strata-terminal-{name}-{}-{nonce}",
            std::process::id()
        ));
        let data_home = root.join("data");
        let state_home = root.join("state");
        let config_home = root.join("config");
        fs::create_dir_all(data_home.join("strata")).unwrap();
        fs::create_dir_all(&state_home).unwrap();
        fs::create_dir_all(&config_home).unwrap();
        Self {
            root,
            data_home,
            state_home,
            config_home,
        }
    }

    fn run_case(&self, fault: Option<&str>, input: Option<u8>) -> Output {
        let marker = self.root.join("restore-marker.txt");
        let binary = shell_quote(Path::new(env!("CARGO_BIN_EXE_strata")));
        let command_line = format!(
            "before=$(stty -g); {binary}; status=$?; after=$(stty -g); \
             printf '\\n__STRATA_BEFORE__=%s\\n__STRATA_AFTER__=%s\\n__STRATA_STATUS__=%s\\n' \
             \"$before\" \"$after\" \"$status\"; exit \"$status\""
        );

        let mut command = Command::new("timeout");
        command
            .args(["12s", "script", "-qefc", &command_line, "/dev/null"])
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_PROFILE")
            .env_remove("STRATA_DATA_DIR")
            .env("STRATA_TEST_TERMINAL_RESTORE_MARKER", &marker)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(fault) = fault {
            command.env("STRATA_TEST_TUI_FAULT", fault);
        } else {
            command.env_remove("STRATA_TEST_TUI_FAULT");
        }

        let mut child = command.spawn().expect("PTY process should start");
        if let Some(input) = input {
            let mut stdin = child.stdin.take().expect("PTY stdin should exist");
            thread::sleep(Duration::from_millis(1_100));
            stdin.write_all(&[input]).ok();
            stdin.flush().ok();
        }
        let output = child.wait_with_output().expect("PTY process should finish");
        let marker_lines = fs::read_to_string(&marker)
            .unwrap_or_default()
            .lines()
            .count();
        assert_eq!(marker_lines, 1, "terminal restoration must execute once");
        output
    }

    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_strata"))
            .args(args)
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_PROFILE")
            .env_remove("STRATA_DATA_DIR")
            .output()
            .expect("Strata CLI should run")
    }

    fn seed_work_category(&self) {
        let initialized = self.cli(&["report", "--today"]);
        assert!(
            initialized.status.success(),
            "{}",
            combined_output(&initialized)
        );
        Connection::open(self.database_path())
            .unwrap()
            .execute(
                "INSERT INTO categories(id, name, description, color_index, balance_effect, sort_order)
                 VALUES (1, 'Work', '', 0, 1, 1)",
                [],
            )
            .unwrap();
    }

    fn control_socket_path(&self) -> PathBuf {
        self.state_home.join("strata/runtime.sock")
    }

    fn database_path(&self) -> PathBuf {
        self.data_home.join("strata/strata.sqlite3")
    }

    fn checkpoint_exists(&self) -> bool {
        if !self.database_path().exists() {
            return false;
        }
        let connection = Connection::open(self.database_path()).expect("open profile database");
        let count: i64 = connection
            .query_row("SELECT count(*) FROM runtime_checkpoint", [], |row| {
                row.get(0)
            })
            .expect("query runtime checkpoint custody");
        count > 0
    }
}

impl Drop for TerminalProfile {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn shell_quote(path: &Path) -> String {
    let raw = path.to_string_lossy();
    format!("'{}'", raw.replace('\'', "'\\''"))
}

fn combined_output(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn marker(output: &Output, name: &str) -> String {
    let prefix = format!("__STRATA_{name}__=");
    combined_output(output)
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .find_map(|line| line.strip_prefix(&prefix).map(str::to_string))
        .unwrap_or_else(|| panic!("missing {prefix} marker: {}", combined_output(output)))
}

fn assert_terminal_restored(output: &Output) {
    assert_eq!(marker(output, "BEFORE"), marker(output, "AFTER"));
}

fn wait_for_path(path: &Path) {
    for _ in 0..60 {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {}", path.display());
}

fn spawn_live_tui(profile: &TerminalProfile) -> std::process::Child {
    let binary = shell_quote(Path::new(env!("CARGO_BIN_EXE_strata")));
    Command::new("script")
        .args(["-qefc", &binary, "/dev/null"])
        .env("XDG_DATA_HOME", &profile.data_home)
        .env("XDG_STATE_HOME", &profile.state_home)
        .env("XDG_CONFIG_HOME", &profile.config_home)
        .env_remove("STRATA_PROFILE")
        .env_remove("STRATA_DATA_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("live TUI should start")
}

fn persisted_sand_state(profile: &TerminalProfile) -> (i64, i64, serde_json::Value) {
    let connection = Connection::open(profile.database_path()).expect("open profile database");
    let (grid_width, grid_height, payload_json): (i64, i64, String) = connection
        .query_row(
            "SELECT grid_width, grid_height, payload_json FROM sand_state WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read persisted sand state");
    let payload = serde_json::from_str(&payload_json).expect("sand payload should be valid JSON");
    (grid_width, grid_height, payload)
}

fn sediment_mass_for_category(payload: &serde_json::Value, category_id: u64) -> usize {
    let placed = payload
        .get("grains")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|grain| {
            grain.get("category_id").and_then(serde_json::Value::as_u64) == Some(category_id)
        })
        .count();
    let pending_runs = payload
        .get("pending_runs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|run| {
            run.get("category_id").and_then(serde_json::Value::as_u64) == Some(category_id)
        })
        .map(|run| {
            run.get("count")
                .and_then(serde_json::Value::as_u64)
                .and_then(|count| usize::try_from(count).ok())
                .expect("pending run count should fit usize")
        })
        .sum::<usize>();
    let legacy_pending = payload
        .get("pending_grains")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|id| id.as_u64() == Some(category_id))
        .count();
    placed + pending_runs + legacy_pending
}

#[test]
fn normal_quit_and_detach_restore_terminal_once() {
    let quit = TerminalProfile::new("quit");
    let quit_output = quit.run_case(None, Some(b'q'));
    assert!(
        quit_output.status.success(),
        "{}",
        combined_output(&quit_output)
    );
    assert_terminal_restored(&quit_output);

    let detach = TerminalProfile::new("detach");
    let detach_output = detach.run_case(None, Some(b'd'));
    assert!(
        detach_output.status.success(),
        "{}",
        combined_output(&detach_output)
    );
    assert_terminal_restored(&detach_output);
    assert!(detach.checkpoint_exists());
}

#[test]
fn draw_poll_and_read_failures_restore_terminal_and_checkpoint() {
    for (stage, input) in [("draw", None), ("poll", None), ("read", Some(b'q'))] {
        let profile = TerminalProfile::new(stage);
        let output = profile.run_case(Some(stage), input);
        assert!(!output.status.success(), "{stage} should fail");
        assert_terminal_restored(&output);
        let combined = combined_output(&output);
        assert!(combined.contains(&format!("injected TUI {stage} failure")));
        assert!(combined.contains("emergency checkpoint: committed"));
        assert!(profile.checkpoint_exists());
    }
}

#[test]
fn live_cli_control_is_profile_scoped_and_preserves_continuous_idle() {
    let profile = TerminalProfile::new("live-control");
    profile.seed_work_category();

    let binary = shell_quote(Path::new(env!("CARGO_BIN_EXE_strata")));
    let mut tui = Command::new("script")
        .args(["-qefc", &binary, "/dev/null"])
        .env("XDG_DATA_HOME", &profile.data_home)
        .env("XDG_STATE_HOME", &profile.state_home)
        .env("XDG_CONFIG_HOME", &profile.config_home)
        .env_remove("STRATA_PROFILE")
        .env_remove("STRATA_DATA_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("live TUI should start");

    wait_for_path(&profile.control_socket_path());

    let start = profile.cli(&["start", "Work", "--desc", "focus"]);
    assert!(start.status.success(), "{}", combined_output(&start));
    let connection = Connection::open(profile.database_path()).unwrap();
    let active: (i64, String) = connection
        .query_row(
            "SELECT category_id, description FROM active_session",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(active, (1, "focus".to_string()));
    drop(connection);

    let other = TerminalProfile::new("live-control-other-profile");
    let cross_profile = other.cli(&["stop"]);
    assert!(!cross_profile.status.success());
    assert!(combined_output(&cross_profile).contains("No active"));

    let stop = profile.cli(&["stop"]);
    assert!(stop.status.success(), "{}", combined_output(&stop));
    let idle_category: i64 = Connection::open(profile.database_path())
        .unwrap()
        .query_row("SELECT category_id FROM active_session", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(idle_category, 0);

    if let Some(mut stdin) = tui.stdin.take() {
        stdin.write_all(b"q").ok();
        stdin.flush().ok();
    }
    let status = tui.wait().expect("live TUI should exit");
    assert!(status.success());
}

#[test]
fn shift_c_clears_only_idle_sediment_and_preserves_sqlite_extent() {
    let profile = TerminalProfile::new("shift-c-idle-clear");
    profile.seed_work_category();

    let mut tui = spawn_live_tui(&profile);
    wait_for_path(&profile.control_socket_path());

    let start = profile.cli(&["start", "Work", "--desc", "focus"]);
    assert!(start.status.success(), "{}", combined_output(&start));
    thread::sleep(Duration::from_millis(2_300));

    let stop = profile.cli(&["stop"]);
    assert!(stop.status.success(), "{}", combined_output(&stop));
    thread::sleep(Duration::from_millis(2_300));

    let mut stdin = tui.stdin.take().expect("live TUI stdin should exist");
    stdin.write_all(b"q").expect("quit should reach TUI");
    stdin.flush().expect("quit should flush");
    let status = tui.wait().expect("mixed-sediment TUI should exit");
    assert!(status.success());

    let (before_width, before_height, before_payload) = persisted_sand_state(&profile);
    let work_before = sediment_mass_for_category(&before_payload, 1);
    let idle_before = sediment_mass_for_category(&before_payload, 0);
    assert!(
        work_before >= 2,
        "expected Work sediment, got {work_before}"
    );
    assert!(
        idle_before >= 2,
        "expected Idle sediment, got {idle_before}"
    );

    let mut tui = spawn_live_tui(&profile);
    wait_for_path(&profile.control_socket_path());
    let mut stdin = tui.stdin.take().expect("live TUI stdin should exist");
    stdin
        .write_all(b"C")
        .expect("uppercase C should reach the real PTY");
    stdin.flush().expect("uppercase C should flush");

    let mut cleared = None;
    for _ in 0..80 {
        let state = persisted_sand_state(&profile);
        if sediment_mass_for_category(&state.2, 0) == 0
            && sediment_mass_for_category(&state.2, 1) == work_before
        {
            cleared = Some(state);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let (after_width, after_height, after_payload) =
        cleared.expect("Shift-C should synchronously persist the Idle-only clear");
    assert_eq!((after_width, after_height), (before_width, before_height));
    assert_eq!(sediment_mass_for_category(&after_payload, 0), 0);
    assert_eq!(sediment_mass_for_category(&after_payload, 1), work_before);

    stdin.write_all(b"q").expect("quit should reach TUI");
    stdin.flush().expect("quit should flush");
    let status = tui.wait().expect("post-clear TUI should exit");
    assert!(status.success());

    let mut reopened = spawn_live_tui(&profile);
    wait_for_path(&profile.control_socket_path());
    let mut reopened_stdin = reopened
        .stdin
        .take()
        .expect("reopened TUI stdin should exist");
    reopened_stdin
        .write_all(b"q")
        .expect("reopened quit should reach TUI");
    reopened_stdin.flush().expect("reopened quit should flush");
    let status = reopened.wait().expect("reopened TUI should exit");
    assert!(status.success());

    let (restart_width, restart_height, restart_payload) = persisted_sand_state(&profile);
    assert_eq!(
        (restart_width, restart_height),
        (before_width, before_height)
    );
    assert_eq!(sediment_mass_for_category(&restart_payload, 1), work_before);
}

#[test]
fn panic_restores_terminal_once_without_runtime_error_claim() {
    let profile = TerminalProfile::new("panic");
    let output = profile.run_case(Some("panic"), None);
    assert!(!output.status.success());
    assert_terminal_restored(&output);
    let combined = combined_output(&output);
    assert!(combined.contains("injected TUI panic"));
    assert!(!combined.contains("emergency checkpoint: committed"));
}
