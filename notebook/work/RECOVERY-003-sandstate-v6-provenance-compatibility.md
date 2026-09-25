---
id: RECOVERY-003
kind: work
state: completed
authority: accepted
created: 2026-09-25
updated: 2026-09-25
summary: Read the known v0.7.7 SandState v6 provenance prototype as current v5 without changing canonical topology or Classic continuation.
---

# RECOVERY-003 — SandState v6 provenance compatibility

## Evidence

- The installed pre-main binary wrote `SandState.version = 6`.
- Current GitHub main is `SandState.VERSION = 5` and rejects v6 during runtime restore.
- The v6 change adds per-grain spawn timestamps and provenance fields on pending runs; it does not replace the canonical coordinate/category grid or Classic runtime payload.
- The profile SQLite database passes integrity and foreign-key checks. A verified pre-recovery backup was created outside the repository.

## Owner outcome

The authoritative current-main runtime must be able to open the known v6 profile while preserving visible topology, category identity, pending mass, and Classic continuation. The prototype's unsupported timestamps are ignored on import; current canonical writes use v5. Unknown future versions remain fail-closed.

## Proof obligations

- v6 JSON with grain provenance and Classic metadata restores as exact current v5 topology/Classic state;
- v5 state and all earlier migrations remain unchanged;
- v6 day-end snapshots remain renderable and immutable;
- recovery validation accepts the known v6 profile payload;
- verify full tests, strict Clippy, installed version, doctor, and the real profile's unchanged canonical grain coordinates/categories across the startup compatibility edge.

## Result

Compatibility is published in main `2031edf` and installed as `strata 0.7.7 (2031edf06dde)`. SandEngine and Classic restoration accept the known v6 provenance envelope by treating it as v5 topology; bounded recovery writes normalized v5 state. Tests verify placed coordinates, category identity, pending mass, exact Classic runtime state, and identical future Classic continuation. Formatting, strict Clippy, 517 unit tests, 24 integration tests, and CLI help smoke pass.

A disposable profile restored from the verified pre-recovery backup started and exited successfully under the installed binary; `sqlite-doctor` passed afterward, its canonical `sand_state` was v5 with 15,120 placed grains and Classic schema 1, and retained daily visual rows continued to include their v6 payloads. The production profile was inspected read-only, still has healthy SQLite v1 with canonical SandState v6 and 15,118 grains, and was not modified during the smoke proof. The original verified backup remains at `/mnt/Tokyo/Lab/.tmp/opencode/strata-profile-before-sand-v6-recovery-20260925.sqlite3`.
