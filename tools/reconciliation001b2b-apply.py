from pathlib import Path

# Extend the legacy transition model without changing the certified switch receipt shape.
legacy_path = Path("src/legacy_transition.rs")
legacy = legacy_path.read_text()
legacy = legacy.replace(
    "pub(crate) enum LegacyTransitionKind {\n    Switch,\n}",
    "pub(crate) enum LegacyTransitionKind {\n    Switch,\n}",
)
legacy = legacy.replace(
    "    fn validate_payload(&self) -> Result<(), String> {",
    "    pub(crate) fn validate_payload(&self) -> Result<(), String> {",
    1,
)
insert_anchor = "\npub(crate) fn reconcile_completed_session(\n"
finish_model = r'''

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LegacyFinishReceipt {
    pub version: u8,
    pub operation_id: String,
    pub expected_previous_category_id: u64,
    pub expected_previous_description: String,
    pub expected_previous_started_at_utc: DateTime<Utc>,
    pub finished_at_utc: DateTime<Utc>,
    pub completed_session: Option<LegacySessionReceipt>,
}

impl LegacyFinishReceipt {
    pub(crate) const VERSION: u8 = 1;

    pub(crate) fn validate_boundaries(&self) -> Result<(), String> {
        if self.version != Self::VERSION {
            return Err(format!(
                "unsupported legacy finish receipt version {}",
                self.version
            ));
        }
        if self.finished_at_utc < self.expected_previous_started_at_utc {
            return Err(format!(
                "legacy finish receipt {} ends before its active start",
                self.operation_id
            ));
        }
        let whole_elapsed = usize::try_from(
            (self.finished_at_utc - self.expected_previous_started_at_utc).num_seconds(),
        )
        .map_err(|_| {
            format!(
                "legacy finish receipt {} duration exceeds this platform's range",
                self.operation_id
            )
        })?;
        match (whole_elapsed, self.completed_session.as_ref()) {
            (0, None) => Ok(()),
            (0, Some(_)) => Err(format!(
                "legacy finish receipt {} stores a completed row for a zero-whole-second finish",
                self.operation_id
            )),
            (_, None) => Err(format!(
                "legacy finish receipt {} omits {} completed whole seconds",
                self.operation_id, whole_elapsed
            )),
            (expected_elapsed, Some(completed)) => {
                completed.validate_payload()?;
                if completed.category_id != self.expected_previous_category_id {
                    return Err(format!(
                        "legacy finish receipt {} completed the wrong category",
                        self.operation_id
                    ));
                }
                if completed.description != self.expected_previous_description {
                    return Err(format!(
                        "legacy finish receipt {} completed the wrong description",
                        self.operation_id
                    ));
                }
                if completed.elapsed_seconds != expected_elapsed {
                    return Err(format!(
                        "legacy finish receipt {} completed {} seconds but its active boundary owns {}",
                        self.operation_id, completed.elapsed_seconds, expected_elapsed
                    ));
                }
                if completed.ended_at_utc != Some(self.finished_at_utc) {
                    return Err(format!(
                        "legacy finish receipt {} has inconsistent completion time",
                        self.operation_id
                    ));
                }
                let elapsed = i64::try_from(expected_elapsed).map_err(|_| {
                    format!(
                        "legacy finish receipt {} duration exceeds chrono range",
                        self.operation_id
                    )
                })?;
                let expected_completed_start = self
                    .finished_at_utc
                    .checked_sub_signed(ChronoDuration::seconds(elapsed))
                    .ok_or_else(|| {
                        format!(
                            "legacy finish receipt {} completed start exceeds chrono range",
                            self.operation_id
                        )
                    })?;
                if completed.started_at_utc != Some(expected_completed_start) {
                    return Err(format!(
                        "legacy finish receipt {} completed start does not preserve its whole-second interval",
                        self.operation_id
                    ));
                }
                Ok(())
            }
        }
    }
}
'''
if insert_anchor not in legacy:
    raise SystemExit("legacy reconcile anchor not found")
