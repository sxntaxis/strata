---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-27
authority: working
summary: HISTORY-001A is natively green; HISTORY-001B From/To range editing is implemented and awaiting native validation.
next: Native-validate HISTORY-001B custom From/To range editing, then begin HISTORY-001C retroactive missed activity.
---

# NOW — Strata

## Current phase

ARCH-001 has been completed. The post-SQLite issue reconciliation program is complete, and the current runtime is SQLite-only. The v0.7.7 direct interaction model is reconciled onto that authority.

A post-merge bootstrap recovery defect was found on a real profile: first-generation checkpoint creation
could persist `active_session_started_at_utc` slightly after `simulation_time_utc`. Hotfix
`f94a919675357c0d4d41f58168c5a95b05a188ca` aligns new bootstrap boundaries and narrowly repairs only the
original Idle `tui-<start>-<pid>` checkpoint shape. Non-bootstrap inversions remain fail-closed.

A second real-profile defect was then reproduced conceptually from owner evidence: detaching while accelerated catch-up still had a queued mutation caused detached checkpoint publication to fail with `runtime checkpoint cannot be written while mutations are pending`. The owner also set a product constraint that catching up to current time should take no more than eight seconds. The certified hotfix removes the live queued-mutation dependency: long backlog uses bounded sediment settlement, mutations during catch-up settle directly to their exact UTC boundary, detach settles before checkpoint publication, and autosave defers while catch-up remains active. Native PTY proof resumed a 15-second stopped TUI and detached in 1.118 seconds with empty pending-mutation evidence; restart completed in 27 milliseconds.

A third real-profile defect was triggered by `C` while Idle: operational-day allocation rejected `32936 of 32937 seconds`. The canonical session duration was correct; the allocator independently floored each wall-clock slice around an exact operational-day boundary. With a sub-second session start, those two floors can lose one whole second. The certified allocator now allocates cumulative whole seconds from the session start and includes the observed 32,937-second cross-boundary clear-all shape as regression evidence. The copied real profile cleared through the normal TUI path without persistence recovery, persisted empty sediment and `pending_mutations: []`, detached, and restarted successfully.

The visible-basin refinement is natively certified: visible viewport bounds are the live physics basin;
hidden topology freezes while cropped and reactivates on expansion; full `c` resets the empty canonical
canvas to the current viewport; uppercase `C` preserves extent and non-Idle mass through real PTY and
SQLite persistence; restore into a larger live viewport expands monotonically; zero-viewport recovery
restore remains exact; the idle tamagotchi is removed; and zero effective counters are hidden.

The day-end visual-memory behavior is now accepted as working baseline product behavior. A live operational-day
cutoff stages the exact cumulative canonical `SandState` as a first-write-wins `daily` checkpoint, including its
original canvas dimensions and topology. Historical Balance prefers that authentic photo for the selected interval
end day; `DailyContribution` is dimension-independent ledger mass only, and the deterministic row-major
`DerivedPreview` remains fallback for cutoffs that were not observed by live physics. No historical photo is
fabricated during bounded recovery.

The certified system includes:

- fail-closed profile-bound SQLite runtime persistence with one current schema;
- monotonic/UTC/fixed-offset time and exact operational-day allocation;
- canonical category/layer, session, active-generation, and report identity;
- conserved sediment, bounded recovery, immutable historical artifacts, and revision-matched daily contributions;
- receipt-governed switch/finish/reset transitions plus atomic receipt-free clear-all;
- active/archived category integrity with stable archive/restore identity;
- explicit report editing, truthful keymap/palette/atlas routing, and exactly-once terminal restoration;
- session-owned active description drafts separated from durable category metadata and reusable tags;
- one process-bound profile UUID owning complete data, state, configuration, recovery, and SQLite paths;
- profile-scoped live CLI-to-TUI control with short-path and long-path socket publication recovery;
- responsive monotonic sediment canvas expansion with conservation;
- real process proofs for profile isolation, copied-artifact refusal, persistence failure, live control, and PTY restoration.

The transitional CSV/JSON runtime, authority selection, activation ceremony, and historical schema
upgrade chain are retired. Portable bundle export/import and SQLite doctor, backup, and restore remain
product functionality. Runtime recovery, checkpoints, receipts, categories, sessions, and sediment are
SQLite-owned.

