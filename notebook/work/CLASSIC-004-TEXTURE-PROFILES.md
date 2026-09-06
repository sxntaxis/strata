---
id: CLASSIC-004
kind: work
state: candidate
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Replace the rejected numeric repose-percentage tuning cheat with four fixed Classic texture profiles that preserve the accepted 5% baseline and test whether rare repose=3 sites plus short spatial correlation can add mesoscopic relief without changing Classic's avalanche law.
---

# CLASSIC-004 — Texture profiles

## Owner decision

The owner found the original CLASSIC-002 5% baseline better than the subsequent numeric percentage sweep. Uniformly increasing the share of repose=2 columns tends to recover another smooth, nearly linear surface instead of the desired heterogeneous relief.

The testing surface is therefore simplified to four named presets:

```text
testingcheats classic texture baseline
testingcheats classic texture textured
testingcheats classic texture rugged
testingcheats classic texture terraced
```

`testingcheats classic texture` reports the active preset. The old numeric `testingcheats classic repose ...` command is retired from the parser.

## Presets

### baseline

Exact CLASSIC-002 behavior:

- 95% repose 1 / 5% repose 2;
- exact historical one-in-twenty RNG mapping;
- no spatial continuation/correlation;
- no repose 3.

This is the accepted control and remains the default.

### textured

Target distribution before local continuation:

- 93% repose 1;
- 6% repose 2;
- 1% repose 3;
- 45% short continuation probability.

Intent: a small amount of coherent local texture with minimal macroform change.

### rugged

Target distribution before local continuation:

- 90% repose 1;
- 8% repose 2;
- 2% repose 3;
- 60% short continuation probability.

Intent: stronger shoulders and short local terraces while preserving the same Classic fall/one-diagonal law.

### terraced

Same base distribution as rugged, but 78% continuation probability.

Intent: isolate whether longer coherent patches, rather than simply more high-repose sites, create the preferred mesoscopic relief.

## Physics boundary

The only additional physical value is `local_repose=3`. Classic remains:

```text
vertical fall if free
otherwise choose exactly one random diagonal
move only if that chosen side is free
and source local repose allows release
```

Repose 2 requires one extra empty cell below the ordinary diagonal target. Repose 3 requires two. There is no second-side retry, rolling layer, front solver, momentum, visual shadow, replay, or Oslo toppling.

Profile selection does not rewrite the live repose field. `testingcheats fill` or `clear` resamples a clean field under the selected profile. Runtime profile selection is debug-only and not persisted.

## Required evidence

- `baseline` preserves CLASSIC-002's exact deterministic one-in-twenty mapping.
- repose 3 blocks only the additional marginal-relief case beyond repose 2.
- all four presets are deterministic for a fixed seed.
- profile selection is nonretroactive and `fill` resamples cleanly.
- mass conservation is exact for all profiles.
- apex and runout remain close to baseline under the compact-wall fixture.
- at least one non-baseline profile materially increases exposed-profile roughness over baseline.
- Classic/Hybrid gravity parity, fill/category provisioning, closed walls, resize, fmt, strict Clippy, full tests, help smoke, and command parser remain green.

## Human gate

After machine green, compare only:

```text
baseline → fill
textured → fill
rugged → fill
terraced → fill
```

Choose the preset with the strongest useful relief that still preserves the owner-preferred Classic avalanche. Do not expose or tune the underlying percentages interactively.
