use super::*;

impl OsloSandboxEngine {
    pub(super) fn unit_observed_settled_target_y(&self, site: usize) -> f32 {
        self.surface.grid_height_dots as f32
            - self.columns.get(site).map_or(0, Vec::len) as f32
            - 1.0
    }

    pub(super) fn unit_observed_rolling_target_y(&self, site: usize) -> f32 {
        self.surface.grid_height_dots as f32
            - self.columns.get(site).map_or(0, Vec::len) as f32
            - self.rolling_depth_at(site) as f32
            - 1.0
    }

    pub(super) fn unit_observed_egress_target_y(&self, source_y: usize) -> f32 {
        (source_y as f32 + 1.0).min(self.surface.grid_height_dots.saturating_sub(1) as f32)
    }

    pub(crate) fn flowviz_unit_perceptual_enabled(&self) -> bool {
        self.flowviz_unit_perceptual
    }

    pub(crate) fn flowviz_unit_presentation_backlog(&self) -> usize {
        if !self.flowviz_unit_perceptual {
            return 0;
        }
        self.flowviz_unit_carriers
            .values()
            .map(|carrier| {
                carrier.queued.len()
                    + carrier.active.is_some() as usize
                    + (!carrier.arrived) as usize
            })
            .sum()
    }

    pub(crate) fn flowviz_unit_presentation_substeps(&self) -> usize {
        match self.flowviz_unit_presentation_backlog() {
            0 => 1,
            1..=2_048 => 2,
            2_049..=8_192 => 3,
            _ => 4,
        }
    }

    /// Testing-only wall-clock frame for SEDIMENT-015C. Authoritative avalanche
    /// physics advances exactly as the ordinary full frame, but ingress dots do
    /// not move or launch while the event is active. Their y-position is only
    /// presentation state and commit was already forbidden by explicit flow.
    pub(crate) fn advance_testing_perceptual_flow_frame(&mut self) -> bool {
        debug_assert!(self.flowviz_unit_perceptual);
        self.frame_count = if self.frame_count.is_multiple_of(2) {
            self.frame_count.wrapping_add(2)
        } else {
            self.frame_count.wrapping_add(1)
        };
        let mut changed = self.advance_fluidization_field();
        changed |= self.advance_rolling_grains();
        changed |= self.advance_rolling_visual_motion();
        if self.topple_one_active_site() {
            return true;
        }
        changed
    }

    /// Presentation-only catch-up used between authoritative flow frames. It
    /// deliberately does not advance falling ingress, fluidity, rolling physics,
    /// thresholds, RNG, or settled mass.
    pub(crate) fn advance_testing_perceptual_visual_frame(&mut self) -> bool {
        debug_assert!(self.flowviz_unit_perceptual);
        self.advance_rolling_visual_motion()
    }

    #[cfg(test)]
    pub(super) fn flowviz_unit_observed_geometry_complete(&self) -> bool {
        if !self.flowviz_unit_perceptual {
            return false;
        }
        self.flowviz_unit_carriers.values().all(|carrier| {
            carrier
                .queued
                .iter()
                .all(|segment| segment.observed_source_y.is_some() && segment.observed_target_y.is_some())
                && carrier.active.is_none_or(|active| {
                    active.segment.observed_source_y.is_some()
                        && active.segment.observed_target_y.is_some()
                })
        })
    }
}
