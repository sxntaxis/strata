from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]

def load(path):
    return (ROOT / path).read_text()

def save(path, text):
    (ROOT / path).write_text(text)

def rep(text, old, new, label, count=1):
    actual = text.count(old)
    if actual != count:
        raise SystemExit(f"{label}: expected {count}, found {actual}")
    return text.replace(old, new)

def sub(text, pattern, repl, label, count=1, flags=0):
    text2, actual = re.subn(pattern, repl, text, count=count, flags=flags)
    if actual != count:
        raise SystemExit(f"{label}: expected {count}, found {actual}")
    return text2

# Domain: the active draft belongs to the active session, never the category catalog.
p = "src/domain.rs"
s = load(p)
s = rep(s,
"    active_category_id: CategoryId,\n}",
"    active_category_id: CategoryId,\n    active_description: String,\n}", "tracker field")
s = rep(s,
"            active_category_id: DRIFT_CATEGORY_ID,\n        }",
"            active_category_id: DRIFT_CATEGORY_ID,\n            active_description: String::new(),\n        }", "tracker init")
s = rep(s,
"            self.active_category_id = DRIFT_CATEGORY_ID;\n        }",
"            self.active_category_id = DRIFT_CATEGORY_ID;\n            self.active_description.clear();\n        }", "loaded invalid active")
s = rep(s,
"    pub fn active_category_id(&self) -> CategoryId {\n        self.active_category_id\n    }\n",
"    pub fn active_category_id(&self) -> CategoryId {\n        self.active_category_id\n    }\n\n    pub fn active_description(&self) -> &str {\n        &self.active_description\n    }\n\n    pub fn set_active_description(&mut self, description: String) {\n        self.active_description = description;\n    }\n", "active description API")
s = rep(s,
"                self.active_category_id = DRIFT_CATEGORY_ID;\n            }",
"                self.active_category_id = DRIFT_CATEGORY_ID;\n                self.active_description.clear();\n            }", "deleted active")
s = sub(s,
r"        let cat_id = self\.active_category_id;\n        let cat_description = self\n            \.category_store\n            \.get_by_id\(cat_id\)\n            \.map\(\|category\| category\.description\.clone\(\)\)\n            \.unwrap_or_default\(\);",
"        let cat_id = self.active_category_id;\n        let active_description = self.active_description.clone();", "finish reads active draft")
s = rep(s,
"            self.record_session_at(cat_id, &cat_description, elapsed, end_local);",
"            self.record_session_at(cat_id, &active_description, elapsed, end_local);", "finish records active draft")
s = sub(s,
r"\n        if let Some\(category\) = self\.category_store\.get_mut_by_id\(cat_id\) \{\n            category\.description\.clear\(\);\n        \}\n\n        self\.current_session_start = None;",
"\n        self.active_description.clear();\n        self.current_session_start = None;", "finish clears draft")
# Proof that metadata and draft are independent.
anchor = "    #[test]\n    fn test_category_id_new()"
proof = '''    #[test]
    fn active_draft_is_independent_from_category_metadata() {
        let mut tracker = TimeTracker::new();
        let id = tracker
            .add_category("Work".to_string(), "Stable metadata".to_string(), None)
            .expect("category should be created");
        assert!(tracker.set_active_category_by_id(id));
        tracker.set_active_description("One-shot draft".to_string());
        assert_eq!(tracker.active_description(), "One-shot draft");
        assert_eq!(tracker.category_description_by_id(id), Some("Stable metadata"));
        tracker.start_session();
        tracker
            .end_session_with_elapsed_at_local(1, Local::now())
            .expect("active session should finish");
        assert_eq!(tracker.sessions.last().unwrap().description, "One-shot draft");
        assert_eq!(tracker.active_description(), "");
        assert_eq!(tracker.category_description_by_id(id), Some("Stable metadata"));
    }

'''
if anchor not in s:
    raise SystemExit("domain test anchor missing")
s = s.replace(anchor, proof + anchor, 1)
save(p, s)

