---
id: SEDIMENT-015E
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Replace 015D's independently accumulating per-carrier replay with a lockstep coherent presentation barrier: one complete authoritative Oslo quantum may emit at most one hop per rolling grain, and no later quantum may run until that already-observed visual batch has caught up.
---

# SEDIMENT-015E — Coherent temporal replay

## Entry condition

SEDIMENT-015D is machine-green and restored real-time raster throughput plus live baseline rain, but the owner rejected the human visual gate.

Observed evidence:

```text
- performance is now acceptable enough to observe;
- mobile colors form implausible ribbons, arcs, and hollow-looking channels;
- final stack correctness does not explain intermediate color placement;
- the unit-grain direction is still preferable, but temporal assembly is not credible.
```

The 015D human result is:

```text
FAIL_INCOHERENT_TEMPORAL_REPLAY
```

## Source-level diagnosis

015D preserves each unit's CategoryId, MotionId, observed adjacent edge, exact final stack, and truthful raster location, but authoritative physics is allowed to keep generating more hops while earlier visual hops remain queued.

Therefore a rendered frame can combine:

```text
settled shadow after later authoritative mutations
+
carrier A replaying an older hop
+
carrier B replaying a newer hop
+
carrier C already caught up
```

Every individual fact can be true while the combined frame never existed as one coherent stage of the avalanche.

## Implementation refinement: lockstep, not a durable event journal

Initial design exploration considered a delayed global event journal. Current source inspection provides a simpler and stronger mechanism.

`advance_rolling_grains()` already guarantees:

```text
every currently mobile grain advances by at most one lattice site per authoritative rolling update
```

Newly recruited grains are excluded from that update's `active_at_start` and cannot hop again until a later authoritative quantum.

Therefore 015E does not need to duplicate Oslo state into a second durable event machine. It establishes a presentation barrier around the existing authoritative quantum:

```text
authoritative quantum N
    - falling presentation advances
    - fluidity physics advances
    - each existing rolling grain: at most one hop/settle
    - at most one active-site topple/recruit operation
        ↓
complete observed visual batch N
        ↓
unit carriers replay that batch concurrently
        ↓
NO authoritative quantum N+1 until batch N is caught up
```

This is a global *batch* barrier, not the rejected historical one-grain barrier. Thousands of grains in one authoritative quantum remain concurrent. The testing scheduler also permits at most one coherent authoritative/visual batch per app render pass, so accelerated wall-clock debt cannot silently consume several fully drained physical epochs before the terminal gets a chance to display the intermediate state.

## Coherent batch debt

A carrier blocks the next physics quantum only when it has actual presentation debt:

```text
queued observed segment
or
active observed segment
or
physical settlement still travelling to its reveal row
or
discharge egress still travelling
```

A physical `Rolling` carrier with no queued/active observed segment is considered caught up and does **not** block the next Oslo quantum merely because the grain remains mobile.

## Ordering inside a coherent frame

015E performs the complete authoritative quantum before advancing its visual interpolation:

```text
falling presentation
→ fluidity physics
→ rolling physics
→ active-site topple
→ presentation interpolation
```

015D's control frame remains unchanged. The new ordering prevents visual reveal/custody cleanup from occurring in the middle of an authoritative quantum that can still recruit another grain.

## Presentation throughput

015D's O(N) truthful raster remains unchanged.

For 015E, presentation substep budget rises only enough to finish an ordinary one-hop segment inside one cooperative wall-clock budget when CPU permits:

```text
small/medium batch: 6 substeps
large batch: 8 substeps
```

This compresses presentation time, not mass, CategoryId, edges, or physical decisions.

## Rain

015D live-rain semantics remain unchanged:

- baseline wall-clock rain continues while a coherent batch is replaying;
- falling dots may continue descending;
- commit into the pile remains forbidden until authoritative flow is quiescent;
- 64x does not multiply rain frequency.

## Frozen authority

015E must not change:

- thresholds `{1,2}`;
- front erosion/support-loss/coast/runout;
- physical rolling direction/site decisions;
- physical CategoryId custody;
- MotionId one-unit semantics;
- adjacent-edge-only transport;
- exact ordered settlement;
- truthful 015D raster;
- persistence/schema;
- canonical production profile.

No physics or momentum source file is changed by 015E.

## Required native evidence

1. New coherent constructor extends 015D with no physics/RNG delta.
2. A queued/active observed segment blocks a later authoritative hop.
3. A caught-up physical rolling carrier releases the barrier without waiting for physical settlement.
4. Repeated coherent flow calls cannot accumulate a second physical epoch while visual debt remains.
5. Shadow + unit-carrier mass remains exact throughout batch interpolation.
6. Lockstep drive sequence preserves frozen-front columns, thresholds, RNG, discharge, avalanche moves, and exact final stack.
7. The app scheduler exposes at most one coherent authoritative/visual batch per render pass, including final visual-only drain.
8. 015D regressions and 5k/10k/20k/40k raster scaling remain green; 015E must not reintroduce raster/PBD cost.
9. 015C/015B/015A/legacy regressions remain green.
10. fmt, strict Clippy, full tests, help smoke, and command parser pass.

## Human question

Only after machine green:

> Do rainbow collapses now read as one coherent moving state — without the long hollow ribbons/arcs caused by carriers replaying different physical epochs — while retaining 015D responsiveness and live rain?
