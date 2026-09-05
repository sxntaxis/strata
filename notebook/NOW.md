---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-09-04
authority: working
summary: H4/v5 remains published production authority and SEDIMENT-008 oslo-vessel-front physics remains owner-frozen. SEDIMENT-015A-R2 and SEDIMENT-015B are machine-green, but the owner rejected 015B visually: waiting white ingress formed a line, dense active mass still felt under-represented, color progression was reconstructed strangely in y, and the fixed 64 ms testing clock left CPU idle while playback slowed. SEDIMENT-015C now owns perceptual conservation: event-time observed y geometry, full local raster-capacity search, offscreen geometry without top-row clamping, ingress isolation during active testing flow, and backlog-aware accelerated presentation while unit mass/routing/custody stay unchanged.
next: Natively validate SEDIMENT-015C. Run constructor/geometry/lane/raster/ingress/frozen-parity gates first, then the 015B and 015A suites, legacy conservative/Oslo gates, testing-clock regression, and explicit ignored 5k/10k/20k/40k scaling probe. Only after full repository green return to the owner rainbow-collapse human gate.
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

SEDIMENT-007 is the current bounded experiment. Owner evidence now selects `oslo-vessel` as the strongest ordinary topology, while Classic remains the reference for dramatic wall-collapse motion. WaveView presentation batching was rejected because it only accelerated unchanged local topplings. Inertia v1 was rejected because its global size/span gate required the avalanche to already be large before the mechanism that was supposed to make it large could engage. SEDIMENT-007 therefore keeps ordinary quiescent vessel evolution exact and introduces a local moving-grain phase only when a single topple sees relief `>= 4` — a discontinuity ordinary one-grain Oslo drive cannot create from a quiescent `{1,2}` surface but horizontal canonical reconnection can. See `notebook/work/SEDIMENT-007-OSLO-VESSEL-MOMENTUM.md`.

SEDIMENT-008 is now **OWNER-FROZEN DEBUG PHYSICS** at `8817e1990188c76b50246e6d84edcdde79a19a60`: Oslo-vessel static topology plus bounded erosion/deposition exchange and uphill support-loss recruitment produced the first wall-collapse behavior the owner strongly accepted. Later units must fork it rather than tune it in place. See `notebook/work/SEDIMENT-008-OSLO-VESSEL-FRONT.md`.

SEDIMENT-009 is the current bounded experiment. It leaves frozen SEDIMENT-008 physics intact, fixes the testing harness so explicit moving phases advance at a stable 32 ms wall-clock physics cadence even during accelerated synthetic time, adds `testingcheats fill` for an 80%-visible-window horizontal category-layer fixture, and introduces `oslo-vessel-fluid`: a curiosity-only discrete partial-fluidization/order-field fork inspired by BCRE and Aranson-Tsimring. See `notebook/work/SEDIMENT-009-OSLO-VESSEL-PARTIAL-FLUIDIZATION.md`.

SEDIMENT-010 supersedes SEDIMENT-009 only at the debug transport/scheduler layer. The rainbow fixture exposed that rolling grains had no vertical presentation coordinate, so category mass could appear many rows lower in one frame. It also exposed that SEDIMENT-009 kept adding 64x/128x synthetic debt while visible flow was deliberately throttled, making fluid look frozen at low CPU use. SEDIMENT-010 adds presentation-only rolling elevation, pauses the drive clock during explicit flow, drains invisible fluid-only relaxation cooperatively, and resumes preserved fast-forward debt only at quiescence. SEDIMENT-008 front and SEDIMENT-009 fluid thresholds remain frozen. See `notebook/work/SEDIMENT-010-TRANSPORT-CLOCK.md`.

SEDIMENT-011 supersedes SEDIMENT-010's **presentation barrier**, not its frozen physics. Human rainbow testing showed the barrier serialized the entire collapse behind one-row visual convergence and made rain appear to stop. SEDIMENT-011 keeps authoritative column mutations immediate, adds per-settled-grain presentation coordinates aligned with the CategoryId stacks, advances all transports concurrently, keeps one baseline live rain grain per wall-clock second during special playback, and drains truly latent fluidity even when ordinary falling dots exist. See `notebook/work/SEDIMENT-011-PARALLEL-TRANSPORT-CLOCK.md`.

