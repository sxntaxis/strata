use std::{collections::HashSet, fmt::Write as _};

use ratatui::prelude::Line;
use serde::{Deserialize, Serialize};

use crate::domain::{Category, CategoryId, DRIFT_CATEGORY_ID};

use super::engine::{PendingGrainRun, SandEngine, SandState, SandStateGrain};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SedimentSnapshotKind {
    CumulativeCheckpoint,
    DailyContribution,
    DerivedPreview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SedimentSnapshotProvenance {
    RuntimeCanonical,
    LegacyDailyRow,
    SessionLedger,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SedimentIdlePolicy {
    Included,
    Excluded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SedimentSnapshot {
    pub schema_version: u8,
    pub kind: SedimentSnapshotKind,
    pub operational_day: Option<String>,
    pub source_revision: String,
    pub provenance: SedimentSnapshotProvenance,
    pub idle_policy: SedimentIdlePolicy,
    pub reconstructed: bool,
    pub state: SandState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DailySedimentSlice {
    pub category_id: u64,
    pub elapsed_seconds: usize,
    pub start_time: String,
    pub end_time: String,
    pub session_id: usize,
}

impl SedimentSnapshot {
    pub const VERSION: u8 = 1;

    fn cumulative_checkpoint(
        operational_day: Option<String>,
        source_revision: String,
        provenance: SedimentSnapshotProvenance,
        state: SandState,
    ) -> Self {
        Self {
            schema_version: Self::VERSION,
            kind: SedimentSnapshotKind::CumulativeCheckpoint,
            operational_day,
            source_revision,
            provenance,
            idle_policy: SedimentIdlePolicy::Included,
            reconstructed: false,
            state,
        }
    }

    pub(crate) fn day_end_checkpoint(operational_day: String, state: SandState) -> Self {
        let mut source = format!("day-end-v1|{operational_day}|").into_bytes();
        source.extend(serde_json::to_vec(&state).unwrap_or_default());
        Self::cumulative_checkpoint(
            Some(operational_day),
            stable_source_revision(&source),
            SedimentSnapshotProvenance::RuntimeCanonical,
            state,
        )
    }

    pub fn daily_contribution(
        operational_day: String,
        source_revision: String,
        state: SandState,
    ) -> Self {
        Self {
            schema_version: Self::VERSION,
            kind: SedimentSnapshotKind::DailyContribution,
            operational_day: Some(operational_day),
            source_revision,
            provenance: SedimentSnapshotProvenance::SessionLedger,
            idle_policy: SedimentIdlePolicy::Included,
            reconstructed: true,
            state,
        }
    }

    pub fn derived_preview(
        operational_day: String,
        source_revision: String,
        state: SandState,
    ) -> Self {
        Self {
            schema_version: Self::VERSION,
            kind: SedimentSnapshotKind::DerivedPreview,
            operational_day: Some(operational_day),
            source_revision,
            provenance: SedimentSnapshotProvenance::SessionLedger,
            idle_policy: SedimentIdlePolicy::Included,
            reconstructed: true,
            state,
        }
    }

    pub(crate) fn legacy_daily_payload(operational_day: String, state: SandState) -> Self {
        let encoded = serde_json::to_vec(&state).unwrap_or_default();
        Self::cumulative_checkpoint(
            Some(operational_day),
            format!("legacy-{}", stable_source_revision(&encoded)),
            SedimentSnapshotProvenance::LegacyDailyRow,
            state,
        )
    }

    pub(crate) fn is_authentic_day_end_for(&self, operational_day: &str) -> bool {
        self.schema_version == Self::VERSION
            && self.kind == SedimentSnapshotKind::CumulativeCheckpoint
            && self.operational_day.as_deref() == Some(operational_day)
            && !self.reconstructed
            && matches!(
                self.provenance,
                SedimentSnapshotProvenance::RuntimeCanonical
                    | SedimentSnapshotProvenance::LegacyDailyRow
            )
    }

    pub fn render_cache_key(&self, width: u16, height: u16) -> String {
        let encoded = serde_json::to_vec(self).unwrap_or_default();
        format!("{}:{width}:{height}", stable_source_revision(&encoded))
    }

    pub fn render_immutable(
        &self,
        width: u16,
        height: u16,
        categories: &[Category],
    ) -> Vec<Line<'static>> {
        let mut valid_category_ids = categories
            .iter()
            .map(|category| category.id)
            .collect::<HashSet<CategoryId>>();
        valid_category_ids.insert(DRIFT_CATEGORY_ID);
        let mut engine = SandEngine::new(width, height);
        engine
            .restore_state(&self.state, &valid_category_ids)
            .expect("validated sediment snapshot must restore");
        engine.resize(width, height);
        engine.render(categories)
    }

    pub fn display_label(&self) -> String {
        let kind = match self.kind {
            SedimentSnapshotKind::CumulativeCheckpoint => {
                if self.operational_day.is_some() && !self.reconstructed {
                    "day-end checkpoint"
                } else {
                    "cumulative checkpoint"
                }
            }
            SedimentSnapshotKind::DailyContribution => "daily contribution",
            SedimentSnapshotKind::DerivedPreview => "derived preview",
        };
        let idle = match self.idle_policy {
            SedimentIdlePolicy::Included => "idle included",
            SedimentIdlePolicy::Excluded => "idle excluded",
        };
        if self.reconstructed {
            format!("{kind} · reconstructed · {idle}")
        } else {
            format!("{kind} · {idle}")
        }
    }
}

pub(crate) fn daily_contribution_from_slices(
    operational_day: &str,
    slices: &[DailySedimentSlice],
) -> Option<SedimentSnapshot> {
    let ordered = ordered_daily_slices(slices);
    if ordered.is_empty() {
        return None;
    }
    let source_revision = ledger_source_revision(operational_day, &ordered, None);
    let pending_runs = pending_runs_for_slices(&ordered)?;
    let state = SandState {
        version: SandState::VERSION,
        grid_width: 0,
        grid_height: 0,
        grains: Vec::new(),
        frame_count: 0,
        sweep_left_to_right: true,
        rng_state: 0,
        pending_grains: Vec::new(),
        pending_runs,
    };
    Some(SedimentSnapshot::daily_contribution(
        operational_day.to_string(),
        source_revision,
        state,
    ))
}

pub(crate) fn derived_preview_from_slices(
    operational_day: &str,
    grid_width: usize,
    grid_height: usize,
    slices: &[DailySedimentSlice],
) -> Option<SedimentSnapshot> {
    let capacity = grid_width.checked_mul(grid_height)?;
    if capacity == 0 {
        return None;
    }
    let ordered = ordered_daily_slices(slices);
    if ordered.is_empty() {
        return None;
    }
    let source_revision = ledger_source_revision(
        operational_day,
        &ordered,
        Some((grid_width, grid_height)),
    );
    let total_seconds = ordered.iter().try_fold(0usize, |total, slice| {
        total.checked_add(slice.elapsed_seconds)
    })?;
    let physical_count = total_seconds.min(capacity);
    let mut grains = Vec::with_capacity(physical_count);
    let mut pending_runs = Vec::<PendingGrainRun>::new();
    let mut physical_remaining = physical_count;

    for slice in ordered {
        let placed = slice.elapsed_seconds.min(physical_remaining);
        for _ in 0..placed {
            let grain_index = grains.len();
            let x = grain_index % grid_width;
            let row = grain_index / grid_width;
            let y = grid_height - 1 - row;
            grains.push(SandStateGrain {
                x,
                y,
                category_id: slice.category_id,
            });
        }
        physical_remaining -= placed;
        append_pending_run(
            &mut pending_runs,
            slice.category_id,
            slice.elapsed_seconds - placed,
        )?;
    }

    let state = SandState {
        version: SandState::VERSION,
        grid_width,
        grid_height,
        grains,
        frame_count: 0,
        sweep_left_to_right: true,
        rng_state: 0,
        pending_grains: Vec::new(),
        pending_runs,
    };
    Some(SedimentSnapshot::derived_preview(
        operational_day.to_string(),
        source_revision,
        state,
    ))
}

fn ordered_daily_slices(slices: &[DailySedimentSlice]) -> Vec<DailySedimentSlice> {
    let mut ordered = slices
        .iter()
        .filter(|slice| slice.elapsed_seconds > 0)
        .cloned()
        .collect::<Vec<_>>();
    ordered.sort_by(|a, b| {
        a.start_time
            .cmp(&b.start_time)
            .then(a.end_time.cmp(&b.end_time))
            .then(a.session_id.cmp(&b.session_id))
            .then(a.category_id.cmp(&b.category_id))
    });
    ordered
}

fn ledger_source_revision(
    operational_day: &str,
    ordered: &[DailySedimentSlice],
    preview_grid: Option<(usize, usize)>,
) -> String {
    let mut revision_material = format!("day={operational_day}|idle=included|quantum=1|");
    if let Some((grid_width, grid_height)) = preview_grid {
        let _ = write!(revision_material, "preview-grid={grid_width}x{grid_height}|");
    }
    for slice in ordered {
        let _ = write!(
            revision_material,
            "{}:{}:{}:{}:{}|",
            slice.category_id,
            slice.elapsed_seconds,
            slice.start_time,
            slice.end_time,
            slice.session_id
        );
    }
    stable_source_revision(revision_material.as_bytes())
}

fn append_pending_run(
    pending_runs: &mut Vec<PendingGrainRun>,
    category_id: u64,
    count: usize,
) -> Option<()> {
    if count == 0 {
        return Some(());
    }
    if let Some(last) = pending_runs.last_mut()
        && last.category_id == category_id
    {
        last.count = last.count.checked_add(count)?;
    } else {
        pending_runs.push(PendingGrainRun { category_id, count });
    }
    Some(())
}

fn pending_runs_for_slices(ordered: &[DailySedimentSlice]) -> Option<Vec<PendingGrainRun>> {
    let mut pending_runs = Vec::new();
    for slice in ordered {
        append_pending_run(&mut pending_runs, slice.category_id, slice.elapsed_seconds)?;
    }
    Some(pending_runs)
}

pub(crate) fn select_historical_visual_artifact(
    operational_day: &str,
    authentic: Option<SedimentSnapshot>,
    derived: Option<SedimentSnapshot>,
) -> Option<SedimentSnapshot> {
    authentic
        .filter(|snapshot| snapshot.is_authentic_day_end_for(operational_day))
        .or(derived)
}

pub fn stable_source_revision(source: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::{
        DailySedimentSlice, SedimentSnapshot, SedimentSnapshotKind, SedimentSnapshotProvenance,
        daily_contribution_from_slices, derived_preview_from_slices,
        select_historical_visual_artifact, stable_source_revision,
    };
    use crate::sand::{PendingGrainRun, SandState, SandStateGrain};

    fn state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 4,
            grid_height: 4,
            grains: vec![SandStateGrain {
                x: 1,
                y: 3,
                category_id: 0,
            }],
            frame_count: 13,
            sweep_left_to_right: false,
            rng_state: 77,
            pending_grains: Vec::new(),
            pending_runs: vec![PendingGrainRun {
                category_id: 0,
                count: 5,
            }],
        }
    }

    fn slices() -> Vec<DailySedimentSlice> {
        vec![
            DailySedimentSlice {
                category_id: 1,
                elapsed_seconds: 5,
                start_time: "09:00:00".to_string(),
                end_time: "09:00:05".to_string(),
                session_id: 1,
            },
            DailySedimentSlice {
                category_id: 0,
                elapsed_seconds: 4,
                start_time: "09:00:05".to_string(),
                end_time: "09:00:09".to_string(),
                session_id: 2,
            },
        ]
    }

    #[test]
    fn snapshot_kinds_are_distinct_and_serializable() {
        let cumulative = SedimentSnapshot::cumulative_checkpoint(
            None,
            "a".to_string(),
            SedimentSnapshotProvenance::RuntimeCanonical,
            state(),
        );
        let daily = SedimentSnapshot::daily_contribution(
            "2026-08-01".to_string(),
            "b".to_string(),
            state(),
        );
        let derived =
            SedimentSnapshot::derived_preview("2026-08-01".to_string(), "c".to_string(), state());

        assert_eq!(cumulative.kind, SedimentSnapshotKind::CumulativeCheckpoint);
        assert_eq!(daily.kind, SedimentSnapshotKind::DailyContribution);
        assert_eq!(derived.kind, SedimentSnapshotKind::DerivedPreview);
        assert_ne!(
            serde_json::to_string(&cumulative).unwrap(),
            serde_json::to_string(&daily).unwrap()
        );
        assert_ne!(
            serde_json::to_string(&daily).unwrap(),
            serde_json::to_string(&derived).unwrap()
        );
    }

    #[test]
    fn bare_legacy_daily_state_is_cumulative_evidence() {
        let legacy = SedimentSnapshot::legacy_daily_payload("2026-08-01".to_string(), state());
        assert_eq!(legacy.kind, SedimentSnapshotKind::CumulativeCheckpoint);
        assert_eq!(
            legacy.provenance,
            SedimentSnapshotProvenance::LegacyDailyRow
        );
        assert!(!legacy.reconstructed);
    }

    #[test]
    fn authentic_day_end_wins_over_reconstructed_preview() {
        let authentic = SedimentSnapshot::day_end_checkpoint("2026-08-01".to_string(), state());
        let derived = derived_preview_from_slices("2026-08-01", 2, 2, &slices()).unwrap();
        let selected = select_historical_visual_artifact(
            "2026-08-01",
            Some(authentic.clone()),
            Some(derived.clone()),
        );
        assert_eq!(selected, Some(authentic));

        let wrong_day = SedimentSnapshot::day_end_checkpoint("2026-08-02".to_string(), state());
        let selected =
            select_historical_visual_artifact("2026-08-01", Some(wrong_day), Some(derived.clone()));
        assert_eq!(selected, Some(derived));
    }

    #[test]
    fn daily_contribution_is_canvas_independent_mass_evidence() {
        let snapshot = daily_contribution_from_slices("2026-08-01", &slices()).unwrap();
        let pending = snapshot
            .state
            .pending_runs
            .iter()
            .map(|run| run.count)
            .sum::<usize>();
        assert_eq!((snapshot.state.grid_width, snapshot.state.grid_height), (0, 0));
        assert!(snapshot.state.grains.is_empty());
        assert_eq!(pending, 9);
        assert_eq!(snapshot.state.pending_runs[0].category_id, 1);
        assert_eq!(snapshot.state.pending_runs[0].count, 5);
        assert_eq!(snapshot.state.pending_runs[1].category_id, 0);
        assert_eq!(snapshot.state.pending_runs[1].count, 4);
    }

    #[test]
    fn day_end_checkpoint_preserves_exact_canonical_state_and_dimensions() {
        let original = state();
        let snapshot =
            SedimentSnapshot::day_end_checkpoint("2026-08-01".to_string(), original.clone());
        assert!(snapshot.is_authentic_day_end_for("2026-08-01"));
        assert_eq!(snapshot.state, original);
        assert_eq!(snapshot.state.grid_width, 4);
        assert_eq!(snapshot.state.grid_height, 4);
        assert_eq!(snapshot.display_label(), "day-end checkpoint · idle included");
    }

    #[test]
    fn authentic_day_end_rendering_projects_without_mutating_original_canvas() {
        let snapshot = SedimentSnapshot::day_end_checkpoint("2026-08-01".to_string(), state());
        let before = snapshot.clone();

        let _small = snapshot.render_immutable(1, 1, &[]);
        let _large = snapshot.render_immutable(8, 6, &[]);

        assert_eq!(snapshot, before);
        assert_eq!((snapshot.state.grid_width, snapshot.state.grid_height), (4, 4));
        assert_eq!(snapshot.state.grains[0].x, 1);
        assert_eq!(snapshot.state.grains[0].y, 3);
    }

    #[test]
    fn repeated_rendering_is_immutable_and_deterministic() {
        let snapshot = SedimentSnapshot::derived_preview(
            "2026-08-01".to_string(),
            "ledger-revision".to_string(),
            state(),
        );
        let before = snapshot.clone();

        let first = snapshot.render_immutable(3, 2, &[]);
        let second = snapshot.render_immutable(3, 2, &[]);

        assert_eq!(first, second);
        assert_eq!(snapshot, before);
        assert_eq!(snapshot.state.frame_count, 13);
        assert!(!snapshot.state.sweep_left_to_right);
        assert_eq!(snapshot.state.rng_state, 77);
    }

    #[test]
    fn source_revision_changes_with_chronology_but_not_unrepresented_description() {
        let before = daily_contribution_from_slices("2026-08-01", &slices()).unwrap();
        let mut changed = slices();
        changed[0].elapsed_seconds += 1;
        changed[0].end_time = "09:00:06".to_string();
        let after = daily_contribution_from_slices("2026-08-01", &changed).unwrap();
        assert_ne!(before.source_revision, after.source_revision);
        assert_ne!(
            stable_source_revision(b"1:60:09:00:10:00"),
            stable_source_revision(b"1:61:09:00:10:01")
        );
    }
}
