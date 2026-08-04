---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-03
authority: working
summary: Legacy transitions are receipt-governed and initial SQLite TUI startup now publishes active generation plus first checkpoint atomically. Issue #10 remains open only for transition-edge sediment and visible cutoff semantics.
next: Complete issue #10 through exact remaining transition-edge sediment attribution and visible recovery cutoff/reconstruction semantics.
---

# NOW — Strata

## Current phase

The SQLite migration, startup authority, temporal, domain, reporting, sediment, interaction, category-integrity, active/checkpoint generation-coherence, and legacy switch/finish/clear-all-replay units are complete.

Strata now has:

- durable fail-closed persistence;
- explicit monotonic/UTC/fixed-offset time authority;
- canonical project/category/session identity;
- truthful deterministic reports and exports;
- conserved sediment mass, topology, recovery, snapshots, and daily contributions;
- explicit report-log view/edit ownership and atomic draft persistence;
- exactly-once terminal restoration and runtime emergency checkpoint custody;
- explicit Bound, Unbound, and Disabled action state with one truthful resolver;
- mandatory recovery-safe Ctrl-C and configurable F1;
- cross-authority active/archived category catalogs and strict reference integrity;
- SQLite active switch/reset/finish transactions that retire prior checkpoint generations coherently;
- protected recovering/quarantined evidence that blocks unsafe active transitions;
- startup checkpoint identity validation before payload application;
- immediate current-generation checkpoint refresh after switch, reset, and active-description mutation;
- prepared legacy switch receipts published before session/category effects;
- idempotent switch replay from every durable publication point;
- prepared legacy finish receipts published before active mutation;
- idempotent finish replay across session, catalog, sediment, and daily-contribution effects;
- archived-safe recovery reload/flush and schema-2 emergency exports;
- whole-second/subsecond transition validation consistent with canonical ledger semantics;
- strict legacy session identity and temporal payload validation;
- receipt-governed clear-all that preserves committed history and resets only provisional idle;
- one SQLite clear-all transaction for active, empty sediment, explicit affected days, and resulting checkpoint;
- deterministic legacy clear-all replay that restores exact canonical elapsed and grid state before daily reconstruction;
- six-point SQLite rollback certification and cross-day stale-artifact deletion proofs;
- atomic SQLite initial active/checkpoint bootstrap after sediment restoration;
- four-point bootstrap rollback, pre-existing evidence refusal, and real TUI failure/retry certification.

The project remains in **post-program issue reconciliation**.

## Verified technical baseline

- SQLite schema version 6 is authoritative after explicit activation.
- CLI and TUI share configuration, repository, temporal, session, recovery, snapshot, interaction, and category boundaries.
- Historical sediment viewing is immutable and daily contributions are revision-matched to ledger truth.
- Report-log editing commits only after successful persistence and retains failed drafts in visible recovery.
- `TerminalSession` owns raw mode, alternate screen, cursor restoration, output flushing, and ratatui terminal state.
- Draw, poll, and read errors attempt direct emergency checkpoint publication while preserving the primary error.
- Linux PTY tests certify unchanged termios state and exactly one restoration on quit, detach, draw/poll/read failure, and panic.
- Contextual aliases are named and disabled actions are unreachable through keys, aliases, and the palette.
- Legacy session loading rejects malformed, duplicate, reserved, and unknown identities with actionable errors.
- Category archive/restore preserves original identity, metadata, tags, reports, sand, and migration state.
- SQLite active transitions validate expected active identity and checkpoint custody in one transaction.
- Ordinary transitions may retire only pending/committed evidence for the expected prior stable ID.
- Transition receipts remain idempotent and do not retire later checkpoint generations.
- Missing or mismatched checkpoint identity fails closed before recovery payload application.
- Legacy switch publication uses checkpoint receipt → session CSV → category catalog → receipt clear.
- Receipt replay exact-matches already published rows, rejects conflict, and retains evidence until all authorities converge.
- All three persisted switch crash states converge to one completed interval and the resulting category metadata.

## Completed post-migration units

- **AUTHORITY-001** — issue #21.
- **TEMPORAL-001** — issue #25.
- **TEMPORAL-002** — issues #4, #23, #27.
- **DOMAIN-001** — issues #2, #12.
- **REPORT-001** — issues #1, #3, #14, #17, #28.
- **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
- **INTERACTION-001A** — issue #19.
- **INTERACTION-001B** — issue #20.
- **INTERACTION-001C** — issue #24.
- **RECONCILIATION-001A** — issue #5 and the historical data-loss portion of #13.
- **RECONCILIATION-001B1** — partial issue #10: active/checkpoint generation coherence and semantic-edge refresh.
- **RECONCILIATION-001B2A** — partial issue #10: prepared legacy switch receipts and idempotent kill-point replay.
- **RECONCILIATION-001B2B** — partial issue #10: prepared legacy finish receipts, multi-authority replay, and archived recovery custody.
- **RECONCILIATION-001B2C** — partial issue #10: non-destructive receipt-governed clear-all/provisional-idle reset with atomic SQLite publication and deterministic legacy replay.
- **RECONCILIATION-001B3A** — partial issue #10: atomic initial SQLite active generation and first checkpoint, with rollback and process retry certification.

## Active sequence

1. Complete issue #10 through exact remaining transition-edge sediment reconciliation and user-visible recovery cutoff semantics.
2. Define the merge/reassignment and permanent-deletion transaction needed to complete issue #13.
3. Later domain/UI distinction work under issue #22.
4. Later profile authority, including complete isolation and deliberate switching under issue #15.

## Current risks

- Exact sediment classification at active transition boundaries has not been certified against receipt replay.
- The recovery interface does not yet expose a complete deterministic cutoff and uncertainty statement for reconstructed elapsed time.
- Issue #13 still lacks explicit category merge/reassignment and permanent-deletion transactions.
- Queued checkpoint mutations have no stable cross-authority receipt identity and fail closed.
- Complete profile switching/isolation remains open.
- Active draft versus category metadata remains an unresolved domain/UI distinction.

## Next

Complete the remaining issue #10 units. Certify exact remaining sediment attribution at transition edges, then expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface. After issue #10 closes, return to the category merge/reassignment and permanent-deletion transaction required by issue #13.
