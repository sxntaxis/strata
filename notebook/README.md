---
id: NOTEBOOK-ROOT
kind: system
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Repository-local project memory for Strata's product concept, evidence, decisions, bounded work, and current frontier.
---

# Strata Notebook

This directory is Strata's durable working memory.

Strata is developed as both a practical temporal instrument and an artistic system. The Notebook exists to preserve that combined intent while keeping exact implementation authority, verified behavior, working interpretation, and unresolved questions distinct.

## Governing flow

```text
conversation, source, runtime, or failure
    ↓
classified evidence and interpretation
    ↓
working synthesis or bounded work unit
    ↓
candidate decision
    ↓
explicit product-owner review
    ↓
accepted authority under docs/
    ↓
branch, implementation, and verification
```

Conversation is input rather than the durable artifact. The Notebook stores the smallest useful synthesis, decision, evidence record, or work unit. Git preserves chronology; Notebook records preserve current meaning.

## Start here

1. Read [`NOW.md`](NOW.md) for the present frontier.
2. Read [`system/AUTHORITY.md`](system/AUTHORITY.md) before resolving conflicts.
3. Read [`system/EPISTEMIC-CLASSES.md`](system/EPISTEMIC-CLASSES.md) before turning reports or impressions into claims.
4. Read [`decisions/DECISION-REGISTER.md`](decisions/DECISION-REGISTER.md) for accepted, candidate, deferred, and unresolved choices.
5. Open only the relevant record under `work/`, `research/`, or `evidence/`.
6. When implementation is involved, return to repository-root `docs/`, source, tests, issues, and runtime.

## Directory map

- `NOW.md` — compact current state, active work, blockers, and next edge.
- `system/` — authority, workflow, and epistemic rules.
- `decisions/` — accepted, candidate, deferred, rejected, and superseded decisions.
- `evidence/` — facts, reports, derived findings, contradictions, and known unknowns.
- `research/` — developed conceptual or source-backed studies.
- `work/` — bounded investigations, migrations, and decision units.

New directories are added only when recurring use proves a separate owner is needed.

## Authority boundary

- `notebook/**` owns developing thought, classified evidence, rationale, and unresolved questions.
- `docs/**` owns accepted product and architecture direction.
- `src/**`, tests, CI, and observed runtime own implementation reality.
- GitHub issues own defect tracking and proposed remediation, not product doctrine.

A polished Notebook statement remains working knowledge until promoted through explicit owner review.

## Current work

- [`work/RELIABILITY-001-persistence-and-audit-remediation.md`](work/RELIABILITY-001-persistence-and-audit-remediation.md) organizes SQLite migration and the confirmed audit findings.
- [`work/CONCEPT-001-product-concept-clarification.md`](work/CONCEPT-001-product-concept-clarification.md) resolves the remaining conceptual and interaction questions according to Strata's own artistic doctrine.
- [`research/CONCEPT-001-continuous-time-and-sedimentary-memory.md`](research/CONCEPT-001-continuous-time-and-sedimentary-memory.md) preserves the developed understanding from the founding conceptual conversation.

## Operating rules

1. Tools may be art; do not reduce Strata to conventional productivity software before evaluating its intended concept.
2. Raw outsider reactions remain useful evidence of legibility, but they do not define the product.
3. Critique the accepted concept rather than silently replacing it with familiar category assumptions.
4. Preserve exact ledger truth and accountable sedimentary history without pretending they have identical precision.
5. Do not implement unresolved metaphors, terminology, hierarchy, or detach behavior by accident.
6. No Notebook record substitutes for code, testing, runtime observation, or user experience.
7. No full chat transcript is stored when durable meaning can be synthesized.
8. Update current records rather than creating chronological duplicates unless provenance itself is the subject.
