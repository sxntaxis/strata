from pathlib import Path

# Recovery authority
path = Path('docs/RECOVERY_AUTHORITY.md')
text = path.read_text()
text = text.replace('Current completed unit: RECONCILIATION-001B2A', 'Current completed unit: RECONCILIATION-001B2B', 1)
insert = '''## Legacy finish transition receipts

Normal legacy finish is also a prepared multi-file transition.

Before mutating the active session, Strata publishes the prior-generation checkpoint with a deterministic finish receipt binding:

- prior category, description, and UTC start;
- canonical finish UTC;
- optional completed session identity and full temporal payload;
- the absence of a resulting active generation.

If prepared receipt publication fails, the active session remains unchanged. Once durable, finish proceeds through completed history, cleared category-description state, canonical sediment, and every affected daily contribution. The checkpoint is removed only after all of those authorities converge.

Startup recognizes a finish receipt before ordinary active recovery. It validates the prior checkpoint generation and whole-second boundaries, publishes missing effects idempotently, exact-matches an existing completed row, rejects conflict, reconciles every affected operational day, and deletes the receipt. A receipt-marked finished generation is never resumed as active.

Kill-point tests certify receipt-only, receipt-plus-session, receipt-plus-session-plus-catalog, and receipt-plus-session-plus-catalog-plus-sand states. A later publication failure retains the receipt. Retry also reconciles all affected days before receipt deletion, including multi-day sessions.

Normal legacy finish now persists the cleared active description. Legacy recovery flush and reload preserve active and archived category catalogs, archived session references, and archived sediment identities. Emergency recovery JSON schema 2 includes every category with an explicit `archived` flag.

'''
anchor = '## Bounded sediment recovery\n'
if anchor not in text:
    raise SystemExit('recovery insertion anchor missing')
text = text.replace(anchor, insert + anchor, 1)
start = text.index('## Remaining legacy transitions\n')
end = text.index('\n## Initial active start\n', start)
text = text[:start] + '''## Remaining legacy transitions

Legacy switch and normal finish now have certified receipt protocols. Clear-all/reset remains outside that boundary because it also mutates idle-session history and sediment state.

Until a dedicated reset unit completes it:

- issue #10 remains open;
- reset multi-file atomicity is not claimed;
- recovery must retain evidence and fail visibly rather than inventing cleared history;
- switch or finish certification cannot be generalized to reset.
''' + text[end:]
text = text.replace('RECONCILIATION-001B1 and RECONCILIATION-001B2A pass:', 'RECONCILIATION-001B1, RECONCILIATION-001B2A, and RECONCILIATION-001B2B pass:', 1)
text = text.replace('- 199 library tests;', '- 205 library tests;', 1)
text = text.replace('all persisted switch kill points, and receipt retention after catalog-publication failure.', 'all persisted switch and finish kill points, receipt retention after publication failure, multi-day finish reconciliation, archived-authority reload, and schema-2 emergency export custody.', 1)
text = text.replace('- stable legacy reset and finish receipts with kill-point replay certification;', '- a stable legacy clear-all/reset receipt with kill-point replay certification;', 1)
path.write_text(text)

# Decisions
path = Path('docs/DECISIONS.md')
text = path.read_text()
needle = '| STRATA-D045 | A legacy switch is a receipt-governed multi-file transition: publish the resulting checkpoint and deterministic receipt first, replay session/catalog effects idempotently, and clear the receipt only after every authority converges. Whole-second ledger semantics—not exact subsecond wall-start equality—own the completed row. | implemented and certified |\n'
addition = needle + '| STRATA-D046 | A normal legacy finish is a receipt-governed terminal transition: publish prior-generation evidence before active mutation, never resume a receipt-marked finished generation, and retire the receipt only after session, catalog, sediment, and every affected daily contribution converge. | implemented and certified |\n| STRATA-D047 | Persistence recovery must preserve active and archived category meaning in reload, flush, sediment validation, and emergency export; recovery artifacts identify archival state explicitly. | implemented and certified |\n'
if needle not in text:
    raise SystemExit('decision D045 anchor missing')
