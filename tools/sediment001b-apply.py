from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if text.count(old) != 1:
        raise SystemExit(f"expected one match in {path}: {old[:100]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


engine_path = Path("src/sand/engine.rs")
engine = engine_path.read_text()
engine = engine.replace("\nuse super::resize::resize_grid;\n", "\n", 1)
engine = engine.replace(
    "    pub grid_width_dots: u16,\n    pub grid_height_dots: u16,",
    "    pub grid_width_dots: usize,\n    pub grid_height_dots: usize,",
    1,
)
engine_path.write_text(engine)

replace_between(
    "src/sand/engine.rs",
    "impl SandEngine {\n    pub fn new",
    "    fn capacity(&self)",
    '''impl SandEngine {
    pub fn new(width: u16, height: u16) -> Self {
        let grid_width_dots = width as usize * SAND_ENGINE.dot_width;
        let grid_height_dots = height as usize * SAND_ENGINE.dot_height;

        Self {
            grid: vec![vec![None; grid_width_dots]; grid_height_dots],
            cell_width: width,
            cell_height: height,
            grid_width_dots,
            grid_height_dots,
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: rand::random::<u64>() | 1,
            pending_grains: VecDeque::new(),
            grain_count: 0,
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.cell_width = width;
        self.cell_height = height;
    }

''',
)

replace_between(
    "src/sand/engine.rs",
    "    pub fn render(&self, categories: &[Category])",
    "    pub fn clear(&mut self)",
    '''    pub fn render(&self, categories: &[Category]) -> Vec<Line<'static>> {
        let cell_w = self.cell_width as usize;
        let cell_h = self.cell_height as usize;
        let viewport_width_dots = cell_w.saturating_mul(SAND_ENGINE.dot_width);
        let viewport_height_dots = cell_h.saturating_mul(SAND_ENGINE.dot_height);
        let grid_h = self.grid.len();
        let grid_w = self.grid.first().map_or(0, |row| row.len());
        let visible_width = grid_w.min(viewport_width_dots);
        let visible_height = grid_h.min(viewport_height_dots);
        let source_x = grid_w.saturating_sub(visible_width) / 2;
        let source_y = grid_h.saturating_sub(visible_height);
        let destination_x = viewport_width_dots.saturating_sub(visible_width) / 2;
        let destination_y = viewport_height_dots.saturating_sub(visible_height);
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(cell_h);

        let category_colors: HashMap<CategoryId, Color> = categories
            .iter()
            .map(|category| (category.id, category.color))
            .collect();
        let none_id = DRIFT_CATEGORY_ID;

        for cy in 0..cell_h {
            let mut spans: Vec<Span<'static>> = Vec::with_capacity(cell_w);

            for cx in 0..cell_w {
                let mut dots = 0u8;
                let mut counts: HashMap<CategoryId, usize> = HashMap::new();

                for dy in 0..SAND_ENGINE.dot_height {
                    for dx in 0..SAND_ENGINE.dot_width {
                        let viewport_x = cx * SAND_ENGINE.dot_width + dx;
                        let viewport_y = cy * SAND_ENGINE.dot_height + dy;

                        if viewport_x < destination_x
                            || viewport_x >= destination_x + visible_width
                            || viewport_y < destination_y
                            || viewport_y >= destination_y + visible_height
                        {
                            continue;
                        }

                        let grid_x = source_x + viewport_x - destination_x;
                        let grid_y = source_y + viewport_y - destination_y;

                        if let Some(cat_id) = self.grid[grid_y][grid_x] {
                            let dot_index = match (dx, dy) {
                                (0, 0) => 0,
                                (0, 1) => 1,
                                (0, 2) => 2,
                                (0, 3) => 6,
                                (1, 0) => 3,
                                (1, 1) => 4,
                                (1, 2) => 5,
                                (1, 3) => 7,
                                _ => 0,
                            };
                            dots |= 1 << dot_index;
                            *counts.entry(cat_id).or_insert(0) += 1;
                        }
                    }
                }

                let total_colored_dots: usize = counts.values().sum();
                let color = if total_colored_dots > 0 {
                    let mut blended_r = 0f32;
                    let mut blended_g = 0f32;
                    let mut blended_b = 0f32;

                    for (category_id, count) in &counts {
                        let (r, g, b) = if *category_id == none_id {
                            (255u8, 255u8, 255u8)
                        } else {
                            match category_colors
                                .get(category_id)
                                .copied()
                                .unwrap_or(Color::White)
                            {
                                Color::Rgb(r, g, b) => (r, g, b),
                                _ => (255, 255, 255),
                            }
                        };

                        let weight = *count as f32 / total_colored_dots as f32;
                        blended_r += r as f32 * weight;
                        blended_g += g as f32 * weight;
                        blended_b += b as f32 * weight;
                    }

                    Color::Rgb(blended_r as u8, blended_g as u8, blended_b as u8)
                } else {
                    Color::White
                };

                let ch = char::from_u32(SAND_ENGINE.braille_base + dots as u32).unwrap_or(' ');
                spans.push(Span::raw(ch.to_string()).fg(color));
            }

            lines.push(Line::from(spans));
        }

        lines
    }

''',
)

replace_between(
    "src/sand/engine.rs",
    "    pub fn restore_state(&mut self, state: &SandState",
    "    fn next_random_u64(&mut self)",
    '''    pub fn restore_state(&mut self, state: &SandState, valid_category_ids: &HashSet<CategoryId>) {
        if state.version != SandState::VERSION {
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

        self.grid = restored;
        self.grid_width_dots = state.grid_width;
        self.grid_height_dots = state.grid_height;
        self.pending_grains = state
            .pending_grains
            .iter()
            .map(|category_id| {
                let category_id = CategoryId::new(*category_id);
                if valid_category_ids.contains(&category_id) {
                    category_id
                } else {
                    none_id
                }
            })
            .collect();
        self.refresh_logical_grain_count();
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
start = engine.index("    #[test]\n    fn test_sand_resize_basic_copy()")
end = engine.index("    #[test]\n    fn test_sand_state_snapshot_restore_round_trip()", start)
new_resize_tests = '''    #[test]
    fn viewport_resize_preserves_exact_logical_state() {
        let mut engine = SandEngine::new(12, 8);
        engine.clear();
        engine.grid[0][0] = Some(CategoryId::new(1));
        engine.grid[10][12] = Some(CategoryId::new(2));
        engine.grid[31][23] = Some(CategoryId::new(1));
        engine.pending_grains.push_back(CategoryId::new(2));
        engine.grain_count = 4;
        engine.frame_count = 17;
        engine.sweep_left_to_right = false;
        engine.rng_state = 0xCAFE_BABE;
        let before = engine.snapshot_state();

        engine.resize(3, 2);
        assert_eq!(engine.snapshot_state(), before);

        engine.resize(30, 20);
        assert_eq!(engine.snapshot_state(), before);
    }

    #[test]
    fn oscillating_viewport_resizes_are_logically_idempotent() {
        let mut engine = SandEngine::new(10, 6);
        engine.clear();
        for (x, y, category_id) in [(0, 0, 1), (7, 8, 2), (19, 23, 1)] {
            engine.grid[y][x] = Some(CategoryId::new(category_id));
        }
        engine.grain_count = 3;
        let baseline = engine.snapshot_state();

        for (width, height) in [(2, 1), (40, 20), (5, 3), (10, 6), (1, 1)] {
            engine.resize(width, height);
            assert_eq!(engine.snapshot_state(), baseline);
        }
    }

    #[test]
    fn hidden_grains_reappear_when_viewport_expands() {
        let mut engine = SandEngine::new(4, 2);
        engine.clear();
        engine.grid[0][0] = Some(CategoryId::new(0));
        engine.grain_count = 1;

        engine.resize(2, 1);
        let cropped = engine.render(&[]);
        assert!(cropped.iter().flat_map(|line| line.spans.iter()).all(|span| {
            span.content.as_ref() == "\u{2800}"
        }));

        engine.resize(4, 2);
        let expanded = engine.render(&[]);
        assert!(expanded.iter().flat_map(|line| line.spans.iter()).any(|span| {
            span.content.as_ref() != "\u{2800}"
        }));
    }

    #[test]
    fn viewport_render_size_is_independent_of_logical_canvas_size() {
        let mut engine = SandEngine::new(4, 2);
        engine.resize(9, 5);
        let expanded = engine.render(&[]);
        assert_eq!(expanded.len(), 5);
        assert!(expanded.iter().all(|line| line.spans.len() == 9));

        engine.resize(1, 1);
        let shrunk = engine.render(&[]);
        assert_eq!(shrunk.len(), 1);
        assert_eq!(shrunk[0].spans.len(), 1);
        assert_eq!(engine.grid_width_dots, 8);
        assert_eq!(engine.grid_height_dots, 8);
    }

'''
engine = engine[:start] + new_resize_tests + engine[end:]
engine_path.write_text(engine)

engine = engine_path.read_text()
start = engine.index("    #[test]\n    fn test_sand_state_restore_resizes_to_current_grid()")
end = engine.index("    #[test]\n    fn test_clear_category_removes_only_requested_id()", start)
replacement = '''    #[test]
    fn test_sand_state_restore_preserves_canonical_grid_on_different_viewport() {
        let mut source = SandEngine::new(20, 20);
        source.clear();
        source.grid[2][2] = Some(CategoryId::new(1));
        source.grid[20][20] = Some(CategoryId::new(2));
        source.grain_count = 2;
        let state = source.snapshot_state();

        let mut restored = SandEngine::new(40, 40);
        let valid = HashSet::from([CategoryId::new(0), CategoryId::new(1), CategoryId::new(2)]);
        restored.restore_state(&state, &valid);

        assert_eq!(restored.snapshot_state(), state);
        assert_eq!(restored.cell_width, 40);
        assert_eq!(restored.cell_height, 40);
        assert_eq!(restored.grid_width_dots, state.grid_width);
        assert_eq!(restored.grid_height_dots, state.grid_height);
    }

'''
engine_path.write_text(engine[:start] + replacement + engine[end:])

replace_once(
    "src/sand/engine.rs",
    "    use crate::{constants::SAND_ENGINE, domain::CategoryId, sand::SandEngine};",
    "    use crate::{domain::CategoryId, sand::SandEngine};",
)
replace_once(
    "src/sand/engine.rs",
    "        assert_eq!(engine.grid_width_dots as usize, 3 * 2);\n        assert_eq!(engine.grid_height_dots as usize, 2 * 4);",
    "        assert_eq!(engine.grid_width_dots, 3 * 2);\n        assert_eq!(engine.grid_height_dots, 2 * 4);",
)

replace_once(
    "src/app/render_views.rs",
    "use crate::constants::{APP_LAYOUT_SETTINGS, SAND_ENGINE};",
    "use crate::constants::APP_LAYOUT_SETTINGS;",
)
replace_once(
    "src/app/render_views.rs",
    "        if self.sand_engine.grid_width_dots != inner_width * SAND_ENGINE.dot_width as u16\n            || self.sand_engine.grid_height_dots\n                != inner_height * SAND_ENGINE.dot_height as u16",
    "        if self.sand_engine.cell_width != inner_width\n            || self.sand_engine.cell_height != inner_height",
)

replace_once(
    "src/app.rs",
    "        let expected_grid_width = cell_width * SAND_ENGINE.dot_width as u16;\n        let expected_grid_height = cell_height * SAND_ENGINE.dot_height as u16;\n        let visual_cadence = Duration::from_millis(CATCHUP_SETTINGS.visual_refresh_ms);",
    "        let visual_cadence = Duration::from_millis(CATCHUP_SETTINGS.visual_refresh_ms);",
)
replace_once(
    "src/app.rs",
    "                engine.grid_width_dots != expected_grid_width\n                    || engine.grid_height_dots != expected_grid_height",
    "                engine.cell_width != cell_width || engine.cell_height != cell_height",
)

replace_once(
    "src/app/report_state.rs",
    "        let grid_width = self.sand_engine.grid_width_dots as usize;\n        let grid_height = self.sand_engine.grid_height_dots as usize;",
    "        let grid_width = self.sand_engine.grid_width_dots;\n        let grid_height = self.sand_engine.grid_height_dots;",
)

replace_once("src/sand/mod.rs", "mod engine;\nmod resize;", "mod engine;")

constants = Path("src/constants.rs")
text = constants.read_text()
start = text.index("pub const SAND_RESIZE_SETTINGS")
end = text.index("pub const ATLAS_LAYOUT_SETTINGS", start)
text = text[:start] + text[end:]
start = text.index("pub struct SandResizeSettings")
end = text.index("pub struct AtlasLayoutSettings", start)
text = text[:start] + text[end:]
constants.write_text(text)

Path("src/sand/resize.rs").unlink()

for temporary in [
    ".github/workflows/sediment001b-apply.yml",
    "tools/sediment001b-apply.py",
    "tools/sediment001b.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
