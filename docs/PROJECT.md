# Strata product authority

Status: accepted product direction
Last reviewed: 2026-08-27

## Purpose

Strata is a general-purpose continuous temporal ledger and active timer expressed through a falling-sand terminal interface.

It may be used for study, habits, projects, work, leisure, creative practice, or other user-defined activities. It is not defined primarily as a freelance, billing, or project-management tracker.

## Governing concept

Time does not stop when the user is not actively classifying it. Strata continuously represents elapsed time as falling sediment.

- **Idle** is the baseline state of continuous time.
- Selecting a **layer** gives passing time an active identity, color, and balance direction.
- `stop` returns to idle; it does not create a hole in chronology.
- Idle remains part of sedimentary history while being omitted from ordinary active-time accounting.

## Session identity

The current product has one reportable classification axis: category/layer. Session description/tag captures the immediate text attached to that interval.

There is no independent canonical `project` axis. The old prerelease positional CLI parameter named `project` functioned as a layer fallback, while the TUI never had a project workflow. The later independent field was therefore retired instead of being promoted into the UX merely to justify its schema presence.

Future grouping or context above layers is an open product question and requires its own workflow evidence.

## Historical truth

Strata preserves two related forms of history:

1. **Chronological truth** — exact intervals, timestamps, durations, layers, notes, and reportable totals.
2. **Sedimentary truth** — the accountable visual formation produced by those intervals: mass, color composition, topology, contours, neighborhoods, and broad chronology.

The chronological ledger is more precise. Sediment is a visualization, but not disposable decoration.

## Artistic and functional unity

The falling sediment, color mixing, accumulation, idle presence, and physical behavior are product meaning rather than a cosmetic wrapper around a conventional timer.

The logical sand canvas may expand when a terminal grows but does not shrink merely because the viewport shrinks. This keeps the artwork responsive without making terminal dimensions destructive persistence authority.

## Balance

Layers may carry a positive, negative, or neutral directional value. **Balance** is the accepted product vocabulary for the historical/report surface and this directional accounting. The former `Karma` name is retired from current product vocabulary. The default main-view opener is `b`.

## Persistence and live control

Each selected profile owns one `data/strata.sqlite3` database shared by CLI and TUI. SQLite is the sole live persistence authority.

When a TUI is running, mutating CLI commands are delivered through a profile-scoped Unix socket so the TUI executes them through the same in-memory and SQLite transition path. The socket is ephemeral transport/discovery, not persistence authority. Without a live TUI, the CLI uses the same SQLite domain semantics headlessly.

Portable CSV remains interchange only. Doctor, backup, restore, recovery, checkpoints, and transition receipts operate on SQLite directly.

## Open product questions

- final vertical chronology semantics beyond the accepted bottom-aligned viewport projection;
- whether layers remain flat or gain optional context/relationships;
- durable artistic semantics for clearing, hiding, compacting, or beginning a new formation;
- configurable temporal quantum for existing formations.
