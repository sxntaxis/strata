# Strata accepted decision index

Status: accepted authority
Last reviewed: 2026-08-01

Detailed rationale and unresolved implications live in `notebook/decisions/DECISION-REGISTER.md`. This file contains only decisions accepted strongly enough to constrain implementation.

| ID | Decision | State |
|---|---|---|
| STRATA-D001 | Strata combines a continuous temporal ledger with an active timer; these are complementary rather than competing models. | accepted |
| STRATA-D002 | Rename the baseline `drift` concept to `idle`. | accepted; implementation pending |
| STRATA-D003 | Idle time continues producing sediment but is omitted from ordinary active-time accounting. | accepted |
| STRATA-D004 | Strata is general-purpose across study, habits, projects, work, leisure, and other user-defined activities. | accepted |
| STRATA-D005 | Chronological ledger truth and sedimentary visual truth are both historically meaningful, with different precision obligations. | accepted |
| STRATA-D006 | Sediment is part of the product's artistic and functional meaning, not disposable decoration. | accepted |
| STRATA-D007 | Mixed foreground color inside one Braille cell is an intentional representation of subcell composition and sand mixing. | accepted |
| STRATA-D008 | The current visual quantum is one grain per elapsed second. | accepted current behavior |
| STRATA-D009 | SQLite should replace CSV/JSON as live authority while deterministic CSV remains first-class import/export. | accepted direction; implementation pending |

## Explicitly unresolved

The following are not accepted decisions:

- final vertical chronology semantics;
- flat layers versus optional context or relationships;
- final `Karma`/balance terminology;
- clearing and formation lifecycle;
- detached and crash-inferred interval semantics;
- configurable quantum migration rules.
