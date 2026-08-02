from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:160]!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/lib.rs",
    "mod keybindings;\n",
    "mod keybindings;\nmod legacy_transition;\n",
)
replace_once(
    "src/domain.rs",
    "pub struct TimeTracker {\n",
    "#[derive(Clone)]\npub struct TimeTracker {\n",
)

path = Path("src/app.rs")
text = path.read_text()
text = text.replace(
    "    keybindings::{self, Action, ActionBindingState, KeyBinding},\n",
    "    keybindings::{self, Action, ActionBindingState, KeyBinding},\n    legacy_transition::{\n        LegacyActiveReceipt, LegacySessionReceipt, LegacyTransitionKind,\n        LegacyTransitionReceipt, reconcile_completed_session,\n    },\n",
    1,
)
text = text.replace(
    '''    #[serde(default)]
    legacy_recovery_committed: bool,
}

impl DetachedRuntimeCheckpoint {
    const VERSION: u8 = 2;
    const LEGACY_VERSION: u8 = 1;
}

struct SessionState {
''',
    '''    #[serde(default)]
    legacy_recovery_committed: bool,
    #[serde(default)]
    legacy_transition: Option<LegacyTransitionReceipt>,
}

impl DetachedRuntimeCheckpoint {
    const VERSION: u8 = 3;
    const PREVIOUS_VERSION: u8 = 2;
    const LEGACY_VERSION: u8 = 1;
}

#[derive(Clone)]
struct SessionState {
''',
    1,
)
# Every current checkpoint constructor starts without a receipt.
text = text.replace(
    "            legacy_recovery_committed: false,\n",
    "            legacy_recovery_committed: false,\n            legacy_transition: None,\n",
)
# Version compatibility.
text = text.replace(
    '''        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION
            && checkpoint.schema_version != DetachedRuntimeCheckpoint::LEGACY_VERSION
''',
    '''        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION
            && checkpoint.schema_version != DetachedRuntimeCheckpoint::PREVIOUS_VERSION
            && checkpoint.schema_version != DetachedRuntimeCheckpoint::LEGACY_VERSION
''',
    1,
)

# Startup replay method inserted before detached restore.
anchor = "    fn restore_from_detached_checkpoint(&mut self) -> bool {\n"
method = r'''    fn reconcile_legacy_transition_receipt(
        &mut self,
        checkpoint: &mut DetachedRuntimeCheckpoint,
    ) -> Result<(), String> {
        let Some(receipt) = checkpoint.legacy_transition.clone() else {
            return Ok(());
        };
        if self.sqlite_database_path.is_some() {
            return Err(
                "legacy transition receipt appeared under SQLite authority; evidence retained"
                    .to_string(),
            );
        }
        if receipt.version != LegacyTransitionReceipt::VERSION {
            return Err(format!(
                "unsupported legacy transition receipt version {}",
                receipt.version
            ));
        }
        if receipt.kind != LegacyTransitionKind::Switch {
            return Err("unsupported legacy transition kind; evidence retained".to_string());
        }
        if checkpoint.active_category_id != receipt.resulting_active.category_id
            || checkpoint.active_description != receipt.resulting_active.description
            || checkpoint.active_session_started_at_utc
                != Some(receipt.resulting_active.started_at_utc)
        {
            return Err(format!(
                "legacy switch receipt {} does not match its resulting checkpoint generation",
                receipt.operation_id
            ));
        }

        let mut staged_tracker = self.time_tracker.clone();
        reconcile_completed_session(
            &mut staged_tracker.sessions,
            &mut staged_tracker.session_id_counter,
            receipt.completed_session.as_ref(),
        )?;
        let previous_category_id = CategoryId::new(receipt.expected_previous_category_id);
        if !staged_tracker.set_category_description_by_id(previous_category_id, String::new()) {
            return Err(format!(
                "legacy switch receipt {} references unavailable previous category {}",
                receipt.operation_id, receipt.expected_previous_category_id
            ));
        }
        let resulting_category_id = CategoryId::new(receipt.resulting_active.category_id);
        if !staged_tracker.set_category_description_by_id(
            resulting_category_id,
            receipt.resulting_active.description.clone(),
        ) {
            return Err(format!(
                "legacy switch receipt {} references unavailable resulting category {}",
                receipt.operation_id, receipt.resulting_active.category_id
            ));
        }

        let mut catalog = staged_tracker.categories_for_storage();
        catalog.extend(self.archived_categories.iter().cloned());
        storage::save_sessions_to_csv(
            &storage::get_time_log_path(),
            &staged_tracker.sessions,
            &catalog,
        )?;
        storage::save_category_catalog_to_csv(
            &storage::get_categories_path(),
            &staged_tracker.categories_for_storage(),
            &self.archived_categories,
        )?;

        checkpoint.legacy_transition = None;
        checkpoint.schema_version = DetachedRuntimeCheckpoint::VERSION;
        storage::write_json_atomic(&storage::get_detached_runtime_path(), checkpoint)?;
        self.time_tracker = staged_tracker;
        Ok(())
    }

'''
if text.count(anchor) != 1:
    raise SystemExit("detached restore insertion anchor not found")
