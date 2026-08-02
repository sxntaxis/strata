from pathlib import Path

# ---------------------------------------------------------------------------
# Receipt model
# ---------------------------------------------------------------------------
legacy_path = Path("src/legacy_transition.rs")
legacy = legacy_path.read_text()
legacy = legacy.replace(
    "    domain::{CategoryId, OperationalDayPolicy, Session},",
    "    domain::{CategoryId, OperationalDayPolicy, Session, DRIFT_CATEGORY_ID},",
    1,
)
anchor = "\npub(crate) fn reconcile_completed_session(\n"
model = r'''

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ClearAllReceipt {
    pub version: u8,
    pub operation_id: String,
    pub applied_at_utc: DateTime<Utc>,
    pub previous_active: LegacyActiveReceipt,
    pub resulting_active: LegacyActiveReceipt,
    pub idle_reset: bool,
    pub affected_operational_days: Vec<String>,
}

impl ClearAllReceipt {
    pub(crate) const VERSION: u8 = 1;

    pub(crate) fn validate_boundaries(&self) -> Result<(), String> {
        if self.version != Self::VERSION {
            return Err(format!(
                "unsupported clear-all receipt version {}",
                self.version
            ));
        }
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
'''
if anchor not in legacy:
    raise SystemExit("legacy reconcile anchor not found")
legacy = legacy.replace(anchor, model + anchor, 1)

test_anchor = "    #[test]\n    fn absent_receipt_session_is_appended_once() {\n"
tests = r'''    fn clear_all_receipt(idle_reset: bool) -> ClearAllReceipt {
        let previous = LegacyActiveReceipt {
            category_id: if idle_reset { 0 } else { 4 },
            description: if idle_reset { "" } else { "work" }.to_string(),
            started_at_utc: Utc.with_ymd_and_hms(2026, 8, 1, 16, 0, 0).unwrap(),
        };
        let applied_at_utc = Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap();
        ClearAllReceipt {
            version: ClearAllReceipt::VERSION,
            operation_id: "clear-all:test".to_string(),
            applied_at_utc,
            previous_active: previous.clone(),
            resulting_active: LegacyActiveReceipt {
                category_id: previous.category_id,
                description: previous.description.clone(),
                started_at_utc: if idle_reset {
                    applied_at_utc
                } else {
                    previous.started_at_utc
                },
            },
            idle_reset,
            affected_operational_days: vec![
                "2026-08-01".to_string(),
                "2026-08-02".to_string(),
            ],
        }
    }

    #[test]
    fn clear_all_receipt_validates_idle_reset_and_non_idle_continuity() {
        clear_all_receipt(true).validate_boundaries().unwrap();
        clear_all_receipt(false).validate_boundaries().unwrap();
    }

    #[test]
    fn clear_all_receipt_rejects_hidden_classification_or_day_ambiguity() {
        let mut changed = clear_all_receipt(true);
        changed.resulting_active.category_id = 4;
        assert!(changed.validate_boundaries().unwrap_err().contains("classification"));

        let mut duplicate = clear_all_receipt(true);
        duplicate.affected_operational_days = vec![
            "2026-08-02".to_string(),
            "2026-08-02".to_string(),
        ];
        assert!(duplicate.validate_boundaries().unwrap_err().contains("unique and sorted"));
    }

'''
if test_anchor not in legacy:
    raise SystemExit("legacy test anchor not found")
legacy = legacy.replace(test_anchor, tests + test_anchor, 1)
legacy_path.write_text(legacy)

# ---------------------------------------------------------------------------
# Application semantics and legacy replay
# ---------------------------------------------------------------------------
app_path = Path("src/app.rs")
app = app_path.read_text()
app = app.replace(
    "    collections::{HashSet, VecDeque},",
    "    collections::{BTreeSet, HashSet, VecDeque},",
    1,
)
app = app.replace(
    "        LegacyActiveReceipt, LegacyFinishReceipt, LegacySessionReceipt, LegacyTransitionKind,\n        LegacyTransitionReceipt, reconcile_completed_session,",
    "        ClearAllReceipt, LegacyActiveReceipt, LegacyFinishReceipt, LegacySessionReceipt,\n        LegacyTransitionKind, LegacyTransitionReceipt, reconcile_completed_session,",
    1,
)
app = app.replace(
    "    legacy_finish: Option<LegacyFinishReceipt>,\n}",
    "    legacy_finish: Option<LegacyFinishReceipt>,\n    #[serde(default)]\n    clear_all: Option<ClearAllReceipt>,\n}",
    1,
)
# Add clear_all to every known constructor that currently ends with legacy_finish.
app = app.replace(
    "            legacy_finish: None,\n        })",
    "            legacy_finish: None,\n            clear_all: None,\n        })",
)
app = app.replace(
    "            legacy_finish: Some(receipt),\n        }",
    "            legacy_finish: Some(receipt),\n            clear_all: None,\n        }",
)
app = app.replace(
    "            legacy_finish: None,\n        }",
    "            legacy_finish: None,\n            clear_all: None,\n        }",
)

