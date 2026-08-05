from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


def sub_once(text: str, pattern: str, replacement: str, label: str) -> str:
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return result


Path("src/runtime_receipts.rs").write_text('''use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    domain::DRIFT_CATEGORY_ID,
    temporal,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ActiveIntervalReceipt {
    pub category_id: u64,
    pub description: String,
    pub started_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ClearAllReceipt {
    pub operation_id: String,
    pub applied_at_utc: DateTime<Utc>,
    pub previous_active: ActiveIntervalReceipt,
    pub resulting_active: ActiveIntervalReceipt,
    pub idle_reset: bool,
    pub previous_elapsed_seconds: usize,
    pub affected_operational_days: Vec<String>,
}

impl ClearAllReceipt {
    pub(crate) fn validate_boundaries(&self) -> Result<(), String> {
        if self.previous_active.category_id != self.resulting_active.category_id
            || self.previous_active.description != self.resulting_active.description
        {
            return Err(format!(
                "clear-all receipt {} changes active classification",
                self.operation_id
            ));
        }
        if self.applied_at_utc < self.previous_active.started_at_utc {
            return Err(format!(
                "clear-all receipt {} predates its active generation",
                self.operation_id
            ));
        }
        let wall_seconds = u64::try_from(
            (self.applied_at_utc - self.previous_active.started_at_utc).num_seconds(),
        )
        .map_err(|_| format!("clear-all receipt {} has an invalid wall interval", self.operation_id))?;
        let elapsed_seconds = u64::try_from(self.previous_elapsed_seconds).map_err(|_| {
            format!(
                "clear-all receipt {} elapsed value exceeds the supported range",
                self.operation_id
            )
        })?;
        if wall_seconds.abs_diff(elapsed_seconds) > temporal::MAX_LIVE_CLOCK_SKEW.as_secs() {
            return Err(format!(
                "clear-all receipt {} elapsed payload diverges from its UTC interval",
                self.operation_id
            ));
        }
        if self.idle_reset {
            if self.previous_active.category_id != DRIFT_CATEGORY_ID.0 {
                return Err(format!(
                    "clear-all receipt {} resets a non-idle active generation",
                    self.operation_id
                ));
            }
            if self.resulting_active.started_at_utc != self.applied_at_utc {
                return Err(format!(
                    "clear-all receipt {} has inconsistent idle reset time",
                    self.operation_id
                ));
            }
        } else {
            if self.previous_active.category_id == DRIFT_CATEGORY_ID.0 {
                return Err(format!(
                    "clear-all receipt {} leaves an idle generation unreset",
                    self.operation_id
                ));
            }
            if self.resulting_active.started_at_utc != self.previous_active.started_at_utc {
                return Err(format!(
                    "clear-all receipt {} changes a non-idle active start",
                    self.operation_id
                ));
            }
        }
        if self.affected_operational_days.is_empty() {
            return Err(format!(
                "clear-all receipt {} has no affected operational day",
                self.operation_id
            ));
        }
        let mut previous = None;
        for value in &self.affected_operational_days {
            let day = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|error| {
                format!(
                    "clear-all receipt {} has invalid operational day '{}': {error}",
                    self.operation_id, value
                )
            })?;
            if previous.is_some_and(|prior| prior >= day) {
                return Err(format!(
                    "clear-all receipt {} operational days are not unique and sorted",
                    self.operation_id
                ));
            }
            previous = Some(day);
        }
        Ok(())
    }
}
''')

path = Path("src/lib.rs")
text = path.read_text()
text = text.replace("#[allow(dead_code)]\nmod legacy_category_lifecycle;\nmod legacy_transition;\n", "")
text = text.replace("mod profile;\n", "mod profile;\nmod runtime_receipts;\n")
path.write_text(text)

for obsolete in [
    "src/legacy_category_lifecycle.rs",
    "src/legacy_transition.rs",
    "src/sqlite/legacy_disposition.rs",
    "src/sqlite/legacy_import.rs",
    "src/sqlite/migration_command.rs",
    "src/sqlite/closure_tests.rs",
]:
    Path(obsolete).unlink(missing_ok=True)

