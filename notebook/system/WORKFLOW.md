---
id: SYS-WORKFLOW-001
kind: system
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Review-first workflow from conversation and observation into Strata decisions, accepted authority, implementation, and proof.
---

# Workflow

## Normal flow

```text
conversation or observation
    ↓
develop and classify the meaning
    ↓
update the smallest canonical record
    ↓
review candidate consequences
    ↓
promote accepted direction into docs/
    ↓
implement on a bounded branch
    ↓
verify against product and runtime obligations
```

The Notebook is maintained for the project owner. The owner should not need to manually file every thought.

## Classification and destination

| Meaning | Destination |
|---|---|
| Current condition, blockers, or next edge | `NOW.md` |
| Direct facts, reports, contradictions, or unknowns | `evidence/` |
| Developed conceptual or source-backed investigation | `research/` |
| Accepted, candidate, deferred, rejected, or superseded choice | `decisions/` |
| Bounded outcome, migration, experiment, or decision gate | `work/` |
| Authority, epistemic, and maintenance rules | `system/` |
| Accepted product or architecture constraint | repository-root `docs/` |
| Defect reproduction and closure | GitHub issue |
| Implementation and proof | branch, pull request, tests, and runtime evidence |

## Checkpoint rule

Update durable memory when:

- the owner accepts, rejects, or materially corrects a direction;
- a conceptual distinction changes the likely architecture;
- a meaningful unknown becomes explicit;
- a bounded work unit begins or changes state;
- implementation is about to encode a product assumption;
- another agent or future session would otherwise repeat the same conceptual reconstruction.

Do not create chronological conversation summaries. Update current meaning and preserve provenance through Git.

## Critique protocol

Strata is intentionally unconventional. Before product criticism:

1. recover the accepted concept;
2. distinguish an outsider's raw reaction from a claim about intended behavior;
3. identify what the raw reaction reveals about legibility;
4. critique the intended concept on its own terms;
5. ask only for genuinely missing product meaning;
6. record accepted corrections and remaining unknowns separately.

A conventional productivity-app assumption is not neutral evidence.

## Concurrent work

- Read the latest blob before updating an existing record.
- Use the blob SHA as optimistic concurrency control.
- On conflict, refetch and merge meaning.
- Avoid shared generated files unless deterministic rebuild and ownership are established.
- Separate conceptual, reliability, and implementation branches when their acceptance gates differ.

## Implementation boundary

GitHub issues are the closure units for confirmed bugs. Notebook work records organize sequence, product dependencies, and acceptance boundaries; they do not duplicate issue status line by line.
