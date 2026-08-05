from pathlib import Path

path = Path('.github/scripts/cleanup_current_schema.py')
text = path.read_text()
start_marker = '# Maintenance asserts the exact current product schema, with no import queue.\n'
end_marker = '# Final architectural certification.\n'
start = text.find(start_marker)
end = text.find(end_marker)
if start < 0 or end < 0 or end <= start:
    raise SystemExit('maintenance cleanup section markers missing')
replacement = '''# Maintenance asserts the exact current product schema, with no import queue.
path = Path("src/sqlite/maintenance.rs")
text = path.read_text()
text = re.sub(r'^\\s*"schema_migrations",\\n', '', text, flags=re.M)
text = re.sub(r'^\\s*"legacy_imports",\\n', '', text, flags=re.M)
text = re.sub(
    r"\\n    let pending_imports = if existing_tables.*?\\n    checks\\.push\\(check\\(.*?\\n    \\)\\);\\n",
    "\\n",
    text,
    count=1,
    flags=re.S,
)
path.write_text(text)

'''
path.write_text(text[:start] + replacement + text[end:])
print('current-schema cleanup normalized')
