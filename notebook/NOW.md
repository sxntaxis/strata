---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-28
authority: working
summary: The validated plateau is installed on the real profile; PLATEAU-001H H1 hardens nozzle-like ingress and Balance footer clutter from first daily-use evidence.
next: Native-validate H1 against the published plateau, then cut it over only if rain breadth, Balance footer clarity, and all persistence/history regressions remain green.
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
- explicit report editing, truthful keymap/palette/Settings routing, and exactly-once terminal restoration;
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

HISTORY-001B is now **COMPLETE / NATIVE-GREEN** at `ce9dd7281d3fb064302099e7cb274800c4f0ca9c`: formatter, strict Clippy, 238 tests, help smoke, targeted custom-range interaction proof, preset regression, provisional-live routing, and historical-sediment end-day selection all pass. The final delta over the authored candidate was rustfmt-only.

HISTORY-001C is now **COMPLETE / NATIVE-GREEN** at `09412b703cf41016f889d725ba235a7a1e63ae6a`: formatting, strict Clippy, 245 tests, focused editor/transaction proofs, isolated startup smoke, and validator profile-custody proof all pass. Its completed-Idle split transaction, active-preview validation, atomic daily-contribution replacement, rollback proof, and post-commit memory reload are retained as the safe foundation for generalized historical editing.

HISTORY-001D is now **COMPLETE / NATIVE-GREEN** at `bfaa8bf29f8019c25fe4f2ee8b1d60c554e5e988`: formatting, strict Clippy, 258 tests, 18 focused generalized transaction tests, Balance/report regression, isolated startup, and bubblewrap profile-custody proof all pass. Arbitrary `From < To <= now` assignment, gap insertion, collision confirmation, active-generation rebasing with protected live selection, daily-contribution reconciliation, rollback, and sediment/snapshot non-mutation are certified.

HISTORY-001E is now **COMPLETE / NATIVE-GREEN** at `d67c8e382708dbbf3f71bf2a67d7daa81b2e36b8`: formatting, strict Clippy, 263 tests, pure recolor and historical-sediment transaction proofs, HISTORY-001D and sediment regression, isolated startup/detach/restart, and bubblewrap profile-custody proof all pass. Retained source-category mass recolors deterministically in place; true historical gaps and cleared-away mass never fabricate current grains; authentic first-write day-end snapshots remain immutable.

SEDIMENT-002 is **COMPLETE / NATIVE-GREEN** at `4059e28df2ebf82bd31453ee208093eef57a4511`: formatting, strict Clippy, 280 tests, focused friction/biased-rain/recovery proofs, visible-basin and HISTORY-001E regression, isolated startup/detach/restart, and bubblewrap profile-custody proof all pass.

INTERACTION-002 is **COMPLETE / NATIVE-GREEN** at `b0f60eb3c6d76d1afee8d46737baab8ed220b01b`: formatting, strict Clippy, 283 tests, Settings/keymap and INTERACTION-001 regression, Balance/HISTORY and SEDIMENT-002 regression, isolated TUI startup, and bubblewrap profile-custody proof all pass. The former Atlas is now plain Settings; human action labels and Main / Navigation / Layer / Balance / Settings grouping are authoritative; the misplaced Main `t → Detach` fallback is retired; historical physical keys remain scoped to Balance while the command palette is the deliberate universal launcher.

The plateau cutover is **COMPLETE / REAL-PROFILE GREEN** from published main `3062e115de1bdf16985275ff1476ba22f213f744`: the new binary started on the real profile, v1/v2 sediment restored and persisted as SandState v3 with a valid ingress focus, Settings and Balance were visible, normal restart passed, and the pre-v3 backup was retained.

The active edge is **PLATEAU-001H / H1 — de-nozzle rain + contextual Balance footer**. First owner screenshots after cutover showed two bounded daily-use defects: falling grains still reveal the persisted ingress focus too clearly, and the Balance bottom border stacks technical status/period/action hints such that the bare `l` is not understandable. H1 is implemented as a source candidate: every drop keeps full-width support with only occasional soft focus preference; current `live sediment` footer noise is hidden; period hints stay on the default summary; detail/editor footers become contextual; and the visible retroactive action is **Log past activity…** while `balance_log_activity` remains the stable config/API name. Native validation is pending.

The stale dirty adaptive-resize branch remains preserved as custody evidence and is superseded by the current-main
visible-basin, resize/restore, and atomic clear-all authority. No adaptive code was ported.

The required core path is now PLATEAU-001H product hardening. Pomodoro/RHYTHM-001 remains optional and must not block plateau.

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

The accepted implementation does not settle every possible future product direction. Remaining design questions include vertical chronology, optional category relationships, future sediment clear semantics and formation controls beyond SEDIMENT-002, zoom/compression/panning, configurable quantum migration, possible IANA timezone support, and any future stable identity for queued cross-authority mutation replay.

These are not open implementation defects. They require new evidence and an explicit future unit before constraining the current system.

## Next

Freeze and natively validate the HISTORY-001D generalized historical-assignment candidate. Prove arbitrary gap insertion, transparent Idle/same-layer overlap, multi-session collision preview plus exact-plan confirmation, future rejection, corrupted-overlap refusal, active-generation correction with protected live layer/description, same-layer active backdating including exact-boundary touch, whole-second/fractional conservation, atomic checkpoint and daily-contribution publication, rollback, post-commit in-memory reload, and unchanged current sediment/authentic day-end snapshots. Fix only HISTORY-001D fallout.
