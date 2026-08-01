# Strata Notebook

This directory is Strata's repository-local working memory. It preserves current frontier, evidence, research, decisions, and bounded work without treating conversation or speculative prose as implementation authority.

## Start here

1. `NOW.md` — current frontier and next bounded edge.
2. `../docs/PROJECT.md` — accepted product doctrine.
3. `../docs/ARCHITECTURE.md` — current verified system and authority state.
4. `../docs/DECISIONS.md` — accepted implementation constraints.
5. The smallest relevant record under `work/`, `research/`, `decisions/`, or `evidence/`.

## Directory roles

- `system/` — Notebook authority, epistemic, and workflow contracts.
- `decisions/` — detailed accepted, candidate, superseded, and open decisions.
- `evidence/` — verified facts, unknowns, and reconciliation records.
- `research/` — synthesized exploration that is not accepted authority by itself.
- `work/` — bounded active or completed programs and their sequencing.

## Promotion rule

Notebook material becomes accepted authority only through explicit owner approval and a reviewed change to `docs/`. Source and tests may establish implementation reality, but they do not silently settle product questions.

## Current status

The SQLite authority migration is complete. The active frontier is configuration/profile authority, followed by temporal correctness, reporting semantics, and sediment conservation.
