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

Path(__file__).unlink(missing_ok=True)
