# Recovery authority

Status: partially implemented and certified
Current completed unit: RECONCILIATION-001B3A
Issue in progress: #10
Last reviewed: 2026-08-03

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

If current-generation checkpoint publication fails after an active transition, Strata enters visible persistence recovery before accepting ordinary input. It does not claim the transition is fully recoverable merely because one authority write succeeded.

## Startup identity validation

Checkpoint recovery validates evidence identity before applying sediment or active-session payload state.

- a checkpoint with no active stable identity is quarantined;
- a checkpoint naming a different active stable ID than authoritative active state is quarantined;
- malformed checkpoint JSON is quarantined;
- repository and database-integrity checks may reject impossible states even earlier;
- recovery cannot install stale identity in memory and discover the mismatch only during commit.

## Atomic initial active generation

Under SQLite authority, first TUI startup no longer publishes an active row before its first checkpoint.

- sediment authority is restored and validated before bootstrap publication;
- one typed bootstrap request carries the stable active identity, category, description, UTC start, checkpoint capture time, simulation time, and serialized runtime state;
- one immediate transaction verifies that neither active state nor checkpoint evidence already exists;
- the transaction inserts exactly one active row and one pending checkpoint whose dedicated identity column names the same stable generation;
- pre-existing checkpoint evidence of any status blocks bootstrap and remains unchanged;
- failures at `before-write`, `active`, `checkpoint`, or `commit` leave neither new row durable;
- a failed real TUI startup exits visibly and a later retry may create one clean generation.

The checkpoint identity column owns generation identity; the serialized payload owns runtime state. These are complementary parts of one checkpoint row, not duplicate identity fields. Existing recovery-only active reconstruction remains a separate internal primitive and is not used for ordinary initial startup.

Legacy-file startup remains one atomic checkpoint-file publication and does not gain a second competing active-session authority.

## Legacy switch transition receipts

Legacy switch transitions cross separate checkpoint, session CSV, and category-catalog files. They use a prepared receipt rather than pretending those files form one atomic transaction.

Publication order is:

1. stage the switch in memory;
2. publish a schema-3 resulting checkpoint carrying one deterministic switch receipt;
3. publish completed session history;
4. publish category-description state;
5. replace the checkpoint without the receipt.

If step 2 fails, the in-memory switch rolls back. After step 2 succeeds, the receipt remains the replay authority until every later publication succeeds.

The receipt binds:

- prior category and UTC active start;
- switch UTC timestamp;
- optional completed session identity and full temporal payload;
- resulting category, description, and UTC start;
- a deterministic operation ID derived from those boundaries.

Startup validates the receipt and resulting checkpoint generation before publication replay. It appends a missing completed row, exact-matches an existing row, and rejects conflicting or out-of-order history. It republishes category descriptions and clears the receipt only after session and catalog authority converge.

Whole-second semantics are explicit:

- one or more whole elapsed seconds require exactly one completed row;
- a zero-whole-second switch requires no completed row;
- subsecond monotonic remainder means the completed row starts at `switch UTC - whole elapsed seconds`, which may differ subsecond-wise from the original wall start;
- UTC endpoints, elapsed duration, civil labels, operational-day policy, and operational-day key must all agree.

Legacy session parsing rejects reserved ID `0`, malformed or duplicate IDs, malformed elapsed values, unknown categories, and conflicting receipt history rather than skipping or coercing them.

Kill-point tests certify convergence from receipt-only, receipt-plus-session, and receipt-plus-session-plus-catalog states. A catalog failure after session publication leaves the receipt durable for retry.

## Legacy finish transition receipts

Normal legacy finish is also a prepared multi-file transition.

Before mutating the active session, Strata publishes the prior-generation checkpoint with a deterministic finish receipt binding:

- prior category, description, and UTC start;
- canonical finish UTC;
- optional completed session identity and full temporal payload;
- the absence of a resulting active generation.

If prepared receipt publication fails, the active session remains unchanged. Once durable, finish proceeds through completed history, cleared category-description state, canonical sediment, and every affected daily contribution. The checkpoint is removed only after all of those authorities converge.

Startup recognizes a finish receipt before ordinary active recovery. It validates the prior checkpoint generation and whole-second boundaries, publishes missing effects idempotently, exact-matches an existing completed row, rejects conflict, reconciles every affected operational day, and deletes the receipt. A receipt-marked finished generation is never resumed as active.

Kill-point tests certify receipt-only, receipt-plus-session, receipt-plus-session-plus-catalog, and receipt-plus-session-plus-catalog-plus-sand states. A later publication failure retains the receipt. Retry also reconciles all affected days before receipt deletion, including multi-day sessions.

