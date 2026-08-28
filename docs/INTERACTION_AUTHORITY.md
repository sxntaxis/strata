# Interaction authority

Status: implemented and certified
Program: INTERACTION-001 + INTERACTION-002 convergence
Current completed unit: INTERACTION-002; PLATEAU-001H H1 presentation hardening certified
Issues completed: #19, #20, #24
Last reviewed: 2026-08-27

## Purpose

Interaction authority determines whether an input is navigation, a command, text, confirmation, cancellation, contextual policy, or mandatory emergency control, and who owns the host terminal while the TUI is active. Ambiguous focus must not mutate history, hidden fallback behavior must not bypass configuration, and runtime failure must not strand the terminal in application mode.

## View and edit ownership

Report-log view mode is read-only. Navigation, report commands, deletion, modal cancellation, and quit remain commands.

Historical description editing begins only after an explicit Confirm action on a selected persisted session row. Edit state owns:

- the stable session ID;
- a complete draft copied from the current persisted description;
- visible edit-mode status;
- commit or cancellation responsibility.

Selection movement or row ordering does not change the draft's owning identity.

## Edit-mode input

While report description edit mode is active:

- unmodified character input, including ordinary command letters, spaces, and Unicode, appends to the draft;
- Backspace and Delete remove one Unicode scalar from the draft;
- Enter requests one commit;
- Esc cancels the complete draft;
- unrecognized modified input is ignored;
- a modified key executes only when the mandatory key policy resolves it to emergency Quit.

No plain character is interpreted as a global or report command while editing. Outside edit mode, command routing follows the shared keymap resolver.

## Edit persistence boundary

A draft changes canonical history only after explicit commit.

SQLite authority performs one fenced session-description update. Memory changes only after that transaction succeeds.

SQLite is the only runtime persistence authority. Portable exports are projections and cannot be edited as a
second live session collection.

A failed persistence attempt:

- does not alter canonical in-memory history;
- retains the complete draft and stable session ID;
- enters the existing visible persistence-recovery state;
- allows retry or authority reload under the established failure contract.

A successful commit closes edit mode. Cancel closes edit mode without a persistence write. Description edits do not invalidate daily sediment contributions because description text is not sediment-relevant chronology.

## Visible edit state

The report modal displays either:

- `VIEW · Enter edit · Esc back`; or
- `EDIT DESCRIPTION · Enter commit · Esc cancel`.

The selected row renders the live draft with an explicit cursor marker. Closing the report modal, changing UI mode, or resetting report state discards any uncommitted draft.

Deletion remains a separate configured command. Enter no longer exits the report-log view; it enters editing. Quit from report context returns an application exit decision instead of being silently ignored.

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

## Balance vocabulary cutover

The owner has accepted **Balance** as the report/historical surface vocabulary. HISTORY-001A changes the default main-view opener to `b` and current action/config names to `open_balance_popup` / `balance_*`. This vocabulary change must preserve the configured Bound / Unbound / Disabled model and contextual routing semantics described below.

## Balance custom-range editor

HISTORY-001B exposes arbitrary operational-day windows inside Balance without creating a second report surface or report engine. `day`, `week`, and `month` remain presets; `range` is an explicit custom window backed by the same domain `ReportWindow`.

The default `balance_range` action is bound to `r` and opens an inline From/To editor. The editor starts from the currently displayed window so the user can refine an existing preset or custom range rather than re-entering both dates from memory.

While the range editor is active:

- From and To use `YYYY-MM-DD`;
- the focused field is visually explicit and starts selected as a whole field;
- typing a digit or `-` replaces the selected field and then appends normally;
- Backspace/Delete clears a whole selected field or removes one character otherwise;
- Tab/BackTab switches fields and selects the destination field;
- Enter validates and applies the complete range;
- Esc cancels without changing the active report window;
- only mandatory `Ctrl-C` may escape the editor as an application-level action.

Invalid dates or reversed bounds remain in edit mode with visible validation feedback. Applying a valid range updates summary rows, detail logs, provisional active time, and historical sediment selection together because all consume the same explicit report window.

