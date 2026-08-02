from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected one match in {path}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


engine_path = Path("src/sand/engine.rs")
engine = engine_path.read_text()
engine = engine.replace(
    '''#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandState {
''',
    '''#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingGrainRun {
    pub category_id: u64,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandState {
''',
    1,
)
engine = engine.replace(
    '''    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_grains: Vec<u64>,
}

impl SandState {
    pub const VERSION: u8 = 1;
}
''',
    '''    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_grains: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_runs: Vec<PendingGrainRun>,
}

impl SandState {
    pub const VERSION: u8 = 2;
    pub const LEGACY_VERSION: u8 = 1;
}
''',
    1,
)
engine = engine.replace(
    '''pub struct SandEngine {
''',
    '''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingRun {
    category_id: CategoryId,
    count: usize,
}

pub struct SandEngine {
''',
    1,
)
engine = engine.replace(
    "    pending_grains: VecDeque<CategoryId>,\n",
    "    pending_runs: VecDeque<PendingRun>,\n",
    1,
)
engine = engine.replace(
    "            pending_grains: VecDeque::new(),\n",
    "            pending_runs: VecDeque::new(),\n",
    1,
)
engine_path.write_text(engine)

replace_between(
    "src/sand/engine.rs",
    "    pub fn spawn(&mut self, category_id: CategoryId)",
    "    fn apply_gravity(&mut self)",
    '''    pub fn spawn(&mut self, category_id: CategoryId) {
        self.add_logical_grains(category_id, 1)
            .expect("single-grain logical count must fit usize");
    }

    pub fn add_logical_grains(
        &mut self,
        category_id: CategoryId,
        count: usize,
    ) -> Result<(), String> {
        if count == 0 {
            return Ok(());
        }

        let next_total = self
            .grain_count
            .checked_add(count)
            .ok_or_else(|| "logical sediment count exceeds the supported range".to_string())?;
        Self::append_pending_run(&mut self.pending_runs, category_id, count)?;
        self.grain_count = next_total;
        self.flush_pending_grains();
        Ok(())
    }

    pub fn pending_grain_count(&self) -> usize {
        self.pending_runs.iter().map(|run| run.count).sum()
    }

    pub fn pending_run_count(&self) -> usize {
        self.pending_runs.len()
    }

    pub fn physical_grain_count(&self) -> usize {
        self.grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_some())
            .count()
    }

    fn refresh_logical_grain_count(&mut self) {
        self.grain_count = self
            .physical_grain_count()
            .checked_add(self.pending_grain_count())
            .expect("validated logical sediment count must fit usize");
    }

    fn append_pending_run(
        runs: &mut VecDeque<PendingRun>,
        category_id: CategoryId,
        count: usize,
    ) -> Result<(), String> {
        if count == 0 {
            return Ok(());
        }

        if let Some(last) = runs.back_mut()
            && last.category_id == category_id
        {
            last.count = last.count.checked_add(count).ok_or_else(|| {
                "pending sediment run exceeds the supported range".to_string()
            })?;
            return Ok(());
        }

        runs.push_back(PendingRun { category_id, count });
        Ok(())
    }

    fn flush_pending_grains(&mut self) {
        if self.capacity() == 0 || self.pending_runs.is_empty() {
            return;
        }

        let mut free_columns = self.grid[0]
            .iter()
            .enumerate()
            .filter_map(|(x, cell)| cell.is_none().then_some(x))
            .collect::<Vec<_>>();

        while !free_columns.is_empty() && !self.pending_runs.is_empty() {
            let free_index = self.random_index(free_columns.len());
            let x = free_columns.swap_remove(free_index);
            let category_id = self
                .pending_runs
                .front()
                .expect("pending run exists")
                .category_id;
            self.grid[0][x] = Some(category_id);

            let exhausted = {
                let run = self.pending_runs.front_mut().expect("pending run exists");
                run.count -= 1;
                run.count == 0
            };
            if exhausted {
                self.pending_runs.pop_front();
            }
        }
    }

''',
)

