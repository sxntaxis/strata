---
id: RECONCILIATION-001C2
kind: work
state: active
authority: working
created: 2026-08-03
updated: 2026-08-03
---

# RECONCILIATION-001C2 — legacy lifecycle replay and explicit confirmation

## Issue

RECONCILIATION-001C1 established complete SQLite category lifecycle authority. Issue #13 remains open because legacy-file authority still lacks crash-safe merge/deletion replay, retired-ID custody, and a user-visible workflow distinct from ordinary archive.

The current `x` category action archives a category. That behavior is safe and remains unchanged. Destructive or transformative lifecycle is a separate action and interaction state.

## Selected contract

### Shared preview

Both SQLite and legacy paths expose the same semantic preview:

- explicit source stable ID;
- explicit target stable ID or explicit targetless deletion;
- source and target metadata snapshots;
- completed-session, active-state, tag, placed/pending sediment, snapshot/daily-artifact, and detached/runtime-checkpoint reference counts;
- checkpoint/recovery custody status;
- one deterministic revision over every mutation-relevant authority;
- exact confirmation phrase derived from source, target, and revision.

Confirmation is rejected when the preview revision changes.

### Interaction

- `x` continues to archive the selected non-idle category;
- a distinct configurable `Shift-X` lifecycle action opens a blocking overlay;
- the overlay first chooses an explicit target category or explicit permanent deletion;
- review displays source/target IDs and names, every affected count, checkpoint custody, and the revision;
- the user must type the exact displayed phrase before Enter may apply;
- Esc cancels without mutation;
- ordinary category controls remain blocked while the overlay owns input;
- archive remains the recommended/default retirement operation.

### Legacy prepared receipt

Legacy lifecycle uses two separate artifacts:

1. a **prepared receipt** containing the reviewed source/target metadata, complete counts, revision, operation ID, exact resulting catalog, session ledger, tags, canonical sediment, every affected daily contribution, receipt-free detached checkpoint payload, and resulting permanent lifecycle ledger;
2. a **permanent lifecycle ledger** containing committed merge/deletion receipts and retired source IDs.

The prepared receipt is atomically published before any resulting authority file. It is the sole replay source after that point.

Publication order is deterministic:

1. prepared receipt;
2. session ledger;
3. category tags;
4. canonical sediment;
5. every affected daily contribution, including explicit deletion of now-empty artifacts;
6. detached checkpoint payload where present;
7. category catalog;
8. permanent lifecycle ledger;
9. prepared receipt removal.

Startup detects a prepared receipt before ordinary detached recovery, validates it against supported schema and stable identities, and idempotently republishes each exact resulting artifact. Existing exact artifacts are accepted; conflicting artifacts fail closed. The receipt remains until every named authority and the permanent ledger converge.

### Permanent identity custody

- source idle is forbidden;
- source and target must differ;
- targetless deletion requires zero references in the complete preview;
- every committed source ID remains retired forever;
- legacy category allocation considers both current catalog IDs and permanent lifecycle-ledger source/target IDs;
- permanent and prepared receipt files use atomic private JSON publication;
- migration to SQLite imports committed lifecycle receipts and preserves the identity high-water mark.

### Recovery constraints

- malformed detached checkpoints fail closed;
- unresolved switch, finish, or clear receipts block lifecycle preparation;
- no lifecycle operation may run while persistence recovery or recovery acknowledgment owns input;
- failed preparation leaves memory and every authority unchanged;
- after prepared receipt publication, failure enters visible recovery and retains replay evidence;
- repeated startup or repeated confirmation cannot duplicate mutation.

## Acceptance proofs

- archive action remains unchanged and distinct from lifecycle action;
- lifecycle action is configurable and truthfully represented in atlas/palette/runtime routing;
- explicit target/deletion selection and exact phrase confirmation are required;
- source idle, self-merge, missing target, stale preview, nonzero-reference delete, protected checkpoint, and receipt-bearing checkpoint fail closed;
- successful legacy merge preserves session IDs, chronology, elapsed time, active start/description, target metadata, sediment mass, and FIFO category order;
- tags merge deterministically and daily contributions match reassigned ledger truth;
- prepared-receipt failure leaves all authority unchanged;
- kill points after each publication step converge idempotently on restart;
- permanent ledger prevents retired-ID reuse and survives restart and SQLite migration;
- SQLite TUI lifecycle uses the same review/confirmation surface and delegates mutation to C1;
- issue #13 closes only after both authorities and the complete interaction/process suite pass.
