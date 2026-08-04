from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


# Accept the working contract and record exact implemented evidence.
replace_once(
    "notebook/work/RECONCILIATION-001C1.md",
    "state: active\nauthority: working",
    "state: accepted\nauthority: accepted",
)
path = Path("notebook/work/RECONCILIATION-001C1.md")
content = path.read_text()
content += '''

## Implemented result

- SQLite schema version 7 adds strict `category_lifecycle_receipts` authority;
- one typed preview resolves source and optional target by stable ID, inventories completed and active sessions, tags, canonical sediment, persisted snapshots, daily contributions, and runtime-checkpoint references, and binds them with a deterministic revision;
- one immediate transaction rejects stale previews, protected or malformed recovery evidence, source idle, self-merge, and unresolved transition receipts before publication;
- merge reassigns only category identity while preserving completed-session ID/stable ID, project, description, UTC chronology, elapsed duration, active stable identity, active start, and target metadata;
- target-first ordered tags deduplicate deterministically;
- placed, legacy-pending, and compressed-pending sediment preserve total mass and FIFO category order;
- cumulative/manual snapshots remap identity and daily contributions are regenerated from reassigned canonical ledger slices;
- receipt-free checkpoint payloads remap active identity, sediment, and queued switch mutations; receipt-bearing evidence fails closed;
- targetless permanent deletion is permitted only after a complete zero-reference preview;
- every merge or deletion records source/target metadata, preview revision, reference counts, and application time;
- repeated application of the same reviewed operation returns the existing receipt without duplicate mutation;
- retired source IDs remain permanently unavailable to category allocation;
- raw backup/restore and portable bundle schema 3 preserve lifecycle receipts and retired-ID custody;
- `sqlite doctor` rejects malformed receipt metadata/counts/timestamps and catalog reuse of retired identities.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 238 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 14 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- ten lifecycle publication fault boundaries: complete rollback;
- stale-preview, protected-checkpoint, receipt-custody, daily-revision, idempotent-retry, bundle round-trip, retired-ID nonreuse, and doctor-tamper proofs: pass;
- temporary transformation, audit, proof, and workflow machinery: absent from the permanent tree.

RECONCILIATION-001C1 completes the SQLite authority half of issue #13. The issue remains open for C2: a prepared legacy-file lifecycle receipt with idempotent replay and an explicit user-visible review/confirmation surface shared across supported authorities.
'''
path.write_text(content)

