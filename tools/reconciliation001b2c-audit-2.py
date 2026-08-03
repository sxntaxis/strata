from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label} anchor missing")
    return text.replace(old, new, 1)

# ---------------------------------------------------------------------------
# Application replay: deterministic operation identity, canonical elapsed,
# resulting active state, and checkpoint grid before daily reconciliation.
# ---------------------------------------------------------------------------
app_path = Path("src/app.rs")
app = app_path.read_text()
transition_anchor = '''fn transition_operation_id(
    kind: &str,
    expected_stable_id: &str,
    at_utc: DateTime<Utc>,
    discriminator: &str,
) -> String {
    format!(
        "{}:{}:{}:{}",
        kind,
        expected_stable_id,
        at_utc.to_rfc3339_opts(SecondsFormat::Nanos, true),
        discriminator
    )
}
'''
helpers = transition_anchor + r'''

fn clear_all_operation_id(
    previous_active: &LegacyActiveReceipt,
    applied_at_utc: DateTime<Utc>,
    idle_reset: bool,
    previous_elapsed_seconds: usize,
    affected_operational_days: &[String],
) -> String {
    let description = &previous_active.description;
    let identity = format!(
        "{}:{}:{}:{}:{}:{}",
        previous_active.category_id,
        description.len(),
        description,
        previous_active
            .started_at_utc
            .to_rfc3339_opts(SecondsFormat::Nanos, true),
        previous_elapsed_seconds,
        affected_operational_days.join(",")
    );
    transition_operation_id(
        "clear-all",
        &identity,
        applied_at_utc,
        if idle_reset {
            "idle-reset"
        } else {
            "active-preserved"
        },
    )
}

fn clear_all_affected_days_for_interval(
    operation_day: NaiveDate,
    idle_reset: bool,
    previous_started_at_utc: DateTime<Utc>,
    applied_at_utc: DateTime<Utc>,
    previous_elapsed_seconds: usize,
    policy: OperationalDayPolicy,
) -> Result<BTreeSet<NaiveDate>, String> {
    let mut days = BTreeSet::from([operation_day]);
    if idle_reset {
        days.extend(
            temporal::allocate_operational_day_slices(
                previous_started_at_utc,
                applied_at_utc,
                previous_elapsed_seconds,
                policy,
            )?
            .into_iter()
            .map(|slice| slice.operational_day),
        );
    }
    Ok(days)
}

fn stage_clear_all_active_state(
    tracker: &mut TimeTracker,
    active_session_started_at_utc: &mut Option<DateTime<Utc>>,
    receipt: &ClearAllReceipt,
) -> Result<(), String> {
    let resulting_category_id = CategoryId::new(receipt.resulting_active.category_id);
    if !tracker.set_active_category_by_id(resulting_category_id) {
        return Err(format!(
            "clear-all receipt {} references unavailable resulting category {}",
            receipt.operation_id, receipt.resulting_active.category_id
        ));
    }
    if !tracker.set_category_description_by_id(
        resulting_category_id,
        receipt.resulting_active.description.clone(),
    ) {
        return Err(format!(
            "clear-all receipt {} cannot restore its resulting description",
            receipt.operation_id
        ));
    }
    let resulting_elapsed_seconds = if receipt.idle_reset {
        0
    } else {
        receipt.previous_elapsed_seconds
    };
    tracker.start_session_with_elapsed(resulting_elapsed_seconds)?;
    *active_session_started_at_utc = Some(receipt.resulting_active.started_at_utc);
    Ok(())
}
'''
app = replace_once(app, transition_anchor, helpers, "clear-all helper insertion")

old_id_validation = '''    let prior_identity = format!(
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
        if receipt.idle_reset {
            "idle-reset"
        } else {
            "active-preserved"
        },
    );'''
new_id_validation = '''    let expected_operation_id = clear_all_operation_id(
        &receipt.previous_active,
        receipt.applied_at_utc,
        receipt.idle_reset,
        receipt.previous_elapsed_seconds,
        &receipt.affected_operational_days,
    );'''
