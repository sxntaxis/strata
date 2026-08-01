# SQLite Migration Closure Audit

- Final audit unit: `SQLITE-012`
- Baseline before closure: `fb89964fb0404217d5c52dbe664e8cf23633cbe1`
- Tracking issue: #8
- Audit date: 2026-08-01
- Verdict: **READY TO CLOSE — 9/9 PASS**

## Executive conclusion

The authoritative persistence migration is complete.

After explicit migration and activation, CLI and TUI share one SQLite authority. Runtime transitions and detached recovery are transactional, fenced, retry-safe, and crash-recoverable. Persistence failures remain visible and actionable. Deterministic CSV bundles support validation-only import and lossless publication. Database doctor, backup, restore, migration rollback, multi-process coordination, and exhaustive persistence-fault tests are present. Legacy sources remain available until an explicit, provenance-verified archive and separately confirmed removal operation.

No acceptance criterion requires automatic startup migration, removal of migration parsers, or import of SQLITE-009 emergency JSON custody bundles.

## Acceptance-criteria reconciliation

| # | Issue #8 acceptance criterion | Verdict | Final evidence |
|---|---|---|---|
| 1 | SQLite is the sole live source of truth after migration. | PASS | SQLITE-006 activates a verified candidate explicitly. SQLITE-007 routes CLI and TUI through SQLite and proves no legacy dual writes. |
| 2 | Existing CSV/JSON data imports without losing IDs, category/project identity, descriptions, timestamps, or duration totals. | PASS | SQLITE-002 performs strict transactional import and complete reconciliation. SQLITE-004 proves repository/domain parity. |
| 3 | CLI and TUI use the same repository and avoid stale-snapshot overwrites. | PASS | SQLITE-007 completes the TUI cutover and update-only persistence. SQLITE-008 adds stable-ID fencing and durable operation receipts. |
| 4 | Active-session and detach/recovery transitions are transactional and crash-safe. | PASS | SQLITE-008 makes runtime transitions idempotent and retains checkpoints until atomic recovery commit. SQLITE-010 certifies every persistence family under failure. |
| 5 | Historical sessions cannot become invisible through category deletion or missing references. | PASS | Foreign keys, category archival, stable-ID restoration, and history-preserving repository/TUI behavior are tested from SQLITE-001 through SQLITE-007. |
| 6 | Database/schema errors are visible and never trigger writable empty fallback state. | PASS | Authority disagreement, unsupported schema, corruption, busy, read-only, full-disk, constraint, commit, and I/O failures fail visibly. SQLITE-009 supplies non-dismissible retry/export/safe-exit recovery. |
| 7 | First-class deterministic CSV export and validated CSV import are documented and tested. | PASS | SQLITE-005 provides deterministic versioned bundles and strict round-trip import. SQLITE-012 adds `sqlite-import --dry-run` through the same temporary SQLite import, integrity, and exact snapshot-reconciliation path without target publication. README documents both workflows. |
| 8 | Backup, restore, integrity-check, migration rollback, and multi-process tests exist. | PASS | SQLITE-003 covers migration rollback/publication. SQLITE-005 covers doctor, backup, restore, maintenance locking, and interrupted restore. SQLITE-008/010 cover concurrent transitions and real SQLite failure classes. |
| 9 | Legacy files remain available until the user explicitly archives or removes them. | PASS | Migration and activation preserve source bytes. SQLITE-012 adds verified inventory, archive-first custody, exact-fingerprint removal confirmation, and a retryable removal ledger. |

Summary: **9 PASS, 0 PARTIAL, 0 FAIL**.

## SQLITE-012 closure controls

### Validation-only portable import

```bash
strata sqlite-import --bundle <DIRECTORY> --dry-run
```

The dry-run path:

1. parses and fingerprints the complete bundle;
2. validates manifest sizes, schemas, ordering, identities, and references;
3. imports into a unique disposable SQLite database;
4. checkpoints and runs database health checks;
5. reads the complete repository snapshot back;
6. requires exact equality with the parsed bundle;
7. removes all disposable database artifacts;
8. publishes no target, target parent, maintenance lock, or authority marker.

