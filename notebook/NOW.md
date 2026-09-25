---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-09-25
authority: working
summary: "RECOVERY-003 is published and installed at main `2031edf`; the known v6 profile shape starts on a disposable copy and retains canonical topology and Classic state."
next: Verify the real profile's first startup on installed main writes canonical SandState v5, then resume SEDIMENT-007 from its bounded work record.
---

## RAIN-005 production cutover correction — 2026-09-10

**NATIVELY CERTIFIED / PUBLICATION PENDING:** the corrective branch closes the `ef46772d...` integration error by making the exact accepted C2R2 Classic engine the real normal-TUI authority. Native Rust 1.98.0 validation passed formatting, strict Clippy, 512 library + 24 integration tests, explicit `App.sand_engine`/`new_production`/spawn/update routing proof, H4-v5 zero-pre-tick topology/category drift with exact FIFO and mass preservation, deterministic hidden-state initialization, Classic restart/future-continuation equality, SQLite round-trip, malformed/fail-closed recovery gates, B2/B2R1/C1/C2R2 regressions, anti-nozzle gates, PERF-001 at 8.31x, release provenance, and a disposable copy of the live profile through startup/update/exit/reopen/doctor. Certified source head: `9c2e7b213316a74e312b1a6bf8c051404b2ae7b2`; release SHA256: `11feeca34e71b12e886e349314f9a28c094f73bcfe6800e22b006db5ff4a9080`. The live profile was backed up and not mutated. No morphology tuning is reopened. See `notebook/work/RAIN-005-PRODUCTION-CUTOVER.md`.

## RAIN-005 final morphology closure — 2026-09-09

**CLOSED / OWNER-ACCEPTED DEFAULT:** C2R2 `anchor-apex-latent`. The owner preferred it over C1R1 because it cleans the narrow 1–3-dot apex artifact without a perceptible loss of the large-scale relief, crests/valleys, lateral thickening/thinning, pinch-outs, or avalanche behavior that define the accepted morphology. Later G1/G2/G2R1 geography experiments did not supersede it: the strongest heritage-walk arm became an undesirable monomountain, while the final heritage-walk + boundary-avulsion hybrid recovered ~98.1% of C2R2 stratal span but only ~2.93% of the heritage arm's d8 relief gain. Morphology search is closed; treat C2R2 as the runtime default and move on. See `notebook/work/RAIN-005C2R2-HUMAN-AB-APEX-LATENT.md`.

## RAIN-005C2R2 bounded human A/B — 2026-09-08

C2R1 completed 256 runs and returned `NO_PIKE_FIX_PRESERVES_OWNER_MACRO_FLOOR`. The owner then clarified that the exact mathematical floor is stricter than the product criterion: a tiny thickness-CV loss is acceptable if the visual macro-topography is unchanged, and narrow pikes may themselves overstate that metric. R2 therefore performs no new search and promotes only the already-measured `anchor-apex-latent` near-miss to a runtime human A/B candidate. Full certification remains deferred until an owner visual preference exists. See `notebook/work/RAIN-005C2R2-HUMAN-AB-APEX-LATENT.md`.

## RAIN-005C2R1 surgical pike refinement — 2026-09-08

C2 completed 320 runs. Broad `shoulder-route` clearly suppressed the pike diagnostic but reduced thickness CV and pinch-out, so it is not eligible to replace the owner-accepted C1R1 baseline. R1 tests only anchor-specific shoulder routing, geometry-driven apex-latent anchor authority, and a strong-tail shoulder guard. The selection rule now makes the owner's full target set an exact hard floor rather than allowing a Pareto trade. C2 also exposed an obsolete ordinary unit-test assumption: C1R1 intentionally transports `(repose,memory)` tuples, so three-refresh memory follows the routed state rather than a fixed column coordinate. The test is corrected without changing runtime. See `notebook/work/RAIN-005C2R1-SURGICAL-PIKE-REFINEMENT.md`.

## RAIN-005C2 pike-artifact factorial — 2026-09-08

The owner accepted C1R1 `convexity-route` as the new preferred Classic morphology baseline after the 24-run promotion smoke and direct visual review: the gain is small but clearly visible in large-scale relief, crests/valleys, and lateral stratal variation. That review also exposed occasional narrow one-column pikes of roughly one to three dots. C2 keeps the accepted C1R1 runtime as its exact control and tests four test-only causal alternatives: delay only a newly routed anchor's fourth repose unit until later supported accretion, keep anchors stationary while routing the weaker states, defer routing until later supported accretion, and route toward shoulders/non-apex convex support rather than strict one-column apices. A new final-surface pike density/excess diagnostic is paired with the existing macrorelief and avalanche-safety metrics. The five 64-run arms execute concurrently on scoped worker threads. No runtime candidate, full 192-row certification, merge, or push is authorized by this unit. See `notebook/work/RAIN-005C2-PIKE-ARTIFACT-FACTORIAL.md`.