# Category authority.
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    '''Status: implemented and certified
Current completed unit: RECONCILIATION-001A
Issue completed: #5
Issue narrowed: #13
Last reviewed: 2026-08-02
''',
    '''Status: partially implemented and certified
Current completed unit: RECONCILIATION-001C1
Issue completed: #5
Issue narrowed: #13
Last reviewed: 2026-08-03
''',
)
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    '''## Legacy-file authority
''',
    '''## SQLite lifecycle transformation

Archive remains the ordinary retirement operation. Merge/reassignment and permanent deletion are distinct reviewed lifecycle operations.

Before either operation, SQLite builds one typed preview that:

- names an explicit source stable ID and optional explicit target stable ID;
- rejects idle, self-merge, and missing identities;
- resolves active and archived rows without name ambiguity;
- inventories completed sessions, active state, tags, placed and pending canonical sediment, every persisted snapshot, daily contributions, and runtime-checkpoint payload references;
- exposes checkpoint custody status and source/target metadata snapshots;
- binds all mutation-relevant authority state with a deterministic revision.

Application recomputes that revision inside one immediate transaction. A stale preview, protected `recovering`/`quarantined` checkpoint, malformed payload, or unresolved transition/finish/clear receipt blocks the operation before any authority changes.

A merge changes category identity only:

- completed session ID, stable ID, project, description, UTC chronology, operational-day policy, and elapsed duration remain unchanged;
- active stable ID, start, description, and recovery kind remain unchanged;
- target name, description, color, balance effect, archival state, and sort identity remain target-owned;
- target tags precede source-only tags and exact duplicates collapse;
- placed and pending sediment preserve mass and FIFO order;
- cumulative/manual snapshots remap category identity;
- daily contributions are regenerated from reassigned canonical session slices and receive matching source revisions;
- receipt-free checkpoints remap active, sediment, and queued-switch identity;
- the source row is removed only after a complete zero-residual-reference check.

Permanent deletion without a target is allowed only when the same complete preview reports zero references in every family. Idle cannot be deleted.

Each committed operation writes an immutable lifecycle receipt with source and target metadata, preview revision, affected counts, and application timestamp. Retry returns the same receipt idempotently. Receipt source IDs are retired forever and category allocation advances beyond all current and retired identities.

SQLite schema version 7 owns lifecycle receipts. Consistent repository snapshots, raw backup/restore, portable bundle schema 3, import validation, and `sqlite doctor` preserve and validate those receipts. A bundle or database that reintroduces a retired source ID fails integrity validation.

## Legacy-file authority
''',
)
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    '''## Certified proofs

- legacy catalog backward compatibility;
''',
    '''## Certified proofs

- complete SQLite reference preview and deterministic stale-preview rejection;
- atomic merge across completed/active sessions, tags, canonical sediment, snapshots, daily contributions, checkpoint payload, source removal, and receipt;
- ten injected publication boundaries with full rollback;
- completed-session and active-generation identity/chronology preservation;
- target metadata preservation and deterministic tag deduplication;
- sediment mass/FIFO preservation and daily-revision regeneration;
- protected or receipt-bearing checkpoint refusal;
- zero-reference-only permanent deletion and idle refusal;
- idempotent lifecycle retry;
- retired-ID nonreuse before and after portable bundle round trip;
- lifecycle receipt validation and doctor detection of tamper or retired-ID collision;
- legacy catalog backward compatibility;
''',
)
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    '''## Unresolved boundary

Category merge/reassignment and permanent destructive deletion are not implemented. Any future permanent deletion must require zero references or one reviewed transaction that reassigns every session, snapshot, sediment contribution, tag, and other category-owned record before removing the identity.
''',
    '''## Remaining issue #13 boundary

SQLite lifecycle authority is implemented and certified. Issue #13 remains open because legacy-file authority still needs a prepared receipt and idempotent crash replay across catalog, sessions, tags, canonical sediment, daily artifacts, detached checkpoint evidence, and retired-ID custody.

The product also needs one explicit review and confirmation surface that presents the complete preview and refuses stale confirmation under both supported authorities. Until C2 is complete, archive remains the only ordinary TUI retirement operation and no legacy merge or permanent deletion may claim success.
''',
)

# Architecture authority.
replace_once(
    "docs/ARCHITECTURE.md",
    '''- `src/domain.rs` — canonical sessions, project/category identity, operational-day allocation, reports, and cloneable staged legacy transition state.
''',
    '''- `src/domain.rs` — canonical sessions, project/category identity, operational-day allocation, reports, and cloneable staged legacy transition state.
- `src/category_lifecycle.rs` — storage-neutral category identity counting and remapping for sediment, snapshots, and receipt-free runtime checkpoint payloads.
''',
)
replace_once(
    "docs/ARCHITECTURE.md",
    '''- `src/sqlite.rs` and `src/sqlite/**` — schema migrations, category archival, repositories, active/checkpoint transition transactions, checkpoint custody, deterministic interchange, backup/restore, and fault certification.
''',
    '''- `src/sqlite.rs` and `src/sqlite/**` — schema migrations, category archival and lifecycle transactions, repositories, active/checkpoint transition transactions, checkpoint custody, deterministic interchange, backup/restore, and fault certification.
''',
)
replace_once(
    "docs/ARCHITECTURE.md",
    '''- Legacy-to-SQLite migration retains archived state and original session foreign keys.

The detailed category contract is `docs/CATEGORY_AUTHORITY.md`.
''',
    '''- Legacy-to-SQLite migration retains archived state and original session foreign keys.
- SQLite schema version 7 owns reviewed category lifecycle receipts.
- Merge/reassignment requires an explicit source and target plus a deterministic complete-reference preview; the preview is recomputed inside one immediate transaction and stale confirmation fails closed.
- The transaction reassigns category identity across completed and active sessions, tags, canonical sediment, snapshots, regenerated daily contributions, and receipt-free checkpoint payloads while preserving non-category identity, chronology, target metadata, mass, and FIFO order.
- Permanent deletion without reassignment requires zero references across the same complete inventory.
- Lifecycle receipts retire source IDs permanently; creation, backup/restore, portable bundle schema 3, import validation, and doctor integrity preserve that custody.
- Protected, malformed, or transition-receipt-bearing checkpoint evidence blocks lifecycle mutation.

The detailed category contract is `docs/CATEGORY_AUTHORITY.md`.
''',
)
replace_once(
    "docs/ARCHITECTURE.md",
    '''- SQLite schema version 6 and distinct legacy-file paths preserve old cumulative daily evidence without reinterpretation.
''',
    '''- SQLite schema version 6 introduced typed daily-contribution storage; current schema version 7 retains it while adding category lifecycle receipts, and distinct legacy-file paths preserve old cumulative daily evidence without reinterpretation.
''',
)
replace_once(
    "docs/ARCHITECTURE.md",
    '''### Category catalog

Owns stable category identity, active/archived state, historical display metadata, and reference validation. Retirement may hide an identity from new selection but may not erase, relabel, or redirect existing sessions, sediment, snapshots, or tags.
''',
    '''### Category catalog and lifecycle

Owns stable category identity, active/archived state, historical display metadata, reference validation, and retired-ID custody. Retirement may hide an identity from new selection but may not erase, relabel, or redirect existing sessions, sediment, snapshots, or tags. A reviewed SQLite lifecycle receipt may redirect all source-owned references atomically or certify zero-reference deletion; no partial or stale transformation is authority.
''',
)
replace_once(
    "docs/ARCHITECTURE.md",
    '''Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, and crash-recovery authority are complete. The next priorities are:

1. design the explicit merge/reassignment and permanent-deletion remainder of issue #13;
2. later domain/UI distinction work under issue #22;
3. later profile authority, including complete isolation and deliberate switching under issue #15.
''',
    '''Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, crash-recovery authority, and the SQLite category-lifecycle transaction are complete. The next priorities are:

1. complete issue #13 through a prepared legacy lifecycle receipt, idempotent crash replay, and explicit TUI review/confirmation under RECONCILIATION-001C2;
2. later domain/UI distinction work under issue #22;
3. later profile authority, including complete isolation and deliberate switching under issue #15.
''',
)
replace_once(
    "docs/ARCHITECTURE.md",
    '''- An archived category is not deleted history.
''',
    '''- An archived category is not deleted history.
- A source category removed by a certified lifecycle transaction is not an identity available for reuse.
- A lifecycle preview is not mutation authority after its revision becomes stale.
''',
)

