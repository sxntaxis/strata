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
text = text.replace(old_alias_insert, new_alias_insert, 1)

old_effective_replace = '''if text.count(old_effective) != 1:
    raise SystemExit("effective key fallback block not found")
text = text.replace(old_effective, new_effective, 1)
'''
new_effective_replace = '''effective_start = text.index("    fn effective_keys_for_action(")
effective_end = text.index("    fn atlas_item_description", effective_start)
text = text[:effective_start] + new_effective + text[effective_end:]
'''
if old_effective_replace not in text:
    raise SystemExit("effective key replacement block was not found")
text = text.replace(old_effective_replace, new_effective_replace, 1)

path.write_text(text)
Path(__file__).unlink(missing_ok=True)