SEDIMENT-012 attempted true per-grain Transport2D identities/paths, but owner rainbow evidence rejected the result: local paths remained visually mechanical because the front solver does not actually define a unique continuous grain trajectory. SEDIMENT-013 therefore abandons destination queues and exact visual grain identity. `oslo-vessel-front-flowviz` records only real adjacent front/moving-layer edge flux and renders a bounded, presentation-only category-colored tracer cloud advected by recent local flux, gravity, and current bed relief. Frozen front physics remains unchanged. See `notebook/work/SEDIMENT-013-OSLO-VESSEL-FLOWVIZ.md`.

SEDIMENT-014R3 is **MACHINE-GREEN / HUMAN-REJECTED FOR AGGREGATE TRANSPORT FIDELITY**. R1-R3 establish conservative shadow custody, delayed settlement, real adjacent transport accounting, fungible discharge reconciliation, zero false custody misses, and exact total/CategoryId conservation without changing SEDIMENT-008 physics. Owner rainbow evidence nevertheless shows that weighted parcels and aggregate category reconciliation lose too much spatial/timing information: downstream colors/progression still look reconstructed rather than transported. Do not extend the parcel model with R4.

SEDIMENT-015A is the current bounded experiment. `oslo-vessel-front-grains` keeps settled topology compact but assigns one ephemeral `MotionId`/visual carrier to each unit while it is physically rolling, visually pending settlement, or visually pending egress. Carriers may replay only already-observed physical adjacent edges; they never predict destinations or affect physics. Pending custody is aligned to exact authoritative stack depth and drains by ordered vector equality. 015A intentionally uses simple interpolation and is machine-gated; local separation/PBD-inspired micro-motion is reserved for SEDIMENT-015B after the causal unit foundation is native-green. See `notebook/work/SEDIMENT-015A-HYBRID-ACTIVE-GRAIN-TRANSPORT.md`.

`notebook/work/PLATEAU-001.md` is the durable roadmap for the remaining core-development arc. The owner has accepted the existing day-end snapshot behavior as baseline and does not want an artificial re-certification gate before continuing.

HISTORY-001B is now **COMPLETE / NATIVE-GREEN** at `ce9dd7281d3fb064302099e7cb274800c4f0ca9c`: formatter, strict Clippy, 238 tests, help smoke, targeted custom-range interaction proof, preset regression, provisional-live routing, and historical-sediment end-day selection all pass. The final delta over the authored candidate was rustfmt-only.

HISTORY-001C is now **COMPLETE / NATIVE-GREEN** at `09412b703cf41016f889d725ba235a7a1e63ae6a`: formatting, strict Clippy, 245 tests, focused editor/transaction proofs, isolated startup smoke, and validator profile-custody proof all pass. Its completed-Idle split transaction, active-preview validation, atomic daily-contribution replacement, rollback proof, and post-commit memory reload are retained as the safe foundation for generalized historical editing.

HISTORY-001D is now **COMPLETE / NATIVE-GREEN** at `bfaa8bf29f8019c25fe4f2ee8b1d60c554e5e988`: formatting, strict Clippy, 258 tests, 18 focused generalized transaction tests, Balance/report regression, isolated startup, and bubblewrap profile-custody proof all pass. Arbitrary `From < To <= now` assignment, gap insertion, collision confirmation, active-generation rebasing with protected live selection, daily-contribution reconciliation, rollback, and sediment/snapshot non-mutation are certified.

HISTORY-001E is now **COMPLETE / NATIVE-GREEN** at `d67c8e382708dbbf3f71bf2a67d7daa81b2e36b8`: formatting, strict Clippy, 263 tests, pure recolor and historical-sediment transaction proofs, HISTORY-001D and sediment regression, isolated startup/detach/restart, and bubblewrap profile-custody proof all pass. Retained source-category mass recolors deterministically in place; true historical gaps and cleared-away mass never fabricate current grains; authentic first-write day-end snapshots remain immutable.

SEDIMENT-002 is **COMPLETE / NATIVE-GREEN** at `4059e28df2ebf82bd31453ee208093eef57a4511`: formatting, strict Clippy, 280 tests, focused friction/biased-rain/recovery proofs, visible-basin and HISTORY-001E regression, isolated startup/detach/restart, and bubblewrap profile-custody proof all pass.

