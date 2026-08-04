use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::sand::{SandState, SedimentSnapshot, stable_source_revision};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SedimentCategoryReferences {
    pub placed: u64,
    pub pending: u64,
}

impl SedimentCategoryReferences {
    pub(crate) fn total(self) -> Result<u64, String> {
        self.placed
            .checked_add(self.pending)
            .ok_or_else(|| "category sediment reference count exceeds u64".to_string())
    }
}

pub(crate) fn count_sand_state_category(
    state: &SandState,
    source_category_id: u64,
) -> Result<SedimentCategoryReferences, String> {
    let placed = u64::try_from(
        state
            .grains
            .iter()
            .filter(|grain| grain.category_id == source_category_id)
            .count(),
    )
    .map_err(|_| "placed sediment reference count exceeds u64".to_string())?;

    let legacy_pending = u64::try_from(
        state
            .pending_grains
            .iter()
            .filter(|category_id| **category_id == source_category_id)
            .count(),
    )
    .map_err(|_| "legacy pending sediment count exceeds u64".to_string())?;
    let compressed_pending = state
        .pending_runs
        .iter()
        .filter(|run| run.category_id == source_category_id)
        .try_fold(0u64, |total, run| {
            let count = u64::try_from(run.count)
                .map_err(|_| "compressed pending sediment count exceeds u64".to_string())?;
            total
                .checked_add(count)
                .ok_or_else(|| "pending sediment reference count exceeds u64".to_string())
        })?;

    Ok(SedimentCategoryReferences {
        placed,
        pending: legacy_pending
            .checked_add(compressed_pending)
            .ok_or_else(|| "pending sediment reference count exceeds u64".to_string())?,
    })
}

pub(crate) fn reassign_sand_state_category(
    state: &mut SandState,
    source_category_id: u64,
    target_category_id: u64,
) -> Result<SedimentCategoryReferences, String> {
    if source_category_id == target_category_id {
        return Err("category reassignment source and target are identical".to_string());
    }
    let references = count_sand_state_category(state, source_category_id)?;
    for grain in &mut state.grains {
        if grain.category_id == source_category_id {
            grain.category_id = target_category_id;
        }
    }
    for category_id in &mut state.pending_grains {
        if *category_id == source_category_id {
            *category_id = target_category_id;
        }
    }
    for run in &mut state.pending_runs {
        if run.category_id == source_category_id {
            run.category_id = target_category_id;
        }
    }
    coalesce_pending_runs(state)?;
    Ok(references)
}

fn coalesce_pending_runs(state: &mut SandState) -> Result<(), String> {
    let mut compacted: Vec<crate::sand::PendingGrainRun> =
        Vec::with_capacity(state.pending_runs.len());
    for run in state.pending_runs.drain(..) {
        if run.count == 0 {
            continue;
        }
        if let Some(previous) = compacted.last_mut()
            && previous.category_id == run.category_id
        {
            previous.count = previous
                .count
                .checked_add(run.count)
                .ok_or_else(|| "coalesced pending sediment run exceeds usize".to_string())?;
        } else {
            compacted.push(run);
        }
    }
    state.pending_runs = compacted;
    Ok(())
}

pub(crate) fn count_snapshot_category(
    snapshot: &SedimentSnapshot,
    source_category_id: u64,
) -> Result<SedimentCategoryReferences, String> {
    count_sand_state_category(&snapshot.state, source_category_id)
}

pub(crate) fn reassign_snapshot_category(
    snapshot: &mut SedimentSnapshot,
    source_category_id: u64,
    target_category_id: u64,
) -> Result<SedimentCategoryReferences, String> {
    let references =
        reassign_sand_state_category(&mut snapshot.state, source_category_id, target_category_id)?;
    if references.total()? > 0 {
        snapshot.source_revision.clear();
        let material = serde_json::to_vec(snapshot)
            .map_err(|error| format!("cannot revise reassigned snapshot provenance: {error}"))?;
        snapshot.source_revision = format!(
            "category-reassignment-{}",
            stable_source_revision(&material)
        );
    }
    Ok(references)
}