helper_anchor = "\n#[derive(Clone)]\nstruct SessionState"
helpers = r'''

fn sand_state_is_empty(state: &SandState) -> bool {
    state.grains.is_empty()
        && state.pending_grains.is_empty()
        && state.pending_runs.is_empty()
}

fn validate_clear_all_checkpoint(
    checkpoint: &DetachedRuntimeCheckpoint,
    receipt: &ClearAllReceipt,
) -> Result<(), String> {
    if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION {
        return Err(format!(
            "clear-all receipt requires checkpoint schema {}, found {}; evidence retained",
            DetachedRuntimeCheckpoint::VERSION,
            checkpoint.schema_version
        ));
    }
    if checkpoint.legacy_transition.is_some() || checkpoint.legacy_finish.is_some() {
        return Err("checkpoint contains overlapping transition receipts; evidence retained".to_string());
    }
    receipt.validate_boundaries()?;
    let prior_identity = format!(
        "{}:{}:{}",
        receipt.previous_active.category_id,
        receipt.previous_active.description,
        receipt
            .previous_active
            .started_at_utc
            .to_rfc3339_opts(SecondsFormat::Nanos, true)
    );
    let expected_operation_id = transition_operation_id(
        "clear-all",
        &prior_identity,
        receipt.applied_at_utc,
        if receipt.idle_reset { "idle-reset" } else { "active-preserved" },
    );
    if receipt.operation_id != expected_operation_id {
        return Err(format!(
            "clear-all receipt operation ID {} is inconsistent; evidence retained",
            receipt.operation_id
        ));
    }
    if checkpoint.active_category_id != receipt.resulting_active.category_id
        || checkpoint.active_description != receipt.resulting_active.description
        || checkpoint.active_session_started_at_utc
            != Some(receipt.resulting_active.started_at_utc)
    {
        return Err(format!(
            "clear-all receipt {} does not match its resulting checkpoint generation",
            receipt.operation_id
        ));
    }
    if !sand_state_is_empty(&checkpoint.sand_state) {
        return Err(format!(
            "clear-all receipt {} carries non-empty sediment",
            receipt.operation_id
        ));
    }
    Ok(())
}
'''
if helper_anchor not in app:
    raise SystemExit("app session helper anchor not found")
app = app.replace(helper_anchor, helpers + helper_anchor, 1)

