# Interaction authority

Status: accepted authority; STRATA-D071/D072/D073/D074 are natively certified; STRATA-D075/D076 remain the validated/reviewed frontier; STRATA-D077-D081 owner review is staged as an implementation candidate pending native validation
Program: INTERACTION-001 + INTERACTION-002 convergence
Current completed unit: INTERACTION-002; PLATEAU-001H H1 presentation hardening certified
Issues completed: #19, #20, #24
Last reviewed: 2026-09-30

## Purpose

Interaction authority determines whether an input is navigation, a command, text, confirmation, cancellation, contextual policy, or mandatory emergency control, and who owns the host terminal while the TUI is active. Ambiguous focus must not mutate history, hidden fallback behavior must not bypass configuration, and runtime failure must not strand the terminal in application mode.

## View and edit ownership

Balance layer detail is the layer's ledger surface, not a second report modal. Ordinary navigation, report-period commands, deletion, modal cancellation, and quit remain commands while no ledger entry is being edited.

Confirm on a completed persisted entry opens an in-row ledger editor owned by that stable session ID. The editor copies the entry's Tag plus complete civil start/end boundaries; row ordering or later selection movement cannot retarget that draft. A synthetic final `+ Add entry…` row opens the same in-row grammar as a new historical entry already scoped to the current layer. The provisional live generation is also a first-class ledger identity: its Tag and civil start boundary are editable while its open end remains `now`.

## Ledger edit-mode input

The active ledger field exclusively owns ordinary text input:

- Tag accepts unmodified character input and `;` separates independent attribution tags;
- an empty current Tag segment has no completion; after its first typed character, a known tag may appear as a dim completion suffix without becoming stored text;
- Left/Right in Tag cycles known tags for only the current semicolon segment. If that segment began as a typed prefix, cycling is restricted to known tags matching that prefix until ordinary typing/backspace starts a new cycle. An exact known tag has no retained prefix and therefore cycles the available known-tag set. Tags already used in prior segments are excluded;
- date fields accept `YYYY-MM-DD` characters;
- time fields accept forgiving hour/minute/optional-second forms, including compact input such as `6`, `650`, `1530`, or `65030` and colon input such as `6:50` or `6:50:30`. Colon-separated components accept at most two digits but are not range-blocked while typing: semantic action normalizes missing components to zero and carries overflow arithmetically through seconds → minutes → hours → civil Date (`12:99` becomes `13:39`, `99:00` carries four civil days to `03:00`);
- Backspace/Delete edits around the real caret. `Ctrl+Left` / `Ctrl+Right` moves that caret one character and Home/End moves to field start/end; plain arrows retain field semantics;
- Tab always normalizes the current temporal value when interpretable, then moves to the next field; Shift+Tab/BackTab does the same toward the previous field. Completion never intercepts field navigation;
- Left/Right in a Date field changes the selected civil date by one day, while Shift+Left/Right changes it by one civil month, preserving the day where possible and clamping to the destination month end otherwise. In a Time field, arrow magnitude follows the precision the user expressed: without seconds Left/Right is ±1 minute and Shift is ±1 hour; with seconds Left/Right is ±1 second and Shift is ±1 minute. Normalization occurs before adjustment and carries/borrows across the civil Date;
- Up/Down have no ledger-edit meaning;
- Enter validates and commits;
- Esc cancels the draft, or dismisses an add-collision confirmation back to editing;
- only mandatory `Ctrl-C` may escape as an application-level command.

A same-day entry exposes one date plus start/end time; moving the end earlier than the start represents the next civil day. Entries already spanning distinct dates expose both complete date boundaries. No ordinary Balance command letter is interpreted while the row editor owns input.

## Ledger persistence boundary

A draft changes canonical history only after explicit commit. SQLite remains the sole runtime persistence authority.

Editing an existing completed entry preserves its layer identity and keeps the source stable identity where the corrected target row continues, while allowing Tag and civil start/end correction. Add and Edit use the same historical-correction planner. The transaction requires `start < end <= now`; changes confined to the source row plus implicit Idle need no warning, while changes to another explicit row or relevant current recorded activity require the exact collateral Before/After confirmation before commit. Every affected daily-contribution projection is replaced atomically and memory reloads only after SQLite commit.

