---
id: PLATEAU-001
kind: work
state: active
created: 2026-08-27
updated: 2026-08-28
authority: working
summary: Final core-development roadmap: historical time editing, more organic sediment formation, interaction/menu convergence, then sustained product hardening; Pomodoro remains an optional post-plateau experiment.
---

# PLATEAU-001 — Road to a stable Strata core

## Why this exists

Strata is approaching a product plateau: the continuous ledger, SQLite authority, recovery, live sediment,
visible-basin behavior, direct interaction, and authentic day-end visual memory are already usable in daily
practice. The remaining work should close concrete product gaps rather than reopen settled architecture.

This record is the durable orientation point for agents working after the August 2026 sediment/snapshot arc.
Read `AGENTS.md`, `notebook/NOW.md`, this file, then the authority document named by the active unit below.

## Owner decisions captured on 2026-08-27

- The implemented day-end snapshot behavior is accepted as baseline product behavior. Do not create a separate
  certification program merely to re-prove it unless a later change puts it at risk.
- The historical/report surface is renamed **Karma → Balance**. Default main-view key becomes `b`.
- The current **Atlas** name is rejected as overnamed. Its eventual user-facing vocabulary should be ordinary
  **Settings** (or an equally plain final label chosen during INTERACTION-002), because configuration also serves
  as discoverable guidance.
- Arbitrary **From → To** reporting is required in the TUI. The CLI/domain already contain much of the underlying
  range capability; do not build a second reporting engine.
- Retroactive logging is required: a user must be able to record an activity after the interval has passed.
- Historical correction must conserve chronology. A missed activity normally reclassifies a past interval rather
  than inserting overlapping time that double-counts duration.
- Retroactive correction should also be able to update the current canonical sediment composition. Preferred
  visual semantics are **recolor equivalent mass in place**, preserving topology, rather than replaying old physics.
  This is a nice-to-have subunit, not a prerequisite for basic historical editing.
- Already-captured authentic day-end visual snapshots remain governed by current immutable-evidence authority for
  now. Whether a later ledger correction should rewrite those historical photographs is an explicit unresolved
  product question; agents must not silently choose either behavior.
- Sediment should look more organically random without becoming a terrain generator. Improve local physics and
  ingress correlation; preserve emergent formation.
- Pomodoro/focus rhythm is interesting but must not block core plateau. If developed, it should overlay ordinary
  activity tracking rather than create a competing session authority.

## Program order

### HISTORY-001 — Balance and historical editing

Goal: make arbitrary past time first-class and editable without violating the continuous ledger.

#### HISTORY-001A — Balance vocabulary and report-window seam

Status: **COMPLETE / NATIVE-GREEN**.

Deliverables:

1. Rename the historical/report product vocabulary from Karma to Balance across runtime-visible UI, direct command,
   action/config names, report types, and current authority text.
2. Move the default Balance opener from `k` to `b`.
3. Introduce one explicit report-window value (`start`, `end`, label) in the domain.
4. Make preset Day/Week/Month reporting resolve through that window seam, so custom From/To can reuse identical
   report/log/live-session logic rather than branching into a second implementation.
5. Preserve existing report semantics and historical sediment selection.

Exit: current preset behavior is unchanged except vocabulary/key, and the domain has a validated arbitrary-window
entrypoint suitable for the TUI.

#### HISTORY-001B — From/To in Balance

Status: **COMPLETE / NATIVE-GREEN** at `ce9dd7281d3fb064302099e7cb274800c4f0ca9c`.

Interaction decision: `range` is a fourth Balance mode reached by the configurable `balance_range` action (default `r`). It edits explicit `From` and `To` ISO dates inline, commits only after validation, and then shifts as a whole window by its own span; it does not overload preset offsets. Native formatter, strict Clippy, 238 tests, smoke, and targeted TUI range proof pass; validation changed only rustfmt output.

Deliverables:

- Add a visible custom range choice beside Day / Week / Month.
- Provide explicit From and To date editing with validation (`from <= to`).
- Keep preset navigation convenient; custom range must not inherit nonsensical preset-offset semantics.
- Drive summary rows, detail logs, live provisional time, and historical snapshot selection from the same explicit
  report window.
- Converge visible TUI behavior with existing CLI range reporting.

#### HISTORY-001C — Retroactive missed activity

