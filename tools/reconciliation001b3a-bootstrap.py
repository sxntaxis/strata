from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text()


def write(path: str, content: str) -> None:
    Path(path).write_text(content)


def insert_before(path: str, marker: str, insertion: str) -> None:
    content = read(path)
    if insertion.strip() in content:
        return
    if marker not in content:
        raise SystemExit(f"marker missing in {path}: {marker[:80]!r}")
    write(path, content.replace(marker, insertion + marker, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    content = read(path)
    start_index = content.find(start)
    if start_index < 0:
        raise SystemExit(f"start marker missing in {path}: {start[:80]!r}")
    end_index = content.find(end, start_index)
    if end_index < 0:
        raise SystemExit(f"end marker missing in {path}: {end[:80]!r}")
    write(path, content[:start_index] + replacement + content[end_index:])


runtime_bootstrap = r'''pub(crate) fn start_active_session_with_checkpoint(
    repository: &mut SqliteRepository,
    active: &NewActiveSession<'_>,
    detached_at_utc: &str,
    simulation_time_utc: &str,
    payload_json: &str,
) -> Result<(), CoordinationError> {
    require_non_empty(active.stable_id, "active stable ID")?;
    require_non_empty(detached_at_utc, "checkpoint capture timestamp")?;
    require_non_empty(simulation_time_utc, "checkpoint simulation timestamp")?;
    require_non_empty(payload_json, "checkpoint payload")?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    maybe_inject_test_fault("active-bootstrap", "before-write")?;
    if let Some(current) = query_active(&transaction)? {
        return Err(CoordinationError::ActiveSessionConflict {
            expected: "no active session".to_string(),
            actual: current.stable_id,
        });
    }
    let checkpoint: Option<(String, Option<String>)> = transaction
        .query_row(
            "SELECT status, active_session_stable_id
             FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((status, identity)) = checkpoint {
        return Err(CoordinationError::CheckpointConflict {
            expected: "no checkpoint before initial active generation".to_string(),
            actual: format!(
                "{status} for {}",
                identity.as_deref().unwrap_or("no active identity")
            ),
        });
    }

    insert_active(&transaction, active)?;
    maybe_inject_test_fault("active-bootstrap", "active")?;
    transaction.execute(
        "INSERT INTO runtime_checkpoint (
            singleton, status, detached_at_utc, simulation_time_utc,
            active_session_stable_id, payload_json, legacy_import_id
         ) VALUES (1, 'pending', ?1, ?2, ?3, ?4, NULL)",
        params![
            detached_at_utc,
            simulation_time_utc,
            active.stable_id,
            payload_json,
        ],
    )?;
    maybe_inject_test_fault("active-bootstrap", "checkpoint")?;
    maybe_inject_test_fault("active-bootstrap", "commit")?;
    transaction.commit()?;
    Ok(())
}

'''
insert_before(
    "src/sqlite/runtime_coordination.rs",
    "pub(crate) fn finish_active_session(",
    runtime_bootstrap,
)

runtime_tests = r'''    #[test]
    fn initial_active_and_checkpoint_commit_as_one_generation() {
        let path = database_path("initial-generation");
        let mut repository = SqliteRepository::open(&path).unwrap();
        repository
            .create_category(&NewCategoryRecord {
                name: "Work",
                description: "",
                color_index: 0,
                balance_effect: 1,
            })
            .unwrap();
        let active = NewActiveSession {
            stable_id: "initial-active",
            project: "",
            category_id: 1,
            description: "Focused",
            started_at_utc: "2026-08-03T18:00:00Z",
            recovery_kind: "live",
        };

        start_active_session_with_checkpoint(
            &mut repository,
            &active,
            "2026-08-03T18:00:00Z",
            "2026-08-03T18:00:00Z",
            r#"{"schema_version":3}"#,
        )
        .unwrap();

        let active_identity: String = repository
            .connection
            .query_row(
                "SELECT stable_id FROM active_session WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let checkpoint: (String, String, String) = repository
            .connection
            .query_row(
                "SELECT status, active_session_stable_id, payload_json
                 FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(active_identity, "initial-active");
        assert_eq!(checkpoint.0, "pending");
        assert_eq!(checkpoint.1, active_identity);
        assert_eq!(checkpoint.2, r#"{"schema_version":3}"#);
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn initial_active_and_checkpoint_roll_back_at_every_fault_boundary() {
        for phase in ["before-write", "active", "checkpoint", "commit"] {
            let path = database_path(&format!("initial-generation-{phase}"));
            let mut repository = SqliteRepository::open(&path).unwrap();
            repository
                .create_category(&NewCategoryRecord {
                    name: "Work",
                    description: "",
                    color_index: 0,
                    balance_effect: 1,
                })
                .unwrap();
            let active = NewActiveSession {
                stable_id: "initial-active",
                project: "",
                category_id: 1,
                description: "Focused",
                started_at_utc: "2026-08-03T18:00:00Z",
                recovery_kind: "live",
            };
            let error = with_test_fault("active-bootstrap", phase, "kill", || {
                start_active_session_with_checkpoint(
                    &mut repository,
                    &active,
                    "2026-08-03T18:00:00Z",
                    "2026-08-03T18:00:00Z",
                    r#"{"schema_version":3}"#,
                )
            })
            .unwrap_err();
            assert!(matches!(error, CoordinationError::InjectedFailure { .. }));
            let active_count: i64 = repository
                .connection
                .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
                .unwrap();
            let checkpoint_count: i64 = repository
                .connection
                .query_row("SELECT count(*) FROM runtime_checkpoint", [], |row| row.get(0))
                .unwrap();
            assert_eq!(active_count, 0, "phase {phase} left an orphan active row");
            assert_eq!(
                checkpoint_count, 0,
                "phase {phase} left an orphan checkpoint"
            );
            drop(repository);
            remove_database(&path);
        }
    }

'''
insert_before(
    "src/sqlite/runtime_coordination.rs",
    "    #[test]\n    fn concurrent_identical_finish_converges_on_one_receipt()",
    runtime_tests,
)

runtime_adapter = r'''pub(crate) fn start_active_session_with_checkpoint<T: Serialize>(
    database_path: &Path,
    category_id: CategoryId,
    description: &str,
    started_at_utc: DateTime<Utc>,
    detached_at_utc: DateTime<Utc>,
    simulation_time_utc: DateTime<Utc>,
    checkpoint: &T,
) -> Result<String, String> {
    let mut repository = open_cli_repository(database_path)?;
    let stable_id = stable_id("tui", started_at_utc);
    let started = timestamp(started_at_utc);
    let payload_json = serde_json::to_string(checkpoint).map_err(|error| error.to_string())?;
    runtime_coordination::start_active_session_with_checkpoint(
        &mut repository,
        &NewActiveSession {
            stable_id: &stable_id,
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
    .map_err(|error| error.to_string())?;
    Ok(stable_id)
}

'''
insert_before(
    "src/sqlite/tui_runtime.rs",
    "#[allow(clippy::too_many_arguments)]\npub(crate) fn switch_active_session(",
    runtime_adapter,
)

sqlite = read("src/sqlite.rs")
old_export = "    ensure_active_session as ensure_tui_active_session,\n"
new_exports = (
    old_export
    + "    start_active_session_with_checkpoint as start_tui_active_session_with_checkpoint,\n"
)
if old_export not in sqlite:
    raise SystemExit("SQLite TUI active-start export marker missing")
write("src/sqlite.rs", sqlite.replace(old_export, new_exports, 1))

app_constructor_and_bootstrap = r'''        app.persist_category_tags();

        let had_sqlite_active_session = sqlite_active_session.is_some();
        let mut initial_checkpoint_published = false;
        if !app.restore_from_detached_checkpoint() && !app.has_persistence_recovery() {
            if let Some(active) = sqlite_active_session {
                if !app
                    .time_tracker
                    .set_active_category_by_id(active.category_id)
                {
                    return Err(format!(
                        "SQLite active session references unavailable category {}",
                        active.category_id.0
                    ));
                }
                let _ = app
                    .time_tracker
                    .set_category_description_by_id(active.category_id, active.description);
                app.session.active_session_stable_id = Some(active.stable_id);
                app.begin_active_session_at(active.started_at_utc, false)?;
            } else {
                app.begin_active_session_now();
            }
            app.restore_sand_state();
            if !had_sqlite_active_session
                && app.sqlite_database_path.is_some()
                && !app.has_persistence_recovery()
            {
                app.sync_drift_idle_state();
                initial_checkpoint_published = app.persist_initial_active_generation();
            }
        }

        app.sync_drift_idle_state();
        app.commit_checkpoint_recovery_if_ready();
        if !app.has_persistence_recovery() && !initial_checkpoint_published {
            app.persist_runtime_checkpoint();
        }
        if let Some(recovery) = app.persistence_recovery.take() {
            return Err(recovery.failure.summary());
        }

        Ok(app)
    }

    fn persist_initial_active_generation(&mut self) -> bool {
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
        let checkpoint = match self.build_runtime_checkpoint() {
            Ok(checkpoint) => checkpoint,
            Err(error) => {
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
            category_id,
            &description,
            started_at_utc,
            checkpoint.detached_at_utc,
            checkpoint.simulation_time_utc,
            &checkpoint,
        );
        let Some(stable_id) = self.record_storage_result_for(
            PersistenceOperation::ActiveStart,
            RecoveryAction::ReloadAuthority,
            result,
        ) else {
            return false;
        };
        self.session.active_session_stable_id = Some(stable_id);
        true
    }

'''
replace_between(
    "src/app.rs",
    "        app.persist_category_tags();",
    "    fn reload_sqlite_sessions(&mut self) -> bool {",
    app_constructor_and_bootstrap,
)

process_test = r'''#[test]
fn initial_tui_bootstrap_failure_leaves_no_orphan_generation() {
    for phase in ["before-write", "active", "checkpoint", "commit"] {
        let profile = TestProfile::new(&format!("initial-bootstrap-{phase}"));
        profile.migrate();
        assert!(profile.activate().status.success());
        let fault = format!("active-bootstrap:{phase}:commit");

        let tui = profile.run_tui_with_input(b"q", Some(&fault));
        assert!(
            !tui.status.success(),
            "injected {phase} failure unexpectedly succeeded"
        );

        let connection = Connection::open(profile.database_path()).expect("database should open");
        let active_count: i64 = connection
            .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
            .unwrap();
        let checkpoint_count: i64 = connection
            .query_row("SELECT count(*) FROM runtime_checkpoint", [], |row| row.get(0))
            .unwrap();
        assert_eq!(active_count, 0, "phase {phase} left an orphan active row");
        assert_eq!(
            checkpoint_count, 0,
            "phase {phase} left an orphan checkpoint"
        );
        drop(connection);

        let retry = profile.run_tui();
        assert!(
            retry.status.success(),
            "retry after {phase} failed: stdout={} stderr={}",
            stdout(&retry),
            stderr(&retry)
        );
    }
}

'''
insert_before(
    "tests/sqlite_cli_authority.rs",
    "fn recovery_bundle(profile: &TestProfile) -> Value {",
    process_test,
)

work = r'''---
id: RECONCILIATION-001B3A
kind: work
state: active
authority: working
created: 2026-08-03
updated: 2026-08-03
---

# RECONCILIATION-001B3A — atomic initial active generation

## Issue

Under SQLite authority, first TUI startup with no existing active generation currently commits the `active_session` row before restoring sediment and publishing the first runtime checkpoint. Process death or a later startup failure can therefore expose an active generation with no matching recoverable checkpoint evidence.

## Selected contract

- A new SQLite TUI active generation and its first pending runtime checkpoint form one immediate transaction.
- The checkpoint binds the same stable active identity, UTC start, category, description, simulation timestamp, accumulators, and canonical sediment state staged in memory.
- Existing active state or any pre-existing checkpoint blocks initial bootstrap; startup never overwrites unresolved evidence.
- Sediment restoration succeeds before the transaction is attempted.
- Failure before write, after active insertion, after checkpoint insertion, or immediately before commit leaves neither row durable.
- Once committed, restart always observes both authorities under the same stable identity.
- Legacy-file startup remains a single atomic checkpoint-file publication and does not gain a second competing active authority.

## Acceptance proofs

- successful initial bootstrap creates exactly one active row and one pending checkpoint with the same stable ID;
- every injected SQLite bootstrap failure rolls both rows back;
- the real TUI startup path fails visibly without leaving an orphan generation and can be retried cleanly;
- existing active/checkpoint recovery paths remain unchanged;
- formatting, strict Clippy, all tests, and process suites remain green.

## Boundary

This unit closes only the initial active-start/checkpoint window. Exact remaining transition-edge sediment attribution and visible recovery cutoff/reconstruction semantics remain later issue #10 units.
'''
Path("notebook/work/RECONCILIATION-001B3A.md").write_text(work)
