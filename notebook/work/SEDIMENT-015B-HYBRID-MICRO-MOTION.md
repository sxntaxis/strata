---
id: SEDIMENT-015B
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Add presentation-only local separation and exact pending-depth approach to the machine-green SEDIMENT-015A unit-carrier model so simultaneously transported CategoryId units remain individually legible without changing observed routes, custody, mass, or frozen Oslo/front physics.
---

# SEDIMENT-015B — Hybrid active-grain micro-motion

## Entry condition

SEDIMENT-015A-R2 is machine-green at final native HEAD `0e791a8d10de838cd3b085a944ba4a973bc704b2`.

The foundation now proves:

```text
one active visual carrier = one CategoryId unit
only observed physical edges may be replayed
settle -> re-entry keeps the same ephemeral MotionId
final visual stack == authoritative CategoryId stack in exact order
visual presentation cannot fabricate physical completion
discharge leaves visually exactly once
frozen SEDIMENT-008 front physics is unchanged
```

SEDIMENT-015B therefore must not reopen custody or routing design. Its sole purpose is to make those already-causal unit carriers perceptually readable.

## Owner visual evidence

The SEDIMENT-014 rainbow-collapse human gate showed that mathematically conservative aggregate parcels still produced implausible color progression and coarse spatial redistribution. The owner explicitly permits spending materially more CPU on active transit and rejects optimizing the active layer as though Strata were CPU-constrained.

The goal is not decorative extra particles. Every rendered mobile dot must still correspond to one real SEDIMENT-015A unit carrier.

## Research basis

The accepted working direction follows the earlier research synthesis:

- Yue et al., *Hybrid Grains: Adaptive Coupling of Discrete and Continuum Simulations of Granular Media*, ACM TOG 37(6), 2018, DOI `10.1145/3272127.3275095`: retain discrete detail in the active region while compact representation remains appropriate elsewhere.
- Zhu & Bridson, *Animating Sand as a Fluid*, ACM TOG 24(3), 2005, DOI `10.1145/1073204.1073298`: hybrid particle/grid representations avoid requiring globally discrete simulation while preserving particle-like transport where visually useful.
- Position-based particle methods, including the local-separation approach exposed by SideFX POP Grains, motivate iterative positional constraints as a stable presentation mechanism. They do not become Strata physics authority.

SEDIMENT-015B uses only the narrow consequence relevant here: a few deterministic local positional-projection iterations may separate nearby presentation particles while a tether keeps each particle close to its already-authoritative route.

## Architecture

The SEDIMENT-015A carrier gains two coordinate layers:

```text
ideal_x / ideal_y
    causal replay track
    derived only from already-observed transport + exact pending settlement depth

x / y
    rendered micro-position
    may be displaced locally for legibility
```

The micro-position can never change:

- `MotionId`;
- `CategoryId`;
- queued or active observed edges;
- physical `Rolling / Settled / Discharged` state;
- custody stack;
- settlement order;
- discharge accounting;
- Oslo thresholds, RNG, front rules, rain/focus, or persistence.

### Exact pending-depth approach

SEDIMENT-015A correctly preserved stack order but its simple renderer targeted the current shadow height for every physically settled pending carrier. Multiple pending grains in one column could therefore visually converge on the same next-visible row even though their authoritative custody depths were distinct.

SEDIMENT-015B derives the final presentation target directly from:

```text
flowviz_unit_custody[site][depth] == Some(MotionId)
```

so two pending units at different depths have different final `y` targets before either is revealed. Depths above the visible top remain distinct negative presentation coordinates rather than saturating onto row 0; they are clipped by rendering, not collapsed semantically. The lower pending unit still gates reveal of upper units; exact stack semantics are unchanged.

### Frozen segment target

While micro-motion is enabled, each observed segment freezes its presentation endpoint when replay begins. Later shadow changes do not bend an already-active segment underneath the carrier. This is presentation geometry only: the source/destination lattice edge remains the exact SEDIMENT-015A observed edge.