# Decision index.
replace_once(
    "docs/DECISIONS.md",
    '''| STRATA-D051 | Checkpoint recovery owns one persisted target reused across retry. Successful recovery must visibly distinguish durable evidence, reconstructed time through that cutoff, and post-target provisional live time; emergency export projects the same structured statement. | implemented and certified |
''',
    '''| STRATA-D051 | Checkpoint recovery owns one persisted target reused across retry. Successful recovery must visibly distinguish durable evidence, reconstructed time through that cutoff, and post-target provisional live time; emergency export projects the same structured statement. | implemented and certified |
| STRATA-D052 | A SQLite category merge or permanent deletion requires one complete revision-bound preview and one immediate transaction. Merge reassigns every supported category-owned authority while preserving non-category identity, chronology, target metadata, sediment mass, and FIFO order; targetless deletion requires zero references. Every committed source identity is retired permanently through an auditable receipt preserved by backup, interchange, import validation, and doctor integrity. | implemented and certified |
''',
)
replace_once(
    "docs/DECISIONS.md",
    '''- category merge/reassignment and permanent destructive deletion under issue #13;
''',
    '''- the legacy-file receipt/replay and explicit TUI confirmation half of category merge/reassignment and permanent deletion under issue #13;
''',
)

# Notebook frontier.
replace_once(
    "notebook/NOW.md",
    '''summary: Crash-recovery authority is complete: identity, receipts, clear-all, initial bootstrap, exact transition edges, persisted cutoff reuse, visible uncertainty, and export parity are certified.
next: Define the category merge/reassignment and permanent-deletion transaction required to complete issue #13.
''',
    '''summary: SQLite category lifecycle authority is certified: complete preview, stale guard, atomic merge or zero-reference deletion, receipts, retired-ID custody, bundle parity, and doctor integrity.
next: Complete issue #13 through the legacy-file lifecycle receipt/replay protocol and explicit TUI review/confirmation surface.
''',
)
replace_once(
    "notebook/NOW.md",
    '''- emergency recovery schema 3 parity with the visible structured statement.
''',
    '''- emergency recovery schema 3 parity with the visible structured statement;
- SQLite schema 7 category lifecycle receipts and complete reference previews;
- revision-bound atomic category merge/reassignment across ledger, active state, tags, sediment, snapshots, daily contributions, and receipt-free checkpoints;
- zero-reference-only permanent deletion, idempotent retry, and permanent retired-ID custody;
- portable bundle schema 3 lifecycle receipt parity and doctor detection of tamper or retired-ID reuse.
''',
)
replace_once(
    "notebook/NOW.md",
    '''- SQLite schema version 6 is authoritative after explicit activation.
''',
    '''- SQLite schema version 7 is authoritative after explicit activation.
''',
)
replace_once(
    "notebook/NOW.md",
    '''- **RECONCILIATION-001B3C** — completed issue #10: persisted deterministic cutoff, visible exact/reconstructed/provisional evidence, acknowledgment custody, repeated-retry proof, and schema-3 export parity.
''',
    '''- **RECONCILIATION-001B3C** — completed issue #10: persisted deterministic cutoff, visible exact/reconstructed/provisional evidence, acknowledgment custody, repeated-retry proof, and schema-3 export parity.
- **RECONCILIATION-001C1** — partial issue #13: complete SQLite lifecycle preview, stale guard, atomic merge or zero-reference deletion, auditable receipts, retired-ID nonreuse, portable bundle schema 3, and doctor integrity.
''',
)
replace_once(
    "notebook/NOW.md",
    '''1. Define the merge/reassignment and permanent-deletion transaction needed to complete issue #13.
2. Later domain/UI distinction work under issue #22.
3. Later profile authority, including complete isolation and deliberate switching under issue #15.
''',
    '''1. Complete RECONCILIATION-001C2: legacy-file prepared lifecycle receipt, idempotent replay, and explicit TUI review/confirmation for issue #13.
2. Later domain/UI distinction work under issue #22.
3. Later profile authority, including complete isolation and deliberate switching under issue #15.
''',
)
replace_once(
    "notebook/NOW.md",
    '''- Issue #13 still lacks explicit category merge/reassignment and permanent-deletion transactions.
''',
    '''- Issue #13 still lacks legacy-file crash-safe lifecycle replay and the final explicit user review/confirmation surface.
''',
)
replace_once(
    "notebook/NOW.md",
    '''Define the category merge/reassignment and permanent-deletion transaction required to complete issue #13. Preserve historical meaning, refuse ambiguous destructive operations, and certify both SQLite and legacy authority before closure.
''',
    '''Implement RECONCILIATION-001C2. Preserve the C1 complete-reference and stale-preview contract while adding a prepared legacy receipt across every file authority, idempotent startup replay, retired-ID custody, and one explicit review/confirmation interaction before issue #13 closure.
''',
)

