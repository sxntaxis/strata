---
id: SEDIMENT-008
kind: work
state: active
created: 2026-09-03
updated: 2026-09-03
authority: working
summary: Keep oslo-vessel as the static pile, but replace momentum-v1's isolated rolling grains with a bounded static/rolling exchange front: erosion on steep moving passages, higher static start than dynamic stop, and uphill propagation by loss of support.
---

# SEDIMENT-008 — Oslo-vessel erosion/support front

## Owner evidence entering this unit

The owner judged `oslo-vessel-momentum` materially more interesting than the earlier WaveView and global-inertia experiments, but still short of the desired wall-collapse behavior. Human resize evidence showed:

- ordinary Oslo-vessel topography remained strong;
- steep reconnection could release visible moving grains and produce skirts/runout;
- however the main wall often remained too intact while material escaped locally at its feet;
- the event still did not consistently read as a broad body losing support and joining the flow.

This is enough evidence to retain the **separate moving phase** but reject momentum-v1's rule that mobile grains merely coast independently over a fixed static bed.

Production H4/v5 remains untouched. SEDIMENT-008 is debug-only and never persisted.

## Literature direction

This unit is informed by four converging granular-flow results, used qualitatively rather than as a claim of numerical reproduction:

1. **BCRE/static + rolling phases.** Bouchaud, Cates, Ravi Prakash and Edwards model a sandpile with separate immobile and rolling populations and exchange between them: moving grains may deposit below repose and dislodge static grains above it.
2. **Start/stop hysteresis.** Granular layers require a higher condition to start moving than already-moving material needs to keep moving; Pouliquen/Forterre and later friction laws distinguish `start` and `stop` thresholds and show dependence on flow thickness.
3. **Two avalanche regimes.** Daerr & Douady observe thin avalanches propagating downhill/laterally, while sufficiently thick layers develop an **uphill-propagating front** because grains uphill lose support and tumble into the moving layer.
4. **Erosion/deposition controls propagation.** Later experiments show that an avalanche propagating over an erodible bed can entrain material at the front and deposit material behind it; the balance depends on avalanche mass/shape as well as slope.

The product implication is narrower than a continuum PDE: Strata needs a discrete dot-preserving analogue in which a moving front can exchange mass with the Oslo bed instead of merely sliding over it.

## Baseline and controls

SEDIMENT-008 is authored on top of SEDIMENT-007 `dbfb8d1af74e09a7823a513ab22db8050c1da9c3d` so the accepted causal rolling infrastructure remains available as a control.

Testing models remain:

```text
oslo-vessel
oslo-vessel-momentum
oslo-vessel-front
```

`oslo-vessel-front` keeps:

- the exact Oslo static threshold law `{1,2}`;
- accepted `oslo-vessel` boundary/canonical semantics;
- the local catastrophic seed at selected relief `>= 4`;
- physical transient rolling grains with CategoryId identity;
- one-lattice-site-per-physics-tick visual motion;
- quiescent drive ordering and existing testing-cheats cadence.

It does **not** reintroduce WaveView batching or a global avalanche-size gate.

## Static/dynamic hysteresis

The new model uses three deliberately separated discrete conditions:

```text
ordinary static Oslo threshold: 1 or 2
support-loss start relief:      3
catastrophic initial seed:      4
```

A moving grain, once present, can continue downhill on any positive settled relief and receives only a short bounded flat coast. This makes already-moving material easier to continue than static material is to start, mirroring the qualitative `theta_start > theta_stop` hysteresis in granular-flow experiments.

The thresholds are not fitted to a material. They are chosen to preserve an important Strata invariant:

- ordinary one-grain quiescent Oslo drive cannot create relief `4` from a stable `{1,2}` surface;
- structural reconnection can;
- once that structural failure exists, support loss at relief `3` can allow the failure front to propagate through the wall rather than waiting for each column to independently reach relief `4`.

## Erosion exchange

Momentum-v1 let rolling grains move over an unchanged static bed. SEDIMENT-008 instead treats a steep moving passage as erosion.

When a rolling grain moves from settled source `A` to lower destination `B`:

```text
if height(A) - height(B) >= 2:
    entrain at most one additional settled grain from A
    place it into the rolling phase at B
```

The source threshold is not redrawn merely because of erosion; this is a rolling/static exchange event, not an Oslo toppling event.

To avoid a numerical instantaneous column deletion, a source site can be eroded at most once per rolling update tick. A large event must therefore propagate over physical time rather than vaporizing a wall in one solver call.

## Uphill propagation by loss of support

Every time a settled source loses mass during an active front — either an Oslo topple or moving-layer erosion — the immediately uphill neighbour is reconsidered.

