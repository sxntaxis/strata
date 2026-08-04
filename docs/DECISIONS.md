# Strata accepted decision index

Status: accepted authority
Last reviewed: 2026-08-03

Detailed rationale and unresolved implications live in `notebook/decisions/DECISION-REGISTER.md`. This file contains only decisions accepted strongly enough to constrain implementation.

| ID | Decision | State |
|---|---|---|
| STRATA-D001 | Strata combines a continuous temporal ledger with an active timer; these are complementary rather than competing models. | accepted |
| STRATA-D002 | Rename the baseline `drift` concept to `idle`; historical names remain compatibility aliases only. | implemented and certified |
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
| STRATA-D018 | Project identity and category identity are independent canonical session axes; absent project remains empty rather than invented. | implemented and certified |
| STRATA-D019 | CLI starts require explicit category classification; idle is explicitly selectable and omission never silently becomes idle. | implemented and certified |
| STRATA-D020 | Report ranges are inclusive operational-day projections over exact canonical overlap slices; reporting never fragments or mutates the owning session. | implemented and certified |
| STRATA-D021 | The active interval is included by default in reports and exports as explicitly provisional state; `--completed-only` selects committed history. | implemented and certified |
| STRATA-D022 | Report/export ordering is deterministic; ICS uses stable identities and authoritative UTC chronology with RFC 5545-safe serialization, and fails closed rather than inventing timestamps. | implemented and certified |
| STRATA-D023 | Every due sediment grain is conserved logical mass in exactly one placed or pending form; physical ingress blockage never authorizes loss, and category identity persists in either form. | implemented and certified |
| STRATA-D024 | Terminal-cell dimensions and Braille-dot grid dimensions are separate named units; rendering emits one Braille character per drawable terminal cell while simulation and persistence operate in dot-grid units. | implemented and certified |
| STRATA-D025 | The persisted logical dot grid owns canonical sediment topology; terminal resizing is a centered, bottom-aligned projection-only operation and cannot mutate, repack, relax, or discard logical history. | implemented and certified |
| STRATA-D026 | Pending logical sediment is represented as ordered category/count runs; compression may reduce storage and work but cannot alter total mass, category identity, or FIFO category order. | implemented and certified |
| STRATA-D027 | Recovery event counts and accumulator remainders are calculated with checked integer arithmetic; detached duration must not require one loop iteration or allocation per missed event. | implemented and certified |
| STRATA-D028 | Runtime recovery follows claim → persist target → derive bounded pending mass → publish → retain or replace evidence; it never replays missed physics or installs a relaxed replacement topology. | implemented and certified |
| STRATA-D029 | Normal shutdown may retire only pending or committed checkpoint evidence; recovering and quarantined evidence remain protected. Runtime checkpoints are refused while mutations are queued, and legacy mutation-bearing evidence fails closed without stable receipts. | implemented and certified |
| STRATA-D030 | Historical sediment artifacts have explicit semantic kinds: cumulative checkpoint, daily contribution, or derived preview. These kinds are not interchangeable, and legacy cumulative daily rows cannot silently satisfy daily-contribution requests. | implemented and certified |
| STRATA-D031 | Historical snapshot viewing is immutable and projection-only: it never advances physics, mutates the artifact, or persists a derived preview; provenance, reconstruction status, source revision, and idle policy remain explicit. | implemented and certified |
| STRATA-D032 | Persisted daily sediment is a typed ledger-derived contribution accepted only when schema, kind, operational day, and sediment-relevant source revision exactly match canonical session slices. | implemented and certified |
| STRATA-D033 | Autosave, relevant mutation, and recovery completion reconcile every affected operational day; legacy cumulative daily rows and files remain archive-in-place evidence and are never reinterpreted as daily contributions. | implemented and certified |
| STRATA-D034 | Historical-description viewing and editing are explicit separate modes. Plain characters are commands in view mode and draft text in edit mode; Enter commits, Esc cancels, and only a configured modified emergency Quit may escape edit routing. | implemented and certified |
| STRATA-D035 | An edit draft is not canonical history. SQLite or legacy-file persistence must succeed before memory changes; failed commit retains the complete stable-ID draft and enters visible recovery. | implemented and certified |
| STRATA-D036 | One RAII terminal session owns raw mode, alternate-screen state, cursor restoration, output flushing, and the ratatui terminal. Explicit close, Drop, partial-startup failure, and panic converge on one idempotent exactly-once restoration boundary. | implemented and certified |
| STRATA-D037 | Draw, poll, and read failures attempt one direct emergency checkpoint, preserve the original I/O error kind and text, and attach checkpoint/cleanup outcomes only as context. Panic restores the terminal without claiming persistence success. | implemented and certified |
| STRATA-D038 | Every action has exactly one configured state: Bound, Unbound, or Disabled. Null physical-key entries remove only that key; `unbind_actions` means Disabled; contradictory bound-and-disabled configuration fails closed. | implemented and certified |
| STRATA-D039 | One context-aware resolver owns direct and contextual key routing. Ctrl-C Quit is the sole separate mandatory key, cannot be configured, and remains under persistence-recovery custody; F1 is an ordinary configurable default. | implemented and certified |
| STRATA-D040 | The command atlas and palette must expose the same reachable action graph as runtime: no invented fallback keys, disabled actions are unavailable, aliases are named with conditions, and editable controls distinguish Disable from Unbind. | implemented and certified |
| STRATA-D041 | A persisted session category reference must resolve to active or archived metadata. Malformed or unknown identities fail closed with the original value preserved; no authority may reinterpret an unresolved reference as intentional idle. | implemented and certified |
| STRATA-D042 | Category retirement changes availability, not historical meaning. Stable ID, name, description, color, karma effect, tags, report visibility, sediment meaning, and migration state survive archive and restore under both SQLite and legacy-file authority. | implemented and certified |
| STRATA-D043 | An active-session transition and retirement of its prior checkpoint generation form one coherence boundary. SQLite switch, reset, and finish may retire only pending or committed evidence for the expected prior stable ID; incompatible evidence aborts the transaction before history changes. | implemented and certified |
| STRATA-D044 | Checkpoint evidence must identify the authoritative active generation before recovery payload state is applied. Missing or mismatched identity is quarantined, and successful switch/reset/active-description changes publish current-generation evidence immediately at the semantic edge. | implemented and certified |
| STRATA-D045 | A legacy switch is a receipt-governed multi-file transition: publish the resulting checkpoint and deterministic receipt first, replay session/catalog effects idempotently, and clear the receipt only after every authority converges. Whole-second ledger semantics—not exact subsecond wall-start equality—own the completed row. | implemented and certified |
| STRATA-D046 | A normal legacy finish is a receipt-governed terminal transition: publish prior-generation evidence before active mutation, never resume a receipt-marked finished generation, and retire the receipt only after session, catalog, sediment, and every affected daily contribution converge. | implemented and certified |
| STRATA-D047 | Persistence recovery must preserve active and archived category meaning in reload, flush, sediment validation, and emergency export; recovery artifacts identify archival state explicitly. | implemented and certified |
| STRATA-D048 | Clear-all is a receipt-governed sediment operation plus provisional-idle reset, never committed-ledger deletion. SQLite publishes active, empty sediment, affected daily contributions, and checkpoint atomically; legacy replay restores exact active/grid state and clears evidence only after convergence. | implemented and certified |
| STRATA-D049 | A new SQLite TUI active generation and its first pending checkpoint are one bootstrap transaction after sediment restoration. Existing active or checkpoint evidence blocks bootstrap, and every failed write boundary leaves neither new row durable. | implemented and certified |
| STRATA-D050 | Sediment settles through the exact chronological transition timestamp under the outgoing category before switch, clear, or finish. Exact-boundary mass is outgoing; later mass is resulting; bounded FIFO settlement preserves mass without iterative replay. | implemented and certified |
| STRATA-D051 | Checkpoint recovery owns one persisted target reused across retry. Successful recovery must visibly distinguish durable evidence, reconstructed time through that cutoff, and post-target provisional live time; emergency export projects the same structured statement. | implemented and certified |
| STRATA-D052 | A SQLite category merge or permanent deletion requires one complete revision-bound preview and one immediate transaction. Merge reassigns every supported category-owned authority while preserving non-category identity, chronology, target metadata, sediment mass, and FIFO order; targetless deletion requires zero references. Every committed source identity is retired permanently through an auditable receipt preserved by backup, interchange, import validation, and doctor integrity. | implemented and certified |

## Explicitly unresolved

The following are not accepted decisions:

- final vertical chronology semantics beyond the accepted bottom-aligned viewport projection;
- flat categories versus optional context or relationships;
- final `Karma`/balance terminology;
- clearing and formation lifecycle beyond placed/pending mass conservation;
- future zoom, compression, panning, or explicit canonical-canvas migration;
- safe cross-authority replay of queued checkpoint mutations, if it is ever required;
- configurable quantum migration rules;
- complete profile switching and isolation semantics under issue #15;
- the legacy-file receipt/replay and explicit TUI confirmation half of category merge/reassignment and permanent deletion under issue #13;
- future adoption of IANA timezone/DST semantics, if any; the implemented authority is fixed-offset.
