use crate::domain::CategoryId;

#[derive(Default)]
struct LostGrains {
    left: Vec<CategoryId>,
    right: Vec<CategoryId>,
    top: Vec<CategoryId>,
    bottom: Vec<CategoryId>,
}

pub fn resize_grid(
    old_grid: &[Vec<Option<CategoryId>>],
    new_w: usize,
    new_h: usize,
    dot_width: usize,
    dot_height: usize,
) -> Vec<Vec<Option<CategoryId>>> {
    let old_h = old_grid.len();
    let old_w = old_grid.first().map_or(0, |row| row.len());

    if old_w == 0 || old_h == 0 {
        return vec![vec![None; new_w]; new_h];
    }

    if old_w == new_w && old_h == new_h {
        return old_grid.to_vec();
    }

    let mut new_grid = vec![vec![None; new_w]; new_h];

    let (x_src_start, x_src_end, x_dest_offset) = kept_window(old_w, new_w);
    let (y_src_start, y_src_end, y_dest_offset) = kept_window(old_h, new_h);

    for y_src in y_src_start..y_src_end {
        for x_src in x_src_start..x_src_end {
            let x_dest = x_src - x_src_start + x_dest_offset;
            let y_dest = y_src - y_src_start + y_dest_offset;
            new_grid[y_dest][x_dest] = old_grid[y_src][x_src];
        }
    }

    let lost = classify_lost_grains(old_grid, x_src_start, x_src_end, y_src_start, y_src_end);

    let new_cell_w = if dot_width == 0 { 0 } else { new_w / dot_width };
    let new_cell_h = if dot_height == 0 {
        0
    } else {
        new_h / dot_height
    };
    let band_w = (new_cell_w / 40).max(2).min(6);
    let band_h = (new_cell_h / 40).max(1).min(3);
    let band_w_px = (band_w * dot_width).min(new_w);
    let band_h_px = (band_h * dot_height).min(new_h);

    place_left_band(&mut new_grid, &lost.left, band_w_px);
    place_right_band(&mut new_grid, &lost.right, band_w_px);
    place_top_band(&mut new_grid, &lost.top, band_h_px);
    place_bottom_band(&mut new_grid, &lost.bottom, band_h_px);

    let left_capacity = band_w_px * new_h;
    let right_capacity = band_w_px * new_h;
    let top_capacity = band_h_px * new_w;
    let bottom_capacity = band_h_px * new_w;

    let mut remaining = Vec::new();
    remaining.extend(lost.left.iter().skip(left_capacity).copied());
    remaining.extend(lost.right.iter().skip(right_capacity).copied());
    remaining.extend(lost.top.iter().skip(top_capacity).copied());
    remaining.extend(lost.bottom.iter().skip(bottom_capacity).copied());

    place_overflow(&mut new_grid, &remaining);

    new_grid
}

fn kept_window(old_len: usize, new_len: usize) -> (usize, usize, usize) {
    if new_len < old_len {
        let src_start = (old_len - new_len) / 2;
        (src_start, src_start + new_len, 0)
    } else if new_len > old_len {
        let dest_offset = (new_len - old_len) / 2;
        (0, old_len, dest_offset)
    } else {
        (0, old_len, 0)
    }
}

fn classify_lost_grains(
    old_grid: &[Vec<Option<CategoryId>>],
    x_src_start: usize,
    x_src_end: usize,
    y_src_start: usize,
    y_src_end: usize,
) -> LostGrains {
    let mut lost = LostGrains::default();

    let old_h = old_grid.len();
    let old_w = old_grid.first().map_or(0, |row| row.len());

    for y in 0..old_h {
        for x in 0..old_w {
            if x >= x_src_start && x < x_src_end && y >= y_src_start && y < y_src_end {
                continue;
            }

            if let Some(cat_id) = old_grid[y][x] {
                let lost_from_left = x < x_src_start;
                let lost_from_right = x >= x_src_end;
                let lost_from_top = y < y_src_start;
                let lost_from_bottom = y >= y_src_end;

                if lost_from_left || lost_from_right {
                    if lost_from_left {
                        lost.left.push(cat_id);
                    }
                    if lost_from_right {
                        lost.right.push(cat_id);
                    }
                } else if lost_from_top || lost_from_bottom {
                    if lost_from_top {
                        lost.top.push(cat_id);
                    }
                    if lost_from_bottom {
                        lost.bottom.push(cat_id);
                    }
                }
            }
        }
    }

    lost
}

