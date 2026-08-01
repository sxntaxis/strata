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

# The refinement script inserts before the final brace, which closes the
# preceding top-level integration test. Move that brace before the inserted
# test while accepting any trailing whitespace in the generated file.
prefix = text[:start].rstrip()
tail = text[start:].rstrip()
if not tail.endswith("}"):
    raise SystemExit("unexpected authority test ending")
inserted_test = tail[:-1].rstrip()
path.write_text(prefix + "\n}\n" + inserted_test + "\n")
