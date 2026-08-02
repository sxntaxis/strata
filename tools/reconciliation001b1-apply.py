from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:160]!r}")
    target.write_text(text.replace(old, new, 1))


# -------------------------------------------------------------------------
# SQLite runtime transitions retire checkpoint generations atomically.
# -------------------------------------------------------------------------
path = Path("src/sqlite/runtime_coordination.rs")
text = path.read_text()
helper_anchor = "fn query_active(connection: &Connection)"
helper = r'''fn retire_checkpoint_for_active_transition(
    transaction: &Transaction<'_>,
    expected_active_stable_id: &str,
) -> Result<(), CoordinationError> {
    let checkpoint: Option<(String, Option<String>)> = transaction
        .query_row(
            "SELECT status, active_session_stable_id
             FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((status, active_session_stable_id)) = checkpoint else {
        return Ok(());
    };

    let replaceable = matches!(status.as_str(), "pending" | "committed")
        && active_session_stable_id.as_deref() == Some(expected_active_stable_id);
    if !replaceable {
        let actual_identity = active_session_stable_id
            .as_deref()
            .unwrap_or("no active identity");
        return Err(CoordinationError::CheckpointConflict {
            expected: format!(
                "no checkpoint or pending/committed checkpoint for active session {expected_active_stable_id}"
            ),
            actual: format!("{status} for {actual_identity}"),
        });
    }

    transaction.execute(
        "DELETE FROM runtime_checkpoint
         WHERE singleton = 1 AND status IN ('pending', 'committed')
           AND active_session_stable_id = ?1",
        params![expected_active_stable_id],
    )?;
    Ok(())
}

'''
if helper_anchor not in text:
    raise SystemExit("runtime checkpoint helper anchor not found")
text = text.replace(helper_anchor, helper + helper_anchor, 1)

for function_name in [
    "finish_active_session",
    "switch_active_session",
    "reset_active_session",
]:
    start = text.index(f"pub(crate) fn {function_name}")
    marker = "    require_expected_active(&active, expected_active_stable_id)?;\n"
    position = text.index(marker, start) + len(marker)
    text = (
        text[:position]
        + "    retire_checkpoint_for_active_transition(&transaction, expected_active_stable_id)?;\n"
        + text[position:]
    )

proof_marker = '''    #[test]
    fn pending_checkpoint_retry_replaces_payload_for_same_active_identity() {
'''
proofs = r'''    #[test]
    fn active_transitions_atomically_retire_prior_checkpoint_generation() {
        let path = database_path("transition-checkpoint-retirement");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T10:30:00Z",
            "2026-08-01T10:29:59Z",
            "{\"active\":\"a\"}",
        )
        .unwrap();
        switch_active_session(
            &mut repository,
            "active-a",
            "switch:a:b",
            &completion("tui-runtime"),
            &NewActiveSession {
                stable_id: "active-b",
                project: "",
                category_id: 1,
                description: "next",
                started_at_utc: "2026-08-01T11:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        assert!(repository.checkpoint().unwrap().is_none());

        save_checkpoint(
            &mut repository,
            "active-b",
            "2026-08-01T11:30:00Z",
            "2026-08-01T11:29:59Z",
            "{\"active\":\"b\"}",
        )
        .unwrap();
        reset_active_session(
            &mut repository,
            "active-b",
            "reset:b:c",
            &NewActiveSession {
                stable_id: "active-c",
                project: "",
                category_id: 1,
                description: "reset",
                started_at_utc: "2026-08-01T12:00:00Z",
                recovery_kind: "live",
            },
            "2026-08-01T12:00:00Z",
            "tui-runtime",
        )
        .unwrap();
        assert!(repository.checkpoint().unwrap().is_none());

        save_checkpoint(
            &mut repository,
            "active-c",
            "2026-08-01T12:30:00Z",
            "2026-08-01T12:29:59Z",
            "{\"active\":\"c\"}",
        )
        .unwrap();
        finish_active_session(
            &mut repository,
            "active-c",
            "finish:active-c",
            &SessionCompletion {
                ended_at_utc: "2026-08-01T13:00:00Z",
                operational_day: "2026-08-01",
                elapsed_seconds: 3600,
                boundary_utc_offset_seconds: -21600,
                boundary_start_minutes: 360,
                source: "tui-runtime",
            },
            true,
        )
        .unwrap();
        assert!(repository.checkpoint().unwrap().is_none());
        assert!(repository.active_session().unwrap().is_none());
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn recovering_or_mismatched_checkpoint_blocks_active_transition() {
        let path = database_path("transition-checkpoint-conflict");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T10:30:00Z",
            "2026-08-01T10:29:59Z",
            "{}",
        )
        .unwrap();
        claim_checkpoint(&mut repository).unwrap().unwrap();
        let error = switch_active_session(
            &mut repository,
            "active-a",
            "switch:blocked",
            &completion("tui-runtime"),
            &NewActiveSession {
                stable_id: "active-b",
                project: "",
                category_id: 1,
                description: "",
                started_at_utc: "2026-08-01T11:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CoordinationError::CheckpointConflict { ref actual, .. }
                if actual.contains("recovering")
        ));
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "active-a"
        );
        assert!(repository.list_sessions().unwrap().is_empty());

        repository
            .connection
            .execute(
                "UPDATE runtime_checkpoint
                 SET status = 'pending', active_session_stable_id = 'other-active'
                 WHERE singleton = 1",
                [],
            )
            .unwrap();
        let error = finish_active_session(
            &mut repository,
            "active-a",
            "finish:blocked",
            &completion("tui-runtime"),
            true,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CoordinationError::CheckpointConflict { ref actual, .. }
                if actual.contains("other-active")
        ));
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "active-a"
        );
        assert!(repository.list_sessions().unwrap().is_empty());
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn pending_checkpoint_retry_replaces_payload_for_same_active_identity() {
'''
if proof_marker not in text:
    raise SystemExit("runtime coordination proof marker not found")
