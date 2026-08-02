---
id: RECONCILIATION-001B1
kind: work
state: completed
authority: accepted
created: 2026-08-02
updated: 2026-08-02
---

# RECONCILIATION-001B1 — active identity and checkpoint coherence

## Issue

Issue #10 was substantially improved by the SQLite active-session, finish-receipt, bounded checkpoint, persistence-recovery, and terminal-emergency programs. Criterion-by-criterion crash-window reconciliation nevertheless found concrete remaining contradictions:

- SQLite switch, reset, or finish could commit a replacement active generation while leaving checkpoint evidence bound to the prior stable ID;
- the next checkpoint write could then conflict, and restart could attempt to claim stale evidence;
- legacy switch and reset left a known pre-transition checkpoint as the latest recoverable payload until periodic autosave;
- active description changes could be newer than the checkpoint used for restart recovery;
- an early implementation audit forced checkpoint writes after unrelated category metadata changes, creating false recovery risk.

## Accepted contract

### Coherent active generation

A successful active-session switch, reset, or finish must not leave ordinary checkpoint evidence for the replaced active stable identity.

SQLite now retires checkpoint evidence inside the same transaction as the active transition:

- no checkpoint is valid;
- `pending` or `committed` evidence is replaceable only when it names the expected prior stable ID;
- `recovering` or `quarantined` evidence blocks the transition;
- missing or mismatched active identity blocks the transition;
- the transaction rolls back before completed history or replacement active state changes when checkpoint custody is incompatible.

Idempotent transition receipts remain authoritative: a repeated operation with an existing matching receipt returns before attempting to retire a later checkpoint generation.

### Immediate checkpoint refresh

After a successful active switch or reset, the application publishes a checkpoint for the new active identity immediately rather than waiting for the periodic autosave interval.

After a persisted description edit, the application refreshes the checkpoint only when the edited category is the current active category. Unrelated color, order, karma, archive, restore, or inactive-description changes do not force a checkpoint write and cannot create false persistence recovery.

If the immediate refresh fails after a successful active transition, Strata enters the existing visible persistence-recovery contract before accepting ordinary input. It does not report the transition as fully recoverable.

### Startup validation

SQLite startup validates claimed checkpoint identity against authoritative active-session state before applying checkpoint payload recovery.

- missing active identity is quarantined and fails visibly;
- mismatched active identity is quarantined and fails visibly;
- malformed payload remains quarantined;
- recovery never installs one identity and discovers the contradiction only at commit time.

SQLite schema and foreign-key integrity already prevent several fabricated mismatched states before the TUI checkpoint loader. Tests therefore exercise the reachable missing-identity corruption through an activated SQLite authority fixture.

### Legacy boundary

Legacy-file authority immediately replaces the runtime checkpoint after every successful active switch, reset, or active-description persistence. It no longer knowingly leaves a stale pre-transition payload until autosave.

This establishes current-generation coherence but does not make the multi-file transition atomic. A process death between legacy session publication and replacement checkpoint publication still requires a stable transition receipt and deterministic replay policy.

## Bugs found and fixed

1. SQLite switch/reset/finish could leave a stale checkpoint bound to the prior active stable ID.
2. Recovering, quarantined, identity-less, or mismatched evidence did not participate in the active-transition transaction boundary.
3. Legacy switch/reset did not refresh checkpoint evidence immediately.
4. Active-description changes could remain newer than recovery evidence.
5. Startup checkpoint identity validation occurred too late for malformed payloads.
6. A broad first implementation refreshed checkpoints after every category persistence, including unrelated metadata, creating unnecessary writes and false-recovery exposure.
7. The focused startup corruption fixture initially omitted activated SQLite authority metadata and therefore tested startup authority rejection instead of checkpoint quarantine; the fixture was corrected rather than weakening production validation.

## Certified proofs

- switch with a pending checkpoint cannot leave the prior stable ID recoverable;
- reset with a pending checkpoint cannot leave the prior stable ID recoverable;
- finish with a pending checkpoint cannot resurrect the completed active session;
- recovering and mismatched evidence block active transitions without partial session mutation;
- transition receipts remain idempotent across later checkpoint generations;
- missing checkpoint identity is quarantined under activated SQLite authority;
- successful switch/reset refreshes the current active checkpoint immediately;
- active-description edits refresh recovery evidence while unrelated category metadata does not;
- formatting passes;
- strict Clippy with all targets/features and warnings denied passes;
- 190 library tests pass;
- 9 CLI lifecycle tests pass;
- 6 configuration-authority tests pass;
- 1 report-help regression test passes;
- 12 SQLite/TUI process tests pass;
- 2 temporal-authority tests pass;
- 3 terminal-lifecycle PTY process tests pass.

## Durable authority

- `docs/RECOVERY_AUTHORITY.md` records active/checkpoint generation coherence and the remaining legacy transaction boundary;
- `docs/ARCHITECTURE.md` assigns checkpoint retirement to runtime coordination transactions and semantic-edge refresh to application orchestration;
- STRATA-D043 and STRATA-D044 constrain checkpoint generation transitions and startup identity validation;
- `notebook/work/ISSUE-RECONCILIATION-001.md` records B1 as accepted partial satisfaction of issue #10.

## Remaining boundary

Issue #10 remains open. RECONCILIATION-001B2 must:

- add stable legacy transition receipts for switch, reset, and finish;
- certify process death between every legacy file publication;
- replay or reconcile transitions idempotently without duplicate sessions;
- address the initial active-start/checkpoint publication window;
- make the recovery cutoff and reconstructed interval explicit and user-visible.

B1 must not be cited as full crash-recovery closure.