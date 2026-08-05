from pathlib import Path
import re

path = Path('.github/scripts/apply_current_schema.py')
text = path.read_text()
pattern = r'''text = sub_once\(\n    text,\n    r"\\n    let pending_imports = if existing_tables.*?    "pending import doctor check",\n\)\n'''
text, count = re.subn(pattern, '', text, count=1, flags=re.S)
if count != 1:
    raise SystemExit(f'pending-import transformation marker count: {count}')
path.write_text(text)
print('current-schema script normalized')
