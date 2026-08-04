from pathlib import Path

path = Path("tools/reconciliation001b3b-docs.py")
content = path.read_text()
old = '''replace(
    "1. Complete issue #10 through exact remaining transition-edge sediment reconciliation and user-visible recovery cutoff semantics.",
    "1. Complete issue #10 through user-visible deterministic recovery cutoff and uncertainty semantics.",
)
'''
new = '''replace(
    "notebook/NOW.md",
    "1. Complete issue #10 through exact remaining transition-edge sediment reconciliation and user-visible recovery cutoff semantics.",
    "1. Complete issue #10 through user-visible deterministic recovery cutoff and uncertainty semantics.",
)
'''
if old not in content:
    raise SystemExit("Notebook active-sequence replacement marker missing")
path.write_text(content.replace(old, new, 1))