## RAIN-005C1R1 bounded human-smoke gate — 2026-09-08

The owner returned before the planned full 192-row promotion certification and correctly questioned whether the measured C1 gains (roughly low-single-digit coarse-relief improvements) will be visually meaningful. To avoid spending another long native pass on an implementation-equivalence question before human value is established, C1R1 now adds a test-only bounded promotion smoke. It selects every eighth A3 geometry from the frozen 192-row convexity-route reference (12 geometries spanning the exact geometry order) and runs both exact seeds, for 24 physical runs total. Every reported morphology and avalanche summary field must reproduce the C1 experimental winner at nine printed decimals. This does not weaken the eventual certification: if the owner visually accepts C1R1, the unchanged full 192-row promotion gate remains required before freezing or merging.

## RAIN-005C1R1 runtime convexity routing — 2026-09-08

RAIN-005C1 completed 960 physical runs and selected `convexity-route` as its only unique full human-eligible finalist. The arm preserves the exact local repose-state budget and RNG stream, but after an ordinary movement-triggered refresh it permutes the touched radius-one neighborhood so stronger `(repose, memory)` states occupy the more locally convex supported columns. Full 192-run evidence improved all five coarse relief/curvature objectives and also raised thickness CV and pinch-out while remaining avalanche-safe. C1R1 promotes only that exact mechanism; no anchor probability, persistence interval, radius, threshold, or avalanche law changes. Promotion must reproduce the frozen 192 selected samples before human review. See `notebook/work/RAIN-005C1R1-RUNTIME-CONVEXITY-ROUTING.md`.


## RAIN-005C1 macrorelief stability factorial — 2026-09-08

RAIN-005B2R1 passed its full 192-run machine gate and is the new owner baseline, but human review preserved a narrower target from the exact historical H4 installed binary (`b6f3af...f3559f93`): large-scale relief, crests, valleys, and laterally thickening/pinching strata. The Notebook already proves that installed SHA belongs to published H4 v5 (`67ffd84...`). C1 freezes R1 runtime and tests only local stability semantics. A factual correction matters: current repose refresh is already movement-coupled, so “persist until movement” would not be a new mechanism. The unattended suite instead tests anchor ablation, strong-tail remap, explicit high-repose locks, and two budget-neutral structural-routing laws (height and local convexity), including two interactions. Stage 1 is 64 geometries x 1 seed x 9 arms; it automatically selects at most two avalanche-safe Pareto finalists and immediately runs full 192-row evidence for those finalists plus R1 control. Primary metrics now directly target coarse relief/curvature, thickness CV, pinch-out and continuity. See `notebook/work/RAIN-005C1-MACRORELIEF-STABILITY-FACTORIAL.md`.


## RAIN-005B2R1 runtime boundary avulsion — 2026-09-08

RAIN-005B2D2 completed its 192-run three-arm factorial and selected `boundary-avulsion-golden-small` under the predeclared minimum-change rule. At the original B2 `1/phi^2` authority, the selected arm passed all subset gates with CV `+0.001302657`, corr `-0.315065940`, TV `+0.035007331`, shift `+0.046770031`, span `+0.082278422`, and shift-vs-log-width `-0.026201417`. R1 promotes only that actually-entered category-boundary focus relocation to WanderingFocus runtime; B2 scheduling, authority, kernel, meander-between-boundaries and Classic settlement remain unchanged. Full 192-run A3 machine validation is required before human review, including exact 64-row reproduction of the D2-selected arm. See `notebook/work/RAIN-005B2R1-RUNTIME-BOUNDARY-AVULSION.md`.



## RAIN-005B2D2 boundary-avulsion factorial — 2026-09-07

RAIN-005B2D1 completed 320 physical morphology runs and classified `NO_DYADIC_SUBSET_FRONTIER`. Increasing focused authority strongly raised thickness CV, but adjacent-stratum correlation became progressively positive; at the smallest increase, CV and shift were already positive while corr and span failed. D2 therefore does not interpolate another probability. Production remains exact B2 while a test-only 3-arm factorial asks whether relocating only the invisible focus at an actually-entered category boundary can decorrelate successive strata without sacrificing within-layer concentration. It tests B2 control, golden-small + boundary avulsion, and the smallest D1 authority increase + boundary avulsion over the first 64 A3 geometries / seed-slot 1. See `notebook/work/RAIN-005B2D2-BOUNDARY-AVULSION-FACTORIAL.md`.


## RAIN-005B2D1 focus-authority sweep — 2026-09-07

