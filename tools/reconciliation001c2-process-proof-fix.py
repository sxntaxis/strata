from pathlib import Path

path = Path("tests/sqlite_cli_authority.rs")
content = path.read_text()

old_matcher = "rendered.match_indices(prefix).rev()"
new_matcher = "rendered.rmatch_indices(prefix)"
if old_matcher not in content:
    raise SystemExit("confirmation phrase reverse matcher missing")
content = content.replace(old_matcher, new_matcher, 1)

old_navigation = '        send(&mut stdin, b"j");\n'
new_navigation = '        send(&mut stdin, b"\\x1b[B");\n'
if old_navigation not in content:
    raise SystemExit("category modal navigation input missing")
content = content.replace(old_navigation, new_navigation, 1)

path.write_text(content)
