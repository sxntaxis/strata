---
id: PLATEAU-001
kind: work
state: active
created: 2026-08-27
updated: 2026-08-27
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

Status: **IMPLEMENTED / NATIVE VALIDATION PENDING**.

Interaction decision: `range` is a fourth Balance mode reached by the configurable `balance_range` action (default `r`). It edits explicit `From` and `To` ISO dates inline, commits only after validation, and then shifts as a whole window by its own span; it does not overload preset offsets.

Deliverables:

- Add a visible custom range choice beside Day / Week / Month.
- Provide explicit From and To date editing with validation (`from <= to`).
- Keep preset navigation convenient; custom range must not inherit nonsensical preset-offset semantics.
- Drive summary rows, detail logs, live provisional time, and historical snapshot selection from the same explicit
  report window.
- Converge visible TUI behavior with existing CLI range reporting.

#### HISTORY-001C — Retroactive missed activity

Deliverables:

- Add an explicit historical operation for recording a missed activity interval.
- Default safe case: reclassify an interval currently owned by Idle.
- Split surrounding canonical sessions as needed while conserving every second.
- Refuse or explicitly escalate overlaps with already-classified non-Idle time; never silently double-count.
- Persist transactionally and reconcile every affected operational day.
- Preserve provenance sufficient to distinguish corrected history where useful without inventing per-grain time
  provenance.

#### HISTORY-001D — General historical correction

Deliverables:

- Deliberately reclassify an existing non-Idle interval when the user asks to correct it.
- Define conflict, split, merge-adjacent, and failure behavior around canonical session identity.
- Keep SQLite, in-memory authority, reports, and daily contribution reconciliation atomic/coherent.

#### HISTORY-001E — Sediment recolor after correction (nice to have)

Deliverables:

- Change the current canonical sediment category composition by the corrected duration.
- Prefer deterministic in-place recolor of equivalent old-category grains to new-category grains.
- Preserve grain coordinates/topology and total mass.
- Do not add timestamps/session IDs to every grain solely for this feature.
- Keep the historical day-end-snapshot rewrite question explicit and separate.

### SEDIMENT-002 — Organic formation

Goal: make emergent shapes less white-noise-like while preserving falling-sand semantics.

Deliverables:

- If only one diagonal is open, move there deterministically; random choice belongs only where both paths are valid.
- Replace independent uniform top-edge ingress with a spatially correlated/wandering source or an equivalently
  simple correlated process.
- Persist any additional small stochastic state required for deterministic restart continuity.
- Keep new ingress inside the visible physics basin and preserve existing shrink/freeze/re-expand behavior.
- Do not add mountain generators, terrain post-processing, or user-facing randomness knobs until a behavior is
  demonstrated to be worth controlling.

### INTERACTION-002 — Menus, Settings, and vocabulary convergence

Goal: polish action placement after historical operations exist, rather than polishing an incomplete menu graph.

Expected ownership:

- Main view: what is happening now.
- Layer pop-up: current classification, tags, layer management.
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

### RHYTHM-001 — Focus/Pomodoro experiment (optional)

Not a plateau blocker.

Preferred progression:

1. turn existing `timer <duration>` parsing into a real live countdown;
2. optionally compose focus → break → focus cycles;
3. keep the active Strata layer/session authoritative throughout;
4. do not automatically falsify the ledger merely because a timer expires;
5. only promote Pomodoro to durable product UI/state if repeated use justifies it.

## Agent locator

Current edge: **HISTORY-001B native validation — visible From → To selection in Balance**.

The validated HISTORY-001A implementation includes the Balance runtime/source vocabulary cutover, default `b` opener, shared `ReportWindow`, preset and live/log routing through that window, CLI custom-range reuse of the same domain seam, and JSON export schema 4 for `balance_effect`. HISTORY-001B now adds the inline `range` editor and custom-window navigation on top of that seam; native formatter, Clippy, full tests, smoke, and targeted TUI proof are the remaining gate before moving to HISTORY-001C.

The old dirty adaptive-resize implementation is preserved externally as custody evidence and is superseded by the
authoritative current-main visible-basin and atomic clear-all architecture. No adaptive code from that stale branch is
part of this unit.
