use std::collections::HashSet;

use ratatui::prelude::Line;
use serde::{Deserialize, Serialize};

use crate::domain::{Category, CategoryId};

use super::engine::{SandEngine, SandState};

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

impl SedimentSnapshot {
    pub const VERSION: u8 = 1;

    pub fn cumulative_checkpoint(
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

    #[cfg(test)]
    pub fn daily_contribution(
        operational_day: String,
        source_revision: String,
        provenance: SedimentSnapshotProvenance,
        idle_policy: SedimentIdlePolicy,
        reconstructed: bool,
        state: SandState,
    ) -> Self {
        Self {
            schema_version: Self::VERSION,
            kind: SedimentSnapshotKind::DailyContribution,
            operational_day: Some(operational_day),
            source_revision,
            provenance,
            idle_policy,
            reconstructed,
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

    pub fn legacy_daily_payload(operational_day: String, state: SandState) -> Self {
        let encoded = serde_json::to_vec(&state).unwrap_or_default();
        Self::cumulative_checkpoint(
            Some(operational_day),
            format!("legacy-{}", stable_source_revision(&encoded)),
            SedimentSnapshotProvenance::LegacyDailyRow,
            state,
        )
    }

    pub fn is_daily_contribution_for(&self, operational_day: &str) -> bool {
        self.schema_version == Self::VERSION
            && self.kind == SedimentSnapshotKind::DailyContribution
            && self.operational_day.as_deref() == Some(operational_day)
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
        let valid_category_ids = categories
            .iter()
            .map(|category| category.id)
            .collect::<HashSet<CategoryId>>();
        let mut engine = SandEngine::new(width, height);
        engine.restore_state(&self.state, &valid_category_ids);
        engine.resize(width, height);
        engine.render(categories)
    }

    pub fn display_label(&self) -> String {
        let kind = match self.kind {
            SedimentSnapshotKind::CumulativeCheckpoint => "cumulative checkpoint",
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

pub(crate) fn select_daily_artifact(
    operational_day: &str,
    persisted: Option<SedimentSnapshot>,
    derived: Option<SedimentSnapshot>,
) -> Option<SedimentSnapshot> {
    persisted
        .filter(|snapshot| snapshot.is_daily_contribution_for(operational_day))
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
        SedimentIdlePolicy, SedimentSnapshot, SedimentSnapshotKind, SedimentSnapshotProvenance,
        select_daily_artifact, stable_source_revision,
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
            SedimentSnapshotProvenance::SessionLedger,
            SedimentIdlePolicy::Included,
            false,
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
    fn cumulative_evidence_cannot_substitute_for_daily_contribution() {
        let legacy = SedimentSnapshot::legacy_daily_payload("2026-08-01".to_string(), state());
        let derived = SedimentSnapshot::derived_preview(
            "2026-08-01".to_string(),
            "ledger-revision".to_string(),
            state(),
        );

        let selected = select_daily_artifact("2026-08-01", Some(legacy), Some(derived.clone()));
        assert_eq!(selected, Some(derived));
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
    fn source_revision_changes_with_chronology_material() {
        let before = stable_source_revision(b"1:60:09:00:10:00");
        let after = stable_source_revision(b"1:61:09:00:10:01");
        assert_ne!(before, after);
    }
}
