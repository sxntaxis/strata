#![cfg(target_os = "linux")]

use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use serde_json::Value;

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
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "strata-sqlite006-{name}-{}-{nonce}",
            std::process::id()
        ));
        let data_home = root.join("data");
        let state_home = root.join("state");
        let config_home = root.join("config");
        fs::create_dir_all(data_home.join("strata")).expect("data directory should be created");
        fs::create_dir_all(&state_home).expect("state directory should be created");
        fs::create_dir_all(&config_home).expect("config directory should be created");
        fs::write(
            data_home.join("strata/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,Focused work,0,1\n",
        )
        .expect("categories should be written");
        Self {
            root,
            data_home,
            state_home,
            config_home,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_strata"));
        command
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR");
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command()
            .args(args)
            .output()
            .expect("Strata subprocess should run")
    }

    fn run_tui(&self) -> Output {
        self.run_tui_with_input(b"q", None)
    }

    fn run_tui_with_input(&self, input: &[u8], fault: Option<&str>) -> Output {
        let mut command = Command::new("timeout");
        let tui_command = format!(
            "stty cols 120 rows 40; exec {}",
            env!("CARGO_BIN_EXE_strata")
        );
        command
            .args(["10s", "script", "-qefc", &tui_command, "/dev/null"])
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR")
            .env_remove("STRATA_TEST_SQLITE_FAULT")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(fault) = fault {
            command.env("STRATA_TEST_SQLITE_FAULT", fault);
        }
        let mut child = command.spawn().expect("pseudo-terminal TUI should start");
        let mut stdin = child.stdin.take().expect("TUI stdin should exist");
        for byte in input {
            thread::sleep(Duration::from_millis(1_100));
            if let Err(error) = stdin.write_all(std::slice::from_ref(byte)) {
                if error.kind() == std::io::ErrorKind::BrokenPipe {
                    break;
                }
                panic!("TUI input should be written: {error}");
            }
            if let Err(error) = stdin.flush() {
                if error.kind() == std::io::ErrorKind::BrokenPipe {
                    break;
                }
                panic!("TUI input should flush: {error}");
            }
        }
        drop(stdin);
        child.wait_with_output().expect("TUI process should finish")
    }

    fn run_tui_lifecycle_merge(&self) -> Output {
        let mut command = Command::new("timeout");
        let tui_command = format!(
            "stty cols 120 rows 40; exec {}",
            env!("CARGO_BIN_EXE_strata")
        );
        command
            .args(["20s", "script", "-qefc", &tui_command, "/dev/null"])
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().expect("pseudo-terminal TUI should start");
        let mut stdin = child.stdin.take().expect("TUI stdin should exist");
        let mut stdout = child.stdout.take().expect("TUI stdout should exist");
        let mut stderr = child.stderr.take().expect("TUI stderr should exist");
        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        let captured_stdout = Arc::clone(&captured);
        let stdout_reader = thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match stdout.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => captured_stdout
                        .lock()
                        .expect("captured stdout lock should be available")
                        .extend_from_slice(&buffer[..count]),
                    Err(error) => panic!("TUI stdout should be readable: {error}"),
                }
            }
        });
        let stderr_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr
                .read_to_end(&mut bytes)
                .expect("TUI stderr should be readable");
            bytes
        });

        let send = |stdin: &mut std::process::ChildStdin, bytes: &[u8]| {
            stdin.write_all(bytes).expect("TUI input should be written");
            stdin.flush().expect("TUI input should flush");
            thread::sleep(Duration::from_millis(900));
        };

        thread::sleep(Duration::from_millis(1_200));
        send(&mut stdin, b"\r");
        send(&mut stdin, b"\x1b[B");
        send(&mut stdin, b"X");
        send(&mut stdin, b"\r");

        let deadline = Instant::now() + Duration::from_secs(6);
        let phrase = loop {
            let bytes = captured
                .lock()
                .expect("captured stdout lock should be available")
                .clone();
            if let Some(phrase) = lifecycle_confirmation_phrase(&bytes) {
                break phrase;
            }
            assert!(
                Instant::now() < deadline,
                "lifecycle confirmation phrase was not rendered: {:?}",
                String::from_utf8_lossy(&bytes)
            );
            thread::sleep(Duration::from_millis(100));
        };

        stdin
            .write_all(phrase.as_bytes())
            .expect("confirmation phrase should be written");
        stdin.flush().expect("confirmation phrase should flush");
        send(&mut stdin, b"\r");

        let receipt_deadline = Instant::now() + Duration::from_secs(6);
        loop {
            if self.lifecycle_receipt_count() == 1 {
                break;
            }
            assert!(
                Instant::now() < receipt_deadline,
                "lifecycle receipt was not committed after exact confirmation"
            );
            thread::sleep(Duration::from_millis(100));
        }
        send(&mut stdin, b"q");
        drop(stdin);

        let status = child.wait().expect("TUI process should finish");
        stdout_reader.join().expect("stdout reader should finish");
        let stderr = stderr_reader.join().expect("stderr reader should finish");
        let stdout = captured
            .lock()
            .expect("captured stdout lock should be available")
            .clone();
        Output {
            status,
            stdout,
            stderr,
        }
    }

    fn lifecycle_receipt_count(&self) -> i64 {
        let connection = Connection::open(self.database_path()).expect("database should open");
        connection
            .query_row(
                "SELECT count(*) FROM category_lifecycle_receipts",
                [],
                |row| row.get(0),
            )
            .expect("lifecycle receipt count should be readable")
    }

    fn recovery_files(&self) -> Vec<PathBuf> {
        let directory = self.state_home.join("strata/recovery");
        let mut files: Vec<PathBuf> = fs::read_dir(directory)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        files
    }

    fn database_path(&self) -> PathBuf {
        self.data_home.join("strata/strata.sqlite3")
    }

    fn marker_path(&self) -> PathBuf {
        self.state_home.join("strata/storage_authority.json")
    }

    fn categories_path(&self) -> PathBuf {
        self.data_home.join("strata/categories.csv")
    }

    fn legacy_active_path(&self) -> PathBuf {
        self.state_home.join("strata/active_session.json")
    }

    fn legacy_time_log_path(&self) -> PathBuf {
        self.data_home.join("strata/time_log.csv")
    }

    fn backdate_sqlite_active_session(&self, seconds: i64) {
        let connection = Connection::open(self.database_path()).expect("database should open");
        let started_at = (Utc::now() - ChronoDuration::seconds(seconds))
            .to_rfc3339_opts(SecondsFormat::Nanos, true);
        let changed = connection
            .execute(
                "UPDATE active_session SET started_at_utc = ?1 WHERE singleton = 1",
                [started_at],
            )
            .expect("active session should be backdated");
        assert_eq!(changed, 1, "one active SQLite session should be backdated");
    }

    fn migrate(&self) {
        let output = self.run(&["migrate-sqlite"]);
        assert!(
            output.status.success(),
            "migration failed: {}",
            stderr(&output)
        );
    }

    fn activate(&self) -> Output {
        self.run(&["activate-sqlite", "--confirm", "--json"])
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn lifecycle_confirmation_phrase(output: &[u8]) -> Option<String> {
    let rendered = String::from_utf8_lossy(output);
    let prefix = "MERGE 1 INTO 2 ";
    for (start, _) in rendered.rmatch_indices(prefix) {
        let revision: String = rendered[start + prefix.len()..].chars().take(16).collect();
        if revision.len() == 16
            && revision
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Some(format!("{prefix}{revision}"));
        }
    }
    None
}

