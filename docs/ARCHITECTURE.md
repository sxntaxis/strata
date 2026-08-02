# Strata architecture authority

Status: current implementation map
Last reviewed: 2026-08-02

## Current system

Strata is one Rust application with two user interfaces:

```text
TUI / CLI
    ↓
shared invocation and validated startup configuration
    ↓
application orchestration
    ↓
domain time, layer, session, report, and recovery rules
    ↓
SQLite repository/runtime coordination + sediment simulation
```

Current responsibility map:

- `src/main.rs` — process entry.
- `src/lib.rs` — shared CLI/TUI invocation, startup configuration validation, and entry-point selection.
- `src/cli.rs` — command parsing, non-interactive lifecycle, reports, exports, migration, and maintenance commands.
- `src/keybindings.rs` — shared keymap and authority/time-setting parsing and validation.
- `src/domain.rs` — canonical session identity, project/category rules, operational-day logic, and report aggregation.
- `src/temporal.rs` — checked wall intervals, monotonic/wall reconciliation, fixed-clock civil policy, operational-day windows, and exact overlap slicing.
- `src/sqlite.rs` and `src/sqlite/**` — schema migration, authoritative repositories, CLI/TUI adapters, runtime coordination, checkpoint transactions, failure certification, deterministic interchange, backup/restore, and legacy-evidence custody.
- `src/storage.rs` — XDG paths, pre-activation legacy compatibility, migration input, atomic file helpers, and legacy runtime-checkpoint files.
- `src/app.rs` and `src/app/**` — TUI orchestration, interaction, rendering, reports, runtime checkpoints, bounded recovery, modals, and persistence-recovery controls.
- `src/sand/**` — canonical logical grains, compressed pending mass, physics, viewport projection, bounded recovery arithmetic, snapshots, and Braille rendering.

## Authority state

The SQLite migration program closed at 9/9 acceptance criteria in issue #8.

- Before explicit activation, legacy CSV/JSON remains the live authority.
- After explicit activation, CLI and TUI share one SQLite authority.
- Activated runtime never dual-writes legacy sources and never falls back to them automatically.
- Runtime transitions are stable-ID fenced, transactional, and retry-safe.
- Persistence failures enter a non-dismissible recovery state rather than continuing with divergent memory.
- Deterministic CSV bundles are interchange, not a competing live ledger.
- Legacy source inventory, archive, and removal are provenance-verified custody operations.

AUTHORITY-001 adds a shared startup gate before either interface resolves or opens data authority:

- one top-level invocation chooses CLI or TUI;
- `keymap.json` is loaded and validated once;
- malformed JSON, unknown key/action data, invalid day/time settings, unsupported UTC offsets, and invalid configured legacy paths stop startup visibly;
- CLI and TUI receive the same validated runtime/storage settings;
- `--ignore-config` is the only deliberate built-in-default bypass;
- TUI hot-reload failures retain the last valid settings rather than applying a partial configuration.

TEMPORAL-001 establishes one explicit temporal authority:

- live elapsed duration is owned by the process monotonic clock;
- UTC owns persisted absolute timestamps;
- a live transition compares observed UTC with the UTC endpoint implied by monotonic elapsed time;
- divergence above five seconds fails closed and preserves active state;
- cross-process recovery uses checked UTC wall intervals because monotonic state cannot survive process death;
- future starts are rejected, and unattended intervals above seven days require explicit CLI confirmation;
- the validated fixed UTC offset owns civil display and new operational-day allocation;
- the operational-day key persisted with a session owns historical report grouping after later setting changes;
- the fixed-offset policy is deliberately not an IANA/DST policy.

The detailed contract and failure matrix are `docs/TEMPORAL_AUTHORITY.md`.

TEMPORAL-002 completes the remaining interval semantics:

- one canonical session identity owns chronology, editing, deletion, and provenance;
- each new session captures its fixed UTC offset and fixed boundary minute;
- reports derive exact overlap slices instead of assigning an entire cross-boundary row to its ending day;
- exact-boundary endpoints create no empty fragments and allocated seconds are conserved;
- the false `sunrise` mode is removed and existing configuration is migrated visibly to fixed-clock policy;
- zero-whole-second finishes and switches create transactional receipts and state transitions but no completed work rows;
- SQLite schema version 5 and bundle schema version 2 preserve the new policy fields.

The SQLite closure evidence is `docs/SQLITE_MIGRATION_CLOSURE_AUDIT.md`.

DOMAIN-001 establishes session-classification authority:

- project identity and category identity are independent canonical session fields;
- CLI starts require an explicit category and cannot silently become idle;
- idle is the user-facing baseline name, explicitly selectable and excluded from ordinary active-time totals;
- historical `none`/`drift` spellings remain compatibility aliases only;
- project survives legacy and SQLite lifecycle paths, TUI synchronization, custody export, JSON, and ICS;
- legacy 8- and 12-column session CSV remains compatible while new 13-column rows preserve project.

The detailed contract is `docs/DOMAIN_AUTHORITY.md`.

REPORT-001 establishes projection authority:

