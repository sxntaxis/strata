---
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
