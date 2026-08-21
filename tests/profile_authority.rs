#![cfg(target_os = "linux")]

use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::Connection;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Arc, Barrier},
    thread,
    time::{SystemTime, UNIX_EPOCH},
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

fn database_path(profile: &Path) -> PathBuf {
    profile.join("data/strata.sqlite3")
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

fn seed(profile: &Path, description: &str) {
    let initialized = run(profile, &["report", "--today"]);
    assert!(initialized.status.success(), "{}", stderr(&initialized));
    Connection::open(database_path(profile))
        .unwrap()
        .execute(
            "INSERT INTO categories(id, name, description, color_index, balance_effect, sort_order)
             VALUES (1, 'Work', ?1, 0, 1, 1)",
            [description],
        )
        .unwrap();
}

fn row_count(profile: &Path, table: &str) -> i64 {
    Connection::open(database_path(profile))
        .unwrap()
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn rooted_profiles_isolate_database_state() {
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

    let started = run(&a, &["start", "Work"]);
    assert!(started.status.success(), "{}", stderr(&started));
    assert_eq!(row_count(&a, "active_session"), 1);
    assert_eq!(row_count(&b, "active_session"), 0);

    let wrong_stop = run(&b, &["stop"]);
    assert!(!wrong_stop.status.success());
    assert!(stderr(&wrong_stop).contains("No active"));

    let started_at = (Utc::now() - ChronoDuration::seconds(2)).to_rfc3339();
    Connection::open(database_path(&a))
        .unwrap()
        .execute(
            "UPDATE active_session SET started_at_utc = ?1",
            [started_at],
        )
        .unwrap();
    let stopped = run(&a, &["stop"]);
    assert!(stopped.status.success(), "{}", stderr(&stopped));
    assert_eq!(row_count(&a, "sessions"), 1);
    assert_eq!(row_count(&b, "sessions"), 0);

    assert!(!a.join("data/categories.csv").exists());
    assert!(!a.join("data/time_log.csv").exists());
    assert!(!a.join("state/active_session.json").exists());

    fs::remove_dir_all(a).ok();
    fs::remove_dir_all(b).ok();
}

#[test]
fn copied_database_is_refused_by_target_profile() {
    let a = root("database-a");
    let b = root("database-b");
    seed(&a, "A metadata");
    seed(&b, "B metadata");

    fs::copy(database_path(&a), database_path(&b)).unwrap();
    let target = run(&b, &["report", "--today"]);
    assert!(!target.status.success());
    assert!(stderr(&target).contains("database belongs to profile"));

    fs::remove_dir_all(a).ok();
    fs::remove_dir_all(b).ok();
}

#[test]
fn explicit_profile_overrides_conflicting_environment_selector() {
    let explicit = root("explicit-precedence");
    let env_profile = root("environment-profile");

    let output = Command::new(env!("CARGO_BIN_EXE_strata"))
        .arg("--profile")
        .arg(&explicit)
        .args(["profile", "--json"])
        .env("STRATA_PROFILE", &env_profile)
        .output()
        .expect("Strata process should run");

    assert!(output.status.success(), "{}", stderr(&output));
    let description: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(description["root"], explicit.to_string_lossy().as_ref());
    assert!(explicit.join("profile.json").exists());
    assert!(!env_profile.exists());

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