text = text.replace(proof_marker, proofs, 1)
path.write_text(text)

# -------------------------------------------------------------------------
# Startup rejects claimed checkpoint identity that differs from active row.
# -------------------------------------------------------------------------
path = Path("src/sqlite/tui_runtime.rs")
text = path.read_text()
old_claim = '''    let Some(claimed) = runtime_coordination::claim_checkpoint(&mut repository)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    match serde_json::from_str(&claimed.payload_json) {
'''
new_claim = '''    let Some(claimed) = runtime_coordination::claim_checkpoint(&mut repository)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let claimed_stable_id = claimed
        .active_session_stable_id
        .as_deref()
        .ok_or_else(|| "Runtime checkpoint has no active stable identity".to_string())?;
    let authoritative_active = repository
        .active_session()
        .map_err(|error| error.to_string())?;
    if authoritative_active.as_ref().map(|active| active.stable_id.as_str())
        != Some(claimed_stable_id)
    {
        runtime_coordination::quarantine_checkpoint(&mut repository)
            .map_err(|error| error.to_string())?;
        let actual = authoritative_active
            .as_ref()
            .map(|active| active.stable_id.as_str())
            .unwrap_or("no active session");
        return Err(format!(
            "Runtime checkpoint active identity {claimed_stable_id} does not match authoritative active session {actual}; evidence quarantined"
        ));
    }
    match serde_json::from_str(&claimed.payload_json) {
'''
if text.count(old_claim) != 1:
    raise SystemExit("TUI checkpoint claim block not found")
text = text.replace(old_claim, new_claim, 1)

