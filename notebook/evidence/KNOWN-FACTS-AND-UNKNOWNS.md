# Known facts and unknowns

Last reconciled: 2026-08-01

## Verified facts

- Strata is a Rust 0.7.6 TUI/CLI application.
- The SQLite migration program merged through SQLITE-012 and issue #8 closed at 9/9 PASS.
- Activated CLI and TUI use the same SQLite authority.
- Runtime transitions are fenced and receipt-backed; persistence failures have explicit recovery.
- The complete CI baseline after SQLITE-012 is 119 unit tests, 7 legacy lifecycle process tests, 11 SQLite authority/TUI process tests, strict Clippy, formatting, and doc tests.
- Deterministic CSV bundle dry-run/import and provenance-verified legacy archive/removal are implemented.
- User-facing runtime vocabulary uses `idle`; historical `drift`/`none` spellings remain compatibility aliases and some internal identifiers retain legacy names.
- PR #29 was based before the SQLite campaign and is stale/non-mergeable; this adoption supersedes it.

## Material unknowns

- Whether issue #21 affects every current CLI command or only paths that call the permissive configuration loader.
- The correct explicit bypass surface for broken configuration (`--ignore-config`, `--profile`, or both).
- The final time authority under wall-clock jumps, suspend, timezone changes, and historical report reproduction.
- Which pre-SQLite issues are fully satisfied versus only premise-changed.
- The logical sediment representation needed to conserve mass and topology independently of viewport.

## Evidence discipline

Issue descriptions are historical defect reports. Reconcile them against current main before closing or implementing them; do not assume their original code-path descriptions remain current.
