use super::*;
use super::flowviz_conservative::PARCEL_RENDER_MASS_CAP;

impl OsloSandboxEngine {
    pub(super) fn render_conservative_shadow_into_surface(&mut self) {
        if !self.flowviz_conservative {
            return;
        }
        let grid_height = self.surface.grid_height_dots;
        for (site, column) in self.flowviz_shadow_columns.iter().enumerate() {
            if site >= self.surface.grid_width_dots {
                break;
            }
            for (depth, category_id) in column.iter().copied().enumerate() {
                let Some(y) = grid_height.checked_sub(depth.saturating_add(1)) else {
                    break;
                };
                self.surface.grid[y][site] = Some(category_id);
            }
        }
    }

    pub(super) fn render_conservative_flowviz_parcels_into_surface(&mut self) {
        const OFFSETS: [(isize, isize); PARCEL_RENDER_MASS_CAP] = [
            (0, 0),
            (-1, 0),
            (1, 0),
            (0, -1),
            (-1, -1),
            (1, -1),
            (0, 1),
            (1, 1),
            (-1, 1),
            (-2, 0),
            (2, 0),
            (-2, -1),
            (2, -1),
            (-2, 1),
            (2, 1),
            (0, -2),
        ];
        let grid_height = self.surface.grid_height_dots;
        let grid_width = self.surface.grid_width_dots;
        for parcel in &self.flowviz_parcels {
            let base_x = parcel.x.round() as isize;
            let base_y = parcel.y.round() as isize;
            for &(dx, dy) in OFFSETS.iter().take(parcel.mass.min(OFFSETS.len())) {
                let x = base_x + dx;
                let y = base_y + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let x = x as usize;
                let y = y as usize;
                if x >= grid_width || y >= grid_height {
                    continue;
                }
                self.surface.grid[y][x] = Some(parcel.category_id);
                self.surface.mobilized[y][x] = true;
            }
        }
    }

    pub(crate) fn flowviz_parcel_mass(&self) -> usize {
        self.flowviz_mobile_mass
    }

    pub(crate) fn flowviz_shadow_mass(&self) -> usize {
        self.flowviz_shadow_columns.iter().map(Vec::len).sum()
    }

    pub(crate) fn flowviz_total_deposit_due(&self) -> usize {
        self.flowviz_deposit_due
            .iter()
            .flat_map(|due| due.iter())
            .map(|(_, count)| *count)
            .sum()
    }

    pub(crate) fn flowviz_peak_mobile_mass(&self) -> usize {
        self.flowviz_peak_mobile_mass
    }

    pub(crate) fn flowviz_visual_withdrawals(&self) -> usize {
        self.flowviz_visual_withdrawals
    }

    pub(crate) fn flowviz_visual_deposits(&self) -> usize {
        self.flowviz_visual_deposits
    }

    pub(crate) fn flowviz_reused_deposits(&self) -> usize {
        self.flowviz_reused_deposits
    }

    pub(crate) fn flowviz_coalesced_mass(&self) -> usize {
        self.flowviz_coalesced_mass
    }

    pub(crate) fn flowviz_shadow_misses(&self) -> usize {
        self.flowviz_shadow_misses
    }

    #[cfg(test)]
    pub(super) fn flowviz_visual_mass_matches_physical_mobile_system(&self) -> bool {
        if !self.flowviz_conservative {
            return true;
        }
        let parcel_sum = self.flowviz_parcels.iter().map(|parcel| parcel.mass).sum::<usize>();
        parcel_sum == self.flowviz_mobile_mass
            && self
                .flowviz_shadow_mass()
                .saturating_add(self.flowviz_parcel_mass())
                == self.settled_count().saturating_add(self.rolling_grains.len())
    }
}
