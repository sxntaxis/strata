from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:160]!r}")
    file.write_text(content.replace(old, new, 1))


replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Status: partially implemented and certified\nCurrent completed unit: RECONCILIATION-001B3B\nIssue in progress: #10",
    "Status: implemented and certified\nCurrent completed unit: RECONCILIATION-001B3C\nCompleted issue: #10",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Legacy recovery flush and reload preserve active and archived category catalogs, archived session references, and archived sediment identities. Emergency recovery JSON schema 2 includes every category with an explicit `archived` flag.",
    "Legacy recovery flush and reload preserve active and archived category catalogs, archived session references, and archived sediment identities. Emergency recovery JSON schema 3 includes every category with an explicit `archived` flag and carries the structured recovery statement when one exists.",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "## Remaining issue #10 recovery work\n\nLegacy switch, normal finish, clear-all/provisional-idle reset, initial SQLite active generation, and transition-edge sediment now have certified coherence boundaries. Issue #10 remains open only for user-visible recovery cutoff/reconstruction semantics.\n\n## User-visible recovery cutoff\n\nCurrent checkpoint recovery reconstructs from persisted checkpoint evidence toward a fixed recovery target. The final product contract must expose:\n\n- checkpoint capture time;\n- recovery target time;\n- active category and description;\n- reconstructed elapsed duration;\n- whether the interval is exact, provisional, or reconstructed;\n- the deterministic cutoff policy applied.\n\nThis presentation and policy remain unresolved. Recovery authority must not imply that elapsed time after the last durable evidence is exact without showing its reconstruction basis.\n",
    """## Visible deterministic recovery cutoff

Every successful checkpoint recovery now creates one structured acknowledgment statement before ordinary controls resume.

The statement exposes:

- active stable identity under SQLite, or explicit legacy-file generation wording;
- active category, description, and original active-session UTC start;
- checkpoint capture UTC;
- checkpoint simulation UTC, the last sediment instant represented directly by the durable payload;
- one recovery target UTC persisted before bounded reconstruction;
- reconstructed duration from simulation UTC through target UTC;
- recovered-interval classification: `exact` when reconstruction duration is zero, otherwise `reconstructed`;
- post-target classification: `provisional live time`;
- the cutoff policy: retry reuses the persisted target, and no later live time is counted as recovered history.

Chronology validation requires:

```text
active start <= durable simulation <= checkpoint capture <= recovery target
```

Impossible ordering fails closed before recovery is presented as successful.

The acknowledgment modal blocks ordinary controls until Enter or Esc. Mandatory emergency quit remains available. Visible persistence recovery has higher priority and preserves the statement for later acknowledgment.

SQLite claim/retry reuses `recovery_target_utc` already persisted in the checkpoint payload. A failed recovery commit followed by a later restart cannot move the target forward merely because wall time advanced. Process certification waits beyond the original target, retries the recovering checkpoint, and verifies the modal still displays the original UTC cutoff and `RECONSTRUCTED -> PROVISIONAL LIVE TIME` classification.

Emergency recovery export schema 3 serializes the same structured statement. The modal and export therefore project one evidence object rather than separately reconstructed UI text.

## Issue #10 closure

Issue #10 is complete. Its original acceptance obligations are now covered by:

- bounded, topology-preserving reconstruction without unbounded physics replay;
- active/checkpoint identity and transaction coherence;
- prepared legacy switch, finish, and clear-all receipts with idempotent kill-point replay;
- non-destructive clear-all custody;
- atomic initial SQLite active generation and first checkpoint;
- exact outgoing-category sediment settlement at transition boundaries;
- persisted deterministic cutoff reuse;
- visible exact/reconstructed/provisional classification;
- structured emergency export evidence;
- repeated process failure/restart certification.

Future queued-mutation replay, if ever introduced, still requires stable cross-authority receipt identity. It is not an unimplemented part of the current recovery contract because unsupported queued checkpoint mutations continue to fail closed.
""",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "RECONCILIATION-001B1, RECONCILIATION-001B2A, RECONCILIATION-001B2B, RECONCILIATION-001B2C, RECONCILIATION-001B3A, and RECONCILIATION-001B3B pass:",
    "RECONCILIATION-001B1, RECONCILIATION-001B2A, RECONCILIATION-001B2B, RECONCILIATION-001B2C, RECONCILIATION-001B3A, RECONCILIATION-001B3B, and RECONCILIATION-001B3C pass:",
)
replace("docs/RECOVERY_AUTHORITY.md", "- 223 library tests;", "- 228 library tests;")
replace("docs/RECOVERY_AUTHORITY.md", "- 13 SQLite/TUI process tests;", "- 14 SQLite/TUI process tests;")
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch and finish rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, clear-all receipt identity over canonical elapsed and affected days, exact active-state staging before legacy daily reconstruction, cross-day idle authority, non-idle identity preservation, stale now-empty daily deletion, all six SQLite clear-all transaction kill points, atomic initial active/checkpoint publication, four bootstrap rollback boundaries, pre-existing checkpoint preservation, real TUI failure/retry, exact outgoing-category boundary attribution, post-clear non-reappearance, billion-second bounded settlement, uninitialized-canvas mass preservation, archived-authority reload, and schema-2 emergency export custody.",
    "Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch and finish rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, clear-all receipt identity over canonical elapsed and affected days, exact active-state staging before legacy daily reconstruction, cross-day idle authority, non-idle identity preservation, stale now-empty daily deletion, all six SQLite clear-all transaction kill points, atomic initial active/checkpoint publication, four bootstrap rollback boundaries, pre-existing checkpoint preservation, real TUI failure/retry, exact outgoing-category boundary attribution, post-clear non-reappearance, billion-second bounded settlement, uninitialized-canvas mass preservation, monotonic recovery-statement chronology, exact versus reconstructed classification, persisted cutoff reuse after failed commit and delayed retry, acknowledgment input custody, archived-authority reload, and schema-3 emergency export parity.",
)
replace(
    "docs/RECOVERY_AUTHORITY.md",
    "## Unresolved boundary\n\nFull crash-recovery authority still requires:\n\n- explicit user-visible recovery cutoff and uncertainty semantics;\n- any future safe queued-mutation replay based on stable cross-authority receipts.\n",
    "## Unsupported future extension\n\nSafe replay of queued checkpoint mutations would require stable cross-authority receipt identity. Current unsupported queued mutation evidence fails closed and is not represented as recoverable authority.\n",
)