engine = engine_path.read_text()
engine = engine.replace("        self.pending_grains.clear();", "        self.pending_runs.clear();", 1)
engine = engine.replace(
    '''        self.pending_grains
            .retain(|pending| *pending != category_id);
''',
    '''        self.pending_runs
            .retain(|run| run.category_id != category_id);
''',
    1,
)
old_pending_remove = '''        let mut pending_removed = 0usize;
        if removed < count {
            let mut retained = VecDeque::with_capacity(self.pending_grains.len());
            while let Some(category) = self.pending_grains.pop_front() {
                if category == category_id && removed + pending_removed < count {
                    pending_removed += 1;
                } else {
                    retained.push_back(category);
                }
            }
            self.pending_grains = retained;
        }
'''
new_pending_remove = '''        let mut pending_removed = 0usize;
        if removed < count {
            let mut remaining = count - removed;
            for run in &mut self.pending_runs {
                if remaining == 0 {
                    break;
                }
                if run.category_id != category_id {
                    continue;
                }
                let take = run.count.min(remaining);
                run.count -= take;
                remaining -= take;
                pending_removed += take;
            }
            self.pending_runs.retain(|run| run.count > 0);
        }
'''
if engine.count(old_pending_remove) != 1:
    raise SystemExit("pending removal block did not match")
engine = engine.replace(old_pending_remove, new_pending_remove, 1)
engine = engine.replace(
    "        let mut grains = Vec::with_capacity(self.grain_count);",
    "        let mut grains = Vec::with_capacity(self.physical_grain_count());",
    1,
)
engine = engine.replace(
    '''            pending_grains: self
                .pending_grains
                .iter()
                .map(|category_id| category_id.0)
                .collect(),
''',
    '''            pending_grains: Vec::new(),
            pending_runs: self
                .pending_runs
                .iter()
                .map(|run| PendingGrainRun {
                    category_id: run.category_id.0,
                    count: run.count,
                })
                .collect(),
''',
    1,
)
engine_path.write_text(engine)

replace_between(
    "src/sand/engine.rs",
    "    pub fn restore_state(&mut self, state: &SandState",
    "    fn next_random_u64(&mut self)",
    '''    pub fn restore_state(&mut self, state: &SandState, valid_category_ids: &HashSet<CategoryId>) {
        if state.version != SandState::VERSION && state.version != SandState::LEGACY_VERSION {
            return;
        }

        if state.grid_width == 0 || state.grid_height == 0 {
            self.clear();
            return;
        }

        let mut restored = vec![vec![None; state.grid_width]; state.grid_height];
        let none_id = DRIFT_CATEGORY_ID;

        for grain in &state.grains {
            if grain.x >= state.grid_width || grain.y >= state.grid_height {
                continue;
            }

            let category_id = CategoryId::new(grain.category_id);
            let normalized_id = if valid_category_ids.contains(&category_id) {
                category_id
            } else {
                none_id
            };

            restored[grain.y][grain.x] = Some(normalized_id);
        }

        let mut pending_runs = VecDeque::new();
        let append_serialized_run =
            |runs: &mut VecDeque<PendingRun>, category_id: u64, count: usize| {
                let category_id = CategoryId::new(category_id);
                let normalized_id = if valid_category_ids.contains(&category_id) {
                    category_id
                } else {
                    none_id
                };
                Self::append_pending_run(runs, normalized_id, count)
            };

        let pending_result = if state.version == SandState::LEGACY_VERSION {
            state.pending_grains.iter().try_for_each(|category_id| {
                append_serialized_run(&mut pending_runs, *category_id, 1)
            })
        } else {
            state.pending_runs.iter().try_for_each(|run| {
                append_serialized_run(&mut pending_runs, run.category_id, run.count)
            })
        };
        if pending_result.is_err() {
            return;
        }

        let physical_count = restored
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_some())
            .count();
        let pending_count = pending_runs.iter().try_fold(0usize, |total, run| {
            total.checked_add(run.count)
        });
        let Some(logical_count) = pending_count.and_then(|pending| physical_count.checked_add(pending))
        else {
            return;
        };

        self.grid = restored;
        self.grid_width_dots = state.grid_width;
        self.grid_height_dots = state.grid_height;
        self.pending_runs = pending_runs;
        self.grain_count = logical_count;
        self.frame_count = state.frame_count;
        self.sweep_left_to_right = state.sweep_left_to_right;
        self.rng_state = if state.rng_state == 0 {
            default_rng_state()
        } else {
            state.rng_state
        };
    }

''',
)

