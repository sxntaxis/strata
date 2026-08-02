---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-02
authority: working
summary: Cross-authority category integrity is complete; unknown legacy references fail closed and archival preserves historical meaning, tags, sand, reports, restore, and migration.
next: Reconcile issue #10 against active-session, checkpoint, receipt, emergency-export, and recovery authority.
---

# NOW — Strata

## Current phase

The SQLite migration, startup authority, temporal, domain, reporting, sediment, interaction, and category-integrity programs are complete.

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
- cross-authority active/archived category catalogs;
- strict malformed/unknown session-category rejection without idle substitution;
- archival that preserves stable ID, labels, descriptions, colors, karma, tags, reports, sand, and migration state;
- backward-compatible five-column legacy category loading and six-column active/archived publication.

The project is in **post-program issue reconciliation**.

## Verified technical baseline

- SQLite schema version 6 is authoritative after explicit activation.
- CLI and TUI share configuration, repository, temporal, session, recovery, snapshot, interaction, and category boundaries.
- Historical sediment viewing is immutable and daily contributions are revision-matched to ledger truth.
- Report-log editing commits only after successful persistence and retains failed drafts in visible recovery.
- `TerminalSession` owns raw mode, alternate screen, cursor restoration, output flushing, and ratatui terminal state.
- Draw, poll, and read errors attempt direct emergency checkpoint publication while preserving the primary error.
- Linux PTY tests certify unchanged termios state and exactly one restoration on quit, detach, draw/poll/read failure, and panic.
- Contextual aliases are named and disabled actions are unreachable through keys, aliases, and the palette.
- Legacy `categories.csv` accepts old active-only files and publishes active/archived state without a sidecar.
- Legacy session loading rejects malformed and unknown category IDs with actionable row-scoped errors.
- Session writing refuses unresolved category identities before publication.
- Category archive/restore preserves the original identity and metadata.
- Archived tags survive retirement, restart, and restore.
- Legacy-to-SQLite migration preserves archived state and referenced session identity.

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

## Active sequence

1. Reconcile issue #10 against active-session persistence, detached checkpoints, finish receipts, recovery claims, emergency checkpointing, and user-visible recovery behavior.
2. Define the merge/reassignment and permanent-deletion transaction needed to complete the remaining acceptance criteria of issue #13.
3. Later domain/UI distinction work under issue #22.
4. Later profile authority, including complete isolation and deliberate switching under issue #15.

## Current risks

- Issue #10 may combine obsolete crash-recovery premises with a remaining user-facing uncertainty gap; every crash window needs direct evidence.
- Issue #13 still lacks explicit category merge/reassignment and permanent-deletion transactions.
- Queued checkpoint mutations have no stable cross-authority receipt identity and fail closed.
- Complete profile switching/isolation remains open.
- Active draft versus category metadata remains an unresolved domain/UI distinction.

## Next

Audit **issue #10** criterion by criterion. Map each crash window to active-session authority, checkpoint publication, finish receipts, recovery claims, emergency exports, and process tests. Close only guarantees supported by exact evidence; isolate any remaining recovery or UX defect into a bounded unit.