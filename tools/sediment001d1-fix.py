from pathlib import Path


module = Path("src/sand/mod.rs")
text = module.read_text()
old = """pub use snapshot::{
    SedimentIdlePolicy, SedimentSnapshot, SedimentSnapshotKind, SedimentSnapshotProvenance,
    stable_source_revision,
};
"""
new = """#[allow(unused_imports)]
pub use snapshot::{
    SedimentIdlePolicy, SedimentSnapshot, SedimentSnapshotKind, SedimentSnapshotProvenance,
    stable_source_revision,
};
"""
if text.count(old) != 1:
    raise SystemExit("snapshot re-export block did not match")
module.write_text(text.replace(old, new, 1))

app = Path("src/app.rs")
text = app.read_text()
text = text.replace("report_snapshot_state", "report_snapshot_artifact")
text = text.replace("report_snapshot_preview_engine", "report_snapshot_preview_lines")
app.write_text(text)

category_state = Path("src/app/category_state.rs")
text = category_state.read_text()
start = text.index("    pub(super) fn delete_daily_sand_snapshot(")
end = text.index("    pub(super) fn sync_modal_description_from_selection", start)
category_state.write_text(text[:start] + text[end:])

persistence = Path("src/app/persistence_recovery.rs")
text = persistence.read_text()
text = text.replace("    DailySnapshotDelete,\n", "", 1)
text = text.replace(
    '            Self::DailySnapshotDelete => "daily sediment snapshot deletion",\n',
    "",
    1,
)
persistence.write_text(text)

sqlite = Path("src/sqlite.rs")
text = sqlite.read_text()
text = text.replace(
    "    delete_daily_snapshot as delete_tui_daily_snapshot,\n",
    "",
    1,
)
sqlite.write_text(text)

snapshot = Path("src/sand/snapshot.rs")
text = snapshot.read_text()
needle = "    pub fn daily_contribution(\n"
if text.count(needle) != 1:
    raise SystemExit("daily contribution constructor was not found")
snapshot.write_text(text.replace(needle, "    #[cfg(test)]\n    pub fn daily_contribution(\n", 1))

Path(__file__).unlink(missing_ok=True)