engine = engine_path.read_text()
engine = engine.replace(
    "        engine.pending_grains.push_back(CategoryId::new(2));",
    "        SandEngine::append_pending_run(&mut engine.pending_runs, CategoryId::new(2), 1)\n            .unwrap();",
    1,
)
engine = engine.replace(
    '''        let state = engine.snapshot_state();
        assert_eq!(state.pending_grains, vec![2]);
''',
    '''        let state = engine.snapshot_state();
        assert!(state.pending_grains.is_empty());
        assert_eq!(
            state.pending_runs,
            vec![super::PendingGrainRun {
                category_id: 2,
                count: 1,
            }]
        );
''',
    1,
)
engine = engine.replace(
    "        assert_eq!(restored.snapshot_state().pending_grains, vec![2]);",
    "        assert_eq!(restored.snapshot_state().pending_runs, state.pending_runs);",
    1,
)
engine_path.write_text(engine)

# Add focused compressed-mass tests before the conservation module closes.
engine = engine_path.read_text()
insert_at = engine.rfind("}\n")
focused = r'''

#[cfg(test)]
mod compressed_mass_tests {
    use std::collections::HashSet;

    use super::{PendingGrainRun, SandEngine, SandState, SandStateGrain};
    use crate::domain::CategoryId;

    #[test]
    fn billion_grains_use_one_pending_run() {
        let mut engine = SandEngine::new(1, 1);
        for cell in &mut engine.grid[0] {
            *cell = Some(CategoryId::new(0));
        }
        engine.grain_count = engine.grid[0].len();

        engine
            .add_logical_grains(CategoryId::new(7), 1_000_000_000)
            .unwrap();

        assert_eq!(engine.pending_run_count(), 1);
        assert_eq!(engine.pending_grain_count(), 1_000_000_000);
        assert_eq!(engine.grain_count, engine.grid[0].len() + 1_000_000_000);
        assert_eq!(engine.snapshot_state().pending_runs[0].count, 1_000_000_000);
    }

    #[test]
    fn adjacent_categories_merge_without_losing_fifo_transitions() {
        let mut engine = SandEngine::new(1, 1);
        for cell in &mut engine.grid[0] {
            *cell = Some(CategoryId::new(0));
        }
        engine.grain_count = engine.grid[0].len();

        engine.add_logical_grains(CategoryId::new(1), 5).unwrap();
        engine.add_logical_grains(CategoryId::new(1), 7).unwrap();
        engine.add_logical_grains(CategoryId::new(2), 3).unwrap();
        engine.add_logical_grains(CategoryId::new(1), 2).unwrap();

        assert_eq!(
            engine.snapshot_state().pending_runs,
            vec![
                PendingGrainRun { category_id: 1, count: 12 },
                PendingGrainRun { category_id: 2, count: 3 },
                PendingGrainRun { category_id: 1, count: 2 },
            ]
        );
    }

    #[test]
    fn legacy_pending_vector_migrates_to_version_two_runs() {
        let legacy = SandState {
            version: SandState::LEGACY_VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: vec![SandStateGrain { x: 0, y: 1, category_id: 1 }],
            frame_count: 4,
            sweep_left_to_right: true,
            rng_state: 9,
            pending_grains: vec![2, 2, 1],
            pending_runs: Vec::new(),
        };
        let valid = HashSet::from([
            CategoryId::new(0),
            CategoryId::new(1),
            CategoryId::new(2),
        ]);
        let mut engine = SandEngine::new(1, 1);
        engine.restore_state(&legacy, &valid);
        let migrated = engine.snapshot_state();

        assert_eq!(migrated.version, SandState::VERSION);
        assert!(migrated.pending_grains.is_empty());
        assert_eq!(
            migrated.pending_runs,
            vec![
                PendingGrainRun { category_id: 2, count: 2 },
                PendingGrainRun { category_id: 1, count: 1 },
            ]
        );
        assert_eq!(engine.grain_count, 4);
    }

    #[test]
    fn category_removal_is_exact_across_compressed_runs() {
        let mut engine = SandEngine::new(1, 1);
        for cell in &mut engine.grid[0] {
            *cell = Some(CategoryId::new(0));
        }
        engine.grain_count = engine.grid[0].len();
        engine.add_logical_grains(CategoryId::new(3), 10).unwrap();
        engine.add_logical_grains(CategoryId::new(4), 2).unwrap();
        engine.add_logical_grains(CategoryId::new(3), 5).unwrap();

        assert_eq!(engine.remove_category_grains(CategoryId::new(3), 12), 12);
        assert_eq!(
            engine.snapshot_state().pending_runs,
            vec![
                PendingGrainRun { category_id: 4, count: 2 },
                PendingGrainRun { category_id: 3, count: 3 },
            ]
        );
    }
}
'''
engine_path.write_text(engine[:insert_at] + focused + engine[insert_at:])