# Keymap: explicit modal mode for editing durable category metadata.
p = "src/keybindings.rs"
s = load(p)
s = rep(s, "    CategoryLifecycle,\n    IncreaseKarma,", "    CategoryLifecycle,\n    EditCategoryDescription,\n    IncreaseKarma,", "action enum")
s = rep(s, "const ALL: [Action; 29]", "const ALL: [Action; 30]", "action count")
s = rep(s, "        Action::CategoryLifecycle,\n        Action::IncreaseKarma,", "        Action::CategoryLifecycle,\n        Action::EditCategoryDescription,\n        Action::IncreaseKarma,", "all action")
s = rep(s, "            Action::CategoryLifecycle => \"category_lifecycle\",\n            Action::IncreaseKarma", "            Action::CategoryLifecycle => \"category_lifecycle\",\n            Action::EditCategoryDescription => \"edit_layer_metadata\",\n            Action::IncreaseKarma", "config name")
s = rep(s, "            \"boost_layer_karma\" | \"increase_karma\" => Some(Self::IncreaseKarma),", "            \"edit_layer_metadata\" | \"edit_category_description\" => {\n                Some(Self::EditCategoryDescription)\n            }\n            \"boost_layer_karma\" | \"increase_karma\" => Some(Self::IncreaseKarma),", "parse action")
s = rep(s, "            Action::CategoryLifecycle => \"Merge or permanently delete selected layer\",\n            Action::IncreaseKarma", "            Action::CategoryLifecycle => \"Merge or permanently delete selected layer\",\n            Action::EditCategoryDescription => \"Toggle durable layer-metadata editing\",\n            Action::IncreaseKarma", "action description")
s = rep(s, "            | Action::CategoryLifecycle\n            | Action::IncreaseKarma", "            | Action::CategoryLifecycle\n            | Action::EditCategoryDescription\n            | Action::IncreaseKarma", "action category")
s = rep(s, "const DEFAULT_BINDINGS: [(&str, Action); 31]", "const DEFAULT_BINDINGS: [(&str, Action); 32]", "binding count")
s = rep(s, "    (\"shift-x\", Action::CategoryLifecycle),\n    (\"+\", Action::IncreaseKarma),", "    (\"shift-x\", Action::CategoryLifecycle),\n    (\"shift-e\", Action::EditCategoryDescription),\n    (\"+\", Action::IncreaseKarma),", "metadata binding")
save(p, s)

