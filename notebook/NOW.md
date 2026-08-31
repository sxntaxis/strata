---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-30
authority: working
summary: H5 resize boundary release and H9 live sediment cadence are native-certified; H6 and H8 propagation experiments are rejected; H7 confirms latent supported rupture opportunity. A clean publication candidate now composes only H5 + H9 production changes while retaining the experimental results as documentation.
next: Certify the clean H5+H9 publication composition against the separately native-certified source branches, then perform one short owner visual pass: resize growth must release former walls, and ordinary live H4 motion must be visibly less batched. Do not revive H6/H8 or add avalanche physics before that judgment. Ingress-focus padding remains the next separate deposition unit afterward.
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

PLATEAU-001H / H1 — de-nozzle rain + contextual Balance footer — is **COMPLETE / PUBLISHED / REAL-PROFILE GREEN**. Runtime source `744b9f11a5341b0948178b932b23dd1d7e59662b` was published through main `d34e2eaae825c78b5754f8efa8b45b3f35e69bf1`, installed atomically, and restarted on the real profile. Daily use confirms that short-term ingress reads as rain rather than a visible nozzle.

The stale dirty adaptive-resize branch remains preserved as custody evidence and is superseded by the current-main
visible-basin, resize/restore, and atomic clear-all authority. No adaptive code was ported.

The required core path is now PLATEAU-001H product hardening. Pomodoro/RHYTHM-001 remains optional and must not block plateau.

PLATEAU-001H / H2 — metastable repose + local avalanches — is **COMPLETE / PUBLISHED / REAL-PROFILE GREEN** from native-green `f581de486a08547ea5fd74ef3ca2f2fb90e1eb34`, published main `71363c694e4c6f6c425f30378e081ef27cebd635`, and installed binary SHA256 `59f34237fab6348560141257c91cee6f6e4c4551a3915a5eef45fb4cc1ced9a`. Static supported relief 3 and dynamic active relief 1 replace the memoryless diagonal lottery; bottom-connected surface relief excludes airborne grains; radius-one local mobilization and one diagonal topple per gravity pass produce true buildup and bursty avalanches. Real-cadence validation corrected the earlier 1:1 overload false blocker: at normal 1000 ms ingress and 64 ms gravity cadence, 40x20 produced 449 events (median size 8, p95 82, quiet buildup 9) and 80x30 produced 658 events (median size 8, p95 20, quiet buildup 10), with 0% one-move events, conserved mass, no runaway, exact v4 restart continuation, all regressions green, 290 non-doc tests, and native gravity cost within the 2x target. The real profile has persisted SandState v4, normal restart and sqlite-doctor passed, and the matching H1 rollback binary plus pre-v4 backup remain retained.

PLATEAU-001H / H3 — isolated-spire cap — is **COMPLETE / PUBLISHED / REAL-PROFILE GREEN**. Its validated lineage was merged through main `f3590a7aeb69a4b88cef90862bb01eb7afd564ba` and installed as SHA256 `fc6f806ba174313b9e89a7aa9814cf6ccf9e76a4ff017c755775a92421dd0350`. H3 remains useful historical evidence, but subsequent daily use rejected two-dot one-column prominences as well, so its shape-specific rule is superseded by the H4 candidate.

PLATEAU-001H / H4 — contact-supported grain-causal avalanches + SandState v5 — is **COMPLETE / PUBLISHED / REAL-PROFILE GREEN**. Behavior remains native-green at `579f3e1b652a2d90efcfcef65e1910d199e464ba`; exact mobilized coordinates, v5 restart/continuation, hidden resize custody, deterministic v4 migration, malformed fail-closed validation, recovery, recolor, and SQLite schema invariance passed at `f00b628bd37c42a9b27b2abb4b73b1068c74f551`. PR #89 published main `67ffd84d3c5c924211ac9a14b52b5749fb07ed8b`, and the installed binary is SHA256 `b6f3af5247ce633b4c01c6232c1f1be057f7f9af562b6a5114f424b5f3559f93`. Real profile `95446134-3681-4390-84d7-8d900ebbb892` crossed v4→v5 successfully and passed a second v5→v5 restart plus sqlite-doctor; the pre-v5 backup and H3 rollback pair remain retained. Regional activity, `active_vertical`, static relief `3`, and isolated-spire heuristics remain retired.

PLATEAU-001H / H5 — resize boundary release — is **NATIVE-CERTIFIED / PUBLICATION PENDING** at `3920ab3899f3249569f2dfb8c990e6389cb6fc47` / tree `8f3f69f7baeb15cd60423e5663b36647efd4a68b`, based on current main `b5afc619bace6d9d98ec14ccd065add3689e2e38`. Focused H5 tests, 22 focused H4/organic tests plus one ignored behavior bench, 272 full library tests plus one ignored bench, 23 integration tests, formatting, strict Clippy, help smoke, and diff hygiene all passed; the only fallout was rustfmt-only. No merge to main has occurred.

