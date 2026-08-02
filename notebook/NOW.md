---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: Legacy switch transitions now use prepared schema-3 receipts and idempotent kill-point replay; issue #10 remains open for reset/finish, initial start, sediment-edge, and cutoff semantics.
next: Implement RECONCILIATION-001B2B for remaining active-transition and recovery-presentation gaps.
---

# NOW — Strata

## Current phase

The SQLite migration, startup authority, temporal, domain, reporting, sediment, interaction, category-integrity, active/checkpoint generation-coherence, and legacy switch-replay units are complete.

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
- whole-second/subsecond transition validation consistent with canonical ledger semantics;
- strict legacy session identity and temporal payload validation.

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

## Active sequence

1. Implement RECONCILIATION-001B2B: legacy reset/finish receipts, initial active-start/checkpoint coherence, transition-edge sediment reconciliation, and user-visible recovery cutoff semantics.
2. Define the merge/reassignment and permanent-deletion transaction needed to complete issue #13.
3. Later domain/UI distinction work under issue #22.
4. Later profile authority, including complete isolation and deliberate switching under issue #15.

## Current risks

- Legacy reset and finish still cross separate authority files without the certified switch receipt protocol.
- SQLite initial active start and first checkpoint publication remain separate operations.
- Exact sediment classification at active transition boundaries has not been certified against receipt replay.
- The recovery interface does not yet expose a complete deterministic cutoff and uncertainty statement for reconstructed elapsed time.
- Issue #13 still lacks explicit category merge/reassignment and permanent-deletion transactions.
- Queued checkpoint mutations have no stable cross-authority receipt identity and fail closed.
- Complete profile switching/isolation remains open.
- Active draft versus category metadata remains an unresolved domain/UI distinction.

## Next

Implement **RECONCILIATION-001B2B**. Extend receipt custody only where the operation semantics justify it, certify every durable reset/finish publication point, reconcile initial active-start evidence, bind sediment contribution to the same transition boundary, and expose checkpoint capture, recovery target, reconstructed duration, and deterministic cutoff policy before issue #10 can close.
