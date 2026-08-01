from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


path = Path("src/sqlite/runtime_coordination.rs")
text = path.read_text()
text = replace_once(
    text,
    "    #[error(\"runtime checkpoint is {actual}; expected {expected}\")]\n    CheckpointConflict { expected: String, actual: String },\n}",
    "    #[error(\"runtime checkpoint is {actual}; expected {expected}\")]\n    CheckpointConflict { expected: String, actual: String },\n    #[error(\"injected {class} failure during {operation} {phase}\")]\n    InjectedFailure {\n        operation: String,\n        phase: String,\n        class: String,\n    },\n}",
    "injected coordination error",
)
# Transaction-level pre-commit failure points.
text = replace_once(
    text,
    "    insert_active(&transaction, active)?;\n    transaction.commit()?;",
    "    insert_active(&transaction, active)?;\n    maybe_inject_test_fault(\"active-start\", \"commit\")?;\n    transaction.commit()?;",
    "active start commit fault",
)
text = replace_once(
    text,
    "        acknowledged_at_utc,\n    )?;\n    transaction.commit()?;\n    Ok(RuntimeTransitionReceipt {\n        operation_id: operation_id.to_string(),\n        operation_kind: \"finish\".to_string(),",
    "        acknowledged_at_utc,\n    )?;\n    maybe_inject_test_fault(\"finish\", \"commit\")?;\n    transaction.commit()?;\n    Ok(RuntimeTransitionReceipt {\n        operation_id: operation_id.to_string(),\n        operation_kind: \"finish\".to_string(),",
    "finish commit fault",
)
text = replace_once(
    text,
    "        Some(completion.ended_at_utc),\n    )?;\n    transaction.commit()?;\n    Ok(RuntimeTransitionReceipt {\n        operation_id: operation_id.to_string(),\n        operation_kind: \"switch\".to_string(),",
    "        Some(completion.ended_at_utc),\n    )?;\n    maybe_inject_test_fault(\"switch\", \"commit\")?;\n    transaction.commit()?;\n    Ok(RuntimeTransitionReceipt {\n        operation_id: operation_id.to_string(),\n        operation_kind: \"switch\".to_string(),",
    "switch commit fault",
)
text = replace_once(
    text,
    "        Some(applied_at_utc),\n    )?;\n    transaction.commit()?;\n    Ok(RuntimeTransitionReceipt {\n        operation_id: operation_id.to_string(),\n        operation_kind: \"reset\".to_string(),",
    "        Some(applied_at_utc),\n    )?;\n    maybe_inject_test_fault(\"reset\", \"commit\")?;\n    transaction.commit()?;\n    Ok(RuntimeTransitionReceipt {\n        operation_id: operation_id.to_string(),\n        operation_kind: \"reset\".to_string(),",
    "reset commit fault",
)
# Make checkpoint-save retry replace the same active session's pending checkpoint.
old_checkpoint_guard = '''    let existing_status: Option<String> = transaction
        .query_row(
            "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(status) = existing_status
        && status != "committed"
    {
        return Err(CoordinationError::CheckpointConflict {
            expected: "no checkpoint or committed".to_string(),
            actual: status,
        });
    }
'''
new_checkpoint_guard = '''    let existing: Option<(String, Option<String>)> = transaction
        .query_row(
            "SELECT status, active_session_stable_id
             FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((status, existing_active)) = existing {
        let replaceable_pending = status == "pending"
            && existing_active.as_deref() == Some(expected_active_stable_id);
        if status != "committed" && !replaceable_pending {
            return Err(CoordinationError::CheckpointConflict {
                expected: "no checkpoint, committed, or pending for the same active session"
                    .to_string(),
                actual: status,
            });
        }
    }
'''
text = replace_once(text, old_checkpoint_guard, new_checkpoint_guard, "checkpoint retry guard")
text = replace_once(
    text,
    "        ],\n    )?;\n    transaction.commit()?;\n    Ok(())\n}\n\npub(crate) fn claim_checkpoint",
    "        ],\n    )?;\n    maybe_inject_test_fault(\"checkpoint-save\", \"commit\")?;\n    transaction.commit()?;\n    Ok(())\n}\n\npub(crate) fn claim_checkpoint",
    "checkpoint save commit fault",
)
text = replace_once(
    text,
    "    if changed != 1 {\n        return Err(CoordinationError::CheckpointConflict {\n            expected: \"recovering\".to_string(),\n            actual: \"changed concurrently\".to_string(),\n        });\n    }\n    transaction.commit()?;\n    Ok(())\n}\n\npub(crate) fn clear_committed_checkpoint",
    "    if changed != 1 {\n        return Err(CoordinationError::CheckpointConflict {\n            expected: \"recovering\".to_string(),\n            actual: \"changed concurrently\".to_string(),\n        });\n    }\n    maybe_inject_test_fault(\"checkpoint-recovery\", \"commit\")?;\n    transaction.commit()?;\n    Ok(())\n}\n\npub(crate) fn clear_committed_checkpoint",
    "checkpoint recovery commit fault",
)
# Debug-only subprocess fault injector. Release builds ignore the test variable.
insert_before = "fn validate_transition_inputs(\n"
fault_helper = r'''pub(crate) fn maybe_inject_test_fault(
    operation: &str,
    phase: &str,
) -> Result<(), CoordinationError> {
    #[cfg(debug_assertions)]
    {
        if let Ok(specification) = std::env::var("STRATA_TEST_SQLITE_FAULT") {
            let mut parts = specification.splitn(3, ':');
            let requested_operation = parts.next().unwrap_or_default();
            let requested_phase = parts.next().unwrap_or_default();
            let class = parts.next().unwrap_or("unknown");
            if requested_operation == operation && requested_phase == phase {
                return Err(CoordinationError::InjectedFailure {
                    operation: operation.to_string(),
                    phase: phase.to_string(),
                    class: class.to_string(),
                });
            }
        }
    }
    Ok(())
}

'''
text = replace_once(text, insert_before, fault_helper + insert_before, "fault helper insertion")
path.write_text(text)


