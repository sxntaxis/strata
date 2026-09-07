use crate::domain::CategoryId;

/// Exact sparse row occupancy mirror for the Classic runtime.
///
/// The grid remains authoritative. This non-persisted cache lets a gravity sweep
/// visit only cells occupied at the start of each row pass. Classic moves grains
/// only to the already-processed row below, so mutating the index during a
/// bottom-up sweep preserves the same visit order as the canonical dense scan.
pub(super) struct RowOccupancyIndex {
    words_per_row: usize,
    rows: Vec<u64>,
    width: usize,
    height: usize,
}

impl RowOccupancyIndex {
    pub(super) fn from_grid(grid: &[Vec<Option<CategoryId>>]) -> Self {
        let height = grid.len();
        let width = grid.first().map_or(0, Vec::len);
        let words_per_row = width.div_ceil(64);
        let mut rows = vec![0u64; height.saturating_mul(words_per_row)];
        for (y, row) in grid.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if cell.is_some() {
                    let word = x / 64;
                    let bit = x % 64;
                    rows[y * words_per_row + word] |= 1u64 << bit;
                }
            }
        }
        Self {
            words_per_row,
            rows,
            width,
            height,
        }
    }

    pub(super) fn set(&mut self, x: usize, y: usize, occupied: bool) {
        if x >= self.width || y >= self.height || self.words_per_row == 0 {
            return;
        }
        let index = y * self.words_per_row + x / 64;
        let mask = 1u64 << (x % 64);
        if occupied {
            self.rows[index] |= mask;
        } else {
            self.rows[index] &= !mask;
        }
    }

    pub(super) fn record_move(
        &mut self,
        source_x: usize,
        source_y: usize,
        target_x: usize,
        target_y: usize,
    ) {
        self.set(source_x, source_y, false);
        self.set(target_x, target_y, true);
    }

    pub(super) fn next_occupied(
        &self,
        y: usize,
        start: usize,
        end_exclusive: usize,
    ) -> Option<usize> {
        if y >= self.height || start >= end_exclusive || start >= self.width {
            return None;
        }
        let end_exclusive = end_exclusive.min(self.width);
        if start >= end_exclusive || self.words_per_row == 0 {
            return None;
        }

        let mut word_index = start / 64;
        let last_word = (end_exclusive - 1) / 64;
        let row_offset = y * self.words_per_row;
        let mut word = self.rows[row_offset + word_index] & (!0u64 << (start % 64));

        loop {
            if word_index == last_word {
                let end_bit = end_exclusive % 64;
                if end_bit != 0 {
                    word &= (1u64 << end_bit) - 1;
                }
            }
            if word != 0 {
                let x = word_index * 64 + word.trailing_zeros() as usize;
                return (x < end_exclusive).then_some(x);
            }
            if word_index == last_word {
                return None;
            }
            word_index += 1;
            word = self.rows[row_offset + word_index];
        }
    }

    pub(super) fn previous_occupied(
        &self,
        y: usize,
        start_inclusive: usize,
        end_exclusive: usize,
    ) -> Option<usize> {
        if y >= self.height || start_inclusive >= end_exclusive || self.words_per_row == 0 {
            return None;
        }
        let end_exclusive = end_exclusive.min(self.width);
        if start_inclusive >= end_exclusive {
            return None;
        }

        let mut word_index = (end_exclusive - 1) / 64;
        let first_word = start_inclusive / 64;
        let row_offset = y * self.words_per_row;
        let high_bit = end_exclusive % 64;
        let mut word = self.rows[row_offset + word_index];
        if high_bit != 0 {
            word &= (1u64 << high_bit) - 1;
        }

        loop {
            if word_index == first_word {
                let low_bit = start_inclusive % 64;
                word &= !0u64 << low_bit;
            }
            if word != 0 {
                let bit = 63usize - word.leading_zeros() as usize;
                let x = word_index * 64 + bit;
                return (x >= start_inclusive && x < end_exclusive).then_some(x);
            }
            if word_index == first_word {
                return None;
            }
            word_index -= 1;
            word = self.rows[row_offset + word_index];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_iteration_matches_row_order_across_word_boundaries() {
        let category = CategoryId::new(1);
        let mut grid = vec![vec![None; 140]; 2];
        for x in [0usize, 1, 63, 64, 65, 127, 128, 139] {
            grid[0][x] = Some(category);
        }
        let index = RowOccupancyIndex::from_grid(&grid);

        let mut forward = Vec::new();
        let mut cursor = 0;
        while let Some(x) = index.next_occupied(0, cursor, 140) {
            forward.push(x);
            cursor = x + 1;
        }
        assert_eq!(forward, vec![0, 1, 63, 64, 65, 127, 128, 139]);

        let mut reverse = Vec::new();
        let mut cursor = 140;
        while let Some(x) = index.previous_occupied(0, 0, cursor) {
            reverse.push(x);
            cursor = x;
        }
        assert_eq!(reverse, vec![139, 128, 127, 65, 64, 63, 1, 0]);
    }

    #[test]
    fn recorded_move_updates_source_and_target_rows() {
        let category = CategoryId::new(1);
        let mut grid = vec![vec![None; 8]; 4];
        grid[1][3] = Some(category);
        let mut index = RowOccupancyIndex::from_grid(&grid);
        index.record_move(3, 1, 4, 2);

        assert_eq!(index.next_occupied(1, 0, 8), None);
        assert_eq!(index.next_occupied(2, 0, 8), Some(4));
    }
}
