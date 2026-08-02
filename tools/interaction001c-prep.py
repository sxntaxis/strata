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

old_confirm_replace = '''if text.count(old_main) != 1:
    raise SystemExit("main confirm fallback not found")
text = text.replace(old_main, "            Action::Confirm => false,\\n", 1)
'''
new_confirm_replace = '''main_start = text.index("    fn handle_main_action")
confirm_start = text.index("            Action::Confirm => {", main_start)
confirm_end = text.index("            Action::SwitchToNone => {", confirm_start)
text = text[:confirm_start] + "            Action::Confirm => false,\\n" + text[confirm_end:]
'''
if old_confirm_replace not in text:
    raise SystemExit("main confirm replacement block was not found")
text = text.replace(old_confirm_replace, new_confirm_replace, 1)

old_cancel_replace = '''if text.count(old_cancel) != 1:
    raise SystemExit("main cancel fallback not found")
text = text.replace(old_cancel, "            Action::Cancel => false,\\n", 1)
'''
new_cancel_replace = '''cancel_start = text.index("            Action::Cancel => {", main_start)
cancel_end = text.index("            Action::ReportToday => {", cancel_start)
text = text[:cancel_start] + "            Action::Cancel => false,\\n" + text[cancel_end:]
'''
if old_cancel_replace not in text:
    raise SystemExit("main cancel replacement block was not found")
text = text.replace(old_cancel_replace, new_cancel_replace, 1)

old_today_replace = '''if text.count(old_today) != 1:
    raise SystemExit("main today fallback not found")
text = text.replace(old_today, new_today, 1)
'''
new_today_replace = '''today_start = text.index("            Action::ReportToday => {", main_start)
today_end = text.index("            _ => false,", today_start)
text = text[:today_start] + new_today + text[today_end:]
'''
if old_today_replace not in text:
    raise SystemExit("main today replacement block was not found")
text = text.replace(old_today_replace, new_today_replace, 1)

atlas_start = text.index("# Atlas tests: defaults now report aliases separately")
atlas_end = text.index("path.write_text(text)", atlas_start)
text = text[:atlas_start] + text[atlas_end:]

keymap_replace_start = text.index('replace_between(\n    "src/keybindings.rs"')
keymap_replace_end = text.index("\n)\n\n# Configuration fields", keymap_replace_start) + 3
normalization = '''text = path.read_text()
keymap_config = text.index("#[derive(Debug, Clone, Serialize, Deserialize)]\\nstruct KeymapConfig")
impl_start = text.index("impl Keymap {")
depth = 0
impl_close = None
for offset, character in enumerate(text[impl_start:keymap_config]):
    if character == "{":
        depth += 1
    elif character == "}":
        depth -= 1
        if depth == 0:
            impl_close = impl_start + offset + 1
            break
if impl_close is None:
    raise SystemExit("generated Keymap implementation is not balanced")
trailing = text[impl_close:keymap_config]
if trailing.strip(" \\n\\t}"):
    raise SystemExit("unexpected generated content after Keymap implementation")
text = text[:impl_close] + "\\n\\n" + text[keymap_config:]
'''
text = (
    text[:keymap_replace_start]
    + "path.write_text(text)\n"
    + text[keymap_replace_start:keymap_replace_end]
    + normalization
    + text[keymap_replace_end:]
)

old_cleanup = '''    ".github/workflows/interaction001c-apply.yml",
    "tools/interaction001c-apply.py",
    "tools/interaction001c.trigger",
'''
new_cleanup = '''    ".github/workflows/interaction001c-apply.yml",
    "tools/interaction001c-apply.py",
    "tools/interaction001c-prep.py",
    "tools/interaction001c.trigger",
    "tools/interaction001c.rerun4",
'''
if old_cleanup not in text:
    raise SystemExit("temporary cleanup list was not found")
text = text.replace(old_cleanup, new_cleanup, 1)

path.write_text(text)
Path(__file__).unlink(missing_ok=True)
