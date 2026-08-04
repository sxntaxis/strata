from pathlib import Path

path = Path("tests/sqlite_cli_authority.rs")
content = path.read_text()
old = '''    let detached = profile.run_tui_with_input(b"d", None);
    assert!(
        detached.status.success(),
        "detach failed: stdout={} stderr={}",
        stdout(&detached),
        stderr(&detached)
    );

    let failed = profile.run_tui_with_input(b"", Some("checkpoint-recovery:commit:cutoff"));
'''
new = '''    let detached = profile.run_tui_with_input(b"d", None);
    assert!(
        detached.status.success(),
        "detach failed: stdout={} stderr={}",
        stdout(&detached),
        stderr(&detached)
    );

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let payload_json: String = connection
        .query_row(
            "SELECT payload_json FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .expect("detached checkpoint should exist");
    let mut payload: Value = serde_json::from_str(&payload_json).expect("payload should be JSON");
    payload["sand_state"]["grid_width"] = Value::from(2);
    payload["sand_state"]["grid_height"] = Value::from(2);
    connection
        .execute(
            "UPDATE runtime_checkpoint SET payload_json = ?1 WHERE singleton = 1",
            [serde_json::to_string(&payload).unwrap()],
        )
        .unwrap();
    drop(connection);

    let failed = profile.run_tui_with_input(b"", Some("checkpoint-recovery:commit:cutoff"));
'''
if old not in content:
    raise SystemExit("visible recovery fixture marker missing")
path.write_text(content.replace(old, new, 1))
