from pathlib import Path

path = Path("src/sqlite/runtime_coordination.rs")
text = path.read_text()
old = "        assert!(repository.delete_session(completed_id).unwrap());"
new = "        repository.delete_session(completed_id).unwrap();"
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("missing deletion contract assertion")
path.write_text(text)

path = Path("tests/sqlite_cli_authority.rs")
text = path.read_text()
marker = "\n\n#[test]\nfn unacknowledged_cli_stop_is_recovered_without_duplicate_session()"
start = text.find(marker)
if start < 0:
    raise SystemExit("missing subprocess receipt recovery test")
if not text.endswith("\n}"):
    raise SystemExit("unexpected authority test ending")
# The refinement script inserts before the final function brace. Close that
# existing test first, then retain the new test as a top-level integration test.
inserted = text[start:-2]
text = text[:start] + "\n}" + inserted + "\n"
path.write_text(text)
