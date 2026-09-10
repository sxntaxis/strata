---
id: RAIN-005-PRODUCTION-CUTOVER
kind: work
state: active
created: 2026-09-10
updated: 2026-09-10
authority: working
summary: Correct the RAIN-005 closure error by making owner-selected Classic C2R2 the actual normal TUI physics authority while preserving the owner's existing H4 v5 sediment and restart custody.
---

# RAIN-005 — production Classic C2R2 cutover

## Why this unit exists

The owner selected C2R2 `anchor-apex-latent` as the final morphology and explicitly intended it to become the default engine. Commit `ef46772d33ad46845b338fd7f020b3866db96a6b` froze and documented that decision but did not perform the runtime wiring: normal `App` still owned legacy H4 `SandEngine`, while Classic remained a development/testing engine. Installation and `--version` therefore proved artifact identity but not physics authority.

This unit corrects that integration defect. It does not reopen morphology search.

## Required production authority

Normal TUI `App.sand_engine` must be Classic C2R2 with the already accepted semantics:

- `ClassicRainMode::WanderingFocus`;
- rugged texture/local-repose distribution;
- `MomentumGroundedContact`;
- C1R1 convexity routing of `(repose, memory)` state;
- C2R2 anchor fourth-unit latency only while a routed repose-4 state is a strict one-column apex;
- B2 golden-small low-discrepancy focused/uniform schedule;
- B2R1 avulsion only when a different category physically enters FIFO ingress;
- optimized Classic gravity path used by real binaries.

No heritage-walk/G2 geography arm is promoted.

## Persistence and migration contract

SandState stays at v5. `classic_runtime` is an additive optional payload. Older H4 v5 JSON therefore remains readable without a database-schema migration.

On the first H4→Classic restore:

1. validate the complete old state and category references before target mutation;
2. preserve placed grain coordinates/category IDs and pending FIFO mass;
3. preserve frame/sweep chronology used by subsequent updates;
4. discard H4 `mobilized_grains`, because that is obsolete hidden H4 dynamics rather than sediment topology;
5. deterministically derive independent Classic physics/rain/repose RNG streams from the persisted H4 RNG seed;
6. initialize the accepted local-repose field without applying gravity or flushing pending ingress during restore;
7. persist Classic continuation metadata on the next normal state/checkpoint write.

Ordinary canonical canvas growth remains governed by STRATA-D025: if the live viewport is larger than the persisted canvas, existing cells may be translated only by that pre-existing center/bottom expansion rule. The cutover itself adds no second topology transform.

On Classic→Classic restore, hidden state must round-trip exactly when the canonical dimensions are unchanged. If the viewport grows, old local-repose/focus state translates with the same horizontal offset while newly exposed columns receive ordinary fresh local-repose state. Invalid or contradictory Classic metadata fails closed before target mutation.

Detached recovery remains arithmetic-only: it may append due FIFO ingress but must not replay missed physics or discard/resample Classic hidden state.

## Provenance contract

`strata physics` reports the compiled production authority string `classic-c2r2-anchor-apex-latent`. This command is a user-facing provenance aid, not sufficient proof by itself. Native certification must additionally compile-test that `App.sand_engine` has type `ClassicProductionEngine`, verify the exact production constructor semantics, and exercise migration plus persistence round trips.

## Safety gate for the owner's current sediment

The live default profile is never a validation target. Native closure must first create a disposable rooted profile cloned from the owner's current profile/SQLite authority, with transient runtime socket/lock evidence excluded. The candidate may then be launched against that clone for a human smoke.

The smoke checks:

- the existing sediment appears with its categories/colors and no clear/reset;
- startup does not replace the pile with an empty or synthetic topology;
- new ingress visibly resumes under C2R2;
- ordinary future Classic movement/avalanches can act on the inherited pile;
- quit/restart of the copied profile preserves the resulting Classic state;
- `strata physics` and `--version` identify the candidate.

No publication or installation is authorized until this copied-profile gate passes.

## Native certification

Required before human copied-profile smoke:

- repository `check` and `test` commands;
- production wiring test;
- exact C2R2 constructor/authority test;
- H4 v5 topology/pending-mass cutover test;
- Classic hidden-state exact round trip;
- malformed hidden-state fail-closed test;
- SQLite sand-authority hidden-state round trip;
- detached-recovery hidden-state preservation;
- existing B2/R1/C1/C2R2 focused regressions;
- PERF-001;
- release build plus `--version`, `physics`, and help smoke.

If compilation exposes only unequivocal formatting/import/type fallout, the local validator may repair it in a separate mechanical commit. Any change to physics, migration semantics, persistence meaning, RNG, morphology, timing, or profile behavior is a semantic blocker and returns to the author.

## Static handoff hardening — 2026-09-10

A post-interruption source audit found two certification defects in the original WIP handoff and corrected them without changing C2R2 physics:

- the H4-v5 migration fixture listed placed grains in non-canonical order while asserting exact equality against the row-major canonical snapshot writer; the fixture now represents real persisted H4 ordering and also proves multi-category FIFO pending-run preservation, frame/sweep preservation, mass conservation, and deterministic hidden-state initialization;
- detached/transition recovery previously preserved `classic_runtime` without validating its cross-field authority invariants first. Recovery now reuses the Classic runtime validator and rejects contradictory Classic metadata (including top-level/runtime RNG disagreement) before arithmetic catch-up. Zero-count pending runs are likewise rejected instead of bypassing the stricter live restore path; unknown fields inside the versioned Classic runtime payload are rejected rather than silently ignored.

The Classic restart gate now continues both the pre-restart and restored engines through the same subsequent category-boundary/spawn/update sequence and requires exact resulting state equality. The App wiring test also type-checks the production `spawn` and `update` method paths on `ClassicProductionEngine`.

These changes remain **AUTHORED / NATIVE VALIDATION PENDING** because the authoring container still has no Rust toolchain. They do not authorize publication, installation, or live-profile mutation.
