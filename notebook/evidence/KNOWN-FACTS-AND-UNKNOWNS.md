---
id: EVIDENCE-001
kind: evidence
state: active
created: 2026-08-01
updated: 2026-08-01
authority: working
summary: Classified baseline separating current implementation facts, owner-reported product meaning, derived consequences, interpretations, and unresolved questions.
---

# Known facts and unknowns

## Observed in the repository

- Strata is a Rust 2024 application with TUI and CLI interfaces.
- The current package version is 0.7.6.
- Current live persistence uses CSV and JSON files distributed across data and state paths.
- The TUI continuously runs a selected category, including the special `none`/drift category.
- Ordinary active-time reports exclude the drift category.
- The current simulation attempts one grain per elapsed spawn tick.
- The current visual model uses Braille characters, each of which represents several physical dot positions with one foreground color.
- Detached recovery currently stores a checkpoint, exits the process, calculates elapsed time on reopen, and catches simulation up at accelerated cadence.
- Current source, static audit findings, and open issues identify material persistence, recovery, resize, export, timekeeping, and interaction defects.

## Reported by the product owner

- The intended public concept is **idle**, not drift.
- Time is continuous; falling idle dots express that time continues even when no active layer is selected.
- Strata becomes a timer when actively used without ceasing to be a continuous ledger.
- Strata is used for study, habits, projects, work, leisure, and other personal activity—not only freelancing.
- Losing the visual sediment hurts even when precise ledger data remains available.
- All sediment properties matter; the distinction is the level of precision, not whether topology is meaningful.
- Color mixing is intentional because Braille cells cannot show several independent foreground colors and because physical sand mixes.
- The current one-second quantum was an intentional choice; configurable quantum is interesting as a later option.
- Detach is valuable as an extremely low-power mode because elapsed time can be reconstructed without keeping a process running.
- Waiting for replay on reopen is annoying, while losing sand after an unexpected close is also unacceptable.

## Derived consequences

- At the current quantum, one hour of represented time owes exactly 3,600 logical grains.
- If idle sediment remains historical while excluded from active reports, report totals and sediment totals intentionally differ by idle duration.
- A viewport cannot be the sole logical capacity if all elapsed grains must remain accountable.
- A future quantum setting cannot be treated as a display-only scalar if existing formations retain historical material identity.
- Deliberate detach and unexpected termination contain different evidence about intended layer classification.
- The SQLite schema must represent unresolved or inferred intervals explicitly if recovery confirmation is part of the product contract.

## Current interpretations

- Strata's key artistic proposition is that attention gives continuous time a layer rather than creating the time itself.
- The chronological ledger and sediment are the same temporal history viewed at different resolutions.
- Macrostructure may carry chronology while microstructure carries physical emergence, but this is not yet decided.
- `Karma` currently appears to represent a directional personal balance axis rather than moral judgment.
- Clearing sediment is currently driven by practical visibility more than an established artistic lifecycle.

## Assumptions prohibited from implementation

- That unclassified time should stop rather than become idle.
- That sediment can be discarded whenever the ledger remains correct.
- That every use case should be modeled as a client/project/activity hierarchy.
- That crash time should automatically continue the previous layer.
- That all sediment topology must be bit-for-bit immutable.
- That vertical position has no chronological meaning.
- That `Karma` is accepted final terminology.

## Material unknowns

- What exact macro- and micro-level meaning should vertical position carry?
- What topology changes are legitimate physical evolution, and which are administrative corruption?
- Should one grain have only one flat layer, or one primary visual layer plus optional contexts?
- What does the user confirm after a crash, and what default classification is safest?
- How should detached grains be deposited deterministically without replaying every missed frame?
- Should hidden idle remain visible in archived formations, aggregate bands, or an inspection mode?
- What is the minimum useful formation lifecycle?
- Which term best names the directional property and its aggregate: Karma, polarity, valence, charge, or balance?
- How should different temporal quanta coexist across formations?