# Add exact periodic arithmetic for the integration unit.
Path("src/sand/recovery.rs").write_text(r'''use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PeriodicAdvance {
    pub due_events: usize,
    pub remainder: Duration,
}

pub(crate) fn advance_periodic(
    accumulator: Duration,
    elapsed: Duration,
    period: Duration,
) -> Result<PeriodicAdvance, String> {
    let period_nanos = period.as_nanos();
    if period_nanos == 0 {
        return Err("periodic recovery interval must be non-zero".to_string());
    }

    let total_nanos = accumulator
        .as_nanos()
        .checked_add(elapsed.as_nanos())
        .ok_or_else(|| "periodic recovery duration overflow".to_string())?;
    let due_events = usize::try_from(total_nanos / period_nanos)
        .map_err(|_| "periodic recovery event count exceeds the supported range".to_string())?;
    let remainder_nanos = total_nanos % period_nanos;
    let remainder_seconds = u64::try_from(remainder_nanos / 1_000_000_000)
        .map_err(|_| "periodic recovery remainder exceeds Duration".to_string())?;
    let remainder_subsec = u32::try_from(remainder_nanos % 1_000_000_000)
        .map_err(|_| "periodic recovery remainder is invalid".to_string())?;

    Ok(PeriodicAdvance {
        due_events,
        remainder: Duration::new(remainder_seconds, remainder_subsec),
    })
}

#[cfg(test)]
mod tests {
    use super::{PeriodicAdvance, advance_periodic};
    use std::time::Duration;

    #[test]
    fn long_gap_is_counted_without_iterative_replay() {
        assert_eq!(
            advance_periodic(
                Duration::ZERO,
                Duration::from_secs(1_000_000_000),
                Duration::from_secs(1),
            )
            .unwrap(),
            PeriodicAdvance {
                due_events: 1_000_000_000,
                remainder: Duration::ZERO,
            }
        );
    }

    #[test]
    fn accumulator_and_remainder_are_exact() {
        assert_eq!(
            advance_periodic(
                Duration::from_millis(750),
                Duration::from_millis(2_500),
                Duration::from_secs(1),
            )
            .unwrap(),
            PeriodicAdvance {
                due_events: 3,
                remainder: Duration::from_millis(250),
            }
        );
    }

    #[test]
    fn zero_period_is_rejected() {
        assert!(advance_periodic(Duration::ZERO, Duration::from_secs(1), Duration::ZERO).is_err());
    }
}
''')
replace_once(
    "src/sand/mod.rs",
    "mod engine;\n\npub use engine::{SandEngine, SandState, SandStateGrain};",
    "mod engine;\nmod recovery;\n\npub use engine::{PendingGrainRun, SandEngine, SandState, SandStateGrain};\npub(crate) use recovery::{PeriodicAdvance, advance_periodic};",
)

# Add the new state field to known non-engine initializers.
for path in [
    Path("src/app.rs"),
    Path("src/app/report_state.rs"),
    Path("src/sqlite/fault_certification.rs"),
    Path("src/sqlite/tui_runtime.rs"),
    Path("src/storage.rs"),
]:
    text = path.read_text()
    lines = text.splitlines(keepends=True)
    out = []
    inside_state = False
    brace_depth = 0
    has_pending_runs = False
    pending_insert_index = None
    for line in lines:
        if "SandState {" in line:
            inside_state = True
            brace_depth = line.count("{") - line.count("}")
            has_pending_runs = False
            pending_insert_index = None
            out.append(line)
            continue
        if inside_state:
            brace_depth += line.count("{") - line.count("}")
            if "pending_runs:" in line:
                has_pending_runs = True
            out.append(line)
            if "pending_grains:" in line:
                pending_insert_index = len(out)
            if brace_depth == 0:
                if pending_insert_index is not None and not has_pending_runs:
                    indent = line[: len(line) - len(line.lstrip())]
                    out.insert(pending_insert_index, f"{indent}pending_runs: Vec::new(),\n")
                inside_state = False
            continue
        out.append(line)
    path.write_text("".join(out))

for temporary in [
    ".github/workflows/sediment001c1-apply.yml",
    "tools/sediment001c1-apply.py",
    "tools/sediment001c1.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
