---
id: RAIN-002
kind: work
state: superseded
created: 2026-09-07
updated: 2026-09-07
authority: working
summary: Preserve the machine-green/human-liked 75/25 broad Hybrid rain and frozen momentum-grounded-contact settlement, but replace the single long focus sweep with dwell/compensational avulsion, sample only genuinely free ingress sites, and make fallspeed changes discard stale multiplier debt immediately.
---

# RAIN-002 — Avulsion focus + free-site ingress

## Owner evidence

After PERF-001 made interactive `fallspeed 128x` genuinely useful, the owner reported three bounded findings while preserving the RAIN-001 positives:

1. Classic and Hybrid airborne dots both remain visually broad; the 25% focus bias is not visibly detectable as a nozzle.
2. Hybrid relief is attractive, but the long focus trajectory still produces one dominant lump; the compensational preference is not visually evident enough.
3. Once the pile reaches the visible top, newly generated dots can appear beside an occupied sampled target because the current allocator samples an impossible target and relocates to the nearest free top cell. The owner wants new dots to choose only currently empty ingress sites.
4. Switching `testingcheats fallspeed 128x` back to `1x` can continue to look accelerated because synthetic wall-speed debt generated at 128x remains in the shared queue.

Classic settlement itself remains frozen at `momentum-grounded-contact`. RAIN-002 must not tune avalanche, repose, texture, anchors, momentum, renderer, category transport, persistence, or the PERF-001 exact optimizer.

## Candidate semantics

### 1. Broad rain remains 75/25

Hybrid keeps the already human-approved anti-nozzle envelope:

- 75% uniform over the complete set of currently free visible-top ingress columns;
- 25% broad focus bias, choosing the nearer of two stochastic free columns inside the focus corridor;
- if no free focus-corridor site exists, the biased event falls back to the already sampled global free site;
- the recursive-golden focus corridor/padding remains unchanged;
- Classic `Uniform` samples uniformly from the same current free-site set.

The key boundary change is that rain never samples an occupied top-row `x` and then relocates it sideways. If the top row is full, the generated category remains pending until a real ingress site opens.

### 2. Dwell + compensational avulsion

Replace the RAIN-001 continuous twelve-hour cross-corridor march with lobe-scale temporal structure:

- initial focus begins with a one-hour (3,600 ingress) dwell;
- during dwell, the focus performs only tiny local ±1/hold wander every 300 ingresses, bounded to a small neighborhood around the current locus;
- when dwell expires, sample three stochastic candidate loci across the focus corridor, excluding a neighborhood of roughly one sixth of the corridor around the current focus;
- score each candidate by the existing broad bottom-connected smoothed terrain window;
- choose the lowest of the three; exact equal minima remain stochastic;
- transition toward the selected locus at a width-normalized rate where a theoretical full-corridor crossing takes at most 1,800 ingresses (~30 simulated minutes);
- on arrival, start a fresh one-hour dwell.

This is a stochastic compensational avulsion model, not a global-lowest scan. Three candidates make lower accommodation materially more influential than RAIN-001's two candidates restricted to the opposite outer sixth, while the broad 75/25 ingress field prevents a visible nozzle.

### 3. Fallspeed debt ownership

Testing synthetic time now has two explicit owners:

- `queued_speed_simulated`: debt accrued from wall time multiplied by `fallspeed`;
- `queued_explicit_simulated`: debt requested by `testingcheats advance`.

Any explicit `testingcheats fallspeed ...` change discards only stale `queued_speed_simulated` before continuing at the new multiplier. It must preserve explicit Advance debt. The cooperative scheduler drains explicit Advance debt before multiplier debt when both exist. Fill/fillhalf/clear/model-reset continue to clear both.

This makes `128x -> 1x` an immediate preview-rate change rather than waiting for old 128x wall debt to drain.

## Machine gates

Required focused proof:

1. 75/25 bias constant and recursive-golden focus bounds unchanged.
2. Focus alternates between dwell and transition and reaches at least three distinct loci over a deterministic multi-hour ingress run while never leaving the corridor.
3. A unique lowest broad terrain candidate among three wins without tie RNG; flat/equal candidates retain stochastic tie choice.
4. Sampled avulsion candidates remain outside the current-locus exclusion radius when geometry permits.
5. 24k-target anti-nozzle histogram remains broad and both padding regions receive targets (`RAIN_002_METRICS`).
6. Uniform and Hybrid spawn only into currently free visible-top cells; a full top row preserves FIFO pending mass until a free site opens.
7. Uniform/Hybrid gravity law remains identical when ingress is removed.
8. PERF-001 optimized/reference exactness remains green for both Classic and the new RAIN-002 Hybrid state, including dwell/transition fields.
9. A fallspeed change drops multiplier debt immediately while preserving explicit Advance debt; scheduler consumption is explicit-first.
10. Classic baseline metrics, mass conservation, appearance/themes, parser/help, fmt, strict Clippy, full tests, and PERF-001 capacity remain green.

## Human gate

Use Hybrid at real accelerated speed:

```text
testingcheats clear
testingcheats model hybrid
testingcheats fallspeed 128x
```

Judge long-form morphology for several simulated hours:

- short-term airborne rain still looks homogeneous/nozzle-free;
- successive broad depositional loci are visibly more diverse than RAIN-001's single dominant lump;
- lower accommodation is perceptibly reoccupied over time without flattening the whole pile;
- strata remain continuous rather than alternating isolated blobs.

Near the visible top, verify new dots appear only in real empty ingress gaps rather than beside an occupied sampled target.

Then run:

```text
testingcheats fallspeed 1x
```

The visual ingress cadence must slow immediately, except for any explicitly requested `testingcheats advance` debt that remains intentionally queued.

## Supersession — RAIN-003

RAIN-002 passed machine validation, including free-site ingress, split fallspeed/Advance debt, PERF exactness/capacity and all regressions. The owner rejected only its long-form morphology: one-hour dwell plus lowest-of-three broad compensational avulsion produced strata that looked too homogeneous/bland. RAIN-003 preserves the independent ingress/debt/performance fixes and replaces only focus morphology with correlated meander, weak terrain steering, category-boundary rephase and a wider focus corridor.
