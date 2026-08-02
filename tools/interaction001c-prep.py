from pathlib import Path

path = Path("tools/interaction001c-apply.py")
text = path.read_text()
old_anchor = "insert_after_action = '''}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\nenum KeyCodeSpec {\n'''"
new_anchor = "insert_after_action = '''    }\n}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\nenum KeyCodeSpec {\n'''"
old_model = "model = '''}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]"
new_model = "model = '''    }\n}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]"
if old_anchor not in text or old_model not in text:
    raise SystemExit("action-model patch strings were not found")
text = text.replace(old_anchor, new_anchor, 1).replace(old_model, new_model, 1)
old_alias_insert = '''anchor = "pub(crate) fn default_keymap() -> Keymap {\\n"
if text.count(anchor) != 1:
    raise SystemExit("default keymap anchor not found")
text = text.replace(anchor, default_aliases + anchor, 1)
'''
new_alias_insert = '''anchor = "pub(crate) fn default_keymap"
position = text.index(anchor)
text = text[:position] + default_aliases + text[position:]
'''
if old_alias_insert not in text:
    raise SystemExit("default alias insertion block was not found")
path.write_text(text.replace(old_alias_insert, new_alias_insert, 1))
Path(__file__).unlink(missing_ok=True)