INTERACTION-002 is **COMPLETE / NATIVE-GREEN** at `b0f60eb3c6d76d1afee8d46737baab8ed220b01b`: formatting, strict Clippy, 283 tests, Settings/keymap and INTERACTION-001 regression, Balance/HISTORY and SEDIMENT-002 regression, isolated TUI startup, and bubblewrap profile-custody proof all pass. The former Atlas is now plain Settings; human action labels and Main / Navigation / Layer / Balance / Settings grouping are authoritative; the misplaced Main `t → Detach` fallback is retired; historical physical keys remain scoped to Balance while the command palette is the deliberate universal launcher.

The plateau cutover is **COMPLETE / REAL-PROFILE GREEN** from published main `3062e115de1bdf16985275ff1476ba22f213f744`: the new binary started on the real profile, v1/v2 sediment restored and persisted as SandState v3 with a valid ingress focus, Settings and Balance were visible, normal restart passed, and the pre-v3 backup was retained.

PLATEAU-001H / H1 — de-nozzle rain + contextual Balance footer — is **COMPLETE / PUBLISHED / REAL-PROFILE GREEN**. Runtime source `744b9f11a5341b0948178b932b23dd1d7e59662b` was published through main `d34e2eaae825c78b5754f8efa8b45b3f35e69bf1`, installed atomically, and restarted on the real profile. Daily use confirms that short-term ingress reads as rain rather than a visible nozzle.

The stale dirty adaptive-resize branch remains preserved as custody evidence and is superseded by the current-main
visible-basin, resize/restore, and atomic clear-all authority. No adaptive code was ported.

The required core path is now PLATEAU-001H product hardening. Pomodoro/RHYTHM-001 remains optional and must not block plateau.

PLATEAU-001H / H2 — metastable repose + local avalanches — is **COMPLETE / PUBLISHED / REAL-PROFILE GREEN** from native-green `f581de486a08547ea5fd74ef3ca2f2fb90e1eb34`, published main `71363c694e4c6f6c425f30378e081ef27cebd635`, and installed binary SHA256 `59f34237fab6348560141257c91cee6f6e4c4551a3915a5eef45fb4cc1ced9a`. Static supported relief 3 and dynamic active relief 1 replace the memoryless diagonal lottery; bottom-connected surface relief excludes airborne grains; radius-one local mobilization and one diagonal topple per gravity pass produce true buildup and bursty avalanches. Real-cadence validation corrected the earlier 1:1 overload false blocker: at normal 1000 ms ingress and 64 ms gravity cadence, 40x20 produced 449 events (median size 8, p95 82, quiet buildup 9) and 80x30 produced 658 events (median size 8, p95 20, quiet buildup 10), with 0% one-move events, conserved mass, no runaway, exact v4 restart continuation, all regressions green, 290 non-doc tests, and native gravity cost within the 2x target. The real profile has persisted SandState v4, normal restart and sqlite-doctor passed, and the matching H1 rollback binary plus pre-v4 backup remain retained.

PLATEAU-001H / H3 — isolated-spire cap — is **COMPLETE / PUBLISHED / REAL-PROFILE GREEN**. Its validated lineage was merged through main `f3590a7aeb69a4b88cef90862bb01eb7afd564ba` and installed as SHA256 `fc6f806ba174313b9e89a7aa9814cf6ccf9e76a4ff017c755775a92421dd0350`. H3 remains useful historical evidence, but subsequent daily use rejected two-dot one-column prominences as well, so its shape-specific rule is superseded by the H4 candidate.

PLATEAU-001H / H4 — contact-supported grain-causal avalanches + SandState v5 — is **COMPLETE / PUBLISHED / REAL-PROFILE GREEN**. Behavior remains native-green at `579f3e1b652a2d90efcfcef65e1910d199e464ba`; exact mobilized coordinates, v5 restart/continuation, hidden resize custody, deterministic v4 migration, malformed fail-closed validation, recovery, recolor, and SQLite schema invariance passed at `f00b628bd37c42a9b27b2abb4b73b1068c74f551`. PR #89 published main `67ffd84d3c5c924211ac9a14b52b5749fb07ed8b`, and the installed binary is SHA256 `b6f3af5247ce633b4c01c6232c1f1be057f7f9af562b6a5114f424b5f3559f93`. Real profile `95446134-3681-4390-84d7-8d900ebbb892` crossed v4→v5 successfully and passed a second v5→v5 restart plus sqlite-doctor; the pre-v5 backup and H3 rollback pair remain retained. Regional activity, `active_vertical`, static relief `3`, and isolated-spire heuristics remain retired.

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