# Add affected-day and replay methods before queue_or_apply_mutation.
method_anchor = "    fn queue_or_apply_mutation(&mut self, mutation: QueuedMutation) {\n"
methods = r'''    fn clear_all_affected_days(
        &self,
        applied_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Result<BTreeSet<NaiveDate>, String> {
        let mut days = BTreeSet::from([operational_day_key_for_utc(applied_at_utc)]);
        if !is_drift_category_id(self.time_tracker.active_category_id()) {
            return Ok(days);
        }
        let interval = self.reconciled_active_interval(applied_at_utc, clock_mode)?;
        let policy = OperationalDayPolicy::from_config(self.runtime_settings.day_boundary);
        days.extend(
            temporal::allocate_operational_day_slices(
                interval.started_at_utc,
                interval.ended_at_utc,
                interval.elapsed_seconds,
                policy,
            )?
            .into_iter()
            .map(|slice| slice.operational_day),
        );
        Ok(days)
    }

    fn reconcile_clear_all_receipt(
        &mut self,
        checkpoint: &mut DetachedRuntimeCheckpoint,
    ) -> Result<(), String> {
        let Some(receipt) = checkpoint.clear_all.clone() else {
            return Ok(());
        };
        validate_clear_all_checkpoint(checkpoint, &receipt)?;
        if self.sqlite_database_path.is_none() {
            storage::save_sand_state(&storage::get_sand_state_path(), &checkpoint.sand_state)?;
            for value in &receipt.affected_operational_days {
                let day = NaiveDate::parse_from_str(value, "%Y-%m-%d")
                    .map_err(|error| error.to_string())?;
                self.reconcile_daily_contribution(day);
                if let Some(recovery) = self.persistence_recovery.as_ref() {
                    return Err(recovery.failure.summary());
                }
            }
            checkpoint.clear_all = None;
            storage::write_json_atomic(&storage::get_detached_runtime_path(), checkpoint)?;
        } else if let Some(database_path) = self.sqlite_database_path.clone() {
            checkpoint.clear_all = None;
            sqlite::replace_tui_recovering_checkpoint(&database_path, checkpoint)?;
        }
        Ok(())
    }

    fn apply_clear_all_at(
        &mut self,
        applied_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) {
        let affected_days = match self.clear_all_affected_days(applied_at_utc, clock_mode) {
            Ok(days) => days,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::SandStateSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        };
        let previous_tracker = self.time_tracker.clone();
        let previous_session = self.session.clone();
        let previous_sand = self.sand_engine.snapshot_state();
        let previous_active = LegacyActiveReceipt {
            category_id: self.time_tracker.active_category_id().0,
            description: self
                .time_tracker
                .category_description_by_id(self.time_tracker.active_category_id())
                .unwrap_or_default()
                .to_string(),
            started_at_utc: match self.session.active_session_started_at_utc {
                Some(value) => value,
                None => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::ActiveReset,
                        RecoveryAction::ReloadAuthority,
                        Err("runtime has no active UTC start timestamp to clear".to_string()),
                    );
                    return;
                }
            },
        };
        let idle_reset = is_drift_category_id(self.time_tracker.active_category_id());

        self.sand_engine.clear();
        if idle_reset {
            if let Err(error) = self.begin_transition_session(applied_at_utc, clock_mode) {
                self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveReset,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        }
        let resulting_active = LegacyActiveReceipt {
            category_id: previous_active.category_id,
            description: previous_active.description.clone(),
            started_at_utc: if idle_reset {
                applied_at_utc
            } else {
                previous_active.started_at_utc
            },
        };
        let prior_identity = format!(
            "{}:{}:{}",
            previous_active.category_id,
            previous_active.description,
            previous_active
                .started_at_utc
                .to_rfc3339_opts(SecondsFormat::Nanos, true)
        );
        let receipt = ClearAllReceipt {
            version: ClearAllReceipt::VERSION,
            operation_id: transition_operation_id(
                "clear-all",
                &prior_identity,
                applied_at_utc,
                if idle_reset { "idle-reset" } else { "active-preserved" },
            ),
            applied_at_utc,
            previous_active,
            resulting_active,
            idle_reset,
            affected_operational_days: affected_days
                .iter()
                .map(|day| day.format("%Y-%m-%d").to_string())
                .collect(),
        };
        let mut checkpoint = match self.build_runtime_checkpoint() {
            Ok(value) => value,
            Err(error) => {
                self.time_tracker = previous_tracker;
                self.session = previous_session;
                self.sand_engine.restore_state(
                    &previous_sand,
                    &self
                        .time_tracker
                        .categories_for_storage()
                        .into_iter()
                        .chain(self.archived_categories.iter().cloned())
                        .map(|category| category.id)
                        .collect(),
                );
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        };
        checkpoint.clear_all = Some(receipt);

        if self.sqlite_database_path.is_some() {
            // The second B2C pass replaces this fail-closed placeholder with one
            // SQLite transaction. Until then, do not publish split authority.
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.sand_engine.restore_state(
                &previous_sand,
                &self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect(),
            );
            self.record_storage_result_for::<()>(
                PersistenceOperation::SandStateSave,
                RecoveryAction::ReloadAuthority,
                Err("SQLite clear-all transaction is not yet installed; no state changed".to_string()),
            );
            return;
        }

        if let Err(error) =
            storage::write_json_atomic(&storage::get_detached_runtime_path(), &checkpoint)
        {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.sand_engine.restore_state(
                &previous_sand,
                &self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect(),
            );
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }
        self.persist_sand_state();
        if self.has_persistence_recovery() {
            return;
        }
        for day in affected_days {
            self.reconcile_daily_contribution(day);
            if self.has_persistence_recovery() {
                return;
            }
        }
        self.refresh_active_runtime_checkpoint();
        self.sync_drift_idle_state();
    }

'''
if method_anchor not in app:
    raise SystemExit("queue mutation anchor not found")