# Reconciliation queue.
replace_once(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    '''| #13 | Historical data-loss defect completed by RECONCILIATION-001A: active/archived metadata, reports, sand, tags, restore, and migration retain stable meaning under SQLite and legacy authority. Remaining scope is explicit merge/reassignment plus permanent-deletion policy and tests. | DOMAIN-002 or dedicated category-merge unit |
''',
    '''| #13 | Partially completed by RECONCILIATION-001A and RECONCILIATION-001C1. Historical meaning survives archive/restore, and SQLite now has a complete revision-bound preview, atomic merge or zero-reference deletion, auditable receipts, retired-ID custody, portable bundle parity, and doctor integrity. Remaining scope is the legacy-file prepared receipt/replay protocol and explicit TUI review/confirmation. | RECONCILIATION-001C2 |
''',
)
replace_once(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    '''Issue #10 is evidence-backed and complete. Next:

1. define category merge/reassignment semantics for issue #13;
2. define permanent deletion preconditions and fail-closed refusal paths;
3. certify historical session, sediment, snapshot, tag, and migration meaning under SQLite and legacy authority.

Do not treat archive as deletion or invent reassignment for unresolved references.
''',
    '''Issue #10 is evidence-backed and complete. RECONCILIATION-001C1 has certified the SQLite lifecycle half of issue #13. Next:

1. design a prepared legacy-file lifecycle receipt that binds the C1 source/target metadata, complete counts, deterministic revision, transformed payloads, affected days, and retired-ID result;
2. publish and replay catalog, session, tag, canonical-sand, daily-artifact, detached-checkpoint, and lifecycle-custody effects idempotently from every durable kill point;
3. expose one explicit TUI preview and confirmation flow that recomputes the revision before mutation and keeps archive as the ordinary retirement path;
4. close issue #13 only after both authorities and the visible interaction are evidence-backed.

Do not treat archive as deletion, reuse a retired identity, or invent reassignment for unresolved references.
''',
)
