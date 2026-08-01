# Strata decision register

## Accepted

| ID | Decision | Evidence/state |
|---|---|---|
| STRATA-D001–D008 | Continuous ledger, idle baseline, general-purpose scope, dual historical truths, artistic sediment, Braille mixing, one-second quantum. | Accepted in `docs/PROJECT.md`. |
| STRATA-D009 | SQLite becomes sole live authority after explicit activation; deterministic CSV remains interchange. | Implemented through SQLITE-001–012; issue #8 closed. |
| STRATA-D010 | Migration and activation are explicit, not automatic startup mutation. | Implemented and documented. |
| STRATA-D011 | Authority failures fail closed with visible recovery; no writable empty or stale-file fallback. | Implemented and fault-certified. |
| STRATA-D012 | Legacy evidence uses archive-first, exact-provenance, separately confirmed removal. | Implemented in SQLITE-012. |

## Candidate

- One validated settings object should own profile/database selection, operational boundary, timezone, and keymap configuration for both CLI and TUI.
- A profile should identify a complete authority rather than a partial path override.
- Historical sessions should snapshot sufficient temporal policy to make reports reproducible after configuration changes.
- Logical sediment mass should be independent of terminal viewport capacity.

## Open

- Final vertical chronology meaning.
- Layer-only versus layer-plus-context model.
- Final balance terminology.
- Formation clearing/archival/compaction lifecycle.
- User-facing crash uncertainty and inferred-time confirmation.
- Temporal quantum migration.
- Actual sunrise semantics versus removal of the claim.