fn read_marker(profile: &TestProfile) -> Value {
    let bytes = fs::read(profile.marker_path()).expect("authority marker should be readable");
    serde_json::from_slice(&bytes).expect("authority marker should be valid JSON")
}

#[test]
fn activation_requires_confirmation_and_preserves_legacy_authority() {
    let profile = TestProfile::new("confirmation");
    profile.migrate();

    let output = profile.run(&["activate-sqlite"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("requires --confirm"));
    assert_eq!(read_marker(&profile)["active_authority"], "legacy-files");
    let connection = Connection::open(profile.database_path()).expect("database should open");
    let authority: String = connection
        .query_row(
            "SELECT value FROM database_metadata WHERE key = 'storage_authority'",
            [],
            |row| row.get(0),
        )
        .expect("authority metadata should exist");
    assert_eq!(authority, "sqlite-candidate");
}

#[test]
fn stale_candidate_is_rejected_before_authority_changes() {
    let profile = TestProfile::new("stale-candidate");
    profile.migrate();
    fs::OpenOptions::new()
        .append(true)
        .open(profile.categories_path())
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(b"2,Changed after migration,,1,1\n")
        })
        .expect("legacy source should be changed");

    let output = profile.activate();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("legacy authority changed"));
    assert_eq!(read_marker(&profile)["active_authority"], "legacy-files");
    let connection = Connection::open(profile.database_path()).expect("database should open");
    let authority: String = connection
        .query_row(
            "SELECT value FROM database_metadata WHERE key = 'storage_authority'",
            [],
            |row| row.get(0),
        )
        .expect("authority metadata should exist");
    assert_eq!(authority, "sqlite-candidate");
}

