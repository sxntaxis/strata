# Category authority

Status: partially implemented and certified
Current completed unit: RECONCILIATION-001C1
Issue completed: #5
Issue narrowed: #13
Last reviewed: 2026-08-03

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

## SQLite lifecycle transformation

Archive remains the ordinary retirement operation. Merge/reassignment and permanent deletion are distinct reviewed lifecycle operations.

Before either operation, SQLite builds one typed preview that:

- names an explicit source stable ID and optional explicit target stable ID;
- rejects idle, self-merge, and missing identities;
- resolves active and archived rows without name ambiguity;
- inventories completed sessions, active state, tags, placed and pending canonical sediment, every persisted snapshot, daily contributions, and runtime-checkpoint payload references;
- exposes checkpoint custody status and source/target metadata snapshots;
- binds all mutation-relevant authority state with a deterministic revision.

Application recomputes that revision inside one immediate transaction. A stale preview, protected `recovering`/`quarantined` checkpoint, malformed payload, or unresolved transition/finish/clear receipt blocks the operation before any authority changes.

A merge changes category identity only:

- completed session ID, stable ID, project, description, UTC chronology, operational-day policy, and elapsed duration remain unchanged;
- active stable ID, start, description, and recovery kind remain unchanged;
- target name, description, color, balance effect, archival state, and sort identity remain target-owned;
- target tags precede source-only tags and exact duplicates collapse;
- placed and pending sediment preserve mass and FIFO order;
- cumulative/manual snapshots remap category identity;
- daily contributions are regenerated from reassigned canonical session slices and receive matching source revisions;
- receipt-free checkpoints remap active, sediment, and queued-switch identity;
- the source row is removed only after a complete zero-residual-reference check.

Permanent deletion without a target is allowed only when the same complete preview reports zero references in every family. Idle cannot be deleted.

Each committed operation writes an immutable lifecycle receipt with source and target metadata, preview revision, affected counts, and application timestamp. Retry returns the same receipt idempotently. Receipt source IDs are retired forever and category allocation advances beyond all current and retired identities.

SQLite schema version 7 owns lifecycle receipts. Consistent repository snapshots, raw backup/restore, portable bundle schema 3, import validation, and `sqlite doctor` preserve and validate those receipts. A bundle or database that reintroduces a retired source ID fails integrity validation.

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

- complete SQLite reference preview and deterministic stale-preview rejection;
- atomic merge across completed/active sessions, tags, canonical sediment, snapshots, daily contributions, checkpoint payload, source removal, and receipt;
- ten injected publication boundaries with full rollback;
- completed-session and active-generation identity/chronology preservation;
- target metadata preservation and deterministic tag deduplication;
- sediment mass/FIFO preservation and daily-revision regeneration;
- protected or receipt-bearing checkpoint refusal;
- zero-reference-only permanent deletion and idle refusal;
- idempotent lifecycle retry;
- retired-ID nonreuse before and after portable bundle round trip;
- lifecycle receipt validation and doctor detection of tamper or retired-ID collision;
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

## Remaining issue #13 boundary

SQLite lifecycle authority is implemented and certified. Issue #13 remains open because legacy-file authority still needs a prepared receipt and idempotent crash replay across catalog, sessions, tags, canonical sediment, daily artifacts, detached checkpoint evidence, and retired-ID custody.

The product also needs one explicit review and confirmation surface that presents the complete preview and refuses stale confirmation under both supported authorities. Until C2 is complete, archive remains the only ordinary TUI retirement operation and no legacy merge or permanent deletion may claim success.
