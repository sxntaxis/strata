---
id: SEDIMENT-004
kind: work
state: active
created: 2026-09-02
updated: 2026-09-02
authority: working
summary: Re-evaluate the visually strongest Oslo-derived sediment on Strata's canonical visible-basin architecture by isolating three boundary conditions: legacy zero-outside, closed box, and canonical-height vessel overflow.
---

# SEDIMENT-004 — Oslo boundary laboratory

## Why this unit exists

Owner visual evidence from SEDIMENT-003 is negative for the replacement direction:

- `classic` / `hybrid` preserve intuitive grain motion but their topography is too bland;
- H4 remains visually better than classic/hybrid but its cascades are not legible enough in ordinary owner use;
- the earlier Oslo-derived sandbox remains the strongest observed combination of topography and visible avalanches, despite its persistent triangular/pyramidal envelope.

The next experiment therefore returns to the Oslo-derived local stochastic-slope law instead of continuing to tune classic gravity.

Production authority remains H4/v5. This unit is debug-only and has no persistence, migration, SQLite, or default-model cutover.

## Corrected causal diagnosis

The earlier Oslo sandbox already propagated avalanches causally: after a topple, the source and destination neighborhoods were re-enqueued and relaxation continued to quiescence before the next drive grain committed. A new `slip` variant that merely adds causal re-enqueueing would duplicate existing Oslo behavior and is not a meaningful experiment.

The old pyramid pressure has a narrower cause. At a visible side edge, the missing neighbor was represented as height `0`. With local critical slopes in `{1,2}`, that pins both side profiles toward low heights and imposes a wedge envelope as the surface rises inward.

SEDIMENT-004 isolates that boundary rule without changing rain or the Oslo relaxation law.

## Shared Oslo law

All three Oslo variants share exactly:

- local stochastic critical slopes `{1,2}`;
- one top grain moved per topple toward the steeper downhill side;
- stochastic left/right tie breaking;
- threshold redraw only at the toppled source;
- source/destination neighborhood re-enqueueing;
- relaxation to quiescence before the next drive commits;
- full-visible-width Strata rain with the accepted 90/10 weak wandering-focus bias;
- canonical horizontal topology, current visible horizontal basin, temporary VW side walls where applicable, hidden horizontal custody on shrink, and direct reconnection on expansion;
- debug-only falling-dot presentation and the cooperative testing-cheats scheduler.

Only the missing-neighbor boundary condition differs.

## Variants

### `testingcheats model oslo-zero`

Historical control matching the visually successful/pyramidal experiment:

- missing side neighbor height = `0`;
- an outward topple discharges the top grain;
- expected to reproduce low edges / wedge pressure and broad avalanche statistics.

This is diagnostic control, not accepted wall semantics.

### `testingcheats model oslo-box`

Literal closed lateral wall:

- the missing side direction contributes zero outward relief;
- no grain can leave through a visible side wall;
- no lateral discharge exists.

This directly tests how much of the old topography came from the zero-outside sink. Prior evidence predicts that a globally driven conservative `{1,2}` surface may settle toward a high, quiet plateau; that prediction is evidence to measure, not an acceptance criterion to weaken.

### `testingcheats model oslo-vessel`

Canonical-height physical vessel:

- below the canonical wall top, side behavior is identical to `oslo-box`;
- the wall height is the monotonic canonical dot-grid height, so shrinking the viewport never lowers it;
- only a column standing above that canonical wall top can have positive outward relief and overflow;
- overflow is the only lateral mass exit in this variant.

This tests the owner's proposed finite-container interpretation: closed walls through the physical canvas, with ordinary overflow only after the vessel actually fills.

Important expected consequence: before the surface reaches canonical wall height, `oslo-vessel` and `oslo-box` should behave the same. Therefore short 10-minute comparisons are insufficient to evaluate vessel overflow; multi-hour simulated-age testing is required.

## Canonical / visible-window contract

Horizontal resize follows the accepted visible-basin semantics:

- canonical width grows monotonically and never shrinks;
- settled columns outside the current VW remain in custody and receive no rain;
- current VW side boundaries are the physics boundary;
- shrink does not move or delete settled hidden columns;
- re-expansion reconnects the preserved terrain directly;
- a reconnection-triggered mega-avalanche is expected and is not a defect by itself.

The Oslo representation is a column heightfield rather than H4's explicit 2-D contact grid. A vertical terminal shrink can therefore hide the top portion of a canonical column without giving the sandbox an independent per-grain vertical activity boundary. SEDIMENT-004 does **not** silently invent vertical clipping/compaction semantics to hide that mismatch. The discriminating resize proof for this unit is horizontal shrink/re-expansion; any vertical hidden-top behavior observed locally is recorded as a limitation for the next design pass.

## Harness review

The local validation agent's responsive-harness commit `91e8effcdf0eccd2d3540ec03715819035d40f64` is retained as useful experimental infrastructure:

- simulated advance is queued rather than monopolizing the UI thread;
- work is processed in approximately 6 ms cooperative chunks;
- `128x` is available;
- long explicit advances are permitted;
- `status` exposes remaining queued simulated time;
- model switch / clear reset the queued experiment.

SEDIMENT-004 further keeps Oslo lattice state authoritative during each acceleration chunk and synchronizes the Ratatui surface only once after the chunk, avoiding a full presentation-grid rebuild on every simulated physics tick.

An additional FIFO robustness rule re-homes an in-flight drive to the nearest accepting visible site if an earlier drive filled its original landing column before it committed. This prevents one stale landing target from blocking the entire accelerated ingress stream and does not change Oslo relaxation.

## Machine questions

The sandbox must prove:

1. `oslo-zero` still exhibits strong center-vs-edge wedge pressure under long drive.
2. `oslo-box` cannot discharge laterally under long conservative drive.
3. `oslo-vessel` is identical to a closed wall below the canonical top.
4. `oslo-vessel` can overflow only after exceeding the canonical wall height.
5. canonical wall height does not fall on viewport shrink.
6. a temporary horizontal VW wall blocks a hidden neighbor, and re-expansion makes that preserved neighbor physically reachable again.
7. after the vessel fills, its overflow regime recovers a non-microscopic, broad avalanche tail rather than the previously observed `p95=2` collapse.

## Owner visual questions

Compare H4, `oslo-zero`, `oslo-box`, and `oslo-vessel` at matched simulated ages.

- Does `oslo-zero` reproduce the topography/avalanches the owner remembers liking?
- Does `oslo-box` remove the forced wedge but become too flat or quiet?
- After enough simulated hours to reach the rim, does `oslo-vessel` restore interesting avalanche activity without the low empty edges?
- Does vessel fill/overflow become visually pathological or clip the useful surface?
- Does horizontal shrink → evolve → expand preserve the fun reconnection avalanche?

No variant is promoted automatically. This laboratory is intended to tell us which physical mechanism is actually providing the useful visual behavior before a production redesign.