Adding through `+ Add entry…` uses that same HISTORY-001D/001E correction transaction with the layer fixed by the detail view. It can fill true gaps, replace Idle or same-layer chronology without confirmation, and carve another explicit layer only after the collateral preview is accepted. The row owns an explicit Tag field and persists its canonical description with the inserted history.

A failed persistence attempt leaves the draft visible and canonical in-memory history unchanged under the existing recovery contract. A successful commit closes edit mode. Esc closes edit mode without a write.

## Visible ledger state

The normal layer detail keeps the ledger itself as the interaction surface rather than showing a permanent instruction footer. Existing entries remain compact rows; the final `+ Add entry…` row is part of normal selection. During edit, the selected row changes in place. Literal `[]` are never selection chrome: real text ownership is shown by the terminal caret, while selected values/facets use the ordinary Strata highlight. Validation or collision text appears only when needed.

Single-day rows use **Tag · Time · Effect** with Time exactly centered and mirrored outer Tag/Effect regions. Rows that show Date use **Tag · Date · Time · Effect**, with Date/Time as a symmetric inner pair and Tag/Effect as mirrored outer regions. Long data may widen the modal within terminal/margin bounds, but one wide Tag never forces every cell to that width. Transient cursor, completion, focus, warning, or filter chrome never participates in modal geometry. A single-day row being edited may alone expose Date without forcing the other rows to four cells or resizing the frame merely because edit mode began. Effect remains visible during editing and previews the draft contribution to the current report window. Truncation remains a physical-terminal fallback.

Closing Balance, leaving layer detail, or replacing the edit with another explicit Balance editor discards the transient draft. Deletion remains a separate configured command and never applies to the synthetic Add row.

## Configured action state

Every configurable action has exactly one state:

- `Bound` — at least one direct configurable key reaches the action;
- `Unbound` — no direct key reaches the action and no explicit prohibition exists;
- `Disabled` — the action is explicitly prohibited from direct and contextual routing.

A null physical-key entry removes only that key. `unbind_actions` is the persisted Disabled marker. Removing every direct key without a disabled marker produces Unbound, not Disabled.

A configuration that binds and disables the same action is contradictory and rejected. Configuration does not silently choose one side of the contradiction.

## Mandatory key policy

`Ctrl-C → Quit` is the sole mandatory process-level key policy.

It is separate from configurable bindings, cannot be rebound or disabled, and resolves before configured direct or contextual actions. Attempts to configure or persist Ctrl-C as an ordinary binding fail before the invalid state is written.

Mandatory Quit remains under persistence-recovery custody. When recovery is active, Ctrl-C first exports the current recovery package and requests the established recovery exit rather than bypassing evidence custody or merely restarting the recovery loop.

F1 is not mandatory. It is an ordinary configurable default for `toggle_settings`; removing or disabling the action removes F1 behavior. `?` is the second default Settings binding except while Layer explicitly owns text input.

## Contextual action policy

One resolver accepts an explicit input context and returns a mandatory, direct, contextual, or absent action result.

Accepted inherited aliases are:

- `main.open_layer`: Confirm/open → Open Layer when the target is Unbound;
- `main.switch_idle`: Cancel/close → Switch to Idle when the target is Unbound;
- `balance.day`: Detach → Balance Day always when the target is not Disabled.

The former inherited `main.balance_today → detach` route is retired. Its behavior was misplaced: changing whether Detach had a direct binding could repurpose a historical-range key on Main. Older explicit alias spellings (`main.confirm`, `main.cancel`, `report.detach`) remain parser compatibility names for the current routes, and an explicitly configured legacy `main.balance_today` rule remains readable rather than silently reinterpreted.

Aliases are named configuration policy, not handler inspection. A disabled target is never reached. Removing an alias leaves the source action unchanged. Event handlers execute the resolver result without inspecting whether some other action has direct keys.

