---
id: RECONCILIATION-001B2A
kind: work
state: accepted
authority: promoted
created: 2026-08-02
updated: 2026-08-02
---

# RECONCILIATION-001B2A — legacy switch transition receipts

## Issue

Legacy layer switching publishes three independent authority files:

1. the replacement runtime checkpoint;
2. completed session history;
3. category-description state.

Immediate checkpoint refresh reduced the stale interval, but process death between publications could still duplicate or omit the completed interval, restore the prior active description, or recover the wrong side of the switch.

## Accepted contract

### Prepared switch receipt

Before publishing completed history, a legacy switch publishes a schema-versioned checkpoint for the resulting active generation containing one stable transition receipt. The receipt records:

- deterministic operation ID;
- expected prior category and UTC start;
- switch UTC timestamp;
- optional completed session row with its stable numeric ID;
- resulting active category, description, and UTC start.

The prepared checkpoint is the replay authority for the multi-file transition.

### Publication order

1. stage the transition in memory;
2. publish the resulting checkpoint with the receipt;
3. publish session CSV idempotently;
4. publish active/archived category catalog state;
5. replace the checkpoint without the receipt.

If prepared checkpoint publication fails, in-memory transition state rolls back. After the prepared receipt succeeds, any later failure retains the transitioned memory plus durable receipt and enters visible persistence recovery.

### Startup replay

Before ordinary detached recovery, startup reconciles any switch receipt:

- append the completed session only when its ID is absent;
- exact-match an already published row;
- fail closed on conflicting same-ID or out-of-order history;
- restore prior/resulting category-description effects;
- publish session and category files idempotently;
- clear the receipt only after both authorities succeed.

Repeated restart at every publication point converges without duplicate elapsed time.

### Whole-second and temporal integrity

Receipt validation follows canonical ledger semantics rather than raw wall-start equality.

- A completed row is required exactly when the prior active interval owns one or more whole seconds.
- Zero-whole-second switches contain no completed row.
- Subsecond monotonic remainder is preserved by validating the completed row start as `transition UTC - whole elapsed seconds`; it is not required to equal the original wall start exactly.
- Completed UTC endpoints, elapsed duration, civil labels, operational-day policy, and operational-day key must agree.
- Receipt operation identity, prior category/start, resulting category/start/description, and checkpoint generation must agree.
- Session ID `0`, malformed IDs, duplicate IDs, malformed elapsed values, impossible policy, and conflicting replay rows fail closed.

### Kill-point custody

The real publication helper is certified from every durable switch crash state:

1. prepared receipt only;
2. prepared receipt plus completed session row;
3. prepared receipt plus completed session row and converged category catalog.

All three converge to exactly one completed row, corrected category metadata, and a receipt-free resulting checkpoint. If category publication fails after session publication, the session row may remain durable but the checkpoint receipt remains present for retry.

## Certification

Exact implementation/proof head before authority promotion: `61204a0e3f56919e26cc6de73f3d56eeac80fdb1`.

Passed:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 199 library tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- 3 terminal-lifecycle PTY process tests.

Focused proofs cover prepared-checkpoint rollback, exact/idempotent session reconciliation, conflicting and older history rejection, subsecond whole-second boundary semantics, strict receipt payload validation, legacy CSV identity integrity, convergence from all three persisted switch kill points, and receipt retention after catalog-publication failure.

## Remaining scope

This unit certifies legacy **switch** transitions only. Issue #10 remains open for:

- legacy reset and finish receipts;
- initial active-start/checkpoint coherence;
- exact sediment classification at transition edges;
- user-visible recovery cutoff, reconstruction, and uncertainty semantics.
