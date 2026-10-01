---
id: SEDIMENT-016
kind: work
state: candidate
authority: working
created: 2026-09-27
updated: 2026-09-27
summary: "Test one simpler resize invariant: visible side edges remain physical walls, while terminal height becomes a render crop and already-placed sediment above it keeps settling through the canonical vertical corridor."
---

# SEDIMENT-016 — vertical physics corridor experiment

## Owner direction

The historical Balance animation exposed a general resize artifact: canonical sand above a shorter viewport can remain frozen and later reappear as large shelves or ceilings. The owner does not want a history-specific patch or a collection of offscreen special cases.

This candidate tests one global invariant only:

> Horizontal viewport edges are physical walls; vertical viewport edges are not.

Equivalently:

- render bounds remain the existing centered, bottom-aligned viewport crop;
- the active physics corridor uses the current visible width but the full canonical height;
- width shrink therefore continues to freeze horizontally hidden terrain and preserve closed side walls;
- height shrink no longer freezes already-placed terrain above the visible top;
- new ingress/rain remains anchored to the visible top edge exactly as before.

No special historical physics law is added. Disposable historical previews inherit the same production gravity geometry automatically.

## Implementation boundary

`SandEngine::viewport_bounds()` remains render/ingress authority. A separate `physics_bounds()` owns gravity geometry: its X range is the same centered active width, while Y spans `0..grid_height`.

Production Classic gravity (optimized and reference paths), the underlying H4 gravity path, and Classic supported-height calculations use the physics corridor. Rain targeting, pending ingress, debug fill, and rendering continue to use viewport bounds.

## Non-goals

This candidate does not:

- run full-canvas horizontal physics;
- allow grains through the visible side walls;
- move ingress to the canonical top;
- delete, repack, or rewrite hidden sand;
- change persistence or SandState schema;
- add resize-triggered reconciliation passes;
- add history-specific settling rules.

## Human test gate

Before promotion, test at least:

1. normal live use;
2. shrink height, wait, then grow height;
3. shrink width, wait, then grow width;
4. historical Balance preview;
5. repeated resize oscillation;
6. mass conservation and restart continuity;
7. CPU behavior after a very tall canonical canvas is later viewed in a short pane.

If this single invariant produces a new unintuitive edge case, reject the candidate rather than layering compensating special cases on top.

## Local validation facts — 2026-09-27

- Formatter, strict Clippy, all-feature tests (539 unit + 24 integration; 20 ignored), and CLI help smoke pass.
- An isolated debug profile was filled, resized vertically between 24, 10, and 160 terminal rows, resized horizontally between 80 and 40 columns, and oscillated seven times. Height growth/shrink and width crop/re-expansion completed without a runtime error.
- With a canonical canvas of at least 320 dot rows while viewing a 10×80 pane, a five-second process sample measured 4.0% of one CPU core and about 14.1 MiB RSS.
- A separate historical Balance profile opened a prior day and completed four 40×10/80×24 resize oscillations without a runtime error. That saved artifact appeared settled in this capture, so this confirms the route/resizing but not visible grain motion.
- Both disposable profiles passed `sqlite-doctor` after normal exit; the fresh profile also reopened with no active session.

Exact persistent mass comparison across a resize-and-restart cycle, broader normal-live visual use, and owner qualitative acceptance remain open gates. The accepted resize doctrine has not been changed.

## Promotion boundary

This branch is an owner-authorized experiment for local visual testing. The accepted resize doctrine in `docs/SEDIMENT_AUTHORITY.md` remains the baseline until the owner reviews this behavior and explicitly promotes or rejects it. Do not silently rewrite accepted sediment authority from this candidate alone.
