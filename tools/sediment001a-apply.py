from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected exactly one match in {path}: {old[:100]!r}")
    target.write_text(text.replace(old, new, 1))


engine = Path("src/sand/engine.rs")
text = engine.read_text()

text = text.replace(
    "use std::collections::{HashMap, HashSet};",
    "use std::collections::{HashMap, HashSet, VecDeque};",
    1,
)
text = text.replace(
    "    #[serde(default = \"default_rng_state\")]\n    pub rng_state: u64,\n",
    "    #[serde(default = \"default_rng_state\")]\n    pub rng_state: u64,\n    #[serde(default)]\n    pub pending_grains: Vec<u64>,\n",
    1,
)
text = text.replace(
    "pub struct SandEngine {\n    pub(crate) grid: Vec<Vec<Option<CategoryId>>>,\n    pub width: u16,\n    pub height: u16,\n    frame_count: usize,\n    sweep_left_to_right: bool,\n    rng_state: u64,\n    pub grain_count: usize,\n}",
    "pub struct SandEngine {\n    pub(crate) grid: Vec<Vec<Option<CategoryId>>>,\n    pub cell_width: u16,\n    pub cell_height: u16,\n    pub grid_width_dots: u16,\n    pub grid_height_dots: u16,\n    frame_count: usize,\n    sweep_left_to_right: bool,\n    rng_state: u64,\n    pending_grains: VecDeque<CategoryId>,\n    pub grain_count: usize,\n}",
    1,
)
text = text.replace(
    "            width,\n            height,\n            frame_count: 0,\n            sweep_left_to_right: true,\n            rng_state: rand::random::<u64>() | 1,\n            grain_count: 0,",
    "            cell_width: width,\n            cell_height: height,\n            grid_width_dots: 0,\n            grid_height_dots: 0,\n            frame_count: 0,\n            sweep_left_to_right: true,\n            rng_state: rand::random::<u64>() | 1,\n            pending_grains: VecDeque::new(),\n            grain_count: 0,",
    1,
)
text = text.replace(
    "        self.width = width * SAND_ENGINE.dot_width as u16;\n        self.height = height * SAND_ENGINE.dot_height as u16;",
    "        self.cell_width = width;\n        self.cell_height = height;\n        self.grid_width_dots = width * SAND_ENGINE.dot_width as u16;\n        self.grid_height_dots = height * SAND_ENGINE.dot_height as u16;",
    1,
)
text = text.replace(
    "        let new_w = self.width as usize;\n        let new_h = self.height as usize;",
    "        let new_w = self.grid_width_dots as usize;\n        let new_h = self.grid_height_dots as usize;",
    1,
)
text = text.replace(
    "            self.grain_count = 0;\n            return;",
    "            self.grain_count = self.pending_grains.len();\n            return;",
    1,
)
text = text.replace(
    "        self.apply_gravity();\n\n        self.grain_count = self\n            .grid\n            .iter()\n            .flat_map(|row| row.iter())\n            .filter(|c| c.is_some())\n            .count();",
    "        self.apply_gravity();\n        self.flush_pending_grains();\n        self.refresh_logical_grain_count();",
    1,
)
old_spawn = '''    pub fn spawn(&mut self, category_id: CategoryId) {
        let capacity = self.capacity();
        if capacity == 0 {
            return;
        }

        let w = self.grid[0].len();

        let x = self.random_index(w);

        if self.grid[0][x].is_none() {
            self.grid[0][x] = Some(category_id);
            self.grain_count += 1;
        } else {
            let fallback_x = self.random_index(w);
            if self.grid[0][fallback_x].is_none() {
                self.grid[0][fallback_x] = Some(category_id);
                self.grain_count += 1;
            }
        }
    }
'''
new_spawn = '''    pub fn spawn(&mut self, category_id: CategoryId) {
        self.pending_grains.push_back(category_id);
        self.grain_count += 1;
        self.flush_pending_grains();
    }

    pub fn pending_grain_count(&self) -> usize {
        self.pending_grains.len()
    }

    pub fn physical_grain_count(&self) -> usize {
        self.grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_some())
            .count()
    }

    fn refresh_logical_grain_count(&mut self) {
        self.grain_count = self.physical_grain_count() + self.pending_grains.len();
    }

    fn flush_pending_grains(&mut self) {
        if self.capacity() == 0 {
            return;
        }

        let width = self.grid[0].len();
        while let Some(category_id) = self.pending_grains.front().copied() {
            let start = self.random_index(width);
            let insertion = (0..width)
                .map(|offset| (start + offset) % width)
                .find(|&x| self.grid[0][x].is_none());

            let Some(x) = insertion else {
                break;
            };

            self.grid[0][x] = Some(category_id);
            self.pending_grains.pop_front();
        }
    }
'''
if text.count(old_spawn) != 1:
    raise SystemExit("spawn implementation did not match")
