# Strata architecture authority

Status: current implementation map
Last reviewed: 2026-08-04

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
SQLite repository/runtime coordination + legacy catalog/checkpoint/receipt custody + sediment simulation
```

Current responsibility map:

- `src/main.rs` — process entry.
- `src/lib.rs` — shared CLI/TUI invocation, process-bound profile selection, and startup authority.
- `src/profile.rs` — stable profile UUID, rooted/XDG data-state-config ownership, manifest validation, and cross-profile artifact refusal.
- `src/cli.rs` — command lifecycle, reports, exports, migration, and maintenance.
- `src/keybindings.rs` — validated runtime/time settings plus configured Bound/Unbound/Disabled action state, mandatory key policy, contextual aliases, and the shared input resolver.
- `src/domain.rs` — canonical sessions, project/category identity, session-owned active draft text, operational-day allocation, reports, and cloneable staged legacy transition state.
- `src/category_lifecycle.rs` — storage-neutral category identity counting and remapping for sediment, snapshots, and receipt-free runtime checkpoint payloads.
- `src/temporal.rs` — monotonic/wall reconciliation, fixed-offset civil policy, and exact overlap slicing.
- `src/legacy_transition.rs` — schema-versioned legacy transition receipts, completed-session payload validation, and exact/idempotent session reconciliation.
- `src/legacy_category_lifecycle.rs` — complete legacy category-reference inventory, exact-result prepared receipts, permanent lifecycle ledger, deterministic replay, and retired-ID allocation.
- `src/sqlite.rs` and `src/sqlite/**` — schema migrations, category archival and lifecycle transactions, repositories, active/checkpoint transition transactions, checkpoint custody, deterministic interchange, backup/restore, and fault certification.
- `src/storage.rs` — selected-profile paths, strict legacy active/archived category catalog, strict session identity/reference validation, atomic file helpers, legacy runtime checkpoint files, and custody-separated contribution files.
- `src/app.rs` and `src/app/**` — TUI orchestration, active/archived category projections, semantic-edge checkpoint refresh, legacy switch/finish/clear-all and category-lifecycle receipt publication/replay, blocking revision-bound lifecycle review, explicit modal/edit state, persistence reconciliation, bounded recovery, historical artifact selection, context selection, resolver execution, palette/atlas projection, and rendering.
- `src/app/recovery_statement.rs` — blocking recovery-evidence acknowledgment, deterministic cutoff presentation, exact/reconstructed/provisional classification, and shared projection of the structured recovery statement.
- `src/app/terminal_lifecycle.rs` — raw-mode/alternate-screen RAII, process-wide panic restoration, exactly-once cleanup, runtime failure composition, and debug fault certification.
- `src/sand/engine.rs` — canonical logical grains, compressed pending mass, physics, viewport projection, and Braille rendering.
- `src/sand/recovery.rs` — bounded recovery arithmetic, topology-preserving detached contribution, and exact transition-boundary settlement with separate initialized/uninitialized canvas policy.
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

### Active draft and category metadata

The active session description and durable category description are separate authorities.

- `TimeTracker` owns one active-session draft independently from the category catalog.
- Starting or switching to a category begins with an explicit draft supplied by the interaction; category metadata is never inherited implicitly.
- Finishing commits the active draft into the completed session and clears only the draft.
- Switch, finish, clear, detach, recovery, lifecycle reassignment, and legacy replay preserve durable category metadata.
- SQLite persists active-description edits against the expected active stable ID before memory changes are accepted.
- Legacy checkpoints and transition receipts carry the active draft independently from the unchanged category catalog.
- The TUI defaults to draft editing; configurable `Shift-E` enters the separate durable metadata-editing mode.
- Reusable category tags remain selectable aids and are not auto-applied as session text.

### Category identity and archival

Category retirement changes availability, not identity or historical meaning.

- Active and archived categories share one stable ID space.
- Reports, exports, session serialization, karma, sand, snapshots, daily contributions, and tags resolve both active and archived metadata.
- SQLite persists archival state through `archived_at_utc` and restricts referenced destructive deletion.
- Legacy `categories.csv` accepts the historical five-column active-only schema and writes a six-column active/archived catalog.
- Legacy catalog parsing rejects malformed, duplicate, reserved, or out-of-range identity and metadata.
- Session category references must resolve to active or archived metadata; malformed or unknown IDs fail closed with the original value preserved.
- Explicit ID 0 remains intentional idle; unresolved references are never converted to idle.
- Archive and restore preserve stable ID, name, description, color, karma effect, and tags.
- Legacy-to-SQLite migration retains archived state and original session foreign keys.
- SQLite schema version 7 owns reviewed category lifecycle receipts.
- Merge/reassignment requires an explicit source and target plus a deterministic complete-reference preview; the preview is recomputed inside one immediate transaction and stale confirmation fails closed.
- The transaction reassigns category identity across completed and active sessions, tags, canonical sediment, snapshots, regenerated daily contributions, and receipt-free checkpoint payloads while preserving non-category identity, chronology, target metadata, mass, and FIFO order.
- Permanent deletion without reassignment requires zero references across the same complete inventory.
- Lifecycle receipts retire source IDs permanently; creation, backup/restore, portable bundle schema 3, import validation, and doctor integrity preserve that custody.
- Protected, malformed, or transition-receipt-bearing checkpoint evidence blocks lifecycle mutation.
- Legacy lifecycle preparation stages exact resulting catalog, sessions, tags, canonical sediment, affected daily contributions, receipt-free checkpoint payload, and permanent ledger before publishing one prepared receipt.
- Startup replays that prepared receipt before ordinary authority load, accepts exact already-published artifacts, rejects conflict, and removes evidence only after every result and the permanent ledger converge.
- The TUI keeps ordinary archive on `x`; a distinct configurable `Shift-X` action requires explicit target/deletion selection, displays the complete revision-bound preview, and applies only after the exact phrase is typed.
- The permanent legacy ledger migrates into SQLite receipts and preserves the shared identity high-water mark.

The detailed category contract is `docs/CATEGORY_AUTHORITY.md`.

### Active-session and checkpoint recovery

A runtime checkpoint belongs to one active-session generation.

- SQLite initial startup publishes the first active row and first pending checkpoint in one transaction after sediment restoration; existing active or checkpoint evidence blocks bootstrap.
- SQLite switch, reset, and finish validate the expected active stable ID and prior checkpoint custody inside the same transaction.
- Only `pending` or `committed` evidence for the expected prior identity may be retired by an ordinary transition.
- `recovering`, `quarantined`, missing-identity, or mismatched evidence blocks the transition before completed history or replacement active state changes.
- Idempotent transition receipts return before touching a later checkpoint generation.
- SQLite startup validates checkpoint identity against authoritative active state before applying recovery payload state; incompatible evidence is quarantined.
- Application orchestration immediately publishes current-generation evidence after successful switch, reset, or persisted active-description change.
- Unrelated category metadata does not trigger checkpoint publication.

Legacy switch transitions use a certified multi-file receipt protocol:

1. stage the resulting state;
2. publish schema-3 checkpoint evidence with a deterministic switch receipt;
3. publish completed session history;
4. publish the unchanged category catalog while the resulting active draft remains receipt/checkpoint-owned;
5. clear the receipt only after convergence.

Prepared-checkpoint failure rolls back staged memory. Once the receipt is durable, startup replays missing effects idempotently, exact-matches already published sessions, rejects conflicts, and retains the receipt after later publication failures. The real publication helper is certified from receipt-only, receipt-plus-session, and receipt-plus-session-plus-catalog crash states.

Whole-second ledger semantics own the completed row. Subsecond monotonic remainder is compatible with a canonical completed start of `switch UTC - whole elapsed seconds`; it is not required to equal the original wall start exactly.

Normal legacy finish uses a second certified receipt protocol. It publishes prior-generation evidence before active mutation, then converges completed history, a cleared active draft, unchanged category metadata, canonical sediment, and every affected daily contribution before deleting the checkpoint. Startup consumes the finish receipt without resuming the finished generation. Four persisted kill points and later-publication failure custody are certified.

Legacy recovery flush/reload validate both active and archived catalogs, retain archived sediment identity, and emergency recovery schema 3 exports explicit archival state plus the structured recovery statement when present.

Clear-all/provisional-idle reset uses a third certified receipt boundary. It preserves all committed history, binds canonical prior elapsed and every affected day, restores exact active and grid state before legacy replay derives daily contributions, and applies active/sand/daily/checkpoint effects atomically in SQLite. Six transaction kill points and deterministic legacy replay are certified.

Initial SQLite TUI startup uses a typed atomic bootstrap request. The active row and first pending checkpoint share one stable identity column and commit together only after runtime state is staged. Four transaction fault boundaries, pre-existing checkpoint refusal, and real process failure/retry are certified.

Immediate and queued switches, clear operations, and normal finish settle sediment to the same UTC boundary used by chronological reconciliation before changing active state. Exact-boundary mass belongs to the outgoing category; bounded compressed settlement preserves category order and topology, and fresh `0×0` live canvases retain due mass without weakening persisted-checkpoint validation.

Successful checkpoint recovery produces one structured evidence statement. It exposes durable simulation time, capture time, persisted target, reconstructed duration, active identity, and exact/reconstructed/provisional classification; blocks ordinary input until acknowledgment; and is serialized unchanged in emergency export schema 3. Retry reuses the persisted cutoff.

The complete recovery contract and issue #10 closure are recorded in `docs/RECOVERY_AUTHORITY.md`.

### Profile authority

One selected profile owns the complete process authority.

- `--profile <directory>` is the explicit CLI/TUI selector; `STRATA_PROFILE` is the environment equivalent.
- A rooted profile owns `data/`, `state/`, and `config/` beneath one canonical root and persists a schema-1 UUID manifest at `profile.json`.
- Without an explicit root, one XDG profile owns the corresponding platform data, state, and configuration directories.
- Historical `STRATA_DATA_DIR` is accepted only as a legacy whole-profile-root alias; it no longer redirects data independently from runtime state.
- The selected profile is initialized before configuration or storage authority resolution and cannot change during the process.
- `time_log_path` configuration and its live atlas editor are removed; obsolete configuration fails with explicit `--profile` migration guidance.
- Legacy active-session files, detached checkpoints, recovery statements, and SQLite authority markers carry or validate the selected profile UUID.
- Rooted profiles reject missing or mismatched artifact identity; copied active/checkpoint evidence from another profile fails closed.
- Profile switching is deliberate close/open behavior: exit Strata and invoke it again with the target profile. There is no hot write redirection or in-memory dataset transfer.
- `strata --profile <directory> profile [--json]` exposes the selected UUID and all owned paths.

The complete contract is `docs/PROFILE_AUTHORITY.md`.

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
- SQLite schema version 6 introduced typed daily-contribution storage; current schema version 7 retains it while adding category lifecycle receipts, and distinct legacy-file paths preserve old cumulative daily evidence without reinterpretation.

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

Owns exact elapsed intervals, timestamps, categories, projects, committed session descriptions, operational-day policy, and reportable totals.

### Profile

Owns the stable profile UUID and the complete data/state/config path set for one process. A path override, copied artifact, or live configuration edit cannot change profile authority. Missing or mismatched identity under an explicit rooted profile fails closed before state is applied.

### Category catalog and lifecycle

Owns stable category identity, active/archived state, historical display metadata, reference validation, and retired-ID custody. Retirement may hide an identity from new selection but may not erase, relabel, or redirect existing sessions, sediment, snapshots, or tags. A reviewed lifecycle receipt may redirect all source-owned references atomically in SQLite or publish one exact-result replay package under legacy authority, or may certify zero-reference deletion; no partial, stale, or unconfirmed transformation is authority.

### Active generation and draft

Owns the current stable active-session identity, one session-owned description draft, and its transition receipts. Category metadata may describe the category but cannot supply, clear, or replace the active draft implicitly. Checkpoint evidence may describe that generation and draft but cannot replace authoritative active identity or survive a completed transition under a stale stable ID.

### Legacy category lifecycle receipt

Owns replay of one reviewed exact-result category merge or zero-reference deletion. It may republish only the receipt-bound catalog, sessions, tags, sediment, daily artifacts, checkpoint payload, and permanent ledger; exact matches are accepted, conflicts fail closed, and source identity remains retired after convergence.

### Legacy transition receipt

Owns replay of one prepared multi-file transition. It may exact-match or publish the recorded completed row and metadata effects, but it cannot reinterpret elapsed duration, invent a missing category, accept conflicting history, or retire itself before every named authority converges.

### Runtime recovery

Owns checkpoint evidence, bounded elapsed contribution since durable simulation time, one persisted target, and the resulting recovery statement. It may add mass and advance accumulator remainders, but may not replay unbounded physics, relax topology, discard protected evidence, apply payload state to a different active generation, move a persisted cutoff forward on retry, or label post-target live time as recovered history.

### Sediment formation

Owns accountable visual history and canonical topology. It must conserve mass and category identity while remaining independent of the current viewport.

### Historical snapshots

Own semantic identity and provenance for persisted or derived visual artifacts. A derived preview is a read-only projection; a daily contribution becomes authority only through explicit typed persistence.

### Interaction

Input routing owns navigation, commands, draft text, commit, cancel, contextual aliases, and mandatory emergency control. Draft state is not canonical history until one successful commit. Configured action state and visible atlas/palette claims must resolve through the same keymap authority.

### Terminal lifecycle

The terminal guard owns host-terminal acquisition and restoration. The application loop owns domain finalization and checkpoint attempts. Cleanup context may annotate an application or runtime error but may not replace its primary cause.

### Interface

TUI and CLI translate user intent and present state. Neither may own an independent ledger, reinterpret authority, mutate canonical sediment to fit the terminal, advance historical artifacts while viewing them, mutate history through ambiguous focus, invent fallback input routes, mislabel unreachable commands, silently convert unresolved categories to idle, apply stale checkpoint identity, clear an unresolved transition receipt, or leave the host terminal in application mode after control exits Strata.

## Current architectural frontier

Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, crash-recovery authority, category lifecycle, active-draft ownership, and complete profile isolation are implemented and certified. The post-SQLite GitHub issue reconciliation queue is empty.

Future work must begin as a new explicitly justified unit rather than being inferred from superseded issue premises. Known non-blocking design questions remain listed in `docs/DECISIONS.md`.

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
- An archived category is not deleted history.
- A source category removed by a certified lifecycle transaction is not an identity available for reuse.
- A lifecycle preview is not mutation authority after its revision becomes stale.
- Target selection or an approximate confirmation phrase is not authority for destructive lifecycle mutation.
- A prepared legacy lifecycle receipt is not disposable until every exact resulting artifact and the permanent ledger converge.
- Durable category metadata is not an active-session draft or reusable phrase template.
- A data-file path override is not a profile and cannot redirect live authority.
- An artifact from another profile is not recovery evidence for the selected profile.
- An unresolved category reference is not idle.
- A receipt for one transition kind is not authority for another transition kind.
- CSV, JSON, and ICS are external adapters, not canonical domain models.
