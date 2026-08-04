#![cfg(target_os = "linux")]

use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{Arc, Barrier},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "strata-profile-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn seed(profile: &Path, description: &str) {
    fs::create_dir_all(profile.join("data")).unwrap();
    fs::write(
        profile.join("data/categories.csv"),
        format!("id,name,description,color_index,karma_effect\n1,Work,{description},0,1\n"),
    )
    .unwrap();
}

fn run(profile: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_strata"))
        .arg("--profile")
        .arg(profile)
        .args(args)
        .env_remove("STRATA_PROFILE")
        .env_remove("STRATA_DATA_DIR")
        .output()
        .expect("Strata process should run")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn detach(profile: &Path) -> Output {
    let command_line = format!(
        "stty cols 100 rows 30; exec {} --profile {}",
        env!("CARGO_BIN_EXE_strata"),
        profile.display()
    );
    let mut child = Command::new("timeout")
        .args(["12s", "script", "-qefc", &command_line, "/dev/null"])
        .env_remove("STRATA_PROFILE")
        .env_remove("STRATA_DATA_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("TUI should start in a PTY");
    thread::sleep(Duration::from_millis(1200));
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"d").unwrap();
    stdin.flush().unwrap();
    drop(stdin);
    child.wait_with_output().expect("TUI should finish")
}

#[test]
fn rooted_profiles_isolate_paths_and_bind_active_state() {
    let a = root("a");
    let b = root("b");
    seed(&a, "A metadata");
    seed(&b, "B metadata");

    let info_a = run(&a, &["profile", "--json"]);
    let info_b = run(&b, &["profile", "--json"]);
    assert!(info_a.status.success(), "{}", stderr(&info_a));
    assert!(info_b.status.success(), "{}", stderr(&info_b));
    let a_json: serde_json::Value = serde_json::from_slice(&info_a.stdout).unwrap();
    let b_json: serde_json::Value = serde_json::from_slice(&info_b.stdout).unwrap();
    assert_ne!(a_json["profile_id"], b_json["profile_id"]);
    assert_eq!(a_json["root"], a.to_string_lossy().as_ref());

    let started = run(&a, &["start", "A project", "--category", "Work"]);
    assert!(started.status.success(), "{}", stderr(&started));
    assert!(a.join("state/active_session.json").exists());
    assert!(!b.join("state/active_session.json").exists());

    let wrong_stop = run(&b, &["stop"]);
    assert!(!wrong_stop.status.success());
    assert!(stderr(&wrong_stop).contains("No active session"));

    fs::create_dir_all(b.join("state")).unwrap();
    fs::copy(
        a.join("state/active_session.json"),
        b.join("state/active_session.json"),
    )
    .unwrap();
    let copied_stop = run(&b, &["stop"]);
    assert!(!copied_stop.status.success());
    assert!(stderr(&copied_stop).contains("belongs to profile"));

    thread::sleep(Duration::from_millis(1_100));
    let stopped = run(&a, &["stop"]);
    assert!(stopped.status.success(), "{}", stderr(&stopped));
    assert!(a.join("data/time_log.csv").exists());
    assert!(!b.join("data/time_log.csv").exists());

    fs::remove_dir_all(a).ok();
    fs::remove_dir_all(b).ok();
}

#[test]
fn copied_detached_checkpoint_is_refused_by_target_profile() {
    let a = root("checkpoint-a");
    let b = root("checkpoint-b");
    seed(&a, "A metadata");
    seed(&b, "B metadata");

    let detached = detach(&a);
    assert!(detached.status.success(), "{}", stderr(&detached));
    let checkpoint = a.join("state/detached_runtime.json");
    assert!(checkpoint.exists());
    fs::create_dir_all(b.join("state")).unwrap();
    fs::copy(&checkpoint, b.join("state/detached_runtime.json")).unwrap();

    let command_line = format!(
        "exec {} --profile {}",
        env!("CARGO_BIN_EXE_strata"),
        b.display()
    );
    let target = Command::new("timeout")
        .args(["8s", "script", "-qefc", &command_line, "/dev/null"])
        .output()
        .expect("target TUI should run under a PTY");
    assert!(!target.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&target.stdout),
        String::from_utf8_lossy(&target.stderr)
    );
    assert!(combined.contains("belongs to profile"), "{combined}");

    fs::remove_dir_all(a).ok();
    fs::remove_dir_all(b).ok();
}

#[test]
fn explicit_profile_overrides_conflicting_environment_selectors() {
    let explicit = root("explicit-precedence");
    let env_profile = root("environment-profile");
    let legacy_alias = root("legacy-alias");

    let output = Command::new(env!("CARGO_BIN_EXE_strata"))
        .arg("--profile")
        .arg(&explicit)
        .args(["profile", "--json"])
        .env("STRATA_PROFILE", &env_profile)
        .env("STRATA_DATA_DIR", &legacy_alias)
        .output()
        .expect("Strata process should run");

    assert!(output.status.success(), "{}", stderr(&output));
    let description: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(description["root"], explicit.to_string_lossy().as_ref());
    assert!(explicit.join("profile.json").exists());
    assert!(!env_profile.exists());
    assert!(!legacy_alias.exists());

    fs::remove_dir_all(explicit).ok();
}

#[test]
fn concurrent_first_use_converges_on_one_durable_profile_identity() {
    for round in 0..8 {
        let profile = root(&format!("concurrent-first-use-{round}"));
        let workers = 24;
        let barrier = Arc::new(Barrier::new(workers));
        let mut handles = Vec::with_capacity(workers);

        for _ in 0..workers {
            let profile = profile.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                run(&profile, &["profile", "--json"])
            }));
        }

        let outputs = handles
            .into_iter()
            .map(|handle| handle.join().expect("profile worker should finish"))
            .collect::<Vec<_>>();
        let failures = outputs
            .iter()
            .filter(|output| !output.status.success())
            .map(stderr)
            .collect::<Vec<_>>();
        assert!(
            failures.is_empty(),
            "concurrent starts failed: {failures:?}"
        );

        let ids = outputs
            .iter()
            .map(|output| {
                let description: serde_json::Value =
                    serde_json::from_slice(&output.stdout).expect("valid profile JSON");
                description["profile_id"]
                    .as_str()
                    .expect("profile ID should be present")
                    .to_string()
            })
            .collect::<HashSet<_>>();
        assert_eq!(
            ids.len(),
            1,
            "workers observed divergent profile IDs: {ids:?}"
        );

        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(profile.join("profile.json")).expect("durable profile manifest"),
        )
        .expect("valid durable profile manifest");
        assert_eq!(
            manifest["profile_id"].as_str(),
            ids.iter().next().map(String::as_str),
            "process identity must match the durable manifest"
        );

        fs::remove_dir_all(profile).ok();
    }
}
