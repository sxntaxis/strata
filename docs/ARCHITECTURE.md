# Strata architecture authority

Status: current implementation map
Last reviewed: 2026-08-02

## Current system

```text
TUI / CLI
    ↓
shared invocation and validated startup configuration
    ↓
terminal lifecycle + application orchestration + explicit interaction modes
    ↓
domain time, category, session, report, recovery, and snapshot rules
    ↓
SQLite repository/runtime coordination + sediment simulation
```

Current responsibility map:

- `src/main.rs` — process entry.
- `src/lib.rs` — shared CLI/TUI invocation and startup authority.
- `src/cli.rs` — command lifecycle, reports, exports, migration, and maintenance.
- `src/keybindings.rs` — validated runtime/time settings plus configured Bound/Unbound/Disabled action state, mandatory key policy, contextual aliases, and the shared input resolver.
- `src/domain.rs` — canonical sessions, project/category identity, operational-day allocation, and reports.
- `src/temporal.rs` — monotonic/wall reconciliation, fixed-offset civil policy, and exact overlap slicing.
- `src/sqlite.rs` and `src/sqlite/**` — schema migrations, repositories, runtime transactions, checkpoint custody, deterministic interchange, backup/restore, and fault certification.
- `src/storage.rs` — XDG paths, legacy-file authority, atomic file helpers, and custody-separated contribution files.
- `src/app.rs` and `src/app/**` — TUI orchestration, explicit modal/edit state, persistence reconciliation, bounded recovery, historical artifact selection, context selection, resolver execution, palette/atlas projection, and rendering.
- `src/app/terminal_lifecycle.rs` — raw-mode/alternate-screen RAII, process-wide panic restoration, exactly-once cleanup, runtime failure composition, and debug fault certification.
- `src/sand/engine.rs` — canonical logical grains, compressed pending mass, physics, viewport projection, and Braille rendering.
- `src/sand/recovery.rs` — bounded recovery arithmetic and topology-preserving detached contribution.
- `src/sand/snapshot.rs` — snapshot kinds, exact daily contribution construction, provenance, revisions, selection, and immutable rendering.

## Established authority

### Persistence and startup

- SQLite becomes live authority only after explicit activation.
- CLI and TUI share one validated startup configuration.
- Activated runtime never dual-writes or silently falls back to legacy sources.
- Persistence and authority failures fail closed with visible recovery controls.
- Deterministic CSV bundles are interchange, not a competing live ledger.

### Time and sessions

- Live duration is monotonic; UTC owns persisted absolute chronology.
- Fixed-offset civil policy owns new operational-day projection.
- Canonical sessions remain singular while exact overlap slices allocate report and daily-contribution mass across operational days.
- Project and category are independent canonical axes.
- Idle is explicit, continues producing sediment, and remains excluded from ordinary active-time totals.

### Reports and exports

- Report ranges are inclusive operational-day projections.
- Active time is included by default as explicit provisional state; `--completed-only` selects committed history.
- Ordering is deterministic.
- JSON schema version 2 and RFC 5545-safe ICS use stable identities and authoritative UTC endpoints.

### Sediment authority

- Every due grain is exactly one placed or pending logical grain.
- Pending mass uses ordered category/count runs.
- Terminal-cell and Braille-dot dimensions are distinct.
- The persisted logical grid owns canonical topology.
- Resize is projection-only.
- Runtime recovery is bounded, topology-preserving, and evidence-safe.
- Historical artifacts have explicit cumulative, daily, or derived identity.
- Historical viewing is immutable.
- Daily contributions derive from exact canonical session slices and are trusted only on revision match.
- SQLite schema version 6 and distinct legacy-file paths preserve old cumulative daily evidence without reinterpretation.

The detailed sediment contract is `docs/SEDIMENT_AUTHORITY.md`.

### Explicit report editing

Report-log view and report-description editing are separate interaction modes.

- View mode is read-only and retains normal command routing.
- Confirm on a persisted report row creates a draft owned by the stable session ID.
- In edit mode, every unmodified character—including command letters, spaces, and Unicode—is draft text.
- Enter requests one persistence commit; Esc discards the complete draft.
- Modified input is ignored unless mandatory policy resolves it to emergency Quit.
- SQLite or legacy-file persistence succeeds before memory changes.
- Failed persistence retains the complete draft and enters visible recovery.
- The report UI exposes VIEW versus EDIT state and the live draft cursor.

