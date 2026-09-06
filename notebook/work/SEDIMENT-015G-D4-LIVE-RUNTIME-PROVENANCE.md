---
id: SEDIMENT-015G-D4
kind: work
state: candidate
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Stop synthetic rainbow-trigger guessing and add a bounded debug-only provenance observer to the owner-preferred SEDIMENT-015G-R2 runtime so the exact human fill/resize/rain/seed history can answer green/cyan transport questions without changing physics or presentation.
---

# SEDIMENT-015G-D4 — live runtime rainbow provenance

## Why D4 replaces D2/D3

D1 proved the observer shape but did not trigger the owner-visible collapse. D2 added direct diagnostic ingress and D3 reproduced the accelerated testingcheats scheduler at a synthetic `40x16` test viewport and fixed test seed. Both still produced zero fixture-layer hops, while the owner can reproduce a large collapse interactively.

The remaining mismatch is therefore environmental/history-specific rather than justification for another synthetic trigger. The human sandbox is created with the live terminal dimensions and the current authoritative sediment RNG state; it may also be resized during the run. D4 records the provenance from that exact owner run instead of guessing those conditions in another test.

## Authority boundary

Start from the machine-green D1 return bundle `da2326ac94b4676560da6d177ecf8afeceef1b69`, whose runtime is SEDIMENT-015G-R2 plus idempotent Fixture-category provisioning.

D4 must not change:

- Oslo thresholds, RNG decisions, relief, front trigger, erosion/support-loss, coast/runout;
- rolling-grain physics;
- visual shadow or custody semantics;
- 015G final-batch lane geometry;
- coherent chronology or scheduler;
- truthful raster policy;
- rain policy;
- settlement/reveal behavior;
- fallspeed behavior;
- persistence schema.

The only runtime addition is debug-only observation and the `testingcheats provenance` read command.

## Live trace contract

`testingcheats fill` already creates/reuses the six persisted Fixture categories. D4 additionally resets a bounded provenance session for those exact CategoryIds.

For fixture-category MotionIds only, the observer records facts after the existing runtime has already decided them:

- first source site;
- every already-observed adjacent source→destination segment;
- hop count and maximum horizontal displacement;
- final settled site/depth or discharge;
- initial Fixture mass and original filled footprint;
- starting sandbox cell/grid dimensions and threshold RNG state.

The observer must never feed a value back into transport or presentation.

## Owner command

After reproducing the normal rainbow collapse in the exact interactive sandbox, run:

```text
testingcheats provenance
```

The command writes the report to `$XDG_CACHE_HOME/strata/rainbow-provenance.txt` (falling back to `$HOME/.cache/strata/rainbow-provenance.txt`) and returns that path. The report must include:

- `RAINBOW_LIVE_ENV` — exact initial cell/grid dimensions and RNG state, plus current dimensions;
- six `RAINBOW_TRACE` lines;
- `RAINBOW_GREEN_MASS`;
- `RAINBOW_CYAN_LOW`;
- up to six `RAINBOW_CYAN_LOW_PATH` lines when cyan actually settles below the initial green-band height.

This is the evidence needed to distinguish:

1. real physical cyan transport that presentation merely makes hard to follow;
2. a remaining presentation jump unsupported by adjacent physical edges;
3. a mostly static basal green core that is genuinely produced by frozen Oslo-front physics.

## Memory bound

Trace collection is debug-only and restricted to the six Fixture CategoryIds. Each MotionId stores counters plus at most 4096 adjacent directions. This is diagnostic evidence, not persistent grain identity and not production state.

## Validation

Native validation must prove:

- D4 starts exactly from the machine-green D1 return base;
- production/release build behavior is unchanged (`cfg(debug_assertions)` observer only);
- the command parses and is available only in debug testingcheats;
- `fill` resets the live observer to the exact current Fixture IDs;
- an adjacent unit segment appears in the report without changing authoritative columns;
- no physics/presentation files other than observation hooks change semantics;
- all SEDIMENT-015G-R2/D1 foundation gates and the full repository checks remain green.

No synthetic `RAINBOW_TRIGGER` assertion and no human visual acceptance gate belong to D4.
