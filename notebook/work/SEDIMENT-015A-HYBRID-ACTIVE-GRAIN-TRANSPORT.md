---
id: SEDIMENT-015A
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Replace SEDIMENT-014 weighted/fungible transit only in a new debug model with one ephemeral visual carrier per physically active CategoryId unit, replaying only already-observed adjacent physical transport and revealing settled mass in exact physical stack order. Frozen SEDIMENT-008 front physics remains authoritative and untouched.
---

# SEDIMENT-015A — Hybrid active-grain transport

## Trigger

SEDIMENT-014R3 is machine-green: the conservative parcel implementation closes its custody misses, preserves total and CategoryId mass, drains re-entry and wall-failure fixtures, and leaves frozen front physics unchanged.

The owner human gate rejects the resulting presentation, however. The rainbow collapse remains visually implausible even though its accounting is correct: CategoryId regions develop with coarse/strange progression and the visible transit does not explain the spatial/color distribution that subsequently appears settled.

This is a representation failure, not a request to tune SEDIMENT-008 physics.

SEDIMENT-014 therefore ends as:

```text
machine conservation/custody: PASS
human aggregate-transport fidelity: FAIL
```

Do not add another parcel-reconciliation R4.

## Research basis

The bounded design follows a hybrid discrete/continuum principle rather than full discrete-element simulation.

- Yue et al., *Hybrid Grains: Adaptive Coupling of Discrete and Continuum Simulations of Granular Media*, ACM TOG 37(6), 2018, DOI `10.1145/3272127.3275095`: use detailed discrete representation where necessary and computationally cheaper continuum representation where safe, with explicit coupling during transitions.
- Zhu & Bridson, *Animating Sand as a Fluid*, ACM TOG 24(3), 2005, DOI `10.1145/1073204.1073298`: large granular volumes need not be represented as a globally discrete grain simulation; hybrid/continuum representations can retain particle-like tracking where useful.
- SideFX Houdini POP Grains documents PBD as a stable local particle-separation mechanism. That is inspiration for a later presentation-only SEDIMENT-015B micro-motion layer, not authority for SEDIMENT-015A routing or physics.

Strata's natural partition is narrower than those general simulators:

```text
settled pile                         active/pending visual transport
compact authoritative columns       ephemeral unit carriers
```

The frozen Oslo/front solver remains the only authority deciding when and where sediment moves.

## Non-goal: full DEM

SEDIMENT-015A does **not** give every settled grain a permanent simulated particle identity.

It does not add contact physics, force integration, collision authority, or particle-chosen destinations.

A settled column remains the compact CategoryId stack already used by frozen SEDIMENT-008.

A unit receives presentation identity only while its physical movement is not yet fully explained by the display.

## New debug model

```text
testingcheats model oslo-vessel-front-grains
```

This is a sibling experiment. Existing models remain available and unchanged:

```text
oslo-vessel-front
oslo-vessel-front-flowviz
oslo-vessel-front-parcels
```

## Hard semantic contract

### 1. One active carrier equals one unit of mass

There is no `mass` field on a unit carrier.

```text
one FlowVizUnitCarrier == one CategoryId unit
```

No coalescing is allowed in the SEDIMENT-015 path.

### 2. Motion identity is ephemeral presentation custody

`FlowVizMotionId` exists only while a unit is active, discharged-but-still-visible, or physically settled but not yet reconciled into the visible settled shadow.

It is not:

- persistent grain identity;
- provenance identity;
- storage identity;
- physics authority;
- RNG input;
- a future destination token.

Once the unit is visibly reconciled into settled shadow or visibly exits the system, the ID is destroyed.

### 3. Visual routes contain only observed physical history

A carrier segment may be appended only after the authoritative solver has selected/performed that unit adjacent transfer.

```text
physical source -> destination event
then
append visual source -> destination segment
```

No future route is precomputed.

No carrier may choose its own neighboring site.

### 4. Re-entry continues the same ephemeral unit

If physical mass settles while its carrier is still visually pending, the aligned settled stack entry stores its `MotionId` as temporary visual custody.

If that same top stack entry re-enters physical motion before visual reconciliation, the custody is popped with it and the same carrier continues.

No same-color nearest-surplus search is needed in the unit model.

### 5. Settled reconciliation preserves exact stack order

The SEDIMENT-014 parcel contract eventually required CategoryId multiset reconciliation per site. SEDIMENT-015A is stronger.

After visual drain:

```text
flowviz_shadow_columns == columns
```

by vector equality, including CategoryId order.

Pending visual custody is aligned one-for-one with authoritative settled stack depth. A physically settled carrier can be revealed only when its exact depth is the next unrevealed depth for that column.

### 6. Discharge remains visible custody

A physically discharged carrier is still part of visual mass until its already-observed egress segment completes. It is then removed exactly once.

### 7. Presentation cannot affect frozen physics

Turning unit FlowViz on/off must not change:

- columns;
- thresholds;
- threshold or relaxation RNG state;
- front seed/erosion/support-loss/coast/runout;
- boundary/discharge decisions;
- rain/focus;
- avalanche move counts;
- persistence/schema.

## Representation

Conceptually:

```rust
FlowVizMotionId(u64)

FlowVizUnitCarrier {
    id,
    category_id,
    x,
    y,
    queued_observed_segments,
    active_segment,
    physical_state,
    arrived,
}
```