For a flow moving right:

```text
uphill     support     downhill
  U           S           →
```

If:

```text
height(U) - height(S) >= 3
```

one top grain from `U` joins the rolling phase at `S` in the same downhill direction.

This is the discrete mechanism corresponding to the Daerr-Douady thick-layer regime: the avalanche can propagate **uphill** because material loses support, while the grains themselves still move downhill.

The recruitment is one grain at a time and therefore remains dot-preserving.

## Dynamic runout and flow thickness

Momentum-v1 used a fixed two-cell flat coast. Human evidence showed interesting motion but also long local skirts without enough wall recruitment.

SEDIMENT-008 therefore changes flat transport:

- baseline moving material may coast only one flat cell;
- a locally thicker rolling layer can extend this budget slightly;
- the budget is capped at three cells.

This borrows only the qualitative experimental result that start/stop behavior depends on flowing-layer thickness. It is intentionally bounded so a large event can remain coherent without turning the vessel floor into a frictionless runway.

## Expected visual regimes

### Small event

```text
local Oslo slip
→ maybe one rolling grain
→ no sustained erosion/support front
→ deposits quickly
```

Expected: ordinary Oslo-vessel character remains dominant.

### Thin moving event

```text
seed
→ downhill transport
→ some local erosion
→ deposits behind/front
```

Expected: a small tongue or skirt rather than whole-wall failure.

### Thick structural reconnection

```text
wall removed
→ relief >= 4 seed
→ rolling layer crosses steep bed
→ erosion recruits more moving grains
→ support column drops
→ uphill neighbour reaches start relief 3
→ uphill support front advances
→ wall can fail progressively while grains move downhill
→ flow thins and deposits
```

Expected: the wall should look like it is being eaten/released from its base and face, with an active front moving uphill through the static mass while the actual dots travel down the slope.

This is the principal human discriminator.

## Boundaries and custody

All SEDIMENT-007 vessel constraints remain:

- canonical width remains monotonic;
- only current VW is active;
- hidden settled terrain is frozen/custodied while cropped;
- temporary VW edges are walls;
- re-expansion removes those walls and reconnects preserved terrain;
- rolling grains may not cross a current VW wall;
- canonical-height overflow remains the only lateral mass exit;
- no persistence/schema change.

## Diagnostics

`testingcheats status` for `oslo-vessel-front` retains momentum counters and adds:

```text
front=erosion:<total> uphill:<total> last=<erosion>/<uphill>
```

The desired resize event should show both:

- `erosion > 0`; and
- `uphill > 0`.

If only erosion fires, the model is still making skirts without true support-front propagation. If only uphill recruitment fires, the moving layer is not exchanging enough mass with the bed.

## Machine gates

The candidate must prove:

1. 2,000 ordinary quiescent drives remain exact `oslo-vessel`, with zero moving seeds, erosion, or uphill recruits;
2. BCRE-like erosion on relief `>= 2` entrains one settled grain, preserves CategoryId/mass, and creates a second physical rolling grain;
3. support-loss relief `>= 3` can recruit the immediately uphill settled grain into an existing moving layer;
4. front baseline flat coast is shorter than momentum-v1, with only a small bounded thickness bonus;
5. a deterministic wall fixture produces both erosion and uphill recruitment, creates at least three simultaneously rolling grains, reaches quiescence, and conserves mass;
6. existing momentum-v1, boundary/vessel, canonical resize, deferred-batch, harness cadence, and command parsing tests remain green;
7. no production/persistence semantics change.

## Human A/B

Compare:

```text
testingcheats model oslo-vessel
testingcheats model oslo-vessel-momentum
testingcheats model oslo-vessel-front
```

Use the same narrow-build → quiesce → large horizontal expansion sequence.

Judge separately:

- **everyday topography:** front must remain recognizably oslo-vessel before a structural failure;
- **onset:** the reconnection should seed immediately rather than waiting for a global event-size gate;
- **mass recruitment:** moving dots should visibly gather additional material rather than merely skate over the bed;
- **uphill failure:** the static wall face should progressively lose support away from the initial seam;
- **runout:** deposition should form a believable toe/skirt without endless flat-floor coasting;
- **visual duration:** a large event should unfold over visible physical time, not teleport and not serialize as isolated Oslo dots;
- **post-event terrain:** after quiescence, the pile should return to useful rough Oslo-vessel character rather than becoming a smooth wedge.

If this still fails, the next distinct step should be a fuller explicit rolling-layer field/partial-fluidization experiment (BCRE/Aranson-Tsimring style), not further tuning of individual rolling-grain coast lengths.
