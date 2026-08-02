from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:120]!r}")
    target.write_text(text.replace(old, new, 1))


# Narrow imports after generated integration.
category = Path("src/app/category_state.rs")
text = category.read_text()
text = text.replace(
    "    domain::{CategoryId, DRIFT_CATEGORY_ID, is_drift_category_id, operational_day_key_now},\n",
    "    domain::{CategoryId, DRIFT_CATEGORY_ID},\n",
    1,
)
category.write_text(text)

persistence = Path("src/app/persistence_recovery.rs")
text = persistence.read_text()
text = text.replace(
    "    domain::{DRIFT_CATEGORY_ID, is_drift_category_id, operational_day_key_now},\n",
    "    domain::{DRIFT_CATEGORY_ID, operational_day_key_now},\n",
    1,
)
persistence.write_text(text)

# Correct over-broad SQLite substitutions by function boundary.
tui = Path("src/sqlite/tui_runtime.rs")
text = tui.read_text()
start = text.index("pub(crate) fn save_sand_state(")
end = text.index("pub(crate) fn load_sand_state(", start)
block = text[start:end].replace(
    "serde_json::to_string(snapshot)", "serde_json::to_string(state)"
)
text = text[:start] + block + text[end:]
start = text.index("pub(crate) fn load_sand_state(")
end = text.index("pub(crate) fn save_daily_snapshot(", start)
block = text[start:end].replace(
    "Result<Option<SedimentSnapshot>, String>", "Result<Option<SandState>, String>"
)
text = text[:start] + block + text[end:]
start = text.index("pub(crate) fn save_daily_snapshot(")
end = text.index("pub(crate) fn load_daily_snapshot(", start)
block = text[start:end].replace(
    "serde_json::to_string(state)", "serde_json::to_string(snapshot)"
)
text = text[:start] + block + text[end:]
start = text.index("pub(crate) fn load_daily_snapshot(")
end = text.index("pub(crate) fn delete_daily_snapshot(", start)
block = text[start:end].replace(
    "Result<Option<SandState>, String>", "Result<Option<SedimentSnapshot>, String>"
)
text = text[:start] + block + text[end:]

# Complete tui_runtime tests and checkpoint calls.
text = text.replace(
    'commit_checkpoint_recovery(&path, "checkpoint-active", "2026-08-01", &state).unwrap();',
    'commit_checkpoint_recovery(&path, "checkpoint-active", "2026-08-01", &state, &daily)\n            .unwrap();',
    1,
)
tui.write_text(text)

# Re-export typed contribution deletion.
sqlite = Path("src/sqlite.rs")
text = sqlite.read_text()
old = "    commit_checkpoint_recovery as commit_tui_checkpoint_recovery,\n    delete_drift_sessions_for_day as delete_tui_drift_sessions_for_day,\n"
new = "    commit_checkpoint_recovery as commit_tui_checkpoint_recovery,\n    delete_daily_snapshot as delete_tui_daily_snapshot,\n    delete_drift_sessions_for_day as delete_tui_drift_sessions_for_day,\n"
if old not in text:
    raise SystemExit("SQLite tui re-export anchor not found")
sqlite.write_text(text.replace(old, new, 1))

# Update fault certification to typed daily fixtures.
fault = Path("src/sqlite/fault_certification.rs")
text = fault.read_text()
text = text.replace("    sand::SandState,", "    sand::{SandState, SedimentSnapshot},", 1)
anchor = "fn with_database(name: &str, action: impl FnOnce(&Path)) {\n"
helper = '''fn daily_snapshot(frame_count: usize) -> SedimentSnapshot {
    SedimentSnapshot::daily_contribution(
        "2026-08-01".to_string(),
        format!("revision-{frame_count}"),
        sand_state(frame_count),
    )
}

'''
if anchor not in text:
    raise SystemExit("fault helper anchor not found")
text = text.replace(anchor, helper + anchor, 1)
text = text.replace(
    'tui_runtime::save_daily_snapshot(path, "2026-08-01", &sand_state(1))',
    'tui_runtime::save_daily_snapshot(path, "2026-08-01", &daily_snapshot(1))',
)
text = text.replace(
    'tui_runtime::save_daily_snapshot(path, "2026-08-01", &sand_state(2))',
    'tui_runtime::save_daily_snapshot(path, "2026-08-01", &daily_snapshot(2))',
)
text = text.replace(
    ".load_daily_snapshot(path, \"2026-08-01\")\n                .unwrap()\n                .unwrap()\n                .frame_count",
    ".load_daily_snapshot(path, \"2026-08-01\")\n                .unwrap()\n                .unwrap()\n                .state\n                .frame_count",
)
text = text.replace(
    'tui_runtime::commit_checkpoint_recovery(path, "active-a", "2026-08-01", &sand_state(2))',
    'tui_runtime::commit_checkpoint_recovery(\n                path,\n                "active-a",\n                "2026-08-01",\n                &sand_state(2),\n                &daily_snapshot(2),\n            )',
    1,
)
# Direct coordination commit in checkpoint-clear fault proof.
text = text.replace(
    '            },\n            "2026-08-01T13:00:00Z",\n        )',
    '            },\n            &serde_json::to_string(&daily_snapshot(1)).unwrap(),\n            "2026-08-01T13:00:00Z",\n        )',
    1,
)
fault.write_text(text)

# Coordination unit tests now provide a typed daily payload JSON argument.
coord = Path("src/sqlite/runtime_coordination.rs")
text = coord.read_text()
# Test calls use local state followed immediately by capture timestamp.
text = text.replace(
    '                &state,\n                "2026-08-01T11:00:00Z",',
    '                &state,\n                "{\\"schema_version\\":1,\\"kind\\":\\"daily-contribution\\"}",\n                "2026-08-01T11:00:00Z",',
)
text = text.replace(
    '            &state,\n            "2026-08-01T11:00:00Z",',
    '            &state,\n            "{\\"schema_version\\":1,\\"kind\\":\\"daily-contribution\\"}",\n            "2026-08-01T11:00:00Z",',
)
coord.write_text(text)

Path(__file__).unlink(missing_ok=True)