`physical_state` is presentation bookkeeping about authoritative custody:

```text
Rolling
Settled(site)
Discharged
```

It does not replace authoritative `RollingGrain` or `columns`.

## Custody alignment

SEDIMENT-015A adds a presentation-only stack parallel to `columns`:

```text
flowviz_unit_custody[site][depth]
    None
    or Some(MotionId)
```

Every authoritative push/pop in the experimental model keeps this stack shape aligned with the CategoryId stack.

A `None` depth is already represented by settled shadow. A `Some(id)` depth is physically settled but still represented by that unit carrier.

Normal falling rain that lands above an unrevealed suffix is adopted as an already-arrived unit carrier instead of being painted immediately above hidden mass. This preserves ordered reveal without changing drive/physics timing.

## Visual clock in 015A

SEDIMENT-015A deliberately uses simple deterministic constrained interpolation over observed segments.

It does **not** yet implement local particle separation/PBD. Multiple carriers may therefore visually overlap during this foundation increment.

That is accepted for the 015A machine gate. Human aesthetic acceptance is deferred to SEDIMENT-015B, whose job is to spend the available CPU on local separation and granular micro-motion without changing carrier routes or mass accounting.

## Resize contract

The existing canonical/visible-basin resize authority remains unchanged.

When the viewport grows, unit shadow/custody and active coordinates shift with the same canonical lattice shift as existing transient presentation state.

When it shrinks, only active rolling presentation is projected into the visible window, matching the existing transient rolling-grain resize contract. Settled canonical custody remains at its authoritative site for later re-expansion.

A wide -> narrow -> wide round trip must preserve unit/category mass, MotionId custody, and exact eventual stack reconciliation.

## Required invariants/tests

SEDIMENT-015A is not accepted by a screenshot alone. Native tests must prove:

1. **Frozen ordinary drive parity** — deterministic front control and unit model produce identical authoritative physics.
2. **Unit mass conservation** — `shadow units + carriers == settled + rolling + visually-pending egress`.
3. **Category conservation during transit** — visual shadow+carrier CategoryId counts equal authoritative settled+rolling+pending-egress CategoryId counts at every checked visual step.
4. **No future prediction** — a carrier contains only segments explicitly appended after observed physical events.
5. **Re-entry continuity** — settle -> re-enter before arrival keeps exactly one MotionId and appends the next real edge.
6. **Exact stack reconciliation** — after drain, `shadow == columns`, not merely equal multisets.
7. **Wall-failure concurrency** — a broad frozen-front failure creates many simultaneous unit carriers, replays more observed segments than carrier peak, preserves physics, and drains exactly.
8. **Resize round trip** — active unit custody survives shrink/re-expand and drains to exact stack.
9. **Legacy conservative regression** — SEDIMENT-014 R1-R3 tests stay green.
10. **Repository gates** — fmt, strict Clippy, full tests, help smoke.

## Acceptance boundary

SEDIMENT-015A is a semantic/machine foundation only.

If machine-green, do **not** yet claim that the owner's color/progression complaint is solved. Proceed to SEDIMENT-015B for visible unit separation/micro-motion, then perform the human rainbow A/B.

## Frozen authority

Unchanged and byte/diff-audited where applicable:

- production H4/v5;
- SEDIMENT-008 `oslo-vessel-front` physics constants and rules;
- Oslo thresholds `{1,2}`;
- rain/focus;
- canonical/vessel boundary semantics;
- persistence/schema;
- scheduler authority except existing generic visual-flow backpressure behavior;
- SEDIMENT-014 parcel model behavior in its own debug model.

## R1 native finding and R2 gate correction

The first certified-base native run reached the unit ordinary-drive gate after compilation and the initial focused custody tests had passed, then stopped at:

```text
unit_flowviz_preserves_frozen_front_physics_and_exact_stack_on_ordinary_drive
unit visual transport failed to drain
```

Static reconstruction against the exact failing candidate shows that this was not evidence that a physically settled carrier could not reconcile. The test reused `direct_drive_and_relax`, a helper that drains only ordinary active-site topples. `oslo-vessel-front` can legitimately create an authoritative `RollingGrain` on a severe failure. Once that occurs, the helper returns while explicit physical flow is still active, but the test then advances only the visual clock and waits for all unit carriers to disappear.

That expectation is invalid for the SEDIMENT-015 contract. A carrier whose authoritative state is still `Rolling` must **not** invent a settlement or egress merely because its currently observed visual segments have finished replaying. Presentation must wait for the next physical fact.

SEDIMENT-015A-R2 therefore changes no runtime source. It repairs the gate so each deterministic drive first reaches complete front-model physical quiescence in both the frozen control and the unit-flowviz sibling by advancing rolling grains plus active-site topples. Exact authoritative parity is checked at that boundary. Only then is presentation drained and exact stack equality required.

A dedicated anti-fabrication regression also freezes the complementary rule:

```text
physical RollingGrain still exists
+ its only observed visual edge has completed
=> carrier remains Rolling and alive
=> visual clock may not synthesize settlement/discharge
```

This is a strengthening of the acceptance test, not a relaxation: R1 could compare only the direct-topple prefix before explicit front flow finished; R2 compares the fully quiescent authoritative front state on every drive, then separately proves exact visual drain.
