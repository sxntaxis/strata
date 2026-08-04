from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text()


def write(path: str, content: str) -> None:
    Path(path).write_text(content)


def replace_once(path: str, old: str, new: str) -> None:
    content = read(path)
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:100]!r}")
    write(path, content.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    content = read(path)
    start_index = content.find(start)
    if start_index < 0:
        raise SystemExit(f"start marker missing in {path}: {start[:100]!r}")
    end_index = content.find(end, start_index)
    if end_index < 0:
        raise SystemExit(f"end marker missing in {path}: {end[:100]!r}")
    write(path, content[:start_index] + replacement + content[end_index:])


adapter = r'''pub(crate) fn initial_active_stable_id(started_at_utc: DateTime<Utc>) -> String {
    stable_id("tui", started_at_utc)
}

pub(crate) struct InitialActiveGenerationRequest<'a, T> {
    pub active_stable_id: &'a str,
    pub category_id: CategoryId,
    pub description: &'a str,
    pub started_at_utc: DateTime<Utc>,
    pub detached_at_utc: DateTime<Utc>,
    pub simulation_time_utc: DateTime<Utc>,
    pub checkpoint: &'a T,
}

pub(crate) fn start_active_session_with_checkpoint<T: Serialize>(
    database_path: &Path,
    request: InitialActiveGenerationRequest<'_, T>,
) -> Result<(), String> {
    let InitialActiveGenerationRequest {
        active_stable_id,
        category_id,
        description,
        started_at_utc,
        detached_at_utc,
        simulation_time_utc,
        checkpoint,
    } = request;
    let mut repository = open_cli_repository(database_path)?;
    let started = timestamp(started_at_utc);
    let payload_json = serde_json::to_string(checkpoint).map_err(|error| error.to_string())?;
    runtime_coordination::start_active_session_with_checkpoint(
        &mut repository,
        &NewActiveSession {
            stable_id: active_stable_id,
            project: "",
            category_id: as_i64(category_id.0, "category ID")?,
            description,
            started_at_utc: &started,
            recovery_kind: "live",
        },
        &timestamp(detached_at_utc),
        &timestamp(simulation_time_utc),
        &payload_json,
    )
    .map_err(|error| error.to_string())
}

'''
replace_between(
    "src/sqlite/tui_runtime.rs",
    "pub(crate) fn start_active_session_with_checkpoint<T: Serialize>(",
    "#[allow(clippy::too_many_arguments)]\npub(crate) fn switch_active_session(",
    adapter,
)

app_method = r'''    fn persist_initial_active_generation(&mut self) -> bool {
        let Some(database_path) = self.sqlite_database_path.clone() else {
            return true;
        };
        let category_id = self.time_tracker.active_category_id();
        let description = self
            .time_tracker
            .category_description_by_id(category_id)
            .unwrap_or_default()
            .to_string();
        let Some(started_at_utc) = self.session.active_session_started_at_utc else {
            self.record_storage_result_for::<()>(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                Err("initial active generation has no UTC start timestamp".to_string()),
            );
            return false;
        };
        let stable_id = sqlite::initial_tui_active_stable_id(started_at_utc);
        self.session.active_session_stable_id = Some(stable_id.clone());
        let checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
                self.session.active_session_stable_id = None;
                self.record_storage_result_for::<()>(
                    PersistenceOperation::CheckpointSave,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return false;
            }
        };
        let result = sqlite::start_tui_active_session_with_checkpoint(
            &database_path,
            sqlite::TuiInitialActiveGenerationRequest {
                active_stable_id: &stable_id,
                category_id,
                description: &description,
                started_at_utc,
                detached_at_utc: checkpoint.detached_at_utc,
                simulation_time_utc: checkpoint.simulation_time_utc,
                checkpoint: &checkpoint,
            },
        );
        if self
            .record_storage_result_for(
                PersistenceOperation::ActiveStart,
                RecoveryAction::ReloadAuthority,
                result,
            )
            .is_none()
        {
            self.session.active_session_stable_id = None;
            return false;
        }
        true
    }

'''
replace_between(
    "src/app.rs",
    "    fn persist_initial_active_generation(&mut self) -> bool {",
    "    fn reload_sqlite_sessions(&mut self) -> bool {",
    app_method,
)

replace_once(
    "src/sqlite.rs",
    "pub(crate) use tui_runtime::{\n    ClearAllStateRequest as TuiClearAllStateRequest,",
    "pub(crate) use tui_runtime::{\n    ClearAllStateRequest as TuiClearAllStateRequest,\n    InitialActiveGenerationRequest as TuiInitialActiveGenerationRequest,",
)
replace_once(
    "src/sqlite.rs",
    "    finish_active_session as finish_tui_active_session, load_checkpoint as load_tui_checkpoint,\n",
    "    finish_active_session as finish_tui_active_session,\n    initial_active_stable_id as initial_tui_active_stable_id,\n    load_checkpoint as load_tui_checkpoint,\n",
)

tests = r'''    #[derive(serde::Serialize)]
    struct BootstrapCheckpointFixture {
        schema_version: u8,
    }

    fn prepare_bootstrap_repository(path: &Path) {
        let mut repository = SqliteRepository::open(path).unwrap();
        repository
            .transition_storage_authority("sqlite-candidate", "sqlite-cli", "2026-08-03T18:00:00Z")
            .unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
    }

    #[test]
    fn initial_bootstrap_binds_active_and_checkpoint_row_identity() {
        let path = repository_file("initial-bootstrap-identity");
        prepare_bootstrap_repository(&path);
        let started_at_utc = DateTime::parse_from_rfc3339("2026-08-03T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let stable_id = initial_active_stable_id(started_at_utc);
        let checkpoint = BootstrapCheckpointFixture { schema_version: 3 };

        start_active_session_with_checkpoint(
            &path,
            InitialActiveGenerationRequest {
                active_stable_id: &stable_id,
                category_id: CategoryId::new(1),
                description: "Focused",
                started_at_utc,
                detached_at_utc: started_at_utc,
                simulation_time_utc: started_at_utc,
                checkpoint: &checkpoint,
            },
        )
        .unwrap();

        let repository = open_cli_repository(&path).unwrap();
        let row: (String, String, String) = repository
            .connection
            .query_row(
                "SELECT active_session.stable_id,
                        runtime_checkpoint.active_session_stable_id,
                        runtime_checkpoint.payload_json
                 FROM active_session CROSS JOIN runtime_checkpoint
                 WHERE active_session.singleton = 1 AND runtime_checkpoint.singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&row.2).unwrap();
        assert_eq!(row.0, stable_id);
        assert_eq!(row.1, stable_id);
        assert_eq!(payload.get("schema_version").and_then(serde_json::Value::as_u64), Some(3));
        drop(repository);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn initial_bootstrap_refuses_preexisting_checkpoint_without_overwrite() {
        let path = repository_file("initial-bootstrap-existing-checkpoint");
        prepare_bootstrap_repository(&path);
        let repository = open_cli_repository(&path).unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO runtime_checkpoint (
                    singleton, status, detached_at_utc, simulation_time_utc,
                    active_session_stable_id, payload_json, legacy_import_id
                 ) VALUES (1, 'quarantined', '2026-08-03T17:00:00Z',
                    '2026-08-03T17:00:00Z', NULL, '{}', NULL)",
                [],
            )
            .unwrap();
        drop(repository);

        let started_at_utc = DateTime::parse_from_rfc3339("2026-08-03T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let stable_id = initial_active_stable_id(started_at_utc);
        let checkpoint = BootstrapCheckpointFixture { schema_version: 3 };
        let error = start_active_session_with_checkpoint(
            &path,
            InitialActiveGenerationRequest {
                active_stable_id: &stable_id,
                category_id: CategoryId::new(1),
                description: "Focused",
                started_at_utc,
                detached_at_utc: started_at_utc,
                simulation_time_utc: started_at_utc,
                checkpoint: &checkpoint,
            },
        )
        .unwrap_err();
        assert!(error.contains("no checkpoint before initial active generation"));

        let repository = open_cli_repository(&path).unwrap();
        let active_count: i64 = repository
            .connection
            .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
            .unwrap();
        let checkpoint: (String, String) = repository
            .connection
            .query_row(
                "SELECT status, payload_json FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(active_count, 0);
        assert_eq!(checkpoint.0, "quarantined");
        assert_eq!(checkpoint.1, "{}");
        drop(repository);
        std::fs::remove_file(path).ok();
    }

'''
replace_once(
    "src/sqlite/tui_runtime.rs",
    "    #[test]\n    fn category_order_and_archival_round_trip() {",
    tests + "    #[test]\n    fn category_order_and_archival_round_trip() {",
)
