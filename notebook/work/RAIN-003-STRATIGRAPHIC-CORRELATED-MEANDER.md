---
id: RAIN-003
kind: work
state: candidate
created: 2026-09-07
updated: 2026-09-07
authority: working
summary: Replace the owner-rejected bland RAIN-002 dwell/lowest-of-three morphology with a continuous aperiodic correlated focus meander, weak broad terrain steering, category-boundary rephasing, and one further recursive-golden reduction of focus-only padding while retaining 75/25 broad rain, RAIN-002 free ingress/debt fixes, PERF-001, and frozen Classic settlement.
---

# RAIN-003 — Stratigraphic correlated meander

## Owner evidence

RAIN-002 passed machine validation but failed the owner morphology gate. Its free-site ingress and immediate `128x -> 1x` debt ownership behavior remain desirable, but one-hour dwell plus lowest-of-three compensational avulsion produced strata that read too homogeneous/bland. The owner preferred the more varied RAIN-001 family, while specifically wanting:

- more wandering / faster focus variation;
- less periodic left-right "Pepsi-logo" structure than the old forced opposite-side waypoint sweep;
- stronger differences between successive color/category strata, not merely more edits to the top relief while one category remains active;
- one additional recursive-golden reduction of focus-only side padding;
- unchanged broad falling-rain appearance with no visible nozzle.

Classic settlement remains frozen at `momentum-grounded-contact`. RAIN-003 must not reopen avalanche, repose, texture, anchors, momentum, category transport, renderer, persistence, or PERF-001 exact optimization.

## Research/design synthesis

The retained sedimentological inspiration is intermediate compensational/autogenic stacking rather than perfect topographic compensation. RAIN-002 over-applied the compensation idea by deliberately selecting the lowest of three broad loci after long dwells; that naturally filled accommodation and homogenized successive envelopes.

RAIN-003 instead uses **correlated motion with weak topographic steering**:

- persistence gives a depositional locus enough coherence to build a recognizable lobe/shoulder;
- stochastic heading updates break periodic forced left-right oscillation;
- broad left-vs-right relief is only one weak input to heading, so the focus may continue upslope when inertia/randomness wins;
- a real category boundary temporarily reduces heading memory without changing focus position, giving the next stratum an opportunity to develop a different envelope;
- no global-lowest scan, explicit waypoint destination, dwell state, category-specific position, or focus teleport exists.

The category transition is a **stratigraphic rephase event**, not category physics. The algorithm observes only that the FIFO ingress `CategoryId` changed; it never assigns meaning to a particular ID/color.

## Candidate semantics

### 1. Broad rain remains 75/25

Hybrid preserves the already owner-approved anti-nozzle envelope:

- 75% uniform over current free visible-top ingress sites;
- 25% broad focus-biased by choosing the nearer of two stochastic free sites in the focus corridor;
- if the focus corridor has no free site, use the already sampled global free site;
- physical terrain and ordinary rain continue over the full visible width.

`RAIN_FOCUS_BIAS_ONE_IN` remains exactly `4`.

### 2. Smaller focus-only padding

RAIN-001/002 used approximately 11.8% side padding:

```text
((1 - 1/phi) / 2) / phi
```

RAIN-003 applies one additional golden subdivision:

```text
((1 - 1/phi) / 2) / phi / phi
```

which is approximately **7.3% per side**, leaving approximately **85.4%** of visible width available to the focus.

This is focus-only. It is not a physical wall, empty margin, or rain exclusion zone.

### 3. Continuous correlated meander

Remove RAIN-002 dwell, relocation candidates, waypoint targets, and transition state.

The focus carries only:

- current `x`;
- current signed direction;
- movement cadence counter;
- heading-decision cadence counter.

A hypothetical uninterrupted edge-to-edge traverse is normalized to approximately **14,400 ingresses (~4 simulated hours)**, about three times the mobility of the earlier ~12-hour RAIN-001 traverse. This is not a periodic traverse requirement; heading may reverse before crossing.

