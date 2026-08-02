from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


# File-authority path: new typed contribution files sit beside untouched legacy daily evidence.
replace_once(
    "src/storage.rs",
    "pub fn get_sand_history_path_for_day(day: NaiveDate) -> PathBuf {\n    let filename = format!(\"{}.json\", day.format(\"%Y-%m-%d\"));\n    get_sand_history_dir().join(filename)\n}\n",
    "pub fn get_sand_history_path_for_day(day: NaiveDate) -> PathBuf {\n    let filename = format!(\"{}.json\", day.format(\"%Y-%m-%d\"));\n    get_sand_history_dir().join(filename)\n}\n\npub fn get_sand_contribution_path_for_day(day: NaiveDate) -> PathBuf {\n    let filename = format!(\"{}.contribution.json\", day.format(\"%Y-%m-%d\"));\n    get_sand_history_dir().join(filename)\n}\n",
)
replace_once(
    "src/storage.rs",
    "    fn test_sand_history_path_for_day_uses_expected_filename() {",
    "    fn test_sand_history_path_for_day_uses_expected_filename() {",
)
storage = Path("src/storage.rs")
text = storage.read_text()
anchor = "    #[test]\n    fn test_sand_state_round_trip() {\n"
proof = '''    #[test]
    fn test_sand_contribution_path_is_distinct_from_legacy_history() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
        let legacy = get_sand_history_path_for_day(day);
        let contribution = get_sand_contribution_path_for_day(day);
        assert_ne!(legacy, contribution);
        assert!(
            contribution
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".contribution.json")
        );
    }

'''
if text.count(anchor) != 1:
    raise SystemExit("storage test anchor not found")
storage.write_text(text.replace(anchor, proof + anchor, 1))

# App persistence boundary now owns typed daily artifacts rather than cumulative live-state copies.
category = Path("src/app/category_state.rs")
text = category.read_text()
text = text.replace(
    "        CategoryId, DRIFT_CATEGORY_ID, is_drift_category_id, operational_day_key_now,\n",
    "        CategoryId, DRIFT_CATEGORY_ID, operational_day_key_now,\n",
    1,
)
text = text.replace(
    "    pub(super) fn persist_daily_sand_snapshot(&mut self) {\n        let mut state = self.sand_engine.snapshot_state();\n        if is_drift_category_id(self.time_tracker.active_category_id()) {\n            state.grains.retain(|grain| grain.category_id != 0);\n        }\n        self.save_daily_sand_snapshot(operational_day_key_now(), &state);\n    }",
    "    pub(super) fn persist_daily_sand_snapshot(&mut self) {\n        self.reconcile_all_daily_contributions();\n    }",
    1,
)
start = text.index("    pub(super) fn load_daily_sand_snapshot(")
end = text.index("    pub(super) fn sync_modal_description_from_selection", start)
replacement = '''    pub(super) fn load_daily_sediment_snapshot(
        &mut self,
        day: NaiveDate,
    ) -> Option<crate::sand::SedimentSnapshot> {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let day = day.format("%Y-%m-%d").to_string();
            match sqlite::load_tui_daily_snapshot(&database_path, &day) {
                Ok(value) => value,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::StateReload,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    None
                }
            }
        } else {
            let path = storage::get_sand_contribution_path_for_day(day);
            if !storage::file_exists(&path) {
                return None;
            }
            match storage::read_json::<crate::sand::SedimentSnapshot>(&path) {
                Ok(snapshot) => Some(snapshot),
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::StateReload,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    None
                }
            }
        }
    }

    pub(super) fn save_daily_sediment_snapshot(
        &mut self,
        day: NaiveDate,
        snapshot: &crate::sand::SedimentSnapshot,
    ) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let day = day.format("%Y-%m-%d").to_string();
            let result = sqlite::save_tui_daily_snapshot(&database_path, &day, snapshot);
            self.record_storage_result_for(
                PersistenceOperation::DailySnapshotSave,
                RecoveryAction::FlushCurrentState,
                result,
            );
        } else {
            let path = storage::get_sand_contribution_path_for_day(day);
            if let Err(error) = storage::write_json_atomic(&path, snapshot) {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::DailySnapshotSave,
                    RecoveryAction::FlushCurrentState,
                    Err(error),
                );
            }
        }
    }

    pub(super) fn delete_daily_sediment_snapshot(&mut self, day: NaiveDate) {
        if let Some(database_path) = self.sqlite_database_path.clone() {
            let day = day.format("%Y-%m-%d").to_string();
            let result = sqlite::delete_tui_daily_snapshot(&database_path, &day);
            self.record_storage_result_for(
                PersistenceOperation::DailySnapshotDelete,
                RecoveryAction::FlushCurrentState,
                result,
            );
        } else {
            let path = storage::get_sand_contribution_path_for_day(day);
            if let Err(error) = storage::delete_file_if_exists(&path) {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::DailySnapshotDelete,
                    RecoveryAction::FlushCurrentState,
                    Err(error),
                );
            }
        }
    }

'''
category.write_text(text[:start] + replacement + text[end:])

