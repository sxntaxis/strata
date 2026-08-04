from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label} anchor missing")
    return text.replace(old, new, 1)

# Recovery authority
p = Path('docs/RECOVERY_AUTHORITY.md')
s = p.read_text()
s = replace_once(s, 'Current completed unit: RECONCILIATION-001B2B', 'Current completed unit: RECONCILIATION-001B2C', 'recovery unit')
s = replace_once(s, 'Last reviewed: 2026-08-02', 'Last reviewed: 2026-08-03', 'recovery date')
anchor = '## Bounded sediment recovery\n'
section = '''## Clear-all and provisional-idle reset receipts

**Clear all sand and reset idle timer** is one receipt-governed operation, not a hidden ledger deletion.

- every committed session row, including idle history, is preserved;
- placed and pending canonical sediment become empty;
- an active idle interval is discarded only as provisional state and replaced by a new idle generation at the operation timestamp;
- a non-idle active generation preserves its stable identity, category, description, canonical elapsed duration, and UTC start;
- the receipt binds the operation timestamp, prior and resulting active state, canonical prior elapsed seconds, the complete sorted affected-day set, and an empty canonical `SandState`;
- affected days remain explicit even when the resulting ledger contribution is empty, so stale daily artifacts are deleted rather than becoming undiscoverable.

SQLite applies active-generation replacement when required, empty sediment, every explicit daily-contribution replacement or deletion, and the resulting checkpoint receipt in one immediate transaction. Existing completed history is neither inserted nor deleted. Fault injection at `before-write`, `active`, `sand`, `daily`, `checkpoint`, and `commit` proves complete rollback.

Legacy-file authority publishes the prepared resulting checkpoint and receipt before later effects. Prepared publication failure restores prior tracker, session, and sediment memory. Startup validates the receipt before ordinary detached recovery, restores the checkpoint's canonical grid and exact resulting active interval in memory, republishes empty sediment, reconciles every explicit operational day idempotently, and clears the receipt only after convergence. Repeated replay cannot duplicate elapsed time or restore pre-clear sediment.

Receipt identity includes canonical elapsed and the affected-day list. Changing either invalidates replay identity. A receipt whose sand payload is non-empty, whose active classification changes, whose elapsed value diverges from its UTC interval beyond the accepted live-clock tolerance, or whose days are malformed, duplicated, or unsorted fails closed.

'''
if anchor not in s:
    raise SystemExit('bounded recovery anchor missing')
s = s.replace(anchor, section + anchor, 1)
start = s.index('## Remaining legacy transitions\n')
end = s.index('## Initial active start\n', start)
s = s[:start] + '''## Remaining issue #10 recovery work

Legacy switch, normal finish, and clear-all/provisional-idle reset now have certified receipt protocols. Issue #10 remains open for the initial active-start/checkpoint window, exact transition-edge sediment attribution beyond the clear-all contract, and user-visible recovery cutoff/reconstruction semantics.

''' + s[end:]
s = replace_once(
    s,
    'RECONCILIATION-001B1, RECONCILIATION-001B2A, and RECONCILIATION-001B2B pass:',
    'RECONCILIATION-001B1, RECONCILIATION-001B2A, RECONCILIATION-001B2B, and RECONCILIATION-001B2C pass:',
    'recovery certification units',
)
s = replace_once(s, '- 205 library tests;', '- 215 library tests;', 'recovery test count')
s = replace_once(
    s,
    'Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, receipt retention after publication failure, multi-day finish reconciliation, archived-authority reload, and schema-2 emergency export custody.',
    'Focused proofs cover transactional SQLite checkpoint retirement, protected recovery evidence, startup identity quarantine, immediate semantic-edge refresh, prepared legacy switch and finish rollback, exact/idempotent session reconciliation, strict receipt payload validation, subsecond whole-second boundaries, all persisted switch and finish kill points, clear-all receipt identity over canonical elapsed and affected days, exact active-state staging before legacy daily reconstruction, cross-day idle authority, non-idle identity preservation, stale now-empty daily deletion, all six SQLite clear-all transaction kill points, archived-authority reload, and schema-2 emergency export custody.',
    'recovery focused proofs',
)
s = replace_once(s, '- a stable legacy clear-all/reset receipt with kill-point replay certification;\n', '', 'remove resolved recovery boundary')
p.write_text(s)

