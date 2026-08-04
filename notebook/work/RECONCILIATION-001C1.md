---
id: RECONCILIATION-001C1
kind: work
state: accepted
authority: accepted
created: 2026-08-03
updated: 2026-08-03
---

# RECONCILIATION-001C1 — SQLite category lifecycle authority

## Issue

Issue #13 still lacks an explicit destructive or transformative category lifecycle. Archive is safe and remains the default, but there is no complete reference inventory, stale-preview guard, atomic merge/reassignment transaction, or zero-reference permanent deletion.

A category identity currently appears in more places than direct SQL foreign keys:

- completed sessions;
- the active session;
- ordered category tags;
- canonical placed and pending sediment;
- persisted sediment snapshots and daily-contribution source revisions;
- runtime checkpoint payload state, queued mutations, and transition receipts;
- report and export projections derived from the ledger.

Deleting or updating only relational rows would leave semantically split history.

## Selected C1 boundary

C1 establishes the SQLite authority layer only. The TUI review surface and legacy multi-file receipt protocol remain C2.

### Preview

A typed preview must:

- require explicit source identity and optional explicit target identity;
- reject source idle and self-merge;
- resolve active versus archived rows by stable ID without name ambiguity;
- count every source-owned reference family;
- identify checkpoint custody status;
- include source and target metadata snapshots;
- produce a deterministic revision over the complete mutation-relevant authority state.

### Merge/reassignment

One immediate SQLite transaction must, after recomputing and matching the preview revision:

- reassign completed and active sessions without changing stable record identity, chronology, duration, project, or description;
- merge tags deterministically with deduplication;
- remap placed, legacy-pending, and compressed-pending sediment identity while conserving mass and order;
- remap every persisted snapshot payload and regenerate daily-contribution source revision from reassigned ledger truth;
- remap receipt-free runtime checkpoint payload identity, sediment, and queued switch mutations; unresolved transition/finish/clear receipts block the operation because their deterministic operation identities bind the original categories;
- delete the source category only after all references have moved;
- insert an auditable receipt containing source/target metadata, preview revision, affected counts, and application time;
- roll back every authority on injected failure.

Protected or malformed checkpoint evidence fails closed rather than being partially transformed.

### Permanent deletion

A targetless deletion transaction is allowed only when the complete preview reports zero references in every authority, including tags and checkpoint payload evidence. It records a deletion receipt and removes only the source category row.

Idle cannot be deleted. Archive remains the ordinary retirement operation.

## Acceptance proofs

- source and target identities are explicit and self-merge is rejected;
- active and archived source/target combinations are unambiguous by ID;
- preview revision changes after any mutation-relevant concurrent change;
- successful merge preserves session IDs, stable IDs, chronology, elapsed time, total sediment mass, and FIFO category order while replacing only source category identity;
- tags merge deterministically without duplicates;
- daily snapshot state and source revision agree with reassigned sessions;
- checkpoint payload references are transformed completely or the operation fails closed;
- permanent deletion refuses any nonzero reference family;
- transaction fault injection proves no partial reassignment or deletion;
- repository snapshot, reports, exports, recovery, and TUI reload remain coherent;
- strict Clippy and the full suite pass.

## Remaining issue #13 scope after C1

- legacy-file prepared receipt and idempotent replay across categories, sessions, tags, sand, daily artifacts, and detached checkpoint evidence;
- user-visible preview and explicit confirmation workflow;
- end-to-end archive/restore coexistence and final issue closure.

## Implemented result

- SQLite schema version 7 adds strict `category_lifecycle_receipts` authority;
- one typed preview resolves source and optional target by stable ID, inventories completed and active sessions, tags, canonical sediment, persisted snapshots, daily contributions, and runtime-checkpoint references, and binds them with a deterministic revision;
- one immediate transaction rejects stale previews, protected or malformed recovery evidence, source idle, self-merge, and unresolved transition receipts before publication;
- merge reassigns only category identity while preserving completed-session ID/stable ID, project, description, UTC chronology, elapsed duration, active stable identity, active start, and target metadata;
- target-first ordered tags deduplicate deterministically;
- placed, legacy-pending, and compressed-pending sediment preserve total mass and FIFO category order;
- cumulative/manual snapshots remap identity and daily contributions are regenerated from reassigned canonical ledger slices;
- receipt-free checkpoint payloads remap active identity, sediment, and queued switch mutations; receipt-bearing evidence fails closed;
- targetless permanent deletion is permitted only after a complete zero-reference preview;
- every merge or deletion records source/target metadata, preview revision, reference counts, and application time;
- repeated application of the same reviewed operation returns the existing receipt without duplicate mutation;
- retired source IDs remain permanently unavailable to category allocation;
- raw backup/restore and portable bundle schema 3 preserve lifecycle receipts and retired-ID custody;
- `sqlite doctor` rejects malformed receipt metadata/counts/timestamps and catalog reuse of retired identities.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 238 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 14 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- ten lifecycle publication fault boundaries: complete rollback;
- stale-preview, protected-checkpoint, receipt-custody, daily-revision, idempotent-retry, bundle round-trip, retired-ID nonreuse, and doctor-tamper proofs: pass;
- permanent diff audit: 12 intended source, test, authority, and Notebook files;
- temporary transformation, audit, proof, and workflow machinery: absent from the permanent tree.

RECONCILIATION-001C1 completes the SQLite authority half of issue #13. The issue remains open for C2: a prepared legacy-file lifecycle receipt with idempotent replay and an explicit user-visible review/confirmation surface shared across supported authorities.