Normal heading decisions occur approximately every 900 ingresses (~15 simulated minutes). At a normal heading update:

- directional persistence is the dominant outcome;
- weak broad-terrain steering may select the lower side;
- stochastic perturbation may choose another direction;
- soft inward edge steering may act near corridor bounds.

At a true corridor boundary, an impossible outward step reverses inward rather than leaving the corridor.

### 4. Weak terrain steering only

Terrain never chooses a destination.

At a heading update, compare broad bottom-connected smoothed terrain on the left and right of the current focus, with sample centers roughly one sixth of the focus corridor away. The lower broad side yields only a steering direction (`-1`, `+1`, or neutral on equality).

During ordinary meander, terrain steering is a minority outcome relative to directional persistence. This deliberately permits runs that continue toward higher terrain, preventing perfect compensation/flattening.

### 5. Category-boundary stratigraphic rephase

The actual FIFO category that obtains a free ingress site is observed before that target is sampled.

- first category: establishes the baseline only;
- repeated same category: no rephase;
- changed category: preserve exact focus `x` and current direction, set a 900-ingress (~15-minute) rephase window, and force the next heading decision immediately;
- during rephase, heading decisions occur every 180 ingresses (~3 minutes) and weight terrain/random perturbation more strongly than normal directional persistence;
- rephase decays automatically back to ordinary correlated meander.

No category change teleports the focus, chooses an opposite side, resets rain RNG, or assigns category-specific geography.

### 6. Retained RAIN-002/PERF-001 fixes

Keep unchanged:

- free-site-only visible-top ingress; no occupied-target nearest-free relocation;
- FIFO pending mass when the top is full;
- separate fallspeed-generated and explicit-Advance debt;
- `fallspeed` changes discard only stale speed debt immediately;
- PERF-001 sparse occupancy, grounded index, exact RNG jump, O(1) metadata, dev optimization and accelerated budgets;
- `rgb-luma-safe` appearance baseline.

## Machine gates

Required focused proof:

1. 75/25 bias constant remains exact and short-window target histogram remains broad/anti-nozzle.
2. recursive-golden side padding is symmetric and approximately 7.3% per side; focus never leaves the resulting corridor; ordinary rain still reaches both padding regions.
3. deterministic long meander exhibits movement in both directions, multiple reversals, and substantial corridor coverage without any waypoint/dwell state.
4. constructed lower terrain on one broad side yields that side as the terrain-steering recommendation.
5. terrain recommendation is **not** compulsory: across deterministic seeds, normal heading updates sometimes preserve the opposite current direction and sometimes follow terrain.
6. category change preserves focus position/direction at the event boundary, activates rephase, and makes the next heading decision immediate; repeated same category does not rephase.
7. free-site ingress and full-top pending FIFO regression remains green.
8. fallspeed debt ownership regression remains green.
9. PERF-001 optimized/reference exactness includes all RAIN-003 focus/rephase/category fields and remains green for Uniform + Hybrid.
10. Classic baseline metrics, mass, themes/appearance, parser/help, fmt, strict Clippy, full tests and native PERF capacity remain green.

Capture `RAIN_003_METRICS` from the 24k target histogram.

## Human gate

After native PASS:

```text
testingcheats clear
testingcheats model hybrid
testingcheats fallspeed 128x
```

Judge several simulated hours and switch active categories often enough to expose multiple strata.

Desired morphology:

- falling rain remains visually broad and the focus is not detectable as a nozzle;
- within one category, relief remains coherent rather than noisy;
- successive category strata develop visibly different lateral thickness envelopes, shoulders and centroids;
- focus can reverse/revisit regions but does not read as deterministic left-right-left-right oscillation;
- terrain lows influence the walk without systematically flattening all relief;
- no single permanently dominant central lump;
- free-top ingress behavior remains natural near the visible top;
- `testingcheats fallspeed 1x` slows immediately.

If morphology still needs adjustment, the first/only tuning axis should be correlated meander persistence/mobility. Do not reopen 75/25 or Classic settlement without new owner evidence.
