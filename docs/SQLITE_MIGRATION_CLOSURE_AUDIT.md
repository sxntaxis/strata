# SQLite Migration Closure Audit

- Audit unit: `SQLITE-011`
- Baseline: `a2b7d1b64779c7db86cd9ffd12a92c5bd459df91`
- Tracking issue: #8
- Audit date: 2026-08-01
- Verdict: **NOT READY TO CLOSE**

## Executive conclusion

The SQLite authority migration is operationally complete after explicit activation: CLI and TUI share one SQLite authority, runtime transitions are transactional and retry-safe, repository failures fail visibly, portable CSV bundles round-trip through a consistent snapshot, and database maintenance operations are tested.

Issue #8 should nevertheless remain open because two requirements are not yet fully satisfied:

1. Portable CSV import has no first-class dry-run/validation-only mode, and the repository does not document the migration and interchange workflow.
2. Legacy CSV/JSON migration evidence is preserved, but Strata has no supported command that inventories and archives or removes it under explicit user control.

These are bounded completion gaps. They do not justify reopening the repository, authority, runtime-coordination, or persistence-failure designs.

## Acceptance-criteria reconciliation

| # | Issue #8 acceptance criterion | Verdict | Merged evidence | Residual |
|---|---|---|---|---|
| 1 | SQLite is the sole live source of truth after migration. | PASS | SQLITE-006 activates a verified candidate explicitly; SQLITE-007 routes CLI and TUI through SQLite and proves no legacy dual writes. | Activation remains deliberately explicit rather than automatic. |
| 2 | Existing CSV/JSON data imports without losing identity, descriptions, timestamps, or duration totals. | PASS | SQLITE-002 performs strict all-source validation and transactional import; SQLITE-004 verifies domain-visible parity. | None. |
| 3 | CLI and TUI use the same repository without stale-snapshot overwrites. | PASS | SQLITE-007 moves the TUI to the SQLite repository and makes autosave update-only; SQLITE-008 adds stable-ID fencing and durable operation receipts. | None. |
| 4 | Active-session and detach/recovery transitions are transactional and crash-safe. | PASS | SQLITE-008 makes start/finish/switch/reset idempotent and retains checkpoints until atomic recovery commit; SQLITE-010 certifies rollback/recoverability across all transition families. | None. |
| 5 | Historical sessions cannot disappear through category deletion or missing references. | PASS | Foreign keys and archival begin in SQLITE-001; repository and TUI archive/restore behavior is covered in SQLITE-004 and SQLITE-007. | None. |
| 6 | Database/schema errors are visible and never trigger writable empty fallback state. | PASS | SQLITE-006/007 fail closed on authority disagreement and repository load failure; SQLITE-009 adds the non-dismissible recovery surface; SQLITE-010 covers real corruption/read-only/full/busy failures. | None. |
| 7 | Deterministic CSV export and validated CSV import are documented and tested. | PARTIAL — BLOCKING | SQLITE-005 implements and tests a versioned seven-file CSV bundle with deterministic ordering, fingerprints, strict validation, and snapshot parity. | No repository documentation existed at audit start, and `sqlite-import` has no `--dry-run` validation-only mode. The generic `export --format` surface still exposes only JSON and ICS; the CSV bundle is a separate `sqlite-export` command. |
| 8 | Backup, restore, integrity, migration rollback, and multi-process tests exist. | PASS | SQLITE-003 covers migration rollback/publication; SQLITE-005 covers doctor/backup/restore and maintenance locking; SQLITE-008/010 cover concurrent transitions, busy locking, commit failure, and recovery. | None. |
| 9 | Legacy files remain available until the user explicitly archives or removes them. | PARTIAL — BLOCKING | Migration and activation preserve legacy files byte-for-byte and stop writing them after activation. | Strata has no supported inventory/archive/remove command, custody manifest, or explicit acknowledgement flow. Manual filesystem deletion is not a sufficient product contract. |

Summary: **7 PASS, 2 PARTIAL/BLOCKING**.

## Normative requirement audit beyond the acceptance checklist

### Satisfied

