from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label} anchor missing")
    return text.replace(old, new, 1)

# Install the one-transaction SQLite clear-all boundary.
tui_path = Path("src/sqlite/tui_runtime.rs")
tui = tui_path.read_text()
insert_anchor = "pub(crate) fn load_daily_snapshot(\n"
clear_fn = r'''pub(crate) fn clear_all_state<T: Serialize>(
    database_path: &Path,
    expected_active_stable_id: &str,
    resulting_active_stable_id: &str,
    resulting_started_at_utc: DateTime<Utc>,
    state: &SandState,
    daily_updates: &[(String, Option<SedimentSnapshot>)],
    detached_at_utc: DateTime<Utc>,
    simulation_time_utc: DateTime<Utc>,
    checkpoint: &T,
) -> Result<(), String> {
    runtime_coordination::maybe_inject_test_fault("clear-all", "before-write")
        .map_err(|error| error.to_string())?;
    if expected_active_stable_id.trim().is_empty() || resulting_active_stable_id.trim().is_empty() {
        return Err("clear-all requires non-empty active stable identities".to_string());
    }
    let mut repository = open_cli_repository(database_path)?;
    let payload_json = serde_json::to_string(state).map_err(|error| error.to_string())?;
    let checkpoint_json =
        serde_json::to_string(checkpoint).map_err(|error| error.to_string())?;
    let transaction = repository
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;

    let active: Option<(String, String, i64, String)> = transaction
        .query_row(
            "SELECT stable_id, project, category_id, description
             FROM active_session WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((actual_stable_id, _, _, _)) = active else {
        return Err("there is no active TUI session to clear".to_string());
    };
    if actual_stable_id != expected_active_stable_id {
        return Err(format!(
            "active session changed concurrently; expected {expected_active_stable_id}, found {actual_stable_id}"
        ));
    }

    let checkpoint_state: Option<(String, Option<String>)> = transaction
        .query_row(
            "SELECT status, active_session_stable_id
             FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if let Some((status, checkpoint_active)) = checkpoint_state {
        let replaceable = matches!(status.as_str(), "pending" | "committed")
            && checkpoint_active.as_deref() == Some(expected_active_stable_id);
        if !replaceable {
            let identity = checkpoint_active.as_deref().unwrap_or("missing");
            return Err(format!(
                "runtime checkpoint is {status} for {identity}; expected pending/committed for {expected_active_stable_id}"
            ));
        }
    }

    if resulting_active_stable_id != expected_active_stable_id {
        let changed = transaction
            .execute(
                "UPDATE active_session
                 SET stable_id = ?1, started_at_utc = ?2, recovery_kind = 'live'
                 WHERE singleton = 1 AND stable_id = ?3",
                params![
                    resulting_active_stable_id,
                    timestamp(resulting_started_at_utc),
                    expected_active_stable_id,
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("active session changed during clear-all".to_string());
        }
    }
    runtime_coordination::maybe_inject_test_fault("clear-all", "active")
        .map_err(|error| error.to_string())?;

    let existing_sand: Option<(String, i64)> = transaction
        .query_row(
            "SELECT formation_id, quantum_seconds FROM sand_state WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (formation_id, quantum_seconds) =
        existing_sand.unwrap_or_else(|| ("default".to_string(), 1));
    let captured_at_utc = timestamp(Utc::now());
    transaction
        .execute(
            "INSERT INTO sand_state (
                singleton, formation_id, quantum_seconds, grid_width, grid_height,
                payload_json, updated_at_utc, legacy_import_id
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, NULL)
             ON CONFLICT(singleton) DO UPDATE SET
                formation_id = excluded.formation_id,
                quantum_seconds = excluded.quantum_seconds,
                grid_width = excluded.grid_width,
                grid_height = excluded.grid_height,
                payload_json = excluded.payload_json,
                updated_at_utc = excluded.updated_at_utc,
                legacy_import_id = NULL",
            params![
                formation_id,
                quantum_seconds,
                i64::try_from(state.grid_width)
                    .map_err(|_| "sand width is too large".to_string())?,
                i64::try_from(state.grid_height)
                    .map_err(|_| "sand height is too large".to_string())?,
                payload_json,
                captured_at_utc,
            ],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("clear-all", "sand")
        .map_err(|error| error.to_string())?;

    for (operational_day, snapshot) in daily_updates {
        transaction
            .execute(
                "DELETE FROM sand_snapshots
                 WHERE snapshot_kind = 'daily-contribution' AND operational_day = ?1",
                params![operational_day],
            )
            .map_err(|error| error.to_string())?;
        if let Some(snapshot) = snapshot {
            let daily_json =
                serde_json::to_string(snapshot).map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO sand_snapshots (
                        formation_id, snapshot_kind, operational_day, quantum_seconds,
                        payload_json, captured_at_utc, legacy_import_id
                     ) VALUES (?1, 'daily-contribution', ?2, ?3, ?4, ?5, NULL)",
                    params![
                        formation_id,
                        operational_day,
                        quantum_seconds,
                        daily_json,
                        captured_at_utc,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    runtime_coordination::maybe_inject_test_fault("clear-all", "daily")
        .map_err(|error| error.to_string())?;

    transaction
        .execute(
            "INSERT INTO runtime_checkpoint (
                singleton, status, detached_at_utc, simulation_time_utc,
                active_session_stable_id, payload_json, legacy_import_id
             ) VALUES (1, 'pending', ?1, ?2, ?3, ?4, NULL)
             ON CONFLICT(singleton) DO UPDATE SET
                status = 'pending',
                detached_at_utc = excluded.detached_at_utc,
                simulation_time_utc = excluded.simulation_time_utc,
                active_session_stable_id = excluded.active_session_stable_id,
                payload_json = excluded.payload_json,
                legacy_import_id = NULL",
            params![
                timestamp(detached_at_utc),
                timestamp(simulation_time_utc),
                resulting_active_stable_id,
                checkpoint_json,
            ],
        )
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("clear-all", "checkpoint")
        .map_err(|error| error.to_string())?;
    runtime_coordination::maybe_inject_test_fault("clear-all", "commit")
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())
}

'''
if insert_anchor not in tui:
    raise SystemExit("daily snapshot anchor missing")
tui = tui.replace(insert_anchor, clear_fn + insert_anchor, 1)

tui_path.write_text(tui)