legacy = legacy.replace(insert_anchor, finish_model + insert_anchor, 1)

# Add focused finish boundary tests.
test_anchor = "    #[test]\n    fn absent_receipt_session_is_appended_once() {\n"
finish_tests = r'''    fn finish_receipt(completed_session: Option<LegacySessionReceipt>) -> LegacyFinishReceipt {
        LegacyFinishReceipt {
            version: LegacyFinishReceipt::VERSION,
            operation_id: "legacy-finish:test".to_string(),
            expected_previous_category_id: 4,
            expected_previous_description: "work".to_string(),
            expected_previous_started_at_utc: Utc
                .with_ymd_and_hms(2026, 8, 2, 16, 0, 0)
                .unwrap(),
            finished_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap(),
            completed_session,
        }
    }

    #[test]
    fn finish_receipt_validates_completed_and_zero_second_boundaries() {
        let completed = LegacySessionReceipt::from_session(&session(7, "work"));
        finish_receipt(Some(completed)).validate_boundaries().unwrap();

        let start = Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap();
        let mut zero = finish_receipt(None);
        zero.expected_previous_started_at_utc = start;
        zero.finished_at_utc = start + ChronoDuration::milliseconds(900);
        zero.validate_boundaries().unwrap();
    }

    #[test]
    fn finish_receipt_rejects_missing_or_wrong_completion() {
        assert!(
            finish_receipt(None)
                .validate_boundaries()
                .unwrap_err()
                .contains("omits 3600 completed whole seconds")
        );
        let mut wrong = session(7, "other");
        wrong.description = "other".to_string();
        assert!(
            finish_receipt(Some(LegacySessionReceipt::from_session(&wrong)))
                .validate_boundaries()
                .unwrap_err()
                .contains("wrong description")
        );
    }

'''
if test_anchor not in legacy:
    raise SystemExit("legacy test anchor not found")
legacy = legacy.replace(test_anchor, finish_tests + test_anchor, 1)
legacy_path.write_text(legacy)

app_path = Path("src/app.rs")
app = app_path.read_text()
app = app.replace(
    "        LegacyActiveReceipt, LegacySessionReceipt, LegacyTransitionKind, LegacyTransitionReceipt,\n        reconcile_completed_session,",
    "        LegacyActiveReceipt, LegacyFinishReceipt, LegacySessionReceipt, LegacyTransitionKind,\n        LegacyTransitionReceipt, reconcile_completed_session,",
    1,
)
app = app.replace(
    "    legacy_transition: Option<LegacyTransitionReceipt>,\n}",
    "    legacy_transition: Option<LegacyTransitionReceipt>,\n    #[serde(default)]\n    legacy_finish: Option<LegacyFinishReceipt>,\n}",
    1,
)
app = app.replace(
    "            legacy_transition: None,\n        })",
    "            legacy_transition: None,\n            legacy_finish: None,\n        })",
    1,
)
# Test checkpoint constructors also need the optional finish field.
app = app.replace(
    "            legacy_transition: None,\n        }\n    }",
    "            legacy_transition: None,\n            legacy_finish: None,\n        }\n    }",
)

