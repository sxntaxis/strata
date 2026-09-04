---
id: SEDIMENT-014
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Replace massless FlowViz tracers with a conservative visual shadow surface plus weighted local-flow parcels so visible deposition cannot appear before corresponding mobile CategoryId mass has been shown in transit.
---

# SEDIMENT-014 — Conservative Visual Parcels

## Owner evidence

Human rainbow-fill testing of SEDIMENT-013 established a narrower presentation defect than the earlier ray/serialization failures:

- local flux tracers were substantially more natural than destination paths;
- frozen SEDIMENT-008 wall-collapse physics still looked useful;
- however the authoritative settled surface was rendered immediately while only a bounded sample of massless tracers represented the moving flow;
- the result was perceptual mass creation: a few visible dots could be in flight while a much larger amount of colored sediment appeared already deposited at the bottom.

The owner explicitly identified that the in-transit dots did not feel commensurate with the amount of material that subsequently appeared settled.

This is classified as **visual mass-continuity failure**, not a physics rejection.

## Frozen authority

SEDIMENT-008 `oslo-vessel-front` remains the owner-frozen debug physics baseline.

SEDIMENT-014 does not tune:

```text
Oslo static thresholds           {1,2}
catastrophic front seed relief   4
moving-layer erosion relief      2
support-loss recruitment relief  3
front coast/runout rules         unchanged
vessel/canonical boundaries      unchanged
rain/focus                       unchanged
CategoryId physical custody      unchanged
```

SEDIMENT-013 `oslo-vessel-front-flowviz` remains available as the massless-tracer control.

## New sandbox

```text
testingcheats model oslo-vessel-front-parcels
```

The new presentation is split into three independent representations:

```text
AUTHORITATIVE FRONT PHYSICS
    columns + rolling_grains
    decides real sediment behavior

VISUAL SHADOW SURFACE
    settled CategoryId mass that the owner has already seen deposit

CONSERVATIVE VISUAL PARCELS
    CategoryId mass removed from the visual shadow but not yet shown depositing
```

The hard visual custody invariant is:

```text
shadow settled mass + parcel mass
=
authoritative settled mass + authoritative rolling mass
```

for the conservative sandbox, excluding ordinary falling/pending rain custody that already has its own explicit visual representation.

## What enters parcel custody

Only a grain that enters the explicit SEDIMENT-008 moving/front phase is delayed visually:

- severe Oslo failure seed;
- front erosion recruitment;
- support-loss recruitment;
- the equivalent fluid release hook, if reused by a future conservative fluid experiment.

At that event:

1. physics removes the real CategoryId exactly as before;
2. the conservative shadow removes one matching CategoryId from the source column;
3. one unit of parcel mass of that CategoryId is created at the source visual surface;
4. the real rolling/front solver continues unchanged.

Ordinary small Oslo topples that never enter the moving phase mirror immediately in the shadow surface because they are not the wall-collapse transport under study.

## Physical settlement credits

When a real rolling grain settles, authoritative physics immediately receives the grain exactly as before. The visual shadow does **not** receive it yet.

Instead SEDIMENT-014 records:

```text
deposit_due[site][CategoryId] += 1
```

A parcel can convert mobile visual mass back into settled visual mass only when it contacts a site with a real outstanding settlement credit of the same CategoryId.

This removes the SEDIMENT-013 failure mode:

```text
physics settles 200 grains immediately
+
FlowViz shows 15 sample tracers
=
visible spawning at the bottom
```

The visual surface can now grow only by consuming parcel mass against real physical settlement credit.

## Re-entry without double counting

A physically settled grain can be mobilized again before its visual parcel has arrived.

When a new moving-phase entry occurs at a source that already has an outstanding settlement credit for the same CategoryId:

- consume that credit;
- keep the existing parcel mass in motion;
- do not withdraw another visual grain;
- do not create another parcel unit.

This preserves visual mass without requiring a persistent per-grain identity.

The same rule reroutes ordinary one-site physical topples that happen before visual deposition: the settlement credit moves locally instead of fabricating a new settled dot.

## Parcels, not tokens

A parcel stores only:

```text
x, y
vx, vy
CategoryId
mass
age
```

It does **not** store:

```text
VisualTokenId
future destination queue
full physical hop history
precomputed path
```

