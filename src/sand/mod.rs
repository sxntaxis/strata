mod engine;
mod recovery;
mod snapshot;

pub(crate) use engine::classic::{CLASSIC_PRODUCTION_AUTHORITY, ClassicSandboxEngine};
pub(crate) type ClassicProductionEngine = ClassicSandboxEngine;
#[allow(unused_imports)]
#[doc(hidden)]
pub use engine::ClassicRuntimeState;
#[cfg(debug_assertions)]
pub(crate) use engine::classic::ClassicRainMode;
#[cfg(debug_assertions)]
pub(crate) use engine::oslo_sandbox::{OsloBoundaryMode, OsloSandboxEngine};
pub(crate) use engine::recolor_state_category_mass;
pub use engine::{PendingGrainRun, SandEngine, SandState, SandStateCoordinate, SandStateGrain};
pub(crate) use recovery::{RecoveryTiming, recover_detached_sediment, settle_transition_sediment};
pub(crate) use snapshot::{
    DailySedimentSlice, daily_contribution_from_slices, derived_preview_from_slices,
    select_historical_visual_artifact,
};
#[allow(unused_imports)]
pub use snapshot::{
    SedimentIdlePolicy, SedimentSnapshot, SedimentSnapshotKind, SedimentSnapshotProvenance,
    stable_source_revision,
};
