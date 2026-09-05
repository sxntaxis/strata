---
id: SEDIMENT-015G-D1
kind: work
state: candidate
created: 2026-09-05
updated: 2026-09-05
authority: working
summary: Roll back the rejected SEDIMENT-016 authoritative-active-grain experiment to the owner-preferred SEDIMENT-015G-R2 visual-support candidate and add diagnostic-only rainbow provenance evidence plus idempotent testingcheats fill categories, without changing Oslo physics or 015G presentation semantics.
---

# SEDIMENT-015G-D1 — rainbow provenance diagnostic

## Owner decision

The SEDIMENT-016 direct authoritative-active-grain experiment is rejected for current work. Human runtime evidence showed a giant visual regression with repeated fan/ray patterns already seen in earlier transport attempts.

Return to SEDIMENT-015G-R2 (`9f0943b044aa04f78bdea161aa8d28a4e2528c59`), which the owner described as the best visual result so far.

Do not redesign the physics/presentation authority boundary in this unit.

## Questions this diagnostic must answer

1. Is the cyan material that appears visually near the basal green band backed by a real sequence of adjacent authoritative CategoryId transport edges?
2. How much green-category transport actually occurs during the deterministic rainbow collapse?
3. Do the first physical batches themselves move cyan aggressively, or is the perceived initial cyan jump primarily presentation timing?

The current model intentionally has no persistent physical grain identity. Therefore diagnostics must not claim a percentage of unique physical grains ever moved. `movement_entries` means transient MotionId motion episodes; it may count the same fungible CategoryId mass again after a fully revealed settlement and later re-entry.

## Diagnostic method

Use the existing 015G unit-carrier facts as a read-only observer. Existing machine contracts already require:

- one carrier per active CategoryId unit;
- adjacent physical edges only;
- no future route prediction;
- exact final CategoryId stack;
- coherent batch replay.

The ignored diagnostic records, without changing any transport decision:

- early physical-batch hop counts per rainbow category;
- per-category initial/final mass, movement episodes, adjacent hops, max hop count, max horizontal displacement and discharge;
- green final mass inside/outside the original filled footprint;
- cyan units that finish below the initial green-band height;
- up to twelve cyan low-settlement MotionId paths as exact adjacent edge sequences.

Each category must reconcile `initial == final + discharged`.

## testingcheats fill UX

`testingcheats fill` must idempotently ensure six real categories in the normal category catalog and persist them through the existing category sync path:

1. Fixture Green
2. Fixture Yellow
3. Fixture Red
4. Fixture Purple
5. Fixture Blue
6. Fixture Cyan

The testing sediment remains isolated from authoritative live sediment. The category catalog is intentionally durable because the owner wants these fixture categories visible in the Strata menu without manual creation.

Repeated `testingcheats fill` must reuse the same active categories rather than creating duplicates.

## Scope boundary

Allowed runtime change:

- debug-only `testingcheats fill` category provisioning and help text.

Allowed test change:

- ignored diagnostic observer over existing 015G MotionId/segment/custody state.

Forbidden:

- Oslo thresholds, front trigger, erosion/support-loss, coast/runout;
- rolling physics;
- visual shadow/custody semantics;
- 015G lane geometry;
- coherent scheduler;
- raster policy;
- rain policy;
- settlement/reveal behavior;
- fallspeed behavior;
- persistence schema.

## Required local output

The local native validator must return all `RAINBOW_BATCH`, `RAINBOW_TRACE`, `RAINBOW_GREEN_MASS`, `RAINBOW_CYAN_LOW`, and `RAINBOW_CYAN_LOW_PATH` lines verbatim enough for owner interpretation.

No new human visual gate is required in this diagnostic unit.
