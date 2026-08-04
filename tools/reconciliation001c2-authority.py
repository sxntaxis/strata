from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


# Architecture authority.
replace_once(
    "docs/ARCHITECTURE.md",
    "- `src/legacy_transition.rs` — schema-versioned legacy transition receipts, completed-session payload validation, and exact/idempotent session reconciliation.\n",
    "- `src/legacy_transition.rs` — schema-versioned legacy transition receipts, completed-session payload validation, and exact/idempotent session reconciliation.\n- `src/legacy_category_lifecycle.rs` — complete legacy category-reference inventory, exact-result prepared receipts, permanent lifecycle ledger, deterministic replay, and retired-ID allocation.\n",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "- `src/app.rs` and `src/app/**` — TUI orchestration, active/archived category projections, semantic-edge checkpoint refresh, legacy switch/finish/clear-all receipt publication and replay, explicit modal/edit state, persistence reconciliation, bounded recovery, historical artifact selection, context selection, resolver execution, palette/atlas projection, and rendering.\n",
    "- `src/app.rs` and `src/app/**` — TUI orchestration, active/archived category projections, semantic-edge checkpoint refresh, legacy switch/finish/clear-all and category-lifecycle receipt publication/replay, blocking revision-bound lifecycle review, explicit modal/edit state, persistence reconciliation, bounded recovery, historical artifact selection, context selection, resolver execution, palette/atlas projection, and rendering.\n",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "- Protected, malformed, or transition-receipt-bearing checkpoint evidence blocks lifecycle mutation.\n\nThe detailed category contract",
    "- Protected, malformed, or transition-receipt-bearing checkpoint evidence blocks lifecycle mutation.\n- Legacy lifecycle preparation stages exact resulting catalog, sessions, tags, canonical sediment, affected daily contributions, receipt-free checkpoint payload, and permanent ledger before publishing one prepared receipt.\n- Startup replays that prepared receipt before ordinary authority load, accepts exact already-published artifacts, rejects conflict, and removes evidence only after every result and the permanent ledger converge.\n- The TUI keeps ordinary archive on `x`; a distinct configurable `Shift-X` action requires explicit target/deletion selection, displays the complete revision-bound preview, and applies only after the exact phrase is typed.\n- The permanent legacy ledger migrates into SQLite receipts and preserves the shared identity high-water mark.\n\nThe detailed category contract",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "Owns stable category identity, active/archived state, historical display metadata, reference validation, and retired-ID custody. Retirement may hide an identity from new selection but may not erase, relabel, or redirect existing sessions, sediment, snapshots, or tags. A reviewed SQLite lifecycle receipt may redirect all source-owned references atomically or certify zero-reference deletion; no partial or stale transformation is authority.\n",
    "Owns stable category identity, active/archived state, historical display metadata, reference validation, and retired-ID custody. Retirement may hide an identity from new selection but may not erase, relabel, or redirect existing sessions, sediment, snapshots, or tags. A reviewed lifecycle receipt may redirect all source-owned references atomically in SQLite or publish one exact-result replay package under legacy authority, or may certify zero-reference deletion; no partial, stale, or unconfirmed transformation is authority.\n",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "### Legacy transition receipt\n\nOwns replay of one prepared multi-file transition.",
    "### Legacy category lifecycle receipt\n\nOwns replay of one reviewed exact-result category merge or zero-reference deletion. It may republish only the receipt-bound catalog, sessions, tags, sediment, daily artifacts, checkpoint payload, and permanent ledger; exact matches are accepted, conflicts fail closed, and source identity remains retired after convergence.\n\n### Legacy transition receipt\n\nOwns replay of one prepared multi-file transition.",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, crash-recovery authority, and the SQLite category-lifecycle transaction are complete. The next priorities are:\n\n1. complete issue #13 through a prepared legacy lifecycle receipt, idempotent crash replay, and explicit TUI review/confirmation under RECONCILIATION-001C2;\n2. later domain/UI distinction work under issue #22;\n3. later profile authority, including complete isolation and deliberate switching under issue #15.\n",
    "Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, crash-recovery authority, and category lifecycle across SQLite, legacy files, migration, and TUI confirmation are complete. The next priorities are:\n\n1. resolve the active draft versus category metadata distinction under issue #22;\n2. later profile authority, including complete isolation and deliberate switching under issue #15.\n",
)
replace_once(
    "docs/ARCHITECTURE.md",
    "- A lifecycle preview is not mutation authority after its revision becomes stale.\n",
    "- A lifecycle preview is not mutation authority after its revision becomes stale.\n- Target selection or an approximate confirmation phrase is not authority for destructive lifecycle mutation.\n- A prepared legacy lifecycle receipt is not disposable until every exact resulting artifact and the permanent ledger converge.\n",
)

