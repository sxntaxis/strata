from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:140]!r}")
    file.write_text(content.replace(old, new, 1))


replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Current completed unit: RECONCILIATION-001B3A",
    "Current completed unit: RECONCILIATION-001B3B",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "## Bounded sediment recovery\n",
    """## Exact transition-edge sediment

Chronological transition timestamps and sediment formation now share one boundary.

- before an immediate switch, clear, provisional-idle reset, or normal finish, simulation settles through the selected UTC boundary under the outgoing active category;
- a grain due exactly at the boundary belongs to the outgoing interval; the first later grain belongs to the resulting category;
- queued mutations at or before the boundary are processed in timestamp order after each preceding segment is settled under its then-authoritative category;
- settlement uses checked periodic arithmetic and compressed FIFO runs, never one replay iteration per missed second;
- existing canonical topology is preserved and skipped physics is explicit projection loss rather than mass or category loss;
- clear operations occur only after pre-boundary mass is settled, so cleared elapsed mass cannot reappear afterward;
- an uninitialized live `0×0` canvas preserves due mass as pending runs without inventing dimensions, while detached persisted-checkpoint recovery continues to reject an empty canvas;
- settlement failure blocks the requested transition and enters existing visible recovery custody.

The same settlement helper owns immediate and queued mutation boundaries. Normal finish settles before ledger reconciliation, so completed history and canonical sediment terminate at one timestamp.

## Bounded sediment recovery
""",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Legacy switch, normal finish, clear-all/provisional-idle reset, and initial SQLite active generation now have certified coherence boundaries. Issue #10 remains open only for exact transition-edge sediment attribution beyond the clear-all contract and user-visible recovery cutoff/reconstruction semantics.",
    "Legacy switch, normal finish, clear-all/provisional-idle reset, initial SQLite active generation, and transition-edge sediment now have certified coherence boundaries. Issue #10 remains open only for user-visible recovery cutoff/reconstruction semantics.",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "RECONCILIATION-001B1, RECONCILIATION-001B2A, RECONCILIATION-001B2B, RECONCILIATION-001B2C, and RECONCILIATION-001B3A pass:",
    "RECONCILIATION-001B1, RECONCILIATION-001B2A, RECONCILIATION-001B2B, RECONCILIATION-001B2C, RECONCILIATION-001B3A, and RECONCILIATION-001B3B pass:",
)
replace("docs/RECOVERY_AUTHORITY.md", "- 219 library tests;", "- 223 library tests;")
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch and finish rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, clear-all receipt identity over canonical elapsed and affected days, exact active-state staging before legacy daily reconstruction, cross-day idle authority, non-idle identity preservation, stale now-empty daily deletion, all six SQLite clear-all transaction kill points, atomic initial active/checkpoint publication, four bootstrap rollback boundaries, pre-existing checkpoint preservation, real TUI failure/retry, archived-authority reload, and schema-2 emergency export custody.",
    "Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch and finish rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, clear-all receipt identity over canonical elapsed and affected days, exact active-state staging before legacy daily reconstruction, cross-day idle authority, non-idle identity preservation, stale now-empty daily deletion, all six SQLite clear-all transaction kill points, atomic initial active/checkpoint publication, four bootstrap rollback boundaries, pre-existing checkpoint preservation, real TUI failure/retry, exact outgoing-category boundary attribution, post-clear non-reappearance, billion-second bounded settlement, uninitialized-canvas mass preservation, archived-authority reload, and schema-2 emergency export custody.",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "- exact sediment classification at transition boundaries;\n- explicit user-visible recovery cutoff and uncertainty semantics;",
    "- explicit user-visible recovery cutoff and uncertainty semantics;",
)

replace(
    "docs/ARCHITECTURE.md",
    "- `src/sand/recovery.rs` — bounded recovery arithmetic and topology-preserving detached contribution.",
    "- `src/sand/recovery.rs` — bounded recovery arithmetic, topology-preserving detached contribution, and exact transition-boundary settlement with separate initialized/uninitialized canvas policy.",
)
replace(
    "docs/ARCHITECTURE.md",
    "Initial SQLite TUI startup uses a typed atomic bootstrap request. The active row and first pending checkpoint share one stable identity column and commit together only after runtime state is staged. Four transaction fault boundaries, pre-existing checkpoint refusal, and real process failure/retry are certified.\n",
    "Initial SQLite TUI startup uses a typed atomic bootstrap request. The active row and first pending checkpoint share one stable identity column and commit together only after runtime state is staged. Four transaction fault boundaries, pre-existing checkpoint refusal, and real process failure/retry are certified.\n\nImmediate and queued switches, clear operations, and normal finish settle sediment to the same UTC boundary used by chronological reconciliation before changing active state. Exact-boundary mass belongs to the outgoing category; bounded compressed settlement preserves category order and topology, and fresh `0×0` live canvases retain due mass without weakening persisted-checkpoint validation.\n",
)
replace(
    "docs/ARCHITECTURE.md",
    "1. complete issue #10 through exact remaining transition-edge sediment attribution and visible deterministic recovery cutoff/reconstruction semantics;",
    "1. complete issue #10 through visible deterministic recovery cutoff/reconstruction semantics;",
)

replace(
    "docs/DECISIONS.md",
    "| STRATA-D049 | A new SQLite TUI active generation and its first pending checkpoint are one bootstrap transaction after sediment restoration. Existing active or checkpoint evidence blocks bootstrap, and every failed write boundary leaves neither new row durable. | implemented and certified |",
    "| STRATA-D049 | A new SQLite TUI active generation and its first pending checkpoint are one bootstrap transaction after sediment restoration. Existing active or checkpoint evidence blocks bootstrap, and every failed write boundary leaves neither new row durable. | implemented and certified |\n| STRATA-D050 | Sediment settles through the exact chronological transition timestamp under the outgoing category before switch, clear, or finish. Exact-boundary mass is outgoing; later mass is resulting; bounded FIFO settlement preserves mass without iterative replay. | implemented and certified |",
)
replace("docs/DECISIONS.md", "- exact sediment classification at active transition boundaries;\n", "")