Status: **COMPLETE / NATIVE-GREEN** at `09412b703cf41016f889d725ba235a7a1e63ae6a`.

Interaction decision: from a persisted Idle detail row in Balance, configurable `balance_log_missed` (default `l`) opens an inline Layer / From / To editor. This unit corrects completed Idle history only; active-Idle backdating and non-Idle reclassification are intentionally not smuggled into the safe first transaction.

Persistence decision: one SQLite `IMMEDIATE` transaction validates the selected canonical Idle source, splits it into up to Idle-before / Activity / Idle-after, marks replacement provenance as `tui-history-correction`, and regenerates every affected `daily-contribution` artifact before commit. If an affected day also contains the current active generation, a stable-identity-qualified monotonic preview is validated against `active_session` and included atomically so historical correction cannot erase today's live mass. The source-start fragment retains the original session/stable identity; inserted fragments receive deterministic split identities. Whole-second conservation uses the source session's existing sub-second lattice and retained boundary provenance.

Deliverables:

- Add an explicit historical operation for recording a missed activity interval.
- Default safe case: reclassify an interval currently owned by Idle.
- Split surrounding canonical sessions as needed while conserving every second.
- Refuse or explicitly escalate overlaps with already-classified non-Idle time; never silently double-count.
- Persist transactionally and reconcile every affected operational day.
- Preserve provenance sufficient to distinguish corrected history where useful without inventing per-grain time
  provenance.

#### HISTORY-001D — Arbitrary retroactive activity

Status: **COMPLETE / NATIVE-GREEN** at `bfaa8bf29f8019c25fe4f2ee8b1d60c554e5e988`.

Product decision: the user states **From X to Y I did Z**. The operation is not scoped to a selected existing session and may cross zero, one, or many canonical rows. Existing session boundaries are implementation detail, not interaction authority.

Accepted semantics:

- `From < To <= now`; historical editing can never create future time.
- Idle behaves as transparent background for collision policy.
- Existing time already classified to the requested layer is non-conflicting.
- Any intersecting explicit different layer produces one visible collision preview and requires confirmation before replacement.
- Confirmation applies only to the exact observed collision plan; changed authority must be previewed again.
- The current selected layer/description is protected. Retroactive correction may split/rebase its historical interval, but it never changes what the user is doing now. Changing the current activity remains an explicit live switch/stop action.
- If the requested layer is the current selected layer and the assignment reaches backward into its start, the active start may move backward when chronology becomes continuously that layer. If a different historical layer is written through part of the active interval, the selected live activity resumes after the corrected interval.
- No future timestamp, overlapping canonical double-count, or silent overwrite of a different explicit activity is permitted.
- SQLite chronology, active-generation/checkpoint authority, daily contributions, and the in-memory projection publish coherently.
- Current sand and authentic historical day-end snapshots remain unchanged in HISTORY-001D; visual recolor remains HISTORY-001E.

Deliverables:

- Generalize `Log missed activity` into a Balance-wide `Log activity…` operation independent of selected Idle rows.
- Accept arbitrary historical From/To boundaries up to the current snapshot time and an existing target layer.
- Detect all intersecting explicit different-layer collisions across completed and current history.
- Require explicit confirmation for collisions, then atomically rewrite the interval while conserving canonical whole seconds.
- Preserve the current selected layer/description even when the rewrite intersects the active generation.
- Reconcile every affected operational day and reload the in-memory ledger without restart.

#### HISTORY-001E — Retained current-sediment recolor after correction (nice to have)

Status: **COMPLETE / NATIVE-GREEN** at `d67c8e382708dbbf3f71bf2a67d7daa81b2e36b8`.

Product decision: current canonical sediment is retained visual mass, not a complete replayable ledger. A historical assignment can fill a true gap, and prior full/category clears can remove sediment that once corresponded to corrected chronology. HISTORY-001E therefore reconciles only source-category mass that is still retained in the current canonical `SandState`.

Deliverables:

- Derive category-transfer counts from canonical seconds that changed from one existing category to the requested category; true-gap seconds have no source sediment transfer.
- Deterministically recolor up to the retained amount of each source category into the requested category, preferring placed grains before pending mass.
- Preserve grain coordinates/topology, pending order/count, total mass, frame/sweep/RNG metadata, and canonical canvas dimensions.
- If a prior clear means less source-category mass remains than the corrected duration, recolor only what remains; do not fabricate grains or steal unrelated categories to force the pile to equal the ledger.
- Treat the chosen grains as deterministic category-composition reconciliation only; without per-grain temporal provenance, do not claim they are the exact physical grains emitted by the corrected interval.
- Publish history, affected daily contributions, current canonical sediment, and runtime checkpoint coherently in the same SQLite transaction.
- Return/install the exact persisted resulting `SandState` in memory after commit.
- Do not add timestamps/session IDs to every grain solely for this feature.
- Authentic first-write day-end snapshots remain immutable. `DerivedPreview` and `DailyContribution` continue to follow corrected ledger truth without rewriting authentic photographs.


### SEDIMENT-002 — Organic formation

Status: **COMPLETE / NATIVE-GREEN** at `4059e28df2ebf82bd31453ee208093eef57a4511`.

Goal: make emergent shapes less white-noise-like while preserving ordinary falling-sand semantics and all certified mass/topology/recovery invariants.

Accepted implementation:

- Down remains unconditional. When down is blocked and at least one diagonal is open, apply a memoryless stochastic-friction gate: initially one-quarter slide and three-quarters temporary hold. After a permitted slide, take the sole open diagonal or randomize left/right when both are open. No-route grains stay put without a friction/lateral choice.
- Replace independent uniform top-edge ingress with rain influenced by one slowly wandering persisted focus. SEDIMENT-002 established the persisted focus and nearest-free visible fallback; PLATEAU-001H H1 later weakens the per-grain focus effect so short-term fall remains broad rain while long-run accumulation stays biased.
- The ingress focus is canonical dot-grid state, constrained to the active visible basin when used and shifted with the existing horizontal-center canvas expansion so growth does not teleport the favored rain region relative to retained topology.
- Persist the focus in `SandState` v3 beside the existing RNG state. v1 legacy pending vectors and v2 compressed-pending states remain readable and migrate to v3 with no invented focus; the existing RNG state deterministically seeds the first post-migration ingress.
- Full clear removes all mass and resets the ingress focus while retaining the RNG stream; category-specific clear preserves the formation focus.
- Keep ingress inside the visible physics basin and preserve shrink/freeze/re-expand semantics, pending FIFO mass, exact category identity, bounded recovery, and immutable historical snapshots.
- Do not add mountain generators, terrain post-processing, momentum fields, or user-facing randomness knobs in this unit.

Native proof must cover straight-down priority, friction hold and later slide for sole/both-diagonal cases, left/right choice after a permitted both-open slide, broad-but-biased rain sampling with slow focus motion and whole-width outliers, nearest-free fallback, blocked-ingress mass conservation, focus shift/clamp across resize, snapshot/restart continuity, v1/v2 compatibility, bounded recovery continuity, and all visible-basin/clear/history regressions.

### INTERACTION-002 — Menus, Settings, and vocabulary convergence

Status: **COMPLETE / NATIVE-GREEN** at `b0f60eb3c6d76d1afee8d46737baab8ed220b01b`.

Native evidence: formatting, strict Clippy, 283 tests, Settings/keymap-focused proof, INTERACTION-001 regression, Balance/HISTORY and SEDIMENT-002 regression, isolated TUI startup, and bubblewrap profile-custody proof all pass. The final native fallout was one coherent bounded follow-up commit and changed no architecture semantics.

Goal: polish action placement after historical operations exist, rather than polishing an incomplete menu graph.

Expected ownership:

- Main view: what is happening now.
- Layer: current classification, tags, layer management.
- Balance: past time, reports, inspection, historical correction.
- Command palette: universal deliberate launcher/direct commands.
- Settings: configuration, bindings, and discoverability/guide material.

Deliverables:

- Rename Atlas to a plain Settings-facing vocabulary and remove pretentious/internal naming where practical.
- Inventory every reachable action and contextual alias.
- Remove awkward duplication and misplaced actions.
- Revisit key bindings after Balance/custom-range/historical-edit actions exist.
- Preserve configured Bound / Unbound / Disabled truth and mandatory `Ctrl-C` policy.
- Keep command palette, visible menus, settings, and runtime routing in agreement.

Accepted implementation:

- The former Atlas becomes **Settings** everywhere current users see or current source names the surface. Existing Atlas config spellings remain parser-only aliases so the current real profile is not broken by the vocabulary cutover.
- Settings keeps the established F1 / `?` defaults; a plain-letter Settings key is deliberately avoided because Layer owns ordinary text input.
- Settings rows use human action names rather than config identifiers and are grouped as Main / Navigation / Layer / Balance / Settings. Bound / Unbound / Disabled and mandatory Ctrl-C remain exactly the underlying keymap truth.
- Default contextual routing is reduced to Main Confirm→Layer when Layer is otherwise unbound, Main Cancel→Idle when Idle switch is otherwise unbound, and Balance Detach→Day. The old conditional Main Balance-day→Detach route is retired.
- Balance physical keys `t/w/m/r/l` do not act as hidden Main shortcuts. The command palette remains the explicit universal launcher for Balance periods, custom range, and Log past activity.
- No new menu framework or Settings tab architecture is introduced in this unit; the existing surface is converged rather than replaced.
- First real-profile hardening evidence after cutover: the persisted sediment focus is still too visually legible as a nozzle. Short-term fall should look like broad rain; the focus should become apparent only through long-term accumulation.
- The same real-profile pass found Balance footer overload. A bare `l` is not self-explanatory; current-day `live sediment` status is noise; summary/detail/editor controls should be contextual and the retroactive action should read **Log past activity…**.

### PLATEAU-001H — Product hardening

Goal: stop feature-seeking and use real daily friction as the admission criterion for changes.

Hunt:

- awkward interaction;
- resize/sediment edge cases;
- misleading labels;
- historical-edit overlap and boundary bugs;
- persistence/restart surprises;
- visually poor sediment formation;
- ordinary runtime defects.

Plateau condition: sustained real-profile use stops uncovering structural/core changes and mostly produces bounded
polish or optional ideas.

#### H1 — De-nozzle rain + contextual Balance footer

Status: **COMPLETE / NATIVE-GREEN** at `744b9f11a5341b0948178b932b23dd1d7e59662b`.

Admitted directly from the first owner screenshots after the plateau cutover:

- replace the hard local ingress cloud / rare-global mixture with full-width rain on every drop plus only an occasional soft two-candidate preference toward the slow persisted focus;
- retain the existing friction/avalanche rule, focus persistence, visible-basin custody, mass conservation, and SandState v3 schema;
- hide non-informative current `live sediment` footer status;
- keep period labels on the default Balance summary, but remove them while detail/edit modes own the footer;
- render the retroactive action as a readable key hint plus **Log past**, and rename the visible action to **Log past activity…** without changing `balance_log_activity` persistence/API identity.

This pack is bounded hardening. It introduces no new feature surface, SQLite migration, sediment state field, or historical-edit semantic change.

Native evidence: formatting, strict Clippy, 284 tests, organic-formation and friction regression, deterministic full-width-rain coverage and weak-focus-bias proof, Balance rendering, HISTORY/recovery/schema regression, isolated bubblewrapped TUI smoke, and validator profile custody all pass. Native fallout was limited to one candidate test iterator correction plus rustfmt in `src/app/report_modal_view.rs` and `src/sand/engine.rs`; architecture semantics did not change.

#### H2 — Metastable repose + local avalanches

Status: **COMPLETE / NATIVE-GREEN** at `f581de486a08547ea5fd74ef3ca2f2fb90e1eb34`.

The design bench selected static/dynamic supported relief `3/1`. The implementation retires the memoryless 25% diagonal
lottery, derives bottom-connected supported surface height from the canonical grid, performs one deterministic diagonal
topple per gravity pass, refreshes a radius-one local active region, and persists sorted active canonical columns in
SandState v4. H1 ingress, pending mass, visible-basin custody, resize, and HISTORY semantics are unchanged.

Design-bench evidence: selected 3/1 produced median event sizes 3–4, p95 7–11, median quiet buildup 11.5–13 grains,
0% one-move events, and no runaway in the synthetic long run. The first native statistics harness falsely blocked H2 by feeding one new grain per gravity pass, roughly 15.6 times normal live ingress. H2R1 corrected the harness to the product's exact 1000 ms ingress / 32 ms engine-update cadence with gravity every second update. Real-cadence proof produced 449 events at 40x20 (median size 8, p95 82, quiet buildup 9) and 658 at 80x30 (median size 8, p95 20, quiet buildup 10), with 0% one-move events, conserved mass, and no runaway. The 1:1 feed remains a bounded overload stress only.