# Category authority.
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    "Status: partially implemented and certified\nCurrent completed unit: RECONCILIATION-001C1\nIssue completed: #5\nIssue narrowed: #13\nLast reviewed: 2026-08-03\n",
    "Status: implemented and certified\nCompleted units: RECONCILIATION-001A, RECONCILIATION-001C1, RECONCILIATION-001C2\nIssues completed: #5, #13\nLast reviewed: 2026-08-03\n",
)
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    "Malformed catalog rows are not skipped and default values are not invented.\n\n## Session reference integrity\n",
    "Malformed catalog rows are not skipped and default values are not invented.\n\n## Legacy lifecycle transformation\n\nLegacy merge/reassignment and permanent deletion use the same explicit source/target semantics and complete reference model as SQLite, but publication is governed by an exact-result prepared receipt. Before any authority file changes, preparation validates source idle, self-merge, target existence, zero-reference deletion, stale revision, checkpoint shape, and absence of unresolved switch/finish/clear receipts.\n\nThe prepared receipt contains the reviewed metadata, counts, revision, operation identity, and exact resulting catalog, session ledger, tags, canonical sediment, affected daily contributions or explicit deletions, receipt-free detached checkpoint payload, and permanent lifecycle ledger. It is published atomically before any result. Startup replays it before ordinary legacy state load in deterministic order and accepts only exact already-published artifacts. Conflicting or malformed state fails closed and retains evidence. The prepared receipt is removed only after every named authority and the permanent ledger converge.\n\nThe permanent ledger records committed merge/deletion receipts and retired source identities. Legacy allocation advances beyond both catalog and ledger identities. Migration fingerprints and imports this ledger into SQLite schema-7 lifecycle receipts, preserving the identity high-water mark after activation.\n\n## Explicit lifecycle interaction\n\nOrdinary `x` remains archive. A distinct configurable `Shift-X` action opens a blocking lifecycle overlay, chooses an explicit target or targetless deletion, displays source/target identity, all affected reference counts, checkpoint custody, and revision, and requires the exact displayed revision-bound phrase. Esc cancels without mutation. Approximate, case-folded, whitespace-normalized, stale, or missing confirmation never applies. SQLite delegates to the C1 transaction; legacy authority publishes the prepared receipt and reloads through the same replay path.\n\n## Session reference integrity\n",
)
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    "- validation and database publication remain transactional and fail closed.\n",
    "- committed legacy lifecycle receipts import into schema-7 receipt authority;\n- retired source IDs and the identity high-water mark survive activation;\n- validation and database publication remain transactional and fail closed.\n",
)
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    "Category catalog writes use atomic file publication. A failed legacy write enters the existing visible persistence-recovery contract; it cannot produce a partially written catalog.\n",
    "Category catalog writes use atomic file publication. A failed ordinary legacy write enters the existing visible persistence-recovery contract; it cannot produce a partially written catalog. Once a lifecycle prepared receipt is durable, failure retains that evidence and retry/restart republishes its exact results rather than reconstructing intent from partial files.\n",
)
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    "- lifecycle receipt validation and doctor detection of tamper or retired-ID collision;\n",
    "- lifecycle receipt validation and doctor detection of tamper or retired-ID collision;\n- complete legacy reference inventory and deterministic revision;\n- exact-result prepared receipt across catalog, sessions, tags, sediment, daily artifacts, checkpoint, and permanent ledger;\n- eight persisted replay kill points with retained evidence and clean retry convergence;\n- legacy zero-reference-only deletion, idle/self-merge/stale/protected-evidence refusal, and target metadata preservation;\n- permanent ledger restart custody, retired-ID nonreuse, migration fingerprinting, and schema-7 import;\n- distinct archive and configurable lifecycle actions across resolver, atlas, palette, and runtime;\n- exact confirmation phrase unit proofs and a live PTY round trip that reads the rendered phrase, types it back, commits one receipt, and verifies reassignment;\n",
)
replace_once(
    "docs/CATEGORY_AUTHORITY.md",
    "## Remaining issue #13 boundary\n\nSQLite lifecycle authority is implemented and certified. Issue #13 remains open because legacy-file authority still needs a prepared receipt and idempotent crash replay across catalog, sessions, tags, canonical sediment, daily artifacts, detached checkpoint evidence, and retired-ID custody.\n\nThe product also needs one explicit review and confirmation surface that presents the complete preview and refuses stale confirmation under both supported authorities. Until C2 is complete, archive remains the only ordinary TUI retirement operation and no legacy merge or permanent deletion may claim success.",
    "## Issue #13 closure\n\nRECONCILIATION-001A preserves historical meaning through archive/restore and strict reference resolution. RECONCILIATION-001C1 provides the complete SQLite preview, transaction, receipt, and retired-ID custody. RECONCILIATION-001C2 provides legacy exact-result receipt/replay, permanent ledger migration, and the shared explicit TUI confirmation surface. Archive remains the ordinary retirement operation; reviewed merge/reassignment and zero-reference permanent deletion are now implemented and certified under every supported authority.",
)

