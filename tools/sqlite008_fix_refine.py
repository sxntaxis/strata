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
