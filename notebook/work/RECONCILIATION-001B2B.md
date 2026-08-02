---
id: RECONCILIATION-001B2B
kind: work
state: accepted
authority: promoted
created: 2026-08-02
updated: 2026-08-02
---

# RECONCILIATION-001B2B — legacy finish transition receipts

## Issue

Normal legacy-file exit previously mutated the active session in memory, then published session history, category metadata, sediment, daily contributions, and checkpoint deletion as separate operations.

Process death between those publications could leave a completed session row while the prior active checkpoint remained recoverable, duplicating time on restart. The finish path also cleared the active description only in memory, while legacy recovery reload and emergency export could lose archived category meaning.

## Accepted contract

### Prepared finish receipt

Before finalizing the active session in memory, Strata captures the prior active checkpoint and attaches one deterministic finish receipt. The receipt binds:

- prior active category, description, and UTC start;
- canonical finish UTC;
- optional completed session row, present exactly when one or more whole seconds exist;
- no resulting active generation.

The prepared checkpoint publishes before completed history or metadata effects. If it cannot publish, the active session and prior in-memory state remain unchanged.

### Publication order

Normal legacy finish follows:

1. publish prior-generation checkpoint plus finish receipt;
2. finalize the active interval in memory;
3. publish completed session history;
4. publish cleared category-description authority;
5. publish canonical sediment;
6. reconcile every affected operational-day contribution;
7. remove the checkpoint only after all authorities converge.

Retry follows the same custody rule. A multi-day session cannot retire its receipt after updating only the current day.

### Startup replay

Startup detects a finish receipt before ordinary active recovery.

It:

- validates schema, operation identity, prior checkpoint generation, category, description, UTC boundaries, whole elapsed seconds, civil labels, and operational-day policy;
- appends a missing completed row exactly once or exact-matches an already published row;
- rejects conflicting or out-of-order history;
- republishes cleared category metadata and canonical sediment;
- reconciles every affected daily contribution;
- removes the receipt only after convergence;
- starts a new active generation rather than resuming the receipt-marked finished one.

### Whole-second semantics

- One or more whole elapsed seconds require exactly one completed row.
- A zero-whole-second finish contains no completed row.
- The completed row starts at `finish UTC - whole elapsed seconds`; subsecond monotonic remainder does not require exact equality with the original wall start.
- The completed description must match the prior active description.

### Archived recovery custody

Legacy persistence recovery now treats active and archived category metadata as one reference authority.

- Full-state flush writes both active and archived catalog rows.
- Session validation during flush and reload resolves both active and archived identities.
- Reload refreshes `archived_categories` and restores sediment against active plus archived IDs.
- Emergency recovery JSON schema 2 exports every category with explicit `archived` state.
- A recovery action may not preserve session IDs while discarding the metadata needed to interpret them.

## Kill-point certification

The real finish publication helper is certified from four durable states:

1. prepared receipt only;
2. receipt plus completed session;
3. receipt plus completed session and cleared catalog;
4. receipt plus completed session, catalog, and sediment.

Every state converges to exactly one completed interval, cleared active description, canonical sediment, and retained receipt until final daily reconciliation/removal. A forced catalog-publication failure proves that session history may converge while sediment remains unpublished and the receipt remains durable for retry.

## Bugs found and fixed

1. Normal legacy finish had no stable replay identity across separate authority files.
2. Prepared-receipt failure could not previously guarantee unchanged active memory.
3. Normal finish did not persist the cleared active category description.
4. Startup could resume a checkpoint whose session had already been completed.
5. Finish retry reconciled only the current operational day before deleting evidence, leaving multi-day contributions stale.
6. Legacy recovery flush wrote active-only category metadata and could discard archived catalog rows.
7. Legacy recovery reload validated sessions and sediment against active categories only and did not refresh archived state.
8. Emergency recovery export omitted archived category metadata and archival state.
9. The obsolete active-only category writer remained after the authoritative catalog path replaced it.

## Certification

Exact implementation and authority-promotion baseline before this final record: `ef8b42f1c930f6283ac2fc39cb3169678b66447b`.

Passed:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 205 library tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- 3 terminal-lifecycle PTY process tests.

Focused proofs cover finish boundary validation, zero-whole-second finish, exact/idempotent session replay, all four durable finish kill points, publication-failure receipt custody, multi-day contribution reconciliation, archived session-reference reload, archived sediment identity, and schema-2 emergency category export.

## Remaining scope

Issue #10 remains open for:

- clear-all/reset receipt custody across deleted idle history, cleared sediment, replacement active state, and daily contributions;
- initial active-start/checkpoint coherence;
- exact sediment classification at active transition edges;
- user-visible checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty semantics.