PLATEAU-001H / H6 — causal slip-front threshold-cross propagation — is **NATIVE-BENCHED / REJECTED FOR PRODUCTION** at final source `f4f9683586f362342b259f6c83170a6e02ee3bf4` / tree `aebfef878ec28a59a9e182f64e01b412d25fac1f`. The test-only rule did create front activity, but it did not materially widen ordinary avalanches: column p95 remained 3 in every comparable lane; 40x20 / 4,000 multi-column episodes were unchanged `133 -> 133`; and 80x30 / 10,000 fell `407 -> 348`. Topology guardrails also moved the wrong way, including 40x20 / 4,000 roughness `1.0506 -> 1.8861` and 80x30 / 10,000 variance `44.1750 -> 56.3875`. The 40x20 / 10,000 shape endpoint is excluded from topology judgment because the 6,400-dot basin saturated with 3,600 pending grains. H6 stays test-only evidence and no production slip-front behavior is accepted.

PLATEAU-001H / H7 — rupture-opportunity diagnostics — is **NATIVE-CERTIFIED PASSIVE EVIDENCE** at final source `46a3621ccbcb174921d8f015c834838fa31c6f9b` / tree `e132f614485013a2fd12eb028af84896b0e5a10e`. H6 4,000-ingress outputs remained exactly identical apart from timing. H7 found no immediately-uphill H4 support-loss cases in any lane, while H6 threshold crossings occurred about 43-57 per 1,000 topples and already-dynamic supported opportunities about 43-60 per 1,000. The unsaturated 80x30 / 10,000 lane had 125 threshold crossings versus 154 latent already-dynamic opportunities: relief 2 = 100, relief 3 = 32, relief >=4 = 22. The result supports one more narrowly bounded face-transfer experiment but not regional activation.

PLATEAU-001H / H8 — exact-path relief-2 rupture token — is **NATIVE-BENCHED / REJECTED FOR PRODUCTION** at final source `8dc98d4d93c4742db322fb34b07c3749af0bc795` / tree `f3cebc0c64f3a46c7d1baec243c26af0d0849997`. The token executed and occasionally chained, but ordinary avalanche width did not improve: column p95 remained 3 in every lane; 40x20 / 4,000 multi-column episodes rose only `133 -> 140`, 80x30 / 4,000 fell `130 -> 117`, and unsaturated 80x30 / 10,000 fell `407 -> 322`. The long 80x30 lane also reduced move p95 `5 -> 4`. Topology movement was inconsistent rather than safely neutral: 40x20 / 4,000 roughness rose `1.0506 -> 1.2532` and variance `53.1000 -> 62.8000`, while 80x30 lanes became somewhat smoother but more plateau-heavy. Token chains were rare (3, 1, 3, and 4 across the four lanes), so the mechanism did not create a broad causal front. H8 remains test-only evidence and must not be promoted.

PLATEAU-001H / H9 — live sediment presentation cadence — is **NATIVE-CERTIFIED / PUBLICATION COMPOSITION PENDING**. H6 and H8 show that adding causal propagation changes equilibrium without materially increasing ordinary cascade width. Code-history review found a separate presentation regression: before detached catch-up (pre-v0.7.5), ordinary live runtime called `SandEngine::update()` at the 32 ms physics cadence and marked a redraw; detached catch-up later routed all positive live backlog through a 120 ms cadence bucket. H4 gravity runs every second engine update, so its physical move cadence is 64 ms while the target render cadence is about 41 ms. The 120 ms live bucket can therefore execute one or two H4 gravity moves before exposing another frame, which is especially damaging now that H4 structural episodes are typically only one to five moves.

H9 separates **live** from **catch-up** without changing H4, ingress, RNG, persistence, or event ordering. Backlog at or below the existing 120 ms visibility threshold advances immediately to the current UTC target using the existing spawn/physics accumulators; only backlog above that threshold uses accelerated visual catch-up, and backlog above eight seconds still uses bounded settlement. `advance_simulation_by` now reports whether a spawn/physics event occurred so normal live time requests redraw only at simulation events. Because the same accumulators own due-event ordering, finer live slicing changes presentation cadence, not the canonical state produced for a given simulated interval. The scheduler regression asserts that 32 ms physics and the exact 120 ms edge stay on the live path, 121 ms enters visual catch-up, >8 seconds remains bounded, and the 24 fps render interval remains shorter than the 64 ms H4 gravity interval.

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

Certify the clean publication composition containing only H5 boundary release plus H9 live cadence production changes. Prove equivalence to the already native-certified H5/H9 source diffs, absence of H6/H8 experimental runtime/test machinery, full checks, and isolated runtime startup. Then use one short owner visual pass to judge resize release and avalanche perceptibility before merging to main. Ingress-focus padding remains the next separate deposition change.
