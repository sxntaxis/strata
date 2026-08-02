from pathlib import Path

path = Path("src/app.rs")
text = path.read_text()

text = text.replace("    path::PathBuf,", "    path::{Path, PathBuf},")

impl_anchor = '''impl DetachedRuntimeCheckpoint {
    const VERSION: u8 = 3;
    const PREVIOUS_VERSION: u8 = 2;
    const LEGACY_VERSION: u8 = 1;
}
'''
helpers = '''impl DetachedRuntimeCheckpoint {
    const VERSION: u8 = 3;
    const PREVIOUS_VERSION: u8 = 2;
    const LEGACY_VERSION: u8 = 1;
}

fn transition_operation_id(
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

fn validate_legacy_switch_checkpoint(
    checkpoint: &DetachedRuntimeCheckpoint,
    receipt: &LegacyTransitionReceipt,
) -> Result<(), String> {
    if checkpoint.schema_version != DetachedRuntimeCheckpoint::VERSION {
        return Err(format!(
            "legacy transition receipt requires checkpoint schema {}, found {}; evidence retained",
            DetachedRuntimeCheckpoint::VERSION,
            checkpoint.schema_version
        ));
    }
    receipt.validate_switch_boundaries()?;
    let expected_identity = format!(
        "legacy:{}:{}",
        receipt.expected_previous_category_id,
        receipt
            .expected_previous_started_at_utc
            .to_rfc3339_opts(SecondsFormat::Nanos, true)
    );
    let expected_operation_id = transition_operation_id(
        "legacy-switch",
        &expected_identity,
        receipt.transition_at_utc,
        &receipt.resulting_active.category_id.to_string(),
    );
    if receipt.operation_id != expected_operation_id {
        return Err(format!(
            "legacy switch receipt operation ID {} is inconsistent; evidence retained",
            receipt.operation_id
        ));
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
    Ok(())
}

fn publish_legacy_switch_replay(
    tracker: &TimeTracker,
    archived_categories: &[Category],
    checkpoint: &mut DetachedRuntimeCheckpoint,
    receipt: &LegacyTransitionReceipt,
    sessions_path: &Path,
    categories_path: &Path,
    checkpoint_path: &Path,
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
    catalog.extend(archived_categories.iter().cloned());
    storage::save_sessions_to_csv(sessions_path, &staged_tracker.sessions, &catalog)?;
    storage::save_category_catalog_to_csv(
        categories_path,
        &staged_tracker.categories_for_storage(),
        archived_categories,
    )?;

    checkpoint.legacy_transition = None;
    checkpoint.schema_version = DetachedRuntimeCheckpoint::VERSION;
    storage::write_json_atomic(checkpoint_path, checkpoint)?;
    Ok(staged_tracker)
}
'''
if impl_anchor not in text:
    raise SystemExit("checkpoint impl anchor not found")
text = text.replace(impl_anchor, helpers, 1)

text = text.replace("self.transition_operation_id(", "transition_operation_id(")