fn place_left_band(grid: &mut [Vec<Option<CategoryId>>], grains: &[CategoryId], band_w_px: usize) {
    let h = grid.len();
    let mut iter = grains.iter();

    'outer: for y in (0..h).rev() {
        for x in 0..band_w_px {
            if grid[y][x].is_none() {
                if let Some(cat) = iter.next() {
                    grid[y][x] = Some(*cat);
                } else {
                    break 'outer;
                }
            }
        }
    }
}

fn place_right_band(grid: &mut [Vec<Option<CategoryId>>], grains: &[CategoryId], band_w_px: usize) {
    let h = grid.len();
    let w = grid.first().map_or(0, |row| row.len());
    let start = w.saturating_sub(band_w_px);
    let mut iter = grains.iter();

    'outer: for y in (0..h).rev() {
        for x in (start..w).rev() {
            if grid[y][x].is_none() {
                if let Some(cat) = iter.next() {
                    grid[y][x] = Some(*cat);
                } else {
                    break 'outer;
                }
            }
        }
    }
}

fn place_top_band(grid: &mut [Vec<Option<CategoryId>>], grains: &[CategoryId], band_h_px: usize) {
    let w = grid.first().map_or(0, |row| row.len());
    let mut iter = grains.iter();

    'outer: for y in (0..band_h_px).rev() {
        for x in 0..w {
            if grid[y][x].is_none() {
                if let Some(cat) = iter.next() {
                    grid[y][x] = Some(*cat);
                } else {
                    break 'outer;
                }
            }
        }
    }
}

fn place_bottom_band(
    grid: &mut [Vec<Option<CategoryId>>],
    grains: &[CategoryId],
    band_h_px: usize,
) {
    let h = grid.len();
    let w = grid.first().map_or(0, |row| row.len());
    let start = h.saturating_sub(band_h_px);
    let mut iter = grains.iter();

    'outer: for y in start..h {
        for x in 0..w {
            if grid[y][x].is_none() {
                if let Some(cat) = iter.next() {
                    grid[y][x] = Some(*cat);
                } else {
                    break 'outer;
                }
            }
        }
    }
}

fn place_overflow(grid: &mut [Vec<Option<CategoryId>>], grains: &[CategoryId]) {
    let h = grid.len();
    let w = grid.first().map_or(0, |row| row.len());

    'grain: for cat in grains {
        for y in (0..h).rev() {
            for x in 0..w {
                if grid[y][x].is_none() {
                    grid[y][x] = Some(*cat);
                    continue 'grain;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resize_grid;
    use crate::domain::CategoryId;

    fn count_grains(grid: &[Vec<Option<CategoryId>>]) -> usize {
        grid.iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_some())
            .count()
    }

    #[test]
    fn test_resize_grid_left_band_preserves_count() {
        let mut old = vec![vec![None; 80]; 40];
        for row in &mut old {
            for cell in row.iter_mut().take(8) {
                *cell = Some(CategoryId::new(1));
            }
        }

        let before = count_grains(&old);
        let resized = resize_grid(&old, 60, 40, 2, 4);
        let after = count_grains(&resized);

        assert_eq!(before, after);
        assert!(resized.iter().any(|row| row[0].is_some()));
    }

    #[test]
    fn test_resize_grid_preserves_category_ids() {
        let mut old = vec![vec![None; 80]; 40];
        for row in old.iter_mut().take(20) {
            for cell in row.iter_mut().take(20) {
                *cell = Some(CategoryId::new(1));
            }
        }
        for row in old.iter_mut().skip(20) {
            for cell in row.iter_mut().skip(20).take(20) {
                *cell = Some(CategoryId::new(2));
            }
        }

        let resized = resize_grid(&old, 60, 30, 2, 4);
        let work_count = resized
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| **cell == Some(CategoryId::new(1)))
            .count();

        assert!(work_count > 0);
    }
}
