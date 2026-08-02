---
id: RECONCILIATION-001A
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# RECONCILIATION-001A — legacy category integrity and archival parity

## Issues reconciled

- #5: completed. Unknown or malformed legacy session category IDs now fail closed instead of becoming intentional idle.
- #13: the historical data-loss defect is completed. The issue remains open only for explicit merge/reassignment and permanent-deletion policy.

## Reconciled premise

SQLite already preserved archived category identities, restricted referenced deletion, and rejected invalid references. The remaining defect was the still-supported legacy-file authority. Correctness now no longer depends on which live authority is selected.

## Accepted contract

### Category catalog

`categories.csv` remains the single auditable legacy category catalog.

- Existing five-column files remain readable and imply every row is active.
- New writes use a sixth `archived` column.
- Active and archived rows preserve the same stable ID, name, description, color index, and karma effect.
- Idle remains canonical ID 0 and is implicit rather than a mutable catalog row.
- Duplicate IDs, duplicate names, malformed values, reserved idle aliases, invalid colors, and invalid karma values fail closed.

### Session integrity

- A session category ID must parse as an integer and resolve to active or archived category metadata.
- Intentional idle ID 0 remains valid.
- Missing or malformed category references return an actionable row-scoped integrity error containing the original value.
- No loader or writer substitutes idle for an unknown category.
- Session CSV labels are written only from active-or-archived metadata; writing an unknown identity is refused before publication.

### Archival behavior

- Removing a legacy category transfers its complete metadata into the archived catalog and persists the combined active/archived catalog.
- Archived categories disappear from ordinary layer selection but remain available to reports, exports, sand restoration, daily sediment rendering, and session serialization.
- Restoring by name reactivates the same stable identity and metadata; it does not create a replacement category or rewrite historical meaning.
- Category tags remain attached to the stable category identity across archive, restart, and restore.
- Startup tag validation recognizes both active and archived category IDs.
- Legacy and SQLite paths expose the same in-memory active/archived model.
- Persistence failure remains under the existing visible recovery/reload contract; it cannot silently publish a partial catalog.

### Migration custody

A later SQLite migration imports both active and archived legacy categories. Archived rows receive an archival timestamp in SQLite, and sessions retain their original foreign-key identity rather than reactivating or losing the category.

## Bugs found and fixed

1. Legacy `time_log.csv` loading converted malformed and unknown category IDs to idle.
2. Legacy category retirement removed metadata, causing reports and sand restoration to lose historical classification after restart.
3. Session serialization invented the idle label when category metadata was missing.
4. Legacy category restore reused the ID but overwrote its historical color and cleared its description.
5. Legacy archive removed category tags instead of preserving stable-identity metadata.
6. SQLite migration could not consume the new archived legacy catalog state.
7. SQLite TUI loading omitted the new archived-catalog field after the shared catalog type changed.
8. Startup tag validation considered only active categories and deleted archived-category tags on restart.

## Certified proofs

- old five-column category files load as active-only catalogs;
- new six-column catalogs round-trip active and archived rows;
- malformed ID, unknown ID, and partial category catalogs fail with actionable session-row errors;
- normal explicit idle sessions load unchanged;
- session writing refuses unknown category IDs;
- archived category metadata keeps session labels and stable IDs round-trippable;
- legacy archive preserves report/sand metadata and tags across restart;
- restore reuses the original ID, name, color, description, karma effect, and tags;
- SQLite import retains archived state and referenced session identity;
- formatting and strict Clippy pass;
- 187 library tests pass;
- 9 CLI lifecycle tests pass;
- 6 configuration-authority tests pass;
- 1 report-help regression test passes;
- 12 SQLite/TUI process tests pass;
- 2 temporal-authority tests pass;
- 3 terminal-lifecycle PTY process tests pass.

## Durable authority

- `docs/CATEGORY_AUTHORITY.md` owns cross-authority category identity, archival, restore, and reference integrity.
- `docs/ARCHITECTURE.md` assigns active/archived catalog custody to storage/repository and category orchestration boundaries.
- STRATA-D041 and STRATA-D042 constrain reference validation and non-destructive retirement.
- `notebook/work/ISSUE-RECONCILIATION-001.md` closes #5 and narrows #13.

## Remaining boundary

This unit does not add category merging, bulk session/snapshot reassignment, or permanent destructive deletion. Those capabilities remain the explicit unresolved remainder of issue #13 and require a separately reviewed transaction model.