Modal-local text and capture controls remain owned by their explicit modal modes and are not represented as configurable action bindings.

## Shared keyboard grammar

STRATA-D074 makes modifier meaning contextual but consistent:

- unmodified arrows operate the currently focused value or selection;
- Shift is reserved for a larger form of the same directional adjustment. It does not switch Balance period presets, recolor layers, or reorder layers;
- Ctrl selects a structural/alternate Layer operation: `Ctrl+↑` / `Ctrl+↓` reorders the selected existing layer, `Ctrl+←` / `Ctrl+→` cycles its color, and `Ctrl+e` (without Shift) renames the selected non-Idle layer. `Ctrl+Shift+E` is distinct and does not invoke rename;
- while forging a new layer, plain `←` / `→` chooses color because there is no Tag axis yet;
- Main `Backspace` clears Idle sand and Main `Delete` clears all sand. Editors retain Backspace/Delete as text editing because they own those keys before action resolution;
- Layer Tag/name input owns printable characters before configurable command routing once text editing has begun. Uppercase letters and symbols therefore remain text rather than hidden Shift-letter commands; on an existing layer, the leading `+` / `=` and `-` / `_` keys are the deliberate exception while no Tag edit has started, preserving their positive/negative polarity actions. After any Tag text edit begins, those same symbols are ordinary text. A manually typed Tag therefore may contain but cannot begin with those four reserved symbols.
- `Ctrl+x` is the contextual destructive chord: archive the selected Layer in Layer, delete the selected persisted entry in Layer Detail, and open permanent-Layer deletion confirmation from the Balance summary. Plain `x` remains ordinary Tag/name text.

Layer rename is an isolated sub-editor. `Ctrl+e` starts from the current layer name; printable text plus Backspace/Delete edit the draft; Enter validates and saves it; Esc cancels. Empty names, reserved Idle names, and case-insensitive duplicates remain in edit mode with visible feedback. Idle cannot be renamed. Rename preserves the stable CategoryId, existing sessions, current session Tag, and legacy category-description data. The legacy description field is not exposed as an editing or command-palette search surface.
While a real text field owns input, Strata exposes the real terminal cursor at the insertion point and requests a blinking block cursor shape where the terminal supports that standard control sequence. Rename, Layer Tag typing, new-Layer naming, and inline ledger Tag/Date/Time editing use this cursor; non-editing navigation hides it. Cursor shape is presentation only and restoration returns control to the user's terminal-configured default shape.

The ordinary Layer Tag is a live preview, not an immediate persistence boundary. Opening Layer snapshots the current active Tag. Typing, Backspace, or Tag cycling on the active layer updates the running presentation immediately; Enter accepts that preview, while Esc restores the opening Tag before closing. A Tag drafted on an inactive layer remains prospective until Enter switches to that layer. Structural Layer mutations—color, order, polarity, rename, and archive—remain immediate and are not rolled back by Esc.

## Balance vocabulary cutover

The owner has accepted **Balance** as the report/historical surface vocabulary. HISTORY-001A changes the default main-view opener to `b` and current action/config names to `open_balance_popup` / `balance_*`. This vocabulary change must preserve the configured Bound / Unbound / Disabled model and contextual routing semantics described below.

## Balance custom-range editor

HISTORY-001B exposes arbitrary operational-day windows inside Balance without creating a second report surface or report engine. `day`, `week`, and `month` remain presets; custom ranges are backed by the same inclusive domain `ReportWindow`.

Balance chrome presents every interval as explicit **start and exclusive-end boundaries** even though the internal `ReportWindow` remains inclusive. A one-day internal window `2026-09-21..2026-09-21` therefore displays `Sep 21 – Sep 22`; a full internal week `2026-09-21..2026-09-27` displays `Sep 21 – Sep 28`. Current partial periods remain partial: the visible exclusive end is exactly one operational day after the last included internal day, not the nominal future end of the calendar period.

