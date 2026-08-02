---
id: RECONCILIATION-001B2A
kind: work
state: active
authority: working
created: 2026-08-02
updated: 2026-08-02
---

# RECONCILIATION-001B2A — legacy switch transition receipts

## Issue

Legacy layer switching publishes three independent authority files:

1. the replacement runtime checkpoint;
2. completed session history;
3. category-description state.

Immediate checkpoint refresh reduced the stale interval, but process death between publications can still duplicate or omit the completed interval, restore the prior active description, or recover the wrong side of the switch.

## Selected contract

### Prepared switch receipt

Before publishing completed history, a legacy switch must publish a schema-versioned checkpoint for the resulting active generation containing one stable transition receipt. The receipt records:

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

Before ordinary detached recovery, startup must reconcile any receipt:

- append the completed session only when its ID is absent;
- exact-match an already published row;
- fail closed on conflicting same-ID history;
- restore prior/resulting category description effects;
- publish session and category files idempotently;
- clear the receipt only after both authorities succeed.

Repeated restart at every publication point must converge without duplicate elapsed time.

### Scope

This unit covers legacy **switch** transitions only. Reset, finish, initial active start, exact sand-edge allocation, and user-visible cutoff semantics remain for B2B/B2C.

## Acceptance proofs

- crash after prepared receipt but before sessions adds one completed row;
- crash after sessions but before category catalog exact-matches the row and completes metadata publication;
- crash after category catalog but before receipt clearing converges on repeat;
- conflicting same-ID session fails closed and retains receipt;
- prepared-checkpoint failure restores pre-switch in-memory state;
- old checkpoint schemas remain readable;
- all prior suites remain green.