# Strata accepted decision index

Status: accepted authority
Last reviewed: 2026-08-02

Detailed rationale and unresolved implications live in `notebook/decisions/DECISION-REGISTER.md`. This file contains only decisions accepted strongly enough to constrain implementation.

| ID | Decision | State |
|---|---|---|
| STRATA-D001 | Strata combines a continuous temporal ledger with an active timer; these are complementary rather than competing models. | accepted |
| STRATA-D002 | Rename the baseline `drift` concept to `idle`. | accepted; runtime vocabulary pending |
| STRATA-D003 | Idle time continues producing sediment but is omitted from ordinary active-time accounting. | accepted |
| STRATA-D004 | Strata is general-purpose across study, habits, projects, work, leisure, and other user-defined activities. | accepted |
| STRATA-D005 | Chronological ledger truth and sedimentary visual truth are both historically meaningful, with different precision obligations. | accepted |
| STRATA-D006 | Sediment is part of the product's artistic and functional meaning, not disposable decoration. | accepted |
| STRATA-D007 | Mixed foreground color inside one Braille cell intentionally represents subcell composition and sand mixing. | accepted |
| STRATA-D008 | The current visual quantum is one grain per elapsed second. | accepted current behavior |
| STRATA-D009 | SQLite is the live authority after explicit activation; deterministic CSV remains first-class interchange. | implemented and certified |
| STRATA-D010 | Migration and activation are explicit commands rather than automatic startup mutation. | implemented and certified |
| STRATA-D011 | Authority failures fail closed; no writable empty fallback or activated legacy fallback is permitted. | implemented and certified |
| STRATA-D012 | Legacy sources remain evidence until archive-first, provenance-verified, separately confirmed removal. | implemented and certified |
| STRATA-D013 | CLI and TUI share one validated startup configuration; invalid configuration blocks authority resolution unless `--ignore-config` is explicitly supplied. | implemented and certified |
| STRATA-D014 | Live duration is monotonic; persisted timestamps are UTC; civil projection uses the validated fixed offset; persisted operational-day keys own historical grouping; ambiguous clock discontinuities fail closed. | implemented and certified |
| STRATA-D015 | A logical session remains one canonical ledger identity; reports allocate its duration through exact operational-day overlap slices using policy captured with the session. | implemented and certified |
| STRATA-D016 | Fixed-clock policy is the only supported operational-day mode; the former sunrise label is removed and migrated visibly because no solar calculation existed. | implemented and certified |
| STRATA-D017 | Zero-whole-second finishes and switches are transactional transition events with receipts, not completed work rows or sediment. | implemented and certified |

## Explicitly unresolved

The following are not accepted decisions:

- final vertical chronology semantics;
- flat layers versus optional context or relationships;
- final `Karma`/balance terminology;
- clearing and formation lifecycle;
- user-facing crash uncertainty beyond current recovery mechanics;
- configurable quantum migration rules;
- complete profile switching and isolation semantics under issue #15;
- future adoption of IANA timezone/DST semantics, if any; the implemented authority is fixed-offset.
