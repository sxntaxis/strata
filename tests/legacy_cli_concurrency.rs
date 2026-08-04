#![cfg(target_os = "linux")]

use chrono::{Duration as ChronoDuration, Utc};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Arc, Barrier},
    thread,
    time::{SystemTime, UNIX_EPOCH},
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
            "strata-legacy-cli-race-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(
            root.join("data/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .unwrap();
        let profile = Self { root };
        let initialized = profile.run(&["profile", "--json"]);
        assert!(
            initialized.status.success(),
            "profile initialization failed: {}",
            String::from_utf8_lossy(&initialized.stderr)
        );
        profile
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_strata"));
        command
            .arg("--profile")
            .arg(&self.root)
            .env_remove("STRATA_PROFILE")
            .env_remove("STRATA_DATA_DIR");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command()
            .args(args)
            .output()
            .expect("Strata process should run")
    }

    fn active_path(&self) -> PathBuf {
        self.root.join("state/active_session.json")
    }

    fn time_log_path(&self) -> PathBuf {
        self.root.join("data/time_log.csv")
    }

    fn backdate_active(&self, seconds: i64) {
        let path = self.active_path();
        let mut value: serde_json::Value = serde_json::from_slice(
            &fs::read(&path).expect("active session should be readable"),
        )
        .expect("active session should be valid JSON");
        value["start_time"] = serde_json::Value::String(
            (Utc::now() - ChronoDuration::seconds(seconds)).to_rfc3339(),
        );
        fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
}

impl Drop for Profile {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn concurrent_runs(profile: &Profile, argument_sets: Vec<Vec<String>>) -> Vec<Output> {
    let workers = argument_sets.len();
    let barrier = Arc::new(Barrier::new(workers));
    let root = profile.root.clone();
    let mut handles = Vec::with_capacity(workers);

    for args in argument_sets {
        let barrier = Arc::clone(&barrier);
        let root = root.clone();
        handles.push(thread::spawn(move || {
            barrier.wait();
            Command::new(env!("CARGO_BIN_EXE_strata"))
                .arg("--profile")
                .arg(root)
                .args(args)
                .env_remove("STRATA_PROFILE")
                .env_remove("STRATA_DATA_DIR")
                .output()
                .expect("Strata process should run")
        }));
    }

    handles
        .into_iter()
        .map(|handle| handle.join().expect("worker should finish"))
        .collect()
}

fn successful_projects(outputs: &[Output]) -> HashSet<String> {
    outputs
        .iter()
        .enumerate()
        .filter(|(_, output)| output.status.success())
        .map(|(index, _)| format!("project-{index}"))
        .collect()
}

#[test]
fn concurrent_starts_publish_exactly_one_active_generation() {
    for round in 0..6 {
        let profile = Profile::new(&format!("start-{round}"));
        let workers = 24;
        let arguments = (0..workers)
            .map(|index| {
                vec![
                    "start".to_string(),
                    format!("project-{index}"),
                    "--category".to_string(),
                    "Work".to_string(),
                ]
            })
            .collect();
        let outputs = concurrent_runs(&profile, arguments);
        let winners = successful_projects(&outputs);

        assert_eq!(
            winners.len(),
            1,
            "exactly one concurrent start may succeed; winners={winners:?}; errors={:?}",
            outputs
                .iter()
                .filter(|output| !output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stderr).into_owned())
                .collect::<Vec<_>>()
        );

        let active: serde_json::Value = serde_json::from_slice(
            &fs::read(profile.active_path()).expect("one active session should exist"),
        )
        .expect("active session should be valid JSON");
        let active_project = active["project"]
            .as_str()
            .expect("active project should be present");
        assert!(winners.contains(active_project));
    }
}

#[test]
fn concurrent_stops_commit_exactly_one_terminal_transition() {
    for round in 0..6 {
        let profile = Profile::new(&format!("stop-{round}"));
        let start = profile.run(&["start", "single-project", "--category", "Work"]);
        assert!(
            start.status.success(),
            "start failed: {}",
            String::from_utf8_lossy(&start.stderr)
        );
        profile.backdate_active(2);

        let workers = 24;
        let outputs = concurrent_runs(
            &profile,
            (0..workers).map(|_| vec!["stop".to_string()]).collect(),
        );
        let successes = outputs
            .iter()
            .filter(|output| output.status.success())
            .count();
        assert_eq!(
            successes,
            1,
            "exactly one concurrent stop may succeed; outputs={:?}",
            outputs
                .iter()
                .map(|output| (
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr).into_owned()
                ))
                .collect::<Vec<_>>()
        );
        assert!(!profile.active_path().exists());

        let log = fs::read_to_string(profile.time_log_path())
            .expect("one committed session log should exist");
        let rows = log.lines().skip(1).filter(|line| !line.is_empty()).count();
        assert_eq!(rows, 1, "one stop must commit one completed row: {log}");
    }
}

#[allow(dead_code)]
fn assert_path_exists(path: &Path) {
    assert!(path.exists(), "expected {} to exist", path.display());
}