Nearby parcels of the same CategoryId may coalesce. The bounded parcel count therefore does not imply bounded mass: `mass` remains exact even when multiple units share one visual carrier.

At the soft parcel cap, same-category parcels absorb additional mass rather than dropping it. SEDIMENT-014 therefore has no equivalent of SEDIMENT-013's massless `dropped tracer sample` semantics for conservative custody.

## Motion and settlement

Parcels reuse the useful SEDIMENT-013 local-flow idea:

- recent signed physical edge flux biases lateral motion;
- gravity handles airborne descent;
- the **visual shadow bed** supplies contact geometry;
- downhill shadow relief is fallback guidance;
- outstanding same-category settlement credits form a bounded local sink field when recent flux becomes weak.

The sink is recomputed from current state every frame. A parcel does not own a hidden final destination.

When a parcel contacts a matching settlement credit, a bounded amount of its mass can deposit during that visual frame. Each consumed unit:

```text
parcel mass -= 1
shadow column pushes CategoryId
settlement credit -= 1
```

Thus every newly visible settled dot is backed by a unit that was previously in mobile visual custody.

## Rendering

`oslo-vessel-front-parcels` does not render authoritative `columns` directly during the experiment.

It renders:

1. conservative shadow columns;
2. ordinary falling rain;
3. conservative parcel samples.

A weighted parcel renders a compact dot cloud whose visible density grows with its mass. This is a presentation sampling choice only; the exact conservative quantity is the parcel `mass`, not the number of terminal cells that happen to remain unobscured after raster overlap.

## Scheduler

SEDIMENT-011 clock separation remains unchanged:

- front physics uses stable wall-clock avalanche cadence;
- 64x/128x synthetic drive debt pauses during visible avalanche playback;
- baseline live rain remains approximately one dot per wall-clock second;
- conservative parcel drainage continues after authoritative physics becomes quiescent;
- queued synthetic advance resumes only after visual mobile custody drains.

No global one-dot barrier is reintroduced.

## Diagnostics

`testingcheats status` exposes conservative custody:

```text
parcels=count:N
        mass:N
        shadow:N
        due:N
        flux_edges:N
        peak_count:N
        peak_mass:N
        withdrawals:N
        deposits:N
        reused:N
        coalesced:N
        misses:N
```

`misses` must remain zero in accepted tests. A nonzero value means authoritative physical mass requested a CategoryId that the shadow/due ledger could not account for and is a correctness failure, not a harmless visual drop.

## Machine gates

Native validation must prove:

1. 2,000 ordinary quiescent drives remain exact frozen-front physics;
2. physical state/RNG/front counters are unchanged;
3. moving-phase entry transfers exactly one CategoryId unit from shadow to parcel custody;
4. physical settlement creates credit but does not immediately grow the visual shadow;
5. visual deposition consumes both one parcel unit and one same-category physical settlement credit;
6. a settle/re-enter cycle reuses pending visual mass rather than duplicating it;
7. deterministic wall failure preserves `shadow + parcel == settled + rolling` throughout visual drainage;
8. the wall failure produces substantial concurrent parcel mass;
9. the parcel cloud drains to zero credits without shadow-accounting misses;
10. SEDIMENT-011 live-rain and stable-clock contracts remain green;
11. `testingcheats fill` remains the 80%-VW rainbow fixture;
12. production H4 and persistence remain untouched.

## Human gate

Use:

```text
testingcheats model oslo-vessel-front-parcels
testingcheats fill
testingcheats fallspeed 64x
```

Then perform the established wide → narrow → brief wait → wide reconnection.

Judge primarily:

- settled colored material must not appear at the foot of the collapse before a commensurate amount of parcel mass has visibly travelled through the event;
- the bottom should grow progressively as mobile parcels arrive, not jump to authoritative future state;
- no SEDIMENT-010 serialization;
- no SEDIMENT-011 rays;
- no SEDIMENT-012 destination combs;
- no SEDIMENT-013 sparse-tracer / large-deposit mismatch;
- the accepted SEDIMENT-008 wall-collapse morphology must remain physically unchanged;
- live rain and stable visual cadence remain intact.

If conservative parcels establish convincing mass continuity but their local sink/reconciliation motion is too artificial, the next bounded problem is **how to advect conservative mobile mass**, not whether to return to token identity or immediate authoritative rendering.
