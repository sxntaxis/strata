---
id: RAIN-004
kind: work
state: reference
created: 2026-09-07
updated: 2026-09-07
authority: working
summary: Machine-green and owner-selected as the best-so-far morphology reference after full-width focus plus golden-small broad bias; it is a comparison baseline for further heterogeneity work, not a declaration that its magic constants are perfect or permanently frozen.
---

# RAIN-004 — Full-width golden-bias correlated meander

## Owner evidence

RAIN-003's correlated meander and category rephase produced a more interesting stratigraphic result than the owner-rejected RAIN-002 dwell/lowest-of-three morphology. The owner now requests exactly two additional morphology changes:

1. remove the remaining focus-only side padding so the wandering focus can use the complete active width;
2. increase the broad focus-biased ingress share from 25% to the small part of the golden ratio.

The owner did **not** request a new focus algorithm, avalanche/settlement tuning, a nozzle, category-specific geography, or a return to RAIN-002 compensation. RAIN-003's correlated meander is therefore retained as the morphology law.

## Candidate semantics

### 1. Full-width focus domain

The focus domain is exactly the complete current active width:

```text
focus_start = bounds.x_start
focus_end   = bounds.x_end
```

There is no focus-only padding or centered corridor. Ordinary uniform rain and the broad focus-biased component already use the complete active width; RAIN-004 makes focus authority match that width as well.

The existing soft edge steering remains only a **directional influence near the real active-width edges**. It does not reserve an inaccessible margin and does not change the hard closed-wall physics of Classic.

### 2. Golden-ratio mixed rain

Let:

```text
phi = 1.618033988749895
golden_small = 1 / phi^2 ~= 0.38196601125
golden_large = 1 - golden_small ~= 0.61803398875
```

Hybrid ingress becomes:

- `golden_large` (~61.8%): uniform sample from all currently free visible-top sites;
- `golden_small` (~38.2%): broad focus-biased sample, still implemented by choosing the nearer of two stochastic free sites from the full focus domain.

This is still a broad statistical bias. It is **not** direct spawn at `focus_x`, a Gaussian/nozzle emitter, or a narrow spray.

The probability draw consumes the existing rain RNG stream; it introduces no second stochastic authority.

### 3. Retained RAIN-003 morphology

Keep unchanged:

- continuous correlated meander;
- directional persistence;
- ~14,400-ingress hypothetical full-width traverse normalization;
- normal and rephase heading cadences;
- weak broad terrain steering, never destination selection;
- stochastic heading perturbation;
- category-boundary rephase that preserves exact focus position and current direction at the event boundary;
- no category-specific target, no teleport, no forced left/right alternation.

Because the active focus width is now larger, the same edge-to-edge normalization naturally applies across the complete active width.

### 4. Retained ingress/performance/physics authority

Keep unchanged:

- RAIN-002 free-site-only top ingress;
- FIFO pending mass when the top is full;
- separate fallspeed and explicit-Advance debt ownership;
- immediate `128x -> 1x` fallspeed slowdown semantics;
- PERF-001 sparse occupancy/grounded indexes, exact RNG jump, O(1) metadata and dev optimization;
- frozen `momentum-grounded-contact` settlement;
- `rgb-luma-safe` appearance baseline;
- persistence/schema/theme/catch-up/Advance authority outside this slice.

## Machine gates

Required focused proof:

1. `RAIN_FOCUS_BIAS_PROBABILITY` equals `1 / phi^2` and is approximately `0.38196601125`.
2. A deterministic probability sample converges to the golden-small share within a bounded tolerance.
3. Focus bounds equal the complete active width exactly; no residual side padding is encoded in focus bounds.
4. Long deterministic correlated meander remains inside the active width, moves in both directions, reverses multiple times, and covers a substantial portion of the full width.
5. Weak terrain steering remains advisory rather than compulsory.
6. Category-change rephase semantics remain unchanged.
7. Free-site ingress/full-top pending FIFO remains green.
8. The 24k target histogram remains broad with every horizontal bin populated and no nozzle-like dominant bin; capture `RAIN_004_METRICS`.
9. PERF-001 optimized/reference exactness remains green for Uniform + new Hybrid, including rain RNG/focus/rephase state.
10. Classic baseline metrics, mass, appearance/themes, parser/help, fmt, strict Clippy, full tests and native PERF capacity remain green.

## Human gate

After native PASS:

```text
testingcheats clear
testingcheats model hybrid
testingcheats fallspeed 128x
```

Judge several category transitions and simulated hours.

Desired result:

- focus-generated relief can occupy more of the full width, without a reserved side margin;
- successive strata remain varied/coherent rather than RAIN-002 bland;
- the stronger ~38.2% bias accentuates stratigraphic differences without making the falling focus visually obvious;
- no deterministic Pepsi-like left/right pattern;
- no narrow nozzle or side-spawn regression;
- `testingcheats fallspeed 1x` slows immediately.

Do not introduce another morphology rule if this needs tuning. The next owner decision should judge the two authorized knobs themselves: full-width focus and golden-small bias.


## Native + owner reference gate — 2026-09-07

RAIN-004 passed machine validation (509 passed, 6 ignored; PERF median 1457.78x) and the owner judged its long-form morphology the best Strata result so far. It is therefore the reference/control for RAIN-005 work. This does **not** promote `1/phi^2`, 14,400/900/180 ingress timing, width/6 terrain offset, or width/12 smoothing/edge distances into final product constants. The owner explicitly wants the next derived model to pursue **equal or greater heterogeneity** while preserving the no-nozzle quality and avoiding RAIN-002 blandness or periodic Pepsi-like oscillation.
