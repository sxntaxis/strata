# Domain authority

Status: accepted and certified
Implemented by: DOMAIN-001
Issues: #2, #12
Last reviewed: 2026-08-02

## Purpose

Strata must preserve what a session is about without confusing that identity with how the activity is classified. Project identity, category identity, and idle time therefore have separate contracts.

## Independent identity axes

A canonical session contains:

- a stable session identity;
- an optional project string;
- one category/layer identity;
- description, chronology, duration, source, and operational-day provenance.

The project string answers “for what context or effort?” The category answers “what kind of activity?” They are not aliases and neither may be silently derived from the other.

The CLI currently requires a non-empty positional project. TUI-created general intervals may carry an empty project. Empty means no project was supplied; Strata does not invent a placeholder in canonical history.

## Explicit classification

`strata start` requires `--category <CATEGORY>`. Omission fails before active state is created under either legacy or SQLite authority.

Accepted category selectors include a case-insensitive category name, a category ID, and the explicit baseline name `idle`. Historical `none` and `drift` spellings remain compatibility aliases for category ID `0`; user-facing output and documentation use `idle`.

A normal work session therefore uses an explicit reportable category:

```bash
strata start client-a --category Work
```

A deliberate baseline interval uses:

```bash
strata start break --category idle
```

## Idle contract

Idle is the continuous-ledger baseline category.

- it is represented in sediment;
- it is neutral in balance calculations;
- it is excluded from ordinary active-time totals and ICS work events;
- it is selected explicitly by CLI users rather than inferred from missing classification;
- internal `drift`-named functions, fault identifiers, or keybinding aliases are compatibility mechanics, not user-facing doctrine.

## Persistence and compatibility

SQLite already stores project independently on active and completed sessions. DOMAIN-001 extends that identity through the shared domain model, TUI synchronization, emergency custody export, legacy stop/reload, JSON export, and ICS export.

Legacy `time_log.csv` supports three generations:

1. 8 columns — original chronology without project or absolute temporal provenance;
2. 12 columns — temporal provenance without project;
3. 13 columns — project plus temporal provenance.

Old rows load with an empty project. New rows persist the supplied project exactly. Entirely absent temporal provenance is handled through the existing explicit legacy reconstruction policy; partial provenance fails closed.

No SQLite schema migration is required because project columns were already authoritative. Deterministic SQLite bundles already include project identity.

## Boundaries

DOMAIN-001 does not add:

- project CRUD or a project registry workflow;
- project selection in the TUI;
- project-grouped or project-filtered reports;
- custom report ranges;
- export-format redesign;
- sediment topology changes.

Those features must build on, not reinterpret, the persisted project/category axes.

## Certification

DOMAIN-001 covers:

- legacy start → stop → reload project preservation;
- SQLite active and completed project preservation;
- TUI load/synchronization without project loss;
- JSON and ICS project propagation;
- omitted-category rejection before mutation under both authorities;
- explicit idle and explicit work classification;
- 8-, 12-, and 13-column CSV compatibility;
- strict migration import of project-bearing rows;
- all existing persistence, temporal, recovery, CLI, and TUI gates.
