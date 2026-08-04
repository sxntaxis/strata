from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load(path: str) -> str:
    return (ROOT / path).read_text()


def save(path: str, text: str) -> None:
    target = ROOT / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text)


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1, found {count}")
    return text.replace(old, new, 1)


# Architecture authority.
path = "docs/ARCHITECTURE.md"
text = load(path)
text = replace_once(text, "Last reviewed: 2026-08-03", "Last reviewed: 2026-08-04", "architecture review date")
text = replace_once(
    text,
    "- `src/lib.rs` — shared CLI/TUI invocation and startup authority.\n",
    "- `src/lib.rs` — shared CLI/TUI invocation, process-bound profile selection, and startup authority.\n- `src/profile.rs` — stable profile UUID, rooted/XDG data-state-config ownership, manifest validation, and cross-profile artifact refusal.\n",
    "architecture profile map",
)
text = replace_once(
    text,
    "- `src/domain.rs` — canonical sessions, project/category identity, operational-day allocation, reports, and cloneable staged legacy transition state.",
    "- `src/domain.rs` — canonical sessions, project/category identity, session-owned active draft text, operational-day allocation, reports, and cloneable staged legacy transition state.",
    "architecture domain map",
)
text = replace_once(
    text,
    "- `src/storage.rs` — XDG paths, strict legacy active/archived category catalog, strict session identity/reference validation, atomic file helpers, legacy runtime checkpoint files, and custody-separated contribution files.",
    "- `src/storage.rs` — selected-profile paths, strict legacy active/archived category catalog, strict session identity/reference validation, atomic file helpers, legacy runtime checkpoint files, and custody-separated contribution files.",
    "architecture storage map",
)
text = replace_once(
    text,
    "- Idle is explicit, continues producing sediment, and remains excluded from ordinary active-time totals.\n\n### Category identity and archival",
    "- Idle is explicit, continues producing sediment, and remains excluded from ordinary active-time totals.\n\n### Active draft and category metadata\n\nThe active session description and durable category description are separate authorities.\n\n- `TimeTracker` owns one active-session draft independently from the category catalog.\n- Starting or switching to a category begins with an explicit draft supplied by the interaction; category metadata is never inherited implicitly.\n- Finishing commits the active draft into the completed session and clears only the draft.\n- Switch, finish, clear, detach, recovery, lifecycle reassignment, and legacy replay preserve durable category metadata.\n- SQLite persists active-description edits against the expected active stable ID before memory changes are accepted.\n- Legacy checkpoints and transition receipts carry the active draft independently from the unchanged category catalog.\n- The TUI defaults to draft editing; configurable `Shift-E` enters the separate durable metadata-editing mode.\n- Reusable category tags remain selectable aids and are not auto-applied as session text.\n\n### Category identity and archival",
    "architecture active draft section",
)
text = replace_once(
    text,
    "4. publish category-description authority;",
    "4. publish the unchanged category catalog while the resulting active draft remains receipt/checkpoint-owned;",
    "legacy switch wording",
)
text = replace_once(
    text,
    "Normal legacy finish uses a second certified receipt protocol. It publishes prior-generation evidence before active mutation, then converges completed history, cleared category metadata, canonical sediment, and every affected daily contribution before deleting the checkpoint.",
    "Normal legacy finish uses a second certified receipt protocol. It publishes prior-generation evidence before active mutation, then converges completed history, a cleared active draft, unchanged category metadata, canonical sediment, and every affected daily contribution before deleting the checkpoint.",
    "legacy finish wording",
)
text = replace_once(
    text,
    "The complete recovery contract and issue #10 closure are recorded in `docs/RECOVERY_AUTHORITY.md`.\n\n### Reports and exports",
    "The complete recovery contract and issue #10 closure are recorded in `docs/RECOVERY_AUTHORITY.md`.\n\n### Profile authority\n\nOne selected profile owns the complete process authority.\n\n- `--profile <directory>` is the explicit CLI/TUI selector; `STRATA_PROFILE` is the environment equivalent.\n- A rooted profile owns `data/`, `state/`, and `config/` beneath one canonical root and persists a schema-1 UUID manifest at `profile.json`.\n- Without an explicit root, one XDG profile owns the corresponding platform data, state, and configuration directories.\n- Historical `STRATA_DATA_DIR` is accepted only as a legacy whole-profile-root alias; it no longer redirects data independently from runtime state.\n- The selected profile is initialized before configuration or storage authority resolution and cannot change during the process.\n- `time_log_path` configuration and its live atlas editor are removed; obsolete configuration fails with explicit `--profile` migration guidance.\n- Legacy active-session files, detached checkpoints, recovery statements, and SQLite authority markers carry or validate the selected profile UUID.\n- Rooted profiles reject missing or mismatched artifact identity; copied active/checkpoint evidence from another profile fails closed.\n- Profile switching is deliberate close/open behavior: exit Strata and invoke it again with the target profile. There is no hot write redirection or in-memory dataset transfer.\n- `strata --profile <directory> profile [--json]` exposes the selected UUID and all owned paths.\n\nThe complete contract is `docs/PROFILE_AUTHORITY.md`.\n\n### Reports and exports",
    "architecture profile section",
)
text = replace_once(
    text,
    "### Active generation\n\nOwns the current stable active-session identity and its transition receipts. Checkpoint evidence may describe that generation but cannot replace authoritative active identity or survive a completed transition under a stale stable ID.",
    "### Active generation and draft\n\nOwns the current stable active-session identity, one session-owned description draft, and its transition receipts. Category metadata may describe the category but cannot supply, clear, or replace the active draft implicitly. Checkpoint evidence may describe that generation and draft but cannot replace authoritative active identity or survive a completed transition under a stale stable ID.",
    "active generation truth boundary",
)
text = replace_once(
    text,
    "### Chronological ledger\n\nOwns exact elapsed intervals, timestamps, categories, projects, descriptions, operational-day policy, and reportable totals.\n\n### Category catalog and lifecycle",
    "### Chronological ledger\n\nOwns exact elapsed intervals, timestamps, categories, projects, committed session descriptions, operational-day policy, and reportable totals.\n\n### Profile\n\nOwns the stable profile UUID and the complete data/state/config path set for one process. A path override, copied artifact, or live configuration edit cannot change profile authority. Missing or mismatched identity under an explicit rooted profile fails closed before state is applied.\n\n### Category catalog and lifecycle",
    "profile truth boundary",
)
text = replace_once(
    text,
    "## Current architectural frontier\n\nPersistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, crash-recovery authority, and category lifecycle across SQLite, legacy files, migration, and TUI confirmation are complete. The next priorities are:\n\n1. resolve the active draft versus category metadata distinction under issue #22;\n2. later profile authority, including complete isolation and deliberate switching under issue #15.",
    "## Current architectural frontier\n\nPersistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, crash-recovery authority, category lifecycle, active-draft ownership, and complete profile isolation are implemented and certified. The post-SQLite GitHub issue reconciliation queue is empty.\n\nFuture work must begin as a new explicitly justified unit rather than being inferred from superseded issue premises. Known non-blocking design questions remain listed in `docs/DECISIONS.md`.",
    "architecture frontier",
)
text = replace_once(
    text,
    "- An unresolved category reference is not idle.\n",
    "- Durable category metadata is not an active-session draft or reusable phrase template.\n- A data-file path override is not a profile and cannot redirect live authority.\n- An artifact from another profile is not recovery evidence for the selected profile.\n- An unresolved category reference is not idle.\n",
    "architecture non-authority additions",
)
save(path, text)

