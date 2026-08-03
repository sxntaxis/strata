from pathlib import Path

tui_path = Path("src/sqlite/tui_runtime.rs")
tui = tui_path.read_text()
# Add focused transactional proofs.
tests = r'''

#[cfg(test)]
mod clear_all_transaction_tests {
    use std::path::{Path, PathBuf};

    use serde_json::Value;

    use super::*;
    use crate::sqlite::{NewActiveSession, SqliteRepository, runtime_coordination};

    fn database_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "strata-clear-all-{label}-{}-{}.sqlite3",
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
                stable_id: "idle-a",
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
            "idle-a",
            Utc::now(),
            Utc::now(),
            &serde_json::json!({"before": true}),
        )
        .unwrap();
    }

    #[test]
    fn clear_all_is_atomic_and_preserves_committed_history() {
        let path = database_path("commit");
        seed(&path);
        let empty = state(false);
        let checkpoint = serde_json::json!({"clear_all": {"operation_id": "clear"}});
        let updates = [("2026-08-01".to_string(), None)];
        clear_all_state(
            &path,
            ClearAllStateRequest {
                expected_active_stable_id: "idle-a",
                resulting_active_stable_id: "idle-b",
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
        let session_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(session_count, 1);
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "idle-b"
        );
        let checkpoint = repository.checkpoint().unwrap().unwrap();
        assert_eq!(checkpoint.active_session_stable_id.as_deref(), Some("idle-b"));
        let payload: Value = serde_json::from_str(&checkpoint.payload_json).unwrap();
        assert_eq!(payload["clear_all"]["operation_id"], "clear");
        drop(repository);
        assert_eq!(load_sand_state(&path).unwrap(), Some(empty));
        assert!(load_daily_snapshot(&path, "2026-08-01").unwrap().is_none());
        remove_database(&path);
    }

    #[test]
    fn clear_all_fault_rolls_back_every_authority() {
        let path = database_path("rollback");
        seed(&path);
        let result = runtime_coordination::with_test_fault(
            "clear-all",
            "commit",
            "io",
            || {
                let empty = state(false);
                let updates = [("2026-08-01".to_string(), None)];
                let checkpoint = serde_json::json!({"clear_all": true});
                clear_all_state(
                    &path,
                    ClearAllStateRequest {
                        expected_active_stable_id: "idle-a",
                        resulting_active_stable_id: "idle-b",
                        resulting_started_at_utc: Utc::now(),
                        state: &empty,
                        daily_updates: &updates,
                        detached_at_utc: Utc::now(),
                        simulation_time_utc: Utc::now(),
                        checkpoint: &checkpoint,
                    },
                )
            },
        );
        assert!(result.is_err());
        let repository = open_cli_repository(&path).unwrap();
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "idle-a"
        );
        assert_eq!(
            repository
                .connection
                .query_row("SELECT count(*) FROM sessions", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(repository);
        assert_eq!(load_sand_state(&path).unwrap(), Some(state(true)));
        assert!(load_daily_snapshot(&path, "2026-08-01").unwrap().is_some());
        remove_database(&path);
    }
}
'''
tui += tests
tui_path.write_text(tui)
