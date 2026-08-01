---
id: CONCEPT-001
kind: work
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Resolve the remaining product and interaction questions before persistence and recovery implementation.
---

# CONCEPT-001 — Product concept clarification

## Outcome

Define the interaction and historical contracts that let Strata remain a continuous ledger, active timer, low-power temporal system, exact chronological record, accountable sediment formation, general-purpose tool, and artwork.

## Authority

- Accepted baseline: `docs/PROJECT.md`.
- Decisions and candidates: `notebook/decisions/DECISION-REGISTER.md`.
- Concept synthesis: `notebook/research/CONCEPT-001-continuous-time-and-sedimentary-memory.md`.
- Facts and unknowns: `notebook/evidence/KNOWN-FACTS-AND-UNKNOWNS.md`.
- Current implementation and defects: source plus GitHub issues #1–#28.

## Constraints

- Preserve continuous idle time.
- Preserve sediment as meaningful history.
- Preserve living physical behavior without allowing administrative operations to arbitrarily rewrite formations.
- Keep layer switching lightweight.
- Keep Strata general-purpose.
- Preserve deliberate detach as a low-power opportunity.
- Do not hide unresolved meaning inside the SQLite schema.

## Decision units

### C1 — Deliberate detach

Decide what detach promises, what state is persisted, how elapsed time becomes grains, and whether reconstruction animation may continue after current interaction is already available.

Candidate:

> Deliberate detach intentionally continues the selected layer. Reopen establishes ledger truth and owed sediment immediately, performs bounded reconstruction, and may animate the result without delaying present commands.

### C2 — Unexpected termination

Decide how unclean closure is detected and how the elapsed interval is classified.

Candidate:

> The interval is real, but its classification is uncertain. Reopen offers previous layer, idle, or split/edit and records how the recovered material was classified.

### C3 — Layer and context

Compare:

1. flat layers only;
2. hierarchical layers;
3. one primary visual layer plus optional contexts.

Evaluate switching speed, color identity, general-purpose use, reporting value, schema cost, and migration from current categories.

Current candidate: one primary visual layer plus optional contexts.

### C4 — Vertical chronology

Choose among strict chronology, pure physical settlement, and broad chronology with local emergence.

Define exact mass invariants, structural tolerances, legitimate physical change, prohibited administrative change, and tests for resize and recovery.

Current candidate: broad chronology with local emergence.

### C5 — Formation lifecycle

Separate and scope:

- hide/show idle;
- begin a new formation;
- inspect prior formations;
- compact older material;
- rebuild from ledger;
- permanently remove sediment.

### C6 — Balance vocabulary

Define the layer property, aggregate view, neutral state, and relationship between actual time and signed balance. Then review Karma, polarity, valence, and Balance.

### C7 — Temporal quantum

Record enough to avoid schema lock-in: formation-level quantum, exact-ledger independence, remainder handling, reprojection rules, and coexistence of different quanta. Implementation remains deferred.

## Outputs

- accepted decisions or explicit deferrals for C1–C7;
- accepted updates to `docs/PROJECT.md` and `docs/DECISIONS.md`;
- a semantic brief for the SQLite schema;
- interaction scenarios for detach, crash recovery, layer selection, hiding idle, and new formations;
- tests that prevent regression into a conventional stopped-timer model or disposable sediment.

## Acceptance gate

CONCEPT-001 may complete when:

- unresolved questions are not accidentally decided by schema design;
- recovery has no ambiguous destructive default;
- ledger and sediment obligations are testable;
- ordinary interaction remains understandable and low-friction;
- the product owner recognizes the intended vision;
- an unfamiliar user's first impression is closer to that vision.

## Next

Resolve deliberate detach first because it affects interval identity, crash distinction, recovery state, sediment reconstruction, and SQLite schema design.