text = text.replace(old_spawn, new_spawn, 1)
text = text.replace(
    "    pub fn update(&mut self) {\n        self.frame_count += 1;\n        if self.frame_count.is_multiple_of(2) {\n            self.apply_gravity();\n        }\n    }",
    "    pub fn update(&mut self) {\n        self.frame_count += 1;\n        if self.frame_count.is_multiple_of(2) {\n            self.apply_gravity();\n        }\n        self.flush_pending_grains();\n    }",
    1,
)
text = text.replace(
    "        let cell_w = self.width as usize;\n        let cell_h = (self.height / SAND_ENGINE.dot_height as u16) as usize;",
    "        let cell_w = self.cell_width as usize;\n        let cell_h = self.cell_height as usize;",
    1,
)
text = text.replace(
    "        self.grain_count = 0;\n    }\n\n    pub fn clear_category",
    "        self.pending_grains.clear();\n        self.grain_count = 0;\n    }\n\n    pub fn clear_category",
    1,
)
old_clear_category = '''        self.grain_count = self.grain_count.saturating_sub(removed);
    }

    pub fn remove_category_grains'''
new_clear_category = '''        self.pending_grains
            .retain(|pending| *pending != category_id);
        self.refresh_logical_grain_count();
    }

    pub fn remove_category_grains'''
if text.count(old_clear_category) != 1:
    raise SystemExit("clear_category tail did not match")
text = text.replace(old_clear_category, new_clear_category, 1)
old_remove_tail = '''        if removed > 0 {
            self.grain_count = self.grain_count.saturating_sub(removed);
            self.apply_gravity();
        }

        removed
    }
'''
new_remove_tail = '''        let mut pending_removed = 0usize;
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

        if removed > 0 {
            self.apply_gravity();
        }
        self.flush_pending_grains();
        self.refresh_logical_grain_count();

        removed + pending_removed
    }
'''
if text.count(old_remove_tail) != 1:
    raise SystemExit("remove_category_grains tail did not match")
text = text.replace(old_remove_tail, new_remove_tail, 1)
text = text.replace(
    "            rng_state: self.rng_state,\n        }",
    "            rng_state: self.rng_state,\n            pending_grains: self\n                .pending_grains\n                .iter()\n                .map(|category_id| category_id.0)\n                .collect(),\n        }",
    1,
)
text = text.replace(
    "        let target_height = self.grid.len();\n        let target_width = self.grid.first().map_or(0, |row| row.len());",
    "        self.pending_grains = state\n            .pending_grains\n            .iter()\n            .map(|category_id| {\n                let category_id = CategoryId::new(*category_id);\n                if valid_category_ids.contains(&category_id) {\n                    category_id\n                } else {\n                    none_id\n                }\n            })\n            .collect();\n\n        let target_height = self.grid.len();\n        let target_width = self.grid.first().map_or(0, |row| row.len());",
    1,
)
old_restore_count = '''        self.grain_count = self
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_some())
            .count();
'''
if text.count(old_restore_count) != 1:
    raise SystemExit("restore grain count did not match")
text = text.replace(old_restore_count, "        self.refresh_logical_grain_count();\n", 1)

