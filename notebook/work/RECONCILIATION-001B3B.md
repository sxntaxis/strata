---
id: RECONCILIATION-001B3B
kind: work
state: active
authority: working
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