replace(
    "notebook/NOW.md",
    "summary: Legacy transitions are receipt-governed and initial SQLite TUI startup now publishes active generation plus first checkpoint atomically. Issue #10 remains open only for transition-edge sediment and visible cutoff semantics.\nnext: Complete issue #10 through exact remaining transition-edge sediment attribution and visible recovery cutoff/reconstruction semantics.",
    "summary: Recovery transitions, initial bootstrap, and exact transition-edge sediment are certified. Issue #10 remains open only for visible deterministic recovery cutoff and uncertainty semantics.\nnext: Complete issue #10 by exposing checkpoint capture, recovery target, reconstructed duration, cutoff policy, and uncertainty.",
)
replace(
    "notebook/NOW.md",
    "- four-point bootstrap rollback, pre-existing evidence refusal, and real TUI failure/retry certification.\n",
    "- four-point bootstrap rollback, pre-existing evidence refusal, and real TUI failure/retry certification;\n- exact outgoing-category sediment ownership at switch, clear, and finish boundaries;\n- bounded FIFO transition settlement, post-clear non-reappearance, and uninitialized-canvas mass preservation.\n",
)
replace(
    "notebook/NOW.md",
    "- **RECONCILIATION-001B3A** — partial issue #10: atomic initial SQLite active generation and first checkpoint, with rollback and process retry certification.\n",
    "- **RECONCILIATION-001B3A** — partial issue #10: atomic initial SQLite active generation and first checkpoint, with rollback and process retry certification.\n- **RECONCILIATION-001B3B** — partial issue #10: exact bounded sediment settlement at immediate, queued, clear, and finish boundaries.\n",
)
replace(
    "1. Complete issue #10 through exact remaining transition-edge sediment reconciliation and user-visible recovery cutoff semantics.",
    "1. Complete issue #10 through user-visible deterministic recovery cutoff and uncertainty semantics.",
)
replace("notebook/NOW.md", "- Exact sediment classification at active transition boundaries has not been certified against receipt replay.\n", "")
replace(
    "notebook/NOW.md",
    "Complete the remaining issue #10 units. Certify exact remaining sediment attribution at transition edges, then expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface. After issue #10 closes, return to the category merge/reassignment and permanent-deletion transaction required by issue #13.",
    "Complete the final issue #10 unit by exposing checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface. After issue #10 closes, return to the category merge/reassignment and permanent-deletion transaction required by issue #13.",
)

replace(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "| #10 | Partially completed by RECONCILIATION-001B1, B2A, B2B, B2C, and B3A: SQLite active/checkpoint generations and initial bootstrap are transactional; legacy switch, finish, and clear-all use deterministic prepared receipts with idempotent replay; clear-all preserves committed history; first TUI startup publishes active state and checkpoint together after sediment restoration. Remaining scope is exact transition-edge sediment reconciliation and explicit recovery-cutoff/uncertainty presentation. | next bounded RECONCILIATION-001B unit |",
    "| #10 | Partially completed by RECONCILIATION-001B1, B2A, B2B, B2C, B3A, and B3B: active/checkpoint generations, legacy replay, clear-all, initial bootstrap, and exact transition-edge sediment are certified. Remaining scope is explicit recovery-cutoff/reconstruction/uncertainty presentation. | RECONCILIATION-001B3C |",
)
replace(
    "Continue issue #10 after accepted RECONCILIATION-001B3A:\n\n1. certify exact remaining sediment classification at active transition boundaries;\n2. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\n3. close issue #10 only when repeated restart and crash-during-recovery evidence satisfies its full acceptance criteria.",
    "Continue issue #10 after accepted RECONCILIATION-001B3B:\n\n1. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\n2. certify repeated restart and crash-during-recovery behavior against the visible statement;\n3. close issue #10 only when the full acceptance criteria are evidence-backed.",
)

replace("notebook/work/RECONCILIATION-001B3B.md", "state: active", "state: accepted")
replace("notebook/work/RECONCILIATION-001B3B.md", "authority: working", "authority: accepted")
with Path("notebook/work/RECONCILIATION-001B3B.md").open("a") as file:
    file.write("""

## Implemented result

- immediate switch, clear, provisional-idle reset, and normal finish settle simulation through one selected UTC boundary before changing chronological authority;
- queued mutations at or before that boundary remain timestamp-ordered and each preceding segment settles under its authoritative category;
- a grain due exactly at the boundary belongs to the outgoing category and the first later grain belongs to the resulting category;
- settlement uses checked periodic arithmetic and compressed pending runs, including billion-second gaps without iterative replay;
- existing canonical topology and FIFO category order are preserved while skipped physics remains explicit projection loss;
- pre-clear elapsed mass is settled before clearing and cannot reappear afterward;
- live uninitialized `0×0` canvases retain due mass as pending runs without invented dimensions;
- detached persisted-checkpoint recovery remains strict and continues to reject an empty canvas;
- settlement failure blocks the requested transition under visible persistence recovery.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 223 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 13 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- temporary transformation, diagnostic, and workflow machinery: absent from the permanent tree.

The unit is accepted as a partial completion of issue #10. User-visible deterministic recovery cutoff, reconstruction, and uncertainty semantics remain the final unresolved boundary.
""")