replace(
    "docs/ARCHITECTURE.md",
    "- `src/app.rs` and `src/app/**` — TUI orchestration, active/archived category projections, semantic-edge checkpoint refresh, legacy switch/finish/clear-all receipt publication and replay, explicit modal/edit state, persistence reconciliation, bounded recovery, historical artifact selection, context selection, resolver execution, palette/atlas projection, and rendering.\n- `src/app/terminal_lifecycle.rs`",
    "- `src/app.rs` and `src/app/**` — TUI orchestration, active/archived category projections, semantic-edge checkpoint refresh, legacy switch/finish/clear-all receipt publication and replay, explicit modal/edit state, persistence reconciliation, bounded recovery, historical artifact selection, context selection, resolver execution, palette/atlas projection, and rendering.\n- `src/app/recovery_statement.rs` — blocking recovery-evidence acknowledgment, deterministic cutoff presentation, exact/reconstructed/provisional classification, and shared projection of the structured recovery statement.\n- `src/app/terminal_lifecycle.rs`",
)
replace(
    "docs/ARCHITECTURE.md",
    "Legacy recovery flush/reload validate both active and archived catalogs, retain archived sediment identity, and emergency recovery schema 2 exports explicit archival state.",
    "Legacy recovery flush/reload validate both active and archived catalogs, retain archived sediment identity, and emergency recovery schema 3 exports explicit archival state plus the structured recovery statement when present.",
)
replace(
    "docs/ARCHITECTURE.md",
    "Immediate and queued switches, clear operations, and normal finish settle sediment to the same UTC boundary used by chronological reconciliation before changing active state. Exact-boundary mass belongs to the outgoing category; bounded compressed settlement preserves category order and topology, and fresh `0×0` live canvases retain due mass without weakening persisted-checkpoint validation.\n\nThe recovery contract and remaining issue #10 boundary are recorded in `docs/RECOVERY_AUTHORITY.md`.",
    "Immediate and queued switches, clear operations, and normal finish settle sediment to the same UTC boundary used by chronological reconciliation before changing active state. Exact-boundary mass belongs to the outgoing category; bounded compressed settlement preserves category order and topology, and fresh `0×0` live canvases retain due mass without weakening persisted-checkpoint validation.\n\nSuccessful checkpoint recovery produces one structured evidence statement. It exposes durable simulation time, capture time, persisted target, reconstructed duration, active identity, and exact/reconstructed/provisional classification; blocks ordinary input until acknowledgment; and is serialized unchanged in emergency export schema 3. Retry reuses the persisted cutoff.\n\nThe complete recovery contract and issue #10 closure are recorded in `docs/RECOVERY_AUTHORITY.md`.",
)
replace(
    "docs/ARCHITECTURE.md",
    "### Runtime recovery\n\nOwns checkpoint evidence and exact elapsed contribution since the checkpoint. It may add mass and advance accumulator remainders, but may not replay unbounded physics, relax topology, discard protected evidence, or apply payload state to a different active generation.",
    "### Runtime recovery\n\nOwns checkpoint evidence, bounded elapsed contribution since durable simulation time, one persisted target, and the resulting recovery statement. It may add mass and advance accumulator remainders, but may not replay unbounded physics, relax topology, discard protected evidence, apply payload state to a different active generation, move a persisted cutoff forward on retry, or label post-target live time as recovered history.",
)
replace(
    "docs/ARCHITECTURE.md",
    "Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, active/checkpoint generation coherence, and legacy switch/finish/clear-all replay are complete. The next priorities are:\n\n1. complete issue #10 through visible deterministic recovery cutoff/reconstruction semantics;\n2. design the explicit merge/reassignment and permanent-deletion remainder of issue #13;\n3. later domain/UI distinction work under issue #22;\n4. later profile authority, including complete isolation and deliberate switching under issue #15.",
    "Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, and crash-recovery authority are complete. The next priorities are:\n\n1. design the explicit merge/reassignment and permanent-deletion remainder of issue #13;\n2. later domain/UI distinction work under issue #22;\n3. later profile authority, including complete isolation and deliberate switching under issue #15.",
)
replace(
    "docs/ARCHITECTURE.md",
    "- A checkpoint without visible cutoff semantics is not proof of exact post-capture elapsed time.\n",
    "",
)

