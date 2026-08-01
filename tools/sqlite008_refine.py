from pathlib import Path


def insert_before_final_brace(text: str, addition: str, label: str) -> str:
    index = text.rfind("\n}")
    if index < 0:
        raise SystemExit(f"missing {label} module close")
    return text[:index] + addition + text[index:]


# Receipts retain their audit record when a completed session is explicitly deleted.
path = Path("src/sqlite.rs")
text = path.read_text()
old = '''    FOREIGN KEY (completed_session_id) REFERENCES sessions(id)
        ON UPDATE RESTRICT
        ON DELETE RESTRICT
) STRICT;'''
new = '''    FOREIGN KEY (completed_session_id) REFERENCES sessions(id)
        ON UPDATE RESTRICT
        ON DELETE SET NULL
) STRICT;'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("missing runtime transition deletion policy")
path.write_text(text)


# Prove that explicit session deletion remains available without destroying the receipt.
path = Path("src/sqlite/runtime_coordination.rs")
text = path.read_text()
marker = "fn completed_session_deletion_retains_receipt_without_dangling_reference()"
if marker not in text:
    addition = r'''

    #[test]
    fn completed_session_deletion_retains_receipt_without_dangling_reference() {
        let path = database_path("receipt-session-delete");
        seed(&path, "active-a");
        let mut repository = SqliteRepository::open(&path).unwrap();
        let receipt = finish_active_session(
            &mut repository,
            "active-a",
            "finish:active-a",
            &completion("tui-runtime"),
            true,
        )
        .unwrap();
        let completed_id = receipt.completed_session_id.unwrap();
        assert!(repository.delete_session(completed_id).unwrap());
        let retained: (i64, Option<i64>) = repository
            .connection
            .query_row(
                "SELECT count(*), max(completed_session_id)
                 FROM runtime_transitions WHERE operation_id = 'finish:active-a'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(retained.0, 1);
        assert_eq!(retained.1, None);
        drop(repository);
        remove_database(&path);
    }
'''
    text = insert_before_final_brace(text, addition, "coordination tests")
path.write_text(text)


# Exercise the CLI crash-window recovery path through the real process boundary.
path = Path("tests/sqlite_cli_authority.rs")
text = path.read_text()
marker = "fn unacknowledged_cli_stop_is_recovered_without_duplicate_session()"
if marker not in text:
    addition = r'''

#[test]
fn unacknowledged_cli_stop_is_recovered_without_duplicate_session() {
    let profile = TestProfile::new("stop-receipt-recovery");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let start = profile.run(&["start", "receipt-project", "--category", "Work"]);
    assert!(start.status.success(), "start failed: {}", stderr(&start));
    let first_stop = profile.run(&["stop"]);
    assert!(
        first_stop.status.success(),
        "initial stop failed: {}",
        stderr(&first_stop)
    );

    {
        let connection = Connection::open(profile.database_path()).expect("database should open");
        let changed = connection
            .execute(
                "UPDATE runtime_transitions
                 SET acknowledged_at_utc = NULL
                 WHERE operation_kind = 'finish' AND source = 'cli-runtime'",
                [],
            )
            .expect("receipt acknowledgement should be cleared");
        assert_eq!(changed, 1);
    }

    let recovered = profile.run(&["stop"]);
    assert!(
        recovered.status.success(),
        "receipt recovery failed: {}",
        stderr(&recovered)
    );
    assert!(stdout(&recovered).contains("Stopped session"));

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let session_count: i64 = connection
        .query_row("SELECT count(*) FROM sessions", [], |row| row.get(0))
        .expect("session count should be readable");
    let unacknowledged: i64 = connection
        .query_row(
            "SELECT count(*) FROM runtime_transitions
             WHERE operation_kind = 'finish' AND acknowledged_at_utc IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("receipt count should be readable");
    assert_eq!(session_count, 1, "recovery must not duplicate time");
    assert_eq!(unacknowledged, 0, "recovered receipt must be acknowledged");
    drop(connection);

    let repeated = profile.run(&["stop"]);
    assert!(!repeated.status.success());
    assert!(stderr(&repeated).contains("No active session"));
}
'''
    text = insert_before_final_brace(text, addition, "authority tests")
path.write_text(text)
