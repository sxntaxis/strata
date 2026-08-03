from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label} anchor missing")
    return text.replace(old, new, 1)

# ---------------------------------------------------------------------------
# SQLite: enforce empty sediment and certify every transaction kill point.
# ---------------------------------------------------------------------------
sqlite_path = Path("src/sqlite/tui_runtime.rs")
sqlite = sqlite_path.read_text()
sqlite = replace_once(
    sqlite,
    '''    if expected_active_stable_id.trim().is_empty() || resulting_active_stable_id.trim().is_empty() {
        return Err("clear-all requires non-empty active stable identities".to_string());
    }
    let mut repository = open_cli_repository(database_path)?;''',
    '''    if expected_active_stable_id.trim().is_empty() || resulting_active_stable_id.trim().is_empty() {
        return Err("clear-all requires non-empty active stable identities".to_string());
    }
    if !state.grains.is_empty()
        || !state.pending_grains.is_empty()
        || !state.pending_runs.is_empty()
    {
        return Err("clear-all refuses a non-empty sediment payload".to_string());
    }
    let mut repository = open_cli_repository(database_path)?;''',
    "SQLite empty-state validation",
)
additional_tests = r'''

#[cfg(test)]
mod clear_all_additional_transaction_tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::sqlite::{NewActiveSession, SqliteRepository, runtime_coordination};

    fn database_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-clear-all-extra-{label}-{}-{}.sqlite3",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn remove_database(path: &Path) {
        std::fs::remove_file(path).ok();
        std::fs::remove_file(format!("{}-wal", path.display())).ok();
        std::fs::remove_file(format!("{}-shm", path.display())).ok();
    }

    fn state(grains: bool) -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: if grains {
                vec![crate::sand::SandStateGrain {
                    x: 0,
                    y: 1,
                    category_id: 0,
                }]
            } else {
                Vec::new()
            },
            frame_count: 3,
            sweep_left_to_right: true,
            rng_state: 9,
            pending_grains: Vec::new(),
            pending_runs: Vec::new(),
        }
    }

    fn seed(path: &Path) {
        let mut repository = SqliteRepository::open(path).unwrap();
        repository
            .transition_storage_authority("sqlite-candidate", "sqlite-cli", "2026-08-01T12:00:00Z")
            .unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO sessions (
                    stable_id, project, category_id, description, started_at_utc,
                    ended_at_utc, operational_day, elapsed_seconds, source
                 ) VALUES ('completed-idle', '', 0, '', '2026-08-01T10:00:00Z',
                    '2026-08-01T11:00:00Z', '2026-08-01', 3600, 'test')",
                [],
            )
            .unwrap();
        runtime_coordination::start_active_session(
            &mut repository,
            &NewActiveSession {
                stable_id: "active-a",
                project: "",
                category_id: 0,
                description: "",
                started_at_utc: "2026-08-01T11:00:00Z",
                recovery_kind: "live",
            },
        )
        .unwrap();
        drop(repository);
        save_sand_state(path, &state(true)).unwrap();
        let daily = SedimentSnapshot::daily_contribution(
            "2026-08-01".to_string(),
            "before".to_string(),
            state(true),
        );
        save_daily_snapshot(path, "2026-08-01", &daily).unwrap();
        save_checkpoint(
            path,
            "active-a",
            Utc::now(),
            Utc::now(),
            &serde_json::json!({"before": true}),
        )
        .unwrap();
    }

    fn assert_original_state(path: &Path) {
        let repository = open_cli_repository(path).unwrap();
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "active-a"
        );
        assert_eq!(
            repository
                .connection
                .query_row("SELECT count(*) FROM sessions", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(repository);
        assert_eq!(load_sand_state(path).unwrap(), Some(state(true)));
        assert!(load_daily_snapshot(path, "2026-08-01").unwrap().is_some());
    }

    #[test]
    fn every_clear_all_kill_point_rolls_back_all_authorities() {
        for point in [
            "before-write",
            "active",
            "sand",
            "daily",
            "checkpoint",
            "commit",
        ] {
            let path = database_path(point);
            seed(&path);
            let empty = state(false);
            let updates = [("2026-08-01".to_string(), None)];
            let checkpoint = serde_json::json!({"clear_all": true});
            let result = runtime_coordination::with_test_fault("clear-all", point, "io", || {
                clear_all_state(
                    &path,
                    ClearAllStateRequest {
                        expected_active_stable_id: "active-a",
                        resulting_active_stable_id: "active-b",
                        resulting_started_at_utc: Utc::now(),
                        state: &empty,
                        daily_updates: &updates,
                        detached_at_utc: Utc::now(),
                        simulation_time_utc: Utc::now(),
                        checkpoint: &checkpoint,
                    },
                )
            });
            assert!(result.is_err(), "kill point {point} unexpectedly committed");
            assert_original_state(&path);
            remove_database(&path);
        }
    }

    #[test]
    fn non_reset_clear_preserves_active_identity_and_start() {
        let path = database_path("preserve-active");
        seed(&path);
        let repository = open_cli_repository(&path).unwrap();
        let prior_start: String = repository
            .connection
            .query_row(
                "SELECT started_at_utc FROM active_session WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(repository);
        let empty = state(false);
        let updates = [("2026-08-01".to_string(), None)];
        let checkpoint = serde_json::json!({"clear_all": true});
        clear_all_state(
            &path,
            ClearAllStateRequest {
                expected_active_stable_id: "active-a",
                resulting_active_stable_id: "active-a",
                resulting_started_at_utc: Utc::now(),
                state: &empty,
                daily_updates: &updates,
                detached_at_utc: Utc::now(),
                simulation_time_utc: Utc::now(),
                checkpoint: &checkpoint,
            },
        )
        .unwrap();
        let repository = open_cli_repository(&path).unwrap();
        let active = repository.active_session().unwrap().unwrap();
        assert_eq!(active.stable_id, "active-a");
        let resulting_start: String = repository
            .connection
            .query_row(
                "SELECT started_at_utc FROM active_session WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(resulting_start, prior_start);
        assert_eq!(
            repository
                .connection
                .query_row("SELECT count(*) FROM sessions", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn clear_all_refuses_non_empty_resulting_sediment() {
        let path = database_path("non-empty");
        seed(&path);
        let non_empty = state(true);
        let updates = [("2026-08-01".to_string(), None)];
        let checkpoint = serde_json::json!({"clear_all": true});
        let error = clear_all_state(
            &path,
            ClearAllStateRequest {
                expected_active_stable_id: "active-a",
                resulting_active_stable_id: "active-b",
                resulting_started_at_utc: Utc::now(),
                state: &non_empty,
                daily_updates: &updates,
                detached_at_utc: Utc::now(),
                simulation_time_utc: Utc::now(),
                checkpoint: &checkpoint,
            },
        )
        .unwrap_err();
        assert!(error.contains("non-empty"));
        assert_original_state(&path);
        remove_database(&path);
    }
}
'''
sqlite += additional_tests
sqlite_path.write_text(sqlite)
