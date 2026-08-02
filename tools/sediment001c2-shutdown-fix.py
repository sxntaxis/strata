from pathlib import Path


path = Path("src/sqlite/runtime_coordination.rs")
source = path.read_text()
start = source.index("pub(crate) fn clear_committed_checkpoint(")
end = source.index("#[cfg(test)]", start)
replacement = '''pub(crate) fn clear_committed_checkpoint(
    repository: &mut SqliteRepository,
) -> Result<(), CoordinationError> {
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let status: Option<String> = transaction
        .query_row(
            "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match status.as_deref() {
        None => {
            transaction.commit()?;
            Ok(())
        }
        Some("pending" | "committed") => {
            transaction.execute(
                "DELETE FROM runtime_checkpoint
                 WHERE singleton = 1 AND status IN ('pending', 'committed')",
                [],
            )?;
            maybe_inject_test_fault("checkpoint-clear", "commit")?;
            transaction.commit()?;
            Ok(())
        }
        Some(actual) => Err(CoordinationError::CheckpointConflict {
            expected: "pending, committed, or missing".to_string(),
            actual: actual.to_string(),
        }),
    }
}

'''
source = source[:start] + replacement + source[end:]

anchor = '''    #[test]
    fn checkpoint_commit_is_atomic_and_recovering_is_reclaimable() {
'''
proofs = '''    #[test]
    fn normal_shutdown_can_retire_pending_checkpoint() {
        let path = database_path("pending-checkpoint-clear");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T11:00:00Z",
            "2026-08-01T10:59:00Z",
            "{\\"schema_version\\":2}",
        )
        .unwrap();

        clear_committed_checkpoint(&mut repository).unwrap();

        assert!(repository.checkpoint().unwrap().is_none());
        assert_eq!(
            repository.active_session().unwrap().unwrap().stable_id,
            "active-a"
        );
        drop(repository);
        remove_database(&path);
    }

    #[test]
    fn shutdown_clear_refuses_recovering_checkpoint() {
        let path = database_path("recovering-checkpoint-clear");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        save_checkpoint(
            &mut repository,
            "active-a",
            "2026-08-01T11:00:00Z",
            "2026-08-01T10:59:00Z",
            "{\\"schema_version\\":2}",
        )
        .unwrap();
        claim_checkpoint(&mut repository).unwrap().unwrap();

        let error = clear_committed_checkpoint(&mut repository).unwrap_err();

        assert!(matches!(
            error,
            CoordinationError::CheckpointConflict { ref actual, .. } if actual == "recovering"
        ));
        let status: String = repository
            .connection
            .query_row(
                "SELECT status FROM runtime_checkpoint WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "recovering");
        drop(repository);
        remove_database(&path);
    }

'''
if source.count(anchor) != 1:
    raise SystemExit("checkpoint test anchor was not found")
path.write_text(source.replace(anchor, proofs + anchor, 1))
Path(__file__).unlink(missing_ok=True)
