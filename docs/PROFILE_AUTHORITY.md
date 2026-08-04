# Profile authority

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
