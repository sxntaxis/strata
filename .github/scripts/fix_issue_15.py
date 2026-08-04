from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1, found {count}")
    return text.replace(old, new, 1)


# Correct the broad constructor transform and remove the obsolete atlas path editor.
path = Path("src/app.rs")
text = path.read_text()
text, count = re.subn(
    r"(struct DetachedRuntimeCheckpoint \{\n\s*schema_version: u8,\n)\s*profile_id: Some\(crate::profile::profile_id\(\)\),\n",
    r"\1",
    text,
    count=1,
)
if count != 1:
    raise SystemExit("checkpoint definition correction marker missing")
text = replace_once(
    text,
    "enum AtlasSelectable {\n    TimeLogPath,\n    WeekStartDay,",
    "enum AtlasSelectable {\n    WeekStartDay,",
    "atlas selectable path",
)
text = replace_once(
    text,
    "enum AtlasOverlay {\n    CaptureKey { action: keybindings::Action },\n    EditTimeLogPath { input: String },\n    SelectWeekStartDay",
    "enum AtlasOverlay {\n    CaptureKey { action: keybindings::Action },\n    SelectWeekStartDay",
    "atlas overlay path",
)
text = replace_once(
    text,
    "        let mut items = vec![AtlasSelectable::TimeLogPath, AtlasSelectable::WeekStartDay];",
    "        let mut items = vec![AtlasSelectable::WeekStartDay];",
    "atlas item list",
)
text = replace_once(
    text,
    "            AtlasSelectable::TimeLogPath => {\n                \"Path where session rows are written (time_log.csv).\".to_string()\n            }\n",
    "",
    "atlas path description",
)
text = replace_once(
    text,
    "            AtlasSelectable::TimeLogPath => Color::Cyan,\n",
    "",
    "atlas path color",
)
text = replace_once(
    text,
    "            AtlasSelectable::TimeLogPath => {\n                self.atlas_overlay = Some(AtlasOverlay::EditTimeLogPath {\n                    input: storage::get_time_log_path().display().to_string(),\n                });\n            }\n",
    "",
    "atlas path editor",
)
text = replace_once(
    text,
    "        set_runtime_settings(self.runtime_settings);\n        storage::set_runtime_storage_settings(storage::RuntimeStorageSettings {\n            time_log_path: loaded.time_log_path,\n        });",
    "        set_runtime_settings(self.runtime_settings);",
    "hot runtime storage reload",
)
path.write_text(text)

# Remove the obsolete atlas input route and handler.
path = Path("src/app/event_handlers.rs")
text = path.read_text()
text = replace_once(
    text,
    "            super::AtlasOverlay::EditTimeLogPath { .. } => {\n                self.handle_atlas_time_log_input(key);\n            }\n",
    "",
    "atlas overlay route",
)
text, count = re.subn(
    r"\n    fn handle_atlas_time_log_input\(&mut self, key: KeyEvent\) \{.*?\n    \}\n\n    fn handle_atlas_week_start_dropdown",
    "\n    fn handle_atlas_week_start_dropdown",
    text,
    count=1,
    flags=re.S,
)
if count != 1:
    raise SystemExit("atlas time-log handler marker missing")
path.write_text(text)

# Remove the atlas row and overlay renderer.
path = Path("src/app/keybindings_modal_view.rs")
text = path.read_text()
text, count = re.subn(
    r"\n            self\.selectable_row\(\n                AtlasSelectable::TimeLogPath,.*?\n            \),",
    "",
    text,
    count=1,
    flags=re.S,
)
if count != 1:
    raise SystemExit("atlas time-log row marker missing")
text, count = re.subn(
    r"\n            AtlasOverlay::EditTimeLogPath \{ input \} => \{.*?\n            \}\n            AtlasOverlay::SelectWeekStartDay",
    "\n            AtlasOverlay::SelectWeekStartDay",
    text,
    count=1,
    flags=re.S,
)
if count != 1:
    raise SystemExit("atlas time-log renderer marker missing")
path.write_text(text)

# Remove the superseded mutable runtime storage singleton left after path routing changed.
path = Path("src/storage.rs")
text = path.read_text()
text, count = re.subn(
    r"\npub fn runtime_storage_settings\(\) -> RuntimeStorageSettings \{.*?\n\}\n\npub fn set_runtime_storage_settings\(settings: RuntimeStorageSettings\) \{.*?\n\}\n",
    "\n",
    text,
    count=1,
    flags=re.S,
)
if count != 1:
    raise SystemExit("runtime storage functions marker missing")
path.write_text(text)

# PathBuf is no longer part of keymap authority.
path = Path("src/keybindings.rs")
text = path.read_text()
text = replace_once(text, "path::{Path, PathBuf}", "path::Path", "keymap path import")
path.write_text(text)

# Bind the recovery-export fixture to the selected profile.
path = Path("src/app/persistence_recovery.rs")
text = path.read_text()
text = replace_once(
    text,
    "        let statement = RecoveryStatement {\n            checkpoint_captured_at_utc: captured,",
    "        let statement = RecoveryStatement {\n            profile_id: crate::profile::profile_id(),\n            checkpoint_captured_at_utc: captured,",
    "recovery statement fixture",
)
path.write_text(text)
