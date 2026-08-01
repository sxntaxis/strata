---
id: CONCEPT-001-RESEARCH
kind: research
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Product synthesis of Strata as a continuous temporal ledger, active timer, low-power inference system, and accountable sedimentary artwork.
---

# CONCEPT-001 — Continuous time and sedimentary memory

## Purpose

This record preserves the durable meaning developed during the first conceptual audit of Strata after its static code and reliability audit.

It is not a transcript. It separates:

- the outsider's raw first impression;
- the product owner's corrections and intent;
- accepted product doctrine;
- candidate interpretations;
- unresolved questions;
- implementation consequences.

## Starting perception

The initial outsider reading interpreted Strata primarily as a conventional time tracker with an artistic visualization. From that frame, several features appeared conceptually confused:

- drift looked like a substitute for a stopped timer;
- the sediment looked like a derived visualization rather than historical material;
- project, category, tag, and description concepts looked under-specified for freelance reporting;
- `Karma` looked like moral or productivity scoring;
- detach looked like misleading background tracking;
- physical sand mixing looked like a threat to chronology;
- clearing looked like an unsafe deletion command.

These conclusions were partly wrong because they imported assumptions from ordinary productivity software.

The raw reaction remains useful as **legibility evidence**. It shows what Strata may communicate to an unfamiliar user before its doctrine is understood. It does not define the intended product.

## Owner correction

The product owner clarified four foundational points.

### Time is continuous

Strata is a continuous ledger because time does not stop when the user stops actively classifying it. The constant fall of baseline sediment gives that fact a material presence.

The baseline state should be called **idle**, not drift.

Idle is not missing data, an error state, or an accidental fallback. It is the ordinary condition of unassigned continuous time.

### Active use is still a timer

The continuous-ledger model and timer model are not opposites.

```text
continuous elapsed time
    ├── idle
    └── active selected layer
```

When the user selects a layer, Strata functions as a timer for that activity. The timer classifies the ongoing flow rather than starting time itself.

### Strata is general-purpose

The project is not primarily a freelancer tool. It is used for:

- study tracking;
- habit management;
- project tracking;
- work;
- leisure;
- creative practice;
- other personally meaningful layers.

A client/project/activity hierarchy may be useful in one context but cannot define the base product ontology.

### Tools may be art

The sediment is not an ornamental skin around the real tool. Falling, mixing, accumulation, topology, color, idle presence, and loss are part of the product's meaning.

A critique that removes those properties in order to make Strata resemble a conventional timer has misunderstood the design problem.

## Governing proposition

> Time continuously deposits material. Idle is its baseline state. Selecting a layer gives passing time identity, color, and balance direction. The chronological ledger records this exactly; the sediment embodies it with accountable but organic precision.

Strata is therefore not a timer plus a visualization. It is one temporal instrument expressed through two resolutions of history.

## Two historical truths

### Chronological truth

The precise historical record answers exact questions:

- when an interval began and ended;
- how long it lasted;
- which layer owned it;
- what note or context applied;
- how operational-day boundaries divide it;
- how it contributes to reports and balance.

This truth requires exact timestamps, durations, stable identities, and auditable correction.

### Sedimentary truth

The formation answers experiential and material questions:

- how much time accumulated;
- which colors and activities formed it;
- what contours emerged;
- which materials became neighbors;
- how layers mixed;
- what broad chronological body was produced;
- what the passage of time felt and looked like.

It is less precise than the chronological ledger, but less precise does not mean disposable.

The product owner explicitly reports that losing the sediment hurts. That emotional consequence is product evidence: the formation has historical value independent of reporting utility.

## Fidelity hierarchy

All sediment properties matter. Their preservation obligations may differ.

### Exact invariants

- total represented temporal mass;
- per-layer temporal mass;
- no silent grain invention;
- no silent grain loss;
- stable relation to a defined temporal quantum;
- explicit handling when physical display capacity is exceeded.

### Strong structural invariants

- broad age ordering where chronology is intended;
- pile contours;
- category neighborhoods;
- layer composition;
- formation identity;
- survival across resize, reopen, migration, and recovery.

These may allow bounded physical evolution, but not arbitrary administrative remixing.

### Emergent local detail

- the exact cell chosen by a falling grain;
- small rolls and avalanches produced during live simulation;
- the exact subcell seam at which colors meet;
- local settling that is part of the living physical process.