replace(
    "docs/DECISIONS.md",
    "| STRATA-D050 | Sediment settles through the exact chronological transition timestamp under the outgoing category before switch, clear, or finish. Exact-boundary mass is outgoing; later mass is resulting; bounded FIFO settlement preserves mass without iterative replay. | implemented and certified |",
    "| STRATA-D050 | Sediment settles through the exact chronological transition timestamp under the outgoing category before switch, clear, or finish. Exact-boundary mass is outgoing; later mass is resulting; bounded FIFO settlement preserves mass without iterative replay. | implemented and certified |\n| STRATA-D051 | Checkpoint recovery owns one persisted target reused across retry. Successful recovery must visibly distinguish durable evidence, reconstructed time through that cutoff, and post-target provisional live time; emergency export projects the same structured statement. | implemented and certified |",
)
replace(
    "docs/DECISIONS.md",
    "- user-visible recovery cutoff, reconstruction, and uncertainty semantics under issue #10;\n",
    "",
)

replace(
    "notebook/NOW.md",
    "summary: Recovery transitions, initial bootstrap, and exact transition-edge sediment are certified. Issue #10 remains open only for visible deterministic recovery cutoff and uncertainty semantics.\nnext: Complete issue #10 by exposing checkpoint capture, recovery target, reconstructed duration, cutoff policy, and uncertainty.",
    "summary: Crash-recovery authority is complete: identity, receipts, clear-all, initial bootstrap, exact transition edges, persisted cutoff reuse, visible uncertainty, and export parity are certified.\nnext: Define the category merge/reassignment and permanent-deletion transaction required to complete issue #13.",
)
replace(
    "notebook/NOW.md",
    "- bounded FIFO transition settlement, post-clear non-reappearance, and uninitialized-canvas mass preservation.\n",
    "- bounded FIFO transition settlement, post-clear non-reappearance, and uninitialized-canvas mass preservation;\n- blocking recovery evidence acknowledgment with durable simulation, capture, target, reconstructed duration, and active identity;\n- exact/reconstructed/provisional classification with persisted cutoff reuse across failed commit and delayed retry;\n- emergency recovery schema 3 parity with the visible structured statement.\n",
)
replace(
    "notebook/NOW.md",
    "- **RECONCILIATION-001B3B** — partial issue #10: exact bounded sediment settlement at immediate, queued, clear, and finish boundaries.\n",
    "- **RECONCILIATION-001B3B** — partial issue #10: exact bounded sediment settlement at immediate, queued, clear, and finish boundaries.\n- **RECONCILIATION-001B3C** — completed issue #10: persisted deterministic cutoff, visible exact/reconstructed/provisional evidence, acknowledgment custody, repeated-retry proof, and schema-3 export parity.\n",
)
replace(
    "notebook/NOW.md",
    "1. Complete issue #10 through user-visible deterministic recovery cutoff and uncertainty semantics.\n2. Define the merge/reassignment and permanent-deletion transaction needed to complete issue #13.\n3. Later domain/UI distinction work under issue #22.\n4. Later profile authority, including complete isolation and deliberate switching under issue #15.",
    "1. Define the merge/reassignment and permanent-deletion transaction needed to complete issue #13.\n2. Later domain/UI distinction work under issue #22.\n3. Later profile authority, including complete isolation and deliberate switching under issue #15.",
)
replace(
    "notebook/NOW.md",
    "- The recovery interface does not yet expose a complete deterministic cutoff and uncertainty statement for reconstructed elapsed time.\n",
    "",
)
replace(
    "notebook/NOW.md",
    "Complete the final issue #10 unit by exposing checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface. After issue #10 closes, return to the category merge/reassignment and permanent-deletion transaction required by issue #13.",
    "Define the category merge/reassignment and permanent-deletion transaction required to complete issue #13. Preserve historical meaning, refuse ambiguous destructive operations, and certify both SQLite and legacy authority before closure.",
)

