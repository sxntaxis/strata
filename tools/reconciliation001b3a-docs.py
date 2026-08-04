from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:120]!r}")
    file.write_text(content.replace(old, new, 1))


replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Current completed unit: RECONCILIATION-001B2C",
    "Current completed unit: RECONCILIATION-001B3A",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "## Legacy switch transition receipts\n",
    """## Atomic initial active generation

Under SQLite authority, first TUI startup no longer publishes an active row before its first checkpoint.

- sediment authority is restored and validated before bootstrap publication;
- one typed bootstrap request carries the stable active identity, category, description, UTC start, checkpoint capture time, simulation time, and serialized runtime state;
- one immediate transaction verifies that neither active state nor checkpoint evidence already exists;
- the transaction inserts exactly one active row and one pending checkpoint whose dedicated identity column names the same stable generation;
- pre-existing checkpoint evidence of any status blocks bootstrap and remains unchanged;
- failures at `before-write`, `active`, `checkpoint`, or `commit` leave neither new row durable;
- a failed real TUI startup exits visibly and a later retry may create one clean generation.

The checkpoint identity column owns generation identity; the serialized payload owns runtime state. These are complementary parts of one checkpoint row, not duplicate identity fields. Existing recovery-only active reconstruction remains a separate internal primitive and is not used for ordinary initial startup.

Legacy-file startup remains one atomic checkpoint-file publication and does not gain a second competing active-session authority.

## Legacy switch transition receipts
""",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Legacy switch, normal finish, and clear-all/provisional-idle reset now have certified receipt protocols. Issue #10 remains open for the initial active-start/checkpoint window, exact transition-edge sediment attribution beyond the clear-all contract, and user-visible recovery cutoff/reconstruction semantics.\n\n## Initial active start\n\nSQLite active-session start and first checkpoint publication remain separate operations. The active row is authoritative chronological state, but a process death before the first checkpoint can leave no sediment/runtime evidence for the new active generation.\n\nThis window remains part of a later bounded issue #10 unit. Full closure requires an atomic start-plus-evidence transaction or an explicit certified recovery policy for active rows without checkpoints.\n",
    "Legacy switch, normal finish, clear-all/provisional-idle reset, and initial SQLite active generation now have certified coherence boundaries. Issue #10 remains open only for exact transition-edge sediment attribution beyond the clear-all contract and user-visible recovery cutoff/reconstruction semantics.\n",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "RECONCILIATION-001B1, RECONCILIATION-001B2A, RECONCILIATION-001B2B, and RECONCILIATION-001B2C pass:",
    "RECONCILIATION-001B1, RECONCILIATION-001B2A, RECONCILIATION-001B2B, RECONCILIATION-001B2C, and RECONCILIATION-001B3A pass:",
)
replace("docs/RECOVERY_AUTHORITY.md", "- 215 library tests;", "- 219 library tests;")
replace("docs/RECOVERY_AUTHORITY.md", "- 12 SQLite/TUI process tests;", "- 13 SQLite/TUI process tests;")
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch and finish rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, clear-all receipt identity over canonical elapsed and affected days, exact active-state staging before legacy daily reconstruction, cross-day idle authority, non-idle identity preservation, stale now-empty daily deletion, all six SQLite clear-all transaction kill points, archived-authority reload, and schema-2 emergency export custody.",
    "Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch and finish rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, clear-all receipt identity over canonical elapsed and affected days, exact active-state staging before legacy daily reconstruction, cross-day idle authority, non-idle identity preservation, stale now-empty daily deletion, all six SQLite clear-all transaction kill points, atomic initial active/checkpoint publication, four bootstrap rollback boundaries, pre-existing checkpoint preservation, real TUI failure/retry, archived-authority reload, and schema-2 emergency export custody.",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "- initial active-start/checkpoint coherence;\n- exact sediment classification at transition boundaries;",
    "- exact sediment classification at transition boundaries;",
)

