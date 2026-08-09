# Category authority

Status: implemented and certified
Completed units: RECONCILIATION-001A, RECONCILIATION-001C1, RECONCILIATION-001C2
Issues completed: #5, #13
Last reviewed: 2026-08-03

## Purpose

Category authority preserves the meaning of recorded time and sediment across active use, archival, restart, and interchange. A missing or retired category must never be reinterpreted as intentional idle.

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
- portable restore.

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

Portable bundle category data is validated against the SQLite category catalog.

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

## Legacy lifecycle transformation

Legacy merge/reassignment and permanent deletion use the same explicit source/target semantics and complete reference model as SQLite, but publication is governed by an exact-result prepared receipt. Before any authority file changes, preparation validates source idle, self-merge, target existence, zero-reference deletion, stale revision, checkpoint shape, and absence of unresolved switch/finish/clear receipts.

The prepared receipt contains the reviewed metadata, counts, revision, operation identity, and exact resulting catalog, session ledger, tags, canonical sediment, affected daily contributions or explicit deletions, detached checkpoint payload, and permanent lifecycle ledger. It is published atomically before any result. Startup applies it before ordinary state load and accepts only exact already-published artifacts. Conflicting or malformed state fails closed and retains evidence. The prepared receipt is removed only after every named artifact and the permanent ledger converge.

The permanent ledger records committed merge/deletion receipts and retired source identities. Category allocation advances beyond both catalog and ledger identities. SQLite lifecycle receipts preserve the identity high-water mark.

## Explicit lifecycle interaction

Ordinary `x` remains archive. A distinct configurable `Shift-X` action opens a blocking lifecycle overlay, chooses an explicit target or targetless deletion, displays source/target identity, all affected reference counts, checkpoint custody, and revision, and requires the exact displayed revision-bound phrase. Esc cancels without mutation. Approximate, case-folded, whitespace-normalized, stale, or missing confirmation never applies. SQLite publishes the prepared receipt and reloads through the same replay path.

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

Portable bundle import accepts the current category bundle schema.

- five-column rows import as active;
- six-column archived rows import with `archived_at_utc` populated;
- sessions retain their original category foreign key;
- import does not reactivate archived categories or rewrite references;
- committed lifecycle receipts remain SQLite receipt authority;
- retired source IDs and the identity high-water mark survive backup and restore;
- validation and database publication remain transactional and fail closed.

## Persistence failure

Category catalog writes use SQLite transactions. A failed write enters the existing visible persistence-recovery contract; it cannot produce a partially written catalog. Once a lifecycle prepared receipt is durable, failure retains that evidence and retry/restart republishes its exact results rather than reconstructing intent from partial state.

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
- complete reference inventory and deterministic revision;
- exact-result prepared receipt across catalog, sessions, tags, sediment, daily artifacts, checkpoint, and permanent ledger;
- eight persisted replay kill points with retained evidence and clean retry convergence;
- zero-reference-only deletion, idle/self-merge/stale/protected-evidence refusal, and target metadata preservation;
- permanent ledger restart custody, retired-ID nonreuse, bundle fingerprinting, and current-schema import;
- distinct archive and configurable lifecycle actions across resolver, atlas, palette, and runtime;
- exact confirmation phrase unit proofs and a live PTY round trip that reads the rendered phrase, types it back, commits one receipt, and verifies reassignment;
- current category catalog validation;
- active/archived catalog round trip;
- malformed and unknown session-reference rejection;
- explicit idle preservation;
- unknown writer rejection;
- archived session-label round trip;
- archive/restore stable identity and metadata;
- archived tag retention across startup;
- archived category interchange into SQLite;
- existing report, sediment, TUI, CLI, interaction, failure-recovery, and PTY suites remain green.

## Issue #13 closure

RECONCILIATION-001A preserves historical meaning through archive/restore and strict reference resolution. RECONCILIATION-001C1/C2 provide the complete SQLite preview, transaction, receipt, retired-ID custody, and shared explicit TUI confirmation surface. Archive remains the ordinary retirement operation; reviewed merge/reassignment and zero-reference permanent deletion are implemented and certified.
