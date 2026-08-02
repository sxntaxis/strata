from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


app = Path("src/app.rs")
text = app.read_text()
text = text.replace(
    '''use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};
''',
    '''use crossterm::event::{self, Event};
use ratatui::layout::Rect;
''',
    1,
)
text = text.replace("mod time_format;\n", "mod terminal_lifecycle;\nmod time_format;\n", 1)
text = text.replace(
    "use persistence_recovery::{PersistenceOperation, PersistenceRecoveryState, RecoveryAction};\n",
    "use persistence_recovery::{PersistenceOperation, PersistenceRecoveryState, RecoveryAction};\nuse terminal_lifecycle::{ManagedTerminal, TerminalSession};\n",
    1,
)
app.write_text(text)

replace_between(
    "src/app.rs",
    "    fn persist_runtime_checkpoint(&mut self)",
    "    fn clear_detached_checkpoint(&mut self)",
    '''    fn build_runtime_checkpoint(&self) -> Result<DetachedRuntimeCheckpoint, String> {
        if self.checkpoint_recovery_active {
            return Err("checkpoint recovery is still active".to_string());
        }
        if !self.simulation.pending_mutations.is_empty() {
            return Err(
                "runtime checkpoint cannot be written while mutations are pending".to_string(),
            );
        }

        let active_category_id = self.time_tracker.active_category_id();
        let active_description = self
            .time_tracker
            .category_description_by_id(active_category_id)
            .unwrap_or_default()
            .to_string();
        let spawn_accumulator_nanos = u64::try_from(self.simulation.spawn_accumulator.as_nanos())
            .map_err(|_| "spawn accumulator exceeds checkpoint range".to_string())?;
        let physics_accumulator_nanos =
            u64::try_from(self.simulation.physics_accumulator.as_nanos())
                .map_err(|_| "physics accumulator exceeds checkpoint range".to_string())?;

        Ok(DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: Utc::now(),
            simulation_time_utc: self.simulation.simulation_time_utc,
            spawn_accumulator_nanos,
            physics_accumulator_nanos,
            active_category_id: active_category_id.0,
            active_description,
            active_session_started_at_utc: self.session.active_session_started_at_utc,
            sand_state: self.sand_engine.snapshot_state(),
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
        })
    }

    fn try_write_runtime_checkpoint(&self) -> Result<(), String> {
        let checkpoint = self.build_runtime_checkpoint()?;
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let expected_stable_id = self
                .session
                .active_session_stable_id
                .as_deref()
                .ok_or_else(|| {
                    "SQLite runtime has no active stable identity to checkpoint".to_string()
                })?;
            sqlite::save_tui_checkpoint(
                &database_path,
                expected_stable_id,
                checkpoint.detached_at_utc,
                checkpoint.simulation_time_utc,
                &checkpoint,
            )
        } else {
            storage::write_json_atomic(&storage::get_detached_runtime_path(), &checkpoint)
        }
    }

    fn try_emergency_runtime_checkpoint(&self) -> Result<(), String> {
        self.try_write_runtime_checkpoint()
    }

    fn persist_runtime_checkpoint(&mut self) {
        if self.checkpoint_recovery_active {
            return;
        }
        let result = self.try_write_runtime_checkpoint();
        self.record_storage_result_for(
            PersistenceOperation::CheckpointSave,
            RecoveryAction::DetachAndExit,
            result,
        );
    }

''',
)