# Decision index.
path = "docs/DECISIONS.md"
text = load(path)
text = replace_once(text, "Last reviewed: 2026-08-03", "Last reviewed: 2026-08-04", "decision review date")
text = replace_once(
    text,
    "| STRATA-D053 | Legacy category lifecycle publishes one exact-result prepared receipt before any multi-file mutation, replays it idempotently before ordinary startup load, and retires it only after catalog, sessions, tags, sediment, daily artifacts, checkpoint, and permanent ledger converge. Ordinary archive remains `x`; merge or permanent deletion is a distinct configurable action requiring explicit target/deletion selection and the exact displayed revision-bound phrase. | implemented and certified |",
    "| STRATA-D053 | Legacy category lifecycle publishes one exact-result prepared receipt before any multi-file mutation, replays it idempotently before ordinary startup load, and retires it only after catalog, sessions, tags, sediment, daily artifacts, checkpoint, and permanent ledger converge. Ordinary archive remains `x`; merge or permanent deletion is a distinct configurable action requiring explicit target/deletion selection and the exact displayed revision-bound phrase. | implemented and certified |\n| STRATA-D054 | The active session owns one description draft independently from durable category metadata and reusable tags. Finish commits and clears only that draft; switch, recovery, replay, and lifecycle operations preserve category metadata. Draft editing is the ordinary category-modal route and durable metadata editing is a distinct configurable action. | implemented and certified |\n| STRATA-D055 | One process-bound profile UUID owns the complete data, state, and configuration path set. Explicit `--profile`/`STRATA_PROFILE` selection occurs before authority resolution; copied mismatched artifacts and partial `time_log_path` redirection fail closed; changing profiles requires exit and a new invocation. | implemented and certified |",
    "new accepted decisions",
)
text = replace_once(
    text,
    "- complete profile switching and isolation semantics under issue #15;\n",
    "",
    "remove resolved profile decision",
)
save(path, text)