# Add finish validation and publication core next to switch helpers.
helper_anchor = "\n#[derive(Clone)]\nstruct SessionState"
finish_helpers = r'''

fn validate_legacy_finish_checkpoint(
    checkpoint: &DetachedRuntimeCheckpoint,
    receipt: &LegacyFinishReceipt,
) -> Result<(), String> {
    if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION {
        return Err(format!(
            "legacy finish receipt requires checkpoint schema {}, found {}; evidence retained",
            DetachedRuntimeCheckpoint::VERSION,
            checkpoint.schema_version
        ));
    }
    if checkpoint.legacy_transition.is_some() {
        return Err("checkpoint contains both switch and finish receipts; evidence retained".to_string());
    }
    receipt.validate_boundaries()?;
    let expected_identity = format!(
        "legacy:{}:{}",
        receipt.expected_previous_category_id,
        receipt
            .expected_previous_started_at_utc
            .to_rfc3339_opts(SecondsFormat::Nanos, true)
    );
    let expected_operation_id = transition_operation_id(
        "legacy-finish",
        &expected_identity,
        receipt.finished_at_utc,
        "complete",
    );
    if receipt.operation_id != expected_operation_id {
        return Err(format!(
            "legacy finish receipt operation ID {} is inconsistent; evidence retained",
            receipt.operation_id
        ));
    }
    if checkpoint.active_category_id != receipt.expected_previous_category_id
        || checkpoint.active_description != receipt.expected_previous_description
        || checkpoint.active_session_started_at_utc
            != Some(receipt.expected_previous_started_at_utc)
    {
        return Err(format!(
            "legacy finish receipt {} does not match its prior checkpoint generation",
            receipt.operation_id
        ));
    }
    Ok(())
}

fn publish_legacy_finish_replay(
    tracker: &TimeTracker,
    archived_categories: &[Category],
    checkpoint: &DetachedRuntimeCheckpoint,
    receipt: &LegacyFinishReceipt,
    sessions_path: &Path,
    categories_path: &Path,
    sand_path: &Path,
) -> Result<TimeTracker, String> {
    let mut staged_tracker = tracker.clone();
    reconcile_completed_session(
        &mut staged_tracker.sessions,
        &mut staged_tracker.session_id_counter,
        receipt.completed_session.as_ref(),
    )?;
    let previous_category_id = CategoryId::new(receipt.expected_previous_category_id);
    if !staged_tracker.set_category_description_by_id(previous_category_id, String::new()) {
        return Err(format!(
            "legacy finish receipt {} references unavailable previous category {}",
            receipt.operation_id, receipt.expected_previous_category_id
        ));
    }
    let mut catalog = staged_tracker.categories_for_storage();
    catalog.extend(archived_categories.iter().cloned());
    storage::save_sessions_to_csv(sessions_path, &staged_tracker.sessions, &catalog)?;
    storage::save_category_catalog_to_csv(
        categories_path,
        &staged_tracker.categories_for_storage(),
        archived_categories,
    )?;
    storage::save_sand_state(sand_path, &checkpoint.sand_state)?;
    Ok(staged_tracker)
}
'''
if helper_anchor not in app:
    raise SystemExit("app helper anchor not found")
app = app.replace(helper_anchor, finish_helpers + helper_anchor, 1)

