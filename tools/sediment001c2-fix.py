from pathlib import Path


apply_path = Path("tools/sediment001c2-apply.py")
source = apply_path.read_text()
section = source.index("# Checkpoint schema and app lifecycle.")
first = source.index("replace_once(", section)
first_end = source.index("\n)\n", first) + 3
second = source.index("replace_once(", first_end)
second_end = source.index("\n)\n", second) + 3
apply_path.write_text(source[:second] + source[second_end:])

exec(compile(apply_path.read_text(), str(apply_path), "exec"), {"__name__": "__main__"})

module = Path("src/sand/mod.rs")
source = module.read_text()
old = (
    "mod engine;\nmod recovery;\n\n"
    "pub use engine::{PendingGrainRun, SandEngine, SandState, SandStateGrain};\n"
    "pub(crate) use recovery::{RecoveredSediment, advance_periodic, recover_detached_sediment};"
)
new = (
    "mod engine;\nmod recovery;\n\n"
    "#[allow(unused_imports)]\n"
    "pub use engine::{PendingGrainRun, SandEngine, SandState, SandStateGrain};\n"
    "pub(crate) use recovery::{RecoveryTiming, recover_detached_sediment};"
)
if old not in source:
    raise SystemExit("generated sand module exports were not found")
module.write_text(source.replace(old, new, 1))

recovery = Path("src/sand/recovery.rs")
source = recovery.read_text()
marker = """#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveredSediment {
    pub state: SandState,
    pub spawn_remainder: Duration,
    pub physics_remainder: Duration,
    pub added_grains: usize,
    pub skipped_physics_events: usize,
}
"""
addition = marker + """
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveryTiming {
    pub elapsed: Duration,
    pub spawn_accumulator: Duration,
    pub physics_accumulator: Duration,
    pub spawn_period: Duration,
    pub physics_period: Duration,
}
"""
if marker not in source:
    raise SystemExit("recovered sediment declaration was not found")
source = source.replace(marker, addition, 1)
old_signature = """pub(crate) fn recover_detached_sediment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    elapsed: Duration,
    spawn_accumulator: Duration,
    physics_accumulator: Duration,
    spawn_period: Duration,
    physics_period: Duration,
) -> Result<RecoveredSediment, String> {"""
new_signature = """pub(crate) fn recover_detached_sediment(
    base_state: &SandState,
    valid_category_ids: &HashSet<CategoryId>,
    active_category_id: CategoryId,
    timing: RecoveryTiming,
) -> Result<RecoveredSediment, String> {"""
if old_signature not in source:
    raise SystemExit("recovery function signature was not found")
source = source.replace(old_signature, new_signature, 1)
old_advance = (
    "    let spawn = advance_periodic(spawn_accumulator, elapsed, spawn_period)?;\n"
    "    let physics = advance_periodic(physics_accumulator, elapsed, physics_period)?;"
)
new_advance = """    let spawn = advance_periodic(
        timing.spawn_accumulator,
        timing.elapsed,
        timing.spawn_period,
    )?;
    let physics = advance_periodic(
        timing.physics_accumulator,
        timing.elapsed,
        timing.physics_period,
    )?;"""
if old_advance not in source:
    raise SystemExit("recovery periodic calls were not found")
source = source.replace(old_advance, new_advance, 1)
source = source.replace(
    "use super::{PeriodicAdvance, advance_periodic, recover_detached_sediment};",
    "use super::{PeriodicAdvance, RecoveryTiming, advance_periodic, recover_detached_sediment};",
    1,
)
old_calls = [
    """            Duration::from_millis(2_500),
            Duration::from_millis(750),
            Duration::from_millis(20),
            Duration::from_secs(1),
            Duration::from_millis(50),""",
    """            Duration::from_secs(1_000_000_000),
            Duration::ZERO,
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_millis(50),""",
    """                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                Duration::from_secs(1),
                Duration::from_millis(50),""",
]
new_calls = [
    """            RecoveryTiming {
                elapsed: Duration::from_millis(2_500),
                spawn_accumulator: Duration::from_millis(750),
                physics_accumulator: Duration::from_millis(20),
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },""",
    """            RecoveryTiming {
                elapsed: Duration::from_secs(1_000_000_000),
                spawn_accumulator: Duration::ZERO,
                physics_accumulator: Duration::ZERO,
                spawn_period: Duration::from_secs(1),
                physics_period: Duration::from_millis(50),
            },""",
    """                RecoveryTiming {
                    elapsed: Duration::ZERO,
                    spawn_accumulator: Duration::ZERO,
                    physics_accumulator: Duration::ZERO,
                    spawn_period: Duration::from_secs(1),
                    physics_period: Duration::from_millis(50),
                },""",
]
for old_call, new_call in zip(old_calls, new_calls):
    if old_call not in source:
        raise SystemExit("a recovery test call was not found")
    source = source.replace(old_call, new_call, 1)
recovery.write_text(source)

app = Path("src/app.rs")
source = app.read_text()
old_import = "sand::{SandEngine, SandState, SandStateGrain, recover_detached_sediment},"
new_import = (
    "sand::{RecoveryTiming, SandEngine, SandState, SandStateGrain, "
    "recover_detached_sediment},"
)
if old_import not in source:
    raise SystemExit("application recovery import was not found")
source = source.replace(old_import, new_import, 1)
old_call = """            elapsed,
            Duration::from_nanos(checkpoint.spawn_accumulator_nanos),
            Duration::from_nanos(checkpoint.physics_accumulator_nanos),
            Duration::from_millis(TIME_SETTINGS.tick_ms),
            Duration::from_millis(TIME_SETTINGS.physics_ms),"""
new_call = """            RecoveryTiming {
                elapsed,
                spawn_accumulator: Duration::from_nanos(checkpoint.spawn_accumulator_nanos),
                physics_accumulator: Duration::from_nanos(checkpoint.physics_accumulator_nanos),
                spawn_period: Duration::from_millis(TIME_SETTINGS.tick_ms),
                physics_period: Duration::from_millis(TIME_SETTINGS.physics_ms),
            },"""
if old_call not in source:
    raise SystemExit("application recovery call was not found")
app.write_text(source.replace(old_call, new_call, 1))

persistence = Path("src/app/persistence_recovery.rs")
source = persistence.read_text()
start = source.index("    pub(super) fn begin_manual_persistence_failure(")
end = source.index("    pub(super) fn has_persistence_recovery", start)
persistence.write_text(source[:start] + source[end:])

Path(__file__).unlink(missing_ok=True)