# Restore explicit delete operation because D2 deletes only typed contributions, never legacy evidence.
persistence = Path("src/app/persistence_recovery.rs")
text = persistence.read_text()
text = text.replace("    DailySnapshotSave,\n", "    DailySnapshotSave,\n    DailySnapshotDelete,\n", 1)
text = text.replace(
    '            Self::DailySnapshotSave => "daily sediment snapshot save",\n',
    '            Self::DailySnapshotSave => "daily sediment snapshot save",\n            Self::DailySnapshotDelete => "daily sediment snapshot deletion",\n',
    1,
)
text = text.replace(
    "        CategoryId, DRIFT_CATEGORY_ID, is_drift_category_id, operational_day_key_now,\n",
    "        CategoryId, DRIFT_CATEGORY_ID, operational_day_key_now,\n",
    1,
)
old_flush_start = '''        let categories = self.time_tracker.categories_for_storage();
        let mut state = self.sand_engine.snapshot_state();
        if is_drift_category_id(self.time_tracker.active_category_id()) {
            state.grains.retain(|grain| grain.category_id != 0);
        }
        let operational_day = operational_day_key_now().format("%Y-%m-%d").to_string();
'''
new_flush_start = '''        let categories = self.time_tracker.categories_for_storage();
        let state = self.sand_engine.snapshot_state();
        let operational_day_date = operational_day_key_now();
        let operational_day = operational_day_date.format("%Y-%m-%d").to_string();
        let daily_contribution = self.daily_contribution_from_time_log(operational_day_date);
'''
if text.count(old_flush_start) != 1:
    raise SystemExit("persistence flush start not found")
text = text.replace(old_flush_start, new_flush_start, 1)
text = text.replace(
    "            sqlite::save_tui_daily_snapshot(&database_path, &operational_day, &state)?;",
    "            if let Some(snapshot) = daily_contribution.as_ref() {\n                sqlite::save_tui_daily_snapshot(&database_path, &operational_day, snapshot)?;\n            } else {\n                sqlite::delete_tui_daily_snapshot(&database_path, &operational_day)?;\n            }",
    1,
)
text = text.replace(
    "            storage::save_sand_state(\n                &storage::get_sand_history_path_for_day(operational_day_key_now()),\n                &state,\n            )?;",
    "            let contribution_path =\n                storage::get_sand_contribution_path_for_day(operational_day_date);\n            if let Some(snapshot) = daily_contribution.as_ref() {\n                storage::write_json_atomic(&contribution_path, snapshot)?;\n            } else {\n                storage::delete_file_if_exists(&contribution_path)?;\n            }",
    1,
)
persistence.write_text(text)