# Add representative operation-level failure points around TUI repository access.
path = Path("src/sqlite/tui_runtime.rs")
text = path.read_text()
text = replace_once(
    text,
    "pub(crate) fn load_state(database_path: &Path) -> Result<SqliteTuiState, String> {\n    let repository = open_cli_repository(database_path)?;",
    "pub(crate) fn load_state(database_path: &Path) -> Result<SqliteTuiState, String> {\n    runtime_coordination::maybe_inject_test_fault(\"state-load\", \"before-read\")\n        .map_err(|error| error.to_string())?;\n    let repository = open_cli_repository(database_path)?;",
    "state-load fault",
)
text = replace_once(
    text,
    "pub(crate) fn sync_categories(\n    database_path: &Path,\n    categories: &[Category],\n    active_category_id: CategoryId,\n    expected_active_stable_id: Option<&str>,\n) -> Result<Vec<Category>, String> {\n    let mut repository = open_cli_repository(database_path)?;",
    "pub(crate) fn sync_categories(\n    database_path: &Path,\n    categories: &[Category],\n    active_category_id: CategoryId,\n    expected_active_stable_id: Option<&str>,\n) -> Result<Vec<Category>, String> {\n    runtime_coordination::maybe_inject_test_fault(\"category-sync\", \"before-write\")\n        .map_err(|error| error.to_string())?;\n    let mut repository = open_cli_repository(database_path)?;",
    "category sync fault",
)
text = replace_once(
    text,
    "pub(crate) fn update_session_description(\n    database_path: &Path,\n    session_id: usize,\n    description: &str,\n) -> Result<(), String> {\n    let repository = open_cli_repository(database_path)?;",
    "pub(crate) fn update_session_description(\n    database_path: &Path,\n    session_id: usize,\n    description: &str,\n) -> Result<(), String> {\n    runtime_coordination::maybe_inject_test_fault(\"session-edit\", \"before-write\")\n        .map_err(|error| error.to_string())?;\n    let repository = open_cli_repository(database_path)?;",
    "session edit fault",
)
text = replace_once(
    text,
    "pub(crate) fn delete_session(database_path: &Path, session_id: usize) -> Result<(), String> {\n    let repository = open_cli_repository(database_path)?;",
    "pub(crate) fn delete_session(database_path: &Path, session_id: usize) -> Result<(), String> {\n    runtime_coordination::maybe_inject_test_fault(\"session-delete\", \"before-write\")\n        .map_err(|error| error.to_string())?;\n    let repository = open_cli_repository(database_path)?;",
    "session delete fault",
)
text = replace_once(
    text,
    "pub(crate) fn save_sand_state(database_path: &Path, state: &SandState) -> Result<(), String> {\n    let mut repository = open_cli_repository(database_path)?;",
    "pub(crate) fn save_sand_state(database_path: &Path, state: &SandState) -> Result<(), String> {\n    runtime_coordination::maybe_inject_test_fault(\"sand-state\", \"before-write\")\n        .map_err(|error| error.to_string())?;\n    let mut repository = open_cli_repository(database_path)?;",
    "sand state fault",
)
text = replace_once(
    text,
    "pub(crate) fn save_daily_snapshot(\n    database_path: &Path,\n    operational_day: &str,\n    state: &SandState,\n) -> Result<(), String> {\n    let mut repository = open_cli_repository(database_path)?;",
    "pub(crate) fn save_daily_snapshot(\n    database_path: &Path,\n    operational_day: &str,\n    state: &SandState,\n) -> Result<(), String> {\n    runtime_coordination::maybe_inject_test_fault(\"daily-snapshot\", \"before-write\")\n        .map_err(|error| error.to_string())?;\n    let mut repository = open_cli_repository(database_path)?;",
    "daily snapshot fault",
)
path.write_text(text)


