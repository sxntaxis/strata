from pathlib import Path

path = Path("src/app.rs")
content = path.read_text()

old = '''fn settle_transition_sediment_segment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    elapsed: Duration,
    spawn_accumulator: Duration,
    physics_accumulator: Duration,
    spawn_period: Duration,
    physics_period: Duration,
) -> Result<TransitionSedimentSettlement, String> {
    let recovered = recover_detached_sediment(
        base_state,
        valid_category_ids,
        active_category_id,
        RecoveryTiming {
            elapsed,
            spawn_accumulator,
            physics_accumulator,
            spawn_period,
            physics_period,
        },
    )?;
'''
new = '''fn settle_transition_sediment_segment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    timing: RecoveryTiming,
) -> Result<TransitionSedimentSettlement, String> {
    let recovered = recover_detached_sediment(
        base_state,
        valid_category_ids,
        active_category_id,
        timing,
    )?;
'''
if old not in content:
    raise SystemExit("transition settlement helper signature marker missing")
content = content.replace(old, new, 1)

old = '''        let settlement = settle_transition_sediment_segment(
            &self.sand_engine.snapshot_state(),
            &valid_category_ids,
            self.time_tracker.active_category_id(),
            elapsed,
            self.simulation.spawn_accumulator,
            self.simulation.physics_accumulator,
            Duration::from_millis(TIME_SETTINGS.tick_ms),
            Duration::from_millis(TIME_SETTINGS.physics_ms),
        )?;
'''
new = '''        let settlement = settle_transition_sediment_segment(
            &self.sand_engine.snapshot_state(),
            &valid_category_ids,
            self.time_tracker.active_category_id(),
            RecoveryTiming {
                elapsed,
                spawn_accumulator: self.simulation.spawn_accumulator,
                physics_accumulator: self.simulation.physics_accumulator,
                spawn_period: Duration::from_millis(TIME_SETTINGS.tick_ms),
                physics_period: Duration::from_millis(TIME_SETTINGS.physics_ms),
            },
        )?;
'''
if old not in content:
    raise SystemExit("application transition settlement call marker missing")
content = content.replace(old, new, 1)

replacements = [
('''        let outgoing = settle_transition_sediment_segment(
            &empty_state(),
            &categories(),
            CategoryId::new(1),
            Duration::from_millis(100),
            Duration::from_millis(900),
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
''', '''        let outgoing = settle_transition_sediment_segment(
            &empty_state(),
            &categories(),
            CategoryId::new(1),
            RecoveryTiming {
                elapsed: Duration::from_millis(100),
                spawn_accumulator: Duration::from_millis(900),
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
'''),
('''        let resulting = settle_transition_sediment_segment(
            &outgoing.state,
            &categories(),
            CategoryId::new(2),
            Duration::from_secs(1),
            outgoing.spawn_remainder,
            outgoing.physics_remainder,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
''', '''        let resulting = settle_transition_sediment_segment(
            &outgoing.state,
            &categories(),
            CategoryId::new(2),
            RecoveryTiming {
                elapsed: Duration::from_secs(1),
                spawn_accumulator: outgoing.spawn_remainder,
                physics_accumulator: outgoing.physics_remainder,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
'''),
('''        let settled = settle_transition_sediment_segment(
            &empty_state(),
            &categories(),
            CategoryId::new(1),
            Duration::from_millis(100),
            Duration::from_millis(900),
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
''', '''        let settled = settle_transition_sediment_segment(
            &empty_state(),
            &categories(),
            CategoryId::new(1),
            RecoveryTiming {
                elapsed: Duration::from_millis(100),
                spawn_accumulator: Duration::from_millis(900),
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
'''),
('''        let before_next_tick = settle_transition_sediment_segment(
            &cleared,
            &categories(),
            CategoryId::new(2),
            Duration::from_millis(999),
            settled.spawn_remainder,
            settled.physics_remainder,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
''', '''        let before_next_tick = settle_transition_sediment_segment(
            &cleared,
            &categories(),
            CategoryId::new(2),
            RecoveryTiming {
                elapsed: Duration::from_millis(999),
                spawn_accumulator: settled.spawn_remainder,
                physics_accumulator: settled.physics_remainder,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
'''),
('''        let settled = settle_transition_sediment_segment(
            &state,
            &categories(),
            CategoryId::new(2),
            Duration::from_secs(1_000_000_000),
            Duration::ZERO,
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_millis(50),
        )
''', '''        let settled = settle_transition_sediment_segment(
            &state,
            &categories(),
            CategoryId::new(2),
            RecoveryTiming {
                elapsed: Duration::from_secs(1_000_000_000),
                spawn_accumulator: Duration::ZERO,
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },
        )
'''),
]
for old_call, new_call in replacements:
    if old_call not in content:
        raise SystemExit(f"test settlement call marker missing: {old_call.splitlines()[0]}")
    content = content.replace(old_call, new_call, 1)

content = content.replace(
    "    use super::settle_transition_sediment_segment;",
    "    use super::{RecoveryTiming, settle_transition_sediment_segment};",
    1,
)
path.write_text(content)