The configurable `balance_range_start` and `balance_range_end` actions default to `[` and `]`. Pressing either begins one boundary-edit transaction and snapshots the complete opening report range/navigation state; `[` and `]` may then switch which endpoint is active without creating a second transaction. The active endpoint is shown with ordinary selection highlight rather than literal brackets. Plain Left/Right previews a one-reporting-cycle move of the selected boundary, while Shift+Left/Right previews a one-civil-month move, preserving the day where possible and clamping to month end plus the same one-day minimum/current-day safety limits. Up/Down may continue to navigate the Balance list without cancelling the transaction. Enter is the sole commit and Esc the sole rollback; unrelated commands do not silently behave like Esc.

Without an active boundary handle, Left/Right keeps its existing whole-window older/newer navigation and Shift+Left/Right has no period-cycling meaning. Explicit `t` / `w` / `m` presets remain authoritative. Other explicit report-mode transitions may replace the transient boundary editor rather than creating nested range-edit state.

`balance_range` no longer has a default physical key. The existing inline From/To editor remains an unbound configurable/palette action pending a later Settings/palette pass, while `[` and `]` are the normal keyboard route for constructing a custom range. When invoked, its visible values still use the same boundary convention: `From 2026-09-21` / `To 2026-09-22` means exactly the single included operational day Sep 21.

While the range editor is active:

- From and To use `YYYY-MM-DD` boundary dates and require `From < To`;
- the focused field is visually explicit and starts selected as a whole field;
- typing a digit or `-` replaces the selected field and then appends normally;
- Backspace/Delete clears a whole selected field or removes one character otherwise;
- Tab/BackTab switches fields and selects the destination field;
- Enter validates and applies the complete range;
- Esc cancels without changing the active report window;
- only mandatory `Ctrl-C` may escape the editor as an application-level action.

Invalid dates, equal boundaries, or reversed bounds remain in edit mode with visible validation feedback. Applying a valid range updates summary rows, detail logs, provisional active time, and historical sediment selection together because all consume the same inclusive internal report window.

After application, left/right shifts the whole custom window by its own inclusive span. Movement toward the present never advances the window beyond the current operational day. Switching back to a day/week/month preset leaves custom mode and restores normal preset-offset navigation.

## Balance historical activity editor

The layer-detail ledger is the canonical historical-correction surface. The synthetic `+ Add entry…` row is the normal keyboard route: select it and Confirm. The configurable `balance_log_activity` action remains available to configuration/palette but has no default `l` binding. The former separate Balance-wide Layer/From/To editor is retired rather than maintained as a parallel correction interface.

Add and existing-row Edit share one correction planner/executor. The editor owns Tag plus complete civil start/end
boundaries. A fresh Add defaults to a recent canonical interval; Edit starts from the selected stable source row.
`From < To <= now` remains mandatory. Time owned only by the selected source and implicit Idle may be reassigned
without confirmation. If the requested correction changes another explicit completed entry or relevant current recorded
activity, the layer hero is temporarily replaced by the centered Before/After collateral card; Enter applies exactly
the observed plan and Esc returns to editing. Authority changes invalidate that preview rather than silently applying a
stale plan.

Historical editing never selects a new current activity. When the requested interval intersects the active generation,
persistence may finalize displaced past time and rebase/restart the same selected live layer after the corrected
interval. The live layer and live description remain protected. SQLite publishes chronology rewriting, active-generation
rebasing when needed, affected daily-contribution replacement, checkpoint state, and bounded retained-sediment recolor
atomically before memory installs the result.

The Tag field uses STRATA-D072 semantics: semicolon-separated values are canonicalized as individual attribution tags
and persisted in the existing session description field. Active-generation rebasing preserves the persisted live
description.

### Balance layer detail ledger

Entering a Balance layer keeps the same modal geometry and visual hierarchy while changing the content from aggregate
Balance to one layer's ledger. The frame title is the layer name; the global top-left interval, global net title,
side-border arrows, and ordinary instruction footer are absent. The selected report interval remains the same centered
bottom-border boundary object used by the parent Balance view.

