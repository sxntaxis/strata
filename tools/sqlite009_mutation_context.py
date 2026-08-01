from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


path = Path("src/app/category_state.rs")
text = path.read_text()
text = replace_once(
    text,
    "use super::App;",
    "use super::{App, PersistenceOperation, RecoveryAction};",
    "category recovery imports",
)
text = replace_once(
    text,
    "            if let Some(archived) = self.record_storage_result(result) {",
    "            if let Some(archived) = self.record_storage_result_for(\n                PersistenceOperation::CategorySync,\n                RecoveryAction::FlushCurrentState,\n                result,\n            ) {",
    "category sync context",
)
text = replace_once(
    text,
    "            self.record_storage_result(result);\n        } else {\n            let categories = self.time_tracker.categories_for_storage();",
    "            self.record_storage_result_for(\n                PersistenceOperation::SessionSync,\n                RecoveryAction::FlushCurrentState,\n                result,\n            );\n        } else {\n            let categories = self.time_tracker.categories_for_storage();",
    "session sync context",
)
text = replace_once(
    text,
    "            let result = sqlite::save_tui_sand_state(&database_path, &state);\n            self.record_storage_result(result);",
    "            let result = sqlite::save_tui_sand_state(&database_path, &state);\n            self.record_storage_result_for(\n                PersistenceOperation::SandStateSave,\n                RecoveryAction::FlushCurrentState,\n                result,\n            );",
    "sand state context",
)
text = replace_once(
    text,
    "            let result =\n                sqlite::sync_tui_category_tags(&database_path, &self.category_tags, &category_ids);\n            self.record_storage_result(result);",
    "            let result =\n                sqlite::sync_tui_category_tags(&database_path, &self.category_tags, &category_ids);\n            self.record_storage_result_for(\n                PersistenceOperation::CategoryTagsSync,\n                RecoveryAction::FlushCurrentState,\n                result,\n            );",
    "category tags context",
)
text = replace_once(
    text,
    "                    self.record_storage_result::<()>(Err(error));\n                    return;",
    "                    self.record_storage_result_for::<()>(\n                        PersistenceOperation::StateReload,\n                        RecoveryAction::ReloadAuthority,\n                        Err(error),\n                    );\n                    return;",
    "sand state load context",
)
text = replace_once(
    text,
    "                    self.record_storage_result::<()>(Err(error));\n                    None",
    "                    self.record_storage_result_for::<()>(\n                        PersistenceOperation::StateReload,\n                        RecoveryAction::ReloadAuthority,\n                        Err(error),\n                    );\n                    None",
    "daily snapshot load context",
)
text = replace_once(
    text,
    "            let result = sqlite::save_tui_daily_snapshot(&database_path, &day, state);\n            self.record_storage_result(result);",
    "            let result = sqlite::save_tui_daily_snapshot(&database_path, &day, state);\n            self.record_storage_result_for(\n                PersistenceOperation::DailySnapshotSave,\n                RecoveryAction::FlushCurrentState,\n                result,\n            );",
    "daily snapshot save context",
)
text = replace_once(
    text,
    "            let result = sqlite::delete_tui_daily_snapshot(&database_path, &day);\n            self.record_storage_result(result);",
    "            let result = sqlite::delete_tui_daily_snapshot(&database_path, &day);\n            self.record_storage_result_for(\n                PersistenceOperation::DailySnapshotDelete,\n                RecoveryAction::FlushCurrentState,\n                result,\n            );",
    "daily snapshot delete context",
)
text = replace_once(
    text,
    "                let result = sqlite::archive_tui_category(&database_path, category_id);\n                if self.record_storage_result(result).is_none() {",
    "                let result = sqlite::archive_tui_category(&database_path, category_id);\n                if self\n                    .record_storage_result_for(\n                        PersistenceOperation::CategoryArchive,\n                        RecoveryAction::ReloadAuthority,\n                        result,\n                    )\n                    .is_none()\n                {",
    "category archive context",
)
path.write_text(text)


path = Path("src/app/report_state.rs")
text = path.read_text()
text = replace_once(
    text,
    "use super::App;",
    "use super::{App, PersistenceOperation, RecoveryAction};",
    "report recovery imports",
)
text = replace_once(
    text,
    "            let result = crate::sqlite::delete_tui_session(&database_path, session_id);\n            if self.record_storage_result(result).is_none() {",
    "            let result = crate::sqlite::delete_tui_session(&database_path, session_id);\n            if self\n                .record_storage_result_for(\n                    PersistenceOperation::SessionDelete,\n                    RecoveryAction::ReloadAuthority,\n                    result,\n                )\n                .is_none()\n            {",
    "session deletion context",
)
text = replace_once(
    text,
    "            if self.record_storage_result(result).is_none() {\n                return false;\n            }",
    "            if self\n                .record_storage_result_for(\n                    PersistenceOperation::SessionEdit,\n                    RecoveryAction::ReloadAuthority,\n                    result,\n                )\n                .is_none()\n            {\n                return false;\n            }",
    "session edit context",
)
path.write_text(text)


# Stop chained modal mutations as soon as a persistence failure has opened recovery.
path = Path("src/app/event_handlers.rs")
text = path.read_text()
text = replace_once(
    text,
    "                    self.persist_categories();\n                }",
    "                    self.persist_categories();\n                    if self.has_persistence_recovery() {\n                        return true;\n                    }\n                }",
    "category reorder up recovery stop",
)
text = replace_once(
    text,
    "                    self.persist_categories();\n                }",
    "                    self.persist_categories();\n                    if self.has_persistence_recovery() {\n                        return true;\n                    }\n                }",
    "category reorder down recovery stop",
)
# Confirm path: do not add tags or switch after category description persistence fails.
text = replace_once(
    text,
    "                            self.persist_categories();\n                        }\n                        self.remember_selected_tag();",
    "                            self.persist_categories();\n                            if self.has_persistence_recovery() {\n                                self.render_needed = true;\n                                return true;\n                            }\n                        }\n                        self.remember_selected_tag();\n                        if self.has_persistence_recovery() {\n                            self.render_needed = true;\n                            return true;\n                        }",
    "category confirmation recovery stop",
)
path.write_text(text)
