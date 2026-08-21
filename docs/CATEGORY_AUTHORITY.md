# Category authority

Status: accepted and certified
Last reviewed: 2026-08-20

## Purpose

Category authority preserves the meaning of recorded time and sediment across active use, archive/restore, restart, recovery, and interchange. Missing category identity must never be reinterpreted as intentional idle.

## Canonical identity

A category is identified by one stable numeric `CategoryId`. Durable category state includes:

- name;
- description/metadata;
- palette color index;
- karma effect;
- active or archived state;
- reusable tags attached to the stable ID.

Idle is reserved category ID `0`.

## Archive and restore

Archive is the ordinary retirement operation. It changes availability, not historical meaning.

Archived categories are hidden from ordinary new-session selection but remain authoritative for:

- historical sessions, reports, and exports;
- sediment restoration and rendering;
- daily sediment contributions;
- tags and metadata;
- portable backup/interchange.

Restore reactivates the same SQLite row and stable ID. Category allocation advances beyond the maximum category ID still present in the catalog. Because archive does not physically delete rows, archived identities are not reused.

Strata does not currently implement category merge or permanent deletion. The prerelease reviewed-lifecycle machinery for revision-bound merge/deletion, retired-ID receipts, and destructive confirmation was speculative and has been retired rather than kept as dormant architecture.

## SQLite authority

SQLite stores archival state in `categories.archived_at_utc`.

- active and archived rows share one identity space;
- session foreign keys retain historical category identity;
- TUI archive is transactional;
- recovery loads both active and archived metadata where historical meaning requires it;
- unknown references fail closed.

## Session reference integrity

Every persisted session category ID must resolve to the catalog or to explicit idle ID `0`.

- malformed IDs are rejected;
- unknown IDs are rejected rather than mapped to idle;
- session labels come from resolved category identity;
- archival does not rewrite historical session foreign keys.

## Tags and metadata

Tags belong to stable category identity and survive archive/restore. The active session description/draft is separate from durable category metadata. Ordinary layer switching edits the active-session text; durable metadata has an explicit edit mode.

## Portable interchange

Portable CSV is an interchange representation of the SQLite repository, not live authority. Bundle import validates category/session references and preserves active/archived state and stable IDs.

## Persistence failure

A failed category write enters the visible persistence-recovery contract. Strata never reports archive/restore success after an authoritative SQLite failure and never falls back to a file-backed category catalog.
