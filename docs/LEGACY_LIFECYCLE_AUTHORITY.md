# Legacy CLI lifecycle authority

Status: implemented and certified
Completed unit: FIX-004
Last reviewed: 2026-08-04

## Scope

A legacy-file `start` or `stop` is one profile mutation, not a sequence of independent file operations. Complete atomic file publication prevents torn bytes, but does not by itself prevent two processes from claiming the same lifecycle transition.

## Lock contract

- One profile-local lock file coordinates legacy CLI lifecycle mutation.
- The lock is acquired through the operating system before the active-session existence check.
- Acquisition is non-blocking; a competing mutation fails visibly rather than waiting indefinitely.
- The lock remains held through authority reads, completed-history publication, active-state publication or removal, and success reporting.
- Closing or crashing the process releases the operating-system lock automatically.
- The persistent lock pathname is not itself ownership evidence; only the live OS lock owns the transition.
- SQLite lifecycle transactions remain independently owned by SQLite and do not use this lock.

## Certified proofs

- Six rounds of twenty-four simultaneous legacy starts produce exactly one successful active generation.
- The persisted active project is the project reported by the sole successful caller.
- Six rounds of twenty-four simultaneous legacy stops produce exactly one successful terminal transition.
- One stop produces exactly one completed ledger row and removes the active state.
- Formatting, strict Clippy, the complete Rust suite, and every existing process suite remain green.