text = text.replace(needle, addition, 1)
text = text.replace('- stable legacy reset and finish receipts across separate file publications;', '- a stable legacy clear-all/reset receipt across separate file publications;', 1)
path.write_text(text)

# Architecture
path = Path('docs/ARCHITECTURE.md')
text = path.read_text()
text = text.replace('legacy switch receipt publication/replay', 'legacy switch/finish receipt publication and replay', 1)
text = text.replace('Legacy switch transitions use a certified multi-file receipt protocol:', 'Legacy switch transitions use a certified multi-file receipt protocol:', 1)
needle = 'Legacy reset and finish remain outside this certified receipt boundary.\n'
replacement = '''Normal legacy finish uses a second certified receipt protocol. It publishes prior-generation evidence before active mutation, then converges completed history, cleared category metadata, canonical sediment, and every affected daily contribution before deleting the checkpoint. Startup consumes the finish receipt without resuming the finished generation. Four persisted kill points and later-publication failure custody are certified.

Legacy recovery flush/reload validate both active and archived catalogs, retain archived sediment identity, and emergency recovery schema 2 exports explicit archival state.

Legacy clear-all/reset remains outside the certified receipt boundary.
'''
if needle not in text:
    raise SystemExit('architecture legacy boundary anchor missing')
text = text.replace(needle, replacement, 1)
text = text.replace('Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, active/checkpoint generation coherence, and legacy switch replay are complete.', 'Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, active/checkpoint generation coherence, and legacy switch/finish replay are complete.', 1)
text = text.replace('1. implement RECONCILIATION-001B2B: stable legacy reset/finish receipts, initial active-start evidence, exact transition-edge sediment reconciliation, and visible recovery cutoff semantics for issue #10;', '1. implement RECONCILIATION-001B2C: legacy clear-all/reset receipt custody, then initial active-start evidence, exact transition-edge sediment reconciliation, and visible recovery cutoff semantics for issue #10;', 1)
text = text.replace('- A switch receipt is not authority for reset or finish.', '- A switch or finish receipt is not authority for clear-all/reset.', 1)
path.write_text(text)

# Notebook NOW
path = Path('notebook/NOW.md')
text = path.read_text()
text = text.replace('summary: Legacy switch transitions now use prepared schema-3 receipts and idempotent kill-point replay; issue #10 remains open for reset/finish, initial start, sediment-edge, and cutoff semantics.', 'summary: Legacy switch and normal finish now use prepared receipts with idempotent kill-point replay; recovery preserves archived meaning. Issue #10 remains open for clear-all/reset, initial start, sediment-edge, and cutoff semantics.', 1)
text = text.replace('next: Implement RECONCILIATION-001B2B for remaining active-transition and recovery-presentation gaps.', 'next: Implement RECONCILIATION-001B2C for clear-all/reset custody, then close the remaining initial-start and recovery-presentation gaps.', 1)
text = text.replace('category-integrity, active/checkpoint generation-coherence, and legacy switch-replay units are complete.', 'category-integrity, active/checkpoint generation-coherence, and legacy switch/finish-replay units are complete.', 1)
needle = '- prepared legacy switch receipts published before session/category effects;\n- idempotent switch replay from every durable publication point;\n'
replacement = needle + '- prepared legacy finish receipts published before active mutation;\n- idempotent finish replay across session, catalog, sediment, and daily-contribution effects;\n- archived-safe recovery reload/flush and schema-2 emergency exports;\n'
if needle not in text:
    raise SystemExit('NOW capability anchor missing')
