---
id: RAIN-005-PRODUCTION-CUTOVER
kind: work
state: complete
created: 2026-09-10
updated: 2026-09-10
authority: working
summary: C2R2 is natively certified as the actual normal-TUI physics authority, with H4-v5 migration and Classic restart custody proven before publication/install.
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

## Native certification closure — 2026-09-10

The local native gate consumed the exact author-hardened complete-history bundle (`SHA256 942b6a5d87040a5f523b7e87940160409d555030587de4aec6104c2bb13a7077`) and returned **PASS_STRATA_C2R2_PRODUCTION_CUTOVER_NATIVE**. Native validation used Rust/Cargo 1.98.0 on x86_64 Linux.

Certified source lineage before this Notebook-only closure:

- HEAD `9c2e7b213316a74e312b1a6bf8c051404b2ae7b2`;
- tree `617e3830dd2c730b757bd9a84c9edf1f2bc39a81`;
- parent `3391a907f0062c86653d2c4233b764101e0496d1`;
- published RAIN-005 closure `ef46772d33ad46845b338fd7f020b3866db96a6b` confirmed as ancestor;
- final worktree clean.

Native evidence:

- `cargo fmt --all -- --check`: PASS;
- strict Clippy: PASS;
- full tests: 512 library + 24 integration passed, 20 ignored, 0 failed;
- production routing: `App.sand_engine` is `ClassicProductionEngine`, constructed with `new_production`; real spawn/update paths use that engine; authority string is `classic-c2r2-anchor-apex-latent`;
- H4-v5 migration: 8x6 fixture, 4 placed grains, 6 pending mass, total mass 10, FIFO `[1,1,2,2,2,1]`, frame 41, sweep false; zero pre-first-tick topology/category drift and H4 mobilization discarded;
- deterministic H4-derived Classic hidden-state initialization: PASS;
- Classic snapshot/restart plus future-continuation equality: PASS;
- SQLite Classic runtime round-trip: PASS;
- malformed/fail-closed and detached-recovery gates: PASS;
- B2/B2R1/C1/C2R2 focused regressions: PASS;
- anti-nozzle: N=4 max 2, N=20 max 8;
- PERF-001: PASS at 8.31x;
- release `physics`: `Physics: classic-c2r2-anchor-apex-latent`;
- certified release version: `strata 0.7.7 (9c2e7b213316)`;
- certified release SHA256: `11feeca34e71b12e886e349314f9a28c094f73bcfe6800e22b006db5ff4a9080`.

The validator created and verified a backup of the live SQLite/profile, did **not** mutate the live profile, and ran the candidate against a disposable copied profile. TUI startup, update, exit, reopen, persistence, doctor, version, and physics smoke all passed.

Three native fallout commits were reviewed as mechanical and behavior-preserving only: rustfmt formatting (`7824a9b`), strict-Clippy compatibility (`3391a90`), and restoration of a test-only coordinate re-export with scoped unused-import allowance (`9c2e7b2`). No physics, migration, persistence, RNG, morphology, timing, or profile semantics changed in those fixes.

## Closure and rollout boundary

RAIN-005 morphology and the corrective production cutover are now technically closed. Publication/install is a rollout operation, not another semantic gate. Publish the exact certified lineage through the repository branch/PR workflow, rebuild from the resulting published `main` (the commit/version hash will necessarily differ if this Notebook-only closure is included), then run a bounded post-install smoke for version, `physics`, profile doctor, and ordinary TUI startup/quit/reopen. Do not reopen morphology or change migration semantics during rollout.