Actual import calls the same candidate-validation function before atomic publication, preventing validation drift.

### Legacy evidence inventory

```bash
strata sqlite-legacy-inventory
```

Inventory fails closed unless all authority and provenance layers agree:

- active authority marker is `sqlite-cli`;
- activation and candidate paths/fingerprints agree;
- SQLite integrity passes;
- database metadata identifies the verified migration;
- `legacy_imports.source_manifest_json` contains the same logical names, original paths, existence flags, byte counts, and content fingerprints as the immutable backup provenance;
- live originals are regular files whose bytes match the migration backup.

This cross-check prevents a modified `source_paths.json` from redirecting archive or removal to another path.

### Archive-first custody

```bash
strata sqlite-legacy-archive --out <DIRECTORY> --confirm
```

Archive publication:

- copies from the immutable migration backup, not mutable live files;
- includes source provenance and a custody manifest;
- verifies every byte before publication;
- uses a fingerprint-owned staging directory and atomic rename;
- is idempotent when the target already matches;
- finishes a complete interrupted publication;
- safely rebuilds an owned incomplete stage;
- refuses foreign or mismatched staging directories.

### Separately confirmed removal

```bash
strata sqlite-legacy-remove \
  --archive <DIRECTORY> \
  --confirm-fingerprint <MIGRATION_FINGERPRINT>
```

Removal requires a healthy active SQLite authority, a verified archive, the exact migration fingerprint, and unchanged live source bytes. A durable ledger is published before the first deletion. Missing files are accepted only while resuming that matching in-progress ledger. Completion is durable and idempotent.

Only exact paths recorded in SQLite's verified source manifest are eligible. Unrelated files sharing a source directory are never selected. The SQLite database, immutable migration backup, custody archive, authority marker, and removal ledger remain intact.

## Final executable evidence

The hosted SQLITE-012 gate passed:

- formatting: PASS;
- strict Clippy with all targets/features and warnings denied: PASS;
- unit tests: **119 passed**;
- legacy CLI lifecycle process tests: **7 passed**;
- SQLite authority/TUI process tests: **11 passed**;
- doc tests: PASS.

Focused SQLITE-012 proofs include:

- full validation-only import with no requested target or parent publication;
- ordinary import through the same validation path;
- legacy inventory against active authority and immutable provenance;
- archive idempotency;
- recovery from an owned partial archive stage;
- refusal of changed live evidence;
- refusal of incorrect removal confirmation;
- retry after an injected interruption between legacy-file deletions;
- rejection of tampered path provenance against SQLite's verified source manifest.

The complete prior persistence-fault matrix remains green: all 19 authoritative write/transition families plus real busy, read-only, constraint, full-disk, and corrupt-database scenarios.

## Authority and custody end state

Before explicit activation:

```text
CLI + TUI -> legacy CSV/JSON authority
SQLite    -> absent or verified candidate
```

After explicit activation:

```text
CLI + TUI -> SQLite authority
legacy CSV/JSON -> unchanged migration evidence, no dual writes
```

After optional verified removal:

```text
CLI + TUI -> SQLite authority
immutable migration backup -> retained
verified custody archive    -> retained
original legacy paths       -> removed under exact-fingerprint confirmation
removal ledger              -> retained
```

## Scope decisions retained

### Explicit migration rather than automatic startup mutation

`migrate-sqlite` and `activate-sqlite --confirm` remain deliberate commands. This is safer than automatic first-run authority mutation and satisfies the requirement that SQLite becomes sole authority after migration.

### Emergency recovery JSON

The SQLITE-009 emergency JSON file is a custody artifact generated from in-memory state during write failure. A future reconciliation/import feature may be useful, but it is not the deterministic CSV interchange requirement and is not a migration-closure blocker.

### Legacy compatibility code

Legacy readers and strict import parsers remain necessary before activation and for migration evidence interpretation. Their presence does not create dual authority because activated CLI/TUI paths use SQLite exclusively.

## Closure decision

Issue #8 may close when SQLITE-012 merges and the exact final source/documentation tree passes ordinary CI. No residual migration-program implementation requirement remains.