# Decision authority.
replace_once(
    "docs/DECISIONS.md",
    "| STRATA-D052 | A SQLite category merge or permanent deletion requires one complete revision-bound preview and one immediate transaction. Merge reassigns every supported category-owned authority while preserving non-category identity, chronology, target metadata, sediment mass, and FIFO order; targetless deletion requires zero references. Every committed source identity is retired permanently through an auditable receipt preserved by backup, interchange, import validation, and doctor integrity. | implemented and certified |\n",
    "| STRATA-D052 | A SQLite category merge or permanent deletion requires one complete revision-bound preview and one immediate transaction. Merge reassigns every supported category-owned authority while preserving non-category identity, chronology, target metadata, sediment mass, and FIFO order; targetless deletion requires zero references. Every committed source identity is retired permanently through an auditable receipt preserved by backup, interchange, import validation, and doctor integrity. | implemented and certified |\n| STRATA-D053 | Legacy category lifecycle publishes one exact-result prepared receipt before any multi-file mutation, replays it idempotently before ordinary startup load, and retires it only after catalog, sessions, tags, sediment, daily artifacts, checkpoint, and permanent ledger converge. Ordinary archive remains `x`; merge or permanent deletion is a distinct configurable action requiring explicit target/deletion selection and the exact displayed revision-bound phrase. | implemented and certified |\n",
)
replace_once(
    "docs/DECISIONS.md",
    "- the legacy-file receipt/replay and explicit TUI confirmation half of category merge/reassignment and permanent deletion under issue #13;\n",
    "",
)