RAIN-005B2 blocked only on tiny negative median shift/span while its CV/corr/TV medians passed and its low-discrepancy scheduler strongly suppressed short focused bursts. The owner explicitly reopened the RAIN-004 `1/phi^2` focus share as an experimental variable rather than a frozen golden constant. B2D1 leaves production runtime at exact B2 and sweeps only test-time focus authority along a dyadic bracket from `p0` toward the physical `p=1` limit. The first 64 A3 geometries/seed-slot 1 are run at five levels (320 physical morphology runs), with exact nine-decimal reproduction of a 64-row native B2 control fixture. If multiple levels pass CV/corr/TV/shift/span plus width-dependence gates, only the smallest authority increase is eligible for a later full candidate. See `notebook/work/RAIN-005B2D1-FOCUS-AUTHORITY-SWEEP.md`.


## RAIN-005B2 golden low-discrepancy ingress — 2026-09-07

RAIN-005B1/R1/R2/R3 all failed the unchanged thickness-CV Pareto floor. D1 ruled out terrain/edge steering, and D2 found no derived mobility subset frontier: the full `L` jump retains lateral gains while losing CV, whereas `E*L` and `E*W+` recover CV only by losing shift/span and worsening width dependence beyond RAIN-004. B1 is therefore exhausted rather than interpolated. B2 branches from exact RAIN-005A3/RAIN-004 runtime and preserves the marginal `1/phi^2` bias, focused two-candidate kernel and RAIN-004 meander. It changes only serial focus-authority timing: a seed-phased 53-bit low-discrepancy rotation guarantees less than one focused-ingress count error in every contiguous eligible window, so an `N_airborne`-sized visible interval cannot receive an IID burst of focused authority. See `notebook/work/RAIN-005B2-GOLDEN-DISCREPANCY-INGRESS.md`.


## RAIN-005A3 extended geometry ensemble — 2026-09-07

RAIN-005A2 passed machine validation and showed useful scale stability, but the owner correctly noticed that its 16 morphology cases were concentrated in small/medium terminal sizes. RAIN-005A3 remains runtime-exact to RAIN-004 and expands machine evidence instead of asking for more manual review: 96 logarithmically sampled area/aspect geometries x two independent seeds (192 physical morphology runs), explicit geometry-vs-seed dependence measurements, and a 4096-case analytical nozzle sweep over much larger arbitrary dot geometry. One hour or more of native test time is acceptable; case count/range must not be reduced for convenience. See `notebook/work/RAIN-005A3-EXTENDED-GEOMETRY-ENSEMBLE.md`.


## RAIN-005A2 morphology ensemble — 2026-09-07

RAIN-005A machine validation passed and preserved RAIN-004 exactly, but the owner correctly noted that one later stochastic capture looked more thickness-homogeneous than the strongest prior RAIN-004 examples. RAIN-005A2 therefore makes no runtime change: it adds ignored native probes that characterize RAIN-004 across a low-discrepancy continuum of viewport shapes/seeds and an analytical arbitrary-geometry nozzle sweep. The goal is to establish a family-level statistical floor before RAIN-005B, not to optimize for the owner's current window or freeze one screenshot. See `notebook/work/RAIN-005A2-MORPHOLOGY-ENSEMBLE.md`.


## RAIN-005A derived morphology diagnostics — 2026-09-07

RAIN-004 passed machine validation and the owner selected it as the best-so-far morphology reference, while explicitly rejecting any interpretation that its current golden bias or meander timings are therefore perfect/final. RAIN-005A is a measurement-only pass: add `testingcheats classic rainmetrics` to quantify canonical-1x visible airborne population, analytical nozzleness of the current broad rain kernel, equivalent-layer and one-dot excess-mound scales, surface relief, and adjacent-category thickness-profile heterogeneity. Runtime morphology remains exactly RAIN-004. See `notebook/work/RAIN-005A-DERIVED-MORPHOLOGY-DIAGNOSTICS.md`.

## RAIN-004 machine-green / owner reference — 2026-09-07

RAIN-004 passed 509 tests with no failures, retained PERF capacity (median 1457.78x), and the owner judged its morphology the strongest result so far. It is the RAIN-005 comparison baseline, **not** a perfection claim or a freeze of its magic constants. Future candidates should target equal-or-greater heterogeneity while retaining broad/nozzle-free falling rain.

## RAIN-004 authored candidate — 2026-09-07

RAIN-003's correlated meander/category rephase direction survived owner morphology review, but the owner wants the focus to use the complete active width and wants the broad focus-biased share strengthened from 25% to the small golden-ratio part (`1/phi^2`, ~38.2%). RAIN-004 makes only those two morphology changes while retaining free-site ingress, immediate fallspeed debt ownership, PERF-001, frozen `momentum-grounded-contact`, and the no-nozzle broad focus sampler. See `notebook/work/RAIN-004-FULL-WIDTH-GOLDEN-BIAS.md`.


