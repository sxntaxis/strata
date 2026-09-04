---
id: SEDIMENT-010
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Preserve SEDIMENT-008 front and SEDIMENT-009 fluid physics while eliminating category teleport in explicit rolling transport and separating accelerated drive time from stable avalanche playback.
---

# SEDIMENT-010 — explicit grain transport + three-clock debug harness

## Owner evidence entering this unit

The SEDIMENT-008 front behavior remains the strongest accepted avalanche model. The 80%-VW category fill then exposed two infrastructure defects that were difficult to see with a single color:

1. category colors appeared to **teleport vertically** during a wall failure; and
2. the SEDIMENT-009 fluid curiosity fork could appear to freeze while host CPU usage remained low.

These are not evidence to retune the accepted front thresholds or the fluid order-field constants.

## Root cause A — `RollingGrain` had no vertical position

The explicit moving phase stored only:

```text
site/x
CategoryId
direction
coast budget
```

After erosion or support-loss recruitment, physics moved a grain from a tall source column to the adjacent lower destination site. Rendering then placed that grain immediately above the destination bed. A large relief therefore produced a one-frame vertical discontinuity even though lateral motion remained one site at a time.

The rainbow fixture made this visible as a category band apparently appearing many rows lower without occupying the intervening cells.

## Repair A — presentation elevation without changing sediment decisions

`RollingGrain` now also owns a transient `visual_y`.

For every severe seed, front erosion, support-loss recruitment, and fluid release:

1. capture the actual source top-cell elevation before popping the CategoryId;
2. perform the same existing physical mass transfer into the rolling queue;
3. retain that source elevation as the rolling dot's presentation position;
4. pause the next sediment event while the dot advances vertically by one visible row toward the already-computed destination surface.

The physical `site`, direction, thresholds, erosion, support-loss recruitment, fluid exchange, and eventual settled result are unchanged. The added substeps are presentation interpolation only.

A physics hop still changes at most one lattice x-site. Its `visual_y` remains where it was, then converges one dot at a time to the new bed before another sediment event can occur in the testing harness.

This makes CategoryId provenance spatially legible without fabricating or duplicating grain mass.

## Root cause B — accelerated drive debt and visual flow shared one queue

SEDIMENT-009 deliberately limited explicit flow to a wall-clock cadence, but `fallspeed 64x/128x` continued adding accelerated synthetic time while that flow was being consumed near 1x.

Example at 64x:

```text
32 ms wall time
→ +2048 ms synthetic debt
→ roughly one 32 ms flow event consumed
```

The UI was therefore not resource-starved. It was intentionally idle between visual ticks while a large synthetic backlog accumulated faster than it could be drained. A fluidity field with no currently visible rolling grain could keep that throttle active and make the sandbox look frozen despite low CPU use.

## Repair B — three clocks

The debug harness now separates:

```text
DRIVE CLOCK
rain / fallspeed / queued advance

PHYSICS CLOCK
ordered Oslo/front/fluid sediment events

VISUAL GRAIN CLOCK
falling and explicit rolling-dot presentation
```

### Ordinary fast-forward

When no explicit flow exists, the proven cooperative 4 ms CPU-budget path remains. `fallspeed` and `advance` use available CPU to drain synthetic time.

### Explicit avalanche

As soon as front/fluid flow exists:

- preserve already-requested queued synthetic time;
- stop adding accelerated wall time to that debt;
- pause new drive/fast-forward progression;
- play visible sediment events on the wall-clock physics cadence;
- resume the preserved synthetic debt only after the event reaches quiescence.

This follows the slow-drive / relax-to-quiescence separation already inherent in Oslo rather than allowing accelerated rain to outrun an avalanche being deliberately shown in real time.

### Rolling interpolation

Vertical rolling transport advances one dot every two 32 ms internal frames (64 ms), matching the existing Oslo falling-dot vertical cadence. Extra elapsed wall time is discarded rather than replayed as a burst; a delayed UI frame therefore cannot make the avalanche suddenly accelerate to catch up.

### Latent fluid field

If fluidity remains active but there is no rolling grain and no falling dot to display, the invisible order field is drained cooperatively inside the ordinary CPU budget until it either:

- releases visible mass, at which point the stable visual clock takes over from a fresh wall-clock epoch; or
- decays to zero.

Invisible order-parameter work therefore no longer forces a visually idle 32 ms-per-event crawl.

## Semantics-preserving fluid hot path

The fluid field previously called `rolling_depth_at(site)` while scanning every visible site, which rescanned the rolling queue each time. SEDIMENT-010 builds one per-site rolling-depth array before the scan, reducing this part from roughly `width × rolling_count` work to `width + rolling_count` without changing decisions.

## Frozen physics boundaries

SEDIMENT-010 must not tune SEDIMENT-008 or SEDIMENT-009 physics. In particular it leaves unchanged:

- Oslo `{1,2}` thresholds;
- vessel/canonical boundaries;
- severe seed relief `4`;
- front erosion relief `2`;
- support-loss relief `3`;
- coast/runout constants;
- rain/focus;
- fluidity `0..8`, seed/spread/release/start/stop thresholds;
- CategoryId mass transfer and conservation.

The only new grain state is a transient presentation elevation for already-moving mass.

## Machine gates

Native validation must prove:

1. all SEDIMENT-008 and SEDIMENT-009 focused tests remain green;
2. a severe front seed keeps the source CategoryId at source elevation first, then descends exactly one visible row per presentation step;
3. a physical rolling hop changes one x-site without an accompanying vertical teleport;
4. interpolation does not change grain count, final CategoryId, thresholds, RNG, discharge, or wall semantics;
5. during explicit flow, accelerated wall time does not grow queued synthetic debt;
6. a visible flow never consumes presentation backlog in a burst after a delayed frame;
7. latent fluid-only work can use the cooperative CPU budget and hands off to visible playback with zero inherited presentation debt;
8. the rainbow fill remains exactly 80% of the current VW;
9. formatting, strict Clippy, full tests, and help smoke pass.

## Human gate

Use the same rainbow fixture for both models:

```text
testingcheats model oslo-vessel-front
testingcheats fill
```

and then:

```text
testingcheats model oslo-vessel-fluid
testingcheats fill
```

For each, shrink horizontally and re-expand.

Judge:

- CategoryId bands should visibly travel through the collapse rather than appear many rows lower in one frame.
- Front morphology should retain the owner-accepted SEDIMENT-008 character.
- Avalanche speed should remain steady even when the event becomes computationally cheap or expensive.
- `fallspeed 64x/128x` must not create a growing post-avalanche backlog while the visible event is playing.
- Fluid-only pauses should be brief/internal; if fluidity later releases mass, visible motion should resume immediately at the normal avalanche clock.

No production promotion follows automatically.