replace(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "| #10 | Partially completed by RECONCILIATION-001B1, B2A, B2B, B2C, B3A, and B3B: active/checkpoint generations, legacy replay, clear-all, initial bootstrap, and exact transition-edge sediment are certified. Remaining scope is explicit recovery-cutoff/reconstruction/uncertainty presentation. | RECONCILIATION-001B3C |",
    "| #10 | Completed by RECONCILIATION-001B1, B2A, B2B, B2C, B3A, B3B, and B3C: active/checkpoint generations, prepared legacy replay, non-destructive clear-all, atomic initial bootstrap, exact transition-edge sediment, persisted cutoff reuse, visible recovery classification, repeated restart proof, and schema-3 export parity are certified. | none |",
)
replace(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "Continue issue #10 after accepted RECONCILIATION-001B3B:\n\n1. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;\n2. certify repeated restart and crash-during-recovery behavior against the visible statement;\n3. close issue #10 only when the full acceptance criteria are evidence-backed.\n\nAfter issue #10 reaches evidence-based closure, return to the merge/reassignment transaction required to complete issue #13.",
    "Issue #10 is evidence-backed and complete. Next:\n\n1. define category merge/reassignment semantics for issue #13;\n2. define permanent deletion preconditions and fail-closed refusal paths;\n3. certify historical session, sediment, snapshot, tag, and migration meaning under SQLite and legacy authority.\n\nDo not treat archive as deletion or invent reassignment for unresolved references.",
)

replace("notebook/work/RECONCILIATION-001B3C.md", "state: active", "state: accepted")
replace("notebook/work/RECONCILIATION-001B3C.md", "authority: working", "authority: accepted")
with Path("notebook/work/RECONCILIATION-001B3C.md").open("a") as file:
    file.write("""

## Implemented result

- successful checkpoint recovery builds one structured evidence statement from the claimed checkpoint and persisted target;
- chronology fails closed unless active start, durable simulation, capture, and target are monotonic;
- the statement exposes active identity, category, description, start, capture, durable simulation, target, reconstructed duration, recovered classification, post-target classification, and cutoff policy;
- exact zero-duration recovery is distinguished from reconstructed recovery;
- post-target time is always labeled `provisional live time` and is not folded into recovered history;
- ordinary controls remain blocked until Enter or Esc acknowledgment, while mandatory emergency quit and higher-priority persistence recovery remain available;
- a failed recovery commit retains the target in recovering evidence; delayed retry reuses and displays that original cutoff;
- emergency recovery export schema 3 carries the same structured statement and classifications.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 228 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 14 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- repeated failed-commit/delayed-retry process proof: pass;
- emergency export schema/value parity proof: pass;
- temporary transformation, audit, and workflow machinery: absent from the permanent tree.

RECONCILIATION-001B3C completes issue #10. Crash recovery now has evidence-backed identity, atomicity, replay, bounded reconstruction, exact transition edges, deterministic cutoff reuse, visible uncertainty, acknowledgment custody, and export parity.
""")