# Report derivation and invalidation use one exact ledger-slice builder.
report = Path("src/app/report_state.rs")
text = report.read_text()
text = text.replace(
    "use std::fmt::Write as _;",
    "use std::collections::BTreeSet;",
    1,
)
text = text.replace(
    "use crate::sand::{\n    SandState, SandStateGrain, SedimentSnapshot, select_daily_artifact, stable_source_revision,\n};",
    "use crate::sand::{\n    DailySedimentSlice, SedimentSnapshot, daily_contribution_from_slices,\n    derived_preview_from_slices, select_daily_artifact,\n};",
    1,
)
text = text.replace(
    "        let persisted = self\n            .load_daily_sand_snapshot(end_day)\n            .map(|state| SedimentSnapshot::legacy_daily_payload(key.clone(), state));",
    "        let persisted = self.load_daily_sediment_snapshot(end_day);",
    1,
)
# Capture all days touched by a canonical session before deletion.
old_delete_prelude = '''        let removed_seconds = row.elapsed_seconds;
        let Some(category_id) = self.report_logs_category_id else {
            return false;
        };
'''
new_delete_prelude = '''        let removed_seconds = row.elapsed_seconds;
        let Some(category_id) = self.report_logs_category_id else {
            return false;
        };
        let affected_days = self
            .time_tracker
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| {
                session_slices(session)
                    .into_iter()
                    .map(|slice| slice.operational_day)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
'''
if text.count(old_delete_prelude) != 1:
    raise SystemExit("report deletion prelude not found")
text = text.replace(old_delete_prelude, new_delete_prelude, 1)
text = text.replace(
    "        self.rebuild_report_snapshot_for_interval_end_day();\n",
    "        for day in affected_days {\n            self.reconcile_daily_contribution(day);\n        }\n        self.clear_report_snapshot_cache();\n",
    1,
)
start = text.index("    pub(super) fn rebuild_report_snapshot_for_interval_end_day")
end = text.index("    pub(super) fn clamp_report_selection", start)
replacement = '''    pub(super) fn daily_contribution_from_time_log(
        &self,
        day: NaiveDate,
    ) -> Option<SedimentSnapshot> {
        let slices = self.daily_sediment_slices(day);
        let day_key = day.format("%Y-%m-%d").to_string();
        daily_contribution_from_slices(
            &day_key,
            self.sand_engine.grid_width_dots,
            self.sand_engine.grid_height_dots,
            &slices,
        )
    }

    fn synthetic_snapshot_from_time_log(&self, day: NaiveDate) -> Option<SedimentSnapshot> {
        let slices = self.daily_sediment_slices(day);
        let day_key = day.format("%Y-%m-%d").to_string();
        derived_preview_from_slices(
            &day_key,
            self.sand_engine.grid_width_dots,
            self.sand_engine.grid_height_dots,
            &slices,
        )
    }

    fn daily_sediment_slices(&self, day: NaiveDate) -> Vec<DailySedimentSlice> {
        let mut slices = self
            .time_tracker
            .sessions
            .iter()
            .flat_map(|session| {
                session_slices(session)
                    .into_iter()
                    .filter(move |slice| slice.operational_day == day)
                    .map(move |slice| DailySedimentSlice {
                        category_id: session.category_id.0,
                        elapsed_seconds: slice.elapsed_seconds,
                        start_time: slice.start_time,
                        end_time: slice.end_time,
                        session_id: session.id,
                    })
            })
            .collect::<Vec<_>>();

        if let Some(preview) = self.live_preview_session() {
            slices.extend(
                session_slices(&preview)
                    .into_iter()
                    .filter(|slice| slice.operational_day == day)
                    .map(|slice| DailySedimentSlice {
                        category_id: preview.category_id.0,
                        elapsed_seconds: slice.elapsed_seconds,
                        start_time: slice.start_time,
                        end_time: slice.end_time,
                        session_id: usize::MAX,
                    }),
            );
        }
        slices
    }

    fn live_preview_session(&self) -> Option<crate::domain::Session> {
        let day = operational_day_key_now();
        let live = self.live_session_preview()?;
        Some(crate::domain::Session {
            id: usize::MAX,
            date: day.format("%Y-%m-%d").to_string(),
            category_id: live.category_id,
            project: String::new(),
            description: live.description,
            start_time: String::new(),
            end_time: String::new(),
            elapsed_seconds: live.elapsed_seconds,
            started_at_utc: Some(live.started_at_utc),
            ended_at_utc: Some(live.ended_at_utc),
            operational_day_policy: Some(live.operational_day_policy),
        })
    }

    pub(super) fn daily_contribution_days(&self) -> BTreeSet<NaiveDate> {
        let mut days = self
            .time_tracker
            .sessions
            .iter()
            .flat_map(session_slices)
            .map(|slice| slice.operational_day)
            .collect::<BTreeSet<_>>();
        if let Some(preview) = self.live_preview_session() {
            days.extend(
                session_slices(&preview)
                    .into_iter()
                    .map(|slice| slice.operational_day),
            );
        }
        days
    }

    pub(super) fn reconcile_all_daily_contributions(&mut self) {
        let days = self.daily_contribution_days();
        for day in days {
            self.reconcile_daily_contribution(day);
            if self.has_persistence_recovery() {
                break;
            }
        }
    }

    pub(super) fn reconcile_daily_contribution(&mut self, day: NaiveDate) {
        let expected = self.daily_contribution_from_time_log(day);
        let existing = self.load_daily_sediment_snapshot(day);
        if existing == expected {
            return;
        }
        match expected {
            Some(snapshot) => self.save_daily_sediment_snapshot(day, &snapshot),
            None => self.delete_daily_sediment_snapshot(day),
        }
    }

'''
report.write_text(text[:start] + replacement + text[end:])