# Current Notebook status.
replace_once(
    "notebook/NOW.md",
    "summary: SQLite category lifecycle authority is certified: complete preview, stale guard, atomic merge or zero-reference deletion, receipts, retired-ID custody, bundle parity, and doctor integrity.\nnext: Complete issue #13 through the legacy-file lifecycle receipt/replay protocol and explicit TUI review/confirmation surface.\n",
    "summary: Category lifecycle is complete across SQLite, legacy-file replay, migration, and explicit TUI confirmation; issue #13 is evidence-backed for closure.\nnext: Resolve the active draft versus category metadata distinction under issue #22.\n",
)
replace_once(
    "notebook/NOW.md",
    "The SQLite migration, startup authority, temporal, domain, reporting, sediment, interaction, category-integrity, active/checkpoint generation-coherence, and legacy switch/finish/clear-all-replay units are complete.\n",
    "The SQLite migration, startup authority, temporal, domain, reporting, sediment, interaction, category-integrity, active/checkpoint generation-coherence, legacy transition replay, and complete category-lifecycle units are complete.\n",
)
replace_once(
    "notebook/NOW.md",
    "- portable bundle schema 3 lifecycle receipt parity and doctor detection of tamper or retired-ID reuse.\n",
    "- portable bundle schema 3 lifecycle receipt parity and doctor detection of tamper or retired-ID reuse;\n- exact-result legacy lifecycle preparation and startup replay across every file authority;\n- permanent legacy lifecycle ledger, restart-safe retired-ID allocation, and schema-7 migration import;\n- distinct archive and lifecycle action truth across keymap, atlas, palette, and runtime;\n- blocking target/deletion review with exact revision-bound typed confirmation;\n- live PTY proof of rendered-phrase capture, confirmation, one receipt, reassignment, and normal active-interval finish.\n",
)
replace_once(
    "notebook/NOW.md",
    "- **RECONCILIATION-001C1** — partial issue #13: complete SQLite lifecycle preview, stale guard, atomic merge or zero-reference deletion, auditable receipts, retired-ID nonreuse, portable bundle schema 3, and doctor integrity.\n",
    "- **RECONCILIATION-001C1** — partial issue #13: complete SQLite lifecycle preview, stale guard, atomic merge or zero-reference deletion, auditable receipts, retired-ID nonreuse, portable bundle schema 3, and doctor integrity.\n- **RECONCILIATION-001C2** — completed issue #13: exact-result legacy prepared receipt/replay, permanent lifecycle ledger and migration custody, distinct archive/lifecycle actions, revision-bound typed confirmation, and live PTY proof.\n",
)
replace_once(
    "notebook/NOW.md",
    "1. Complete RECONCILIATION-001C2: legacy-file prepared lifecycle receipt, idempotent replay, and explicit TUI review/confirmation for issue #13.\n2. Later domain/UI distinction work under issue #22.\n3. Later profile authority, including complete isolation and deliberate switching under issue #15.\n",
    "1. Resolve the active draft versus category metadata distinction under issue #22.\n2. Later profile authority, including complete isolation and deliberate switching under issue #15.\n",
)
replace_once(
    "notebook/NOW.md",
    "- Issue #13 still lacks legacy-file crash-safe lifecycle replay and the final explicit user review/confirmation surface.\n",
    "",
)
replace_once(
    "notebook/NOW.md",
    "Implement RECONCILIATION-001C2. Preserve the C1 complete-reference and stale-preview contract while adding a prepared legacy receipt across every file authority, idempotent startup replay, retired-ID custody, and one explicit review/confirmation interaction before issue #13 closure.\n",
    "Start the issue #22 domain/UI distinction unit. Preserve the completed category lifecycle and historical-meaning contracts while deciding whether active draft text and durable category metadata remain coupled or become explicit separate concepts.\n",
)

# Reconciliation queue.
replace_once(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "| #13 | Partially completed by RECONCILIATION-001A and RECONCILIATION-001C1. Historical meaning survives archive/restore, and SQLite now has a complete revision-bound preview, atomic merge or zero-reference deletion, auditable receipts, retired-ID custody, portable bundle parity, and doctor integrity. Remaining scope is the legacy-file prepared receipt/replay protocol and explicit TUI review/confirmation. | RECONCILIATION-001C2 |\n",
    "| #13 | Completed by RECONCILIATION-001A, RECONCILIATION-001C1, and RECONCILIATION-001C2: archive/restore preserves historical meaning; SQLite provides the complete transaction and receipt; legacy files provide exact-result prepared replay and permanent retired-ID custody; the TUI requires explicit target/deletion review and exact revision-bound confirmation. | none |\n",
)
replace_once(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    "Issue #10 is evidence-backed and complete. RECONCILIATION-001C1 has certified the SQLite lifecycle half of issue #13. Next:\n\n1. design a prepared legacy-file lifecycle receipt that binds the C1 source/target metadata, complete counts, deterministic revision, transformed payloads, affected days, and retired-ID result;\n2. publish and replay catalog, session, tag, canonical-sand, daily-artifact, detached-checkpoint, and lifecycle-custody effects idempotently from every durable kill point;\n3. expose one explicit TUI preview and confirmation flow that recomputes the revision before mutation and keeps archive as the ordinary retirement path;\n4. close issue #13 only after both authorities and the visible interaction are evidence-backed.\n\nDo not treat archive as deletion, reuse a retired identity, or invent reassignment for unresolved references.\n",
    "Issues #10 and #13 are evidence-backed and complete. Next:\n\n1. resolve issue #22: the active draft versus durable category metadata distinction;\n2. later define complete profile identity, isolation, and deliberate switching under issue #15.\n\nDo not weaken the completed archive/lifecycle distinction, retired-ID custody, or fail-closed handling of unresolved references while pursuing later domain work.\n",
)

