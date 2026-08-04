---
id: RECONCILIATION-001B2C
kind: work
state: accepted
authority: accepted
created: 2026-08-02
updated: 2026-08-03
---

# RECONCILIATION-001B2C — clear-all sediment and idle-reset custody

## Issue

The public command is named and described as **Clear all sand and reset idle timer**. The implementation additionally deletes committed idle sessions for the current operational day. That hidden ledger mutation is not part of the interface contract, can erase cross-day canonical history wholesale, and creates non-atomic session/sand/checkpoint/daily state under both SQLite and legacy-file authority.

Post-mutation daily-artifact discovery also omits a day that becomes empty, allowing a stale daily contribution to survive.

## Selected contract

### Product meaning

Clear-all is a sediment operation plus an explicitly visible provisional-idle reset.

- Clear every placed and pending grain from canonical sediment.
- Preserve every committed session row, including idle history.
- If the active category is idle, discard only the current provisional idle interval and start a new idle active generation at the operation timestamp.
- If the active category is not idle, preserve its active identity and UTC start.
- Do not reinterpret visual clearing as chronological-ledger deletion.

### Affected-day authority

The operation records the union of operational days touched by the pre-reset provisional idle interval and the operation day. Those days remain explicit even when the post-operation ledger has no slices, so stale daily contributions are deleted rather than becoming undiscoverable.

### Prepared receipt

The resulting runtime checkpoint carries one deterministic clear-all receipt binding:

- operation ID and UTC timestamp;
- prior active category, description, and UTC start;
- resulting active category, description, and UTC start;
- whether idle reset occurred;
- canonical prior elapsed seconds;
- explicit affected operational days;
- an empty canonical `SandState`.

### SQLite boundary

SQLite applies, in one immediate transaction:

1. validate active identity and checkpoint custody;
2. optionally replace the active idle generation;
3. publish empty canonical sand;
4. replace or delete every explicit affected daily contribution;
5. publish the resulting checkpoint with receipt evidence.

No completed session is deleted or inserted.

### Legacy boundary

Legacy authority stages the same resulting memory state and publishes the checkpoint receipt first. It then publishes empty sand and reconciles every explicit affected day. Prepared-receipt failure restores prior memory and sediment. The receipt clears only after every named authority converges.

### Startup replay

Startup validates the receipt and resulting checkpoint generation before ordinary detached recovery. It republishes empty sediment and all explicit affected daily contributions idempotently, then removes the receipt while retaining the resulting active checkpoint for ordinary recovery.

## Acceptance proofs

- committed idle sessions survive clear-all under both authorities;
- cross-day idle sessions are never deleted or fragmented;
- active non-idle identity and start survive unchanged;
- active idle resets to one new generation with no completed row;
- every explicit affected day is replaced or deleted, including now-empty days;
- prepared legacy failure restores prior tracker, session, and sand state;
- SQLite fault injection rolls the entire clear-all transaction back;
- restart and repeated replay converge without duplicate time or restored pre-clear sediment;
- all switch, finish, recovery, sediment, CLI/TUI, and PTY gates remain green.

## Boundary

This unit does not yet close issue #10. Initial active-start/checkpoint coherence, final transition-edge sediment timing, and user-visible recovery cutoff/reconstruction semantics remain subsequent bounded work.

## Implemented result

- committed idle and non-idle session history is never deleted by clear-all;
- idle clears start one new provisional idle generation without a completed row;
- non-idle active stable identity, description, canonical elapsed, and UTC start survive;
- SQLite owns one immediate transaction across active state, empty canonical sediment, every explicit daily replacement/deletion, and resulting checkpoint receipt;
- legacy prepared publication rolls memory back before receipt durability and replays exact active/grid state before deriving daily contributions;
- operation identity binds canonical elapsed and the complete affected-day set;
- non-empty clear-all sediment payloads and ambiguous receipt boundaries fail closed;
- every SQLite kill point (`before-write`, `active`, `sand`, `daily`, `checkpoint`, `commit`) rolls all authorities back;
- cross-day idle intervals name every touched operational day, while non-idle clear names only the operation day;
- stale now-empty daily artifacts are deleted explicitly.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 215 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 12 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- temporary transformation and audit machinery: absent from the permanent tree.

The unit is accepted as a partial completion of issue #10. It does not claim initial active-start/checkpoint atomicity, complete transition-edge sediment attribution, or final user-visible recovery cutoff/reconstruction semantics.

## Merge gate

The documentation-complete branch must pass ordinary GitHub Actions on this exact permanent head before the pull request is marked ready for review.
