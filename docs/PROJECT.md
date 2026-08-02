# Strata project authority

Status: accepted product direction
Last reviewed: 2026-08-02

## Purpose

Strata is a general-purpose continuous temporal ledger and active timer expressed through a falling-sand terminal interface.

It may be used for study, habits, projects, work, leisure, creative practice, or other user-defined activities. It is not defined primarily as a freelance or billing tracker.

## Governing concept

Time does not stop when the user is not actively classifying it. Strata continuously represents elapsed time as falling sediment.

- **Idle** is the accepted name for the baseline state of continuous time.
- Selecting a layer gives passing time an active identity, color, and balance direction.
- Active use functions as a timer without suspending the continuous-ledger model.
- Idle time remains part of the sedimentary history while being omitted from ordinary active-time accounting.

The user-facing runtime term is **idle**. Historical `drift` and `none` inputs and internal identifiers may remain as compatibility vocabulary, but they do not define the product concept.

## Session identity

Project and category are independent axes of chronological truth.

- **Project** identifies the subject, client, effort, or context of a session. It is persisted exactly when supplied and may be empty for general TUI-created intervals.
- **Category/layer** identifies the kind of activity, color, and balance direction. CLI starts require an explicit category.
- Neither axis substitutes for the other. A project must not be encoded as a category merely to survive persistence, and an omitted category must not be inferred as idle.
- Idle is category ID `0`, selected deliberately, represented in sediment, and excluded from ordinary active-time totals.

The CLI positional project remains required by the current command surface. Project management, project CRUD, and project-filtered report UI are separate future product questions; preserving project identity does not imply those features already exist.

## Historical truth

Strata preserves two related forms of history:

1. **Chronological truth** — exact intervals, timestamps, durations, layers, notes, project identity, and reportable totals.
2. **Sedimentary truth** — the accountable visual formation produced by those intervals: mass, color composition, topology, contours, neighborhoods, and broad chronology.

The chronological ledger is more precise. The sediment is a visualization, but it is not disposable decoration. Losing or arbitrarily rewriting it damages the user's experienced historical artifact.

The current visual quantum is one grain per elapsed second. A configurable temporal quantum is a future candidate, not current behavior.

## Artistic and functional unity

Strata is both a tool and an artwork. The falling sediment, color mixing, accumulation, idle presence, and physical behavior are product meaning rather than a cosmetic layer around a conventional timer.

Braille characters encode several physical dot positions with one foreground color. Mixed colors at cell seams are therefore an intentional composition of subcell material and are compatible with the behavior of mixed sand.

## Balance

Layers may carry a user-defined positive, negative, or neutral directional value. This supports balance rather than prescribing morality. Work/leisure is one interpretation, not the only one.

The current term `Karma` remains under terminology review. `Balance`, `polarity`, and `valence` are candidate vocabulary; no rename is accepted here.

## Persistence and custody

After explicit migration and activation, one SQLite database is the live authority shared by CLI and TUI. Deterministic CSV remains a public interchange format. Legacy CSV/JSON sources are preserved migration evidence until the user explicitly archives and, separately, removes them through the verified custody workflow.

Authority must fail closed. Strata must not silently redirect work to defaults, fall back from a damaged SQLite authority to stale files, or claim a mutation succeeded before it is durable.

## Open product questions

The following remain unresolved and must not be guessed into implementation:

- the intended macro- and micro-chronological meaning of vertical sediment position;
- whether layers remain flat or gain optional context or relationships;
- the durable artistic meaning of clearing, hiding, compacting, or beginning a new formation;
- final user-facing semantics for crash uncertainty and inferred elapsed time beyond the transactional recovery mechanism already implemented;
- the final name and presentation of the balance system;
- whether and how configurable temporal quantum applies to existing formations.