# Dedicated profile authority.
save(
    "docs/PROFILE_AUTHORITY.md",
    """# Profile authority

Status: implemented and certified
Completed unit: AUTHORITY-002
Issue completed: #15
Last reviewed: 2026-08-04

## Purpose

A Strata profile is the complete authority boundary for one internally consistent dataset and its runtime custody. It prevents categories or sessions from one dataset from being combined with active state, recovery evidence, sediment, tags, configuration, or SQLite activation state from another.

## Selection

Profile selection happens once, before configuration and storage authority resolution.

Precedence:

1. explicit global `--profile <directory>`;
2. `STRATA_PROFILE`;
3. legacy `STRATA_DATA_DIR`, interpreted as a whole-profile-root alias;
4. the platform XDG profile.

Conflicting `STRATA_PROFILE` and `STRATA_DATA_DIR` values fail before authority opens. Once initialized, the profile cannot change inside the process.

## Rooted profile layout

An explicit profile root owns:

```text
<root>/profile.json
<root>/data/
<root>/state/
<root>/config/
```

The schema-1 manifest contains a stable UUID. Publication is atomic. Malformed, unsupported, or non-UUID manifests fail closed.

All legacy and SQLite paths derive from the selected profile:

- categories, sessions, and interchange sources;
- active-session state;
- detached checkpoints and transition receipts;
- canonical sediment, history, and daily contributions;
- category tags and lifecycle ledgers;
- recovery exports;
- keymap/configuration;
- SQLite database, migration artifacts, and authority marker.

## XDG profile

Without an explicit root, Strata uses one XDG profile across platform data, state, and configuration directories. The stable manifest is stored under the XDG data directory. Existing unbound legacy artifacts remain readable only in this compatibility profile; explicit rooted profiles require identity-bearing active and checkpoint artifacts.

## Artifact identity

The selected profile UUID is written to or projected through:

- legacy active-session files;
- detached runtime checkpoints;
- structured recovery statements and emergency exports;
- SQLite migration/activation authority markers.

A mismatched UUID is never rewritten or treated as current evidence. Rooted profiles also reject missing identity where ambiguity would permit cross-profile adoption.

## Switching doctrine

Profile switching is process-bound close/open behavior.

- There is no live switch command.
- There is no `time_log_path` hot redirect.
- The command atlas cannot edit one authority path independently.
- Runtime configuration reload may update supported key/time settings but cannot change profile identity or owned paths.
- To use another profile, exit Strata and invoke it again with `--profile <directory>`.
- No active session, pending mutation, checkpoint, or in-memory ledger is transferred implicitly.

`strata --profile <directory> profile` displays the UUID and owned paths; `--json` provides a deterministic machine-readable form.

## Failure policy

- a profile root that is not a directory fails before configuration load;
- conflicting environment selectors fail before mutation;
- invalid manifest schema or UUID fails before storage resolution;
- copied active-session evidence from another profile blocks stop/report use;
- copied detached checkpoint evidence from another profile enters visible fail-closed recovery rather than being applied;
- mismatched SQLite authority markers are invalid authority;
- obsolete `time_log_path` configuration fails with explicit `--profile` migration guidance.

## Certified proofs

- two explicit roots receive different persistent UUIDs and separate data/state/config trees;
- an active session started under profile A is absent from profile B;
- profile B cannot stop profile A's copied active-session file;
- profile A's completed ledger is never written under profile B;
- a detached checkpoint copied from profile A is refused under profile B in a real PTY process;
- obsolete partial path configuration is rejected by unit and process tests;
- profile UUID generation and manifest shape are validated;
- all existing persistence, migration, recovery, lifecycle, interaction, temporal, reporting, and terminal suites remain green.
""",
)

