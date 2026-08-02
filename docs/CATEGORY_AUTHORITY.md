# Category authority

Status: implemented and certified
Current completed unit: RECONCILIATION-001A
Issue completed: #5
Issue narrowed: #13
Last reviewed: 2026-08-02

## Purpose

Category authority preserves the meaning of recorded time and sediment across active use, archival, restart, interchange, and migration. A missing or retired category must never be reinterpreted as intentional idle, and authority selection must not change historical classification.

## Canonical identity

A category is identified by one stable numeric `CategoryId`. Its durable historical metadata includes:

- name;
- description;
- palette color index;
- karma effect;
- active or archived state;
- tags attached to the stable ID.

Idle is the canonical reserved ID 0. It is explicit in sessions and sediment but is not a mutable category-catalog row.

Category names are unique across the active and archived catalog. Restoring an archived name reuses the original stable identity rather than allocating a replacement.

## Active and archived state

Active categories are available for ordinary selection and new session classification.

Archived categories are hidden from ordinary selection but remain authoritative for:

- historical reports and exports;
- session display and serialization;
- karma reconstruction;
- sand-state restoration and rendering;
- daily sediment contributions;
- category tags;
- later migration or restore.

Archival changes availability, not historical meaning. Name, description, color, karma effect, stable ID, and tags survive retirement and restoration.

## SQLite authority

SQLite stores archival state through `archived_at_utc`.

- referenced categories are protected by foreign-key and restricted-deletion policy;
- TUI retirement archives rather than destructively deletes;
- active and archived categories load into separate projections while retaining one identity space;
- reports, sand, and persistence combine the projections whenever historical metadata is required;
- restore reactivates the existing row.

## Legacy-file authority

`categories.csv` is the single auditable legacy category catalog.

### Backward compatibility

The historical five-column schema remains readable:

```text
id,name,description,color_index,karma_effect
```

Every row in that schema is treated as active.

New writes use:

```text
id,name,description,color_index,karma_effect,archived
```

The `archived` field is a strict boolean. Active and archived rows coexist in the same atomic catalog file.

### Validation

Loading fails closed for:

- malformed or duplicate IDs;
- ID 0 catalog rows;
- empty, duplicate, or reserved idle names;
- invalid or out-of-range color indexes;
- invalid or out-of-range karma effects;
- malformed archived state.

Malformed catalog rows are not skipped and default values are not invented.

## Session reference integrity

Every persisted session category ID must resolve to active or archived category metadata.

- malformed IDs produce a row-scoped integrity error containing the original value;
- unknown IDs produce an actionable error that preserves the original identity for repair;
- explicit ID 0 remains valid intentional idle;
- loaders never convert an unresolved ID to idle;
- writers refuse unknown IDs before publishing a CSV;
- session labels are derived from the resolved catalog entry rather than a fallback label.

Failing the load is preferable to showing plausible but false totals.

## Tags

Tags belong to stable category identity, not active visibility.

- archive does not delete tags;
- startup validation accepts active and archived IDs;
- restart does not prune archived-category tags;
- restore makes the same tags visible again with the same category ID.

## Migration custody

Legacy-to-SQLite migration accepts both five- and six-column category catalogs.

- five-column rows import as active;
- six-column archived rows import with `archived_at_utc` populated;
- sessions retain their original category foreign key;
- migration does not reactivate archived categories or rewrite references;
- validation and database publication remain transactional and fail closed.

## Persistence failure

Category catalog writes use atomic file publication. A failed legacy write enters the existing visible persistence-recovery contract; it cannot produce a partially written catalog.

SQLite archival remains transactional. Neither authority may claim successful retirement when its authoritative persistence operation failed.

## Certified proofs

- legacy catalog backward compatibility;
- active/archived catalog round trip;
- malformed and unknown session-reference rejection;
- explicit idle preservation;
- unknown writer rejection;
- archived session-label round trip;
- archive/restore stable identity and metadata;
- archived tag retention across startup;
- archived legacy migration into SQLite;
- existing report, sediment, migration, TUI, CLI, interaction, failure-recovery, and PTY suites remain green.

## Unresolved boundary

Category merge/reassignment and permanent destructive deletion are not implemented. Any future permanent deletion must require zero references or one reviewed transaction that reassigns every session, snapshot, sediment contribution, tag, and other category-owned record before removing the identity.