text += r'''

#[cfg(test)]
mod checkpoint_identity_tests {
    use std::path::{Path, PathBuf};

    use serde_json::Value;

    use super::*;
    use crate::sqlite::{
        NewActiveSession, SqliteRepository,
        repository::NewCategoryRecord,
        runtime_coordination,
    };

    fn database_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-checkpoint-identity-{}-{}.sqlite3",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn remove_database(path: &Path) {
        std::fs::remove_file(path).ok();
        std::fs::remove_file(format!("{}-wal", path.display())).ok();
        std::fs::remove_file(format!("{}-shm", path.display())).ok();
    }

    #[test]
    fn startup_quarantines_checkpoint_for_replaced_active_identity() {
        let path = database_path();
        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
        runtime_coordination::start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id: "active-a",
                project: "",
                category_id: 1,
                description: "",
                started_at_utc: "2026-08-01T10:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        runtime_coordination::save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T10:01:00Z",
            "2026-08-01T10:00:59Z",
            "{}",
        )
        .unwrap();
        repository
            .connection
            .execute(
                "UPDATE active_session SET stable_id = 'active-b' WHERE singleton = 1",
                [],
            )
            .unwrap();
        drop(repository);

        let error = load_checkpoint::<Value>(&path).unwrap_err();
        assert!(error.contains("does not match authoritative active session active-b"));
        let repository = SqliteRepository::open(&path).unwrap();
        let status: String = repository
            .connection
            .query_row(
                "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "quarantined");
        drop(repository);
        remove_database(&path);
    }
}
'''
path.write_text(text)

# -------------------------------------------------------------------------
# Application refreshes the active checkpoint at every metadata/transition edge.
# -------------------------------------------------------------------------
path = Path("src/app.rs")
text = path.read_text()
old_checkpoint_method = '''    fn persist_runtime_checkpoint(&mut self) {
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
'''
new_checkpoint_method = '''    fn persist_runtime_checkpoint(&mut self) {
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

    fn refresh_active_runtime_checkpoint(&mut self) {
        if self.session.active_session_started_at_utc.is_some()
            && !self.has_persistence_recovery()
        {
            self.persist_runtime_checkpoint();
        }
    }
'''
if text.count(old_checkpoint_method) != 1:
    raise SystemExit("runtime checkpoint method not found")
text = text.replace(old_checkpoint_method, new_checkpoint_method, 1)

old_reset_end = '''        if let Err(error) = self.begin_active_session_at(started_at_utc, accept_large_wall_interval)
        {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
        }
    }
'''
new_reset_end = '''        if let Err(error) = self.begin_active_session_at(started_at_utc, accept_large_wall_interval)
        {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }
        self.refresh_active_runtime_checkpoint();
    }
'''
if text.count(old_reset_end) != 1:
    raise SystemExit("active reset completion block not found")
text = text.replace(old_reset_end, new_reset_end, 1)

sqlite_switch_return = '''            self.persist_categories();
            self.sync_drift_idle_state();
            return true;
'''
new_sqlite_switch_return = '''            self.persist_categories();
            self.sync_drift_idle_state();
            self.refresh_active_runtime_checkpoint();
            return !self.has_persistence_recovery();
'''
if text.count(sqlite_switch_return) != 1:
    raise SystemExit("SQLite switch completion block not found")
text = text.replace(sqlite_switch_return, new_sqlite_switch_return, 1)

legacy_switch_return = '''        self.sync_drift_idle_state();

        true
    }
'''
new_legacy_switch_return = '''        self.sync_drift_idle_state();
        self.refresh_active_runtime_checkpoint();

        !self.has_persistence_recovery()
    }
'''
if text.count(legacy_switch_return) != 1:
    raise SystemExit("legacy switch completion block not found")
text = text.replace(legacy_switch_return, new_legacy_switch_return, 1)
path.write_text(text)

path = Path("src/app/category_state.rs")
text = path.read_text()
old_persist_end = '''        } else {
            let path = storage::get_categories_path();
            if let Err(error) =
                storage::save_category_catalog_to_csv(&path, &categories, &self.archived_categories)
            {
                self.record_storage_result::<()>(Err(error));
            }
        }
    }
'''
new_persist_end = '''        } else {
            let path = storage::get_categories_path();
            if let Err(error) =
                storage::save_category_catalog_to_csv(&path, &categories, &self.archived_categories)
            {
                self.record_storage_result::<()>(Err(error));
            }
        }
        self.refresh_active_runtime_checkpoint();
    }
'''
if text.count(old_persist_end) != 1:
    raise SystemExit("category persistence completion block not found")
text = text.replace(old_persist_end, new_persist_end, 1)
path.write_text(text)

for temporary in [
    ".github/workflows/reconciliation001b1-apply.yml",
    "tools/reconciliation001b1-apply.py",
]:
    Path(temporary).unlink(missing_ok=True)