app = replace_once(app, old_id_validation, new_id_validation, "clear-all operation validation")

old_effect = '''    fn clear_all_affected_days(
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
                self.session.active_session_started_at_utc.ok_or_else(|| {
                    "active session is missing its UTC start timestamp".to_string()
                })?,
                interval.ended_at_utc,
                interval.elapsed_seconds,
                policy,
            )?
            .into_iter()
            .map(|slice| slice.operational_day),
        );
        Ok(days)
    }
'''
new_effect = '''    fn clear_all_effect(
        &self,
        applied_at_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Result<(BTreeSet<NaiveDate>, usize), String> {
        let interval = self.reconciled_active_interval(applied_at_utc, clock_mode)?;
        let previous_started_at_utc = self
            .session
            .active_session_started_at_utc
            .ok_or_else(|| "active session is missing its UTC start timestamp".to_string())?;
        let days = clear_all_affected_days_for_interval(
            operational_day_key_for_utc(applied_at_utc),
            is_drift_category_id(self.time_tracker.active_category_id()),
            previous_started_at_utc,
            interval.ended_at_utc,
            interval.elapsed_seconds,
            OperationalDayPolicy::from_config(self.runtime_settings.day_boundary),
        )?;
        Ok((days, interval.elapsed_seconds))
    }
'''
app = replace_once(app, old_effect, new_effect, "clear-all effect")

app = replace_once(
    app,
    '''        let affected_days = match self.clear_all_affected_days(applied_at_utc, clock_mode) {
            Ok(days) => days,''',
    '''        let (affected_days, previous_elapsed_seconds) =
            match self.clear_all_effect(applied_at_utc, clock_mode) {
                Ok(effect) => effect,''',
    "clear-all effect call",
)
# The replacement adds one indentation level to only one match arm; normalize the closing block.
app = replace_once(
    app,
    '''            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::SandStateSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return;
            }
        };
        let previous_tracker = self.time_tracker.clone();''',
    '''                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::SandStateSave,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return;
                }
            };
        let previous_tracker = self.time_tracker.clone();''',
    "clear-all effect match formatting",
)

old_receipt_build = '''        let prior_identity = format!(
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
                if idle_reset {
                    "idle-reset"
                } else {
                    "active-preserved"
                },
            ),
            applied_at_utc,
            previous_active,
            resulting_active,
            idle_reset,
            affected_operational_days: affected_days
                .iter()
                .map(|day| day.format("%Y-%m-%d").to_string())
                .collect(),
        };'''
new_receipt_build = '''        let affected_operational_days = affected_days
            .iter()
            .map(|day| day.format("%Y-%m-%d").to_string())
            .collect::<Vec<_>>();
        let operation_id = clear_all_operation_id(
            &previous_active,
            applied_at_utc,
            idle_reset,
            previous_elapsed_seconds,
            &affected_operational_days,
        );
        let receipt = ClearAllReceipt {
            version: ClearAllReceipt::VERSION,
            operation_id,
            applied_at_utc,
            previous_active,
            resulting_active,
            idle_reset,
            previous_elapsed_seconds,
            affected_operational_days,
        };'''
app = replace_once(app, old_receipt_build, new_receipt_build, "clear-all receipt construction")

old_legacy_replay = '''        if self.sqlite_database_path.is_none() {
            storage::save_sand_state(&storage::get_sand_state_path(), &checkpoint.sand_state)?;
            for value in &receipt.affected_operational_days {'''
new_legacy_replay = '''        if self.sqlite_database_path.is_none() {
            let valid_category_ids = self
                .time_tracker
                .categories_for_storage()
                .into_iter()
                .chain(self.archived_categories.iter().cloned())
                .map(|category| category.id)
                .collect::<HashSet<_>>();
            self.sand_engine
                .restore_state(&checkpoint.sand_state, &valid_category_ids);
            stage_clear_all_active_state(
                &mut self.time_tracker,
                &mut self.session.active_session_started_at_utc,
                &receipt,
            )?;
            storage::save_sand_state(&storage::get_sand_state_path(), &checkpoint.sand_state)?;
            for value in &receipt.affected_operational_days {'''