### Keymap and action authority

The keymap owns configured action state and one context-aware resolver.

- Every action is exactly Bound, Unbound, or Disabled.
- Null physical-key entries remove only that key; `unbind_actions` is the Disabled marker.
- Contradictory bound-and-disabled configuration fails closed.
- Ctrl-C Quit is the sole mandatory key policy, separate from configurable bindings, and remains under persistence-recovery custody.
- F1 is an ordinary configurable default and has no physical-event bypass.
- Contextual behavior is represented by named aliases with explicit conditions.
- Disabled targets are unreachable through direct keys, aliases, and the command palette.
- Event handlers execute one resolver result and do not inspect another action's configured keys.
- Atlas rows and control hints project direct, unbound, disabled, mandatory, and contextual runtime truth.
- Atlas editing distinguishes Disable from Unbind.

### Terminal lifecycle authority

`TerminalSession` is the sole owner of raw mode, alternate-screen state, cursor restoration, output flushing, and the ratatui terminal.

- Acquisition and partial-startup failure share one RAII cleanup boundary.
- Explicit close, `Drop`, and the process-wide panic hook converge on idempotent `restore_once()` state.
- Cleanup attempts all applicable restoration operations and aggregates failures.
- Application finalization remains separate from terminal restoration.
- Draw, poll, and read errors enter an outer runtime-failure boundary.
- Runtime I/O failure attempts one direct emergency checkpoint before returning.
- The original I/O error kind and message remain primary; checkpoint and cleanup outcomes are attached as context.
- Panic restores the terminal before delegating to the previous hook and does not claim application persistence.
- Linux PTY tests verify exact termios restoration and one cleanup execution on normal quit, detach, draw/poll/read failure, and panic.

The complete interaction contract is `docs/INTERACTION_AUTHORITY.md`.

## Truth boundaries

### Chronological ledger

Owns exact elapsed intervals, timestamps, categories, projects, descriptions, operational-day policy, and reportable totals.

### Sediment formation

Owns accountable visual history and canonical topology. It must conserve mass and category identity while remaining independent of the current viewport.

### Runtime recovery

Owns checkpoint evidence and exact elapsed contribution since the checkpoint. It may add mass and advance accumulator remainders, but may not replay unbounded physics, relax topology, or discard unresolved evidence.

### Historical snapshots

Own semantic identity and provenance for persisted or derived visual artifacts. A derived preview is a read-only projection; a daily contribution becomes authority only through explicit typed persistence.

### Interaction

Input routing owns navigation, commands, draft text, commit, cancel, contextual aliases, and mandatory emergency control. Draft state is not canonical history until one successful commit. Configured action state and visible atlas/palette claims must resolve through the same keymap authority.

### Terminal lifecycle

The terminal guard owns host-terminal acquisition and restoration. The application loop owns domain finalization and checkpoint attempts. Cleanup context may annotate an application or runtime error but may not replace its primary cause.

### Interface

TUI and CLI translate user intent and present state. Neither may own an independent ledger, reinterpret authority, mutate canonical sediment to fit the terminal, advance historical artifacts while viewing them, mutate history through ambiguous focus, invent fallback input routes, mislabel unreachable commands, or leave the host terminal in application mode after control exits Strata.

## Current architectural frontier

Persistence, temporal, domain, report, sediment, and interaction authority are complete. The next priorities are:

1. criterion-by-criterion reconciliation of partially satisfied issues #5, #10, and #13;
2. later domain/UI distinction work under issue #22;
3. later profile authority, including complete isolation and deliberate switching under issue #15.

## Non-authority

- GitHub issues do not override accepted doctrine.
- Notebook research is working memory until promoted.
- Terminal dimensions are not canonical sediment dimensions.
- A derived preview is not persisted authority.
- Legacy cumulative daily rows/files are evidence, not daily contributions.
- An uncommitted edit draft is not canonical session history.
- A panic cleanup is not evidence of successful application persistence.
- A contextual alias is not a direct physical binding.
- An unbound action is not a disabled action.
- CSV, JSON, and ICS are external adapters, not canonical domain models.