---
id: CLASSIC-005
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Freeze rugged as the owner-selected Classic texture baseline and expose six preplanned debug-only experiment profiles that add one bounded local mechanism at a time so the owner can find the last useful complexity step before Classic's avalanche character degrades.
---

# CLASSIC-005 — Complexity ladder

## Owner decision

`rugged` is now the preferred Classic texture baseline. The owner wants to test how much additional physical complexity can be added while retaining Classic's unusually strong simple avalanche behavior, but does not want manual multi-parameter cheats.

The experiment surface is therefore:

```text
testingcheats classic experiment rugged
testingcheats classic experiment memory
testingcheats classic experiment slope
testingcheats classic experiment memory-slope
testingcheats classic experiment anchored
testingcheats classic experiment momentum
```

`testingcheats classic experiment` reports the active profile. Every experiment uses the rugged texture field as its base and is non-retroactive; `testingcheats fill` starts a clean comparison.

## Profiles

### rugged

Owner-selected control. CLASSIC-004 rugged only: 90/8/2 repose `{1,2,3}` with 60% short spatial continuation. Original Classic vertical fall and one-random-diagonal rule remain otherwise exact.

### memory

Rugged plus short local repose memory. A resampled column keeps its current repose through the next three surface refresh attempts before becoming eligible to resample again.

Purpose: let local shoulders persist instead of immediately averaging away.

### slope

Rugged plus a mild steep-side preference only when both ordinary Classic diagonals are open. The steeper side is selected with 75% preference; ties and one-side-open cases preserve the existing random Classic choice.

Purpose: let local relief inform direction without adding a second-side retry or new mobile state.

### memory-slope

Rugged + the bounded three-refresh repose memory + the mild 75% steep-side preference.

This is the expected likely sweet spot and the last profile that changes only local settle/direction decisions.

### anchored

Memory-slope plus rare repose=4 sites. Distribution before short rugged patch continuation is approximately 90% repose 1, 8% repose 2, 1.5% repose 3, 0.5% repose 4.

Purpose: test whether rare stronger local anchors create useful shoulders without changing the avalanche mechanism.

### momentum

Anchored plus at most one bonus same-direction diagonal hop after an ordinary successful Classic diagonal, and only when that next diagonal still has at least two dots of open relief.

There is no persistent velocity, rolling grain, front state, second renderer, or multi-hop loop. This is intentionally the boundary experiment: if this degrades Classic, complexity stops at the preceding profile.

## Hard boundary

Still forbidden in this ladder:

- rolling grains or persistent mobile objects;
- front erosion/support-loss solvers;
- MotionId/custody/replay;
- visual shadow/presentation physics;
- second-side retry after a blocked Classic diagonal;
- persistence/schema changes;
- Oslo topology or threshold dynamics.

## Validation intent

Machine validation must prove command parsing, deterministic profile selection, bounded repose memory, steep-side bias only when both sides are open, rare repose=4 generation, one-hop-only momentum, exact mass conservation, existing fill/category provisioning, resize, closed walls, Classic/Hybrid controls, fmt, strict Clippy, full tests, and help smoke.

The human comparison is intentionally simple: select one profile, run `testingcheats fill`, observe the avalanche and final relief, then advance to the next. Freeze the last profile that still feels unmistakably like Classic.