# Insert prepared legacy finish method after the simple now wrapper.
method_anchor = '''    fn end_active_session_now(&mut self) -> Option<usize> {
        self.end_active_session_at(Utc::now(), SessionClockMode::LiveMonotonic)
    }

'''
prepare_method = r'''    fn end_active_session_now(&mut self) -> Option<usize> {
        self.end_active_session_at(Utc::now(), SessionClockMode::LiveMonotonic)
    }

    fn prepare_active_finish_for_exit(&mut self) -> Option<usize> {
        if self.sqlite_database_path.is_some() {
            return self.end_active_session_now();
        }
        let interval = match self.reconciled_active_interval(
            Utc::now(),
            SessionClockMode::LiveMonotonic,
        ) {
            Ok(interval) => interval,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveFinish,
                    RecoveryAction::FinishAndExit,
                    Err(error),
                );
                return None;
            }
        };
        let previous_tracker = self.time_tracker.clone();
        let previous_session = self.session.clone();
        let previous_category_id = self.time_tracker.active_category_id();
        let previous_description = self
            .time_tracker
            .category_description_by_id(previous_category_id)
            .unwrap_or_default()
            .to_string();
        let Some(previous_started_at_utc) = self.session.active_session_started_at_utc else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveFinish,
                RecoveryAction::FinishAndExit,
                Err("legacy runtime has no active UTC start timestamp to finish".to_string()),
            );
            return None;
        };
        let mut prepared_checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::FinishAndExit,
                    Err(error),
                );
                return None;
            }
        };
        let previous_session_count = self.time_tracker.sessions.len();
        let ended_civil = civil_time_for_utc(interval.ended_at_utc);
        let result = self
            .time_tracker
            .end_session_with_elapsed_at_local(interval.elapsed_seconds, ended_civil);
        self.session.active_session_started_at_utc = None;
        let completed_session = self
            .time_tracker
            .sessions
            .get(previous_session_count)
            .map(LegacySessionReceipt::from_session);
        let expected_identity = format!(
            "legacy:{}:{}",
            previous_category_id.0,
            previous_started_at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true)
        );
        let receipt = LegacyFinishReceipt {
            version: LegacyFinishReceipt::VERSION,
            operation_id: transition_operation_id(
                "legacy-finish",
                &expected_identity,
                interval.ended_at_utc,
                "complete",
            ),
            expected_previous_category_id: previous_category_id.0,
            expected_previous_description: previous_description,
            expected_previous_started_at_utc: previous_started_at_utc,
            finished_at_utc: interval.ended_at_utc,
            completed_session,
        };
        prepared_checkpoint.legacy_finish = Some(receipt);
        if let Err(error) = storage::write_json_atomic(
            &storage::get_detached_runtime_path(),
            &prepared_checkpoint,
        ) {
            self.time_tracker = previous_tracker;
            self.session = previous_session;
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointSave,
                RecoveryAction::FinishAndExit,
                Err(error),
            );
            return None;
        }
        result
    }

'''
if method_anchor not in app:
    raise SystemExit("finish wrapper anchor not found")
app = app.replace(method_anchor, prepare_method, 1)

# Replace replay dispatch with finish-aware outcome.
old_reconcile = r'''    fn reconcile_legacy_transition_receipt(
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
        validate_legacy_switch_checkpoint(checkpoint, &receipt)?;
        let staged_tracker = publish_legacy_switch_replay(
            &self.time_tracker,
            &self.archived_categories,
            checkpoint,
            &receipt,
            &storage::get_time_log_path(),
            &storage::get_categories_path(),
            &storage::get_detached_runtime_path(),
        )?;
        self.time_tracker = staged_tracker;
        Ok(())
    }
'''
new_reconcile = r'''    fn reconcile_legacy_transition_receipt(
        &mut self,
        checkpoint: &mut DetachedRuntimeCheckpoint,
    ) -> Result<bool, String> {
        if self.sqlite_database_path.is_some()
            && (checkpoint.legacy_transition.is_some() || checkpoint.legacy_finish.is_some())
        {
            return Err(
                "legacy transition receipt appeared under SQLite authority; evidence retained"
                    .to_string(),
            );
        }
        if let Some(receipt) = checkpoint.legacy_finish.clone() {
            validate_legacy_finish_checkpoint(checkpoint, &receipt)?;
            let staged_tracker = publish_legacy_finish_replay(
                &self.time_tracker,
                &self.archived_categories,
                checkpoint,
                &receipt,
                &storage::get_time_log_path(),
                &storage::get_categories_path(),
                &storage::get_sand_state_path(),
            )?;
            self.time_tracker = staged_tracker;
            let valid_category_ids = self
                .time_tracker
                .categories_for_storage()
                .into_iter()
                .chain(self.archived_categories.iter().cloned())
                .map(|category| category.id)
                .collect::<HashSet<_>>();
            self.sand_engine
                .restore_state(&checkpoint.sand_state, &valid_category_ids);
            self.reconcile_all_daily_contributions();
            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
            storage::delete_file_if_exists(&storage::get_detached_runtime_path())?;
            return Ok(true);
        }
        let Some(receipt) = checkpoint.legacy_transition.clone() else {
            return Ok(false);
        };
        validate_legacy_switch_checkpoint(checkpoint, &receipt)?;
        let staged_tracker = publish_legacy_switch_replay(
            &self.time_tracker,
            &self.archived_categories,
            checkpoint,
            &receipt,
            &storage::get_time_log_path(),
            &storage::get_categories_path(),
            &storage::get_detached_runtime_path(),
        )?;
        self.time_tracker = staged_tracker;
        Ok(false)
    }
'''
if old_reconcile not in app:
    raise SystemExit("reconcile function anchor not found")
