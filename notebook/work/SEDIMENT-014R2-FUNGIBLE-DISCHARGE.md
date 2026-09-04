---
id: SEDIMENT-014R2
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Close the remaining SEDIMENT-014R1 false discharge misses by consuming anonymous same-CategoryId visual surplus from current physical-vs-shadow imbalance rather than requiring exact-source custody.
---

# SEDIMENT-014R2 — Fungible Discharge Reconciliation

## Trigger

Native validation of SEDIMENT-014R1 closed the delayed-deposit, re-entry, two-edge
fungible routing, and wall-drain failures, but ordinary deterministic driving
still ended with:

```text
flowviz_shadow_misses = 11
```

The only remaining increment site for that diagnostic was settled discharge.

## Diagnosis

R1 correctly made settlement demand dynamic and same-CategoryId moving custody
fungible, but discharge still used this fallback order:

```text
exact source shadow
→ any same-category parcel mass
→ MISS
```

That final step was still too identity-like. After anonymous settle/re-entry and
ordinary local-transfer cycles, conservative visual custody can legally hold a
same-CategoryId *settled surplus* at another site while the physical unit that
is now discharging is at the edge.

A physical discharge changes total/category mass, not persistent grain identity.
If exact-source shadow and parcel custody do not contain the unit, the correct
identity-free question is:

```text
Does the current visual shadow contain any same-category site where
visual_count > physical_count?
```

If yes, that surplus is the anonymous visual custody that must leave the system.

## R2 contract

For settled discharge, consume custody in this order:

1. matching CategoryId at the physical source shadow;
2. matching CategoryId parcel mass;
3. nearest same-CategoryId **current shadow surplus**, defined strictly as
   `visual_count(site, category) > physical_count(site, category)`;
4. increment `flowviz_shadow_misses` only if none of those custody forms exists.

The surplus search is nearest-site only for presentation locality. It may never
remove a unit from a site that is not currently in same-category visual surplus,
so it cannot manufacture positive settlement demand or alter physical state.

## Invariants

R2 does not change:

- SEDIMENT-008 front physics;
- Oslo thresholds, front triggers, erosion, support loss, or runout;
- physical CategoryId custody;
- dynamic settlement demand;
- directional real-transfer quotas;
- parcel routing or motion;
- scheduler/live-rain behavior.

`flowviz_shadow_misses` now means a true custody invariant break: after a
physical discharge, no same-category custody exists in source shadow, parcel
mass, or any current same-category shadow surplus.

## Acceptance

Native validation must prove:

- the R1 ordinary 2,000-drive test now has zero misses;
- the R1 wall-drain and fungible re-entry tests remain green;
- a targeted anonymous-discharge fixture consumes nearest same-category shadow
  surplus without creating parcel mass or a miss;
- total/category visual conservation remains exact;
- full repository gates pass before any human A/B resumes.
