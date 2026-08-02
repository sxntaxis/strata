from pathlib import Path


def replace_exact(path: str, old: str, new: str, expected: int = 1) -> None:
    target = Path(path)
    text = target.read_text()
    actual = text.count(old)
    if actual != expected:
        raise SystemExit(
            f"expected {expected} matches in {path}, found {actual}: {old[:100]!r}"
        )
    target.write_text(text.replace(old, new))


replace_exact(
    "src/sand/engine.rs",
    '#[serde(default)]\n    pub pending_grains: Vec<u64>,',
    '#[serde(default, skip_serializing_if = "Vec::is_empty")]\n    pub pending_grains: Vec<u64>,',
)

replace_exact(
    "src/app/render_views.rs",
    "        if self.sand_engine.width != inner_width * SAND_ENGINE.dot_width as u16\n            || self.sand_engine.height != inner_height * SAND_ENGINE.dot_height as u16",
    "        if self.sand_engine.grid_width_dots != inner_width * SAND_ENGINE.dot_width as u16\n            || self.sand_engine.grid_height_dots\n                != inner_height * SAND_ENGINE.dot_height as u16",
)

replace_exact(
    "src/app/report_state.rs",
    "        let grid_width = self.sand_engine.width as usize;\n        let grid_height = self.sand_engine.height as usize;",
    "        let grid_width = self.sand_engine.grid_width_dots as usize;\n        let grid_height = self.sand_engine.grid_height_dots as usize;",
)
replace_exact(
    "src/app/report_state.rs",
    "            sweep_left_to_right: true,\n            rng_state: 0,\n        })",
    "            sweep_left_to_right: true,\n            rng_state: 0,\n            pending_grains: Vec::new(),\n        })",
)

replace_exact(
    "src/sand/engine.rs",
    "        let cell_w = se.width as usize / SAND_ENGINE.dot_width;",
    "        let cell_w = se.cell_width as usize;",
    expected=2,
)
replace_exact(
    "src/sand/engine.rs",
    "        let cell_h = se.height as usize / SAND_ENGINE.dot_height;",
    "        let cell_h = se.cell_height as usize;",
    expected=2,
)
replace_exact(
    "src/sand/engine.rs",
    "    pub fn pending_grain_count(&self) -> usize {",
    "    #[cfg(test)]\n    fn pending_grain_count(&self) -> usize {",
)

replace_exact(
    "src/sand/engine.rs",
    "    pub fn clear_category(&mut self, category_id: CategoryId) {\n        let mut removed = 0usize;\n\n        for row in &mut self.grid {\n            for cell in row {\n                if *cell == Some(category_id) {\n                    *cell = None;\n                    removed += 1;\n                }\n            }\n        }",
    "    pub fn clear_category(&mut self, category_id: CategoryId) {\n        for row in &mut self.grid {\n            for cell in row {\n                if *cell == Some(category_id) {\n                    *cell = None;\n                }\n            }\n        }",
)

replace_exact(
    "src/sand/engine.rs",
    "        engine.grid[0][0] = None;\n        engine.update();\n\n        assert_eq!(engine.pending_grain_count(), 0);\n        assert_eq!(engine.physical_grain_count(), ingress_width + 1);",
    "        let displaced = engine.grid[0][0].take().expect(\"occupied ingress\");\n        engine.grid[1][0] = Some(displaced);\n        engine.update();\n\n        assert_eq!(engine.pending_grain_count(), 0);\n        assert_eq!(engine.physical_grain_count(), ingress_width + 1);",
)

replace_exact(
    "src/sqlite/fault_certification.rs",
    "        sweep_left_to_right: true,\n        rng_state: u64::try_from(frame_count).unwrap(),\n    }",
    "        sweep_left_to_right: true,\n        rng_state: u64::try_from(frame_count).unwrap(),\n        pending_grains: Vec::new(),\n    }",
)
replace_exact(
    "src/sqlite/tui_runtime.rs",
    "            sweep_left_to_right: true,\n            rng_state: 9,\n        };",
    "            sweep_left_to_right: true,\n            rng_state: 9,\n            pending_grains: Vec::new(),\n        };",
)
replace_exact(
    "src/storage.rs",
    "            sweep_left_to_right: false,\n            rng_state: 12345,\n        };",
    "            sweep_left_to_right: false,\n            rng_state: 12345,\n            pending_grains: vec![3],\n        };",
)

Path("tools/sediment001a-fix.py").unlink(missing_ok=True)