# Architecture
p = Path('docs/ARCHITECTURE.md')
s = p.read_text()
s = replace_once(s, 'Last reviewed: 2026-08-02', 'Last reviewed: 2026-08-03', 'architecture date')
s = replace_once(
    s,
    'legacy switch/finish receipt publication and replay,',
    'legacy switch/finish/clear-all receipt publication and replay,',
    'architecture app responsibility',
)
s = replace_once(
    s,
    'Legacy clear-all/reset remains outside the certified receipt boundary.\n',
    '''Clear-all/provisional-idle reset uses a third certified receipt boundary. It preserves all committed history, binds canonical prior elapsed and every affected day, restores exact active and grid state before legacy replay derives daily contributions, and applies active/sand/daily/checkpoint effects atomically in SQLite. Six transaction kill points and deterministic legacy replay are certified.\n''',
    'architecture clear-all boundary',
)
old_frontier = '''Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, active/checkpoint generation coherence, and legacy switch/finish replay are complete. The next priorities are:

1. implement RECONCILIATION-001B2C: legacy clear-all/reset receipt custody, then initial active-start evidence, exact transition-edge sediment reconciliation, and visible recovery cutoff semantics for issue #10;
2. design the explicit merge/reassignment and permanent-deletion remainder of issue #13;
3. later domain/UI distinction work under issue #22;
4. later profile authority, including complete isolation and deliberate switching under issue #15.
'''
new_frontier = '''Persistence, temporal, domain, report, sediment, interaction, cross-authority category integrity, active/checkpoint generation coherence, and legacy switch/finish/clear-all replay are complete. The next priorities are:

1. complete issue #10 through initial active-start/checkpoint coherence, exact remaining transition-edge sediment attribution, and visible deterministic recovery cutoff/reconstruction semantics;
2. design the explicit merge/reassignment and permanent-deletion remainder of issue #13;
3. later domain/UI distinction work under issue #22;
4. later profile authority, including complete isolation and deliberate switching under issue #15.
'''
s = replace_once(s, old_frontier, new_frontier, 'architecture frontier')
s = replace_once(
    s,
    '- A switch or finish receipt is not authority for clear-all/reset.\n',
    '- A receipt for one transition kind is not authority for another transition kind.\n',
    'architecture non-authority receipt',
)
p.write_text(s)

# Decisions
p = Path('docs/DECISIONS.md')
s = p.read_text()
s = replace_once(s, 'Last reviewed: 2026-08-02', 'Last reviewed: 2026-08-03', 'decisions date')
s = replace_once(
    s,
    '| STRATA-D047 | Persistence recovery must preserve active and archived category meaning in reload, flush, sediment validation, and emergency export; recovery artifacts identify archival state explicitly. | implemented and certified |',
    '| STRATA-D047 | Persistence recovery must preserve active and archived category meaning in reload, flush, sediment validation, and emergency export; recovery artifacts identify archival state explicitly. | implemented and certified |\n| STRATA-D048 | Clear-all is a receipt-governed sediment operation plus provisional-idle reset, never committed-ledger deletion. SQLite publishes active, empty sediment, affected daily contributions, and checkpoint atomically; legacy replay restores exact active/grid state and clears evidence only after convergence. | implemented and certified |',
    'decision D048',
)
s = replace_once(s, '- a stable legacy clear-all/reset receipt across separate file publications;\n', '', 'remove resolved decision')
p.write_text(s)

# Notebook NOW
p = Path('notebook/NOW.md')
s = p.read_text()
s = replace_once(s, 'updated: 2026-08-02', 'updated: 2026-08-03', 'now date')
s = replace_once(
    s,
    'summary: Legacy switch and normal finish now use prepared receipts with idempotent kill-point replay; recovery preserves archived meaning. Issue #10 remains open for clear-all/reset, initial start, sediment-edge, and cutoff semantics.',
    'summary: Legacy switch, finish, and clear-all now use deterministic prepared receipts with idempotent replay; committed history survives clear-all and SQLite publishes the operation atomically. Issue #10 remains open for initial start, remaining sediment-edge, and cutoff semantics.',
    'now summary',
)
s = replace_once(
    s,
    'next: Implement RECONCILIATION-001B2C for clear-all/reset custody, then close the remaining initial-start and recovery-presentation gaps.',
    'next: Complete issue #10 through initial active-start/checkpoint coherence, exact remaining transition-edge sediment attribution, and visible recovery cutoff/reconstruction semantics.',
    'now next header',
)
s = replace_once(s, 'legacy switch/finish-replay units are complete.', 'legacy switch/finish/clear-all-replay units are complete.', 'now phase')
s = replace_once(
    s,
    '- strict legacy session identity and temporal payload validation.\n',
    '''- strict legacy session identity and temporal payload validation;
- receipt-governed clear-all that preserves committed history and resets only provisional idle;
- one SQLite clear-all transaction for active, empty sediment, explicit affected days, and resulting checkpoint;
- deterministic legacy clear-all replay that restores exact canonical elapsed and grid state before daily reconstruction;
- six-point SQLite rollback certification and cross-day stale-artifact deletion proofs.
''',
    'now clear-all bullets',
)
s = replace_once(
    s,
    '- **RECONCILIATION-001B2B** — partial issue #10: prepared legacy finish receipts, multi-authority replay, and archived recovery custody.\n',
    '- **RECONCILIATION-001B2B** — partial issue #10: prepared legacy finish receipts, multi-authority replay, and archived recovery custody.\n- **RECONCILIATION-001B2C** — partial issue #10: non-destructive receipt-governed clear-all/provisional-idle reset with atomic SQLite publication and deterministic legacy replay.\n',
    'now completed B2C',
)
s = replace_once(
    s,
    '1. Implement RECONCILIATION-001B2C: clear-all/reset receipt custody; then address initial active-start/checkpoint coherence, transition-edge sediment reconciliation, and user-visible recovery cutoff semantics.',
    '1. Complete issue #10 through initial active-start/checkpoint coherence, exact remaining transition-edge sediment reconciliation, and user-visible recovery cutoff semantics.',
    'now active sequence',
)
s = replace_once(s, '- Legacy clear-all/reset still crosses session, sediment, daily-contribution, and checkpoint authorities without a certified receipt protocol.\n', '', 'remove now risk')
start = s.index('## Next\n')
s = s[:start] + '''## Next

Complete the remaining issue #10 units. First reconcile initial active-session creation with first checkpoint evidence. Then certify exact remaining sediment attribution at transition edges and expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface. After issue #10 closes, return to the category merge/reassignment and permanent-deletion transaction required by issue #13.
'''
p.write_text(s)

