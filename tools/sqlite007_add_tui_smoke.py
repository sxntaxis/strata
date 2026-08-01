from pathlib import Path

path = Path("tests/sqlite_cli_authority.rs")
text = path.read_text()
text = text.replace(
    '''use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};''',
    '''use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};''',
    1,
)
if "fn run_tui(&self) -> Output" not in text:
    text = text.replace(
        '''    fn database_path(&self) -> PathBuf {''',
        '''    fn run_tui(&self) -> Output {
        let mut child = Command::new("timeout");
        child
            .args([
                "10s",
                "script",
                "-qefc",
                env!("CARGO_BIN_EXE_strata"),
                "/dev/null",
            ])
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = child.spawn().expect("pseudo-terminal TUI should start");
        child
            .stdin
            .take()
            .expect("TUI stdin should exist")
            .write_all(b"q")
            .expect("quit key should be written");
        child.wait_with_output().expect("TUI process should finish")
    }

    fn database_path(&self) -> PathBuf {''',
        1,
    )
if "activated_tui_runs_and_persists_only_sqlite" not in text:
    text += '''

#[test]
fn activated_tui_runs_and_persists_only_sqlite() {
    let profile = TestProfile::new("tui-cutover");
    let legacy_categories_before =
        fs::read(profile.categories_path()).expect("legacy categories should be readable");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let tui = profile.run_tui();
    assert!(
        tui.status.success(),
        "TUI smoke failed with status {:?}: stdout={} stderr={}",
        tui.status.code(),
        stdout(&tui),
        stderr(&tui)
    );

    assert!(!profile.legacy_active_path().exists());
    assert!(!profile.legacy_time_log_path().exists());
    assert_eq!(
        fs::read(profile.categories_path()).expect("legacy categories should remain readable"),
        legacy_categories_before
    );

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .expect("active count should be readable");
    let tui_sessions: i64 = connection
        .query_row(
            "SELECT count(*) FROM sessions WHERE source = 'tui-runtime'",
            [],
            |row| row.get(0),
        )
        .expect("TUI session count should be readable");
    let sand_state_count: i64 = connection
        .query_row("SELECT count(*) FROM sand_state", [], |row| row.get(0))
        .expect("sand state count should be readable");
    assert_eq!(active_count, 0, "normal TUI exit must complete its active interval");
    assert_eq!(tui_sessions, 1, "TUI should persist exactly one completed interval");
    assert_eq!(sand_state_count, 1, "TUI should persist sediment state to SQLite");
}
'''
path.write_text(text)
