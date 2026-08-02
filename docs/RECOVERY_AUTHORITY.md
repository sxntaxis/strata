# Recovery authority

Status: partially implemented and certified
Current completed unit: RECONCILIATION-001B1
Issue in progress: #10
Last reviewed: 2026-08-02

## Purpose

Recovery authority preserves the relationship between active-session identity, chronological history, runtime checkpoint evidence, sediment state, and user-visible uncertainty across detach, runtime failure, process death, restart, and retry.

A checkpoint is evidence for one active generation. It cannot be silently applied to a replacement active identity, retired while under recovery custody, or treated as proof that a later multi-authority transition completed.

## Evidence states

SQLite checkpoint evidence has explicit status:

- `pending` — current recoverable runtime evidence;
- `recovering` — claimed by a recovery attempt;
- `committed` — recovered authority has been published but evidence remains reclaimable until replacement;
- `quarantined` — evidence is incompatible or malformed and must remain protected.

Normal replacement or retirement may affect only `pending` or `committed` evidence whose active stable ID matches the expected active generation. `recovering`, `quarantined`, missing-identity, or mismatched evidence blocks ordinary active transitions.

## Active generation coherence

SQLite switch, reset, and finish transactions own both the active-session transition and retirement of the prior checkpoint generation.

Within one transaction:

1. validate the expected active stable ID;
2. return an existing matching transition receipt when the operation already committed;
3. validate checkpoint status and identity;
4. retire replaceable prior-generation checkpoint evidence;
5. write completed history or zero-work receipt when applicable;
6. install the replacement active row or remove the completed active row;
7. commit all changes together.

If checkpoint custody is incompatible, the transaction fails before active history changes. A completed session cannot coexist with ordinary recoverable evidence that would resurrect its prior active generation.

## Immediate current-generation evidence

Periodic autosave is not the only heartbeat boundary.

The application publishes fresh runtime checkpoint evidence immediately after:

- a successful active switch;
- a successful active reset;
- a persisted description change for the currently active category.

The application does not refresh checkpoint evidence after unrelated category color, ordering, karma, archive, restore, or inactive-description changes. Checkpoint publication follows semantic recovery changes rather than every persistence call.

If current-generation checkpoint publication fails after an active transition, Strata enters visible persistence recovery before accepting ordinary input. It does not claim the transition is fully recoverable merely because the active-session transaction succeeded.

## Startup identity validation

Checkpoint recovery validates evidence identity before applying sediment or active-session payload state.

- a checkpoint with no active stable identity is quarantined;
- a checkpoint naming a different active stable ID than authoritative active state is quarantined;
- malformed checkpoint JSON is quarantined;
- repository and database-integrity checks may reject impossible states even earlier;
- recovery cannot install stale identity in memory and discover the mismatch only during commit.

## Bounded sediment recovery

The bounded sediment rules in `docs/SEDIMENT_AUTHORITY.md` remain authoritative:

- persist one fixed recovery target;
- restore canonical topology directly;
- calculate elapsed contribution with checked arithmetic;
- append compressed pending mass;
- never replay unbounded physics;
- publish recovered authority atomically where the storage authority supports it;
- retain or replace evidence according to explicit status.

Active/checkpoint generation coherence constrains which evidence may enter that process.

## Runtime and terminal failure

Draw, poll, and read failures attempt one direct emergency checkpoint before terminal restoration and error return. The original runtime error remains primary. Checkpoint and cleanup outcomes are context only.

Panic restoration returns the host terminal to normal state but does not claim application persistence.

Mandatory Ctrl-C during visible persistence recovery exports current recovery evidence before requesting exit.

## Legacy-file authority

Legacy authority now replaces its runtime checkpoint immediately after successful switch, reset, or active-description persistence. This eliminates the known autosave-length stale-evidence interval.

Legacy sessions and checkpoints remain separate atomic files, however. Immediate replacement is not equivalent to one atomic cross-file transition. A crash between publications can still leave a completed session file and a prior checkpoint that describe different transition stages.

Until stable transition receipts exist:

- issue #10 remains open;
- legacy multi-file transition atomicity is not claimed;
- recovery must fail visibly rather than silently inventing a cutoff;
- B1 cannot be used as evidence that every kill point is idempotently recoverable.

## Initial active start

SQLite active-session start and first checkpoint publication remain separate operations. The active row is authoritative chronological state, but a process death before the first checkpoint can leave no sediment/runtime evidence for the new active generation.

This window remains part of RECONCILIATION-001B2 or a later bounded unit. Full issue #10 closure requires an atomic start-plus-evidence transaction or an explicit certified recovery policy for active rows without checkpoints.

## User-visible recovery cutoff

Current checkpoint recovery reconstructs from persisted checkpoint evidence toward a fixed recovery target. The final product contract must expose:

- checkpoint capture time;
- recovery target time;
- active category and description;
- reconstructed elapsed duration;
- whether the interval is exact, provisional, or reconstructed;
- the deterministic cutoff policy applied.

This presentation and policy remain unresolved for B2. Recovery authority must not imply that elapsed time after the last durable evidence is exact without showing its reconstruction basis.

## Certification

RECONCILIATION-001B1 passes:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 190 library tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- 3 terminal-lifecycle PTY process tests.

Focused proofs cover transactional checkpoint retirement on switch/reset/finish, protected recovery evidence, mismatched checkpoint conflict, startup quarantine, idempotent receipts, immediate semantic-edge refresh, and absence of unrelated metadata checkpoint writes.

## Unresolved boundary

Full crash-recovery authority requires:

- stable legacy transition receipts and idempotent replay;
- kill-point certification between every legacy publication;
- initial active-start/checkpoint coherence;
- explicit user-visible recovery cutoff and uncertainty semantics;
- any future safe queued-mutation replay based on stable cross-authority receipts.