app = replace_once(app, old_legacy_replay, new_legacy_replay, "legacy clear-all replay state")

# Targeted pure proofs for cross-day authority, receipt binding, and replay state.
test_anchor = "#[cfg(test)]\nmod bounded_checkpoint_tests {"
clear_tests = r'''#[cfg(test)]
mod clear_all_replay_tests {
    use chrono::{TimeZone, Utc};
    use ratatui::style::Color;

    use super::{
        ClearAllReceipt, DetachedRuntimeCheckpoint, LegacyActiveReceipt,
        clear_all_affected_days_for_interval, clear_all_operation_id,
        stage_clear_all_active_state, validate_clear_all_checkpoint,
    };
    use crate::{
        domain::{Category, CategoryId, OperationalDayPolicy, TimeTracker},
        sand::{SandState, SandStateGrain},
    };

    fn categories() -> Vec<Category> {
        vec![
            Category {
                id: CategoryId::new(0),
                name: "idle".to_string(),
                color: Color::White,
                description: String::new(),
                karma_effect: 0,
            },
            Category {
                id: CategoryId::new(1),
                name: "Work".to_string(),
                color: Color::Blue,
                description: "focus".to_string(),
                karma_effect: 1,
            },
        ]
    }

    fn receipt(idle_reset: bool) -> ClearAllReceipt {
        let previous_started_at_utc = Utc.with_ymd_and_hms(2026, 8, 1, 23, 0, 0).unwrap();
        let applied_at_utc = Utc.with_ymd_and_hms(2026, 8, 2, 1, 0, 0).unwrap();
        let previous_active = LegacyActiveReceipt {
            category_id: if idle_reset { 0 } else { 1 },
            description: if idle_reset { "" } else { "focus" }.to_string(),
            started_at_utc: previous_started_at_utc,
        };
        let affected_operational_days = if idle_reset {
            vec!["2026-08-01".to_string(), "2026-08-02".to_string()]
        } else {
            vec!["2026-08-02".to_string()]
        };
        let previous_elapsed_seconds = 7_200;
        ClearAllReceipt {
            version: ClearAllReceipt::VERSION,
            operation_id: clear_all_operation_id(
                &previous_active,
                applied_at_utc,
                idle_reset,
                previous_elapsed_seconds,
                &affected_operational_days,
            ),
            applied_at_utc,
            resulting_active: LegacyActiveReceipt {
                category_id: previous_active.category_id,
                description: previous_active.description.clone(),
                started_at_utc: if idle_reset {
                    applied_at_utc
                } else {
                    previous_started_at_utc
                },
            },
            previous_active,
            idle_reset,
            previous_elapsed_seconds,
            affected_operational_days,
        }
    }

    fn checkpoint(receipt: ClearAllReceipt) -> DetachedRuntimeCheckpoint {
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: receipt.applied_at_utc,
            simulation_time_utc: receipt.applied_at_utc,
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: receipt.resulting_active.category_id,
            active_description: receipt.resulting_active.description.clone(),
            active_session_started_at_utc: Some(receipt.resulting_active.started_at_utc),
            sand_state: SandState {
                version: SandState::VERSION,
                grid_width: 3,
                grid_height: 5,
                grains: Vec::new(),
                frame_count: 9,
                sweep_left_to_right: true,
                rng_state: 7,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
            legacy_transition: None,
            legacy_finish: None,
            clear_all: Some(receipt),
        }
    }

    #[test]
    fn idle_cross_day_effect_names_every_touched_day() {
        let days = clear_all_affected_days_for_interval(
            Utc.with_ymd_and_hms(2026, 8, 3, 1, 0, 0)
                .unwrap()
                .date_naive(),
            true,
            Utc.with_ymd_and_hms(2026, 8, 1, 23, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 8, 3, 1, 0, 0).unwrap(),
            93_600,
            OperationalDayPolicy {
                utc_offset_seconds: 0,
                start_minutes: 0,
            },
        )
        .unwrap();
        assert_eq!(
            days.into_iter().collect::<Vec<_>>(),
            vec![
                Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0)
                    .unwrap()
                    .date_naive(),
                Utc.with_ymd_and_hms(2026, 8, 2, 0, 0, 0)
                    .unwrap()
                    .date_naive(),
                Utc.with_ymd_and_hms(2026, 8, 3, 0, 0, 0)
                    .unwrap()
                    .date_naive(),
            ]
        );
    }

    #[test]
    fn non_idle_effect_names_only_operation_day() {
        let operation_day = Utc
            .with_ymd_and_hms(2026, 8, 3, 0, 0, 0)
            .unwrap()
            .date_naive();
        let days = clear_all_affected_days_for_interval(
            operation_day,
            false,
            Utc.with_ymd_and_hms(2026, 8, 1, 23, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2026, 8, 3, 1, 0, 0).unwrap(),
            93_600,
            OperationalDayPolicy {
                utc_offset_seconds: 0,
                start_minutes: 0,
            },
        )
        .unwrap();
        assert_eq!(days.into_iter().collect::<Vec<_>>(), vec![operation_day]);
    }

    #[test]
    fn receipt_identity_binds_elapsed_days_and_empty_sediment() {
        let receipt = receipt(false);
        let checkpoint = checkpoint(receipt.clone());
        validate_clear_all_checkpoint(&checkpoint, &receipt).unwrap();

        let mut changed_days = receipt.clone();
        changed_days.affected_operational_days = vec!["2026-08-01".to_string()];
        assert!(
            validate_clear_all_checkpoint(&checkpoint, &changed_days)
                .unwrap_err()
                .contains("operation ID")
        );

        let mut changed_elapsed = receipt.clone();
        changed_elapsed.previous_elapsed_seconds += 1;
        assert!(
            validate_clear_all_checkpoint(&checkpoint, &changed_elapsed)
                .unwrap_err()
                .contains("operation ID")
        );

        let mut non_empty = checkpoint;
        non_empty.sand_state.grains.push(SandStateGrain {
            x: 0,
            y: 0,
            category_id: 1,
        });
        assert!(
            validate_clear_all_checkpoint(&non_empty, &receipt)
                .unwrap_err()
                .contains("non-empty")
        );
    }

    #[test]
    fn replay_stages_exact_resulting_active_interval() {
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(categories(), 2, Vec::new(), 1);
        let mut started_at_utc = None;
        let non_idle = receipt(false);
        stage_clear_all_active_state(&mut tracker, &mut started_at_utc, &non_idle).unwrap();
        assert_eq!(tracker.active_category_id(), CategoryId::new(1));
        assert_eq!(started_at_utc, Some(non_idle.resulting_active.started_at_utc));
        assert!(tracker.current_elapsed().unwrap().as_secs() as usize >= non_idle.previous_elapsed_seconds);
        assert!(
            tracker.current_elapsed().unwrap().as_secs() as usize
                <= non_idle.previous_elapsed_seconds.saturating_add(1)
        );

        let idle = receipt(true);
        stage_clear_all_active_state(&mut tracker, &mut started_at_utc, &idle).unwrap();
        assert_eq!(tracker.active_category_id(), CategoryId::new(0));
        assert_eq!(started_at_utc, Some(idle.applied_at_utc));
        assert!(tracker.current_elapsed().unwrap().as_secs() <= 1);
    }
}

'''
if test_anchor not in app:
    raise SystemExit("clear-all test insertion anchor missing")
app = app.replace(test_anchor, clear_tests + test_anchor, 1)
app_path.write_text(app)
