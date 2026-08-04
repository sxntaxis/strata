#![cfg(target_os = "linux")]

// These process proofs require damaged authority files to remain byte-for-byte untouched.
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "strata-legacy-custody-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn seed(profile: &Path) {
    fs::create_dir_all(profile.join("data")).unwrap();
    fs::create_dir_all(profile.join("state")).unwrap();
    fs::write(
        profile.join("data/categories.csv"),
        "id,name,description,color_index,karma_effect\n1,Work,Stable metadata,0,1\n",
    )
    .unwrap();
}

fn run_tui(profile: &Path) -> Output {
    let command_line = format!(
        "stty cols 100 rows 30; exec {} --profile {}",
        env!("CARGO_BIN_EXE_strata"),
        profile.display()
    );
    let mut child = Command::new("timeout")
        .args(["8s", "script", "-qefc", &command_line, "/dev/null"])
        .env_remove("STRATA_PROFILE")
        .env_remove("STRATA_DATA_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("TUI should start in a PTY");
    thread::sleep(Duration::from_millis(900));
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"q");
        let _ = stdin.flush();
    }
    child.wait_with_output().expect("TUI process should finish")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn malformed_category_tags_block_startup_without_rewrite() {
    let profile = root("malformed-tags");
    seed(&profile);
    let path = profile.join("state/category_tags.json");
    let original = b"{not valid category tags";
    fs::write(&path, original).unwrap();

    let output = run_tui(&profile);
    let evidence = combined(&output);
    assert!(
        !output.status.success(),
        "startup unexpectedly succeeded: {evidence}"
    );
    assert!(
        evidence.to_ascii_lowercase().contains("category tags"),
        "missing actionable category-tags failure: {evidence}"
    );
    assert_eq!(fs::read(&path).unwrap(), original);

    fs::remove_dir_all(profile).ok();
}

#[test]
fn malformed_sand_state_blocks_startup_without_rewrite() {
    let profile = root("malformed-sand");
    seed(&profile);
    let path = profile.join("state/sand_state.json");
    let original = b"{not valid sediment";
    fs::write(&path, original).unwrap();

    let output = run_tui(&profile);
    let evidence = combined(&output);
    assert!(
        !output.status.success(),
        "startup unexpectedly succeeded: {evidence}"
    );
    assert!(
        evidence.to_ascii_lowercase().contains("sand state")
            || evidence.to_ascii_lowercase().contains("sediment"),
        "missing actionable sediment failure: {evidence}"
    );
    assert_eq!(fs::read(&path).unwrap(), original);

    fs::remove_dir_all(profile).ok();
}

#[test]
fn unknown_sediment_identity_blocks_startup_without_rewrite() {
    let profile = root("unknown-sediment-identity");
    seed(&profile);
    let path = profile.join("state/sand_state.json");
    let original = br#"{
  "version": 2,
  "grid_width": 1,
  "grid_height": 1,
  "grains": [{"x": 0, "y": 0, "category_id": 999}],
  "frame_count": 0,
  "sweep_left_to_right": true,
  "rng_state": 1,
  "pending_grains": [],
  "pending_runs": []
}"#;
    fs::write(&path, original).unwrap();

    let output = run_tui(&profile);
    let evidence = combined(&output);
    assert!(
        !output.status.success(),
        "startup unexpectedly succeeded: {evidence}"
    );
    assert!(
        evidence.contains("999") && evidence.to_ascii_lowercase().contains("category"),
        "missing unknown-category failure: {evidence}"
    );
    assert_eq!(fs::read(&path).unwrap(), original);

    fs::remove_dir_all(profile).ok();
}