- Foreign keys are enabled on repository connections.
- WAL mode and a bounded busy timeout are deliberate connection policy.
- Schema migrations are versioned and tested.
- Legacy sources are parsed completely before mutation.
- Migration uses an immutable fingerprinted backup and publishes a verified candidate atomically.
- Source identity, row counts, ID sets, elapsed totals, per-category totals, active state, checkpoints, category tags, sediment state, and snapshots are reconciled.
- Consistent repository read transactions back portable exports.
- Import refuses existing targets and validates fingerprints, sizes, schemas, references, and snapshot parity.
- Doctor, backup, and restore are explicit user operations.
- Runtime authority never falls back automatically from a damaged SQLite database to stale legacy files.
- Persistence failure handling preserves active authority and offers retry, authoritative reload, emergency export, safe export-and-exit, or explicit exit without saving.

### Incomplete

#### R1 — Validation-only portable import

Issue #8 asks for CSV import with dry-run validation and an actionable error report. `strata sqlite-import` currently validates while constructing a new database, but it has no validation-only mode.

Required closure behavior:

- `strata sqlite-import --bundle <dir> --dry-run` performs all manifest, CSV, identity, reference, schema, fingerprint, and repository-snapshot checks;
- it creates no target database, lock, temporary publication artifact, or authority marker;
- human and `--json` reports identify the failing file, row/field where available, invariant, and suggested correction;
- dry-run and actual import share one validation pipeline so they cannot drift.

#### R2 — Explicit legacy-evidence disposition

Legacy sources are correctly preserved after activation, but there is no supported end state for users who want to archive or remove them.

Required closure behavior:

- inventory every migration source and compare it with the fingerprinted migration manifest;
- refuse operation unless SQLite is active, healthy, and matches the verified candidate provenance;
- default to archive, not deletion;
- create a deterministic archive directory or package containing source bytes, manifest, candidate/report references, and archive timestamp;
- require explicit confirmation;
- make retries idempotent and fail closed on partial publication;
- offer removal only as a separate, more explicit mode after verified archive publication;
- never touch unrelated custom files merely because they share a directory.

#### R3 — Durable user documentation

This audit updates the README, but final closure documentation must remain synchronized with the actual command surface and include:

- authority states and one-way activation semantics;
- migration dry-run, migration execution, and activation sequence;
- portable CSV bundle structure and schema version;
- doctor, backup, and restore workflows;
- legacy evidence retention and disposition;
- recovery behavior for busy, read-only, full, corrupt, and unsupported-schema failures.

R3 is resolved for the current command surface by SQLITE-011's README update, but documentation must be amended again when R1 and R2 land.

## Scope decisions

### Emergency recovery JSON

SQLITE-009 emergency JSON is a custody artifact generated from in-memory application state when authority writes fail. A supported import/reconciliation workflow would be valuable, but it is not the deterministic SQLite CSV interchange requirement defined by issue #8.

It should be tracked as a separate recovery feature rather than expanding the migration-closure critical path.

### Automatic first-run migration

The implemented model requires explicit `migrate-sqlite` and `activate-sqlite --confirm`. This is intentionally safer than mutating authority during normal startup and still satisfies the acceptance criterion that SQLite becomes sole authority *after migration*.

Automatic startup migration is therefore not a closure requirement.

### Legacy runtime code

Legacy readers/writers remain necessary before explicit activation and for strict source migration. Their continued presence is not dual authority: after activation, CLI and TUI use SQLite and do not write legacy runtime files.

Removing all legacy parsing code is not a closure requirement.

## Required next unit

`SQLITE-012 — close interchange validation and legacy evidence disposition`

Bounded scope:

1. add validation-only `sqlite-import --dry-run` using the existing strict import pipeline;
2. add an explicit legacy evidence inventory/archive command with verified provenance and idempotent publication;
3. add optional separately confirmed source removal only after a verified archive exists;
4. document the final command workflows and portable bundle contract;
5. certify process-level no-side-effect dry-run, archive retry, provenance mismatch refusal, partial-publication recovery, and no-dual-write behavior;
6. rerun the complete SQLite authority, maintenance, runtime coordination, and persistence-fault suites;
7. close issue #8 only if the final audit reaches 9/9 PASS.

## Non-goals for SQLITE-012

- no automatic startup migration or activation;
- no emergency JSON reconciliation;
- no repository or schema redesign unless required for archive provenance;
- no removal of legacy parsers needed for migration;
- no fallback from SQLite authority;
- no unrelated TUI changes.
