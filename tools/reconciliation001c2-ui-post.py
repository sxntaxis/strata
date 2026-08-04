from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


# Generated overlay integration fixes.
replace_once(
    "src/app/category_lifecycle_view.rs",
    "                source.name,\n                \"Idle cannot be merged or permanently deleted.\".to_string(),",
    "                source.name.clone(),\n                \"Idle cannot be merged or permanently deleted.\".to_string(),",
)
replace_once(
    "src/app/category_lifecycle_view.rs",
    '''                .map(|target| u64::try_from(target.id).map(CategoryId::new))
''',
    '''                .map(|target| {
                    u64::try_from(target.id).map(|value| CategoryId::new(value))
                })
''',
)
replace_once(
    "src/app/category_lifecycle_view.rs",
    '''    fn build_category_lifecycle_review(
        &self,
''',
    '''    fn build_category_lifecycle_review(
        &mut self,
''',
)
replace_once(
    "src/app/category_lifecycle_view.rs",
    '''        } else {
            let review = crate::legacy_category_lifecycle::build_review(
''',
    '''        } else {
            self.try_write_runtime_checkpoint()?;
            let review = crate::legacy_category_lifecycle::build_review(
''',
)
replace_once(
    "src/app/category_lifecycle_view.rs",
    '''        let result = if let Some(database_path) = self.sqlite_database_path.as_deref() {
            sqlite::apply_category_lifecycle_at(
                database_path,
''',
    '''        let result = if let Some(database_path) = self.sqlite_database_path.clone() {
            sqlite::apply_category_lifecycle_at(
                &database_path,
''',
)
path = Path("src/app/category_lifecycle_view.rs")
content = path.read_text().replace("        self.category_action_error = None;\n", "")
path.write_text(content)
replace_once(
    "src/app.rs",
    "    fn try_write_runtime_checkpoint(&self) -> Result<(), String> {",
    "    pub(super) fn try_write_runtime_checkpoint(&self) -> Result<(), String> {",
)

# SQLite lifecycle identity high-watermark adapter for TUI reload.
marker = '''pub(crate) fn preview_at(
    database_path: &Path,
'''
function = '''pub(crate) fn identity_high_watermark_at(database_path: &Path) -> Result<u64, String> {
    let repository = SqliteRepository::open(database_path).map_err(|error| error.to_string())?;
    let maximum: i64 = repository
        .connection
        .query_row(
            "SELECT COALESCE(MAX(identity), 0)
             FROM (
                 SELECT id AS identity FROM categories
                 UNION ALL
                 SELECT source_category_id AS identity FROM category_lifecycle_receipts
                 UNION ALL
                 SELECT target_category_id AS identity FROM category_lifecycle_receipts
                 WHERE target_category_id IS NOT NULL
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    u64::try_from(maximum)
        .map_err(|_| "SQLite category identity high-watermark is negative".to_string())
}

'''
replace_once("src/sqlite/category_lifecycle.rs", marker, function + marker)
replace_once(
    "src/sqlite.rs",
    '''    apply_at as apply_category_lifecycle_at, preview as preview_category_lifecycle,
    preview_at as preview_category_lifecycle_at,
''',
    '''    apply_at as apply_category_lifecycle_at,
    identity_high_watermark_at as category_identity_high_watermark_at,
    preview as preview_category_lifecycle, preview_at as preview_category_lifecycle_at,
''',
)
replace_once(
    "src/sqlite/tui_runtime.rs",
    '''    active_categories.sort_by_key(|category| category_sort_order(database_path, category.id.0));

    let mut sessions = Vec::with_capacity(session_rows.len());
''',
    '''    active_categories.sort_by_key(|category| category_sort_order(database_path, category.id.0));
    max_category_id = max_category_id.max(super::category_identity_high_watermark_at(database_path)?);

    let mut sessions = Vec::with_capacity(session_rows.len());
''',
)

# Palette truth: lifecycle must be discoverable independently of archive.
replace_once(
    "src/app/command_palette_view.rs",
    '''            self.palette_action_entry(
                Action::OpenReportModal,
''',
    '''            self.palette_action_entry(
                Action::CategoryLifecycle,
                "Merge or permanently delete active layer",
                &["layer", "merge", "reassign", "permanent", "delete", "destructive"],
            ),
            self.palette_action_entry(
                Action::OpenReportModal,
''',
)

# Ensure all explicit action availability matches include the new lifecycle action.
for file_name in [
    "src/app/command_palette_view.rs",
    "src/app/keybindings_modal_view.rs",
    "src/app/event_handlers.rs",
]:
    path = Path(file_name)
    text = path.read_text()
    text = text.replace(
        "Action::DeleteCategory | Action::ArchiveCategory",
        "Action::DeleteCategory | Action::CategoryLifecycle | Action::ArchiveCategory",
    )
    text = text.replace(
        "Action::DeleteCategory\n            | Action::ArchiveCategory",
        "Action::DeleteCategory\n            | Action::CategoryLifecycle\n            | Action::ArchiveCategory",
    )
    path.write_text(text)