After application, left/right shifts the whole custom window by its own inclusive span. Movement toward the present never advances the window beyond the current operational day. Switching back to a day/week/month preset leaves custom mode and restores normal preset-offset navigation.

## Balance historical activity editor

HISTORY-001C introduced the first historical editor from completed Idle detail rows. HISTORY-001D generalizes that
interaction into one Balance-wide **Log past activity…** operation. The configurable `balance_log_activity` action
defaults to `l`; it is available from Balance summary/detail views and through the command palette rather than
depending on a selected source session. Existing session rows are bookkeeping material, not part of the user
contract.

The inline editor owns three fields:

- Layer: an existing active layer, including Idle, cycled with Left/Right;
- From: civil timestamp `YYYY-MM-DD HH:MM:SS`;
- To: civil timestamp `YYYY-MM-DD HH:MM:SS`.

A fresh editor starts with the currently selected live layer when non-Idle, otherwise the first non-Idle layer,
and defaults to the last fifteen minutes of canonical history ending at the current active-preview snapshot time. The focused field owns its input; Tab/BackTab moves focus, Enter validates/commits,
and Esc cancels. Ordinary bound action characters do not escape the editor; only mandatory `Ctrl-C` remains an
application-level emergency command.

`From < To <= now` is mandatory. The operation may span true gaps and any number of completed rows. Idle and
already-target-layer time do not require confirmation. Any intersecting different explicit layer, including the
historical portion of the active generation, produces one visible collision preview. Enter on that preview
confirms replacement of exactly the observed plan; Esc returns to editing. If canonical authority changes before
confirmation, the old plan token is rejected and the user must preview the new collision set.

Historical editing never selects a new current activity. When the requested interval intersects the active
generation, persistence may finalize displaced past time and rebase/restart the same selected live layer after the
corrected interval. When the requested layer already matches the selected live layer and chronology becomes
continuous backward to it, the active start may move earlier. The live layer and live description remain
unchanged; changing what the user is doing now belongs to ordinary Main/Layer switching.

Persistence owns the mutation. SQLite publishes completed-history carving/insertion, active-generation rebasing when needed, every affected daily-contribution replacement, and any HISTORY-001E retained current-sediment recolor in one transaction. Runtime checkpoint sediment is updated to the same committed `SandState`, and memory installs that exact state only after commit. Recolor is category-composition reconciliation only: true gaps do not fabricate grains, cleared source mass may limit the applied amount, and authentic first-write historical day-end photographs remain unchanged.

The current HISTORY-001D editor does not expose a historical Tag field. Newly inserted retroactive rows therefore
use an empty description instead of borrowing the current live tag. If the active generation is rebased, its
persisted live description is preserved.

### Balance footer hardening

Daily-use proof showed that the original Balance bottom border exposed too many simultaneous implementation hints: current-sediment status, period choices, a bare `l`, and editor controls could all compete on one line. PLATEAU-001H therefore makes the footer contextual rather than encyclopedic.

- The default summary keeps the period choices centered and exposes the retroactive action as a legible key hint plus **Log past** rather than a bare letter or the ambiguous phrase `log activity`.
- The non-informative `live sediment` status is omitted for the current live interval; historical sediment provenance remains visible when Balance is actually showing a historical artifact.
- Detail and edit modes show only controls relevant to that mode. Period selectors are not repeated while a detail/editor owns the footer.
- The user-facing action name is **Log past activity…**; the stable configuration/API action remains `balance_log_activity`.

This is presentation hardening only. It does not alter historical-assignment semantics, collision confirmation, keymap authority, or command-palette reachability.

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

Balance-specific physical keys (`t`, `w`, `m`, `r`, `l`) own historical interaction inside Balance. They are not hidden Main shortcuts. The command palette remains the deliberate universal launcher: **Log past activity…**, **Custom range…**, and Balance period choices may be invoked from anywhere and explicitly enter Balance.

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