The layer hero contains the layer's signed contribution for the selected interval and the same recorded-time meter used
by Balance. STRATA-D071 makes both projections independent on one shared scale: the full line is
`summary.total_seconds`, including Idle, while the Layer Detail marker is that layer's signed contribution divided by
the same denominator. The global marker does not define an envelope. Exact zero alone occupies equilibrium; every
representable non-zero contribution receives at least one cell toward its sign, and the meter consumes the available
instrument width.

Below the hero, the body is the editable layer ledger described above. Multiple operational-day slices belonging to
one canonical completed session project as one ledger row within the selected window, so cross-day sessions are not
presented as unrelated editable records. The synthetic `+ Add entry…` row is the final ordinary row for active layer
identities; archived layers remain browse/edit-only. When room permits, one noninteractive blank row separates the final
chronological entry from `+ Add entry…`; constrained panes collapse that presentation-only spacer before hiding data.
Layer Detail keeps its compact ordinary footprint but widens responsively only when actual visible data needs more horizontal space, always within the existing terminal-margin clamps. Three-cell rows keep Time exactly centered; four-cell rows keep Date/Time symmetrically placed around center. Modal width is not driven by edit mode, cursor, completion, selector chrome, or an editor-specific minimum-width floor. Only the physical terminal clamp permits data truncation.

`balance_filter`, default `f`, operates the selected ledger row. A single-tag row toggles that facet immediately. On a multi-tag row the first `f` opens an in-row selector without changing filter state; Left/Right moves ordinary selection highlight across facets, Enter or `f` applies that facet and closes the selector, and Esc cancels. Up/Down and Shift+Left/Right are inert while the selector owns the row. The filter itself remains an explicit OR-set: matching rows stay normal, nonmatching rows remain visible but dimmed, and an untagged row is filterable as the neutral `—` facet; `—` is presentation only and does not become stored Tag text. After selection is closed, Esc clears an active filter before ordinary Layer Detail back/close behavior resumes. The selected filter survives period navigation inside the same layer.

When a filter is active, the hero subtotal and marker use the union of matching ledger rows, counting each row once even
if it matches multiple selected tags. Only the numerator changes; STRATA-D071's full selected-period
`summary.total_seconds` remains the denominator. Filter state is presented on its own line below the meter using the
canonical `; ` tag separator rather than being appended to the subtotal. Nonmatching chronology uses theme-derived
secondary/de-emphasized presentation plus terminal dimming; filtering never introduces a hardcoded gray color or a
filter-specific appearance-schema field.

### Balance summary instrument

The default Balance summary presents a bipolar instrument above the unchanged category rows:

- The left total sums negative `balance_seconds`, the centered value is the authoritative report net, and the right total sums positive `balance_seconds`.
- The meter projects net displacement over the selected period's full `summary.total_seconds`, including Idle. Exact zero alone rests at center; representable non-zero values receive at least one cell toward their sign, and the meter uses the available instrument width rather than a fixed cap.
- The instrument uses responsive Ratatui geometry, with independently aligned totals and a meter sharing one exact centered width. Layer and the default Balance summary share one centered overlay rule: content defines a hard minimum, one third of the current terminal defines the proportional comfort floor, and configured frame margins provide the final clamp. This preserves breathing room on large terminals while preventing instrument overlap on narrow ones.
- Layer and the default Balance summary use a fixed one-cell horizontal content inset at every ordinary modal size rather than edge-hugging content, width-growing side padding, or a fixed narrow card. Only a physically tiny inner area that cannot retain even one content cell may collapse that inset. Layer uses exactly one top/bottom content cell whenever two spare rows exist, collapsing that vertical inset only when constrained, while Balance keeps its compact category rows and explicit breathing rows around the totals/meter/list groups, including a reserved bottom breathing row when height permits. Wider Balance rows may therefore retain a large name-to-metric gap; that is preferred over consuming surplus width as larger modal padding.
- The selected report interval is the sole centered bottom-border object with actual `←` / `→` navigation arrows. It always shows both interval boundaries with a spaced en dash; the active endpoint is highlighted in place while `[` / `]` remain only the keys that select it. Visible Day/Week/Month/Range labels, side-border arrows, and ordinary action hints are removed from the default summary; configured actions remain reachable through their existing routes, Settings, and the command palette.
- In the default summary, the frame follows the selected layer color just as Layer does; the left/right aggregate signs alone carry negative/positive polarity color while their duration digits remain subdued, and the centered net remains fully polarity-colored. Historical snapshot provenance remains explicit model authority but is omitted from normal-summary chrome. Detail, range-edit, activity-edit, and collision-confirmation modes keep their existing interaction guidance.