# Pseudo-terminal proofs for rollback, busy/read-only recovery exports, and corrupt startup.
path = Path("tests/sqlite_cli_authority.rs")
text = path.read_text()
old_run_tui = '''    fn run_tui(&self) -> Output {
        let mut child = Command::new("timeout");
        child
            .args([
                "10s",
                "script",
                "-qefc",
                env!("CARGO_BIN_EXE_strata"),
                "/dev/null",
            ])
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = child.spawn().expect("pseudo-terminal TUI should start");
        child
            .stdin
            .take()
            .expect("TUI stdin should exist")
            .write_all(b"q")
            .expect("quit key should be written");
        child.wait_with_output().expect("TUI process should finish")
    }
'''
new_run_tui = '''    fn run_tui(&self) -> Output {
        self.run_tui_with_input(b"q", None)
    }

    fn run_tui_with_input(&self, input: &[u8], fault: Option<&str>) -> Output {
        let mut command = Command::new("timeout");
        command
            .args([
                "10s",
                "script",
                "-qefc",
                env!("CARGO_BIN_EXE_strata"),
                "/dev/null",
            ])
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
        child
            .stdin
            .take()
            .expect("TUI stdin should exist")
            .write_all(input)
            .expect("TUI input should be written");
        child.wait_with_output().expect("TUI process should finish")
    }

    fn recovery_files(&self) -> Vec<PathBuf> {
        let directory = self.state_home.join("strata/recovery");
        let mut files = fs::read_dir(directory)
            .map(|entries| entries.filter_map(Result::ok).map(|entry| entry.path()).collect())
            .unwrap_or_default();
        files.sort();
        files
    }
'''
text = replace_once(text, old_run_tui, new_run_tui, "TUI fault runner")
append = r'''

fn recovery_bundle(profile: &TestProfile) -> Value {
    let files = profile.recovery_files();
    assert_eq!(files.len(), 1, "exactly one emergency export is expected");
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
    assert_eq!(session_count, 0, "failed commit must not create completed time");
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
    assert!(tui.status.success(), "busy recovery exit failed: {}", stderr(&tui));

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .unwrap();
    let session_count: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(active_count, 0);
    assert_eq!(session_count, 1, "finish must remain committed before later failure");
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
    assert!(category_count >= 2, "startup failure must not replace authority with an empty database");
}
'''
text += append
path.write_text(text)