# Accepted work record.
save(
    "notebook/work/AUTHORITY-002.md",
    """---
id: AUTHORITY-002
kind: work
state: accepted
authority: accepted
created: 2026-08-04
updated: 2026-08-04
---

# AUTHORITY-002 — active-draft and complete-profile authority

## Scope

AUTHORITY-002 closes the two remaining post-SQLite issues in dependency order:

1. issue #22 — separate the active session's one-shot draft from durable category metadata;
2. issue #15 — bind every authority path and recovery artifact to one deliberate process-lifetime profile.

## Accepted result

### Active draft

- `TimeTracker` owns the active description independently from category storage;
- finish commits and clears only the draft;
- switch intent carries the exact next draft through queued execution and recovery serialization;
- SQLite active-description edits validate the expected active stable ID;
- legacy switch/finish/clear replay preserves category metadata;
- detach/recovery restores only the actually active draft;
- the category modal defaults to draft editing while configurable `Shift-E` enters durable metadata editing;
- reusable tags remain available but are never auto-applied.

### Profile authority

- `--profile`, `STRATA_PROFILE`, and the compatibility `STRATA_DATA_DIR` alias select one complete profile before configuration load;
- a profile UUID manifest binds data, state, and configuration directories;
- storage paths no longer own independent runtime overrides;
- `time_log_path` and its live atlas editor are retired with migration guidance;
- active files, checkpoints, recovery statements, and SQLite authority markers carry or validate profile identity;
- cross-profile active/checkpoint evidence fails closed;
- switching requires process exit and a new invocation;
- `strata profile [--json]` exposes the selected authority.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 248 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority process tests: pass;
- 2 profile-authority process tests: pass;
- 1 report-help regression test: pass;
- 15 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY tests: pass;
- active draft/category metadata independence: pass;
- legacy switch and finish crash replay with metadata preservation: pass;
- two-profile path and active-session isolation: pass;
- copied active-session refusal: pass;
- copied detached-checkpoint refusal under a real PTY: pass;
- obsolete hot path configuration refusal: pass;
- permanent diff audit: product source, tests, authority, and Notebook only;
- temporary transform workflows, scripts, and trigger files: absent.

AUTHORITY-002 completes issues #22 and #15 and empties the post-SQLite GitHub issue reconciliation queue.
""",
)