# App state, receipts, queue and runtime transitions.
p = "src/app.rs"
s = load(p)
s = rep(s,
"enum QueuedMutation {\n    SwitchLayer(CategoryId),",
"enum QueuedMutation {\n    SwitchLayer {\n        category_id: CategoryId,\n        description: String,\n    },", "queued switch")
s = rep(s,
"    SwitchLayer { category_id: u64 },",
"    SwitchLayer {\n        category_id: u64,\n        #[serde(default)]\n        description: String,\n    },", "queued record")
s = rep(s,
"    modal_description: String,\n    category_tags:",
"    modal_description: String,\n    modal_editing_category_metadata: bool,\n    category_tags:", "modal mode field")
s = rep(s,
"            modal_description: String::new(),\n            category_tags,",
"            modal_description: String::new(),\n            modal_editing_category_metadata: false,\n            category_tags,", "modal mode init")
s = rep(s,
"        self.modal_description = String::new();\n        self.modal_tag_index = None;",
"        self.modal_description = String::new();\n        self.modal_editing_category_metadata = false;\n        self.modal_tag_index = None;", "modal close", count=1)
# Active state restore/start uses session draft.
s = rep(s,
"                let _ = app\n                    .time_tracker\n                    .set_category_description_by_id(active.category_id, active.description);",
"                app.time_tracker.set_active_description(active.description);", "startup active draft")
s = sub(s,
r"        let description = self\n            \.time_tracker\n            \.category_description_by_id\(category_id\)\n            \.unwrap_or_default\(\)\n            \.to_string\(\);",
"        let description = self.time_tracker.active_description().to_string();", "initial active draft")
# Clear replay owns active draft.
s = sub(s,
r"    if !tracker\.set_category_description_by_id\(\n        resulting_category_id,\n        receipt\.resulting_active\.description\.clone\(\),\n    \) \{\n        return Err\(format!\(\n            \"clear-all receipt \{\} cannot restore its resulting description\",\n            receipt\.operation_id\n        \)\);\n    \}",
"    tracker.set_active_description(receipt.resulting_active.description.clone());", "clear replay draft")
# Legacy switch replay no longer mutates catalog descriptions.
s = sub(s,
r"    let previous_category_id = CategoryId::new\(receipt\.expected_previous_category_id\);\n    if !staged_tracker\.set_category_description_by_id\(previous_category_id, String::new\(\)\) \{.*?\n    \}\n    let resulting_category_id = CategoryId::new\(receipt\.resulting_active\.category_id\);\n    if !staged_tracker\.set_category_description_by_id\(\n        resulting_category_id,\n        receipt\.resulting_active\.description\.clone\(\),\n    \) \{.*?\n    \}",
"    let resulting_category_id = CategoryId::new(receipt.resulting_active.category_id);\n    if !staged_tracker.set_active_category_by_id(resulting_category_id) {\n        return Err(format!(\n            \"legacy switch receipt {} references unavailable resulting category {}\",\n            receipt.operation_id, receipt.resulting_active.category_id\n        ));\n    }\n    staged_tracker.set_active_description(receipt.resulting_active.description.clone());", "legacy switch replay", flags=re.S)
s = sub(s,
r"    let previous_category_id = CategoryId::new\(receipt\.expected_previous_category_id\);\n    if !staged_tracker\.set_category_description_by_id\(previous_category_id, String::new\(\)\) \{.*?\n    \}\n    let mut catalog",
"    staged_tracker.set_active_description(String::new());\n    let mut catalog", "legacy finish replay", flags=re.S)
# Finish reads active draft; no category clearing/persistence.
s = sub(s,
r"        let previous_description = self\n            \.time_tracker\n            \.category_description_by_id\(previous_category_id\)\n            \.unwrap_or_default\(\)\n            \.to_string\(\);",
"        let previous_description = self.time_tracker.active_description().to_string();", "legacy finish description")
s = sub(s,
r"            let active_category_id = self\.time_tracker\.active_category_id\(\);\n            let _ = self\n                \.time_tracker\n                \.set_category_description_by_id\(active_category_id, String::new\(\)\);\n",
"            self.time_tracker.set_active_description(String::new());\n", "sqlite finish clears draft")
s = rep(s, "            self.persist_categories();\n            return Some(elapsed);", "            return Some(elapsed);", "no category write on finish")
# Switch signature and implementation.
s = rep(s,
"    fn switch_active_category_at(\n        &mut self,\n        category_id: CategoryId,\n        switched_at_utc:",
"    fn switch_active_category_at(\n        &mut self,\n        category_id: CategoryId,\n        next_description: String,\n        switched_at_utc:", "switch signature")
s = sub(s,
r"            let next_description = self\n                \.time_tracker\n                \.category_description_by_id\(category_id\)\n                \.unwrap_or_default\(\)\n                \.to_string\(\);\n",
"", "remove derived next draft")
s = sub(s,
r"            let previous_category_id = self\.time_tracker\.active_category_id\(\);\n            let _ = self\n                \.time_tracker\n                \.set_category_description_by_id\(previous_category_id, String::new\(\)\);\n            if !self\.time_tracker\.set_active_category_by_id\(category_id\) \{",
"            if !self.time_tracker.set_active_category_by_id(category_id) {", "sqlite switch no metadata clear")
s = rep(s,
"            self.session.active_session_stable_id = receipt.resulting_active_stable_id;",
"            self.time_tracker.set_active_description(next_description);\n            self.session.active_session_stable_id = receipt.resulting_active_stable_id;", "sqlite next draft")
s = rep(s, "            self.persist_categories();\n            self.sync_drift_idle_state();", "            self.sync_drift_idle_state();", "no category write on switch")
s = sub(s,
r"        let resulting_description = self\n            \.time_tracker\n            \.category_description_by_id\(category_id\)\n            \.unwrap_or_default\(\)\n            \.to_string\(\);",
"        self.time_tracker.set_active_description(next_description.clone());\n        let resulting_description = next_description;", "legacy next draft")
# Clear receipt/checkpoint/report use active draft.
s = sub(s,
r"            description: self\n                \.time_tracker\n                \.category_description_by_id\(self\.time_tracker\.active_category_id\(\)\)\n                \.unwrap_or_default\(\)\n                \.to_string\(\),",
"            description: self.time_tracker.active_description().to_string(),", "clear active draft")
s = sub(s,
r"        let active_description = self\n            \.time_tracker\n            \.category_description_by_id\(active_category_id\)\n            \.unwrap_or_default\(\)\n            \.to_string\(\);",
"        let active_description = self.time_tracker.active_description().to_string();", "checkpoint active draft")
s = rep(s,
"        let _ = self.time_tracker.set_category_description_by_id(\n            active_category_id,\n            checkpoint.active_description.clone(),\n        );",
"        self.time_tracker\n            .set_active_description(checkpoint.active_description.clone());", "recovery active draft")
# Queue carries the exact draft chosen at intent time.
s = rep(s,
"            QueuedMutation::SwitchLayer(category_id) => {\n                self.apply_switch_layer_at(category_id, scheduled_at_utc, clock_mode);\n            }",
"            QueuedMutation::SwitchLayer {\n                category_id,\n                description,\n            } => {\n                self.apply_switch_layer_at(category_id, description, scheduled_at_utc, clock_mode);\n            }", "apply queued switch")
s = rep(s,
"    fn apply_switch_layer_at(\n        &mut self,\n        category_id: CategoryId,\n        scheduled_at_utc:",
"    fn apply_switch_layer_at(\n        &mut self,\n        category_id: CategoryId,\n        description: String,\n        scheduled_at_utc:", "apply switch signature")
s = rep(s,
"        self.switch_active_category_at(category_id, scheduled_at_utc, clock_mode);",
"        self.switch_active_category_at(category_id, description, scheduled_at_utc, clock_mode);", "apply switch call")
# Emergency serialization.
s = rep(s,
"                    QueuedMutation::SwitchLayer(category_id) => QueuedMutationRecord::SwitchLayer {\n                        category_id: category_id.0,\n                    },",
"                    QueuedMutation::SwitchLayer {\n                        category_id,\n                        ref description,\n                    } => QueuedMutationRecord::SwitchLayer {\n                        category_id: category_id.0,\n                        description: description.clone(),\n                    },", "emergency queued draft")
# Direct calls in this file that are known to mean a fresh draft.
s = s.replace("QueuedMutation::SwitchLayer(category_id)", "QueuedMutation::SwitchLayer { category_id, description: String::new() }")
s = s.replace("QueuedMutation::SwitchLayer(DRIFT_CATEGORY_ID)", "QueuedMutation::SwitchLayer { category_id: DRIFT_CATEGORY_ID, description: String::new() }")
# Any direct internal switches outside modal use a fresh draft.
s = re.sub(r"self\.switch_active_category_at\(\n(\s*)([^,\n]+),\n(\s*)(chrono::Utc::now\(\)|[a-zA-Z_][a-zA-Z0-9_\.]*),", r"self.switch_active_category_at(\n\1\2,\n\3String::new(),\n\3\4,", s)
save(p, s)

