from pathlib import Path

path = Path('.github/scripts/apply_current_schema.py')
text = path.read_text()
old = '''text = sub_once(
    text,
    r"\\n    let pending_imports = if existing_tables\\.contains\\(\\\"legacy_imports\\\"\\) \\{.*?\\n    \\}\\);\\n",
    "\\n",
    "pending import doctor check",
)
'''
new = '''if "let pending_imports" in text:
    text = sub_once(
        text,
        r"\\n    let pending_imports = if existing_tables\\.contains\\(\\\"legacy_imports\\\"\\) \\{.*?\\n    \\}\\);\\n",
        "\\n",
        "pending import doctor check",
    )
'''
if old not in text:
    raise SystemExit('pending-import normalization marker missing')
path.write_text(text.replace(old, new, 1))
print('current-schema script normalized')
