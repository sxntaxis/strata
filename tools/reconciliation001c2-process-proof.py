from pathlib import Path

path = Path("tests/sqlite_cli_authority.rs")
content = path.read_text()

content = content.replace(
    '''    io::Write,
''',
    '''    io::{Read, Write},
''',
    1,
)
content = content.replace(
    '''    process::{Command, Output, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
''',
    '''    process::{Command, Output, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
''',
    1,
)

marker = '''    fn recovery_files(&self) -> Vec<PathBuf> {
'''
helper = r'''    fn run_tui_lifecycle_merge(&self) -> Output {
        let mut command = Command::new("timeout");
        let tui_command = format!(
            "stty cols 120 rows 40; exec {}",
            env!("CARGO_BIN_EXE_strata")
        );
        command
            .args(["20s", "script", "-qefc", &tui_command, "/dev/null"])
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().expect("pseudo-terminal TUI should start");
        let mut stdin = child.stdin.take().expect("TUI stdin should exist");
        let mut stdout = child.stdout.take().expect("TUI stdout should exist");
        let mut stderr = child.stderr.take().expect("TUI stderr should exist");
        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        let captured_stdout = Arc::clone(&captured);
        let stdout_reader = thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match stdout.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => captured_stdout
                        .lock()
                        .expect("captured stdout lock should be available")
                        .extend_from_slice(&buffer[..count]),
                    Err(error) => panic!("TUI stdout should be readable: {error}"),
                }
            }
        });
        let stderr_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr
                .read_to_end(&mut bytes)
                .expect("TUI stderr should be readable");
            bytes
        });

        let send = |stdin: &mut std::process::ChildStdin, bytes: &[u8]| {
            stdin.write_all(bytes).expect("TUI input should be written");
            stdin.flush().expect("TUI input should flush");
            thread::sleep(Duration::from_millis(900));
        };

        thread::sleep(Duration::from_millis(1_200));
        send(&mut stdin, b"\r");
        send(&mut stdin, b"j");
        send(&mut stdin, b"X");
        send(&mut stdin, b"\r");

        let deadline = Instant::now() + Duration::from_secs(6);
        let phrase = loop {
            let bytes = captured
                .lock()
                .expect("captured stdout lock should be available")
                .clone();
            if let Some(phrase) = lifecycle_confirmation_phrase(&bytes) {
                break phrase;
            }
            assert!(
                Instant::now() < deadline,
                "lifecycle confirmation phrase was not rendered: {:?}",
                String::from_utf8_lossy(&bytes)
            );
            thread::sleep(Duration::from_millis(100));
        };

        stdin
            .write_all(phrase.as_bytes())
            .expect("confirmation phrase should be written");
        stdin.flush().expect("confirmation phrase should flush");
        send(&mut stdin, b"\r");

        let receipt_deadline = Instant::now() + Duration::from_secs(6);
        loop {
            if self.lifecycle_receipt_count() == 1 {
                break;
            }
            assert!(
                Instant::now() < receipt_deadline,
                "lifecycle receipt was not committed after exact confirmation"
            );
            thread::sleep(Duration::from_millis(100));
        }
        send(&mut stdin, b"q");
        drop(stdin);

        let status = child.wait().expect("TUI process should finish");
        stdout_reader.join().expect("stdout reader should finish");
        let stderr = stderr_reader.join().expect("stderr reader should finish");
        let stdout = captured
            .lock()
            .expect("captured stdout lock should be available")
            .clone();
        Output {
            status,
            stdout,
            stderr,
        }
    }

    fn lifecycle_receipt_count(&self) -> i64 {
        let connection = Connection::open(self.database_path()).expect("database should open");
        connection
            .query_row(
                "SELECT count(*) FROM category_lifecycle_receipts",
                [],
                |row| row.get(0),
            )
            .expect("lifecycle receipt count should be readable")
    }

'''
if marker not in content:
    raise SystemExit("TestProfile helper insertion marker missing")
content = content.replace(marker, helper + marker, 1)

function_marker = '''fn read_marker(profile: &TestProfile) -> Value {
'''
function = r'''fn lifecycle_confirmation_phrase(output: &[u8]) -> Option<String> {
    let rendered = String::from_utf8_lossy(output);
    let prefix = "MERGE 1 INTO 2 ";
    for (start, _) in rendered.match_indices(prefix).rev() {
        let revision: String = rendered[start + prefix.len()..]
            .chars()
            .take(16)
            .collect();
        if revision.len() == 16 && revision.chars().all(|character| character.is_ascii_hexdigit()) {
            return Some(format!("{prefix}{revision}"));
        }
    }
    None
}

'''
if function_marker not in content:
    raise SystemExit("confirmation parser insertion marker missing")
content = content.replace(function_marker, function + function_marker, 1)

insert_before = '''#[test]
fn unacknowledged_cli_stop_is_recovered_without_duplicate_session() {
'''
test = r'''#[test]
fn lifecycle_overlay_applies_only_the_live_revision_bound_phrase() {
    let profile = TestProfile::new("category-lifecycle-pty");
    fs::write(
        profile.categories_path(),
        "id,name,description,color_index,karma_effect\n\
         1,Work,Focused work,0,1\n\
         2,Target,Merged work,1,0\n",
    )
    .expect("two-category catalog should be written");
    profile.migrate();
    let activation = profile.activate();
    assert!(
        activation.status.success(),
        "activation failed: {}",
        stderr(&activation)
    );

    let tui = profile.run_tui_lifecycle_merge();
    assert!(
        tui.status.success(),
        "lifecycle TUI failed with status {:?}: stdout={} stderr={}",
        tui.status.code(),
        stdout(&tui),
        stderr(&tui)
    );
    let rendered = stdout(&tui);
    assert!(rendered.contains("DESTRUCTIVE LAYER LIFECYCLE"));
    assert!(rendered.contains("MERGE 1 INTO 2 "));

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let source_count: i64 = connection
        .query_row("SELECT count(*) FROM categories WHERE id = 1", [], |row| row.get(0))
        .expect("source count should be readable");
    let target: (String, String) = connection
        .query_row(
            "SELECT name, description FROM categories WHERE id = 2",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("target category should remain");
    let receipt: (String, i64, i64) = connection
        .query_row(
            "SELECT operation_kind, source_category_id, target_category_id
             FROM category_lifecycle_receipts",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("lifecycle receipt should exist");
    let active_count: i64 = connection
        .query_row("SELECT count(*) FROM active_session", [], |row| row.get(0))
        .expect("active count should be readable");

    assert_eq!(source_count, 0, "source identity should be retired");
    assert_eq!(target, ("Target".to_string(), "Merged work".to_string()));
    assert_eq!(receipt, ("merge".to_string(), 1, 2));
    assert_eq!(active_count, 0, "normal exit must finish the active interval");
}

'''
if insert_before not in content:
    raise SystemExit("process test insertion marker missing")
content = content.replace(insert_before, test + insert_before, 1)
path.write_text(content)