text = text.replace(needle, replacement, 1)
text = text.replace('- **RECONCILIATION-001B2A** — partial issue #10: prepared legacy switch receipts and idempotent kill-point replay.', '- **RECONCILIATION-001B2A** — partial issue #10: prepared legacy switch receipts and idempotent kill-point replay.\n- **RECONCILIATION-001B2B** — partial issue #10: prepared legacy finish receipts, multi-authority replay, and archived recovery custody.', 1)
text = text.replace('1. Implement RECONCILIATION-001B2B: legacy reset/finish receipts, initial active-start/checkpoint coherence, transition-edge sediment reconciliation, and user-visible recovery cutoff semantics.', '1. Implement RECONCILIATION-001B2C: clear-all/reset receipt custody; then address initial active-start/checkpoint coherence, transition-edge sediment reconciliation, and user-visible recovery cutoff semantics.', 1)
text = text.replace('- Legacy reset and finish still cross separate authority files without the certified switch receipt protocol.', '- Legacy clear-all/reset still crosses session, sediment, daily-contribution, and checkpoint authorities without a certified receipt protocol.', 1)
text = text.replace('Implement **RECONCILIATION-001B2B**. Extend receipt custody only where the operation semantics justify it, certify every durable reset/finish publication point, reconcile initial active-start evidence, bind sediment contribution to the same transition boundary, and expose checkpoint capture, recovery target, reconstructed duration, and deterministic cutoff policy before issue #10 can close.', 'Implement **RECONCILIATION-001B2C** for clear-all/reset. Bind deleted idle history, cleared sediment, replacement active state, daily contributions, and checkpoint evidence to one replayable operation. After that, reconcile initial active-start evidence and expose checkpoint capture, recovery target, reconstructed duration, and deterministic cutoff policy before issue #10 can close.', 1)
path.write_text(text)

# Issue reconciliation
path = Path('notebook/work/ISSUE-RECONCILIATION-001.md')
text = path.read_text()
old = '| #10 | Partially completed by RECONCILIATION-001B1 and B2A: SQLite active/checkpoint generations are transactional; incompatible evidence blocks transitions; startup validates checkpoint identity; semantic-edge refresh is immediate; legacy switches now publish a prepared schema-3 receipt and replay session/catalog effects idempotently from every durable kill point. Remaining scope is legacy reset/finish receipts, initial active-start/checkpoint coherence, exact transition-edge sediment reconciliation, and explicit recovery-cutoff/uncertainty presentation. | RECONCILIATION-001B2B |'
new = '| #10 | Partially completed by RECONCILIATION-001B1, B2A, and B2B: SQLite active/checkpoint generations are transactional; legacy switch and normal finish publish prepared receipts and replay every certified multi-file kill point idempotently; finish converges session, catalog, sediment, and every affected daily contribution; recovery reload/flush/export preserve archived meaning. Remaining scope is clear-all/reset receipt custody, initial active-start/checkpoint coherence, exact transition-edge sediment reconciliation, and explicit recovery-cutoff/uncertainty presentation. | RECONCILIATION-001B2C |'
if old not in text:
    raise SystemExit('issue #10 row anchor missing')
text = text.replace(old, new, 1)
start = text.index('## Immediate action\n')
text = text[:start] + '''## Immediate action

Implement RECONCILIATION-001B2C for issue #10:

1. define a stable clear-all/reset receipt that binds deleted idle history, cleared sediment, replacement active state, and affected daily contributions;
2. replay every separate publication idempotently and certify every durable kill point;
3. retain receipt evidence after any incomplete publication;
4. then reconcile the initial active-start/checkpoint window;
5. bind exact transition-edge sediment classification to the same authority;
6. expose checkpoint capture, recovery target, reconstructed duration, and deterministic cutoff policy in the recovery interface.

After issue #10 reaches evidence-based closure, return to the merge/reassignment transaction required to complete issue #13.
'''
path.write_text(text)

Path('tools/reconciliation001b2b-promote.py').unlink()
Path('.github/workflows/reconciliation001b2b-promote.yml').unlink()
