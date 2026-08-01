from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


# Preserve a newly surfaced post-commit recovery state instead of rewrapping it
# as the original operation, and only continue exit automatically when safe.
path = Path("src/app/persistence_recovery.rs")
text = path.read_text()
text = text.replace(
    "use std::{fmt, fs, path::PathBuf};",
    "use std::{\n    fmt, fs,\n    fs::OpenOptions,\n    io::Write,\n    path::{Path, PathBuf},\n};",
)
text = replace_once(
    text,
    '''    pub(super) fn promote_recovery_action(&mut self, action: RecoveryAction) {
        if let Some(recovery) = self.persistence_recovery.as_mut() {
            recovery.action = action;
        }
    }
''',
    '''    pub(super) fn promote_recovery_action(&mut self, action: RecoveryAction) {
        if let Some(recovery) = self.persistence_recovery.as_mut() {
            let retryable_finish = recovery.failure.operation == PersistenceOperation::ActiveFinish
                && !matches!(
                    recovery.failure.class,
                    PersistenceFailureClass::Conflict
                        | PersistenceFailureClass::Constraint
                        | PersistenceFailureClass::Corrupt
                        | PersistenceFailureClass::InvalidData
                );
            if recovery.action == RecoveryAction::FlushCurrentState || retryable_finish {
                recovery.action = action;
            }
        }
    }
''',
    "safe exit promotion",
)
old_retry = '''        let result = match action {
            RecoveryAction::FlushCurrentState => self.try_flush_current_state(),
            RecoveryAction::ReloadAuthority => self.try_reload_authority(),
            RecoveryAction::FinishAndExit => self.try_finish_and_exit(),
            RecoveryAction::DetachAndExit => self.try_detach_and_exit(),
            RecoveryAction::CommitCheckpointRecovery => self.try_commit_checkpoint_recovery(),
        };

        match result {
'''
new_retry = '''        let result = match action {
            RecoveryAction::FlushCurrentState => self.try_flush_current_state(),
            RecoveryAction::ReloadAuthority => self.try_reload_authority(),
            RecoveryAction::FinishAndExit => self.try_finish_and_exit(),
            RecoveryAction::DetachAndExit => self.try_detach_and_exit(),
            RecoveryAction::CommitCheckpointRecovery => self.try_commit_checkpoint_recovery(),
        };

        if let Some(recovery) = self.persistence_recovery.as_mut() {
            if recovery.exported_path.is_none() {
                recovery.exported_path = exported_path;
            }
            recovery.export_error = None;
            recovery.exit_without_saving_armed = false;
            self.render_needed = true;
            return;
        }

        match result {
'''
text = replace_once(text, old_retry, new_retry, "nested recovery preservation")
text = text.replace(
    '''            if let Some(recovery) = self.persistence_recovery.take() {
                return Err(recovery.failure.summary());
            }
''',
    '''            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
''',
)
text = replace_once(
    text,
    '''        self.persist_detached_checkpoint();
        if let Some(recovery) = self.persistence_recovery.take() {
            return Err(recovery.failure.summary());
        }
''',
    '''        self.persist_detached_checkpoint();
        if let Some(recovery) = self.persistence_recovery.as_ref() {
            return Err(recovery.failure.summary());
        }
''',
    "detach nested recovery",
)
text = replace_once(
    text,
    '''        self.commit_checkpoint_recovery_if_ready();
        if let Some(recovery) = self.persistence_recovery.take() {
            return Err(recovery.failure.summary());
        }
''',
    '''        self.commit_checkpoint_recovery_if_ready();
        if let Some(recovery) = self.persistence_recovery.as_ref() {
            return Err(recovery.failure.summary());
        }
''',
    "checkpoint nested recovery",
)
text = replace_once(
    text,
    "        storage::write_json_atomic(&path, &bundle)?;\n        Ok(path)",
    "        write_private_json_atomic(&path, &bundle)?;\n        Ok(path)",
    "private emergency export",
)
insert_before = "fn classify_failure(detail: &str) -> PersistenceFailureClass {\n"
private_writer = r'''fn write_private_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    if temporary.exists() {
        fs::remove_file(&temporary).map_err(|error| error.to_string())?;
    }

    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).map_err(|error| error.to_string())?;
        file.write_all(&json).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())?;
        Ok(())
    })();

    if result.is_err() {
        fs::remove_file(&temporary).ok();
    }
    result
}

'''
text = replace_once(text, insert_before, private_writer + insert_before, "private writer insertion")
text = text.replace(
    '''    } else if normalized.contains("invalid")
        || normalized.contains("unsupported")
        || normalized.contains("outside")
''',
    '''    } else if normalized.contains("invalid")
        || normalized.contains("unsupported")
        || normalized.contains("outside")
        || normalized.contains("no active stable identity")
''',
)
path.write_text(text)