# Work record
p = Path('notebook/work/RECONCILIATION-001B2C.md')
s = p.read_text()
s = replace_once(s, 'state: active', 'state: accepted', 'work state')
s = replace_once(s, 'authority: working', 'authority: accepted', 'work authority')
s = replace_once(s, 'updated: 2026-08-02', 'updated: 2026-08-03', 'work date')
s = replace_once(
    s,
    '- whether idle reset occurred;\n- explicit affected operational days;',
    '- whether idle reset occurred;\n- canonical prior elapsed seconds;\n- explicit affected operational days;',
    'work receipt elapsed',
)
s += '''

## Implemented result

- committed idle and non-idle session history is never deleted by clear-all;
- idle clears start one new provisional idle generation without a completed row;
- non-idle active stable identity, description, canonical elapsed, and UTC start survive;
- SQLite owns one immediate transaction across active state, empty canonical sediment, every explicit daily replacement/deletion, and resulting checkpoint receipt;
- legacy prepared publication rolls memory back before receipt durability and replays exact active/grid state before deriving daily contributions;
- operation identity binds canonical elapsed and the complete affected-day set;
- non-empty clear-all sediment payloads and ambiguous receipt boundaries fail closed;
- every SQLite kill point (`before-write`, `active`, `sand`, `daily`, `checkpoint`, `commit`) rolls all authorities back;
- cross-day idle intervals name every touched operational day, while non-idle clear names only the operation day;
- stale now-empty daily artifacts are deleted explicitly.

## Certification

- formatting: pass;
- strict Clippy, all targets/features, warnings denied: pass;
- 215 library tests: pass;
- 9 CLI lifecycle process tests: pass;
- 6 configuration-authority tests: pass;
- 1 report-help regression test: pass;
- 12 SQLite/TUI process tests: pass;
- 2 temporal-authority tests: pass;
- 3 terminal-lifecycle PTY process tests: pass;
- temporary transformation and audit machinery: absent from the permanent tree.

The unit is accepted as a partial completion of issue #10. It does not claim initial active-start/checkpoint atomicity, complete transition-edge sediment attribution, or final user-visible recovery cutoff/reconstruction semantics.
'''
p.write_text(s)

# Issue reconciliation
p = Path('notebook/work/ISSUE-RECONCILIATION-001.md')
s = p.read_text()
s = replace_once(s, 'updated: 2026-08-02', 'updated: 2026-08-03', 'issue reconciliation date')
old_row = '| #10 | Partially completed by RECONCILIATION-001B1, B2A, and B2B: SQLite active/checkpoint generations are transactional; legacy switch and normal finish publish prepared receipts and replay every certified multi-file kill point idempotently; finish converges session, catalog, sediment, and every affected daily contribution; recovery reload/flush/export preserve archived meaning. Remaining scope is clear-all/reset receipt custody, initial active-start/checkpoint coherence, exact transition-edge sediment reconciliation, and explicit recovery-cutoff/uncertainty presentation. | RECONCILIATION-001B2C |'
new_row = '| #10 | Partially completed by RECONCILIATION-001B1, B2A, B2B, and B2C: SQLite active/checkpoint generations are transactional; legacy switch, finish, and clear-all use deterministic prepared receipts with idempotent replay; clear-all preserves all committed history, resets only provisional idle, binds canonical elapsed and affected days, and publishes active/sand/daily/checkpoint effects atomically in SQLite. Remaining scope is initial active-start/checkpoint coherence, exact remaining transition-edge sediment reconciliation, and explicit recovery-cutoff/uncertainty presentation. | next bounded RECONCILIATION-001B unit |'
s = replace_once(s, old_row, new_row, 'issue #10 row')
start = s.index('## Immediate action\n')
s = s[:start] + '''## Immediate action

Continue issue #10 after accepted RECONCILIATION-001B2C:

1. reconcile initial active-session creation with first checkpoint evidence;
2. certify exact remaining sediment classification at active transition boundaries;
3. expose checkpoint capture, recovery target, reconstructed duration, deterministic cutoff, and uncertainty in the recovery interface;
4. close issue #10 only when repeated restart and crash-during-recovery evidence satisfies its full acceptance criteria.

After issue #10 reaches evidence-based closure, return to the merge/reassignment transaction required to complete issue #13.
'''
p.write_text(s)
