#![cfg(target_os = "linux")]

#[path = "support/pty.rs"]
mod pty;

use pty::{PtyChild, PtyOutput};
use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const PTY_TIMEOUT: Duration = Duration::from_secs(12);

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

    fn strata_command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_strata"));
        command
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_PROFILE")
            .env_remove("STRATA_DATA_DIR");
        command
    }

    fn spawn_tui(&self, fault: Option<&str>, marker: Option<&Path>) -> PtyChild {
        let mut command = self.strata_command();
        if let Some(fault) = fault {
            command.env("STRATA_TEST_TUI_FAULT", fault);
        } else {
            command.env_remove("STRATA_TEST_TUI_FAULT");
        }
        if let Some(marker) = marker {
            command.env("STRATA_TEST_TERMINAL_RESTORE_MARKER", marker);
        } else {
            command.env_remove("STRATA_TEST_TERMINAL_RESTORE_MARKER");
        }
        pty::spawn(command).expect("kernel-backed PTY process should start")
    }

    fn run_case(&self, fault: Option<&str>, input: Option<u8>) -> PtyOutput {
        let marker = self.root.join("restore-marker.txt");
        let mut child = self.spawn_tui(fault, Some(&marker));
        if let Some(input) = input {
            thread::sleep(Duration::from_millis(1_100));
            child.write_all(&[input]).ok();
        }
        let output = child
            .wait(PTY_TIMEOUT)
            .expect("PTY process should finish before timeout");
        let marker_lines = fs::read_to_string(&marker)
            .unwrap_or_default()
            .lines()
            .count();
        assert_eq!(marker_lines, 1, "terminal restoration must execute once");
        output
    }

    fn cli(&self, args: &[&str]) -> Output {
        self.strata_command()
            .args(args)
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

fn combined_output(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn pty_output(output: &PtyOutput) -> String {
    String::from_utf8_lossy(&output.bytes).into_owned()
}

fn assert_terminal_restored(output: &PtyOutput) {
    assert_eq!(
        output.before, output.after,
        "terminal termios must be restored exactly"
    );
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

fn spawn_live_tui(profile: &TerminalProfile) -> PtyChild {
    profile.spawn_tui(None, None)
}

fn quit_tui(tui: &mut PtyChild) {
    tui.write_all(b"q").expect("quit should reach TUI");
    let output = tui.wait(PTY_TIMEOUT).expect("TUI should exit");
    assert!(output.status.success(), "{}", pty_output(&output));
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
    assert!(quit_output.status.success(), "{}", pty_output(&quit_output));
    assert_terminal_restored(&quit_output);

    let detach = TerminalProfile::new("detach");
    let detach_output = detach.run_case(None, Some(b'd'));
    assert!(
        detach_output.status.success(),
        "{}",
        pty_output(&detach_output)
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
        let combined = pty_output(&output);
        assert!(combined.contains(&format!("injected TUI {stage} failure")));
        assert!(combined.contains("emergency checkpoint: committed"));
        assert!(profile.checkpoint_exists());
    }
}

#[test]
fn live_cli_control_is_profile_scoped_and_preserves_continuous_idle() {
    let profile = TerminalProfile::new("live-control");
    profile.seed_work_category();

    let mut tui = spawn_live_tui(&profile);
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

    quit_tui(&mut tui);
}

#[test]
fn active_tag_preview_requires_enter_and_escape_restores_opening_tag() {
    let profile = TerminalProfile::new("active-subtitle-live-edit");
    profile.seed_work_category();
    Connection::open(profile.database_path())
        .unwrap()
        .execute(
            "UPDATE categories SET description = 'Layer metadata' WHERE id = 1",
            [],
        )
        .unwrap();

    let mut tui = spawn_live_tui(&profile);
    wait_for_path(&profile.control_socket_path());

    let start = profile.cli(&["start", "Work"]);
    assert!(start.status.success(), "{}", combined_output(&start));
    let durable_before: String = Connection::open(profile.database_path())
        .unwrap()
        .query_row("SELECT description FROM active_session", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(durable_before, "");

    tui.write_all(b"\r")
        .expect("Enter should open the layer pop-up");
    thread::sleep(Duration::from_millis(50));
    tui.write_all(b"focus")
        .expect("typing should reach the active subtitle editor");

    let mut live_status = None;
    for _ in 0..80 {
        let status = profile.cli(&["status"]);
        let output = combined_output(&status);
        if status.status.success() && output.contains("tag 'focus'") {
            live_status = Some(output);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(
        live_status.is_some(),
        "typed subtitle should become live before Enter or Esc"
    );

    let durable_while_open: String = Connection::open(profile.database_path())
        .unwrap()
        .query_row("SELECT description FROM active_session", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        durable_while_open, "",
        "typing should stay live in memory until the pop-up closes"
    );

    tui.write_all(b"\x1b")
        .expect("Esc should restore the opening Tag and close Layer");
    let mut rolled_back = false;
    for _ in 0..80 {
        let status = profile.cli(&["status"]);
        let description: String = Connection::open(profile.database_path())
            .unwrap()
            .query_row("SELECT description FROM active_session", [], |row| {
                row.get(0)
            })
            .unwrap();
        if description.is_empty()
            && status.status.success()
            && !combined_output(&status).contains("tag 'focus'")
        {
            rolled_back = true;
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(rolled_back, "Esc should restore the opening empty Tag");

    tui.write_all(b"\r")
        .expect("Enter should reopen the Layer modal");
    thread::sleep(Duration::from_millis(50));
    tui.write_all(b"focus")
        .expect("typing should preview the active Tag");
    tui.write_all(b"\r")
        .expect("Enter should accept the active Tag preview");

    let mut accepted = false;
    for _ in 0..80 {
        let description: String = Connection::open(profile.database_path())
            .unwrap()
            .query_row("SELECT description FROM active_session", [], |row| {
                row.get(0)
            })
            .unwrap();
        if description == "focus" {
            accepted = true;
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(accepted, "Enter should persist the accepted Tag preview");

    tui.write_all(b"\r")
        .expect("Enter should reopen the Layer modal");
    thread::sleep(Duration::from_millis(50));
    tui.write_all(b"\x7f\x7f\x7f\x7f\x7fdeep")
        .expect("Backspace and typing should preview the replacement Tag");
    let mut replacement_live = false;
    for _ in 0..80 {
        let status = profile.cli(&["status"]);
        if status.status.success() && combined_output(&status).contains("tag 'deep'") {
            replacement_live = true;
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(replacement_live, "the replacement Tag should preview live");

    tui.write_all(b"\x1b")
        .expect("Esc should restore the Tag accepted before this modal opened");
    let mut replacement_rolled_back = false;
    for _ in 0..80 {
        let description: String = Connection::open(profile.database_path())
            .unwrap()
            .query_row("SELECT description FROM active_session", [], |row| {
                row.get(0)
            })
            .unwrap();
        let status = profile.cli(&["status"]);
        if description == "focus"
            && status.status.success()
            && combined_output(&status).contains("tag 'focus'")
        {
            replacement_rolled_back = true;
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(
        replacement_rolled_back,
        "Esc should restore both the durable and live opening Tag"
    );

    tui.write_all(b"\r")
        .expect("Enter should reopen Layer for explicit acceptance");
    thread::sleep(Duration::from_millis(50));
    tui.write_all(b"\x7f\x7f\x7f\x7f\x7fdeep\r")
        .expect("Enter should accept the replacement Tag");
    let mut replacement_accepted = false;
    for _ in 0..80 {
        let description: String = Connection::open(profile.database_path())
            .unwrap()
            .query_row("SELECT description FROM active_session", [], |row| {
                row.get(0)
            })
            .unwrap();
        if description == "deep" {
            replacement_accepted = true;
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(
        replacement_accepted,
        "Enter should persist the replacement Tag"
    );

    tui.write_all(b"\r")
        .expect("Enter should reopen Layer for emergency-quit preview");
    thread::sleep(Duration::from_millis(50));
    tui.write_all(b"\x7f\x7f\x7f\x7ftemporary")
        .expect("typing should preview the unaccepted Tag");
    let mut emergency_preview_live = false;
    for _ in 0..80 {
        let status = profile.cli(&["status"]);
        if status.status.success() && combined_output(&status).contains("tag 'temporary'") {
            emergency_preview_live = true;
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(
        emergency_preview_live,
        "the unaccepted emergency-exit Tag should preview live"
    );
    thread::sleep(Duration::from_millis(1_100));

    tui.write_all(b"\x03")
        .expect("Ctrl-C should take the mandatory emergency-exit path");
    let output = tui.wait(PTY_TIMEOUT).expect("TUI should exit");
    assert!(output.status.success(), "{}", pty_output(&output));
    let active_after_quit: i64 = Connection::open(profile.database_path())
        .unwrap()
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        active_after_quit, 0,
        "mandatory Ctrl-C must finalize rather than preserve the active generation"
    );
    let last_description: String = Connection::open(profile.database_path())
        .unwrap()
        .query_row(
            "SELECT description FROM sessions ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        last_description, "deep",
        "Ctrl-C must discard the unaccepted Tag preview before finalization"
    );

    let mut reopened = spawn_live_tui(&profile);
    wait_for_path(&profile.control_socket_path());
    let status = profile.cli(&["status"]);
    assert!(status.status.success(), "{}", combined_output(&status));
    assert!(
        !combined_output(&status).contains("tag 'deep'"),
        "Ctrl-C must not resurrect the completed session as active"
    );
    quit_tui(&mut reopened);
}

#[test]
fn ctrl_x_balance_delete_requires_confirmation_and_preserves_sand() {
    let profile = TerminalProfile::new("balance-permanent-layer-delete");
    profile.seed_work_category();
    Connection::open(profile.database_path())
        .unwrap()
        .execute(
            "INSERT INTO category_tags(category_id, ordinal, tag) VALUES (1, 0, 'focus')",
            [],
        )
        .unwrap();
    let initial_sand = serde_json::json!({
        "version": 5,
        "grid_width": 2,
        "grid_height": 2,
        "grains": [{"x": 0, "y": 1, "category_id": 1}]
    })
    .to_string();
    Connection::open(profile.database_path())
        .unwrap()
        .execute(
            "INSERT INTO sand_state (
                singleton, formation_id, quantum_seconds, grid_width, grid_height,
                payload_json, updated_at_utc
             ) VALUES (1, 'delete-test', 1, 2, 2, ?1, '2026-09-28T00:00:00Z')",
            [initial_sand],
        )
        .unwrap();

    let mut tui = spawn_live_tui(&profile);
    wait_for_path(&profile.control_socket_path());

    let start = profile.cli(&["start", "Work", "--desc", "focus"]);
    assert!(start.status.success(), "{}", combined_output(&start));
    thread::sleep(Duration::from_millis(2_300));
    let stop = profile.cli(&["stop"]);
    assert!(stop.status.success(), "{}", combined_output(&stop));

    let sand_before = persisted_sand_state(&profile);
    assert!(
        sediment_mass_for_category(&sand_before.2, 1) > 0,
        "the test Layer should have materialized sand before deletion"
    );

    tui.write_all(b"b").expect("Balance should open from Main");
    thread::sleep(Duration::from_millis(150));
    tui.write_all(b"\x1b[B")
        .expect("selection should move from Idle to Work");
    thread::sleep(Duration::from_millis(100));
    tui.write_all(b"\x18")
        .expect("Ctrl+x should request permanent Layer deletion");
    thread::sleep(Duration::from_millis(250));

    let connection = Connection::open(profile.database_path()).unwrap();
    let sessions_before_confirmation: i64 = connection
        .query_row(
            "SELECT count(*) FROM sessions WHERE category_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let tags_before_confirmation: i64 = connection
        .query_row(
            "SELECT count(*) FROM category_tags WHERE category_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);
    assert!(sessions_before_confirmation > 0);
    assert_eq!(tags_before_confirmation, 1);

    tui.write_all(b"\x1b")
        .expect("Esc should cancel permanent Layer deletion");
    thread::sleep(Duration::from_millis(150));
    let still_present: String = Connection::open(profile.database_path())
        .unwrap()
        .query_row("SELECT name FROM categories WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(still_present, "Work");

    tui.write_all(b"\x18\r")
        .expect("Ctrl+x then Enter should confirm permanent Layer deletion");
    let mut deleted = false;
    for _ in 0..80 {
        let connection = Connection::open(profile.database_path()).unwrap();
        let sessions: i64 = connection
            .query_row(
                "SELECT count(*) FROM sessions WHERE category_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let tags: i64 = connection
            .query_row(
                "SELECT count(*) FROM category_tags WHERE category_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        if sessions == 0 && tags == 0 {
            deleted = true;
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(
        deleted,
        "confirmation should delete session and Tag history"
    );
    let sand_after = persisted_sand_state(&profile);
    assert_eq!(
        sand_after, sand_before,
        "Layer deletion must not rewrite sand"
    );
    let tombstone: (String, Option<String>) = Connection::open(profile.database_path())
        .unwrap()
        .query_row(
            "SELECT name, archived_at_utc FROM categories WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(tombstone.0, "__strata_deleted_layer_1__");
    assert!(tombstone.1.is_some());

    quit_tui(&mut tui);
}

#[test]
fn backspace_clears_only_idle_sediment_and_preserves_sqlite_extent() {
    let profile = TerminalProfile::new("backspace-idle-clear");
    profile.seed_work_category();

    let mut tui = spawn_live_tui(&profile);
    wait_for_path(&profile.control_socket_path());

    let start = profile.cli(&["start", "Work", "--desc", "focus"]);
    assert!(start.status.success(), "{}", combined_output(&start));
    thread::sleep(Duration::from_millis(2_300));

    let stop = profile.cli(&["stop"]);
    assert!(stop.status.success(), "{}", combined_output(&stop));
    thread::sleep(Duration::from_millis(2_300));

    quit_tui(&mut tui);

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
    tui.write_all(b"\x7f")
        .expect("Backspace should reach the real PTY");

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
        cleared.expect("Backspace should synchronously persist the Idle-only clear");
    assert_eq!((after_width, after_height), (before_width, before_height));
    assert_eq!(sediment_mass_for_category(&after_payload, 0), 0);
    assert_eq!(sediment_mass_for_category(&after_payload, 1), work_before);

    quit_tui(&mut tui);

    let mut reopened = spawn_live_tui(&profile);
    wait_for_path(&profile.control_socket_path());
    quit_tui(&mut reopened);

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
    let combined = pty_output(&output);
    assert!(combined.contains("injected TUI panic"));
    assert!(!combined.contains("emergency checkpoint: committed"));
}