# Modal semantics: draft mode by default; metadata is an explicit separate mode.
p = "src/app/category_state.rs"
s = load(p)
s = sub(s,
r"    pub\(super\) fn sync_modal_description_from_selection\(&mut self\) \{.*?        self\.modal_tag_index = None;\n    \}",
'''    pub(super) fn sync_modal_description_from_selection(&mut self) {
        self.modal_editing_category_metadata = false;
        if self.is_on_insert_space() {
            self.modal_description.clear();
        } else if self.time_tracker.active_category_index() == Some(self.selected_index) {
            self.modal_description = self.time_tracker.active_description().to_string();
        } else {
            self.modal_description.clear();
        }
        self.modal_tag_index = None;
    }

    pub(super) fn toggle_category_metadata_edit(&mut self) {
        if self.is_on_insert_space() {
            return;
        }
        self.modal_editing_category_metadata = !self.modal_editing_category_metadata;
        self.modal_description = if self.modal_editing_category_metadata {
            self.time_tracker
                .category_description_by_index(self.selected_index)
                .unwrap_or_default()
        } else if self.time_tracker.active_category_index() == Some(self.selected_index) {
            self.time_tracker.active_description().to_string()
        } else {
            String::new()
        };
        self.modal_tag_index = None;
    }''', "modal sync", flags=re.S)