# SQLite typed persistence uses a new kind; legacy 'daily' rows remain evidence.
tui = Path("src/sqlite/tui_runtime.rs")
text = tui.read_text()
text = text.replace("    sand::SandState,", "    sand::{SandState, SedimentSnapshot},", 1)
text = text.replace(
    "    state: &SandState,\n) -> Result<(), String> {\n    runtime_coordination::maybe_inject_test_fault(\"daily-snapshot\", \"before-write\")",
    "    snapshot: &SedimentSnapshot,\n) -> Result<(), String> {\n    runtime_coordination::maybe_inject_test_fault(\"daily-snapshot\", \"before-write\")",
    1,
)
text = text.replace(
    "    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;",
    "    let payload_json = serde_json::to_string(snapshot).map_err(|error| error.to_string())?;",
    1,
)
text = text.replace(
    "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily' AND operational_day = ?1",
    "DELETE FROM sand_snapshots WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1",
    2,
)
text = text.replace(
    "             ) VALUES (?1, 'daily', ?2, ?3, ?4, ?5)",
    "             ) VALUES (?1, 'daily-contribution', ?2, ?3, ?4, ?5)",
    1,
)
text = text.replace(
    ") -> Result<Option<SandState>, String> {",
    ") -> Result<Option<SedimentSnapshot>, String> {",
    1,
)
text = text.replace(
    "             WHERE snapshot_kind = 'daily' AND operational_day = ?1",
    "             WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1",
    1,
)
# Recovery wrapper includes a typed daily contribution.
text = text.replace(
    "    state: &SandState,\n) -> Result<(), String> {",
    "    state: &SandState,\n    daily_contribution: &SedimentSnapshot,\n) -> Result<(), String> {",
    1,
)
text = text.replace(
    "    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;\n    runtime_coordination::commit_checkpoint_recovery(",
    "    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;\n    let daily_payload_json =\n        serde_json::to_string(daily_contribution).map_err(|error| error.to_string())?;\n    runtime_coordination::commit_checkpoint_recovery(",
    1,
)
text = text.replace(
    "        },\n        &now,\n    )",
    "        },\n        &daily_payload_json,\n        &now,\n    )",
    1,
)
# Update runtime-state test and prove legacy row retention.
text = text.replace(
    "        save_daily_snapshot(&path, \"2026-08-01\", &state).unwrap();",
    "        let daily = SedimentSnapshot::daily_contribution(\n            \"2026-08-01\".to_string(),\n            \"revision-a\".to_string(),\n            state.clone(),\n        );\n        save_daily_snapshot(&path, \"2026-08-01\", &daily).unwrap();",
    1,
)
text = text.replace(
    "            load_daily_snapshot(&path, \"2026-08-01\").unwrap(),\n            Some(state.clone())",
    "            load_daily_snapshot(&path, \"2026-08-01\").unwrap(),\n            Some(daily.clone())",
    1,
)
# Insert legacy retention proof before checkpoint load in test.
anchor = "        let checkpoint: Option<SqliteClaimedCheckpoint<BTreeMap<String, String>>> =\n"
proof = '''        let repository = open_cli_repository(&path).unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO sand_snapshots (
                    formation_id, snapshot_kind, operational_day, quantum_seconds,
                    payload_json, captured_at_utc
                 ) VALUES ('default', 'daily', '2026-08-01', 1, '{}', '2026-08-01T12:00:00Z')",
                [],
            )
            .unwrap();
        drop(repository);
        save_daily_snapshot(&path, "2026-08-01", &daily).unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let legacy_count: i64 = repository
            .connection
            .query_row(
                "SELECT count(*) FROM sand_snapshots
                 WHERE snapshot_kind = 'daily' AND operational_day = '2026-08-01'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_count, 1, "legacy daily evidence must remain untouched");
        drop(repository);

'''
if text.count(anchor) != 1:
    raise SystemExit("tui checkpoint test anchor not found")