Normal legacy finish now persists the cleared active description. Legacy recovery flush and reload preserve active and archived category catalogs, archived session references, and archived sediment identities. Emergency recovery JSON schema 2 includes every category with an explicit `archived` flag.

## Clear-all and provisional-idle reset receipts

**Clear all sand and reset idle timer** is one receipt-governed operation, not a hidden ledger deletion.

- every committed session row, including idle history, is preserved;
- placed and pending canonical sediment become empty;
- an active idle interval is discarded only as provisional state and replaced by a new idle generation at the operation timestamp;
- a non-idle active generation preserves its stable identity, category, description, canonical elapsed duration, and UTC start;
- the receipt binds the operation timestamp, prior and resulting active state, canonical prior elapsed seconds, the complete sorted affected-day set, and an empty canonical `SandState`;
- affected days remain explicit even when the resulting ledger contribution is empty, so stale daily artifacts are deleted rather than becoming undiscoverable.

SQLite applies active-generation replacement when required, empty sediment, every explicit daily-contribution replacement or deletion, and the resulting checkpoint receipt in one immediate transaction. Existing completed history is neither inserted nor deleted. Fault injection at `before-write`, `active`, `sand`, `daily`, `checkpoint`, and `commit` proves complete rollback.

Legacy-file authority publishes the prepared resulting checkpoint and receipt before later effects. Prepared publication failure restores prior tracker, session, and sediment memory. Startup validates the receipt before ordinary detached recovery, restores the checkpoint's canonical grid and exact resulting active interval in memory, republishes empty sediment, reconciles every explicit operational day idempotently, and clears the receipt only after convergence. Repeated replay cannot duplicate elapsed time or restore pre-clear sediment.

Receipt identity includes canonical elapsed and the affected-day list. Changing either invalidates replay identity. A receipt whose sand payload is non-empty, whose active classification changes, whose elapsed value diverges from its UTC interval beyond the accepted live-clock tolerance, or whose days are malformed, duplicated, or unsorted fails closed.

## Bounded sediment recovery

The bounded sediment rules in `docs/SEDIMENT_AUTHORITY.md` remain authoritative:

- persist one fixed recovery target;
- restore canonical topology directly;
- calculate elapsed contribution with checked arithmetic;
- append compressed pending mass;
- never replay unbounded physics;
- publish recovered authority atomically where the storage authority supports it;
- retain or replace evidence according to explicit status.

Active/checkpoint generation coherence and transition receipts constrain which evidence may enter that process.

## Runtime and terminal failure

Draw, poll, and read failures attempt one direct emergency checkpoint before terminal restoration and error return. The original runtime error remains primary. Checkpoint and cleanup outcomes are context only.

Panic restoration returns the host terminal to normal state but does not claim application persistence.

Mandatory Ctrl-C during visible persistence recovery exports current recovery evidence before requesting exit.

## Remaining issue #10 recovery work

Legacy switch, normal finish, clear-all/provisional-idle reset, and initial SQLite active generation now have certified coherence boundaries. Issue #10 remains open only for exact transition-edge sediment attribution beyond the clear-all contract and user-visible recovery cutoff/reconstruction semantics.

## User-visible recovery cutoff

Current checkpoint recovery reconstructs from persisted checkpoint evidence toward a fixed recovery target. The final product contract must expose:

- checkpoint capture time;
- recovery target time;
- active category and description;
- reconstructed elapsed duration;
- whether the interval is exact, provisional, or reconstructed;
- the deterministic cutoff policy applied.

This presentation and policy remain unresolved. Recovery authority must not imply that elapsed time after the last durable evidence is exact without showing its reconstruction basis.

## Certification

RECONCILIATION-001B1, RECONCILIATION-001B2A, RECONCILIATION-001B2B, RECONCILIATION-001B2C, and RECONCILIATION-001B3A pass:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 219 library tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 13 SQLite/TUI process tests;
- 2 temporal-authority tests;
- 3 terminal-lifecycle PTY process tests.

Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch and finish rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, clear-all receipt identity over canonical elapsed and affected days, exact active-state staging before legacy daily reconstruction, cross-day idle authority, non-idle identity preservation, stale now-empty daily deletion, all six SQLite clear-all transaction kill points, atomic initial active/checkpoint publication, four bootstrap rollback boundaries, pre-existing checkpoint preservation, real TUI failure/retry, archived-authority reload, and schema-2 emergency export custody.

## Unresolved boundary

Full crash-recovery authority still requires:

- exact sediment classification at transition boundaries;
- explicit user-visible recovery cutoff and uncertainty semantics;
- any future safe queued-mutation replay based on stable cross-authority receipts.