The independent `project` axis and speculative category merge/permanent-delete lifecycle are retired.
Clear-all is receipt-free and atomic. Switch, finish, and reset receipts remain only for their real
runtime/checkpoint failure boundary. Final native validation passed formatting, strict Clippy, the full
test suite, build, help smoke, diff hygiene, and the long-profile dangling-symlink runtime proof.

## Current plateau program

`notebook/work/PLATEAU-001.md` is the durable roadmap for the remaining core-development arc. The owner has accepted the existing day-end snapshot behavior as baseline and does not want an artificial re-certification gate before continuing.

The active edge is **HISTORY-001B native validation — visible From/To selection in Balance**. HISTORY-001A is integrated and green. HISTORY-001B now adds `range` beside Day/Week/Month, a configurable default `r` action, inline ISO From/To editing with validation, and same-span custom-window navigation capped at the current operational day. Summary rows, detail logs, provisional active time, and historical sediment remain routed through the shared `ReportWindow`; no second reporting implementation was introduced.

The stale dirty adaptive-resize branch remains preserved as custody evidence and is superseded by the current-main
visible-basin, resize/restore, and atomic clear-all authority. No adaptive code was ported.

Later required order is HISTORY-001B/C/D (custom range and historical correction), optional HISTORY-001E current-sediment recolor, SEDIMENT-002 organic formation, then INTERACTION-002 menu/Settings convergence and product hardening. Pomodoro/RHYTHM-001 remains optional and must not block plateau.

## Completed post-migration units

- **AUTHORITY-001** — issue #21.
- **AUTHORITY-002** — issues #22 and #15.
- **TEMPORAL-001** — issue #25.
- **TEMPORAL-002** — issues #4, #23, #27.
- **DOMAIN-001** — issues #2, #12.
- **REPORT-001** — issues #1, #3, #14, #17, #28.
- **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
- **INTERACTION-001A** — issue #19.
- **INTERACTION-001B** — issue #20.
- **INTERACTION-001C** — issue #24.
- **RECONCILIATION-001A** — issue #5 and historical-meaning portion of #13.
- **RECONCILIATION-001B1/B2A/B2B/B2C/B3A/B3B/B3C** — issue #10.
- **RECONCILIATION-001C1/C2** — issue #13.

## Verified final baseline

- final native HEAD is `b9fecfa3d277d4e42dfc92aa3ee532d5832ec4f8`;
- formatting, strict Clippy, 196 unit tests, 22 integration/process tests, build, help smoke, and diff hygiene pass;
- fresh-profile direct-SQLite and profile-isolation proofs pass;
- short-path active-socket refusal and long-path live-control proofs pass;
- dangling long-path publication cleanup passes across a real TUI restart;
- copied real-profile recovery completes with coherent checkpoint timestamps and subsequent restart succeeds;
- the non-bootstrap `tui-active:*` inversion remains rejected with the existing recovery error.
- 48-minute and 24-hour bounded checkpoint recovery each complete in 26 milliseconds;
- historical checkpoints containing queued mutation evidence remain fail-closed with the stable-identity error.

## Certification evidence

- current schema initializes fresh databases transactionally at `user_version = 1` and rejects other development versions;
- strict storage-authority residue search is empty outside the authoritative decision record;
- formatting, strict Clippy, tests, fresh-profile smoke proof, help output, diff hygiene, final long-path IPC runtime certification, copied-profile bootstrap recovery certification, bounded catch-up PTY proof, and copied-profile clear-all proof were run for the bounded-catch-up head.

## Known non-blocking questions

The accepted implementation does not settle every possible future product direction. Remaining design questions include vertical chronology, optional category relationships, future sediment clearing/formation semantics, zoom/compression/panning, configurable quantum migration, possible IANA timezone support, and any future stable identity for queued cross-authority mutation replay.

These are not open implementation defects. They require new evidence and an explicit future unit before constraining the current system.

## Next

Run repository-native formatting, strict Clippy, full tests, help smoke, and a focused PTY/interaction proof for HISTORY-001B. Validate `r` range entry, From/To overwrite and field switching, reversed/invalid rejection, commit/cancel behavior, custom-range summary/log/live routing, same-span left/right navigation, and return to preset ranges. Fix only HISTORY-001B fallout. Once green, mark HISTORY-001B complete and begin HISTORY-001C — retroactive missed activity.
