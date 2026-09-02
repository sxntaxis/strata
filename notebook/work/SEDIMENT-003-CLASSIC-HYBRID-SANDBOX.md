---
id: SEDIMENT-003
kind: work
state: active
created: 2026-09-02
updated: 2026-09-02
authority: working
summary: Compare pre-pause grain physics and a weak-focus rain hybrid on the modern canonical visible-basin architecture before any production sediment redesign.
---

# SEDIMENT-003 — classic / hybrid sandbox

## Provenance

- Production/sandbox base: `b5afc619bace6d9d98ec14ccd065add3689e2e38` (published H4/v5 closure before Oslo experiments).
- Pre-pause grain-law reference: `12fa174356b57345c786f4bdd1187512b335fa1a` (`v0.7.6`, 2026-03-02).
- Development pause observed in history: 2026-03-02 through 2026-08-01.

## Owner direction

The raised-open / discharge-boundary direction is rejected. Strata should not model the visible side walls as sinks or as virtual exterior columns of height zero.

The experiment returns to the useful semantics from two earlier generations:

- pre-pause grain physics: individual grains fall straight down first, otherwise choose one random diagonal, and never leave a closed side wall;
- post-pause spatial architecture: the canonical canvas grows monotonically, the current visible window is the active basin, hidden canonical terrain freezes on shrink, and re-expansion reconnects it naturally;
- selected Oslo-era ingress work may be evaluated independently rather than being coupled to Oslo `{1,2}` slope relaxation.

## Debug-only comparison

This unit is intentionally non-authoritative and non-persistent. Production remains H4/v5 at the baseline.

`testingcheats model h4`
: isolated clone of current authoritative H4 sediment.

`testingcheats model classic`
: empty sandbox using pre-pause grain movement with modern canonical/VW semantics and uniform full-width rain.

`testingcheats model hybrid`
: the same classic gravity law, with ingress only changed to 90% uniform full-width rain plus 10% slow wandering-focus bias inside the accepted golden corridor.

All three can use `testingcheats fallspeed`, `advance`, `clear`, `status`, and `reset`.

## Invariants under evaluation

- Visible-window side edges are closed walls; no grain crosses them.
- Shrinking the viewport does not move, delete, compress, recolor, or relax hidden canonical terrain.
- Expanding the viewport reconnects preserved terrain directly; a resize-triggered avalanche is allowed and desirable evidence of continuity.
- No discharge archive, raised-open boundary, bulk dissipation, or buried layer exists in this sandbox.
- `classic` and `hybrid` differ only in ingress sampling, not in gravity.
- Normal ingress remains approximately one grain per second because the existing testing-cheats scheduler is reused unchanged.

## Human questions

Compare `classic` and `hybrid` after similar simulated ages and at several viewport sizes:

1. Does either recover the older natural sand character better than H4?
2. Do the sides fill naturally without the forced Oslo pyramid/wedges?
3. Are avalanches broad, irregular, and visually legible rather than microscopic or continuous noise?
4. Is the weak focus actually useful, or is uniform rain preferable?
5. Does shrink → evolve → expand preserve the enjoyable reconnection / mega-avalanche behavior?

No production promotion follows automatically from a favorable visual test. The result is evidence for the next sediment design decision.
