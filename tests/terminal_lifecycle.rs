#![cfg(target_os = "linux")]

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
