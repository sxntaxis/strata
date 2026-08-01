# Strata agent contract

## Recovery order

1. Read `project.meta.toml` and `project.notebook.toml`.
2. Read `notebook/README.md` and `notebook/NOW.md`.
3. Read the smallest relevant accepted document under `docs/`.
4. Read the smallest relevant Notebook work, research, decision, or evidence record.
5. Inspect current source, tests, CI, issues, and runtime before making implementation claims.

## Authority order

1. Current explicit product-owner decision.
2. Accepted repository authority under `docs/`.
3. Current verified source and runtime behavior where accepted documentation is incomplete.
4. Notebook working records and accepted-but-unpromoted decisions.
5. GitHub issues, external sources, old plans, chat history, and Git history.

A conflict among these layers is a defect or decision gate. Do not silently choose the most convenient version.

## Notebook rules

- Conversation is input, not the durable record.
- Update the smallest canonical Notebook record that owns the meaning.
- Preserve facts, reports, derivations, interpretations, assumptions, decisions, and unknowns as distinct classes.
- Do not store full chat transcripts when a synthesis, decision record, or bounded work unit preserves the durable meaning.
- Research and polished proposals do not become implementation authority without product-owner review and promotion into `docs/`.
- Keep current frontier information in `notebook/NOW.md`; do not duplicate pull-request progress there.
- Do not add structures merely to satisfy Notebook. Every record must reduce uncertainty, preserve truth, enable a decision, or improve execution.

## Repository workflow

- Never write directly to `main`.
- Create a dedicated branch and pull request for each bounded change.
- Preserve current application behavior during Notebook adoption.
- Do not combine governance adoption with cleanup, refactoring, or bug fixes.
- Use optimistic concurrency: fetch the latest blob before updating an existing file and merge meaning rather than overwriting newer work.
- Do not rewrite published history without explicit authorization.

## Verification

For application changes, run the declared commands in `project.meta.toml`.

For Notebook-only changes:

- confirm no production source changed;
- validate TOML and frontmatter structurally;
- verify every accepted decision and unresolved question has a clear state;
- verify `notebook/NOW.md` names a concrete next edge;
- run Notebook conformance when the Notebook CLI is available.
