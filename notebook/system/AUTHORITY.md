---
id: SYS-AUTHORITY-001
kind: system
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Conflict order and promotion boundary for Strata product thought, accepted doctrine, implementation reality, and issue tracking.
---

# Authority

## Classes

### Owner

Current explicit product-owner decisions have highest authority. They may correct earlier documentation, code assumptions, or Notebook interpretations, but the resulting conflict must be recorded and reconciled.

### Accepted

Repository-root `docs/**` owns accepted product and architecture direction that constrains implementation.

### Reality

Current source, tests, CI, persisted data, and observed runtime own implementation reality. Reality may expose a defect or undocumented behavior; it does not automatically redefine product intent.

### Working

`notebook/**` owns developing thought, evidence classification, rationale, research, candidate decisions, and unresolved questions.

### Source

GitHub issues, external material, old plans, release notes, chat history, and recollection supply evidence or proposals. They remain subordinate to current owner decisions, accepted authority, and verified reality.

### Projection

Reports, exports, sediment views, generated summaries, and UI labels project underlying truth. A projection does not become canonical merely because it is visible.

## Conflict order

1. Current explicit product-owner decision.
2. Accepted repository-root authority.
3. Current verified implementation reality where accepted authority is incomplete.
4. Notebook working records and accepted-but-unpromoted decisions.
5. Issues, sources, old plans, recollection, and history.
6. Projections.

## Promotion

A Notebook conclusion becomes accepted authority only when:

1. the owning consequence is identified;
2. evidence, uncertainty, alternatives, and refusals are visible;
3. the product owner explicitly accepts or edits it;
4. the relevant `docs/**` record is updated through a reviewed repository change;
5. implementation is separately authorized where required.

After promotion, the Notebook may retain rationale and unresolved implications but must not maintain a competing live specification.

## Special Strata boundary

The project contains two historical forms:

- exact chronological ledger truth;
- accountable sedimentary visual truth.

Neither is merely a projection of the other. Reports are projections over the ledger. Rendered terminal cells are projections over logical sediment. The logical sediment formation remains historically meaningful even when it is less precise than the ledger.