The non-micro SEDIMENT-015A constructor remains test-only as a semantic control and retains its original interpolation behavior.

## Local separation constraint

Only carriers that are actively replaying, have queued observed history, or are still physically Rolling participate.

Each visual update performs four deterministic local iterations:

1. bucket mobile carriers by dot-space cell;
2. inspect only the 3x3 neighboring bucket region;
3. for pairs closer than `0.92` dot, apply symmetric positional separation;
4. tether each corrected position to within `0.68` dot of its causal ideal position;
5. clamp horizontal displacement to the current observed-edge corridor plus `0.34` dot margin.

Exact overlaps use a deterministic MotionId-derived direction. No physics RNG and no new presentation RNG are consumed.

This is deliberately a PBD-inspired positional projection, not DEM contact dynamics. It cannot choose a route or create a new physical event.

## Raster occupancy

Continuous separation alone does not guarantee that two positions round to distinct Braille-dot cells. A second presentation-only raster placement step therefore assigns each visible carrier a unique nearby dot where local capacity exists.

Rules:

- never merge carrier mass;
- never coalesce CategoryIds;
- prefer cells not occupied by settled shadow or falling rain;
- never place two mobile carriers in the same raster cell;
- remain on the current observed edge's source/destination columns;
- search vertically only within a bounded local radius;
- if no shadow-free local cell exists, mobile visibility may overlay static display before two mobile units are collapsed together;
- offscreen egress is not pulled back into the viewport.

The raster choice has zero authority over carrier movement or arrival.

## Model exposure

The existing experimental command remains:

```text
testingcheats model oslo-vessel-front-grains
```

but now instantiates the 015B micro-motion constructor. The machine-green 015A constructor remains available only to embedded tests as the control.

No additional user-facing testing model is added.

## Performance stance

SEDIMENT-015B intentionally spends more CPU than SEDIMENT-015A. The local solver uses spatial buckets rather than an all-pairs global scan, but no quality degradation or coalescing policy is introduced here.

SEDIMENT-015C remains responsible for measurement and any adaptive presentation clock/quality policy. If later profiling proves a limit, micro-iteration fidelity may degrade before unit accounting or route fidelity is compromised.

## Required native gates

Before human review:

1. 015A semantic suite remains green unchanged.
2. Frozen-front vs 015B ordinary-drive physics parity remains exact.
3. 015B drains to exact `shadow == columns` with zero unit misses.
4. Coincident carriers separate measurably while remaining inside the observed-edge corridor.
5. Dense local carrier rasterization preserves one visible cell per carrier where bounded local capacity exists.
6. Rendered CategoryId multiset equals the visible carrier CategoryId multiset in that dense fixture.
7. Repeated micro relaxation does not mutate columns, thresholds, RNG, edge queue, sequence, or physical state.
8. Resize regressions remain green with ideal/active endpoint coordinates projected consistently.
9. Legacy SEDIMENT-014 conservative tests remain green.
10. `cargo fmt`, strict Clippy, full tests, and help smoke pass.

## Human gate

Only after machine green:

```text
testingcheats model oslo-vessel-front-grains
testingcheats fill
testingcheats fallspeed 64x
```

Observe at least one broad rainbow collapse and perform:

```text
wide -> narrow -> brief hold -> wide
```

The owner judges only the transport/render goal:

> Do the number, colors, spatial progression, and arrival of visible moving dots now plausibly explain the colored sediment that appears, without obvious clumping, bottom spawning, or category-band reconstruction artifacts?

Do not tune Oslo slope/front/rain behavior from this gate.

## Explicitly deferred to SEDIMENT-015C

- presentation-clock rate above the historical 64 ms effective Oslo cadence;
- adaptive time compression under large visual backlog;
- measured 5k/10k/20k/40k active-carrier performance tiers;
- reducing micro iterations under demonstrated load.

Those changes require benchmark evidence rather than assumption.
