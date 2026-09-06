use crate::domain::CategoryId;

use super::ViewportBounds;

/// Ephemeral exact index for bottom-connected Classic pile columns.
///
/// `top[x]` is the first occupied row in the contiguous suffix that reaches the
/// visible bottom boundary, or `y_end` when that column currently has no
/// bottom-connected grain. The index exists only for one gravity sweep and is
/// updated after every in-place move, so it preserves Classic's current-sweep
/// contact semantics without rescanning from each blocker to the floor.
pub(super) struct GroundedColumnIndex {
    top: Vec<usize>,
    y_start: usize,
    y_end: usize,
}

impl GroundedColumnIndex {
    pub(super) fn from_grid(grid: &[Vec<Option<CategoryId>>], bounds: ViewportBounds) -> Self {
        let width = grid.first().map_or(0, Vec::len);
        let mut top = vec![bounds.y_end; width];
        for x in bounds.x_start..bounds.x_end {
            let mut y = bounds.y_end;
            while y > bounds.y_start && grid[y - 1][x].is_some() {
                y -= 1;
            }
            top[x] = y;
        }
        Self {
            top,
            y_start: bounds.y_start,
            y_end: bounds.y_end,
        }
    }

    pub(super) fn is_grounded(&self, grid: &[Vec<Option<CategoryId>>], x: usize, y: usize) -> bool {
        y < self.y_end && y >= self.top[x] && grid[y][x].is_some()
    }

    pub(super) fn record_move(
        &mut self,
        grid: &[Vec<Option<CategoryId>>],
        source_x: usize,
        source_y: usize,
        target_x: usize,
        target_y: usize,
    ) {
        self.record_vacancy(source_x, source_y);
        self.record_fill(grid, target_x, target_y);
    }

    fn record_vacancy(&mut self, x: usize, y: usize) {
        if y >= self.top[x] && y < self.y_end {
            // Removing any member of the bottom-connected suffix leaves the
            // cells strictly below the vacancy as the new exact suffix.
            self.top[x] = y + 1;
        }
    }

    fn record_fill(&mut self, grid: &[Vec<Option<CategoryId>>], x: usize, y: usize) {
        // A newly occupied cell can become grounded only when it closes the gap
        // immediately above the existing grounded suffix. If it does, include
        // any already-occupied contiguous cells above it as well.
        if y + 1 != self.top[x] {
            return;
        }
        let mut new_top = y;
        while new_top > self.y_start && grid[new_top - 1][x].is_some() {
            new_top -= 1;
        }
        self.top[x] = new_top;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_is_grounded(
        grid: &[Vec<Option<CategoryId>>],
        bounds: ViewportBounds,
        x: usize,
        y: usize,
    ) -> bool {
        y < bounds.y_end
            && grid[y][x].is_some()
            && (y..bounds.y_end).all(|row| grid[row][x].is_some())
    }

    #[test]
    fn index_tracks_vertical_and_diagonal_gap_changes_exactly() {
        let bounds = ViewportBounds {
            x_start: 0,
            x_end: 4,
            y_start: 0,
            y_end: 8,
        };
        let category = CategoryId::new(1);
        let mut grid = vec![vec![None; 4]; 8];
        for row in grid.iter_mut().take(8).skip(4) {
            row[1] = Some(category);
        }
        for row in grid.iter_mut().take(8).skip(6) {
            row[2] = Some(category);
        }

        let mut index = GroundedColumnIndex::from_grid(&grid, bounds);
        for x in 0..4 {
            for y in 0..8 {
                assert_eq!(
                    index.is_grounded(&grid, x, y),
                    scan_is_grounded(&grid, bounds, x, y)
                );
            }
        }

        // Diagonalize the top grounded grain from column 1 into the gap directly
        // above column 2's grounded suffix.
        grid[4][1] = None;
        grid[5][2] = Some(category);
        index.record_move(&grid, 1, 4, 2, 5);
        for x in 0..4 {
            for y in 0..8 {
                assert_eq!(
                    index.is_grounded(&grid, x, y),
                    scan_is_grounded(&grid, bounds, x, y)
                );
            }
        }

        // A vertical airborne move that lands directly above grounded terrain
        // must extend the suffix by exactly one row. Use another column with an
        // actual one-cell vertical gap.
        grid[5][3] = Some(category);
        grid[7][3] = Some(category);
        index = GroundedColumnIndex::from_grid(&grid, bounds);
        grid[5][3] = None;
        grid[6][3] = Some(category);
        index.record_move(&grid, 3, 5, 3, 6);
        for y in 0..8 {
            assert_eq!(
                index.is_grounded(&grid, 3, y),
                scan_is_grounded(&grid, bounds, 3, y)
            );
        }
    }
}
