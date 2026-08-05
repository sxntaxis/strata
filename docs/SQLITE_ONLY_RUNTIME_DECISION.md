# SQLite-only runtime decision

Status: authoritative
Date: 2026-08-04

Strata has no deployed users and no compatibility obligation to preserve the earlier CSV/JSON runtime, storage-authority marker, activation ceremony, or historical schema-upgrade path.

The product target is one clean implementation:

- every profile owns exactly one SQLite database at `data/strata.sqlite3`;
- CLI and TUI open or create that database directly;
- profile identity is stored and validated inside the database;
- no runtime authority enum or fallback exists;
- no `migrate-sqlite`, `activate-sqlite`, or legacy-evidence lifecycle commands exist;
- no CSV/JSON start, stop, report, export, sediment, tags, checkpoint, or category authority remains;
- the database is created from one current schema rather than replaying obsolete production migrations;
- development fixtures may be discarded or rebuilt; compatibility code is not a product requirement.

The mixed TUI/CLI ownership failure is treated as evidence against dual authority, not as a reason to add another lock around the obsolete file-backed runtime.