text += '''

#[cfg(test)]
mod conservation_tests {
    use std::collections::HashSet;

    use crate::{domain::CategoryId, sand::SandEngine};

    #[test]
    fn spawn_scans_every_ingress_column_before_blocking() {
        let mut engine = SandEngine::new(4, 2);
        let ingress_width = engine.grid[0].len();
        for x in 0..ingress_width - 1 {
            engine.grid[0][x] = Some(CategoryId::new(1));
        }
        engine.grain_count = ingress_width - 1;

        engine.spawn(CategoryId::new(2));

        assert_eq!(engine.grid[0][ingress_width - 1], Some(CategoryId::new(2)));
        assert_eq!(engine.pending_grain_count(), 0);
        assert_eq!(engine.grain_count, ingress_width);
    }

    #[test]
    fn blocked_spawn_remains_logical_until_ingress_reopens() {
        let mut engine = SandEngine::new(3, 2);
        let ingress_width = engine.grid[0].len();
        for x in 0..ingress_width {
            engine.grid[0][x] = Some(CategoryId::new(1));
        }
        engine.grain_count = ingress_width;

        engine.spawn(CategoryId::new(2));
        assert_eq!(engine.pending_grain_count(), 1);
        assert_eq!(engine.grain_count, ingress_width + 1);

        engine.grid[0][0] = None;
        engine.update();

        assert_eq!(engine.pending_grain_count(), 0);
        assert_eq!(engine.physical_grain_count(), ingress_width + 1);
        assert_eq!(engine.grain_count, ingress_width + 1);
    }

    #[test]
    fn render_uses_terminal_cell_dimensions_exactly() {
        let engine = SandEngine::new(3, 2);
        let lines = engine.render(&[]);

        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.spans.len() == 3));
        assert_eq!(engine.grid_width_dots as usize, 3 * 2);
        assert_eq!(engine.grid_height_dots as usize, 2 * 4);
    }

    #[test]
    fn pending_grains_round_trip_with_category_identity() {
        let mut engine = SandEngine::new(2, 1);
        for cell in &mut engine.grid[0] {
            *cell = Some(CategoryId::new(1));
        }
        engine.grain_count = engine.grid[0].len();
        engine.spawn(CategoryId::new(2));

        let state = engine.snapshot_state();
        assert_eq!(state.pending_grains, vec![2]);

        let mut restored = SandEngine::new(2, 1);
        restored.restore_state(
            &state,
            &HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)]),
        );

        assert_eq!(restored.pending_grain_count(), 1);
        assert_eq!(restored.grain_count, state.grains.len() + 1);
        assert_eq!(restored.snapshot_state().pending_grains, vec![2]);
    }
}
'''
engine.write_text(text)

replace_once(
    "src/app.rs",
    "                engine.width != expected_grid_width || engine.height != expected_grid_height",
    "                engine.grid_width_dots != expected_grid_width\n                    || engine.grid_height_dots != expected_grid_height",
)
replace_once(
    "src/app.rs",
    "            grains: projected_grains,\n            frame_count: state.frame_count,",
    "            grains: projected_grains,\n            frame_count: state.frame_count,\n            pending_grains: state.pending_grains.clone(),",
)

replace_once(
    "notebook/NOW.md",
    "summary: Persistence, temporal, domain, and report/export authority are complete; sediment conservation now leads the frontier.\nnext: Implement SEDIMENT-001 for issues #6, #7, #16, #18, and #26 without weakening chronological ledger truth.",
    "summary: SEDIMENT-001 is active; dimension truth and lossless ingress are the first conservation edge.\nnext: Complete SEDIMENT-001A for issues #16 and #26, then continue to viewport-independent logical sediment.",
)
replace_once(
    "notebook/NOW.md",
    "Implement **SEDIMENT-001**. Reconcile issues #6, #7, #16, #18, and #26 around one conserved logical sediment model. The visual projection may adapt to terminal geometry, but it must not silently create, discard, or reclassify accountable elapsed mass.",
    "Complete **SEDIMENT-001A**. Establish explicit terminal-cell versus dot-grid dimensions and retain every blocked due grain as pending logical mass. Then continue to SEDIMENT-001B without treating the current viewport as canonical storage.",
)

for temporary in [
    ".github/workflows/sediment001a-apply.yml",
    "tools/sediment001a-apply.py",
    "tools/sediment001a.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
