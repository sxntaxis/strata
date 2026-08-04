#![cfg(target_os = "linux")]

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Profile {
    root: PathBuf,
}

impl Profile {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should follow epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "strata-mixed-legacy-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(
            root.join("data/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .unwrap();
        Self { root }
    }

    fn active_path(&self) -> PathBuf {
        self.root.join("state/active_session.json")
    }

    fn run_cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_strata"))
            .arg("--profile")
            .arg(&self.root)
            .args(args)
            .env_remove("STRATA_PROFILE")
            .env_remove("STRATA_DATA_DIR")
            .output()
            .expect("CLI process should run")
    }

    fn launch_tui(&self) -> Child {
        let command_line = format!(
            "stty cols 100 rows 30; exec {} --profile {}",
            env!("CARGO_BIN_EXE_strata"),
            self.root.display()
        );
        Command::new("timeout")
            .args(["12s", "script", "-qefc", &command_line, "/dev/null"])
            .env_remove("STRATA_PROFILE")
            .env_remove("STRATA_DATA_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("TUI should start in a PTY")
    }
}

impl Drop for Profile {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn wait_for_active(profile: &Profile, child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        assert!(
            child.try_wait().unwrap().is_none(),
            "TUI exited before publishing active state"
        );
        if profile.active_path().exists() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("TUI did not publish active state before deadline");
}

#[test]
fn cli_cannot_stop_generation_owned_by_live_legacy_tui() {
    let profile = Profile::new("tui-cli-stop");
    let mut tui = profile.launch_tui();
    wait_for_active(&profile, &mut tui);

    let stop = profile.run_cli(&["stop"]);
    assert!(
        !stop.status.success(),
        "CLI stop terminated a generation still owned by the live TUI: stdout={} stderr={}",
        String::from_utf8_lossy(&stop.stdout),
        String::from_utf8_lossy(&stop.stderr)
    );
    assert!(
        String::from_utf8_lossy(&stop.stderr)
            .to_ascii_lowercase()
            .contains("legacy lifecycle"),
        "CLI stop did not report the competing lifecycle owner: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    assert!(profile.active_path().exists());

    let mut stdin = tui.stdin.take().expect("TUI stdin should remain available");
    stdin.write_all(b"q").unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    let output = tui.wait_with_output().expect("TUI should finish");
    assert!(
        output.status.success(),
        "TUI failed after refusing concurrent CLI stop: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