replace(
    "docs/ARCHITECTURE.md",
    "- SQLite switch, reset, and finish validate the expected active stable ID and prior checkpoint custody inside the same transaction.",
    "- SQLite initial startup publishes the first active row and first pending checkpoint in one transaction after sediment restoration; existing active or checkpoint evidence blocks bootstrap.\n- SQLite switch, reset, and finish validate the expected active stable ID and prior checkpoint custody inside the same transaction.",
)
replace(
    "docs/ARCHITECTURE.md",
    "Clear-all/provisional-idle reset uses a third certified receipt boundary. It preserves all committed history, binds canonical prior elapsed and every affected day, restores exact active and grid state before legacy replay derives daily contributions, and applies active/sand/daily/checkpoint effects atomically in SQLite. Six transaction kill points and deterministic legacy replay are certified.\n",
    "Clear-all/provisional-idle reset uses a third certified receipt boundary. It preserves all committed history, binds canonical prior elapsed and every affected day, restores exact active and grid state before legacy replay derives daily contributions, and applies active/sand/daily/checkpoint effects atomically in SQLite. Six transaction kill points and deterministic legacy replay are certified.\n\nInitial SQLite TUI startup uses a typed atomic bootstrap request. The active row and first pending checkpoint share one stable identity column and commit together only after runtime state is staged. Four transaction fault boundaries, pre-existing checkpoint refusal, and real process failure/retry are certified.\n",
)
replace(
    "docs/ARCHITECTURE.md",
    "1. complete issue #10 through initial active-start/checkpoint coherence, exact remaining transition-edge sediment attribution, and visible deterministic recovery cutoff/reconstruction semantics;",
    "1. complete issue #10 through exact remaining transition-edge sediment attribution and visible deterministic recovery cutoff/reconstruction semantics;",
)

replace(
    "docs/DECISIONS.md",
    "| STRATA-D048 | Clear-all is a receipt-governed sediment operation plus provisional-idle reset, never committed-ledger deletion. SQLite publishes active, empty sediment, affected daily contributions, and checkpoint atomically; legacy replay restores exact active/grid state and clears evidence only after convergence. | implemented and certified |",
    "| STRATA-D048 | Clear-all is a receipt-governed sediment operation plus provisional-idle reset, never committed-ledger deletion. SQLite publishes active, empty sediment, affected daily contributions, and checkpoint atomically; legacy replay restores exact active/grid state and clears evidence only after convergence. | implemented and certified |\n| STRATA-D049 | A new SQLite TUI active generation and its first pending checkpoint are one bootstrap transaction after sediment restoration. Existing active or checkpoint evidence blocks bootstrap, and every failed write boundary leaves neither new row durable. | implemented and certified |",
)
replace(
    "docs/DECISIONS.md",
    "- initial active-start/checkpoint atomicity or its explicit recovery policy;\n",
    "",
)

replace(
    "notebook/NOW.md",
    "summary: Legacy switch, finish, and clear-all now use deterministic prepared receipts with idempotent replay; committed history survives clear-all and SQLite publishes the operation atomically. Issue #10 remains open for initial start, remaining sediment-edge, and cutoff semantics.\nnext: Complete issue #10 through initial active-start/checkpoint coherence, exact remaining transition-edge sediment attribution, and visible recovery cutoff/reconstruction semantics.",
    "summary: Legacy transitions are receipt-governed and initial SQLite TUI startup now publishes active generation plus first checkpoint atomically. Issue #10 remains open only for transition-edge sediment and visible cutoff semantics.\nnext: Complete issue #10 through exact remaining transition-edge sediment attribution and visible recovery cutoff/reconstruction semantics.",
)
replace(
    "notebook/NOW.md",
    "- six-point SQLite rollback certification and cross-day stale-artifact deletion proofs.\n",
    "- six-point SQLite rollback certification and cross-day stale-artifact deletion proofs;\n- atomic SQLite initial active/checkpoint bootstrap after sediment restoration;\n- four-point bootstrap rollback, pre-existing evidence refusal, and real TUI failure/retry certification.\n",
)
replace(
    "notebook/NOW.md",
    "- **RECONCILIATION-001B2C** — partial issue #10: non-destructive receipt-governed clear-all/provisional-idle reset with atomic SQLite publication and deterministic legacy replay.\n",
    "- **RECONCILIATION-001B2C** — partial issue #10: non-destructive receipt-governed clear-all/provisional-idle reset with atomic SQLite publication and deterministic legacy replay.\n- **RECONCILIATION-001B3A** — partial issue #10: atomic initial SQLite active generation and first checkpoint, with rollback and process retry certification.\n",
)
replace(
    "notebook/NOW.md",
    "1. Complete issue #10 through initial active-start/checkpoint coherence, exact remaining transition-edge sediment reconciliation, and user-visible recovery cutoff semantics.",
    "1. Complete issue #10 through exact remaining transition-edge sediment reconciliation and user-visible recovery cutoff semantics.",
)
replace(
    "notebook/NOW.md",
    "- SQLite initial active start and first checkpoint publication remain separate operations.\n",
    "",
)
replace(
    "notebook/NOW.md",
    "Complete the remaining issue #10 units. First reconcile initial active-session creation with first checkpoint evidence. Then certify exact remaining sediment attribution at transition edges and expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface.",
    "Complete the remaining issue #10 units. Certify exact remaining sediment attribution at transition edges, then expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface.",
)

