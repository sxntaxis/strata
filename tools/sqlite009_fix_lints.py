from pathlib import Path

path = Path("src/app/persistence_recovery.rs")
text = path.read_text()
text = text.replace(
    "    domain::{CategoryId, DRIFT_CATEGORY_ID, is_drift_category_id, operational_day_key_now},",
    "    domain::{DRIFT_CATEGORY_ID, is_drift_category_id, operational_day_key_now},",
)
text = text.replace("    EmergencyExport,\n", "")
text = text.replace('            Self::EmergencyExport => "emergency recovery export",\n', "")
path.write_text(text)
