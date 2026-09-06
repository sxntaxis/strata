---
id: CLASSIC-012
kind: work
state: candidate
created: 2026-09-06
updated: 2026-09-06
authority: working
summary: Promote momentum-grounded-contact as the owner-selected Classic default after CLASSIC-011 human review, then remove its repeated bottom-connectivity rescans with an exact sweep-local grounded-column index that preserves grid, RNG, repose, movement, and visual behavior bit-for-bit against the uncached reference path.
---

# CLASSIC-012 — Grounded-contact baseline + exact connectivity cache

## Owner gate

CLASSIC-011 passed machine validation with 431 tests and no mechanical fallout. Human review then found `momentum-grounded-contact` practically perfect and selected it as the new Classic baseline.

The remaining defect is performance, not physics: during the dense `fillhalf` transient, the Strata process can consume roughly one full CPU core/thread and the visual cadence slows. The owner explicitly requires performance work to preserve the selected visual behavior.

Therefore CLASSIC-012 has two authorized outcomes:

1. a newly constructed Classic/Hybrid sandbox defaults to `momentum-grounded-contact` over the existing rugged texture base;
2. optimize only the implementation cost of its grounded-contact predicate, with exact behavioral equivalence as a hard gate.

## Source diagnosis

CLASSIC-011 asks whether an occupied direct blocker is bottom-connected pile before allowing the ordinary diagonal. The current predicate answers that question by scanning every row from the blocker to the visible floor:

```text
(blocker_y .. visible_bottom).all(occupied)
```

That predicate is semantically correct, but `momentum-grounded-contact` invokes it on a large fraction of blocked grains. A dense rainbow wall therefore turns one nominal O(width × height) gravity sweep into repeated overlapping vertical scans approaching O(width × height²) work.

The accepted profile also redundantly rechecks the same direct blocker for the momentum contact gate after the ordinary grounded-contact gate has already proved it.

## Behavior-preserving implementation

Add one **ephemeral, sweep-local** grounded-column index only for profiles whose ordinary diagonal requires grounded contact (currently `momentum-grounded-contact`).

For each visible column the index stores the top row of the contiguous occupied suffix that reaches the visible bottom boundary. A cell is grounded exactly when it lies within that suffix.

At the start of a gravity sweep:

```text
one bottom-up scan per visible column
-> exact grounded suffix top
```

During the existing in-place sweep, update the index after every vertical, ordinary diagonal, and bonus-diagonal move:

- removing a cell from a grounded suffix moves that column's suffix top below the vacancy;
- filling the one-row gap immediately above a grounded suffix extends it upward;
- if already-occupied cells directly above the filled gap become connected by that fill, include that contiguous run as well;
- moves that do not touch the grounded suffix leave the index unchanged.

The index is not persisted, rendered, exposed, or shared with Oslo/H4. It is rebuilt from the authoritative grid every gravity sweep.

For `momentum-grounded-contact`, once the ordinary gate proves the direct blocker grounded, reuse that exact result for the momentum blocker gate instead of scanning the same column again.

## Exactness boundary

CLASSIC-012 may change **cost only** after the owner-authorized default promotion.

For an explicit `momentum-grounded-contact` run, cached and uncached paths must remain exactly equal after every compared gravity pass for:

- complete grid/category placement;
- physics RNG state;
- repose RNG state;
- local repose field;
- repose-memory counters;
- vertical and diagonal movement counts;
- sweep direction;
- physical mass.

No approximate visual equivalence is acceptable.

Do not change:

- the CLASSIC-011 airborne-wait law;
- repose-relative momentum eligibility;
- maximum one bonus hop;
- slope bias, memory, anchor distribution, texture, or RNG consumption;
- any explicit earlier experiment profile;
- `fill` / `fillhalf`;
- renderer, categories, Oslo, H4, persistence, schema, provenance, or application scheduling.

Do not add threads, parallel physics, frame skipping, lower gravity frequency, batch approximation, two-phase physics, or any optimization that changes update ordering.

## Required proof

Machine validation must prove:

1. default texture remains `rugged` and default experiment is now `momentum-grounded-contact`;
2. the cached runtime path matches an uncached reference scan path exactly over multiple seeds and many dense-wall gravity passes, including grid and all RNG/repose/movement states named above;
3. focused CLASSIC-011 airborne-wait and grounded-pile momentum gates remain green;
4. all explicit prior Classic profile metrics remain unchanged;
5. fillhalf, Oslo, provenance, parser/help, category, fmt, Clippy, and full repository tests remain green;
6. a native runtime comparison on fresh `fillhalf` shows a material CPU/frame-time improvement for the new default, with the measurement reported but visual equivalence remaining the semantic authority.

## Human gate

Run the default directly; no experiment command should be necessary:

```text
testingcheats fillhalf
```

Then explicitly select the same profile as a sanity check:

```text
testingcheats classic experiment momentum-grounded-contact
testingcheats fillhalf
```

The two must look identical. The owner should judge only responsiveness/frame cadence versus CLASSIC-011; any physics or visual difference is a failure.
