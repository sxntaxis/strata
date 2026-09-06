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

    pub(crate) fn flowviz_unit_truthful_enabled(&self) -> bool {
        self.flowviz_unit_truthful
    }

    pub(crate) fn flowviz_unit_coherent_enabled(&self) -> bool {
        self.flowviz_unit_coherent
    }

    #[cfg(test)]
    pub(crate) fn flowviz_unit_direct_geometry_enabled(&self) -> bool {
        self.flowviz_unit_direct_geometry
    }

    #[cfg(test)]
    pub(crate) fn flowviz_unit_visual_support_enabled(&self) -> bool {
        self.flowviz_unit_visual_support
    }

    #[cfg(test)]
    pub(crate) fn flowviz_unit_distance_aware_enabled(&self) -> bool {
        self.flowviz_unit_distance_aware
    }

    /// SEDIMENT-015E presentation barrier. A rolling carrier that has already
    /// replayed every observed segment is caught up even though its physical
    /// grain remains mobile. New authoritative Oslo work is blocked only while
    /// an observed segment, settlement arrival, or egress animation is still
    /// pending.
    pub(crate) fn flowviz_unit_coherent_batch_pending(&self) -> bool {
        if !self.flowviz_unit_coherent {
            return false;
        }
        self.flowviz_unit_carriers.values().any(|carrier| {
            carrier.active.is_some()
                || !carrier.queued.is_empty()
                || match carrier.physical {
                    flowviz_unit::UnitPhysicalState::Rolling => false,
                    flowviz_unit::UnitPhysicalState::Settled(_)
                    | flowviz_unit::UnitPhysicalState::Discharged => !carrier.arrived,
                }
        })
    }

    pub(crate) fn flowviz_unit_presentation_backlog(&self) -> usize {
        if !self.flowviz_unit_perceptual {
            return 0;
        }
        self.flowviz_unit_carriers
            .values()
            .map(|carrier| {
                let arrival_debt = if self.flowviz_unit_coherent {
                    matches!(
                        carrier.physical,
                        flowviz_unit::UnitPhysicalState::Settled(_)
                            | flowviz_unit::UnitPhysicalState::Discharged
                    ) && !carrier.arrived
                } else {
                    !carrier.arrived
                };
                carrier.queued.len() + carrier.active.is_some() as usize + arrival_debt as usize
            })
            .sum()
    }

    pub(crate) fn flowviz_unit_presentation_substeps(&self) -> usize {
        let backlog = self.flowviz_unit_presentation_backlog();
        if self.flowviz_unit_coherent {
            return match backlog {
                0 => 1,
                1..=8_192 => 6,
                _ => 8,
            };
        }
        match backlog {
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

    /// SEDIMENT-015D testing frame. Rain remains visually alive while the
    /// avalanche is active, but commit is still impossible until authoritative
    /// flow is quiescent. This spends presentation CPU without allowing ingress
    /// to drive or mutate the active pile.
    pub(crate) fn advance_testing_truthful_flow_frame(&mut self) -> bool {
        debug_assert!(self.flowviz_unit_truthful);
        self.frame_count = if self.frame_count.is_multiple_of(2) {
            self.frame_count.wrapping_add(2)
        } else {
            self.frame_count.wrapping_add(1)
        };
        let mut changed = self.advance_falling_drives();
        changed |= self.advance_fluidization_field();
        changed |= self.advance_rolling_grains();
        changed |= self.advance_rolling_visual_motion();
        if self.topple_one_active_site() {
            return true;
        }
        changed
    }

    /// SEDIMENT-015E lockstep presentation frame. Oslo may emit at most one
    /// lattice hop per rolling grain in an authoritative rolling update. Once
    /// such a batch has produced visual debt, subsequent wall-clock frames
    /// spend their budget only on that already-observed presentation until it
    /// catches up. This prevents a carrier from accumulating hops from multiple
    /// physical epochs while the settled shadow has already advanced further.
    pub(crate) fn advance_testing_coherent_flow_frame(&mut self) -> bool {
        debug_assert!(self.flowviz_unit_coherent);
        if self.flowviz_unit_coherent_batch_pending() {
            self.finalize_direct_rolling_batch_geometry();
            return self.advance_testing_truthful_visual_frame();
        }

        self.frame_count = if self.frame_count.is_multiple_of(2) {
            self.frame_count.wrapping_add(2)
        } else {
            self.frame_count.wrapping_add(1)
        };
        let mut changed = self.advance_falling_drives();
        changed |= self.advance_fluidization_field();
        changed |= self.advance_rolling_grains();
        changed |= self.topple_one_active_site();
        self.finalize_direct_rolling_batch_geometry();
        // Presentation advances only after the complete authoritative quantum.
        // The next authoritative quantum is forbidden until this newly observed
        // batch is visually caught up.
        changed |= self.advance_rolling_visual_motion();
        changed
    }

    /// 015D presentation-only cadence after physical quiescence. Existing rain
    /// keeps falling at the ordinary grain frame rate, while commit remains the
    /// responsibility of a later authoritative quiescent frame.
    pub(crate) fn advance_testing_truthful_visual_frame(&mut self) -> bool {
        debug_assert!(self.flowviz_unit_truthful);
        let mut changed = self.advance_falling_drives();
        changed |= self.advance_rolling_visual_motion();
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
            carrier.queued.iter().all(|segment| {
                segment.observed_source_y.is_some() && segment.observed_target_y.is_some()
            }) && carrier.active.is_none_or(|active| {
                active.segment.observed_source_y.is_some()
                    && active.segment.observed_target_y.is_some()
            })
        })
    }
}
