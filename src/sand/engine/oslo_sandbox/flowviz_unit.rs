use super::*;

pub(super) const UNIT_SEGMENT_STEP: f32 = 0.24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FlowVizMotionId(pub(super) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UnitPhysicalState {
    Rolling,
    Settled(usize),
    Discharged,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UnitSegment {
    pub(super) sequence: u64,
    pub(super) source: usize,
    pub(super) destination: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UnitActiveSegment {
    pub(super) segment: UnitSegment,
    pub(super) start_x: f32,
    pub(super) start_y: f32,
    pub(super) progress: f32,
}

#[derive(Debug, Clone)]
pub(super) struct FlowVizUnitCarrier {
    pub(super) id: FlowVizMotionId,
    pub(super) category_id: CategoryId,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) queued: VecDeque<UnitSegment>,
    pub(super) active: Option<UnitActiveSegment>,
    pub(super) physical: UnitPhysicalState,
    pub(super) arrived: bool,
}

impl OsloSandboxEngine {
    pub(super) fn enable_unit_flowviz(&mut self) {
        self.flowviz_enabled = true;
        self.flowviz_conservative = false;
        self.flowviz_unit = true;
        self.flowviz_edge_flux = vec![0; self.columns.len()];
        self.flowviz_tracers.clear();
        self.flowviz_parcels.clear();
        self.flowviz_shadow_columns = self.columns.clone();
        self.flowviz_unit_custody = self
            .columns
            .iter()
            .map(|column| vec![None; column.len()])
            .collect();
        self.flowviz_unit_carriers.clear();
        self.flowviz_unit_next_id = 1;
        self.flowviz_unit_next_sequence = 1;
        self.flowviz_unit_peak = 0;
        self.flowviz_unit_segments = 0;
        self.flowviz_unit_reveals = 0;
        self.flowviz_unit_misses = 0;
    }

    pub(super) fn reset_unit_flowviz_from_physics(&mut self) {
        if !self.flowviz_unit {
            return;
        }
        self.flowviz_shadow_columns = self.columns.clone();
        self.flowviz_unit_custody = self
            .columns
            .iter()
            .map(|column| vec![None; column.len()])
            .collect();
        self.flowviz_unit_carriers.clear();
        self.flowviz_unit_next_id = 1;
        self.flowviz_unit_next_sequence = 1;
        self.flowviz_unit_peak = 0;
        self.flowviz_unit_segments = 0;
        self.flowviz_unit_reveals = 0;
        self.flowviz_unit_misses = 0;
    }

    pub(super) fn clear_unit_flowviz(&mut self) {
        if !self.flowviz_unit {
            return;
        }
        self.reset_unit_flowviz_from_physics();
    }

    pub(super) fn resize_unit_flowviz_for_growth(&mut self, left_added: usize, new_width: usize) {
        if !self.flowviz_unit {
            return;
        }
        let old_shadow = std::mem::take(&mut self.flowviz_shadow_columns);
        self.flowviz_shadow_columns = vec![Vec::new(); new_width];
        for (index, column) in old_shadow.into_iter().enumerate() {
            self.flowviz_shadow_columns[index + left_added] = column;
        }
        let old_custody = std::mem::take(&mut self.flowviz_unit_custody);
        self.flowviz_unit_custody = vec![Vec::new(); new_width];
        for (index, column) in old_custody.into_iter().enumerate() {
            self.flowviz_unit_custody[index + left_added] = column;
        }
        if left_added == 0 {
            return;
        }
        let shift = left_added as f32;
        for carrier in self.flowviz_unit_carriers.values_mut() {
            carrier.x += shift;
            if let UnitPhysicalState::Settled(site) = &mut carrier.physical {
                *site += left_added;
            }
            if let Some(active) = &mut carrier.active {
                active.start_x += shift;
                active.segment.source += left_added;
                active.segment.destination = active.segment.destination.map(|site| site + left_added);
            }
            for segment in &mut carrier.queued {
                segment.source += left_added;
                segment.destination = segment.destination.map(|site| site + left_added);
            }
        }
    }

    pub(super) fn pop_unit_custody(&mut self, site: usize) -> Option<FlowVizMotionId> {
        self.flowviz_unit_custody
            .get_mut(site)
            .and_then(Vec::pop)
            .flatten()
    }

    pub(super) fn push_unit_custody(&mut self, site: usize, custody: Option<FlowVizMotionId>) {
        if let Some(column) = self.flowviz_unit_custody.get_mut(site) {
            column.push(custody);
        }
    }

    pub(super) fn record_flowviz_mobile_entry_with_custody(
        &mut self,
        source: usize,
        destination: usize,
        category_id: CategoryId,
        source_y: usize,
        existing: Option<FlowVizMotionId>,
    ) -> Option<FlowVizMotionId> {
        if self.flowviz_unit {
            self.begin_unit_motion(
                source,
                Some(destination),
                category_id,
                source_y,
                existing,
            )
        } else {
            self.record_flowviz_mobile_entry(source, destination, category_id, source_y);
            None
        }
    }

    pub(super) fn begin_unit_motion(
        &mut self,
        source: usize,
        destination: Option<usize>,
        category_id: CategoryId,
        source_y: usize,
        existing: Option<FlowVizMotionId>,
    ) -> Option<FlowVizMotionId> {
        if !self.flowviz_unit {
            return None;
        }
        if destination.is_some_and(|site| source.abs_diff(site) != 1) {
            self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
            return None;
        }
        let id = if let Some(id) = existing {
            let Some(carrier) = self.flowviz_unit_carriers.get_mut(&id) else {
                self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
                return None;
            };
            if carrier.category_id != category_id
                || carrier.physical != UnitPhysicalState::Settled(source)
            {
                self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
                return None;
            }
            carrier.physical = UnitPhysicalState::Rolling;
            carrier.arrived = false;
            id
        } else {
            let Some(shadow) = self.flowviz_shadow_columns.get_mut(source) else {
                self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
                return None;
            };
            if shadow.last().copied() != Some(category_id) {
                self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
                return None;
            }
            shadow.pop();
            let id = FlowVizMotionId(self.flowviz_unit_next_id);
            self.flowviz_unit_next_id = self.flowviz_unit_next_id.saturating_add(1);
            self.flowviz_unit_carriers.insert(
                id,
                FlowVizUnitCarrier {
                    id,
                    category_id,
                    x: source as f32,
                    y: source_y as f32,
                    queued: VecDeque::new(),
                    active: None,
                    physical: UnitPhysicalState::Rolling,
                    arrived: false,
                },
            );
            self.flowviz_unit_peak = self.flowviz_unit_peak.max(self.flowviz_unit_carriers.len());
            id
        };
        self.append_unit_segment(id, source, destination);
        Some(id)
    }

    pub(super) fn adopt_unit_visible_settlement(
        &mut self,
        site: usize,
        category_id: CategoryId,
        visual_y: usize,
    ) -> Option<FlowVizMotionId> {
        if !self.flowviz_unit
            || self.flowviz_shadow_columns.get(site).map_or(0, Vec::len)
                >= self.columns.get(site).map_or(0, Vec::len)
        {
            return None;
        }
        let id = FlowVizMotionId(self.flowviz_unit_next_id);
        self.flowviz_unit_next_id = self.flowviz_unit_next_id.saturating_add(1);
        self.flowviz_unit_carriers.insert(
            id,
            FlowVizUnitCarrier {
                id,
                category_id,
                x: site as f32,
                y: visual_y as f32,
                queued: VecDeque::new(),
                active: None,
                physical: UnitPhysicalState::Settled(site),
                arrived: true,
            },
        );
        self.flowviz_unit_peak = self.flowviz_unit_peak.max(self.flowviz_unit_carriers.len());
        Some(id)
    }

    pub(super) fn append_unit_segment(
        &mut self,
        id: FlowVizMotionId,
        source: usize,
        destination: Option<usize>,
    ) {
        if destination.is_some_and(|site| source.abs_diff(site) != 1) {
            self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
            return;
        }
        if !self.flowviz_unit_carriers.contains_key(&id) {
            self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
            return;
        }
        let sequence = self.flowviz_unit_next_sequence;
        self.flowviz_unit_next_sequence = self.flowviz_unit_next_sequence.saturating_add(1);
        let carrier = self
            .flowviz_unit_carriers
            .get_mut(&id)
            .expect("unit carrier existence was checked");
        carrier.queued.push_back(UnitSegment {
            sequence,
            source,
            destination,
        });
        carrier.arrived = false;
        self.flowviz_unit_segments = self.flowviz_unit_segments.saturating_add(1);
    }

    pub(super) fn mark_unit_settled(&mut self, id: FlowVizMotionId, site: usize) {
        if let Some(carrier) = self.flowviz_unit_carriers.get_mut(&id) {
            carrier.physical = UnitPhysicalState::Settled(site);
            carrier.arrived = false;
        } else {
            self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
        }
    }

    pub(super) fn mark_unit_discharged(&mut self, id: FlowVizMotionId) {
        if let Some(carrier) = self.flowviz_unit_carriers.get_mut(&id) {
            carrier.physical = UnitPhysicalState::Discharged;
            carrier.arrived = false;
        } else {
            self.flowviz_unit_misses = self.flowviz_unit_misses.saturating_add(1);
        }
    }
}