path = Path("src/sqlite.rs")
text = path.read_text()
for line in [
    "#[cfg(test)]\nmod closure_tests;\n",
    "mod legacy_disposition;\n",
    "mod legacy_import;\n",
    "mod migration_command;\n",
]:
    text = text.replace(line, "")
text = sub_once(
    text,
    r"pub\(crate\) use legacy_disposition::\{.*?\};\n",
    "",
    "legacy disposition exports",
)
text = text.replace("pub(crate) use migration_command::{ControlledMigrationOptions, ControlledMigrationReport};\n", "")
text = sub_once(
    text,
    r"pub\(crate\) fn run_controlled_migration\(.*?\n\}\n\n",
    "",
    "migration wrapper",
)
text = sub_once(
    text,
    r"pub\(crate\) fn run_legacy_evidence_inventory\(.*?\n\}\n\n.*?pub\(crate\) fn run_legacy_evidence_remove\(.*?\n\}\n\n",
    "",
    "legacy disposition wrappers",
)
path.write_text(text)

path = Path("src/app.rs")
text = path.read_text()
text = sub_once(
    text,
    r"    legacy_transition::\{.*?\},\n",
    "    runtime_receipts::{ActiveIntervalReceipt, ClearAllReceipt},\n",
    "app receipt imports",
)
text = text.replace("LegacyActiveReceipt", "ActiveIntervalReceipt")
text = sub_once(
    text,
    r"    #\[serde\(default\)\]\n    legacy_recovery_committed: bool,\n    #\[serde\(default\)\]\n    legacy_transition: Option<LegacyTransitionReceipt>,\n    #\[serde\(default\)\]\n    legacy_finish: Option<LegacyFinishReceipt>,\n",
    "",
    "checkpoint legacy fields",
)
text = replace_once(
    text,
    '''impl DetachedRuntimeCheckpoint {
    const VERSION: u8 = 3;
    const PREVIOUS_VERSION: u8 = 2;
    const LEGACY_VERSION: u8 = 1;
}''',
    '''impl DetachedRuntimeCheckpoint {
    const VERSION: u8 = 1;
}''',
    "checkpoint version",
)
text = sub_once(
    text,
    r"fn validate_legacy_switch_checkpoint\(.*?\nfn sand_state_is_empty",
    "fn sand_state_is_empty",
    "legacy replay helpers",
)
text = sub_once(
    text,
    r"    if checkpoint\.legacy_transition\.is_some\(\) \|\| checkpoint\.legacy_finish\.is_some\(\) \{.*?    \}\n",
    "",
    "overlapping legacy receipt check",
)
text = sub_once(
    text,
    r"    fn prepare_active_finish_for_exit\(&mut self\) -> Option<usize> \{.*?\n    fn simulation_backlog_duration_at",
    '''    fn prepare_active_finish_for_exit(&mut self) -> Option<usize> {
        let finished_at_utc = Utc::now();
        if let Err(error) = self.settle_transition_boundary(finished_at_utc) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::FinishAndExit,
                Err(error),
            );
            return None;
        }
        self.end_active_session_at(finished_at_utc, SessionClockMode::LiveMonotonic)
    }

    fn end_active_session_at(
        &mut self,
        observed_end_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Option<usize> {
        let interval = match self.reconciled_active_interval(observed_end_utc, clock_mode) {
            Ok(interval) => interval,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveFinish,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return None;
            }
        };
        let elapsed = interval.elapsed_seconds;
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::ReloadAuthority,
                Err("SQLite runtime has no active stable identity to finish".to_string()),
            );
            return None;
        };
        let database_path = self.sqlite_database_path.clone()?;
        let operational_day = operational_day_key_for_utc(interval.ended_at_utc)
            .format("%Y-%m-%d")
            .to_string();
        let operation_id = format!("finish:{expected_stable_id}");
        self.record_storage_result_for(
            PersistenceOperation::ActiveFinish,
            RecoveryAction::ReloadAuthority,
            sqlite::finish_tui_active_session(
                &database_path,
                &expected_stable_id,
                &operation_id,
                interval.ended_at_utc,
                &operational_day,
                elapsed,
            ),
        )?;
        self.time_tracker.set_active_description(String::new());
        self.time_tracker.current_session_start = None;
        self.session.active_session_stable_id = None;
        self.session.active_session_started_at_utc = None;
        self.reload_sqlite_sessions();
        Some(elapsed)
    }

    fn switch_active_category_at(
        &mut self,
        category_id: CategoryId,
        next_description: String,
        switched_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> bool {
        if self.time_tracker.active_category_id() == category_id
            || self.time_tracker.category_by_id(category_id).is_none()
        {
            return false;
        }
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return false;
        };
        let Some(expected_stable_id) = self.session.active_session_stable_id.clone() else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveSwitch,
                RecoveryAction::ReloadAuthority,
                Err("SQLite runtime has no active stable identity to switch".to_string()),
            );
            return false;
        };
        let interval = match self.reconciled_active_interval(switched_at_utc, clock_mode) {
            Ok(interval) => interval,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveSwitch,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return false;
            }
        };
        let elapsed = interval.elapsed_seconds;
        let operational_day = operational_day_key_for_utc(interval.ended_at_utc)
            .format("%Y-%m-%d")
            .to_string();
        let operation_id = transition_operation_id(
            "switch",
            &expected_stable_id,
            interval.ended_at_utc,
            &category_id.0.to_string(),
        );
        let next_stable_id = format!("tui-active:{operation_id}");
        let result = sqlite::switch_tui_active_session(
            &database_path,
            &expected_stable_id,
            &operation_id,
            &next_stable_id,
            category_id,
            &next_description,
            interval.ended_at_utc,
            &operational_day,
            elapsed,
        );
        let Some(receipt) = self.record_storage_result_for(
            PersistenceOperation::ActiveSwitch,
            RecoveryAction::ReloadAuthority,
            result,
        ) else {
            return false;
        };
        if !self.time_tracker.set_active_category_by_id(category_id) {
            return false;
        }
        self.time_tracker.set_active_description(next_description);
        self.session.active_session_stable_id = receipt.resulting_active_stable_id;
        if let Err(error) = self.begin_transition_session(interval.ended_at_utc, clock_mode) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }
        self.reload_sqlite_sessions();
        self.sync_drift_idle_state();
        self.refresh_active_runtime_checkpoint();
        !self.has_persistence_recovery()
    }

    fn simulation_backlog_duration_at''',
    "SQLite transition methods",
)
text = sub_once(
    text,
    r"    fn clear_detached_checkpoint\(&mut self\) \{.*?\n    fn reconcile_legacy_transition_receipt",
    '''    fn clear_detached_checkpoint(&mut self) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return;
        };
        let result = sqlite::clear_tui_checkpoint(&database_path);
        self.record_storage_result_for(
            PersistenceOperation::CheckpointClear,
            RecoveryAction::FlushCurrentState,
            result,
        );
    }

    fn reconcile_legacy_transition_receipt''',
    "checkpoint clear",
)
text = sub_once(
    text,
    r"    fn reconcile_legacy_transition_receipt\(.*?\n    fn restore_from_detached_checkpoint",
    "    fn restore_from_detached_checkpoint",
    "legacy checkpoint reconciliation",
)
text = sub_once(
    text,
    r"        let mut checkpoint: DetachedRuntimeCheckpoint = if let Some\(database_path\) =\n            self\.sqlite_database_path\.clone\(\)\n        \{.*?\n        \};",
    '''        let Some(database_path) = self.sqlite_database_path.clone() else {
            return false;
        };
        let mut checkpoint: DetachedRuntimeCheckpoint = match sqlite::load_tui_checkpoint(&database_path) {
            Ok(Some(claimed)) => {
                let Some(active_stable_id) = claimed.active_session_stable_id else {
                    let _ = sqlite::quarantine_tui_checkpoint(&database_path);
                    self.record_storage_result::<()>(Err(
                        "SQLite recovery checkpoint has no active stable identity".to_string(),
                    ));
                    return false;
                };
                self.session.active_session_stable_id = Some(active_stable_id);
                claimed.payload
            }
            Ok(None) => return false,
            Err(error) => {
                self.record_storage_result::<()>(Err(error));
                return false;
            }
        };''',
    "SQLite checkpoint load",
)
text = sub_once(
    text,
    r"\n        if self\.sqlite_database_path\.is_none\(\) \{\n            match self\.reconcile_legacy_transition_receipt.*?\n        \}\n",
    "\n",
    "legacy checkpoint call",
)
text = sub_once(
    text,
    r"        if checkpoint\.schema_version != DetachedRuntimeCheckpoint::VERSION\n            && checkpoint\.schema_version != DetachedRuntimeCheckpoint::PREVIOUS_VERSION\n            && checkpoint\.schema_version != DetachedRuntimeCheckpoint::LEGACY_VERSION",
    "        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION",
    "checkpoint schema compatibility",
)
for pattern in [
    r"\n\s*legacy_recovery_committed: false,",
    r"\n\s*legacy_transition: None,",
    r"\n\s*legacy_finish: None,",
]:
    text = re.sub(pattern, "", text)