# Give missing stable IDs and checkpoint cleanup their exact operation context,
# and expose a distinct post-commit reload fault point.
path = Path("src/app.rs")
text = path.read_text()
text = replace_once(
    text,
    '''        let Some(state) = self.record_storage_result_for(
            PersistenceOperation::StateReload,
            RecoveryAction::ReloadAuthority,
            sqlite::load_tui_state(&database_path),
        ) else {
''',
    '''        let reload_result = sqlite::inject_tui_test_fault("session-reload", "before-read")
            .and_then(|()| sqlite::load_tui_state(&database_path));
        let Some(state) = self.record_storage_result_for(
            PersistenceOperation::StateReload,
            RecoveryAction::ReloadAuthority,
            reload_result,
        ) else {
''',
    "post-commit reload fault",
)
# There are three generic missing-stable paths; replace each with exact context.
text = replace_once(
    text,
    '''                self.record_storage_result::<()>(Err(
                    "SQLite runtime has no active stable identity to reset".to_string(),
                ));
''',
    '''                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveReset,
                    RecoveryAction::ReloadAuthority,
                    Err("SQLite runtime has no active stable identity to reset".to_string()),
                );
''',
    "reset stable identity context",
)
text = replace_once(
    text,
    '''                self.record_storage_result::<()>(Err(
                    "SQLite runtime has no active stable identity to finish".to_string(),
                ));
''',
    '''                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveFinish,
                    RecoveryAction::ReloadAuthority,
                    Err("SQLite runtime has no active stable identity to finish".to_string()),
                );
''',
    "finish stable identity context",
)
text = replace_once(
    text,
    '''                self.record_storage_result::<()>(Err(
                    "SQLite runtime has no active stable identity to switch".to_string(),
                ));
''',
    '''                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveSwitch,
                    RecoveryAction::ReloadAuthority,
                    Err("SQLite runtime has no active stable identity to switch".to_string()),
                );
''',
    "switch stable identity context",
)
text = replace_once(
    text,
    '''        if self
            .record_storage_result(sqlite::clear_tui_checkpoint(&database_path))
            .is_some()
''',
    '''        if self
            .record_storage_result_for(
                PersistenceOperation::CheckpointClear,
                RecoveryAction::FlushCurrentState,
                sqlite::clear_tui_checkpoint(&database_path),
            )
            .is_some()
''',
    "checkpoint cleanup context",
)
path.write_text(text)


# A crate-internal façade keeps test fault mechanics out of application modules.
path = Path("src/sqlite.rs")
text = path.read_text()
anchor = '''pub(crate) fn run_controlled_migration(
    options: ControlledMigrationOptions,
) -> Result<ControlledMigrationReport, String> {
'''
facade = '''pub(crate) fn inject_tui_test_fault(operation: &str, phase: &str) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault(operation, phase)
        .map_err(|error| error.to_string())
}

'''
text = replace_once(text, anchor, facade + anchor, "test fault façade")
path.write_text(text)