#[test]
fn interrupted_activation_is_recovered_idempotently() {
    let profile = TestProfile::new("activation-recovery");
    profile.migrate();
    let marker_path = profile.marker_path();
    let marker = fs::read_to_string(&marker_path).expect("marker should be readable");
    fs::write(
        &marker_path,
        marker.replacen(
            "\"active_authority\": \"legacy-files\"",
            "\"active_authority\": \"activating-sqlite-cli\"",
            1,
        ),
    )
    .expect("interrupted marker should be written");

    let output = profile.activate();

    assert!(
        output.status.success(),
        "activation failed: {}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("recovered-activation"));
    let marker = read_marker(&profile);
    assert_eq!(marker["active_authority"], "sqlite-cli");
    assert_eq!(marker["sqlite_cli_activation"]["status"], "active");

    let repeated = profile.activate();
    assert!(
        repeated.status.success(),
        "repeat failed: {}",
        stderr(&repeated)
    );
    assert!(stdout(&repeated).contains("already-active"));
}

#[test]
fn activated_cli_uses_sqlite_without_legacy_dual_writes() {
    let profile = TestProfile::new("cli-cutover");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let start = profile.run(&[
        "start",
        "sqlite-project",
        "--category",
        "Work",
        "--desc",
        "SQLite authority",
    ]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    profile.backdate_sqlite_active_session(2);
    assert!(!profile.legacy_active_path().exists());
    assert!(!profile.legacy_time_log_path().exists());

    {
        let connection = Connection::open(profile.database_path()).expect("database should open");
        let active: (String, String) = connection
            .query_row(
                "SELECT project, description FROM active_session WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("SQLite active session should exist");
        assert_eq!(active.0, "sqlite-project");
        assert_eq!(active.1, "SQLite authority");
    }

    let stop = profile.run(&["stop"]);
    assert!(stop.status.success(), "stop failed: {}", stderr(&stop));
    assert!(!profile.legacy_active_path().exists());
    assert!(!profile.legacy_time_log_path().exists());

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let completed: (i64, String, String) = connection
        .query_row(
            "SELECT count(*), max(project), max(source) FROM sessions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("completed SQLite session should exist");
    assert_eq!(completed.0, 1);
    assert_eq!(completed.1, "sqlite-project");
    assert_eq!(completed.2, "cli-runtime");
    drop(connection);

    let report = profile.run(&["report", "--today"]);
    assert!(
        report.status.success(),
        "report failed: {}",
        stderr(&report)
    );
    assert!(stdout(&report).contains("Work"));

    let export = profile.run(&["export", "--format", "json"]);
    assert!(
        export.status.success(),
        "export failed: {}",
        stderr(&export)
    );
    let exported: Value = serde_json::from_slice(&export.stdout).expect("export should be JSON");
    assert_eq!(exported["sessions"][0]["project"], "sqlite-project");
}

#[test]
fn activated_cli_requires_explicit_classification_and_allows_explicit_idle() {
    let profile = TestProfile::new("explicit-classification");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let omitted = profile.run(&["start", "project-only"]);
    assert!(!omitted.status.success());
    assert!(stderr(&omitted).contains("--category <CATEGORY>"));
    let connection = Connection::open(profile.database_path()).expect("database should open");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .expect("active count should be readable");
    assert_eq!(active_count, 0);
    drop(connection);

    let idle = profile.run(&["start", "rest", "--category", "idle"]);
    assert!(
        idle.status.success(),
        "idle start failed: {}",
        stderr(&idle)
    );
    assert!(stdout(&idle).contains("category 'idle'"));
    profile.backdate_sqlite_active_session(2);
    let stop = profile.run(&["stop"]);
    assert!(stop.status.success(), "idle stop failed: {}", stderr(&stop));

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let completed: (String, i64) = connection
        .query_row("SELECT project, category_id FROM sessions", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .expect("explicit idle session should persist");
    assert_eq!(completed.0, "rest");
    assert_eq!(completed.1, 0);
    drop(connection);

    let report = profile.run(&["report", "--today"]);
    assert!(
        report.status.success(),
        "report failed: {}",
        stderr(&report)
    );
    assert!(stdout(&report).contains("TOTAL"));
    assert!(stdout(&report).contains("00:00:00"));
}

#[test]
fn activated_tui_runs_and_persists_only_sqlite() {
    let profile = TestProfile::new("tui-cutover");
    let legacy_categories_before =
        fs::read(profile.categories_path()).expect("legacy categories should be readable");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let tui = profile.run_tui();
    assert!(
        tui.status.success(),
        "TUI smoke failed with status {:?}: stdout={} stderr={}",
        tui.status.code(),
        stdout(&tui),
        stderr(&tui)
    );

    assert!(!profile.legacy_active_path().exists());
    assert!(!profile.legacy_time_log_path().exists());
    assert_eq!(
        fs::read(profile.categories_path()).expect("legacy categories should remain readable"),
        legacy_categories_before
    );

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .expect("active count should be readable");
    let tui_sessions: i64 = connection
        .query_row(
            "SELECT count(*) FROM sessions WHERE source = 'tui-runtime'",
            [],
            |row| row.get(0),
        )
        .expect("TUI session count should be readable");
    let sand_state_count: i64 = connection
        .query_row("SELECT count(*) FROM sand_state", [], |row| row.get(0))
        .expect("sand state count should be readable");
    assert_eq!(
        active_count, 0,
        "normal TUI exit must complete its active interval"
    );
    assert_eq!(
        tui_sessions, 1,
        "TUI should persist exactly one completed interval"
    );
    assert_eq!(
        sand_state_count, 1,
        "TUI should persist sediment state to SQLite"
    );
}

#[test]
fn lifecycle_overlay_applies_only_the_live_revision_bound_phrase() {
    let profile = TestProfile::new("category-lifecycle-pty");
    fs::write(
        profile.categories_path(),
        "id,name,description,color_index,karma_effect\n\
         1,Work,Focused work,0,1\n\
         2,Target,Merged work,1,0\n",
    )
    .expect("two-category catalog should be written");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let tui = profile.run_tui_lifecycle_merge();
    assert!(
        tui.status.success(),
        "lifecycle TUI failed with status {:?}: stdout={} stderr={}",
        tui.status.code(),
        stdout(&tui),
        stderr(&tui)
    );
    let rendered = stdout(&tui);
    assert!(rendered.contains("DESTRUCTIVE LAYER LIFECYCLE"));
    assert!(rendered.contains("MERGE 1 INTO 2 "));

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let source_count: i64 = connection
        .query_row("SELECT count(*) FROM categories WHERE id = 1", [], |row| {
            row.get(0)
        })
        .expect("source count should be readable");
    let target: (String, String) = connection
        .query_row(
            "SELECT name, description FROM categories WHERE id = 2",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("target category should remain");
    let receipt: (String, i64, i64) = connection
        .query_row(
            "SELECT operation_kind, source_category_id, target_category_id
             FROM category_lifecycle_receipts",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("lifecycle receipt should exist");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .expect("active count should be readable");

    assert_eq!(source_count, 0, "source identity should be retired");
    assert_eq!(target, ("Target".to_string(), "Merged work".to_string()));
    assert_eq!(receipt, ("merge".to_string(), 1, 2));
    assert_eq!(
        active_count, 0,
        "normal exit must finish the active interval"
    );
}

#[test]
fn unacknowledged_cli_stop_is_recovered_without_duplicate_session() {
    let profile = TestProfile::new("stop-receipt-recovery");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let start = profile.run(&["start", "receipt-project", "--category", "Work"]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    profile.backdate_sqlite_active_session(2);
    let first_stop = profile.run(&["stop"]);
    assert!(
        first_stop.status.success(),
        "initial stop failed: {}",
        stderr(&first_stop)
    );

    {
        let connection = Connection::open(profile.database_path()).expect("database should open");
        let changed = connection
            .execute(
                "UPDATE runtime_transitions
                 SET acknowledged_at_utc = NULL
                 WHERE operation_kind = 'finish' AND source = 'cli-runtime'",
                [],
            )
            .expect("receipt acknowledgement should be cleared");
        assert_eq!(changed, 1);
    }

    let recovered = profile.run(&["stop"]);
    assert!(
        recovered.status.success(),
        "receipt recovery failed: {}",
        stderr(&recovered)
    );
    assert!(stdout(&recovered).contains("Stopped session"));

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let session_count: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .expect("session count should be readable");
    let unacknowledged: i64 = connection
        .query_row(
            "SELECT count(*) FROM runtime_transitions
             WHERE operation_kind = 'finish' AND acknowledged_at_utc IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("receipt count should be readable");
    assert_eq!(session_count, 1, "recovery must not duplicate time");
    assert_eq!(unacknowledged, 0, "recovered receipt must be acknowledged");
    drop(connection);

    let repeated = profile.run(&["stop"]);
    assert!(!repeated.status.success());
    assert!(stderr(&repeated).contains("No active session"));
}

#[test]
fn initial_tui_bootstrap_failure_leaves_no_orphan_generation() {
    for phase in ["before-write", "active", "checkpoint", "commit"] {
        let profile = TestProfile::new(&format!("initial-bootstrap-{phase}"));
        profile.migrate();
        assert!(profile.activate().status.success());
        let fault = format!("active-bootstrap:{phase}:commit");

        let tui = profile.run_tui_with_input(b"q", Some(&fault));
        assert!(
            !tui.status.success(),
            "injected {phase} failure unexpectedly succeeded"
        );

        let connection = Connection::open(profile.database_path()).expect("database should open");
        let active_count: i64 = connection
            .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
            .unwrap();
        let checkpoint_count: i64 = connection
            .query_row("SELECT count(*) FROM runtime_checkpoint", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(active_count, 0, "phase {phase} left an orphan active row");
        assert_eq!(
            checkpoint_count, 0,
            "phase {phase} left an orphan checkpoint"
        );
        drop(connection);

        let retry = profile.run_tui();
        assert!(
            retry.status.success(),
            "retry after {phase} failed: stdout={} stderr={}",
            stdout(&retry),
            stderr(&retry)
        );
    }
}

#[test]
fn failed_recovery_commit_reuses_and_displays_persisted_cutoff() {
    let profile = TestProfile::new("visible-recovery-cutoff");
    profile.migrate();
    assert!(profile.activate().status.success());

    let detached = profile.run_tui_with_input(b"d", None);
    assert!(
        detached.status.success(),
        "detach failed: stdout={} stderr={}",
        stdout(&detached),
        stderr(&detached)
    );

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let payload_json: String = connection
        .query_row(
            "SELECT payload_json FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .expect("detached checkpoint should exist");
    let payload: Value = serde_json::from_str(&payload_json).expect("payload should be JSON");
    assert!(
        payload["sand_state"]["grid_width"].as_u64().unwrap_or(0) > 0,
        "fixed-size PTY should produce an initialized checkpoint canvas"
    );
    assert!(
        payload["sand_state"]["grid_height"].as_u64().unwrap_or(0) > 0,
        "fixed-size PTY should produce an initialized checkpoint canvas"
    );
    drop(connection);

    let failed = profile.run_tui_with_input(b"", Some("checkpoint-recovery:commit:cutoff"));
    assert!(
        !failed.status.success(),
        "injected recovery commit unexpectedly succeeded"
    );

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let (status, payload_json): (String, String) = connection
        .query_row(
            "SELECT status, payload_json FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("recovering checkpoint should remain");
    assert_eq!(status, "recovering");
    let payload: Value = serde_json::from_str(&payload_json).expect("payload should be JSON");
    let persisted_target = payload["recovery_target_utc"]
        .as_str()
        .expect("recovery target should be persisted")
        .to_string();
    drop(connection);

    thread::sleep(Duration::from_millis(1_200));
    let recovered = profile.run_tui_with_input(b"\rq", None);
    assert!(
        recovered.status.success(),
        "recovery retry failed: stdout={} stderr={}",
        stdout(&recovered),
        stderr(&recovered)
    );
    let visible_target = chrono::DateTime::parse_from_rfc3339(&persisted_target)
        .unwrap()
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let output = stdout(&recovered);
    assert!(
        output.contains("RECOVERY EVIDENCE"),
        "recovery statement was not rendered: {output:?}"
    );
    assert!(output.contains("Recovery target"));
    assert!(output.contains(&visible_target));
    assert!(output.contains("PROVISIONAL LIVE TIME"));
}

fn recovery_bundle(profile: &TestProfile) -> Value {
    let files = profile.recovery_files();
    assert_eq!(files.len(), 1, "exactly one emergency export is expected");
    let metadata = fs::metadata(&files[0]).expect("emergency export metadata should exist");
    assert_eq!(
        metadata.permissions().mode() & 0o077,
        0,
        "emergency export must not be readable by group or other users"
    );
    let bytes = fs::read(&files[0]).expect("emergency export should be readable");
    serde_json::from_slice(&bytes).expect("emergency export should be valid JSON")
}

#[test]
fn tui_finish_commit_failure_exports_without_consuming_active_session() {
    let profile = TestProfile::new("finish-commit-recovery");
    profile.migrate();
    assert!(profile.activate().status.success());

    let tui = profile.run_tui_with_input(b"qq", Some("finish:commit:commit"));
    assert!(
        tui.status.success(),
        "recovery export exit failed: stdout={} stderr={}",
        stdout(&tui),
        stderr(&tui)
    );

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .unwrap();
    let session_count: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(active_count, 1, "failed commit must retain the active row");
    assert_eq!(
        session_count, 0,
        "failed commit must not create completed time"
    );
    drop(connection);

    let bundle = recovery_bundle(&profile);
    assert_eq!(bundle["failure"]["class"], "commit");
    assert!(bundle["active_session"].is_object());
}

#[test]
fn tui_busy_category_sync_exports_committed_finish_and_failure_context() {
    let profile = TestProfile::new("busy-category-recovery");
    profile.migrate();
    assert!(profile.activate().status.success());

    let tui = profile.run_tui_with_input(b"qq", Some("category-sync:before-write:busy"));
    assert!(
        tui.status.success(),
        "busy recovery exit failed: {}",
        stderr(&tui)
    );

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .unwrap();
    let session_count: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(active_count, 0);
    assert_eq!(
        session_count, 1,
        "finish must remain committed before later failure"
    );
    drop(connection);

    let bundle = recovery_bundle(&profile);
    assert_eq!(bundle["failure"]["class"], "busy");
    assert_eq!(bundle["failure"]["operation"], "category synchronization");
}

#[test]
fn tui_readonly_sand_failure_exports_in_memory_recovery_state() {
    let profile = TestProfile::new("readonly-sand-recovery");
    profile.migrate();
    assert!(profile.activate().status.success());

    let tui = profile.run_tui_with_input(b"qq", Some("sand-state:before-write:readonly"));
    assert!(
        tui.status.success(),
        "read-only recovery exit failed: {}",
        stderr(&tui)
    );
    let bundle = recovery_bundle(&profile);
    assert_eq!(bundle["failure"]["class"], "read-only");
    assert_eq!(bundle["failure"]["operation"], "sediment-state save");
    assert!(bundle["sand_state"].is_object());
}

#[test]
fn corrupt_state_load_fails_visible_without_empty_fallback() {
    let profile = TestProfile::new("corrupt-state-load");
    profile.migrate();
    assert!(profile.activate().status.success());

    let tui = profile.run_tui_with_input(b"q", Some("state-load:before-read:corrupt"));
    assert!(!tui.status.success());
    let combined = format!("{}{}", stdout(&tui), stderr(&tui));
    assert!(combined.contains("injected corrupt failure"));

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let category_count: i64 = connection
        .query_row("SELECT count(*) FROM categories", [], |row| row.get(0))
        .unwrap();
    assert!(
        category_count >= 2,
        "startup failure must not replace authority with an empty database"
    );
}

#[test]
fn post_commit_reload_retry_preserves_committed_history_before_exit() {
    let profile = TestProfile::new("post-commit-reload-retry");
    profile.migrate();
    assert!(profile.activate().status.success());

    let tui = profile.run_tui_with_input(b"qRq", Some("session-reload:before-read:busy:once"));
    assert!(
        tui.status.success(),
        "post-commit reload retry failed: stdout={} stderr={}",
        stdout(&tui),
        stderr(&tui)
    );

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .unwrap();
    let session_count: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .unwrap();
    let distinct_stable_ids: i64 = connection
        .query_row(
            "SELECT count(DISTINCT stable_id) FROM sessions",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(active_count, 0);
    assert_eq!(
        session_count, 2,
        "both committed intervals must survive reload retry"
    );
    assert_eq!(
        distinct_stable_ids, 2,
        "reload retry must not duplicate an interval"
    );
    assert!(profile.recovery_files().is_empty());
}