# add_category direct switch gets fresh draft.
s = rep(s,
"            self.switch_active_category_at(\n                added_id,\n                chrono::Utc::now(),",
"            self.switch_active_category_at(\n                added_id,\n                String::new(),\n                chrono::Utc::now(),", "add category switch")
s = rep(s,
"                self.switch_active_category_at(\n                    DRIFT_CATEGORY_ID,\n                    chrono::Utc::now(),",
"                self.switch_active_category_at(\n                    DRIFT_CATEGORY_ID,\n                    String::new(),\n                    chrono::Utc::now(),", "archive active switch")
save(p, s)

p = "src/app/event_handlers.rs"
s = load(p)
s = rep(s,
"        self.queue_or_apply_mutation(QueuedMutation::SwitchLayer(category_id));",
"        self.queue_or_apply_mutation(QueuedMutation::SwitchLayer {\n            category_id,\n            description: String::new(),\n        });", "palette fresh draft")
# Replace modal confirm arm as a unit.
pattern = r"            Action::Confirm => \{\n                if self\.is_on_insert_space\(\) \{.*?\n            \}\n            Action::DeleteCategory =>"
replacement = '''            Action::Confirm => {
                if self.is_on_insert_space() {
                    if !self.new_category_name.is_empty() {
                        self.add_category();
                        self.close_modal();
                    }
                } else if self.modal_editing_category_metadata {
                    if self.time_tracker.set_category_description_by_index(
                        self.selected_index,
                        self.modal_description.clone(),
                    ) {
                        self.persist_categories();
                    }
                    if !self.has_persistence_recovery() {
                        self.close_modal();
                    }
                } else {
                    self.remember_selected_tag();
                    if self.has_persistence_recovery() {
                        self.render_needed = true;
                        return true;
                    }
                    let selected = self
                        .time_tracker
                        .category_by_index(self.selected_index)
                        .map(|category| category.id);
                    if let Some(category_id) = selected {
                        if self.time_tracker.active_category_id() == category_id {
                            self.time_tracker
                                .set_active_description(self.modal_description.clone());
                            if let Some(database_path) = self.sqlite_database_path.clone() {
                                let Some(stable_id) = self.session.active_session_stable_id.clone()
                                else {
                                    self.render_needed = true;
                                    return true;
                                };
                                let result = sqlite::update_tui_active_description(
                                    &database_path,
                                    &stable_id,
                                    &self.modal_description,
                                );
                                if self
                                    .record_storage_result_for(
                                        PersistenceOperation::ActiveDescription,
                                        RecoveryAction::ReloadAuthority,
                                        result,
                                    )
                                    .is_none()
                                {
                                    self.render_needed = true;
                                    return true;
                                }
                            }
                            self.refresh_active_runtime_checkpoint();
                        } else {
                            self.queue_or_apply_mutation(QueuedMutation::SwitchLayer {
                                category_id,
                                description: self.modal_description.clone(),
                            });
                        }
                    }
                    self.close_modal();
                }
            }
            Action::EditCategoryDescription => {
                self.toggle_category_metadata_edit();
            }
            Action::DeleteCategory =>'''
s = sub(s, pattern, replacement, "modal confirm semantics", flags=re.S)
s = rep(s,
"            Action::SwitchToNone => {\n                self.queue_or_apply_mutation(QueuedMutation::SwitchLayer(DRIFT_CATEGORY_ID));",
"            Action::SwitchToNone => {\n                self.queue_or_apply_mutation(QueuedMutation::SwitchLayer {\n                    category_id: DRIFT_CATEGORY_ID,\n                    description: String::new(),\n                });", "idle switch")
save(p, s)