replace(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "| #10 | Partially completed by RECONCILIATION-001B1, B2A, B2B, and B2C: SQLite active/checkpoint generations are transactional; legacy switch, finish, and clear-all use deterministic prepared receipts with idempotent replay; clear-all preserves all committed history, resets only provisional idle, binds canonical elapsed and affected days, and publishes active/sand/daily/checkpoint effects atomically in SQLite. Remaining scope is initial active-start/checkpoint coherence, exact remaining transition-edge sediment reconciliation, and explicit recovery-cutoff/uncertainty presentation. | next bounded RECONCILIATION-001B unit |",
    "| #10 | Partially completed by RECONCILIATION-001B1, B2A, B2B, B2C, and B3A: SQLite active/checkpoint generations and initial bootstrap are transactional; legacy switch, finish, and clear-all use deterministic prepared receipts with idempotent replay; clear-all preserves committed history; first TUI startup publishes active state and checkpoint together after sediment restoration. Remaining scope is exact transition-edge sediment reconciliation and explicit recovery-cutoff/uncertainty presentation. | next bounded RECONCILIATION-001B unit |",
)
replace(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "Continue issue #10 after accepted RECONCILIATION-001B2C:\n\n1. reconcile initial active-session creation with first checkpoint evidence;\n2. certify exact remaining sediment classification at active transition boundaries;\n3. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\n4. close issue #10 only when repeated restart and crash-during-recovery evidence satisfies its full acceptance criteria.",
    "Continue issue #10 after accepted RECONCILIATION-001B3A:\n\n1. certify exact remaining sediment classification at active transition boundaries;\n2. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\n3. close issue #10 only when repeated restart and crash-during-recovery evidence satisfies its full acceptance criteria.",
)

replace("notebook/work/RECONCILIATION-001B3A.md", "state: active", "state: accepted")
replace("notebook/work/RECONCILIATION-001B3A.md", "authority: working", "authority: accepted")
with Path("notebook/work/RECONCILIATION-001B3A.md").open("a") as file:
    file.write("""

## Implemented result

- SQLite restores and validates sediment before publishing a new TUI active generation;
- one typed request carries the stable identity and all first-checkpoint state into one immediate transaction;
- the active row and checkpoint identity column commit together under the same stable ID;
- any existing active row or checkpoint evidence blocks bootstrap without overwrite;
- the recovery-only standalone active-start primitive remains available internally but is no longer used by ordinary first startup;
- failures at `before-write`, `active`, `checkpoint`, and `commit` leave no orphan active row or checkpoint;
- the real TUI process fails visibly under each injected boundary and succeeds cleanly on retry.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 219 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 13 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- temporary transformation and workflow machinery: absent from the permanent tree.

The unit is accepted as a partial completion of issue #10. Exact transition-edge sediment attribution and user-visible deterministic recovery cutoff/reconstruction semantics remain unresolved.
""")
