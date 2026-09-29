# Category authority

Status: accepted and certified
Last reviewed: 2026-09-06

## Purpose

Category authority preserves the meaning of recorded time and sediment across active use, archive/restore, restart, recovery, and interchange. Missing category identity must never be reinterpreted as intentional idle.

## Canonical identity

A category is identified by one stable numeric `CategoryId`. Durable category state includes:

- name;
- legacy description text retained for SQLite and portable-interchange compatibility;
- theme-independent RGB color anchor (legacy `color_index` storage remains a compatibility envelope);
- balance effect;
- active or archived state;
- reusable tags attached to the stable ID.

Idle is reserved category ID `0`.

## Appearance relationship

Category identity and category color choice are durable domain state; resolved RGB presentation is not. The persisted color anchor survives theme changes. The active theme maps that anchor to its nearest eligible sand swatch for rendering, while `Ctrl+←` / `Ctrl+→` chooses a new anchor from the active theme's perceptually ordered sand palette. Themes never name or own concrete user categories. See `docs/APPEARANCE_AUTHORITY.md`. The current color anchor belongs to the stable Layer identity across time: changing it immediately changes current and historical sediment presentation for that CategoryId without moving grains. Permanent Balance deletion is the explicit identity-lifecycle exception and reclassifies retained current/historical sediment from that CategoryId to Idle.

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

Tag suggestions/history belong to stable category identity and survive archive/restore. The active session description/draft is separate from the category's legacy description field. A session may carry multiple independent attribution tags in its existing description field, canonically serialized with `; ` between tags; the per-category tag history stores the individual canonical tag spellings used for completion rather than becoming a second time authority. Ordinary layer switching edits the active-session text. The legacy category-description value is preserved for SQLite and portable interchange, but the current TUI neither edits it nor searches it; layer identity is changed by renaming the category name while preserving its stable ID. Physically dropping stored legacy values requires a separately versioned schema/interchange migration.

## Portable interchange

Portable CSV is an interchange representation of the SQLite repository, not live authority. Bundle import validates category/session references and preserves active/archived state and stable IDs.

## Persistence failure

A failed category write enters the visible persistence-recovery contract. Strata never reports archive/restore success after an authoritative SQLite failure and never falls back to a file-backed category catalog.