text = sub_once(
    text,
    r"\n#\[cfg\(test\)\]\nmod bounded_checkpoint_tests \{.*?\n\}\n\n#\[cfg\(test\)\]\nmod category_catalog_tests",
    "\n#[cfg(test)]\nmod category_catalog_tests",
    "obsolete checkpoint compatibility tests",
)
text = re.sub(r"\n#\[cfg\(test\)\]\nmod legacy_switch_replay_tests \{.*\Z", "\n", text, flags=re.S)
path.write_text(text)

path = Path("src/app/category_lifecycle_view.rs")
text = path.read_text()
text = sub_once(
    text,
    r"use crate::\{\n    domain::\{CategoryId, DRIFT_CATEGORY_ID\},\n    legacy_category_lifecycle::\{.*?\},\n    sqlite,\n\};",
    '''use crate::{
    domain::{CategoryId, DRIFT_CATEGORY_ID},
    sqlite,
};''',
    "category lifecycle imports",
)
text = sub_once(
    text,
    r"    fn build_category_lifecycle_review\(.*?\n    fn apply_category_lifecycle",
    '''    fn build_category_lifecycle_review(
        &mut self,
        source_id: CategoryId,
        target_id: Option<CategoryId>,
    ) -> Result<CategoryLifecycleReview, String> {
        let database_path = self
            .sqlite_database_path
            .as_deref()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
        let preview = sqlite::preview_category_lifecycle_at(database_path, source_id, target_id)?;
        let source_id = CategoryId::new(
            u64::try_from(preview.source.id)
                .map_err(|_| "SQLite source category identity is invalid".to_string())?,
        );
        let target_id = preview
            .target
            .as_ref()
            .map(|target| u64::try_from(target.id).map(CategoryId::new))
            .transpose()
            .map_err(|_| "SQLite target category identity is invalid".to_string())?;
        let confirmation_phrase =
            lifecycle_confirmation_phrase(source_id, target_id, &preview.revision);
        Ok(CategoryLifecycleReview {
            source_id,
            source_name: preview.source.name,
            target_id,
            target_name: preview.target.map(|target| target.name),
            counts: CategoryLifecycleCounts {
                completed_sessions: preview.references.completed_sessions,
                active_sessions: preview.references.active_sessions,
                tags: preview.references.tags,
                sand_placed: preview.references.sand_placed,
                sand_pending: preview.references.sand_pending,
                history_placed: preview.references.snapshot_placed,
                history_pending: preview.references.snapshot_pending,
                checkpoint_references: preview.references.checkpoint_references,
            },
            checkpoint_custody: preview
                .checkpoint_status
                .unwrap_or_else(|| "absent".to_string()),
            revision: preview.revision,
            confirmation_phrase,
        })
    }

    fn apply_category_lifecycle''',
    "category review",
)
text = sub_once(
    text,
    r"    fn apply_category_lifecycle\(&mut self, review: CategoryLifecycleReview\) \{.*?\n    pub\(super\) fn render_category_lifecycle",
    '''    fn apply_category_lifecycle(&mut self, review: CategoryLifecycleReview) {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return;
        };
        let result = sqlite::apply_category_lifecycle_at(
            &database_path,
            review.source_id,
            review.target_id,
            &review.revision,
            Utc::now(),
        )
        .map(|_| ());
        if let Err(error) = result {
            if let Some(overlay) = self.category_lifecycle_overlay.as_mut() {
                overlay.error = Some(error);
                overlay.confirmation_input.clear();
            }
            return;
        }
        if let Err(error) = self.try_reload_authority() {
            let _ = self.record_storage_result_for::<()>(
                PersistenceOperation::CategoryLifecycle,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }
        self.category_lifecycle_overlay = None;
        self.ui_mode = UiMode::Main;
        self.selected_index = 0;
        self.render_needed = true;
    }

    pub(super) fn render_category_lifecycle''',
    "category apply",
)
text = re.sub(r"\nfn normalize_legacy_review\(.*\Z", "\n", text, flags=re.S)
path.write_text(text)

