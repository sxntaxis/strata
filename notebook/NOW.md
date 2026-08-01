---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Strata is entering a reliability and conceptual-convergence phase after a static audit, SQLite direction, and clarification of its continuous-ledger artistic model.
next: Resolve CONCEPT-001 detach semantics and layer/context model before they are encoded into the SQLite schema or recovery implementation.
---

# NOW — Strata

## Current phase

Strata remains a functioning Rust TUI/CLI prototype at version 0.7.6, but it is not ready to treat its persistence and recovery model as dependable historical custody.

The project is now running two coordinated work lines:

1. **RELIABILITY-001** — move live authority from CSV/JSON to SQLite and resolve confirmed persistence, timekeeping, export, simulation, and lifecycle defects.
2. **CONCEPT-001** — stabilize the product concept and interaction doctrine before reliability work accidentally encodes the wrong model.

Notebook adoption is governance-only. It changes no runtime behavior.

## Accepted product baseline

- Strata is a continuous temporal ledger and an active timer.
- Time always passes; the baseline state is **idle**.
- Idle continues depositing sediment but is omitted from ordinary active-time accounting.
- Strata is general-purpose: study, habits, projects, work, leisure, creative practice, and other user-defined uses.
- The exact chronological ledger and the accountable sedimentary artifact are both historically meaningful.
- Sediment is not disposable decoration.
- All sediment properties matter, but their preservation obligations differ in precision.
- One grain currently represents one elapsed second.
- Color mixing inside a Braille cell is intentional and compatible with subcell composition and physical sand mixing.
- Tools may be art; product evaluation must begin from Strata's intended artistic-functional unity.

## Accepted technical direction

GitHub issue #8 defines SQLite as the future live source of truth with deterministic CSV import/export retained as a first-class feature.

SQLite migration must include:

- one transactional repository used by TUI and CLI;
- durable active-session and recovery state;
- stable identities and foreign-key integrity;
- validated legacy import and preserved pre-migration data;
- visible failure behavior;
- database-aware backup and integrity operations;
- no indefinite dual authority between SQLite and CSV.

## Confirmed issue campaign

Open issues #1 through #28 now cover:

- reporting ranges and semantics;
- project identity and category behavior;
- operational-day allocation;
- unknown and retired layers;
- detach, recovery, resize, and grain conservation;
- SQLite migration and write failure handling;
- active-session lifecycle and profile isolation;
- ICS export and historical snapshot semantics;
- text-entry, terminal, keybinding, timekeeping, rendering, and ordering defects.

`RELIABILITY-001` owns sequencing. Individual issues remain the implementation and closure units.

## Conceptual frontier

The central doctrine is now clearer:

> Time continuously deposits material. Idle is its baseline state. Selecting a layer gives passing time identity, color, and balance direction. The chronological ledger records this exactly; the sediment embodies it with accountable but organic precision.

Still unresolved:

- what vertical position means at macro and micro scales;
- whether layers remain flat or gain optional context/relationships;
- whether `Karma` should become Balance, polarity, valence, or another term;
- whether clearing hides, archives, compacts, destroys, or starts a new formation;
- how deliberate detach differs from unexpected termination;
- how inferred detached time is confirmed, classified, and materialized without waiting;
- how a future configurable temporal quantum interacts with existing formations.

## Current blockers and risks

- Implementing SQLite before conceptual decisions could freeze ambiguous layer, balance, formation, and recovery semantics into the schema.
- Fixing detach only as a performance problem could preserve the wrong user contract.
- Treating sediment as reconstructable decoration could satisfy accounting while violating the project's artistic purpose.
- Treating every exact topology cell as immutable could prevent the sand from remaining a living physical simulation.
- Renaming `drift` requires coordinated domain, storage, migration, report, CLI, TUI, test, and documentation work.

## Next

Resolve the two highest-impact CONCEPT-001 questions before schema design:

1. **Detach contract:** deliberate low-power continuation, crash uncertainty, classification, confirmation, and bounded sediment materialization.
2. **Layer model:** whether a grain has one flat layer only or one primary visual layer plus optional context.

After those decisions, derive the minimum SQLite domain schema and migration proof from accepted product meaning rather than current file structures.
