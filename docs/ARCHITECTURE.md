# Strata architecture authority

Status: current implementation map
Last reviewed: 2026-08-20

## System shape

```text
TUI / CLI
    ↓
validated invocation + one process-bound profile
    ↓
application/domain operations
    ↓
SQLite runtime authority + sediment engine
```

Core responsibilities:

- `src/profile.rs` — profile UUID and complete data/state/config ownership.
- `src/cli.rs` — CLI commands, reports, projections, and maintenance entry points.
- `src/command.rs` — small in-TUI direct command language shared with live control intents.
- `src/ipc.rs` — profile-scoped Unix-socket transport to a running TUI; never persistence authority.
- `src/domain.rs` — categories, sessions, operational-day/report rules.
- `src/temporal.rs` — monotonic/wall-clock reconciliation and exact overlap slicing.
- `src/sqlite.rs`, `src/sqlite/**` — the one current schema, repository/runtime transactions, interchange, backup/restore/doctor, and fault certification.
- `src/app.rs`, `src/app/**` — TUI orchestration, explicit interaction modes, recovery, and rendering.
- `src/sand/**` — canonical sediment, recovery arithmetic, snapshots, and viewport rendering.

## Persistence and profile authority

- SQLite is the sole live persistence authority.
- There is one current schema and no historical upgrade chain.
- CLI and TUI open the same profile-bound database.
- `--profile <directory>` or `STRATA_PROFILE` selects one complete profile before config/database access.
- Partial runtime-ledger redirection such as `time_log_path` is rejected.
- Portable CSV is deterministic interchange only.
- A copied database or state artifact bound to another profile fails closed.

## Session model

- Category/layer is the canonical reportable activity axis.
- Session description/tag is interval text, separate from durable category metadata.
- There is no independent canonical `project` field.
- Idle is explicit category ID `0` and keeps the continuous ledger running.
- `stop` transitions to idle rather than leaving time unclassified.
- Live elapsed time uses the monotonic clock; persisted chronology uses UTC.
- Completed sessions remain singular; report/day projections slice exact overlaps without rewriting history.

## Category identity

- Active and archived categories share one stable ID space.
- Archive changes availability, not historical meaning.
- Restore reactivates the same row/ID.
- Archived metadata remains available to sessions, reports, sediment, snapshots, tags, recovery, and interchange.
- Unknown category references fail closed and are never coerced to idle.
- Category merge/permanent deletion is not a current product capability.

## TUI interaction

The product remains keyboard-first and keeps the continuous sand view as its center.

- `Ctrl-P` opens a hybrid palette: valid direct commands execute directly; otherwise the same text remains fuzzy search and Enter executes the selected result.
- Informational direct commands keep the palette open and show their result.
- The category modal is compact in ordinary use; durable metadata has an explicit edit mode.
- Report history is read-only until explicit edit mode; SQLite persistence succeeds before the in-memory row changes.
- The configured keymap remains truthful about bound/unbound/disabled actions. Ctrl-C is the mandatory terminal-safety quit path.

## Live CLI control

A running TUI owns in-memory simulation/session state that cannot safely be bypassed by another process.

On Unix, `state/runtime.sock` beneath the selected profile is an ephemeral control transport:

- same-profile CLI `status`, `start`, and `stop` route to the running TUI;
- the TUI executes the same current domain/SQLite operations as local input;
- a second TUI for the same profile is refused;
- stale/unreachable socket evidence falls back to headless SQLite operation;
- a different profile resolves a different socket and database;
- request/response size is bounded and response IDs must match requests.

Without a running TUI, CLI mutations operate headlessly against SQLite with the same layer/idle semantics.

## Recovery boundaries

Checkpoint evidence belongs to one active stable generation and has explicit pending/recovering/committed/quarantined custody.

SQLite switch, finish, and reset transitions retain durable runtime receipts because their committed database transition is followed by in-memory/checkpoint reconciliation that must be idempotent after failure.

Clear-all is different: active identity (when reset), empty sediment, affected daily contributions, and the resulting checkpoint are already one `IMMEDIATE` SQLite transaction. It therefore has no second clear-all replay receipt.

A fresh TUI bootstraps its initial active generation and checkpoint atomically. Runtime recovery uses a persisted cutoff and bounded sediment arithmetic rather than replaying missed physics frame-by-frame. Persistence failures freeze ordinary mutation and expose retry/reload/emergency-custody actions.

See `docs/RECOVERY_AUTHORITY.md`.

## Sediment authority and resize

- Every logical grain is either placed or pending; mass/category identity are conserved.
- The logical sand canvas is persisted independently from terminal dimensions.
- Shrinking a terminal changes only the viewport/projection.
- Growing beyond the current logical canvas expands it monotonically, preserving existing cells around the horizontal center and bottom baseline and filling new space with emptiness.
- When live viewport widening removes a temporary lateral wall, only that former wall's exact bottom-connected surface grain may receive a one-shot H4 mobility trigger, and only if newly exposed outward space has dynamic relief `>1`; resize itself never reflows grains or changes mass.
- The logical canvas does not shrink again merely because the viewport shrinks.
- Pending grains may occupy newly available capacity after expansion.
- Historical artifacts remain immutable projections of stored or reconstructed sediment state.

This preserves responsive v0.7.7-style artwork without making terminal size destructive persistence authority.

## Reports and exports

- Reports are deterministic projections over canonical sessions.
- Active time is provisional by default; `--completed-only` excludes it.
- Custom date ranges are inclusive operational-day ranges.
- JSON projection schema 3 carries category/session identity and no independent project field.
- ICS uses authoritative UTC endpoints and omits idle events.
- Portable SQLite bundles remain the full-fidelity interchange path.

## Maintenance

- `sqlite-doctor` validates current schema, integrity, references, and profile binding.
- `sqlite-backup` creates a verified standalone backup.
- `sqlite-restore` verifies before atomic replacement.
- `sqlite-export`/`sqlite-import` use the current deterministic bundle schema only.

## Authority rule

Keep an abstraction only when it owns a real invariant or failure boundary. Development-only migration, legacy runtime authority, speculative destructive lifecycle, and unused domain axes are not product architecture.
