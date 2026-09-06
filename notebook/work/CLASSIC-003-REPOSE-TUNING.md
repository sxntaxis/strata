---
id: CLASSIC-003
kind: work
state: superseded
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Add a debug-only live tuning control for Classic's local high-repose probability so the owner can compare 5/10/20/30 percent texture on the same rainbow fixture without changing Classic physics, rendering, persistence, or the accepted 5% default.
---

# CLASSIC-003 — Repose tuning cheat

## Owner decision

CLASSIC-002 materially improved the final exposed slope without harming the simple Classic avalanche, but the owner wants more relief texture and prefers a live `testingcheats` control before freezing a stronger production candidate.

The accepted tuning interface for this work unit is:

```text
testingcheats classic repose
testingcheats classic repose <0..100>
testingcheats classic repose default
```

Typical comparison sequence:

```text
testingcheats classic repose 10
testingcheats fill

testingcheats classic repose 20
testingcheats fill

testingcheats classic repose 30
testingcheats fill
```

## Semantics

- `classic repose` reports the current probability that a newly sampled Classic column receives `local_repose=2`.
- `classic repose N` changes only the probability used by **future repose resamples**.
- Existing live `local_repose` cells are not rewritten by changing the percentage.
- `testingcheats fill` and `clear` rebuild the local repose field using the selected percentage, so they provide clean A/B fixtures.
- `classic repose default` restores the CLASSIC-002 default of 5%, again without retroactively rewriting live columns.
- The command is available only while the testing sandbox is Classic/Hybrid; other models reject it.
- The setting is debug/testing-only and is not persisted.

## Preservation boundary

The 5% default must preserve CLASSIC-002's exact one-in-twenty sample mapping. Adding tunability must not silently change the owner's current baseline before a different percentage is explicitly selected.

No changes are authorized to:

- Classic gravity or diagonal-choice law;
- repose values themselves (`{1,2}` only);
- when source/destination columns resample after successful diagonal motion;
- Classic/Hybrid render path;
- rainbow fill geometry or Fixture category provisioning;
- resize semantics;
- production SandEngine;
- persistence/schema;
- Oslo/SEDIMENT behavior.

## Required native evidence

1. Parser accepts query, `0..100`, and `default`, and rejects invalid/out-of-range values.
2. Default Classic sandbox reports 5%.
3. Setting the percentage does not mutate the existing repose field.
4. `fill`/`clear` after 100% produces only repose 2; after 0% only repose 1.
5. Fixed seed + same tuned percentage remains deterministic.
6. Reset restores 5%.
7. Existing CLASSIC-002 roughness/macroform tests remain green unchanged.
8. Existing CLASSIC-001 fill/category/wall/resize tests remain green.
9. fmt, strict Clippy, full tests, help smoke and command parser pass.

## Human gate

After machine green, compare at least 10%, 20%, and 30% using a fresh `testingcheats fill` after each setting. Pick the smallest percentage that gives visibly richer relief while preserving the simple Classic macro avalanche.

## Supersession

Owner testing found the accepted 5% CLASSIC-002 baseline better than the uniform percentage sweep, including 100%. CLASSIC-004 supersedes this numeric tuning surface with four preplanned texture profiles so testing can vary short-range heterogeneity and rare repose=3 sites without exposing multiple low-level parameters.
