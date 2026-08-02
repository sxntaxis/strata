---
id: RECONCILIATION-001A
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# RECONCILIATION-001A — legacy category integrity and archival parity

## Issues

- #5: legacy `time_log.csv` loading silently maps unknown category IDs to intentional idle, hiding work and destroying the original identity.
- #13: SQLite category deletion is archival, but legacy-file deletion still removes category metadata from `categories.csv`; restart then triggers #5 and historical sessions/sand lose classification.

## Reconciled premise

SQLite already preserves archived category identities and rejects invalid references. The remaining defect is the still-supported legacy-file authority. Correctness must not depend on which live authority is selected.

## Selected contract

### Category catalog

`categories.csv` remains the single auditable legacy category catalog.

- Existing five-column files remain readable and imply every row is active.
- New writes use a sixth `archived` column.
- Active and archived rows preserve the same stable ID, name, description, color index, and karma effect.
- Idle remains canonical ID 0 and is never archived.
- Duplicate or malformed category identity fails closed rather than being skipped into ambiguity.

### Session integrity

- A session category ID must parse as an integer and resolve to active or archived category metadata.
- Intentional idle ID 0 remains valid.
- Missing or malformed category references return an actionable row-scoped integrity error containing the original value.
- No loader or writer substitutes idle for an unknown category.
- Session CSV labels are written from active-or-archived metadata.

### Archival behavior

- Removing a legacy category moves its metadata from active to archived state before persistence.
- Archived categories disappear from ordinary layer selection but remain available to reports, exports, sand restoration, and daily sediment rendering.
- Restoring by name reactivates the same stable identity and metadata; it does not create a replacement category or rewrite historical meaning.
- Category tags remain attached to the stable category identity across archive/restore.
- Legacy and SQLite paths expose the same in-memory active/archived model.

### Migration custody

A later SQLite migration must import both active and archived legacy categories without reclassifying archived history as active or losing referenced identities.

## Acceptance proofs

- old five-column category files load as active-only catalogs;
- new six-column catalogs round-trip active and archived rows;
- missing category ID, malformed category ID, and partial category catalog fail with actionable session-row errors;
- normal idle sessions load unchanged;
- session writing refuses unknown category IDs;
- legacy archive preserves report totals, labels, colors, karma, sand category identity, and tags;
- restore reuses the original ID and metadata;
- SQLite import retains archived category state;
- all existing storage, migration, report, sediment, TUI, CLI, interaction, and PTY tests remain green.

## Boundary

This unit does not add permanent destructive deletion or category merging. If issue #13's explicit merge/reassignment workflow remains absent after archival parity is certified, the issue will be narrowed to that separate product capability rather than kept as a misleading data-loss bug.