# C2 accepted work record.
replace_once(
    "notebook/work/RECONCILIATION-001C2.md",
    "state: active\nauthority: working\n",
    "state: accepted\nauthority: accepted\n",
)
path = Path("notebook/work/RECONCILIATION-001C2.md")
content = path.read_text()
content += '''

## Implemented result

- `legacy_category_lifecycle` inventories catalog/session/active/tag/canonical-sediment/daily-artifact/checkpoint references and binds them with a deterministic revision;
- preparation rejects idle, self-merge, missing target, stale preview, nonzero-reference deletion, malformed checkpoint, and unresolved transition receipts before publication;
- one private prepared receipt stores every exact resulting authority artifact plus the permanent lifecycle ledger before any result file changes;
- replay publishes sessions, tags, sediment, affected daily artifacts, checkpoint, catalog, and permanent ledger deterministically, accepts exact matches, rejects conflicts, and clears evidence only after convergence;
- eight injected post-receipt publication boundaries retain evidence and converge on clean retry without duplicate mutation;
- startup replays prepared lifecycle evidence before ordinary legacy state load;
- permanent ledger custody prevents retired-ID reuse across restart and participates in new category allocation;
- migration fingerprints and imports legacy lifecycle receipts into SQLite schema 7 and preserves the identity high-water mark;
- `x` remains ordinary archive and configurable `Shift-X` is the distinct merge/permanent-delete action;
- atlas, palette, key resolver, category modal, active-layer route, and recovery ownership expose the same action truth;
- the blocking overlay requires explicit target or deletion selection, displays all affected counts and revision, and accepts only the exact phrase derived from source, target, and revision;
- legacy review writes a fresh runtime checkpoint first so the active generation cannot be omitted; successful application reloads through the authoritative replay path;
- SQLite review delegates mutation to the certified C1 transaction and reloads the resulting authority;
- SQLite TUI allocation advances beyond lifecycle receipt identities after reload.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 246 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 15 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- eight legacy lifecycle replay fault boundaries: retained prepared evidence and idempotent convergence;
- source-idle, self-merge, missing-target, stale-preview, nonzero-delete, malformed/protected checkpoint, and transition-receipt refusal proofs: pass;
- session identity/chronology, target metadata, tag order, sediment mass/FIFO, daily contribution, checkpoint, catalog, and permanent-ledger preservation proofs: pass;
- restart retired-ID nonreuse and legacy-ledger-to-SQLite migration custody: pass;
- archive/lifecycle keymap distinction and exact phrase unit proofs: pass;
- live PTY proof captured the rendered phrase, typed it back, committed one merge receipt, retired the source, preserved target metadata, and completed normal shutdown: pass;
- permanent diff audit: intended source, test, authority, and Notebook files only;
- temporary transformation, interaction, process-proof, and workflow machinery: absent from the permanent tree.

RECONCILIATION-001C2 completes the legacy, migration, and interaction half of issue #13. Together with RECONCILIATION-001A and RECONCILIATION-001C1, category archive, merge/reassignment, zero-reference permanent deletion, crash recovery, and retired-identity custody are fully implemented and certified across every supported authority.
'''
path.write_text(content)