- custom report ranges are inclusive operational-day ranges and consume exact canonical overlap slices;
- the active interval is included by default as a provisional projection without mutating or finalizing it;
- `--completed-only` selects committed history for reports and JSON/ICS exports;
- report and export ordering has complete deterministic tie-breakers;
- JSON schema version 2 exposes stable UIDs, provisional state, and authoritative UTC endpoints;
- ICS uses stable UIDs and UTC endpoints, applies RFC 5545 escaping, CRLF delimiters, and line folding, marks provisional events explicitly, and fails closed when absolute chronology is unavailable;
- idle remains excluded from ordinary reports and ICS work events.

The detailed contract is `docs/REPORT_AUTHORITY.md`.

SEDIMENT-001A establishes sediment mass and geometry primitives:

- terminal-cell dimensions and physical Braille-dot grid dimensions are explicit independent units;
- rendering emits exactly one Braille character per drawable terminal cell;
- each due grain is logical mass before placement and cannot disappear because ingress is occupied;
- randomized ingress examines every available column before declaring complete physical blockage;
- blocked grains remain category-preserving pending mass;
- placed plus pending mass is persisted through `SandState`, SQLite, checkpoints, report previews, and legacy JSON compatibility;
- clearing and category removal apply to placed and pending forms.

SEDIMENT-001B separates canonical sediment from terminal geometry:

- the persisted logical dot grid owns canonical dimensions, coordinates, category neighborhoods, and topology;
- terminal dimensions are presentation-only viewport state;
- terminal resize cannot invoke gravity, repacking, ingress placement, or logical-state mutation;
- rendering uses a horizontally centered, bottom-aligned crop/pad projection;
- grains outside a smaller viewport remain recoverable and reappear when the viewport expands;
- restore installs persisted canonical dimensions and coordinates directly;
- repeated no-time-elapsed viewport oscillation is exactly idempotent;
- the destructive resize module and edge-band policy are removed.

SEDIMENT-001C1 establishes bounded mass representation and periodic arithmetic:

- pending logical mass is stored as ordered category/count runs rather than one allocation per grain;
- adjacent same-category runs merge while category transitions preserve FIFO order;
- bulk logical addition is independent of represented count;
- live ingress work is bounded by free columns, and snapshots scale with physical grains plus run changes;
- `SandState` schema version 2 stores compressed runs and migrates version 1 pending vectors;
- count overflow fails visibly;
- periodic event counts and remainders use checked integer arithmetic without iterative replay.

SEDIMENT-001C2 establishes bounded, durable runtime recovery:

- runtime checkpoints cover periodic autosave, detach, terminal closure, and crash recovery;
- checkpoint evidence is claimed and a recovery target is persisted before first publication;
- checkpoint topology and engine metadata restore directly;
- missed spawn mass is appended as compressed pending runs;
- missed physics frames are counted but never replayed, and no relaxed replacement topology is installed;
- work is independent of detached duration apart from compact arithmetic and state validation;
- SQLite publishes recovered sediment, daily snapshot, active-session continuity, and checkpoint status atomically;
- committed SQLite evidence remains reclaimable until a fresh pending checkpoint replaces it;
- legacy-file authority uses a persisted recovery target and committed marker for deterministic overwrite on retry;
- normal shutdown may retire pending or committed evidence but cannot clear recovering or quarantined evidence;
- runtime checkpoint creation refuses queued mutations, and old mutation-bearing checkpoints fail closed with evidence retained because no stable cross-authority mutation receipt exists;
- invalid schemas, timestamps, identities, coordinates, accumulators, and arithmetic fail closed.

The evolving accepted sediment contract is `docs/SEDIMENT_AUTHORITY.md`.

## Truth boundaries

### Chronological ledger

Owns exact elapsed intervals, timestamps, categories, project identity, notes, operational-day interpretation, and reportable totals.

### Sediment formation

Owns accountable visual history. Total represented duration and per-category mass must be conserved exactly. Canonical topology, contours, color composition, neighborhoods, and broad chronology are independent of the current viewport. Pending mass may be compressed without changing count, category identity, or FIFO category order.

### Runtime recovery

Owns custody of the last durably checkpointed simulation state and exact elapsed contribution since that state. Recovery may add missing logical mass and accumulator remainders, but it may not replay unbounded physics, relax canonical topology, silently reclassify categories, or delete unresolved evidence.

### Reports and balance

Are projections over chronological truth. They may omit idle or apply user-defined polarity, but they must not rewrite underlying intervals.

### Interface

TUI and CLI translate user intent and present state. Neither may maintain an independent ledger, independently reinterpret configuration, silently select a different profile, or mutate canonical sediment merely to fit the terminal.

## Current architectural frontier

Persistence, startup configuration, clock authority, interval boundaries, session classification, report/export projection correctness, sediment mass, viewport-independent topology, and bounded runtime recovery are no longer the primary risks. The next programs are:

1. establish explicit immutable historical snapshot kinds and provenance;
2. complete interaction-mode and terminal-lifecycle contracts;
3. reconcile remaining partially satisfied domain and profile issues.

Complete profile isolation and deliberate runtime profile switching remain separate work under issue #15.

## Non-authority

- GitHub issues describe defects and proposals; they do not override accepted product doctrine.
- Notebook research remains working memory until promoted.
- Sediment snapshots and report output are projections, not substitutes for their owning state.
- Terminal dimensions are presentation state, not canonical sediment dimensions.
- Compressed pending runs are an exact representation of logical mass, not aggregation that permits count or category loss.
- Recovery does not claim safe queued-mutation replay without stable receipts.
- CSV, JSON, and ICS are external adapters, not canonical domain models.
