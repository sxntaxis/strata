---
id: CLASSIC-001
kind: work
state: candidate
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Return testingcheats to the simple Classic sandbox as the default experiment, make rainbow fill work directly in Classic with persisted Fixture categories, and preserve production sediment authority unchanged until owner dogfood justifies any promotion.
---

# CLASSIC-001 — Simple baseline

## Owner direction

After the Oslo/SEDIMENT visual experiments, the owner wants to stop iterating on increasingly complex experimental models and return to Classic, the simplest sandbox model. The immediate requested UX fix is that `testingcheats fill` must work in Classic and provision its fixture categories automatically.

This work deliberately does **not** promote Classic into production sediment authority yet. Accepted production SandEngine persistence, snapshots, recovery, daily artifacts, and schema remain untouched. Classic first becomes the default **testingcheats sandbox** so it can be dogfooded with the same bounded tooling used during the model experiments.

## Contract

1. When no testing sandbox is active, the first testingcheats command that needs one creates `classic`, not the H4 production clone.
2. `testingcheats fill` works for `classic` and `hybrid` as well as the existing Oslo sandboxes.
3. Fill remains isolated testing sediment, while the six Fixture category records are created/reused idempotently and persisted through the normal category path:
   - Fixture Green
   - Fixture Yellow
   - Fixture Red
   - Fixture Purple
   - Fixture Blue
   - Fixture Cyan
4. Classic fill builds a bottom-anchored rectangular rainbow occupying 80% of the **current visible** dot height and the full current visible width.
5. Layer order is bottom-to-top in the category order supplied by the fixture helper; no pending grains are introduced by fill.
6. Fill resets Classic sandbox movement counters and pending ingress so the fixture starts from a clean measurement boundary.
7. Re-running fill is deterministic with respect to the fixture shape/category layout.
8. Existing Classic physics is frozen for CLASSIC-001: straight-down gravity, single randomly chosen diagonal when blocked, closed visible walls, monotonic canonical growth, and dormant hidden terrain are unchanged.
9. Existing production `SandEngine` behavior, persistence/schema, Oslo source, and historical artifact semantics are unchanged.
10. Fallspeed/advance remain available for now but are not part of the Classic design target; `fill` should make high acceleration unnecessary for ordinary owner inspection.

## Why production promotion is deferred

`ClassicSandboxEngine` is currently a debug-only experiment and is intentionally not a persisted sediment authority. Promoting it directly into normal runtime would cross persistence, checkpoint, recovery, historical-artifact, and accepted architecture boundaries. That is a separate owner decision after Classic dogfood, not a mechanical consequence of preferring the model visually.

## Required native evidence

- exact input bundle/history and clean checkout;
- scope limited to Classic testing sandbox dispatch, Classic fill implementation/tests, help text, and Notebook;
- default testingcheats model is exactly `classic`;
- Classic wrapper accepts `fill_rainbow_80`;
- 80% bottom-anchored fill geometry and category order are exact;
- fill mass equals physical mass, pending=0, movement counters reset;
- repeat fill returns the same fixture shape/count;
- empty category list fails closed;
- existing Classic conservation/resize/wall tests pass;
- Oslo/SEDIMENT regressions remain unchanged and green;
- production SandEngine source delta is zero unless a purely test-visible/mechanical dependency is strictly required;
- fmt, strict Clippy, full tests, help smoke and command parser pass.

## Human gate

After machine green, start the certified candidate and run only:

```text
testingcheats fill
```

No `testingcheats model classic` should be necessary. Verify that the six Fixture categories appear in the Strata category menu and that the displayed sandbox behaves like Classic. Then inspect ordinary collapse/settling without using 64x unless specifically useful.

Only after that dogfood pass should we decide whether Classic should replace the accepted production sediment engine or remain an experimental/reference model.