Native evidence: formatting, strict Clippy, 267 library + 23 integration tests (290 non-doc total), H1 rain/HISTORY/recovery regressions, exact v1/v2/v3→v4 migration, in-progress avalanche snapshot/restart equality, resize/hidden-activity continuity, airborne/cavity handling, and real-cadence performance all pass. Native H2/current gravity cost was approximately 1.92x at 40x20 and 1.38x at 80x30. No production semantics changed during H2R1; its sole follow-up commit corrects the statistics harness.

#### H3 — Isolated-spire cap

Status: **COMPLETE / NATIVE-GREEN** at `26fe55d`.

Admitted from direct real-profile visual evidence after H2 cutover. H2's ordinary static threshold intentionally leaves relief three stable, which permits a narrow artifact: a fully isolated bottom-supported stack of exactly three dots can remain as a one-column needle with empty space on both sides. The owner accepts the same isolated shape at height two but not height three or more.

Accepted bounded rule:

- define an isolated spire only when the source supported height is greater than two and both immediate visible neighboring columns have supported height zero;
- keep isolated height two stable;
- for an isolated height-three-or-higher source during the static pass, use an effective cap of two so the surface grain yields through the existing H2 diagonal-topple and local-avalanche path;
- do not apply the exception at a viewport side wall where two real neighboring columns are not visible;
- if either immediate neighbor has supported material, retain ordinary H2 static relief `3`;
- dynamic relief remains `1`; H1 rain, SandState v4, active avalanche persistence, resize custody, mass, and HISTORY semantics remain unchanged.

Required native proof includes `0/2/0` long-run stability, `0/3/0` immediate yield, one-sided/broad supported peaks retaining ordinary H2 repose, existing 3/1 metastability and real-cadence avalanche statistics, H1 rain, resize/side-wall, restart/v4, HISTORY/recovery, full suite, and runtime smoke.

Native closure: the authored candidate was transplanted onto real published main as `9da30c2`; rustfmt fallout and minimal missing boundary/invariant tests were committed as `26fe55d`, with no architecture-semantic changes. The full suite passed with 272 library tests and 23 integration tests. Focused H3 proofs passed for isolated 2-dot stability, isolated 3-dot yield through normal avalanche activity, one-sided and broad neighbor protection, visible-wall protection, mass conservation, settled-profile elimination of interior isolated height-three-or-greater needles, and continued legality of isolated 2-dot stacks. Corrected live cadence produced 446 events at 40x20 (median 8, p95 108, max 274, quiet buildup 9, 0% one-move) and 642 at 80x30 (median 8, p95 24, max 52, quiet buildup 10, 0% one-move), satisfying all qualitative guardrails. H1 rain, resize/visible basin, HISTORY, recovery, v4 migration/restart, and SQLite regressions remained green; H3 adds no serialized state or profile migration. The candidate is validated but not published or installed.

#### H4 — Contact-supported grain-causal avalanches

Status: **COMPLETE / PUBLISHED / REAL-PROFILE GREEN**.

Post-H3 daily-use evidence rejected narrow two-dot one-column peaks but retained the desire for steep one-sided slopes such as `0/6/5`. The accepted H4 model uses contact rather than an absolute peak cap: after vertical fall is blocked, a grain may settle with support below plus at least one lower diagonal or visible wall. Unsupported arrivals become exact mobilized grains. Dynamic movement retains relief `>1`; mobility travels with the grain, actual support loss wakes only exact dependents, and diagonal movement may pass mobility to the newly exposed grain on the same source slip face while that face still has a dynamic route. This preserves multi-grain cascades without regional activation, proximity inheritance, pressure/load state, or global terrain scans.

Two rejected prototypes established the negative architecture: a global contact scan over-relaxed the surface, and event-driven contact coupled to regional `active_vertical` remained throughput-pathological. A first exact-grain prototype then proved clean causality but produced zero structural cascades. H4R2C added same-source slip lineage and passed native behavior validation at `579f3e1b652a2d90efcfcef65e1910d199e464ba`: 40x20 / 10,000 ingress yielded 621 structural cascades with 99 multi-lineage episodes; 80x30 yielded 1,070 with 152 multi-lineage episodes. Both runs conserved mass, completed without runaway/continuous motion, preserved broad/one-sided hills, and drained to interior prominence <=1.