app = app.replace(method_anchor, methods + method_anchor, 1)

# Replace destructive clear-all body with the selected operation.
old_body = r'''            QueuedMutation::ClearAllSand => {
                self.sand_engine.clear();

                let scheduled_day = operational_day_key_for_utc(scheduled_at_utc);
                if let Some(database_path) = self.sqlite_database_path.clone() {
                    let day = scheduled_day.format("%Y-%m-%d").to_string();
                    let result = sqlite::delete_tui_drift_sessions_for_day(&database_path, &day);
                    if self
                        .record_storage_result_for(
                            PersistenceOperation::DriftSessionDelete,
                            RecoveryAction::ReloadAuthority,
                            result,
                        )
                        .is_none()
                    {
                        return;
                    }
                }
                self.time_tracker
                    .clear_drift_sessions_for_day(scheduled_day);

                if is_drift_category_id(self.time_tracker.active_category_id()) {
                    self.reset_active_session_at(
                        scheduled_at_utc,
                        clock_mode == SessionClockMode::HistoricalWall,
                    );
                    self.sync_drift_idle_state();
                }

                self.persist_sessions();
                self.persist_sand_state();
                self.persist_daily_sand_snapshot();
            }
'''
new_body = r'''            QueuedMutation::ClearAllSand => {
                self.apply_clear_all_at(scheduled_at_utc, clock_mode);
            }
'''
if old_body not in app:
    raise SystemExit("destructive clear-all body not found")
app = app.replace(old_body, new_body, 1)

# Process clear receipt before legacy-only transition receipts.
old_dispatch = r'''        if self.sqlite_database_path.is_none() {
            match self.reconcile_legacy_transition_receipt(&mut checkpoint) {
'''
new_dispatch = r'''        if checkpoint.clear_all.is_some()
            && let Err(error) = self.reconcile_clear_all_receipt(&mut checkpoint)
        {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }

        if self.sqlite_database_path.is_none() {
            match self.reconcile_legacy_transition_receipt(&mut checkpoint) {
'''
if old_dispatch not in app:
    raise SystemExit("receipt dispatch anchor not found")
app = app.replace(old_dispatch, new_dispatch, 1)
app_path.write_text(app)

# ---------------------------------------------------------------------------
# Remove the hidden committed-session deletion APIs.
# ---------------------------------------------------------------------------
domain_path = Path("src/domain.rs")
domain = domain_path.read_text()
start = domain.find("    pub fn clear_drift_sessions_for_day(&mut self, day: NaiveDate) {")
if start != -1:
    end = domain.index("\n    }\n", start) + len("\n    }\n")
    domain = domain[:start] + domain[end:]
# Remove the focused test if present.
test_start = domain.find("    fn test_clear_drift_sessions_for_day_clears_only_target_day()")
if test_start != -1:
    attr = domain.rfind("    #[test]", 0, test_start)
    next_attr = domain.find("    #[test]", test_start + 1)
    if attr == -1 or next_attr == -1:
        raise SystemExit("could not bound drift clear test")
    domain = domain[:attr] + domain[next_attr:]
domain_path.write_text(domain)

sqlite_path = Path("src/sqlite/tui_runtime.rs")
sqlite = sqlite_path.read_text()
start = sqlite.find("pub(crate) fn delete_drift_sessions_for_day(")
if start == -1:
    raise SystemExit("SQLite drift deletion function not found")
end = sqlite.index("\npub(crate) fn save_sand_state", start)
sqlite = sqlite[:start] + sqlite[end + 1:]
sqlite_path.write_text(sqlite)

root_path = Path("src/sqlite.rs")
root = root_path.read_text()
root = root.replace(
    "    delete_daily_snapshot as delete_tui_daily_snapshot,\n    delete_drift_sessions_for_day as delete_tui_drift_sessions_for_day,\n    delete_session as delete_tui_session,",
    "    delete_daily_snapshot as delete_tui_daily_snapshot, delete_session as delete_tui_session,",
    1,
)
root_path.write_text(root)