# Current status is deliberately concise and authoritative.
save(
    "notebook/NOW.md",
    """---
id: NOW-001
kind: status
state: active
created: 2026-08-01
updated: 2026-08-04
authority: working
summary: Every known GitHub issue is implemented and evidence-backed; active-draft ownership and complete profile isolation close the post-SQLite reconciliation program.
next: Maintain the certified baseline and open future work only through a new explicit issue or architecture unit.
---

# NOW — Strata

## Current phase

The post-SQLite issue reconciliation program is complete. No open GitHub issues remain at certification time.

The certified system includes:

- fail-closed SQLite/legacy authority and explicit activation;
- monotonic/UTC/fixed-offset time and exact operational-day allocation;
- canonical project, category, session, active-generation, and report identity;
- conserved sediment, bounded recovery, immutable historical artifacts, and revision-matched daily contributions;
- receipt-governed switch, finish, clear-all, and category lifecycle replay;
- active/archived category integrity, reviewed merge/deletion, and permanent retired-ID custody;
- explicit report editing, truthful keymap/palette/atlas routing, and exactly-once terminal restoration;
- session-owned active description drafts separated from durable category metadata and reusable tags;
- one process-bound profile UUID owning complete data, state, configuration, recovery, and SQLite authority paths;
- real process proofs for lifecycle confirmation, profile isolation, copied-artifact refusal, persistence failure, and PTY restoration.

## Completed post-migration units

- **AUTHORITY-001** — issue #21.
- **AUTHORITY-002** — issues #22 and #15.
- **TEMPORAL-001** — issue #25.
- **TEMPORAL-002** — issues #4, #23, #27.
- **DOMAIN-001** — issues #2, #12.
- **REPORT-001** — issues #1, #3, #14, #17, #28.
- **SEDIMENT-001** — issues #6, #7, #16, #18, #26.
- **INTERACTION-001A** — issue #19.
- **INTERACTION-001B** — issue #20.
- **INTERACTION-001C** — issue #24.
- **RECONCILIATION-001A** — issue #5 and historical-meaning portion of #13.
- **RECONCILIATION-001B1/B2A/B2B/B2C/B3A/B3B/B3C** — issue #10.
- **RECONCILIATION-001C1/C2** — issue #13.

## Verified final baseline

- formatting and strict Clippy pass;
- 248 library tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 2 profile-authority process tests;
- 1 report-help test;
- 15 SQLite/TUI process tests;
- 2 temporal-authority tests;
- 3 terminal-lifecycle PTY tests.

## Known non-blocking questions

The accepted implementation does not settle every possible future product direction. Remaining design questions include vertical chronology, optional category relationships, final Karma terminology, future sediment clearing/formation semantics, zoom/compression/panning, configurable quantum migration, possible IANA timezone support, and any future stable identity for queued cross-authority mutation replay.

These are not open implementation defects. They require new evidence and an explicit future unit before constraining the current system.

## Next

Preserve the certified baseline. New work begins only from a newly justified issue, decision, or architecture unit; superseded issue premises remain in Git history rather than current authority.
""",
)

# Close the reconciliation work record semantically.
save(
    "notebook/work/ISSUE-RECONCILIATION-001.md",
    """---
id: ISSUE-RECONCILIATION-001
kind: work
state: accepted
authority: accepted
created: 2026-08-01
updated: 2026-08-04
---

# ISSUE-RECONCILIATION-001 — completed post-SQLite queue

The original issue descriptions predated the completed SQLite migration. Each issue was re-evaluated against every supported authority and closed only after its current acceptance boundary received implementation and evidence.

## Final disposition

| Issues | Completed by |
|---|---|
| #8, #9, #11 | SQLite migration program |
| #21 | AUTHORITY-001 |
| #15, #22 | AUTHORITY-002 |
| #25 | TEMPORAL-001 |
| #4, #23, #27 | TEMPORAL-002 |
| #2, #12 | DOMAIN-001 |
| #1, #3, #14, #17, #28 | REPORT-001 |
| #5 | RECONCILIATION-001A |
| #10 | RECONCILIATION-001B1 through B3C |
| #13 | RECONCILIATION-001A, C1, and C2 |
| #6, #7, #16, #18, #26 | SEDIMENT-001 |
| #19 | INTERACTION-001A |
| #20 | INTERACTION-001B |
| #24 | INTERACTION-001C |

## Closure

- issue #22 is closed by explicit active-draft ownership, separate durable metadata editing, and recovery/replay proofs;
- issue #15 is closed by stable complete-profile identity, all-path isolation, copied-artifact refusal, and process-bound switching;
- every issue in the queue is implemented or was already superseded by certified authority;
- no open GitHub issue remains at closure time;
- future questions must enter through a new explicit unit and may not silently reopen superseded premises.
""",
)

print("AUTHORITY-002 authority promoted")
