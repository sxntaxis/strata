# Recovery authority

Status: accepted and certified
Last reviewed: 2026-08-25

## Purpose

Recovery preserves coherence between active-session identity, chronological history, runtime checkpoint evidence, sediment state, and user-visible uncertainty across detach, process death, runtime failure, restart, and retry.

SQLite is the sole live persistence authority. Recovery never falls back to CSV/JSON runtime files.

## Checkpoint custody

Runtime checkpoints have explicit status:

- `pending` — current recoverable evidence;
- `recovering` — claimed by a recovery attempt;
- `committed` — recovered authority has been published but evidence remains until replacement/retirement;
- `quarantined` — malformed or incompatible evidence that must remain protected.

Checkpoint evidence identifies the active stable generation it belongs to. Missing, mismatched, malformed, recovering, or quarantined evidence cannot be silently applied to a replacement generation.

## Active transitions and receipts

Switch, finish, and reset are SQLite transactions with stable operation identities. Their runtime-transition receipts remain because they provide idempotent evidence across a durable transition and the application's subsequent in-memory/checkpoint reconciliation boundary.

A transition:

1. validates the expected active stable ID;
2. converges on an existing matching receipt after retry when appropriate;
3. validates prior checkpoint custody;
4. writes completed history when whole-second work exists;
5. installs/removes the active generation as required;
6. records the transition receipt;
7. commits atomically.

Current-generation checkpoint publication then makes the new in-memory/sediment state recoverable. Failure after the SQLite transition enters visible persistence recovery instead of pretending the full runtime boundary completed.

Receipts are not generic ceremony: they remain only for transitions with this concrete retry/reconciliation boundary.

Runtime transition operation IDs and TUI replacement active-generation IDs are deterministic and bounded. A
transition derives a fixed 128-bit token from length-prefixed transition kind, predecessor stable ID, exact
UTC timestamp, and discriminator; the predecessor is never embedded recursively in the successor. Existing
historical or currently active oversized IDs remain valid predecessor evidence and are not rewritten in
place. The next switch or idle-reset clear cuts that lineage to the bounded format in one transition, while
finish removes the active generation with a bounded receipt identity. This preserves retry fencing without
turning ordinary use into quadratic metadata growth.

## Clear-all is one SQLite transaction

`Clear all sand and reset idle timer` is not ledger deletion and no longer has a separate clear-all receipt.

One `IMMEDIATE` transaction validates expected active identity and publishes together:

- resulting active generation when idle is reset;
- empty canonical sediment;
- explicit affected daily-contribution replacements/deletions;
- resulting runtime checkpoint.

Committed session history is preserved. A non-idle active session keeps its category, description, start, and stable identity. An idle active interval may be replaced by a new idle generation at the clear timestamp.

Because all durable clear-all effects are in the same SQLite transaction, a second prepared/replay receipt would duplicate transaction atomicity. Fault injection remains the proof that pre-commit failure rolls every authority back and committed state is coherent.

## Initial TUI generation

A fresh TUI bootstrap creates its first active generation and first checkpoint in one transaction after sediment restoration/validation. Pre-existing incompatible active/checkpoint evidence blocks bootstrap. A failed bootstrap cannot leave one side durable without the other.

## Exact transition-edge sediment

Before a switch, finish, reset/clear, detach, or live mutation at a known timestamp, sediment settles through that exact boundary under the outgoing category.

- mass due exactly at the boundary belongs to the outgoing interval;
- later mass belongs to the resulting category;
- checked arithmetic and compressed pending runs avoid replay proportional to missed seconds;
- live backlog beyond eight seconds uses this bounded settlement instead of a long accelerated replay;
- a mutation requested during catch-up settles to its exact UTC boundary and applies immediately rather than entering a live mutation queue;
- detach settles to its exit boundary before publishing the detached checkpoint;
- periodic autosave defers while catch-up is active and resumes once the runtime is coherent;
- canonical mass/category identity is conserved;
- an invalid sediment transition blocks the product mutation and enters recovery.

## Bounded checkpoint recovery

Recovery uses one persisted cutoff target and bounded arithmetic; it never replays missed physics frame-by-frame. STRATA-D081 separates **unobserved process/offline time** from a live database write failure.

When startup finds a recoverable checkpoint whose durable simulation time precedes the fixed recovery target, that interval is a **recovery gap** rather than automatically classified work. Strata restores only the durable checkpoint evidence, pauses new simulation classification at the gap boundary, and presents the universal `WARNING` overlay before any missing-time chronology/sediment is published. The instrument shows `●────────?────────▶`: the left value is the durable save time, the center value is the gap duration, and the right value is `now`. The prompt is `Choose how to reconstruct the missing time.`

The owner chooses one of:

- `Reconstruct as <Layer>` — classify the gap as the previously active Layer and catch sediment up with that identity;
- `Reconstruct as Idle` — classify the gap as Idle and catch sediment up as Idle;
- `Keep sediment unchanged` — account the missing interval as Idle while retaining the durable sediment state without adding catch-up sediment.

If the previously active Layer already is Idle, the duplicate first/second choice collapses to one `Reconstruct as Idle` action. A classification is committed before the recovered in-memory authority is installed; failed persistence leaves the same unresolved warning and original checkpoint state available for another decision/retry. Retry retains the fixed target rather than moving reconstructed history forward with wall time.

## Live persistence degradation

A database write failure while Strata remains alive is not an unobserved gap. Runtime/session time and sediment continue because Strata is still observing that interval; only durability is degraded. Short automatic retries occur before interruption. If they remain unsuccessful, Strata shows one universal `ERROR` for that incident with the last durable-save time and elapsed at-risk duration on `●────────────────?`, plus `Continue`, `Retry now`, `Export recovery`, and `Exit without saving`.

`Continue` dismisses the dialog into visible degraded-saving state and background retries continue with bounded backoff without repeatedly reopening the same incident. A successful retry flushes current authority and clears degraded state. A later independent failure may open a new incident. `Exit without saving` is deliberately available from the already-explicit error surface and does not add a second confirmation dialog. Emergency custody export remains a structured JSON evidence artifact, not a second runtime authority or supported portable import format.

## Terminal/runtime failure

Draw, poll, and read failures attempt one emergency checkpoint before terminal restoration. The original runtime error remains primary; checkpoint/cleanup results are context. Panic restoration returns the terminal to normal state without claiming persistence success.

## Unsupported historical/future extension

Current runtime does not queue user mutations merely to wait for catch-up. A checkpoint carrying queued mutation evidence still requires a stable cross-authority identity that Strata does not implement. Such evidence fails closed; Strata does not infer or replay intent from ambiguous state.