tui.write_text(text.replace(anchor, proof + anchor, 1))

# Atomic checkpoint recovery publishes the typed contribution, not cumulative state under a daily key.
coord = Path("src/sqlite/runtime_coordination.rs")
text = coord.read_text()
text = text.replace(
    "    state: &SandStateRecord,\n    captured_at_utc: &str,",
    "    state: &SandStateRecord,\n    daily_payload_json: &str,\n    captured_at_utc: &str,",
    1,
)
text = text.replace(
    "         WHERE snapshot_kind = 'daily' AND operational_day = ?1",
    "         WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1",
    1,
)
text = text.replace(
    "         ) VALUES (?1, 'daily', ?2, ?3, ?4, ?5, NULL)",
    "         ) VALUES (?1, 'daily-contribution', ?2, ?3, ?4, ?5, NULL)",
    1,
)
# In the snapshot insert params, use typed payload rather than canonical state payload.
old_params = '''        params![
            state.formation_id,
            operational_day,
            state.quantum_seconds,
            state.payload_json,
            captured_at_utc,
        ],
'''
new_params = '''        params![
            state.formation_id,
            operational_day,
            state.quantum_seconds,
            daily_payload_json,
            captured_at_utc,
        ],
'''
if text.count(old_params) != 1:
    raise SystemExit("checkpoint snapshot params not found")
coord.write_text(text.replace(old_params, new_params, 1))

# Application recovery supplies the ledger-derived contribution to the atomic transaction.
app = Path("src/app.rs")
text = app.read_text()
old_day = '''        let state = self.sand_engine.snapshot_state();
        let operational_day = operational_day_key_for_utc(self.simulation.simulation_time_utc)
            .format("%Y-%m-%d")
            .to_string();
'''
new_day = '''        let state = self.sand_engine.snapshot_state();
        let operational_day_date = operational_day_key_for_utc(self.simulation.simulation_time_utc);
        let operational_day = operational_day_date.format("%Y-%m-%d").to_string();
        let Some(daily_contribution) =
            self.daily_contribution_from_time_log(operational_day_date)
        else {
            self.record_storage_result::<()>(Err(
                "checkpoint recovery produced no daily contribution for its active session"
                    .to_string(),
            ));
            return;
        };
'''
if text.count(old_day) != 1:
    raise SystemExit("checkpoint operational day block not found")
text = text.replace(old_day, new_day, 1)
text = text.replace(
    "                        &operational_day,\n                        &state,\n                    ),",
    "                        &operational_day,\n                        &state,\n                        &daily_contribution,\n                    ),",
    1,
)
app.write_text(text)

for temporary in [
    ".github/workflows/sediment001d2-apply.yml",
    "tools/sediment001d2-apply.py",
    "tools/sediment001d2.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