The production boundary now requires SandState v5. Current exact mobilized grain coordinates serialize in canonical row-major order. v5→v5 restore must reproduce grid, mobility, RNG, sweep, focus, pending mass, resize/hidden state, and subsequent gravity exactly. v1-v4 lack exact grain-causal mobility; migration therefore preserves their topology/category mass and deliberately does **not** translate v4 regional active columns into false per-grain causality. Instead restore performs one deterministic one-time scan of the pre-v5 bottom-connected canonical surface and marks only contact-unsupported surface grains mobilized; movement occurs later through ordinary H4 gravity. Once written as v5, this normalization never repeats.

Native validation is complete at `f00b628bd37c42a9b27b2abb4b73b1068c74f551`: exact v5 snapshot/restart and multi-pass continuation, hidden-mobility resize custody, v4 semantic migration, malformed restore/recovery rejection, recolor, recovery, v1-v3 regressions, and SQLite schema invariance passed. PR #89 published main `67ffd84d3c5c924211ac9a14b52b5749fb07ed8b` with installed binary SHA256 `b6f3af5247ce633b4c01c6232c1f1be057f7f9af562b6a5114f424b5f3559f93`. Real profile `95446134-3681-4390-84d7-8d900ebbb892` completed the first v4→v5 owner smoke and second v5→v5 owner restart; persisted state is v5 and sqlite-doctor passed. The pre-v5 backup and H3 rollback pair remain retained; the pre-v4 rollback pair remains retained. No production source or schema changed during closure.

#### H5 — Resize boundary release

Status: **AUTHORED CANDIDATE / NATIVE CERTIFICATION REQUIRED**.

Admitted from concrete owner screenshots and daily use, not feature-seeking. When the live terminal widens, a side wall that previously confined the visible basin can become an interior column. H4 correctly treats a visible wall as static contact support, but current resize merely exposes empty space and supplies no physical event, leaving the former edge frozen as an artificial vertical cut.

Accepted semantics are deliberately narrow:

- shrinking remains projection-only and never wakes hidden grains;
- widening detects each lateral visible boundary that actually ceased to be a wall, including re-expansion inside an already larger canonical canvas and first-time canonical growth;
- resize does not move, repack, globally relax, recolor, add, or remove any grain and consumes no RNG;
- only the exact bottom-connected surface grain of each released former wall is eligible for one-shot H4 mobility;
- that grain is marked only when the newly exposed outward diagonal provides the existing dynamic relief `>1` route;
- all later movement, same-source slip lineage, support-loss propagation, sweep ordering, and persistence remain ordinary H4/SandState v5 behavior.

The candidate adds focused regressions for both canonical growth and hidden-canvas re-expansion. Native certification must run formatting, strict Clippy, full tests, help smoke, and focused resize/H4 regressions. No slip-front propagation or ingress-focus padding belongs to H5. Those remain separate follow-ups so relaxation and deposition cannot be conflated with boundary correctness.

### RHYTHM-001 — Focus/Pomodoro experiment (optional)

Not a plateau blocker.

Preferred progression:

1. turn existing `timer <duration>` parsing into a real live countdown;
2. optionally compose focus → break → focus cycles;
3. keep the active Strata layer/session authoritative throughout;
4. do not automatically falsify the ledger merely because a timer expires;
5. only promote Pomodoro to durable product UI/state if repeated use justifies it.

## Agent locator

Current edge: **PLATEAU-001H / H5 resize boundary release**. Concrete owner resize evidence admitted this unit after H4 real-profile use. The authored candidate changes only the one-shot confinement-release trigger; native certification is required before publication. Slip-front avalanche propagation is the next separate bench only after H5 is certified, and ingress-focus padding remains later so deposition is not mixed with relaxation.

HISTORY-001A/B/C are native-green through `09412b703cf41016f889d725ba235a7a1e63ae6a`. HISTORY-001C established the completed-Idle split transaction, active-preview validation, atomic daily-contribution reconciliation, and Balance historical editor. HISTORY-001D now generalizes that safe primitive into arbitrary From/To activity assignment with explicit collision confirmation and protected live selection.

The old dirty adaptive-resize implementation is preserved externally as custody evidence and is superseded by the
authoritative current-main visible-basin and atomic clear-all architecture. No adaptive code from that stale branch is
part of this unit.
