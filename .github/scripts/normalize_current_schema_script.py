from pathlib import Path
import re

path = Path('.github/scripts/apply_current_schema.py')
text = path.read_text()

pending_pattern = r'''text = sub_once\(\n    text,\n    r"\\n    let pending_imports = if existing_tables.*?    "pending import doctor check",\n\)\n'''
text, pending_count = re.subn(pending_pattern, '', text, count=1, flags=re.S)
if pending_count != 1:
    raise SystemExit(f'pending-import transformation marker count: {pending_count}')

certification_pattern = r'''# Hard schema residue gate\..*?print\("current schema reset applied"\)\n'''
text, certification_count = re.subn(
    certification_pattern,
    'print("current schema transformation applied")\n',
    text,
    count=1,
    flags=re.S,
)
if certification_count != 1:
    raise SystemExit(f'schema certification marker count: {certification_count}')

path.write_text(text)
print('current-schema script normalized')