app = app.replace(old_reconcile, new_reconcile, 1)

old_dispatch = r'''        if self.sqlite_database_path.is_none()
            && let Err(error) = self.reconcile_legacy_transition_receipt(&mut checkpoint)
        {
            self.record_storage_result_for::<()>(
                PersistenceOperation::CheckpointRecovery,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return false;
        }
'''
new_dispatch = r'''        if self.sqlite_database_path.is_none() {
            match self.reconcile_legacy_transition_receipt(&mut checkpoint) {
                Ok(true) => return false,
                Ok(false) => {}
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::CheckpointRecovery,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return false;
                }
            }
        }
'''
if old_dispatch not in app:
    raise SystemExit("replay dispatch anchor not found")
app = app.replace(old_dispatch, new_dispatch, 1)

# Normal finish must prepare the receipt and publish the cleared catalog.
app = app.replace(
    "            app.end_active_session_now();\n            if !app.has_persistence_recovery() {\n                app.persist_sessions();\n            }\n            if !app.has_persistence_recovery() {\n                app.persist_sand_state();",
    "            app.prepare_active_finish_for_exit();\n            if !app.has_persistence_recovery() {\n                app.persist_sessions();\n            }\n            if !app.has_persistence_recovery() {\n                app.persist_categories();\n            }\n            if !app.has_persistence_recovery() {\n                app.persist_sand_state();",
    1,
)
app_path.write_text(app)

# Recovery retry must preserve archived categories and know how to prepare a legacy finish.
recovery_path = Path("src/app/persistence_recovery.rs")
recovery = recovery_path.read_text()
old_legacy_flush = r'''        } else {
            storage::save_categories_to_csv(&storage::get_categories_path(), &categories)
                .map_err(|error| error.to_string())?;
            storage::save_category_tags(&storage::get_category_tags_path(), &self.category_tags)?;
            storage::save_sessions_to_csv(
                &storage::get_time_log_path(),
                &self.time_tracker.sessions,
                &categories,
            )
            .map_err(|error| error.to_string())?;
'''
new_legacy_flush = r'''        } else {
            storage::save_category_catalog_to_csv(
                &storage::get_categories_path(),
                &categories,
                &self.archived_categories,
            )?;
            storage::save_category_tags(&storage::get_category_tags_path(), &self.category_tags)?;
            let mut session_categories = categories.clone();
            session_categories.extend(self.archived_categories.iter().cloned());
            storage::save_sessions_to_csv(
                &storage::get_time_log_path(),
                &self.time_tracker.sessions,
                &session_categories,
            )
            .map_err(|error| error.to_string())?;
'''
if old_legacy_flush not in recovery:
    raise SystemExit("legacy flush anchor not found")
recovery = recovery.replace(old_legacy_flush, new_legacy_flush, 1)
old_finish_retry = r'''        if self.session.active_session_stable_id.is_some() {
            self.end_active_session_now();
            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
        }
'''
new_finish_retry = r'''        let has_active = if self.sqlite_database_path.is_some() {
            self.session.active_session_stable_id.is_some()
        } else {
            self.session.active_session_started_at_utc.is_some()
        };
        if has_active {
            self.prepare_active_finish_for_exit();
            if let Some(recovery) = self.persistence_recovery.as_ref() {
                return Err(recovery.failure.summary());
            }
        }
'''
if old_finish_retry not in recovery:
    raise SystemExit("finish retry anchor not found")
recovery = recovery.replace(old_finish_retry, new_finish_retry, 1)
recovery_path.write_text(recovery)
