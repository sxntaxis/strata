from pathlib import Path

path = Path("tests/sqlite_cli_authority.rs")
content = path.read_text()
old = '''        let mut command = Command::new("timeout");
        command
            .args([
                "10s",
                "script",
                "-qefc",
                env!("CARGO_BIN_EXE_strata"),
                "/dev/null",
            ])
'''
new = '''        let mut command = Command::new("timeout");
        let tui_command = format!(
            "stty cols 120 rows 40; exec {}",
            env!("CARGO_BIN_EXE_strata")
        );
        command
            .args(["10s", "script", "-qefc", &tui_command, "/dev/null"])
'''
if old not in content:
    raise SystemExit("TUI process command marker missing")
content = content.replace(old, new, 1)
old = '''    assert!(output.contains("RECOVERY EVIDENCE"));
'''
new = '''    assert!(
        output.contains("RECOVERY EVIDENCE"),
        "recovery statement was not rendered: {output:?}"
    );
'''
if old not in content:
    raise SystemExit("recovery statement output assertion marker missing")
path.write_text(content.replace(old, new, 1))