## RAIN-003 authored candidate — 2026-09-07

RAIN-002 is machine-green but its owner morphology gate rejected the one-hour dwell + lowest-of-three avulsion result as too homogeneous/bland. RAIN-003 retains the accepted 75/25 broad/nozzle-free envelope, free-site ingress, immediate fallspeed debt semantics, PERF-001 and frozen `momentum-grounded-contact`, but replaces focus morphology with a continuous aperiodic correlated meander (~4 h hypothetical traverse), weak broad left/right terrain steering, category-boundary rephase without teleport, and one further recursive-golden focus-padding reduction to ~7.3% per side. See `notebook/work/RAIN-003-STRATIGRAPHIC-CORRELATED-MEANDER.md`.

## RAIN-002 machine-green / morphology rejected — 2026-09-07

RAIN-002 passed all machine gates (483 tests, PERF median 1538.99x) and its free-site ingress plus 128x-to-1x debt ownership remain accepted. Human review rejected only the dwell/lowest-of-three focus morphology because successive strata looked too homogeneous. RAIN-003 supersedes that morphology while preserving those independent fixes.

## RAIN-002 authored candidate — 2026-09-07

PERF-001 is machine-green and human-approved for interactive 128x testing. RAIN-001's 75/25 airborne envelope also passed the owner's anti-nozzle gate, but long-form review found one dominant focus lump, weakly visible compensation, nearest-free top-row relocation, and stale 128x debt after returning to 1x. RAIN-002 retains 75/25 and frozen `momentum-grounded-contact`, replacing only focus timing with one-hour dwell + three-candidate compensational avulsion + bounded local wander, sampling directly from free ingress sites, and splitting multiplier debt from explicit Advance debt so fallspeed changes are immediate. See `notebook/work/RAIN-002-AVULSION-FREE-INGRESS.md`.

## PERF-001 native/human-green — 2026-09-07

PERF-001 is machine-green and human-approved. Exact optimized/reference gates passed, the dev-profile Hybrid capacity probe reached a median 1691.83x, and the owner confirmed interactive `fallspeed 128x` is now fast enough for morphology review. Its sparse occupancy/grounded indexes, exact RNG jump, O(1) metadata, dev optimization, and bounded accelerated budgets are the retained performance baseline. See `notebook/work/PERF-001-CLASSIC-EXACT-HIGH-SPEED.md`.

## RAIN-001 superseded by RAIN-002 — 2026-09-07

Classic settlement physics is frozen at the owner-preferred `momentum-grounded-contact` baseline. RAIN-001 changes only the debug `hybrid` ingress comparison: restore the previously attractive 75/25 broad-rain envelope, keep the accepted slow cross-corridor focus walk, and choose each new long-lived waypoint by a two-candidate weak preference for lower broad grounded terrain. Uniform `classic` remains the control. No global terrain scan per grain, category-driven focus reset, nozzle emitter, persistence change, catch-up change, or Advance change is allowed. See `notebook/work/RAIN-001-COMPENSATIONAL-FOCUS.md`.

## APPEARANCE-001 human-smoked — 2026-09-06

Manual theme authority is machine-green and the disposable-profile btop-inspired theme worked live, including theme selection and arbitrary theme palette behavior. `rgb-luma-safe` remains the accepted Classic blend baseline. Terminal auto light/dark and strict/soft/safe contrast adaptation remain deferred until after the current rain/performance/advance sequence.

# NOW — Strata

## Current phase

**RECOVERY-003** is the active compatibility fix. The installed pre-main v0.7.7 prototype wrote SandState v6 with optional grain-time provenance; current main is v5 and must open that known input without losing canonical topology or Classic continuation.

Native compatibility proof passes: v6 payloads containing per-grain timestamps and pending-run provenance restore to exact v5 topology and Classic state; bounded recovery normalizes known v6 inputs to v5; unknown future versions remain fail-closed. Formatting, strict Clippy, all tests, and CLI help smoke pass. A verified copy of the affected SQLite profile is retained outside the repository.

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

The owner clarified daily visual memory: the latest canonical `SandState` saved during an operational day must remain
that day's historical photo if Strata is not running at the 06:00 cutoff. Autosave photos advance until the live cutoff
promotes the exact boundary state to an immutable final checkpoint. `DailyContribution` remains ledger-derived
accounting evidence; `DerivedPreview` is only a visual fallback when no canonical photo was saved for that day.
HISTORY-002 is certified: formatter, strict Clippy, full tests (515 unit + 24 integration), CLI help smoke, latest-photo
replacement, day-boundary finalization, and SQLite rollback fault coverage passed.

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
