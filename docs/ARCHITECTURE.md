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
domain time, category, session, report, recovery, and snapshot rules
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
- `src/app.rs` and `src/app/**` — TUI orchestration, interaction, rendering, reports, runtime checkpoints, bounded recovery, historical-preview selection, modals, and persistence-recovery controls.
- `src/sand/engine.rs` — canonical logical grains, compressed pending mass, physics, viewport projection, and Braille rendering.
- `src/sand/recovery.rs` — bounded recovery arithmetic and topology-preserving detached contribution.
- `src/sand/snapshot.rs` — typed snapshot identity, provenance, deterministic source revisions, artifact selection, and immutable rendering.

## Authority state

The SQLite migration program closed at 9/9 acceptance criteria in issue #8.

- Before explicit activation, legacy CSV/JSON remains the live authority.
- After explicit activation, CLI and TUI share one SQLite authority.
- Activated runtime never dual-writes legacy sources and never falls back to them automatically.
- Runtime transitions are stable-ID fenced, transactional, and retry-safe.
- Persistence failures enter a non-dismissible recovery state rather than continuing with divergent memory.
- Deterministic CSV bundles are interchange, not a competing live ledger.
- Legacy source inventory, archive, and removal are provenance-verified custody operations.

AUTHORITY-001 establishes one validated startup gate for CLI and TUI. Invalid configuration blocks authority resolution unless `--ignore-config` is explicitly supplied.

TEMPORAL-001 establishes monotonic live duration, UTC persistence, fixed-offset civil authority, checked wall recovery, discontinuity refusal, and historical operational-day ownership.

TEMPORAL-002 keeps one canonical session identity while reports derive exact operational-day overlap slices; it removes false sunrise semantics and treats zero-whole-second transitions as receipt-only events.

DOMAIN-001 makes project and category independent canonical axes, requires explicit CLI classification, and completes the user-facing idle vocabulary migration.

REPORT-001 establishes inclusive operational-day ranges, explicit provisional active projection, deterministic ordering, JSON schema version 2, and RFC 5545-safe ICS using authoritative UTC chronology.

SEDIMENT-001A establishes sediment mass and geometry primitives:

- terminal-cell and Braille-dot dimensions are explicit independent units;
- each due grain exists as placed or pending logical mass;
- ingress examines every available column before blockage;
- blocked mass remains category-preserving and durable;
- clearing and category removal operate on placed and pending forms.

SEDIMENT-001B separates canonical sediment from terminal geometry:

- the persisted logical grid owns dimensions, coordinates, neighborhoods, and topology;
- terminal dimensions are presentation-only viewport state;
- resize cannot invoke gravity, repacking, ingress placement, or logical mutation;
- projection is horizontally centered and bottom-aligned;
- the destructive resize module and edge-band policy are removed.

SEDIMENT-001C1 establishes bounded mass representation and periodic arithmetic:

- pending mass is stored as ordered category/count runs;
- adjacent same-category runs merge while transitions preserve FIFO order;
- work and storage are independent of represented count;
- `SandState` schema version 2 stores runs and migrates version 1 vectors;
- periodic event counts and remainders use checked integer arithmetic without replay.

SEDIMENT-001C2 establishes bounded durable runtime recovery:

- runtime checkpoints cover autosave, detach, terminal closure, and crash recovery;
- evidence is claimed and a recovery target is persisted before publication;
- checkpoint topology and engine metadata restore directly;
- missed mass is appended as compressed pending runs;
- missed physics is never replayed and no relaxed topology is installed;
- SQLite recovery publication is atomic and reclaimable;
- legacy-file recovery uses deterministic target and committed markers;
- unresolved evidence fails closed and remains protected.

SEDIMENT-001D1 establishes snapshot identity and immutable viewing:

- one `SedimentSnapshot` envelope distinguishes cumulative checkpoints, daily contributions, and derived previews;
- each artifact carries optional day, source revision, provenance, idle policy, reconstruction status, and `SandState`;
- legacy bare daily payloads are classified as cumulative legacy evidence rather than silently treated as daily contributions;
- incompatible or missing daily artifacts fall back to in-memory ledger-derived previews;
- historical rendering uses a fresh viewport engine, never advances physics, and cannot mutate or persist the artifact;
- preview cache identity includes the full serialized artifact and viewport;
- report UI exposes artifact kind, reconstruction status, and idle policy.

The evolving accepted sediment contract is `docs/SEDIMENT_AUTHORITY.md`.

## Truth boundaries

### Chronological ledger

Owns exact elapsed intervals, timestamps, categories, project identity, notes, operational-day interpretation, and reportable totals.

### Sediment formation

Owns accountable visual history. Total represented duration and per-category mass must be conserved exactly. Canonical topology and broad chronology are independent of the current viewport. Pending mass may be compressed without changing count, category identity, or FIFO order.

### Runtime recovery

Owns custody of the last durably checkpointed simulation state and exact elapsed contribution since that state. Recovery may add missing logical mass and accumulator remainders, but may not replay unbounded physics, relax topology, silently reclassify categories, or delete unresolved evidence.

### Historical snapshots

A snapshot envelope owns semantic identity and provenance for a visual artifact. `CumulativeCheckpoint`, `DailyContribution`, and `DerivedPreview` are not interchangeable. Viewing is immutable and projection-only. Chronological sessions remain the reconstruction authority for derived previews.

### Reports and balance

Are projections over chronological truth. They may omit idle or apply user-defined polarity, but they must not rewrite underlying intervals or persist a derived preview as authentic history without an explicit authority transition.

### Interface

TUI and CLI translate user intent and present state. Neither may maintain an independent ledger, independently reinterpret configuration, silently select another profile, mutate canonical sediment to fit the terminal, or advance historical snapshots while viewing them.

## Current architectural frontier

Persistence, startup configuration, clock authority, interval boundaries, session classification, report/export correctness, sediment mass, viewport topology, bounded runtime recovery, explicit snapshot identity, and immutable historical viewing are no longer the primary risks.

The next programs are:

1. complete authoritative daily-contribution persistence, revision comparison, mutation invalidation, and legacy snapshot disposition in SEDIMENT-001D2;
2. complete interaction-mode and terminal-lifecycle contracts;
3. reconcile remaining partially satisfied domain and profile issues.

Complete profile isolation and deliberate runtime profile switching remain separate work under issue #15.

## Non-authority

- GitHub issues describe defects and proposals; they do not override accepted product doctrine.
- Notebook research remains working memory until promoted.
- Terminal dimensions are presentation state, not canonical sediment dimensions.
- Compressed pending runs are exact logical mass, not permission for count or category loss.
- Recovery does not claim safe queued-mutation replay without stable receipts.
- A legacy cumulative daily row is evidence, not an authoritative daily contribution.
- A derived preview is a deterministic view, not persisted sediment authority.
- CSV, JSON, and ICS are external adapters, not canonical domain models.
