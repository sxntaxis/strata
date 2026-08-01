# Authority contract

Strata uses the following conflict order:

1. explicit current owner decision;
2. accepted documents under `docs/`;
3. verified source, tests, CI, and runtime reality;
4. Notebook working records;
5. GitHub issues, external sources, old plans, and history;
6. projections such as reports, exports, and sediment previews.

A lower layer may reveal that a higher layer is stale, but it does not silently replace it. Record the conflict and repair the owning authority.

## Persistence authority

After explicit activation, SQLite is the sole live authority. CSV/JSON bundles, legacy sources, reports, exports, snapshots, and emergency custody files are not alternative live ledgers.

## Product authority

Implementation reality does not automatically settle unresolved product questions. Product decisions require explicit owner acceptance and promotion to `docs/`.