method = '''    fn transition_operation_id(
        &self,
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
if method not in text:
    raise SystemExit("transition operation method anchor not found")
text = text.replace(method, "", 1)

start = text.index("    fn reconcile_legacy_transition_receipt(\n")
end = text.index("\n    fn restore_from_detached_checkpoint", start)
replacement = '''    fn reconcile_legacy_transition_receipt(
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
text = text[:start] + replacement + text[end:]

module = r'''

#[cfg(test)]
mod legacy_switch_replay_tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use chrono::{TimeZone, Utc};
    use ratatui::style::Color;

    use super::{
        DetachedRuntimeCheckpoint, publish_legacy_switch_replay, transition_operation_id,
        validate_legacy_switch_checkpoint,
    };
    use crate::{
        domain::{
            Category, CategoryId, OperationalDayPolicy, Session, TimeTracker,
            DRIFT_CATEGORY_ID,
        },
        legacy_transition::{
            LegacyActiveReceipt, LegacySessionReceipt, LegacyTransitionKind,
            LegacyTransitionReceipt,
        },
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
            color: Color::White,
            description: description.to_string(),
            karma_effect: if id == DRIFT_CATEGORY_ID.0 { 0 } else { 1 },
        }
    }

    fn categories(before_switch: bool) -> Vec<Category> {
        vec![
            category(DRIFT_CATEGORY_ID.0, "idle", ""),
            category(1, "Previous", if before_switch { "focus" } else { "" }),
            category(2, "Next", "next task"),
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

    fn receipt() -> LegacyTransitionReceipt {
        let previous_start = Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap();
        let transition = Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap();
        let expected_identity = format!(
            "legacy:1:{}",
            previous_start.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        );
        LegacyTransitionReceipt {
            version: LegacyTransitionReceipt::VERSION,
            operation_id: transition_operation_id(
                "legacy-switch",
                &expected_identity,
                transition,
                "2",
            ),
            kind: LegacyTransitionKind::Switch,
            expected_previous_category_id: 1,
            expected_previous_started_at_utc: previous_start,
            transition_at_utc: transition,
            completed_session: Some(LegacySessionReceipt::from_session(&completed_session())),
            resulting_active: LegacyActiveReceipt {
                category_id: 2,
                description: "next task".to_string(),
                started_at_utc: transition,
            },
        }
    }

    fn checkpoint(receipt: LegacyTransitionReceipt) -> DetachedRuntimeCheckpoint {
        let transition = receipt.transition_at_utc;
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: transition,
            simulation_time_utc: transition,
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: 2,
            active_description: "next task".to_string(),
            active_session_started_at_utc: Some(transition),
            sand_state: SandState {
                version: SandState::VERSION,
                grid_width: 2,
                grid_height: 4,
                grains: Vec::new(),
                frame_count: 0,
                sweep_left_to_right: true,
                rng_state: 1,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
            legacy_transition: Some(receipt),
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
        checkpoint_path: &std::path::Path,
    ) {
        let loaded_categories = storage::try_load_categories_from_csv(categories_path).unwrap();
        let previous = loaded_categories
            .categories
            .iter()
            .find(|category| category.id == CategoryId::new(1))
            .unwrap();
        let next = loaded_categories
            .categories
            .iter()
            .find(|category| category.id == CategoryId::new(2))
            .unwrap();
        assert_eq!(previous.description, "");
        assert_eq!(next.description, "next task");

        let mut catalog = loaded_categories.categories.clone();
        catalog.extend(loaded_categories.archived_categories.iter().cloned());
        let loaded_sessions = storage::try_load_sessions_from_csv(sessions_path, &catalog).unwrap();
        assert_eq!(loaded_sessions.sessions.len(), 1);
        assert_eq!(loaded_sessions.sessions[0].id, 1);
        assert_eq!(loaded_sessions.sessions[0].elapsed_seconds, 3600);

        let checkpoint: DetachedRuntimeCheckpoint = storage::read_json(checkpoint_path).unwrap();
        assert!(checkpoint.legacy_transition.is_none());
    }

    #[test]
    fn every_persisted_switch_kill_point_converges_without_duplicate_time() {
        for phase in 0..3 {
            let dir = unique_dir(&format!("legacy-switch-phase-{phase}"));
            fs::create_dir_all(&dir).unwrap();
            let categories_path = dir.join("categories.csv");
            let sessions_path = dir.join("time_log.csv");
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
            storage::write_json_atomic(&checkpoint_path, &checkpoint).unwrap();

            let tracker = load_tracker(&categories_path, &sessions_path);
            let mut loaded_checkpoint: DetachedRuntimeCheckpoint =
                storage::read_json(&checkpoint_path).unwrap();
            validate_legacy_switch_checkpoint(&loaded_checkpoint, &receipt).unwrap();
            let replayed = publish_legacy_switch_replay(
                &tracker,
                &[],
                &mut loaded_checkpoint,
                &receipt,
                &sessions_path,
                &categories_path,
                &checkpoint_path,
            )
            .unwrap();
            assert_eq!(replayed.sessions.len(), 1);
            assert_converged(&categories_path, &sessions_path, &checkpoint_path);
            fs::remove_dir_all(dir).ok();
        }
    }

    #[test]
    fn failed_catalog_publication_retains_receipt_after_session_converges() {
        let dir = unique_dir("legacy-switch-catalog-failure");
        fs::create_dir_all(&dir).unwrap();
        let categories_path = dir.join("categories-as-directory");
        let sessions_path = dir.join("time_log.csv");
        let checkpoint_path = dir.join("detached_runtime.json");
        fs::create_dir_all(&categories_path).unwrap();

        let receipt = receipt();
        let mut checkpoint = checkpoint(receipt.clone());
        storage::write_json_atomic(&checkpoint_path, &checkpoint).unwrap();
        let mut tracker = TimeTracker::new();
        tracker.apply_loaded_state(categories(true), 3, Vec::new(), 1);

        let error = publish_legacy_switch_replay(
            &tracker,
            &[],
            &mut checkpoint,
            &receipt,
            &sessions_path,
            &categories_path,
            &checkpoint_path,
        )
        .unwrap_err();
        assert!(!error.is_empty());

        let disk_checkpoint: DetachedRuntimeCheckpoint =
            storage::read_json(&checkpoint_path).unwrap();
        assert!(disk_checkpoint.legacy_transition.is_some());
        let loaded_sessions =
            storage::try_load_sessions_from_csv(&sessions_path, &categories(true)).unwrap();
        assert_eq!(loaded_sessions.sessions.len(), 1);
        fs::remove_dir_all(dir).ok();
    }
}
'''
text += module
path.write_text(text)

Path("tools/reconciliation001b2a-replay-proof.py").unlink()
Path(".github/workflows/reconciliation001b2a-replay-proof.yml").unlink()
