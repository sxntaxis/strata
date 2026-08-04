from pathlib import Path

path = Path("tests/sqlite_cli_authority.rs")
content = path.read_text()
old = "rendered.match_indices(prefix).rev()"
new = "rendered.rmatch_indices(prefix)"
if old not in content:
    raise SystemExit("confirmation phrase reverse matcher missing")
path.write_text(content.replace(old, new, 1))