text = text.replace(anchor, method + anchor, 1)

# Reconcile legacy receipts before ordinary checkpoint claim/recovery.
old = '''            };

        self.checkpoint_recovery_active = true;

        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION
'''
new = '''            };

        if self.sqlite_database_path.is_none()
            && let Err(error) = self.reconcile_legacy_transition_receipt(&mut checkpoint)
        {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }

        self.checkpoint_recovery_active = true;

        if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION
'''
if text.count(old) != 1:
    raise SystemExit("legacy receipt reconciliation call anchor not found")
text = text.replace(old, new, 1)

# Replace the legacy switch body with prepared-receipt publication.
old_switch = '''        if self
            .end_active_session_at(switched_at_utc, clock_mode)
            .is_none()
        {
            return false;
        }
        self.persist_sessions();

        if !self.time_tracker.set_active_category_by_id(category_id) {
            return false;
        }

        if let Err(error) = self.begin_transition_session(switched_at_utc, clock_mode) {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }
        self.sync_drift_idle_state();
        self.refresh_active_runtime_checkpoint();

        !self.has_persistence_recovery()
'''
new_switch = '''        let previous_tracker = self.time_tracker.clone();
        let previous_session = self.session.clone();
        let previous_category_id = self.time_tracker.active_category_id();
        let Some(previous_started_at_utc) = self.session.active_session_started_at_utc else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveSwitch,
                RecoveryAction::ReloadAuthority,
                Err("legacy runtime has no active UTC start timestamp to switch".to_string()),
            );
            return false;
        };
        let previous_session_count = self.time_tracker.sessions.len();

        if self
            .end_active_session_at(switched_at_utc, clock_mode)
            .is_none()
        {
            return false;
        }
        let completed_session = self
            .time_tracker
            .sessions
            .get(previous_session_count)
            .map(LegacySessionReceipt::from_session);

        if !self.time_tracker.set_active_category_by_id(category_id) {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            return false;
        }
        if let Err(error) = self.begin_transition_session(switched_at_utc, clock_mode) {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }

        let resulting_description = self
            .time_tracker
            .category_description_by_id(category_id)
            .unwrap_or_default()
            .to_string();
        let expected_identity = format!(
            "legacy:{}:{}",
            previous_category_id.0,
            previous_started_at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true)
        );
        let operation_id = self.transition_operation_id(
            "legacy-switch",
            &expected_identity,
            switched_at_utc,
            &category_id.0.to_string(),
        );
        let receipt = LegacyTransitionReceipt {
            version: LegacyTransitionReceipt::VERSION,
            operation_id,
            kind: LegacyTransitionKind::Switch,
            expected_previous_category_id: previous_category_id.0,
            expected_previous_started_at_utc: previous_started_at_utc,
            transition_at_utc: switched_at_utc,
            completed_session,
            resulting_active: LegacyActiveReceipt {
                category_id: category_id.0,
                description: resulting_description,
                started_at_utc: switched_at_utc,
            },
        };
        let mut prepared_checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.time_tracker = previous_tracker;
                self.session = previous_session;
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return false;
            }
        };
        prepared_checkpoint.legacy_transition = Some(receipt);
        if let Err(error) = storage::write_json_atomic(
            &storage::get_detached_runtime_path(),
            &prepared_checkpoint,
        ) {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }

        self.persist_sessions();
        if self.has_persistence_recovery() {
            return false;
        }
        self.persist_categories();
        if self.has_persistence_recovery() {
            return false;
        }
        self.sync_drift_idle_state();
        self.refresh_active_runtime_checkpoint();

        !self.has_persistence_recovery()
'''
if text.count(old_switch) != 1:
    raise SystemExit("legacy switch body not found")
text = text.replace(old_switch, new_switch, 1)
path.write_text(text)

for temporary in [
    ".github/workflows/reconciliation001b2a-apply.yml",
    "tools/reconciliation001b2a-apply.py",
]:
    Path(temporary).unlink(missing_ok=True)