path = Path("src/app/persistence_recovery.rs")
text = path.read_text()
text = sub_once(
    text,
    r"\nfn load_legacy_recovery_authority\(.*?\n\}\n",
    "\n",
    "legacy recovery loader",
)
text = sub_once(
    text,
    r"        if let Some\(database_path\) = self\.sqlite_database_path\.clone\(\) \{(.*?)\n        \} else \{.*?\n        \}\n        Ok\(\(\)\)\n    \}\n\n    pub\(super\) fn try_reload_authority",
    r'''        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
\1
        Ok(())
    }

    pub(super) fn try_reload_authority''',
    "flush authority",
)
text = sub_once(
    text,
    r"    pub\(super\) fn try_reload_authority\(&mut self\) -> Result<\(\), String> \{\n        if let Some\(database_path\) = self\.sqlite_database_path\.clone\(\) \{(.*?)\n        \} else \{.*?\n        \}\n        self\.sync_drift_idle_state\(\);",
    r'''    pub(super) fn try_reload_authority(&mut self) -> Result<(), String> {
        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
\1
        self.sync_drift_idle_state();''',
    "reload authority",
)
text = sub_once(
    text,
    r"        let has_active = if self\.sqlite_database_path\.is_some\(\) \{\n            self\.session\.active_session_stable_id\.is_some\(\)\n        \} else \{\n            self\.session\.active_session_started_at_utc\.is_some\(\)\n        \};",
    "        let has_active = self.session.active_session_stable_id.is_some();",
    "finish active detection",
)
text = sub_once(
    text,
    r"        if let Some\(database_path\) = self\.sqlite_database_path\.clone\(\) \{\n            sqlite::clear_tui_checkpoint\(&database_path\)\?;\n        \} else \{\n            storage::delete_file_if_exists\(&storage::get_detached_runtime_path\(\)\)\?;\n        \}",
    '''        let database_path = self
            .sqlite_database_path
            .clone()
            .ok_or_else(|| "SQLite authority is unavailable".to_string())?;
        sqlite::clear_tui_checkpoint(&database_path)?;''',
    "finish checkpoint clear",
)
path.write_text(text)

path = Path("src/profile.rs")
text = path.read_text()
text = sub_once(
    text,
    r"        let env_profile = nonempty_env\(\"STRATA_PROFILE\"\);\n        let legacy_alias = nonempty_env\(\"STRATA_DATA_DIR\"\);.*?        env_profile\.or\(legacy_alias\)",
    '''        nonempty_env("STRATA_PROFILE")''',
    "profile alias removal",
)
path.write_text(text)

print("file-backed runtime compatibility pruned")
