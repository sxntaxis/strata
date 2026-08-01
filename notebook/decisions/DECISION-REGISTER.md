---
id: DECISION-REGISTER
kind: decision-register
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Current accepted, candidate, deferred, and unresolved product decisions for Strata.
---

# Decision register

States:

- **accepted** — explicitly approved and eligible for promotion or already promoted;
- **candidate** — developed enough for review but not approved;
- **deferred** — intentionally postponed with a revisit condition;
- **rejected** — considered and refused;
- **open** — material question without a preferred answer;
- **superseded** — replaced by a newer decision while retained as provenance.

| ID | Decision | State | Authority / consequence | Notes |
|---|---|---|---|---|
| STRATA-D001 | Strata is simultaneously a continuous temporal ledger and an active timer. | accepted | promoted to `docs/PROJECT.md` | Active selection gives continuous time a layer; it does not create time itself. |
| STRATA-D002 | Rename the baseline `drift` concept to `idle`. | accepted | promoted; implementation pending | The old term suggested accidental or unclassified time rather than the intended baseline. |
| STRATA-D003 | Idle continuously produces sediment but is omitted from ordinary active-time accounting. | accepted | promoted | Idle remains historically present without being counted as active layer time. |
| STRATA-D004 | Strata is general-purpose rather than freelancing-specific. | accepted | promoted | Study, habits, projects, work, leisure, and other uses are peers. |
| STRATA-D005 | Preserve exact chronological history and accountable sedimentary history as two related truths with different precision. | accepted | promoted | Sediment is visualization but not disposable decoration. |
| STRATA-D006 | Total represented time and per-layer sediment mass require exact conservation. | accepted | product constraint; implementation proof pending | Administrative operations must not silently invent or lose material. |
| STRATA-D007 | Topology, contours, neighborhoods, color composition, and broad chronology are historically meaningful. | accepted | product constraint; tolerance model open | Their exact preservation precision remains to be specified. |
| STRATA-D008 | One visual grain currently represents one elapsed second. | accepted | current behavior | Configurable quantum remains a future candidate. |
| STRATA-D009 | Mixed color inside one Braille cell is intentional and represents subcell composition and sand mixing. | accepted | presentation constraint | Future blending changes must preserve legibility without pretending seams can be avoided. |
| STRATA-D010 | Tools can be art; Strata's artistic behavior is part of product function. | accepted | evaluation doctrine | Critique must begin from the intended concept rather than conventional timer assumptions. |
| STRATA-D011 | Replace live CSV/JSON authority with SQLite while retaining first-class deterministic CSV import/export. | accepted | promoted to `docs/ARCHITECTURE.md`; issue #8 | Implementation and migration proof pending. |
| STRATA-D012 | Use a user-defined directional balance axis rather than a universal moral judgment. | accepted concept | terminology unresolved | Work/leisure is one interpretation among many. |
| STRATA-D013 | Rename the `Karma` surface to `Balance`. | candidate | product-owner review required | `Polarity` or `valence` may remain the layer property while Balance names the aggregate view. |
| STRATA-D014 | A grain has one primary visual layer plus optional context metadata. | candidate | schema and interaction consequence | Preserves one principal color while allowing subject, project, goal, or method context. |
| STRATA-D015 | Keep layers completely flat with no secondary context. | open alternative | conflicts with D014 | Simpler and more immediate, but may force unlike meanings into one list. |
| STRATA-D016 | Use broad vertical chronology with local physical emergence. | candidate | simulation and persistence consequence | Macrostructure records age while local grains may mix, roll, and settle. |
| STRATA-D017 | Make vertical position strict chronology. | open alternative | simulation consequence | Strongest geological record but may overconstrain living sand. |
| STRATA-D018 | Let position represent only physical settlement. | open alternative | metaphor consequence | Most natural simulation but weakest strata chronology. |
| STRATA-D019 | A deliberate detach means low-power continuation of the selected layer until reopen. | candidate | recovery contract | Requires exact ledger continuation and bounded sediment materialization without blocking interaction. |
| STRATA-D020 | Unexpected termination should be treated identically to deliberate detach. | rejected | recovery contract | A crash does not carry the same intentional classification signal. |
| STRATA-D021 | After unexpected termination, the elapsed interval remains real but its classification may require confirmation, adjustment, or idle assignment. | candidate | recovery UX and schema consequence | Default policy may later be configurable. |
| STRATA-D022 | Recovery animation may represent reconstruction but must not delay logical current state or present-day commands. | candidate | recovery implementation | Ledger and active interaction become current immediately. |
| STRATA-D023 | Clearing sediment should permanently erase the current formation by default. | rejected | interaction safety | Practical visibility needs do not justify implicit historical destruction. |
| STRATA-D024 | Provide separate operations for hiding idle, beginning a new formation, compacting older material, rebuilding, and permanent deletion. | candidate | formation lifecycle | Exact first release scope remains open. |
| STRATA-D025 | Make temporal quantum configurable for new formations. | deferred | revisit after formation model | Existing-formation migration and mixed-quantum display must be resolved first. |

## Immediate decision gates

1. **Detach and crash semantics** — review D019, D021, and D022 before SQLite active-session and recovery schema design.
2. **Layer and context model** — choose between D014 and D015 before project/context fields are embedded into persistence.
3. **Vertical chronology** — choose among D016, D017, and D018 before redesigning resize, snapshot, and reconstruction behavior.
4. **Formation lifecycle** — define the minimum accepted subset of D024 before replacing current clear commands.
5. **Balance vocabulary** — review D013 after the underlying polarity/valence model is made explicit.