Balance summary rows are also a presentation filter: ordinary Layers whose displayed signed contribution is exactly zero are omitted, and Idle is omitted when its elapsed time is zero. Period changes may therefore make rows appear or disappear. Selection remains pinned by stable Layer identity when possible and falls to a remaining visible row when that identity is no longer present.

From the Balance summary, `Ctrl+x` requests permanent deletion of the selected non-Idle Layer through the universal `WARNING` overlay. Warning/error overlays are centered over the current content (including another modal), use the existing theme warning/error semantic roles, center subtitle/body content, and present left-aligned selectable action rows with ordinary selection highlight. A destructive warning defaults to its conservative `Go back` row; Enter executes selection and Esc takes the non-destructive exit. The same overlay family owns ledger-entry deletion and collateral-overlap confirmation; generic accent-blue `Enter apply` chrome is retired. Deletion removes that Layer's canonical sessions and Tag history and removes it from normal active/archived product surfaces; if it is currently active, Strata first switches safely to Idle. Current and persisted historical sediment carrying that CategoryId is atomically reclassified to Idle without moving grains or changing mass, topology, pending order/count, chronology, or simulation metadata, and rewritten snapshots receive a refreshed source identity. Persistence retains only an internal, non-restorable tombstone to prevent CategoryId reuse. In Layer Detail, the same `Ctrl+x` chord deletes only the selected persisted ledger entry and has no effect on `+ Add entry…`.

The default-summary instrument geometry is presentation-only. STRATA-D072 filtered Layer Detail intentionally changes the displayed layer numerator as specified above; interval semantics, historical-assignment behavior, collision confirmation, keymap authority, and command-palette reachability otherwise remain unchanged.

## Settings and palette truth

Settings and the command palette expose the same reachable action authority used by runtime. Settings is the plain product name for the former command-atlas surface; `Atlas` is not current user-facing vocabulary.

Settings displays:

- current configuration values that are editable in-app, beginning with **First day of week**;
- direct keys for Bound actions;
- `(unbound)` for Unbound actions;
- `(disabled)` for Disabled actions;
- mandatory Ctrl-C separately on Quit;
- contextual routes in human vocabulary rather than machine config identifiers;
- close, movement, and jump hints derived from current configured bindings;
- Backspace as **Disable action** and Delete as **Unbind** in binding capture.

The visible action sections are **Main**, **Navigation**, **Layer**, **Balance**, and **Settings**. Machine config names remain persistence/API vocabulary and are not used as the Settings action labels.

The default contextual routes are intentionally small:

- Main `Confirm / open` → **Open Layer** when Open Layer has no direct binding;
- Main `Cancel / close` → **Switch to Idle** when Switch to Idle has no direct binding;
- Balance `Detach` → **Day** always, preserving the established `d`/`t` day-range convenience.

The former `main.balance_today` fallback, which could turn the Balance-day key into Detach on Main when Detach was unbound, is retired as misplaced interaction.

Balance-specific default physical keys (`t`, `w`, `m`, `f`, `[`, `]`) own historical interaction inside Balance. They are not hidden Main shortcuts. `balance_range` and `balance_log_activity` remain unbound configurable/palette actions; normal keyboard interaction uses bracket boundaries and the navigable `+ Add entry…` row. Tag filtering remains contextual to an already-open Layer Detail because it operates on the selected ledger row.

Terminal character bindings are defined by the character/case event Strata receives, not by a promise to distinguish physical Shift from Caps Lock on every terminal protocol. Strata does not add Caps-Lock inversion or terminal-specific keyboard requirements. The bracket range controls require no Shift-letter distinction.