pub(crate) fn count_checkpoint_category_references(
    payload_json: &str,
    source_category_id: u64,
) -> Result<u64, String> {
    let mut value: Value = serde_json::from_str(payload_json)
        .map_err(|error| format!("invalid runtime checkpoint JSON: {error}"))?;
    validate_checkpoint_shape(&value)?;
    let mut references = 0u64;
    visit_checkpoint_value(
        &mut value,
        None,
        source_category_id,
        source_category_id,
        &mut references,
    )?;
    Ok(references)
}

pub(crate) fn reassign_checkpoint_category(
    payload_json: &str,
    source_category_id: u64,
    target_category_id: u64,
) -> Result<(String, u64), String> {
    if source_category_id == target_category_id {
        return Err("category reassignment source and target are identical".to_string());
    }
    let mut value: Value = serde_json::from_str(payload_json)
        .map_err(|error| format!("invalid runtime checkpoint JSON: {error}"))?;
    validate_checkpoint_shape(&value)?;
    if checkpoint_has_transition_receipt_value(&value)? {
        return Err(
            "runtime checkpoint carries an unresolved transition receipt and cannot be category-reassigned"
                .to_string(),
        );
    }
    let mut references = 0u64;
    visit_checkpoint_value(
        &mut value,
        None,
        source_category_id,
        target_category_id,
        &mut references,
    )?;
    let encoded = serde_json::to_string(&value)
        .map_err(|error| format!("cannot serialize reassigned runtime checkpoint: {error}"))?;
    Ok((encoded, references))
}

pub(crate) fn checkpoint_has_transition_receipt(payload_json: &str) -> Result<bool, String> {
    let value: Value = serde_json::from_str(payload_json)
        .map_err(|error| format!("invalid runtime checkpoint JSON: {error}"))?;
    validate_checkpoint_shape(&value)?;
    checkpoint_has_transition_receipt_value(&value)
}

fn validate_checkpoint_shape(value: &Value) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| "runtime checkpoint payload is not a JSON object".to_string())?;
    let version = object
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| "runtime checkpoint has no numeric schema_version".to_string())?;
    if !(1..=3).contains(&version) {
        return Err(format!(
            "runtime checkpoint schema version {version} is unsupported for category reassignment"
        ));
    }
    Ok(())
}

fn checkpoint_has_transition_receipt_value(value: &Value) -> Result<bool, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "runtime checkpoint payload is not a JSON object".to_string())?;
    Ok(["legacy_transition", "legacy_finish", "clear_all"]
        .iter()
        .any(|field| object.get(*field).is_some_and(|value| !value.is_null())))
}

