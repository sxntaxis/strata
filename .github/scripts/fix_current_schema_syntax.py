from pathlib import Path
import re

path = Path("src/sqlite/tui_runtime.rs")
text = path.read_text()
text, count = re.subn(
    r"(?m)^(\s*)repository\n\1repository\n",
    r"\1repository\n",
    text,
)
if count != 6:
    raise SystemExit(f"expected six duplicate repository residues, found {count}")
path.write_text(text)
print("current-schema syntax residues removed")
