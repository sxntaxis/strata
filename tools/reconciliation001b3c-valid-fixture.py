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
    let payload: Value = serde_json::from_str(&payload_json).expect("payload should be JSON");
    assert!(
        payload["sand_state"]["grid_width"].as_u64().unwrap_or(0) > 0,
        "fixed-size PTY should produce an initialized checkpoint canvas"
    );
    assert!(
        payload["sand_state"]["grid_height"].as_u64().unwrap_or(0) > 0,
        "fixed-size PTY should produce an initialized checkpoint canvas"
    );
    drop(connection);

    let failed = profile.run_tui_with_input(b"", Some("checkpoint-recovery:commit:cutoff"));
'''
if old not in content:
    raise SystemExit("visible recovery fixture marker missing")
path.write_text(content.replace(old, new, 1))
