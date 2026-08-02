from pathlib import Path

recovery_path = Path("src/app/persistence_recovery.rs")
recovery = recovery_path.read_text()
old = '''        self.try_flush_current_state()?;
        if let Some(database_path) = self.sqlite_database_path.clone() {
'''
new = '''        self.try_flush_current_state()?;
        self.reconcile_all_daily_contributions();
        if let Some(recovery) = self.persistence_recovery.as_ref() {
            return Err(recovery.failure.summary());
        }
        if let Some(database_path) = self.sqlite_database_path.clone() {
'''
if old not in recovery:
    raise SystemExit("finish retry flush anchor not found")
recovery_path.write_text(recovery.replace(old, new, 1))

app_path = Path("src/app.rs")
app = app_path.read_text()
module = r'''

#[cfg(test)]
mod legacy_finish_replay_tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use chrono::{TimeZone, Utc};
    use ratatui::style::Color;

    use super::{
        DetachedRuntimeCheckpoint, publish_legacy_finish_replay, transition_operation_id,
        validate_legacy_finish_checkpoint,
    };
    use crate::{
        domain::{
            Category, CategoryId, OperationalDayPolicy, Session, TimeTracker,
            DRIFT_CATEGORY_ID,
        },
        legacy_transition::{LegacyFinishReceipt, LegacySessionReceipt},
        sand::SandState,
        storage,
    };

    fn unique_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "strata-{label}-{}-{stamp}",
            std::process::id()
        ))
    }

    fn category(id: u64, name: &str, description: &str) -> Category {
        Category {
            id: CategoryId::new(id),
            name: name.to_string(),
            color: if id == DRIFT_CATEGORY_ID.0 {
                Color::White
            } else {
                crate::constants::COLORS
                    [((id - 1) as usize) % crate::constants::COLORS.len()]
            },
            description: description.to_string(),
            karma_effect: if id == DRIFT_CATEGORY_ID.0 { 0 } else { 1 },
        }
    }

    fn categories(before_finish: bool) -> Vec<Category> {
        vec![
            category(DRIFT_CATEGORY_ID.0, "idle", ""),
            category(1, "Work", if before_finish { "focus" } else { "" }),
        ]
    }

    fn completed_session() -> Session {
        Session {
            id: 1,
            date: "2026-08-02".to_string(),
            category_id: CategoryId::new(1),
            project: String::new(),
            description: "focus".to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap()),
            ended_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap()),
            operational_day_policy: Some(OperationalDayPolicy {
                utc_offset_seconds: -21600,
                start_minutes: 360,
            }),
        }
    }

    fn receipt() -> LegacyFinishReceipt {
        let previous_start = Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap();
        let finished = Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap();
        let expected_identity = format!(
            "legacy:1:{}",
            previous_start.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        );
        LegacyFinishReceipt {
            version: LegacyFinishReceipt::VERSION,
            operation_id: transition_operation_id(
                "legacy-finish",
                &expected_identity,
                finished,
                "complete",
            ),
            expected_previous_category_id: 1,
            expected_previous_description: "focus".to_string(),
            expected_previous_started_at_utc: previous_start,
            finished_at_utc: finished,
            completed_session: Some(LegacySessionReceipt::from_session(&completed_session())),
        }
    }

    fn sand_state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 4,
            grains: Vec::new(),
            frame_count: 17,
            sweep_left_to_right: true,
            rng_state: 19,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        }
    }

    fn checkpoint(receipt: LegacyFinishReceipt) -> DetachedRuntimeCheckpoint {
        let finished = receipt.finished_at_utc;
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: finished,
            simulation_time_utc: finished,
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: 1,
            active_description: "focus".to_string(),
            active_session_started_at_utc: Some(receipt.expected_previous_started_at_utc),
            sand_state: sand_state(),
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
            legacy_transition: None,
            legacy_finish: Some(receipt),
        }
    }

    fn load_tracker(categories_path: &std::path::Path, sessions_path: &std::path::Path) -> TimeTracker {
        let loaded_categories = storage::try_load_categories_from_csv(categories_path).unwrap();
        let mut catalog = loaded_categories.categories.clone();
        catalog.extend(loaded_categories.archived_categories.iter().cloned());
        let loaded_sessions = storage::try_load_sessions_from_csv(sessions_path, &catalog).unwrap();
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(
            loaded_categories.categories,
            loaded_categories.next_category_id,
            loaded_sessions.sessions,
            loaded_sessions.next_session_id,
        );
        tracker
    }

    fn assert_converged(
        categories_path: &std::path::Path,
        sessions_path: &std::path::Path,
        sand_path: &std::path::Path,
    ) {
        let loaded_categories = storage::try_load_categories_from_csv(categories_path).unwrap();
        let work = loaded_categories
            .categories
            .iter()
            .find(|category| category.id == CategoryId::new(1))
            .unwrap();
        assert_eq!(work.description, "");
        let mut catalog = loaded_categories.categories.clone();
        catalog.extend(loaded_categories.archived_categories.iter().cloned());
        let loaded_sessions = storage::try_load_sessions_from_csv(sessions_path, &catalog).unwrap();
        assert_eq!(loaded_sessions.sessions.len(), 1);
        assert_eq!(loaded_sessions.sessions[0].id, 1);
        assert_eq!(loaded_sessions.sessions[0].elapsed_seconds, 3600);
        let persisted_sand = storage::load_sand_state(sand_path).unwrap();
        assert_eq!(persisted_sand.frame_count, 17);
        assert_eq!(persisted_sand.rng_state, 19);
    }

    #[test]
    fn every_persisted_finish_kill_point_converges_without_duplicate_time() {
        for phase in 0..4 {
            let dir = unique_dir(&format!("legacy-finish-phase-{phase}"));
            fs::create_dir_all(&dir).unwrap();
            let categories_path = dir.join("categories.csv");
            let sessions_path = dir.join("time_log.csv");
            let sand_path = dir.join("sand_state.json");
            let checkpoint_path = dir.join("detached_runtime.json");
            let receipt = receipt();
            let checkpoint = checkpoint(receipt.clone());

            let seeded_categories = categories(phase < 2);
            storage::save_category_catalog_to_csv(&categories_path, &seeded_categories, &[])
                .unwrap();
            if phase >= 1 {
                storage::save_sessions_to_csv(
                    &sessions_path,
                    &[completed_session()],
                    &seeded_categories,
                )
                .unwrap();
            }
            if phase >= 3 {
                storage::save_sand_state(&sand_path, &sand_state()).unwrap();
            }
            storage::write_json_atomic(&checkpoint_path, &checkpoint).unwrap();

            let tracker = load_tracker(&categories_path, &sessions_path);
            validate_legacy_finish_checkpoint(&checkpoint, &receipt).unwrap();
            let replayed = publish_legacy_finish_replay(
                &tracker,
                &[],
                &checkpoint,
                &receipt,
                &sessions_path,
                &categories_path,
                &sand_path,
            )
            .unwrap();
            assert_eq!(replayed.sessions.len(), 1);
            assert_converged(&categories_path, &sessions_path, &sand_path);
            let retained: DetachedRuntimeCheckpoint = storage::read_json(&checkpoint_path).unwrap();
            assert!(retained.legacy_finish.is_some());
            fs::remove_dir_all(dir).ok();
        }
    }

    #[test]
    fn failed_finish_catalog_publication_retains_receipt_after_session_converges() {
        let dir = unique_dir("legacy-finish-catalog-failure");
        fs::create_dir_all(&dir).unwrap();
        let categories_path = dir.join("categories-as-directory");
        let sessions_path = dir.join("time_log.csv");
        let sand_path = dir.join("sand_state.json");
        let checkpoint_path = dir.join("detached_runtime.json");
        fs::create_dir_all(&categories_path).unwrap();
        let receipt = receipt();
        let checkpoint = checkpoint(receipt.clone());
        storage::write_json_atomic(&checkpoint_path, &checkpoint).unwrap();
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(categories(true), 2, Vec::new(), 1);

        let error = match publish_legacy_finish_replay(
            &tracker,
            &[],
            &checkpoint,
            &receipt,
            &sessions_path,
            &categories_path,
            &sand_path,
        ) {
            Ok(_) => panic!("catalog publication unexpectedly succeeded"),
            Err(error) => error,
        };
        assert!(!error.is_empty());
        let retained: DetachedRuntimeCheckpoint = storage::read_json(&checkpoint_path).unwrap();
        assert!(retained.legacy_finish.is_some());
        let loaded_sessions =
            storage::try_load_sessions_from_csv(&sessions_path, &categories(true)).unwrap();
        assert_eq!(loaded_sessions.sessions.len(), 1);
        assert!(!sand_path.exists());
        fs::remove_dir_all(dir).ok();
    }
}
'''
app_path.write_text(app + module)

Path("tools/reconciliation001b2b-proof.py").unlink()
Path(".github/workflows/reconciliation001b2b-proof.yml").unlink()