fn visit_checkpoint_value(
    value: &mut Value,
    field_name: Option<&str>,
    source_category_id: u64,
    target_category_id: u64,
    references: &mut u64,
) -> Result<(), String> {
    if field_name == Some("pending_grains") {
        let pending = value
            .as_array_mut()
            .ok_or_else(|| "checkpoint pending_grains is not an array".to_string())?;
        for category in pending {
            remap_numeric_category(
                category,
                source_category_id,
                target_category_id,
                references,
                "pending_grains entry",
            )?;
        }
        return Ok(());
    }

    if field_name.is_some_and(is_category_identity_field) {
        return remap_numeric_category(
            value,
            source_category_id,
            target_category_id,
            references,
            field_name.unwrap_or("category identity"),
        );
    }

    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                visit_checkpoint_value(
                    nested,
                    Some(key),
                    source_category_id,
                    target_category_id,
                    references,
                )?;
            }
        }
        Value::Array(values) => {
            for nested in values {
                visit_checkpoint_value(
                    nested,
                    None,
                    source_category_id,
                    target_category_id,
                    references,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_category_identity_field(field_name: &str) -> bool {
    field_name == "category_id" || field_name.ends_with("_category_id")
}

fn remap_numeric_category(
    value: &mut Value,
    source_category_id: u64,
    target_category_id: u64,
    references: &mut u64,
    label: &str,
) -> Result<(), String> {
    let category_id = value
        .as_u64()
        .ok_or_else(|| format!("checkpoint {label} is not an unsigned category identity"))?;
    if category_id == source_category_id {
        *references = references
            .checked_add(1)
            .ok_or_else(|| "checkpoint category reference count exceeds u64".to_string())?;
        *value = Value::from(target_category_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::sand::{PendingGrainRun, SandStateGrain};

    fn state() -> SandState {
        SandState {
            version: SandState::VERSION,
            grid_width: 2,
            grid_height: 2,
            grains: vec![
                SandStateGrain {
                    x: 0,
                    y: 0,
                    category_id: 1,
                },
                SandStateGrain {
                    x: 1,
                    y: 0,
                    category_id: 2,
                },
            ],
            frame_count: 0,
            sweep_left_to_right: true,
            rng_state: 1,
            pending_grains: vec![1],
            pending_runs: vec![
                PendingGrainRun {
                    category_id: 2,
                    count: 3,
                },
                PendingGrainRun {
                    category_id: 1,
                    count: 4,
                },
                PendingGrainRun {
                    category_id: 2,
                    count: 5,
                },
            ],
        }
    }

    #[test]
    fn sand_reassignment_conserves_mass_and_coalesces_fifo_runs() {
        let mut state = state();
        let before = state.grains.len()
            + state.pending_grains.len()
            + state
                .pending_runs
                .iter()
                .map(|run| run.count)
                .sum::<usize>();
        let references = reassign_sand_state_category(&mut state, 1, 2).unwrap();
        assert_eq!(references.placed, 1);
        assert_eq!(references.pending, 5);
        let after = state.grains.len()
            + state.pending_grains.len()
            + state
                .pending_runs
                .iter()
                .map(|run| run.count)
                .sum::<usize>();
        assert_eq!(before, after);
        assert!(state.grains.iter().all(|grain| grain.category_id == 2));
        assert!(state.pending_grains.iter().all(|category| *category == 2));
        assert_eq!(state.pending_runs.len(), 1);
        assert_eq!(state.pending_runs[0].category_id, 2);
        assert_eq!(state.pending_runs[0].count, 12);
    }

    #[test]
    fn checkpoint_reassignment_covers_current_identity_paths() {
        let payload = json!({
            "schema_version": 3,
            "active_category_id": 1,
            "sand_state": {
                "version": 2,
                "grid_width": 1,
                "grid_height": 1,
                "grains": [{"x": 0, "y": 0, "category_id": 1}],
                "pending_grains": [1],
                "pending_runs": [{"category_id": 1, "count": 5}]
            },
            "pending_mutations": [
                {"SwitchLayer": {"category_id": 1}},
                "ClearAllSand"
            ],
            "legacy_transition": null,
            "legacy_finish": null,
            "clear_all": null
        })
        .to_string();
        assert_eq!(
            count_checkpoint_category_references(&payload, 1).unwrap(),
            5
        );
        let (updated, changed) = reassign_checkpoint_category(&payload, 1, 2).unwrap();
        assert_eq!(changed, 5);
        assert_eq!(
            count_checkpoint_category_references(&updated, 1).unwrap(),
            0
        );
        assert_eq!(
            count_checkpoint_category_references(&updated, 2).unwrap(),
            5
        );
    }

    #[test]
    fn receipt_bearing_checkpoint_fails_closed() {
        let payload = json!({
            "schema_version": 3,
            "active_category_id": 1,
            "sand_state": {
                "version": 2,
                "grid_width": 1,
                "grid_height": 1,
                "grains": [],
                "pending_grains": [],
                "pending_runs": []
            },
            "pending_mutations": [],
            "legacy_transition": {"expected_previous_category_id": 1},
            "legacy_finish": null,
            "clear_all": null
        })
        .to_string();
        assert!(checkpoint_has_transition_receipt(&payload).unwrap());
        assert!(
            reassign_checkpoint_category(&payload, 1, 2)
                .unwrap_err()
                .contains("unresolved transition receipt")
        );
    }
}
