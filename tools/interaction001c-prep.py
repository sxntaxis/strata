from pathlib import Path

path = Path("tools/interaction001c-apply.py")
text = path.read_text()
old_anchor = "insert_after_action = '''}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\nenum KeyCodeSpec {\n'''"
new_anchor = "insert_after_action = '''    }\n}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\nenum KeyCodeSpec {\n'''"
old_model = "model = '''}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]"
new_model = "model = '''    }\n}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]"
if old_anchor not in text or old_model not in text:
    raise SystemExit("action-model patch strings were not found")
path.write_text(text.replace(old_anchor, new_anchor, 1).replace(old_model, new_model, 1))
Path(__file__).unlink(missing_ok=True)
