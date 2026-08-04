---
id: RECONCILIATION-001B3B
kind: work
state: accepted
authority: accepted
created: 2026-08-03
updated: 2026-08-03
---

# RECONCILIATION-001B3B — exact transition-edge sediment attribution

## Issue

The runtime advances simulation before polling input, but an input event can arrive after that advance. An immediate layer switch, clear, or normal finish then uses the later UTC/monotonic boundary while canonical sediment may still stop at the earlier simulation timestamp. Any spawn due inside that gap is processed later under the resulting category—or omitted on finish—even though the chronological ledger assigns the elapsed second to the outgoing interval.

Queued mutations already replay simulation to their recorded timestamp before applying the mutation. Immediate mutations and normal finish must obey the same boundary rule.

## Selected contract

- A spawn due exactly at a transition timestamp belongs to the outgoing active interval and category.
- Before an immediate switch, clear-all, idle-sediment clear, or normal finish, simulation settles through the chosen boundary under the pre-transition category.
- Settlement advances canonical simulation time and both periodic remainders to the same boundary used by chronological reconciliation.
- Missed spawn mass is derived with checked bounded arithmetic and appended in category-preserving FIFO form; transition settlement never requires one loop iteration per elapsed second.
- Physics events skipped by bounded settlement are explicitly projection-only loss, not sediment-mass or category loss; canonical topology already present is preserved.
- Queued mutations at or before the boundary are applied in timestamp order, with each preceding segment settled under the category authoritative for that segment.
- A clear operation runs only after pre-boundary mass is settled, so grains due before the clear cannot reappear afterward.
- If settlement or an intervening queued mutation fails, the requested transition does not proceed and existing recovery controls retain the authoritative state.

## Acceptance proofs

- a grain due exactly at a switch boundary is attributed to the outgoing category;
- the first grain after the boundary is attributed to the resulting category;
- immediate and queued switch paths produce the same category run ordering;
- normal finish settles all due outgoing mass before session finalization;
- clear-all and idle clear cannot leave pre-clear elapsed mass to spawn afterward;
- settlement preserves existing topology, total logical mass, periodic remainders, and FIFO category order;
- large boundary gaps use bounded arithmetic rather than iterative second replay;
- all recovery, receipt, SQLite/TUI, CLI, and PTY suites remain green.

## Boundary

This unit does not close issue #10. Visible checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty semantics remain the final bounded recovery unit.

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
- permanent nine-file source, authority, and Notebook diff audit: pass;
- temporary transformation, diagnostic, and workflow machinery: absent from the permanent tree.

The unit is accepted as a partial completion of issue #10. User-visible deterministic recovery cutoff, reconstruction, and uncertainty semantics remain the final unresolved boundary.
