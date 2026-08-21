# Domain authority

Status: accepted and certified
Last reviewed: 2026-08-20

## Purpose

Strata is a continuous temporal ledger whose reportable activity classification is the layer/category. The canonical model must preserve session identity and chronology without inventing a second product axis that the TUI does not expose.

## Canonical session

A completed session contains:

- one stable session identity;
- one category/layer identity;
- the session description/tag;
- authoritative UTC start and end;
- elapsed duration;
- source and operational-day boundary provenance.

There is no independent canonical `project` field. Earlier prerelease CLI syntax used a positional value named `project` as a fallback layer name; it was not a separate user-facing model and is not preserved in the current schema.

## Layer classification

`strata start <LAYER>` resolves a case-insensitive layer name or numeric category ID. `idle` is the explicit continuous-ledger baseline and canonical category ID `0`.

```bash
strata start Work --desc "deep focus"
strata stop
```

`stop` transitions the ledger back to idle. It does not create unclassified wall time.

## Idle contract

Idle:

- remains represented in the active runtime and sediment;
- is neutral in balance calculations;
- is excluded from ordinary active-time totals and ICS work events;
- owns category ID `0`;
- is never inferred from a missing or unknown category reference.

Internal historical `drift` naming may remain where changing it would add no product value, but current user-facing doctrine is `idle`.

## Persistence

SQLite stores category identity directly on active and completed sessions. TUI, headless CLI, and live CLI-to-TUI control use the same category/session semantics. Portable bundles and JSON/ICS projections carry category identity without a separate project field.

A copied database bound to another profile, an unknown category reference, malformed chronology, or unsupported schema fails closed rather than being coerced into idle or another category.

## Product boundary

Strata may be used *for projects*, study, habits, work, leisure, or any other activity. That does not make “project” a canonical storage dimension. If future product requirements need grouping above/beside layers, that must be designed from an actual workflow rather than inferred from the retired prerelease field.
