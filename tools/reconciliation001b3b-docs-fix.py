from pathlib import Path

path = Path("tools/reconciliation001b3b-docs.py")
content = path.read_text()
replacements = [
    (
        '''replace(
    "1. Complete issue #10 through exact remaining transition-edge sediment reconciliation and user-visible recovery cutoff semantics.",
    "1. Complete issue #10 through user-visible deterministic recovery cutoff and uncertainty semantics.",
)
''',
        '''replace(
    "notebook/NOW.md",
    "1. Complete issue #10 through exact remaining transition-edge sediment reconciliation and user-visible recovery cutoff semantics.",
    "1. Complete issue #10 through user-visible deterministic recovery cutoff and uncertainty semantics.",
)
''',
    ),
    (
        '''replace(
    "Continue issue #10 after accepted RECONCILIATION-001B3A:\\n\\n1. certify exact remaining sediment classification at active transition boundaries;\\n2. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\\n3. close issue #10 only when repeated restart and crash-during-recovery evidence satisfies its full acceptance criteria.",
    "Continue issue #10 after accepted RECONCILIATION-001B3B:\\n\\n1. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\\n2. certify repeated restart and crash-during-recovery behavior against the visible statement;\\n3. close issue #10 only when the full acceptance criteria are evidence-backed.",
)
''',
        '''replace(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "Continue issue #10 after accepted RECONCILIATION-001B3A:\\n\\n1. certify exact remaining sediment classification at active transition boundaries;\\n2. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\\n3. close issue #10 only when repeated restart and crash-during-recovery evidence satisfies its full acceptance criteria.",
    "Continue issue #10 after accepted RECONCILIATION-001B3B:\\n\\n1. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\\n2. certify repeated restart and crash-during-recovery behavior against the visible statement;\\n3. close issue #10 only when the full acceptance criteria are evidence-backed.",
)
''',
    ),
]
for old, new in replacements:
    if old not in content:
        raise SystemExit("authority replacement marker missing")
    content = content.replace(old, new, 1)
path.write_text(content)