# Make a fault optionally one-shot so the real TUI retry path can be certified.
path = Path("src/sqlite/runtime_coordination.rs")
text = path.read_text()
text = text.replace(
    "use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};",
    '''use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

#[cfg(debug_assertions)]
use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};''',
)
old_inject = '''        if let Ok(specification) = std::env::var("STRATA_TEST_SQLITE_FAULT") {
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
'''
new_inject = '''        if let Ok(specification) = std::env::var("STRATA_TEST_SQLITE_FAULT") {
            let mut parts = specification.splitn(4, ':');
            let requested_operation = parts.next().unwrap_or_default();
            let requested_phase = parts.next().unwrap_or_default();
            let class = parts.next().unwrap_or("unknown");
            let mode = parts.next().unwrap_or("always");
            if requested_operation == operation && requested_phase == phase {
                if mode == "once" {
                    static FIRED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
                    let key = format!("{operation}:{phase}:{class}");
                    let mut fired = FIRED
                        .get_or_init(|| Mutex::new(HashSet::new()))
                        .lock()
                        .map_err(|_| CoordinationError::InvalidInput(
                            "test fault registry is poisoned".to_string(),
                        ))?;
                    if !fired.insert(key) {
                        return Ok(());
                    }
                }
                return Err(CoordinationError::InjectedFailure {
                    operation: operation.to_string(),
                    phase: phase.to_string(),
                    class: class.to_string(),
                });
            }
        }
'''
text = replace_once(text, old_inject, new_inject, "one-shot fault injection")
# Certify checkpoint-save retry for the same active identity.
insert_test_before = '''    #[test]
    fn checkpoint_commit_is_atomic_and_recovering_is_reclaimable() {
'''
checkpoint_test = r'''    #[test]
    fn pending_checkpoint_retry_replaces_payload_for_same_active_identity() {
        let path = database_path("checkpoint-save-retry");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T11:00:00Z",
            "2026-08-01T10:59:00Z",
            "{\"attempt\":1}",
        )
        .unwrap();
        save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T11:00:01Z",
            "2026-08-01T11:00:00Z",
            "{\"attempt\":2}",
        )
        .unwrap();
        let claimed = claim_checkpoint(&mut repository).unwrap().unwrap();
        assert_eq!(claimed.active_session_stable_id.as_deref(), Some("active-a"));
        assert_eq!(claimed.payload_json, "{\"attempt\":2}");
        drop(repository);
        remove_database(&path);
    }

'''
text = replace_once(text, insert_test_before, checkpoint_test + insert_test_before, "checkpoint retry test")
path.write_text(text)


# Certify post-commit reload retry and private emergency-export custody.
path = Path("tests/sqlite_cli_authority.rs")
text = path.read_text()
text = text.replace(
    "    path::PathBuf,",
    "    os::unix::fs::PermissionsExt,\n    path::PathBuf,",
)
text = replace_once(
    text,
    '''    let bytes = fs::read(&files[0]).expect("emergency export should be readable");
    serde_json::from_slice(&bytes).expect("emergency export should be valid JSON")
''',
    '''    let metadata = fs::metadata(&files[0]).expect("emergency export metadata should exist");
    assert_eq!(
        metadata.permissions().mode() & 0o077,
        0,
        "emergency export must not be readable by group or other users"
    );
    let bytes = fs::read(&files[0]).expect("emergency export should be readable");
    serde_json::from_slice(&bytes).expect("emergency export should be valid JSON")
''',
    "private export assertion",
)
append = r'''

#[test]
fn post_commit_reload_retry_preserves_committed_history_before_exit() {
    let profile = TestProfile::new("post-commit-reload-retry");
    profile.migrate();
    assert!(profile.activate().status.success());

    let tui = profile.run_tui_with_input(
        b"qRq",
        Some("session-reload:before-read:busy:once"),
    );
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
        .query_row("SELECT count(DISTINCT stable_id) FROM sessions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(active_count, 0);
    assert_eq!(session_count, 2, "both committed intervals must survive reload retry");
    assert_eq!(distinct_stable_ids, 2, "reload retry must not duplicate an interval");
    assert!(profile.recovery_files().is_empty());
}
'''
text += append
path.write_text(text)
