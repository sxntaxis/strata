---
id: SEDIMENT-014R1
kind: work
state: active
created: 2026-09-04
updated: 2026-09-04
authority: working
summary: Repair SEDIMENT-014 native failures by treating same-CategoryId visual mass as fungible and routing weighted parcels through real directional adjacent-transfer quotas toward dynamic physical-vs-shadow settlement demand.
---

# SEDIMENT-014R1 — Fungible Flux Custody

## Trigger

Native validation of SEDIMENT-014 v1 failed with 64 false shadow misses during
ordinary deterministic driving and a non-draining parcel cloud in the wall
fixture.

## Diagnosis

The v1 ledger stored durable exact `(site, CategoryId)` settlement credits. That
quietly reintroduced an identity assumption:

```text
"the same CategoryId unit that settled here must later be the unit mobilized here"
```

Oslo/front does not preserve that identity and the product does not need it.
CategoryId mass is fungible within its material class. Exact-site credit reuse
therefore becomes ambiguous after repeated settle/re-entry/local-topple cycles.

The non-drain failure is the same issue geometrically: a stale exact sink can
remain after physical custody has moved again, leaving a parcel with no locally
reachable settlement target.

## R1 contract

Keep:

```text
frozen SEDIMENT-008 front physics
visual shadow settled surface
weighted CategoryId parcels
no immediate visual settlement
live rain / stable avalanche clock
```

Replace exact settlement tokens with:

```text
settlement_demand(site, category)
  = max(physical_settled_count - shadow_settled_count, 0)
```

and record every real adjacent moving-phase crossing as a directional
CategoryId quota:

```text
transport_right[source][category] += mass
transport_left[source][category]  += mass
```

Rolling hops add further local quotas. Ordinary settled topples mirror directly
when the shadow still owns the local CategoryId unit; if the local visual unit
is already mobile, the physical transfer adds only another local transport
quota.

## Fungible re-entry

When physical mass enters the moving phase:

- if the source shadow owns a matching CategoryId unit, withdraw it and create
  one unit of parcel mass;
- otherwise do not flag a miss or create duplicate mass: matching material is
  already under mobile visual custody;
- in both cases record the real adjacent transport quota.

This keeps category totals exact without persistent grain identity.

## Parcel routing

A parcel can:

1. deposit only into current positive same-category settlement demand;
2. otherwise move only across a neighboring edge with outstanding real
   same-category directional transport quota;
3. split weighted mass when only part of the parcel is backed by an outgoing
   quota;
4. coalesce only locally, never by teleporting same-category mass between
   unrelated sites merely to satisfy a parcel-count cap.

This is an Eulerian conservative flow decomposition, not a per-grain path queue.

## Acceptance

Native validation must prove:

- frozen front physical equivalence;
- zero true custody misses;
- shadow + parcel total/category mass equality to physical settled + rolling;
- ordinary deterministic driving does not manufacture false misses;
- wall fixture drains parcel mass and transport quota to zero;
- dynamic settlement demand reaches zero;
- final per-site CategoryId multisets match authoritative columns;
- no persistent grain identity / final-destination path is introduced.