# Modal rendering makes the ownership visible.
p = "src/app/category_modal_view.rs"
s = load(p)
s = rep(s,
"                    let description_text = if self.modal_description.is_empty() {",
"                    let mode = if self.modal_editing_category_metadata {\n                        \"metadata\"\n                    } else {\n                        \"draft\"\n                    };\n                    let description_text = if self.modal_description.is_empty() {", "modal mode label")
s = rep(s,
"                            format!(\" {}\", self.modal_description),",
"                            format!(\" {mode}: {}\", self.modal_description),", "modal draft label")
s = rep(s,
"                        Span::raw(layer_name).fg(text_color),\n                        description_text,",
"                        Span::raw(layer_name).fg(text_color),\n                        Span::styled(\n                            format!(\" · metadata: {}\", cat.description),\n                            Style::default().fg(text_color),\n                        ),\n                        description_text,", "show stable metadata")
save(p, s)

# Reports, lifecycle and recovery consume the explicit active draft.
p = "src/app/report_state.rs"
s = load(p)
s = sub(s,
r"        let description = self\n            \.time_tracker\n            \.category_description_by_id\(category_id\)\n            \.map\(ToString::to_string\)\n            \.unwrap_or_default\(\);",
"        let description = self.time_tracker.active_description().to_string();", "live report draft")
save(p, s)

p = "src/app/category_lifecycle_view.rs"
s = load(p)
s = sub(s,
r"        let active_description = if source_was_active \{.*?\n        \};",
"        let active_description = if source_was_active {\n            self.time_tracker.active_description().to_string()\n        } else {\n            String::new()\n        };", "lifecycle active draft", flags=re.S)
s = sub(s,
r"            let _ = self\n                \.time_tracker\n                \.set_category_description_by_id\(target_id, active_description\);",
"            self.time_tracker.set_active_description(active_description);", "lifecycle restore draft")
save(p, s)

p = "src/app/persistence_recovery.rs"
s = load(p)
s = rep(s,
"                let _ = self\n                    .time_tracker\n                    .set_category_description_by_id(active.category_id, active.description);",
"                self.time_tracker.set_active_description(active.description);", "reload active draft")
s = sub(s,
r"                let description = self\n                    \.time_tracker\n                    \.category_description_by_id\(category_id\)\n                    \.unwrap_or_default\(\)\n                    \.to_string\(\);",
"                let description = self.time_tracker.active_description().to_string();", "ensure active draft")
s = sub(s,
r"                description: self\n                    \.time_tracker\n                    \.category_description_by_id\(self\.time_tracker\.active_category_id\(\)\)\n                    \.unwrap_or_default\(\)\n                    \.to_string\(\),",
"                description: self.time_tracker.active_description().to_string(),", "emergency active draft")
s = rep(s,
"                    QueuedMutation::SwitchLayer(category_id) => QueuedMutationRecord::SwitchLayer {\n                        category_id: category_id.0,\n                    },",
"                    QueuedMutation::SwitchLayer {\n                        category_id,\n                        ref description,\n                    } => QueuedMutationRecord::SwitchLayer {\n                        category_id: category_id.0,\n                        description: description.clone(),\n                    },", "recovery queued draft")
save(p, s)

# SQLite adapter for atomic active-draft persistence.
p = "src/sqlite/tui_runtime.rs"
s = load(p)
insert = '''
pub(crate) fn update_active_description(
    database_path: &Path,
    expected_active_stable_id: &str,
    description: &str,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;
    runtime_coordination::update_active_description(
        &mut repository,
        expected_active_stable_id,
        description,
    )
    .map_err(|error| error.to_string())
}

'''
anchor = "pub(crate) fn archive_category("
if anchor not in s:
    raise SystemExit("tui adapter anchor missing")
s = s.replace(anchor, insert + anchor, 1)
save(p, s)

p = "src/sqlite.rs"
s = load(p)
s = rep(s,
"    update_session_description as update_tui_session_description,\n};",
"    update_active_description as update_tui_active_description,\n    update_session_description as update_tui_session_description,\n};", "export active draft update")
save(p, s)

# Persistence operation vocabulary.
p = "src/app/persistence_recovery.rs"
s = load(p)
s = rep(s, "    ActiveReset,\n    CategorySync,", "    ActiveReset,\n    ActiveDescription,\n    CategorySync,", "operation enum")
s = rep(s,
"            Self::ActiveReset => \"active-session reset\",\n            Self::CategorySync",
"            Self::ActiveReset => \"active-session reset\",\n            Self::ActiveDescription => \"active-session description\",\n            Self::CategorySync", "operation display")
save(p, s)

print("issue #22 transform applied")