The unresolved design task is to distinguish legitimate physical emergence from destructive administrative transformation.

## Temporal quantum

Current behavior uses:

```text
1 grain = 1 elapsed second
```

This is accepted as the present visual scale.

A configurable temporal quantum is a useful future candidate:

```text
1 grain = 5 seconds
1 grain = 30 seconds
1 grain = 1 minute
```

The chronological ledger should remain exact regardless of visual quantum.

A quantum is not merely a zoom level. It changes the density, growth rate, and material scale of a formation. Therefore a future setting likely belongs to the formation and requires explicit behavior for:

- starting a new formation with a new quantum;
- reprojecting an existing formation;
- keeping formations at different quanta;
- comparing or displaying mixed-quantum history;
- accounting for remainder duration that does not fill a complete grain.

No implementation is authorized yet.

## Braille and color mixing

A Braille character contains several physical dot positions but provides one foreground color. When grains of different layers occupy the same character cell, the renderer cannot display each dot with an independent foreground color.

Some compositing rule is unavoidable.

Color blending is accepted because:

- it represents subcell composition rather than hiding all but one layer;
- seams between materials become visible as mixtures;
- physical sand also mixes;
- the limitation becomes part of the TUI medium rather than an error to conceal.

Future evaluation should focus on:

- perceptual legibility;
- whether minority colors disappear;
- contrast across terminal palettes;
- deterministic blending;
- inspection of underlying composition;
- accessibility and monochrome fallback.

The goal is not to eliminate mixing.

## Vertical position

Vertical meaning remains open.

### Model A — strict chronology

Older grains always remain below newer grains.

Advantages:

- strongest geological reading;
- clear age ordering;
- stable historical strata.

Costs:

- limits natural movement;
- may prevent meaningful avalanches and local remixing;
- risks turning sand into a stacked chart.

### Model B — pure physical settlement

Position means only where simulation placed material.

Advantages:

- strongest physical freedom;
- most emergent shapes;
- simplest simulation authority.

Costs:

- age becomes weak or unreadable;
- the name Strata promises more historical structure than the system preserves.

### Model C — broad chronology with local emergence

Macrostructure records age while microstructure may roll, settle, and mix.

Advantages:

- preserves the geological metaphor at formation scale;
- retains living sand behavior;
- supports organic seams and local neighborhoods.

Costs:

- requires an explicit tolerance model;
- recovery and resize must preserve structure without freezing every cell;
- tests become more semantic than simple byte equality.

Model C is the current candidate, not an accepted decision.

## Layers and context

The current layer list is flat. A flat system is immediate and visually coherent: one selected layer gives each grain one principal color.

However, user-defined layers may mix different dimensions:

- activity: Reading;
- subject: Algorithms;
- project: Stereo;
- domain: University;
- goal: Exam preparation;
- state: Leisure.

One real interval may satisfy several descriptions.

### Flat-only model

Every meaning is represented as a peer layer.

Benefits:

- low interaction cost;
- one color and one classification;
- no hierarchy management;
- strong artistic simplicity.

Risks:

- layer list grows incoherently;
- users must choose between activity and purpose;
- reports cannot answer both kinds of questions without duplicated layers.

### Hierarchical model

Layers may nest, such as University → Algorithms → Reading.

Benefits:

- rich structural meaning;
- inherited aggregation.

Risks:

- complex interaction;
- unclear grain color ownership;
- hierarchy may not fit habits, states, and overlapping goals;
- risks turning Strata into a project manager.

### Primary layer plus optional context

One primary layer owns the grain's visual material identity. Optional contexts describe what the time was for.

Example:

```text
primary layer: Reading
contexts: University, Algorithms, Exam preparation
```

Benefits:

- preserves one primary color;
- supports multidimensional reports;
- remains general-purpose;
- avoids mandatory hierarchy.

Risks:

- context entry may burden quick timing;
- schema and interaction become more complex;
- users may be uncertain which dimension deserves primary status.

Primary layer plus optional context is the current candidate, not an accepted decision.

## Balance, polarity, and Karma

The owner describes the feature as balance, commonly between work and leisure but potentially along any personally meaningful axis.

This is not intended as universal moral judgment.

The model may contain two concepts:

- a layer property such as **polarity** or **valence**;
- an aggregate view called **Balance**.

Possible vocabulary:

| Term | Strength | Risk |
|---|---|---|
| Karma | memorable and poetic | moral, religious, and causal baggage |
| Polarity | structurally precise | technical and binary-sounding |
| Valence | captures positive, negative, and neutral personal meaning | unfamiliar to some users |
| Charge | vivid directional metaphor | electrical rather than geological |
| Balance | immediately understandable | names the aggregate more than the property |

The directional model is accepted. Final terminology is not.

## Clearing and formation lifecycle

The current practical reason for clearing is visibility: the user wants actively relevant sediment in sight. Hiding idle can feel like hiding idle time, but permanent destruction is not necessarily intended.

One `clear` command currently collapses several possible meanings.

### Hide idle

A reversible presentation filter. Idle remains historically present.

### Begin a new formation

Preserve the completed formation and start deposition in an empty basin.

Possible boundaries include a day, week, project phase, personal reset, or arbitrary user choice.

### Compact older material

Compress older sediment into a lower or summarized band while preserving mass and composition.

### Rebuild formation

Regenerate a projection from authoritative history under an explicit reconstruction contract.

### Permanently erase

Destroy sedimentary history with explicit confirmation and clear scope.

These operations should not be conflated. The minimum accepted formation lifecycle remains open.

## Detach as low-power inference

Detach addresses a real design opportunity:

> Strata does not need to execute continuously in order for time to continue existing.

A timestamp and selected layer can represent continued elapsed time at almost zero power cost. This is not merely a workaround; it can be a defining low-power mode.

The current problem is that several distinct processes are coupled:

1. elapsed time is calculated;
2. the prior layer is assumed to remain active;
3. ledger history is inferred;
4. grains owed by the interval are calculated;
5. missed physics frames are replayed;
6. present-day interaction waits for simulation catch-up.

Only the first four are needed to establish current truth.

### Candidate deliberate-detach contract

On detach, persist:

- exact detach timestamp;
- selected layer;
- current note/context;
- active interval identity;
- exact sediment state;
- simulation seed or reconstruction state;
- temporal quantum;
- formation identity.

On reopen:

1. calculate exact elapsed duration;
2. finalize or extend the chronological interval transactionally;
3. calculate exact grains owed;
4. materialize them through a deterministic bounded deposition operation;
5. make current interaction available immediately;
6. optionally animate reconstruction without letting animation own validity.

A brief accelerated cascade or "time returning" effect may be artistically appropriate. It must not require replaying hours of simulation wall time.

### Deliberate detach versus unexpected termination

A deliberate detach communicates intent:

> Continue this selected layer while the process is absent.

An unexpected process termination does not necessarily communicate the same intent.

The elapsed interval still exists because time continued. Its classification may be uncertain.

Candidate recovery presentation:

```text
Strata closed unexpectedly 1h 42m ago.

Classify the interval as:
- continue previous layer;
- idle;
- split or edit;
```

A future user setting may choose a default policy. Treating crash and deliberate detach identically has been rejected.

## Refusals

This synthesis refuses:

- reducing Strata to a conventional start/stop timer;
- treating idle as stopped or missing time;
- designing primarily around freelance client accounting;
- treating sediment as disposable decoration;
- eliminating color mixing merely to simplify rendering;
- assuming exact cell immutability is the only way to value topology;
- allowing resize, migration, or recovery to arbitrarily remix history;
- replaying every missed frame as a prerequisite for correctness;
- allowing recovery animation to block present interaction;
- silently inferring crash classification without an accepted policy;
- treating Karma as settled terminology;
- embedding a project hierarchy before the layer model is decided;
- making clearing an ambiguous destructive shortcut.

## Product implications

The SQLite migration must wait for enough conceptual resolution to model:

- idle as baseline continuous time;
- active versus inferred intervals;
- deliberate detach versus unexpected termination;
- primary layer and possible optional context;
- formation identity and temporal quantum;
- exact sediment mass and bounded topology preservation;
- reconstruction provenance;
- hidden, compacted, archived, or erased visual states;
- balance property and aggregate semantics.

The schema should encode accepted product meaning, not fossilize current CSV fields and implementation accidents.

## Current conclusion

Strata's central challenge is not choosing between utility and art. Its value comes from refusing that split.

The system should become more reliable without becoming less alive:

- exact where history must answer factual questions;
- accountable where history becomes material;
- organic where physical emergence creates meaning;
- explicit where inference introduces uncertainty;
- low-power without pretending absent execution observed what happened;
- understandable without translating itself into an ordinary timer.