replace_between(
    "src/app.rs",
    "pub fn run_ui(loaded: keybindings::LoadedKeybindings)",
    "#[cfg(test)]\nmod bounded_checkpoint_tests",
    '''fn run_application_loop(
    app: &mut App,
    terminal: &mut ManagedTerminal,
) -> Result<Option<String>, io::Error> {
    let physics_rate = Duration::from_millis(TIME_SETTINGS.physics_ms);
    let tick_rate = Duration::from_millis(TIME_SETTINGS.tick_ms);
    let render_rate = Duration::from_millis(1000 / TIME_SETTINGS.target_fps);
    let save_rate = Duration::from_secs(RUNTIME_LOOP_SETTINGS.autosave_secs);
    let mut last_simulation_update = Instant::now();
    let mut last_render = Instant::now();
    let mut last_save = Instant::now();
    let mut runtime_error = None;

    'runtime: loop {
        loop {
            if !app.has_persistence_recovery() {
                let now = Instant::now();
                let wall_delta = now.saturating_duration_since(last_simulation_update);
                last_simulation_update = now;
                app.advance_runtime(wall_delta, tick_rate, physics_rate);

                if last_save.elapsed() >= save_rate {
                    app.persist_sessions();
                    if !app.has_persistence_recovery() {
                        app.persist_sand_state();
                    }
                    if !app.has_persistence_recovery() {
                        app.persist_daily_sand_snapshot();
                    }
                    if !app.has_persistence_recovery() {
                        app.persist_runtime_checkpoint();
                    }
                    last_save = Instant::now();
                }

                app.refresh_keymap_if_changed();
            }

            if last_render.elapsed() >= render_rate && app.render_needed {
                terminal_lifecycle::maybe_inject_runtime_io_fault("draw")?;
                terminal.draw(|frame| {
                    app.draw_frame(frame);
                })?;
                app.render_needed = false;
                last_render = Instant::now();
            }

            terminal_lifecycle::maybe_inject_runtime_io_fault("poll")?;
            if event::poll(Duration::from_millis(RUNTIME_LOOP_SETTINGS.input_poll_ms))? {
                terminal_lifecycle::maybe_inject_runtime_io_fault("read")?;
                if let Event::Key(key) = event::read()? {
                    if app.handle_key(key) {
                        break;
                    }
                    if app.detach_requested {
                        break;
                    }
                }
            }
        }

        if app.recovery_exit_requested {
            runtime_error = app.recovery_exit_error.take();
            break 'runtime;
        }

        if app.has_persistence_recovery() {
            continue 'runtime;
        }

        if app.detach_requested {
            app.persist_sessions();
            if !app.has_persistence_recovery() {
                app.persist_sand_state();
            }
            if !app.has_persistence_recovery() {
                app.persist_daily_sand_snapshot();
            }
            if !app.has_persistence_recovery() {
                app.persist_runtime_checkpoint();
            }
            if app.has_persistence_recovery() {
                app.promote_recovery_action(RecoveryAction::DetachAndExit);
                app.detach_requested = false;
                continue 'runtime;
            }
        } else {
            app.end_active_session_now();
            if !app.has_persistence_recovery() {
                app.persist_sessions();
            }
            if !app.has_persistence_recovery() {
                app.persist_sand_state();
            }
            if !app.has_persistence_recovery() {
                app.persist_daily_sand_snapshot();
            }
            if !app.has_persistence_recovery() {
                app.clear_detached_checkpoint();
            }
            if app.has_persistence_recovery() {
                app.promote_recovery_action(RecoveryAction::FinishAndExit);
                continue 'runtime;
            }
        }

        break 'runtime;
    }

    Ok(runtime_error)
}

pub fn run_ui(loaded: keybindings::LoadedKeybindings) -> Result<(), io::Error> {
    let (width, height) = crossterm::terminal::size()?;
    let mut app = App::new(width, height, loaded).map_err(io::Error::other)?;
    let mut terminal_session = TerminalSession::enter()?;
    terminal_lifecycle::maybe_inject_runtime_panic();

    match run_application_loop(&mut app, terminal_session.terminal_mut()) {
        Ok(application_error) => {
            let cleanup_result = terminal_session.restore();
            terminal_lifecycle::finish_normal_run(application_error, cleanup_result)
        }
        Err(primary) => {
            let checkpoint_result = app.try_emergency_runtime_checkpoint();
            let cleanup_result = terminal_session.restore();
            Err(terminal_lifecycle::compose_runtime_failure(
                primary,
                checkpoint_result,
                cleanup_result,
            ))
        }
    }
}

''',
)

# Add Linux PTY process certification.
Path("tests/terminal_lifecycle.rs").write_text(r'''#![cfg(target_os = "linux")]

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
        fs::write(
            data_home.join("strata/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,Focused work,0,1\n",
        )
        .unwrap();
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

    fn checkpoint_path(&self) -> PathBuf {
        self.state_home.join("strata/detached_runtime.json")
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

#[test]
fn normal_quit_and_detach_restore_terminal_once() {
    let quit = TerminalProfile::new("quit");
    let quit_output = quit.run_case(None, Some(b'q'));
    assert!(quit_output.status.success(), "{}", combined_output(&quit_output));
    assert_terminal_restored(&quit_output);

    let detach = TerminalProfile::new("detach");
    let detach_output = detach.run_case(None, Some(b'd'));
    assert!(
        detach_output.status.success(),
        "{}",
        combined_output(&detach_output)
    );
    assert_terminal_restored(&detach_output);
    assert!(detach.checkpoint_path().exists());
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
        assert!(profile.checkpoint_path().exists());
    }
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
''')

for temporary in [
    ".github/workflows/interaction001b-apply.yml",
    "tools/interaction001b-apply.py",
    "tools/interaction001b.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