Disabled actions are removed from the command palette. Unbound actions remain available through deliberate palette invocation and are labeled `unbound`; palette selection is an explicit route rather than an invented physical binding. F1 and `?` remain ordinary configurable defaults for Settings because a plain-letter Settings key would conflict with Layer text entry. `?` remains literal text while Layer owns text input; F1 still opens Settings there.

## Terminal lifecycle ownership

One `TerminalSession` RAII guard owns:

- raw-mode acquisition and release;
- alternate-screen entry and exit;
- cursor restoration;
- terminal-output flushing;
- the ratatui terminal instance;
- registration with the process-wide panic restoration hook.

Terminal restoration is idempotent. The guard marks cleanup complete before issuing restoration operations, so explicit restoration, `Drop`, and the panic hook cannot perform the lifecycle transition more than once.

Startup failures after partial acquisition use the same restoration boundary. Cleanup attempts every applicable step even if an earlier step fails and returns aggregated cleanup context.

## Panic custody

A process-wide panic hook is installed once. While a terminal session is active, the hook restores terminal state before delegating to the previously installed hook.

Panic restoration does not claim that application state or an emergency checkpoint was persisted. Panic output remains owned by the prior hook after the host terminal has been restored.

## Runtime I/O failure custody

Draw, event-poll, and event-read errors leave the inner application loop and enter one outer failure boundary.

Before returning the runtime error, Strata attempts one direct runtime checkpoint using the same validated checkpoint payload and SQLite/file authority paths as ordinary checkpointing. The emergency attempt remains fail-closed when checkpoint prerequisites are unavailable, including active recovery or queued mutations.

The returned `io::Error` preserves:

- the original error kind;
- the original runtime failure text;
- emergency checkpoint success or failure as appended context;
- terminal cleanup failure as appended context.

Checkpoint or cleanup failure cannot erase or replace the primary draw, poll, or read error.

Application finalization and terminal restoration are separate. Normal finalization errors remain primary while cleanup failure is attached as context.

## Exactly-once restoration certification

Linux pseudo-terminal process tests capture `stty -g` before and after each run and require equality. A debug-only restoration marker proves that restoration executes exactly once.

Certified paths include:

- normal quit;
- detach with checkpoint evidence;
- injected draw failure;
- injected poll failure;
- injected read failure;
- injected panic.

Runtime I/O failures must leave emergency checkpoint evidence when prerequisites are valid. Panic must restore the terminal without reporting an emergency checkpoint success claim.

Test-only fault and restoration-marker environment variables are active only in debug builds.

## Certification

INTERACTION-001A through 001C pass:

- formatting;
- strict Clippy with all targets/features and warnings denied;
- 181 library tests;
- 9 CLI lifecycle tests;
- 6 configuration-authority tests;
- 1 report-help regression test;
- 12 SQLite/TUI process tests;
- 2 temporal-authority tests;
- 3 terminal-lifecycle PTY process tests covering six lifecycle paths.

Focused proofs cover explicit view/edit ownership, stable-ID draft persistence, idempotent restoration, primary-error preservation, termios restoration, emergency checkpoint publication, panic cleanup without false persistence claims, distinct Bound/Unbound/Disabled state, fail-closed configuration contradictions, mandatory-key protection, contextual alias conditions, disabled-route exclusion, and Settings/palette/runtime parity.

## Closure

INTERACTION-001 is complete. Future interaction work must preserve these boundaries rather than reintroducing hidden physical-key bypasses, handler fallbacks, ambiguous text ownership, or UI claims that differ from runtime reachability.

## Theme and layer-color interaction

Settings owns global theme selection. The existing Layer modal remains the per-category color surface: `Ctrl+←` / `Ctrl+→` cycles the active theme's eligible sand swatches for an existing layer, while plain `←` / `→` does so on the Forge-new-layer row. Themes may expose any non-empty number of sand colors. Strata derives a stable OKLCH hue wheel for navigation; declaration order and numeric slot position are not persistent semantics. A theme change recolors presentation by perceptual anchor matching and does not mutate category identity, history, or physics.
