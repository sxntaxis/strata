use std::collections::HashMap;

use rand::Rng;
use ratatui::{
    prelude::{Line, Span},
    style::{Color, Stylize},
};

use crate::{
    constants::{COLORS, SAND_ENGINE},
    domain::{Category, CategoryId},
};

use super::resize::resize_grid;

pub struct SandEngine {
    pub(crate) grid: Vec<Vec<Option<CategoryId>>>,
    pub width: u16,
    pub height: u16,
    frame_count: usize,
    pub grain_count: usize,
}

impl SandEngine {
    pub fn new(width: u16, height: u16) -> Self {
        let mut se = Self {
            grid: vec![],
            width,
            height,
            frame_count: 0,
            grain_count: 0,
        };
        se.resize(width, height);
        se
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width * SAND_ENGINE.dot_width as u16;
        self.height = height * SAND_ENGINE.dot_height as u16;

        let old_w = if self.grid.is_empty() {
            0
        } else {
            self.grid[0].len()
        };
        let old_h = self.grid.len();

        let new_w = self.width as usize;
        let new_h = self.height as usize;

        if old_w == 0 || old_h == 0 {
            self.grid = vec![vec![None; new_w]; new_h];
            self.grain_count = 0;
            return;
        }

        if new_w == old_w && new_h == old_h {
            return;
        }

        self.grid = resize_grid(
            &self.grid,
            new_w,
            new_h,
            SAND_ENGINE.dot_width,
            SAND_ENGINE.dot_height,
        );

        self.apply_gravity();

        self.grain_count = self
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.is_some())
            .count();
    }

    fn capacity(&self) -> usize {
        if self.grid.is_empty() || self.grid[0].is_empty() {
            0
        } else {
            self.grid.len() * self.grid[0].len()
        }
    }

    pub fn spawn(&mut self, category_id: CategoryId) {
        let capacity = self.capacity();
        if capacity == 0 {
            return;
        }

        let mut rng = rand::thread_rng();
        let w = self.grid[0].len();

        let x = rng.gen_range(0..w);

        if self.grid[0][x].is_none() {
            self.grid[0][x] = Some(category_id);
            self.grain_count += 1;
        } else {
            let fallback_x = rng.gen_range(0..w);
            if self.grid[0][fallback_x].is_none() {
                self.grid[0][fallback_x] = Some(category_id);
                self.grain_count += 1;
            }
        }
    }

    fn apply_gravity(&mut self) {
        let h = self.grid.len();
        let w = self.grid[0].len();

        for y in (0..h - 1).rev() {
            for x in 0..w {
                if let Some(cat) = self.grid[y][x] {
                    if self.grid[y + 1][x].is_none() {
                        self.grid[y + 1][x] = Some(cat);
                        self.grid[y][x] = None;
                    } else {
                        let dir: isize = if rand::random() { 1 } else { -1 };
                        let nx = (x as isize) + dir;

                        if nx >= 0 && (nx as usize) < w && self.grid[y + 1][nx as usize].is_none() {
                            self.grid[y + 1][nx as usize] = Some(cat);
                            self.grid[y][x] = None;
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self) {
        self.frame_count += 1;
        if self.frame_count % 2 == 0 {
            self.apply_gravity();
        }
    }

    pub fn render(&self, categories: &[Category]) -> Vec<Line<'static>> {
        let cell_w = self.width as usize;
        let cell_h = (self.height / SAND_ENGINE.dot_height as u16) as usize;
        let grid_h = self.grid.len();
        let grid_w = self.grid.first().map_or(0, |row| row.len());
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(cell_h);

        let category_map: HashMap<CategoryId, usize> = categories
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id, i))
            .collect();

        for cy in 0..cell_h {
            let mut spans: Vec<Span<'static>> = Vec::with_capacity(cell_w);

            for cx in 0..cell_w {
                let mut dots = 0u8;
                let num_categories = categories.len();
                let mut counts: Vec<usize> = vec![0; num_categories.max(1)];

                for dy in 0..SAND_ENGINE.dot_height {
                    for dx in 0..SAND_ENGINE.dot_width {
                        let gx = cx * SAND_ENGINE.dot_width + dx;
                        let gy = cy * SAND_ENGINE.dot_height + dy;

                        if gy < grid_h && gx < grid_w {
                            if let Some(cat_id) = self.grid[gy][gx] {
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

                                let cat_pos = category_map.get(&cat_id).copied().unwrap_or(0);
                                let cat_count = cat_pos.min(num_categories.saturating_sub(1));
                                counts[cat_count] += 1;
                            }
                        }
                    }
                }

                let total_colored_dots: usize = counts.iter().sum();
                let color = if total_colored_dots > 0 {
                    let mut blended_r = 0f32;
                    let mut blended_g = 0f32;
                    let mut blended_b = 0f32;

                    for (cat_idx, &count) in counts.iter().enumerate() {
                        if count == 0 {
                            continue;
                        }

                        let (r, g, b) = if cat_idx == 0 {
                            (255u8, 255u8, 255u8)
                        } else if cat_idx < categories.len() {
                            match categories[cat_idx].color {
                                Color::Rgb(r, g, b) => (r, g, b),
                                _ => (255, 255, 255),
                            }
                        } else {
                            match COLORS[cat_idx % COLORS.len()] {
                                Color::Rgb(r, g, b) => (r, g, b),
                                _ => (255, 255, 255),
                            }
                        };

                        let weight = count as f32 / total_colored_dots as f32;
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

    pub fn clear(&mut self) {
        for row in &mut self.grid {
            for cell in row {
                *cell = None;
            }
        }
        self.grain_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::{constants::SAND_ENGINE, domain::CategoryId, sand::SandEngine};

    #[test]
    fn test_sand_resize_basic_copy() {
        let mut se = SandEngine::new(20, 20);
        se.grid[40][20] = Some(CategoryId::new(0));

        let before = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        se.resize(20, 20);

        let after = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        assert_eq!(before, after);
    }

    #[test]
    fn test_sand_resize_expand_preserves_grains() {
        let mut se = SandEngine::new(20, 20);
        se.grid[40][20] = Some(CategoryId::new(0));

        let before = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        se.resize(40, 40);

        let after = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        assert_eq!(before, after);
    }

    #[test]
    fn test_sand_resize_shrink_center_preserves_grains() {
        let mut se = SandEngine::new(40, 40);
        se.grid[80][40] = Some(CategoryId::new(0));

        let before = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        se.resize(20, 20);

        let after = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        assert_eq!(before, after);
    }

    #[test]
    fn test_sand_resize_preserves_count_right_edge() {
        let mut se = SandEngine::new(80, 50);
        let cell_w = se.width as usize / SAND_ENGINE.dot_width as usize;
        let cell_h = se.height as usize / SAND_ENGINE.dot_height as usize;

        for cy in 0..cell_h {
            for cx in (cell_w - 10..cell_w).rev() {
                if cx < cell_w {
                    se.grid[cy][cx] = Some(CategoryId::new(1));
                }
            }
        }

        se.grain_count = se
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.is_some())
            .count();

        let original_count = se.grain_count;

        se.resize(60, 50);

        assert_eq!(se.grain_count, original_count);
    }

    #[test]
    fn test_sand_resize_preserves_count_expand() {
        let mut se = SandEngine::new(50, 50);
        let cell_w = se.width as usize / SAND_ENGINE.dot_width as usize;
        let cell_h = se.height as usize / SAND_ENGINE.dot_height as usize;

        if cell_h > 2 && cell_w > 2 {
            se.grid[cell_h / 2][cell_w / 2] = Some(CategoryId::new(0));
            se.grid[cell_h / 2 + 1][cell_w / 2] = Some(CategoryId::new(0));
        }

        se.grain_count = se
            .grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|c| c.is_some())
            .count();

        let original_count = se.grain_count;

        se.resize(80, 80);

        assert!(se.grain_count >= original_count);
    }

    #[test]
    fn test_sand_resize_left_edge_band() {
        let mut se = SandEngine::new(40, 40);

        for y in 0..se.grid.len() {
            for x in 0..(5 * SAND_ENGINE.dot_width as usize).min(se.grid[0].len()) {
                se.grid[y][x] = Some(CategoryId::new(1));
            }
        }

        let before = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        se.resize(30, 40);

        let after = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| c.is_some())
            .count();

        let new_capacity = 30 * 40 * SAND_ENGINE.dot_width * SAND_ENGINE.dot_height;
        let expected = before.min(new_capacity);

        assert_eq!(after, expected);

        let band_w = (30 / 40).max(2).min(6);
        let left_band_count: usize = (0..se.grid.len())
            .flat_map(|y| (0..band_w).map(move |x| (y, x)))
            .filter(|(y, x)| se.grid[*y][*x].is_some())
            .count();

        assert!(left_band_count > 0);
    }

    #[test]
    fn test_sand_resize_preserves_category_id_per_grain() {
        let mut se = SandEngine::new(40, 40);

        for y in 0..20 {
            for x in 0..20 {
                se.grid[y][x] = Some(CategoryId::new(1));
            }
        }
        for y in 20..40 {
            for x in 20..40 {
                se.grid[y][x] = Some(CategoryId::new(2));
            }
        }

        se.resize(30, 30);

        let work_count = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| **c == Some(CategoryId::new(1)))
            .count();
        let _play_count = se
            .grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|c| **c == Some(CategoryId::new(2)))
            .count();

        assert!(work_count > 0);
    }
}
