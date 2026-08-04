from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


# Normalize overlay-local refusal handling and sibling-module calls.
path = Path("src/app/category_lifecycle_view.rs")
content = path.read_text()
content = content.replace("self.is_on_insert_space()", "self.selected_index >= self.time_tracker.category_count()")
content = content.replace(
    '''            self.category_action_error = Some(
                "Select a non-idle layer. Archive remains the ordinary retirement action."
                    .to_string(),
            );
            self.render_needed = true;
            return;
''',
    '''            self.show_category_lifecycle_error(
                DRIFT_CATEGORY_ID,
                "Unavailable".to_string(),
                "Select a non-idle layer. Archive remains the ordinary retirement action."
                    .to_string(),
            );
            return;
''',
)
content = content.replace(
    '''            self.category_action_error = Some("Selected layer is unavailable.".to_string());
            self.render_needed = true;
            return;
''',
    '''            self.show_category_lifecycle_error(
                DRIFT_CATEGORY_ID,
                "Unavailable".to_string(),
                "Selected layer is unavailable.".to_string(),
            );
            return;
''',
)
content = content.replace(
    '''            self.category_action_error = Some(
                "Idle cannot be merged or permanently deleted. Select another layer."
                    .to_string(),
            );
            self.render_needed = true;
            return;
''',
    '''            self.show_category_lifecycle_error(
                DRIFT_CATEGORY_ID,
                "idle".to_string(),
                "Idle cannot be merged or permanently deleted. Select another layer."
                    .to_string(),
            );
            return;
''',
)
content = content.replace("            self.sync_modal_description_from_selection();\n", "")
content = content.replace(
    '''            self.category_action_error = Some(format!(
                "Layer {} is unavailable for lifecycle review.",
                source_id.0
            ));
            self.render_needed = true;
            return;
''',
    '''            self.show_category_lifecycle_error(
                source_id,
                "Unavailable".to_string(),
                format!("Layer {} is unavailable for lifecycle review.", source_id.0),
            );
            return;
''',
)
content = content.replace(
    '''            self.category_action_error = Some(
                "Idle cannot be merged or permanently deleted.".to_string(),
            );
            self.render_needed = true;
            return;
''',
    '''            self.show_category_lifecycle_error(
                source_id,
                source.name,
                "Idle cannot be merged or permanently deleted.".to_string(),
            );
            return;
''',
)
content = content.replace("        self.category_action_error = None;\n", "")
content = content.replace(
    "impl App {\n    pub(super) fn open_category_lifecycle_for_selected",
    '''impl App {
    fn show_category_lifecycle_error(
        &mut self,
        source_id: CategoryId,
        source_name: String,
        error: String,
    ) {
        self.category_lifecycle_overlay = Some(CategoryLifecycleOverlay {
            source_id,
            source_name,
            targets: Vec::new(),
            selected_target: 0,
            stage: CategoryLifecycleStage::SelectTarget,
            confirmation_input: String::new(),
            error: Some(error),
        });
        self.render_needed = true;
    }

    pub(super) fn open_category_lifecycle_for_selected''',
)
path.write_text(content)

# SQLite path adapters and retired-ID-aware TUI allocation.
replace_once(
    "src/sqlite/category_lifecycle.rs",
    "use std::{collections::BTreeSet, fmt::Write as _};",
    "use std::{collections::BTreeSet, fmt::Write as _, path::Path};",
)
replace_once(
    "src/sqlite/category_lifecycle.rs",
    "    domain::OperationalDayPolicy,",
    "    domain::{CategoryId, OperationalDayPolicy},",
)
wrapper_marker = '''pub(crate) fn preview(
    repository: &SqliteRepository,
'''
wrappers = '''pub(crate) fn preview_at(
    database_path: &Path,
    source_category_id: CategoryId,
    target_category_id: Option<CategoryId>,
) -> Result<CategoryLifecyclePreview, String> {
    let repository = SqliteRepository::open(database_path).map_err(|error| error.to_string())?;
    preview(
        &repository,
        i64::try_from(source_category_id.0)
            .map_err(|_| "source category identity exceeds SQLite range".to_string())?,
        target_category_id
            .map(|target| i64::try_from(target.0))
            .transpose()
            .map_err(|_| "target category identity exceeds SQLite range".to_string())?,
    )
}

pub(crate) fn apply_at(
    database_path: &Path,
    source_category_id: CategoryId,
    target_category_id: Option<CategoryId>,
    expected_revision: &str,
    applied_at_utc: DateTime<Utc>,
) -> Result<CategoryLifecycleReceipt, String> {
    let mut repository = SqliteRepository::open(database_path).map_err(|error| error.to_string())?;
    apply(
        &mut repository,
        CategoryLifecycleRequest {
            source_category_id: i64::try_from(source_category_id.0)
                .map_err(|_| "source category identity exceeds SQLite range".to_string())?,
            target_category_id: target_category_id
                .map(|target| i64::try_from(target.0))
                .transpose()
                .map_err(|_| "target category identity exceeds SQLite range".to_string())?,
            expected_revision,
            applied_at_utc: &applied_at_utc.to_rfc3339(),
        },
    )
}

'''
replace_once("src/sqlite/category_lifecycle.rs", wrapper_marker, wrappers + wrapper_marker)
replace_once(
    "src/sqlite.rs",
    '''    CategoryReferenceCounts, apply as apply_category_lifecycle,
    preview as preview_category_lifecycle,
''',
    '''    CategoryReferenceCounts, apply as apply_category_lifecycle,
    apply_at as apply_category_lifecycle_at, preview as preview_category_lifecycle,
    preview_at as preview_category_lifecycle_at,
''',
)
replace_once(
    "src/sqlite/tui_runtime.rs",
    '''    let max_category_id = repository
        .connection
        .query_row("SELECT COALESCE(MAX(id), 0) FROM categories", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| error.to_string())?;
    let next_category_id = u64::try_from(max_category_id)
        .map_err(|_| "SQLite category ID is negative".to_string())?
        .checked_add(1)
        .ok_or_else(|| "SQLite category ID space is exhausted".to_string())?;
''',
    '''    let max_category_id = repository
        .connection
        .query_row("SELECT COALESCE(MAX(id), 0) FROM categories", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| error.to_string())?;
    let max_lifecycle_id = repository
        .connection
        .query_row(
            "SELECT COALESCE(MAX(identity), 0)
             FROM (
                 SELECT source_category_id AS identity FROM category_lifecycle_receipts
                 UNION ALL
                 SELECT target_category_id AS identity FROM category_lifecycle_receipts
                 WHERE target_category_id IS NOT NULL
             )",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let next_category_id = u64::try_from(max_category_id.max(max_lifecycle_id))
        .map_err(|_| "SQLite category ID is negative".to_string())?
        .checked_add(1)
        .ok_or_else(|| "SQLite category ID space is exhausted".to_string())?;
''',
)

# App state and module wiring.
replace_once(
    "src/app.rs",
    "mod category_modal_view;\nmod category_state;\n",
    "mod category_lifecycle_view;\nmod category_modal_view;\nmod category_state;\n",
)
replace_once(
    "src/app.rs",
    "use persistence_recovery::{PersistenceOperation, PersistenceRecoveryState, RecoveryAction};",
    '''use category_lifecycle_view::CategoryLifecycleOverlay;
use persistence_recovery::{PersistenceOperation, PersistenceRecoveryState, RecoveryAction};''',
)
replace_once(
    "src/app.rs",
    '''    sqlite_database_path: Option<PathBuf>,
    archived_categories: Vec<Category>,
    checkpoint_recovery_active: bool,
''',
    '''    sqlite_database_path: Option<PathBuf>,
    archived_categories: Vec<Category>,
    category_lifecycle_overlay: Option<CategoryLifecycleOverlay>,
    checkpoint_recovery_active: bool,
''',
)
replace_once(
    "src/app.rs",
    '''            sqlite_database_path,
            archived_categories,
            checkpoint_recovery_active: false,
''',
    '''            sqlite_database_path,
            archived_categories,
            category_lifecycle_overlay: None,
            checkpoint_recovery_active: false,
''',
)

# Input and action routing.
replace_once(
    "src/app/event_handlers.rs",
    '''        if self.keymap.matches_mandatory(key, Action::Quit) {
            return true;
        }

        if self.report_log_edit.is_some() {
''',
    '''        if self.keymap.matches_mandatory(key, Action::Quit) {
            return true;
        }

        if self.category_lifecycle_overlay.is_some() {
            return self.handle_category_lifecycle_key(key);
        }

        if self.report_log_edit.is_some() {
''',
)
replace_once(
    "src/app/event_handlers.rs",
    '''            Action::DeleteCategory => self.delete_category(),
            Action::ArchiveCategory => self.delete_category(),
''',
    '''            Action::DeleteCategory => self.delete_category(),
            Action::CategoryLifecycle => self.open_category_lifecycle_for_selected(),
            Action::ArchiveCategory => self.delete_category(),
''',
)
replace_once(
    "src/app/event_handlers.rs",
    '''            Action::OpenCategoryModal => {
                self.open_modal();
                false
            }
''',
    '''            Action::OpenCategoryModal => {
                self.open_modal();
                false
            }
            Action::CategoryLifecycle => {
                self.open_category_lifecycle_for_active();
                false
            }
''',
)

# Rendering priority: lifecycle below recovery overlays, above ordinary modals.
replace_once(
    "src/app/render_views.rs",
    '''        if self.show_command_palette {
            self.render_command_palette(f, size);
        }

        if self.recovery_statement.is_some() {
''',
    '''        if self.show_command_palette {
            self.render_command_palette(f, size);
        }

        if self.category_lifecycle_overlay.is_some() {
            self.render_category_lifecycle(f, size);
        }

        if self.recovery_statement.is_some() {
''',
)

# Recovery operation, replay-aware reload, and overlay convergence.
replace_once(
    "src/app/persistence_recovery.rs",
    '''    CategoryArchive,
    CategoryTagsSync,
''',
    '''    CategoryArchive,
    CategoryLifecycle,
    CategoryTagsSync,
''',
)
replace_once(
    "src/app/persistence_recovery.rs",
    '''            Self::CategoryArchive => "category archive",
            Self::CategoryTagsSync => "category tags sync",
''',
    '''            Self::CategoryArchive => "category archive",
            Self::CategoryLifecycle => "category lifecycle",
            Self::CategoryTagsSync => "category tags sync",
''',
)
replace_once(
    "src/app/persistence_recovery.rs",
    "    fn try_reload_authority(&mut self) -> Result<(), String> {",
    "    pub(super) fn try_reload_authority(&mut self) -> Result<(), String> {",
)
replace_once(
    "src/app/persistence_recovery.rs",
    '''        } else {
            let (categories, sessions) = load_legacy_recovery_authority(
                &storage::get_categories_path(),
                &storage::get_time_log_path(),
            )?;
            let archived_categories = categories.archived_categories;
            self.time_tracker.apply_loaded_state(
                categories.categories,
                categories.next_category_id,
''',
    '''        } else {
            let lifecycle_paths =
                crate::legacy_category_lifecycle::LegacyCategoryLifecyclePaths::runtime();
            crate::legacy_category_lifecycle::replay_prepared(&lifecycle_paths)?;
            let ledger = crate::legacy_category_lifecycle::load_ledger(&lifecycle_paths)?;
            let (mut categories, sessions) = load_legacy_recovery_authority(
                &storage::get_categories_path(),
                &storage::get_time_log_path(),
            )?;
            categories.next_category_id = crate::legacy_category_lifecycle::next_category_id(
                categories.next_category_id,
                &ledger,
            )?;
            let archived_categories = categories.archived_categories;
            self.time_tracker.apply_loaded_state(
                categories.categories,
                categories.next_category_id,
''',
)
replace_once(
    "src/app/persistence_recovery.rs",
    '''        }
        self.sync_drift_idle_state();
        Ok(())
''',
    '''        }
        self.sync_drift_idle_state();
        if self.category_lifecycle_overlay.is_some() {
            self.category_lifecycle_overlay = None;
            self.ui_mode = super::UiMode::Main;
            self.selected_index = 0;
        }
        Ok(())
''',
)

# Truthful configurable action and default Shift-X binding.
replace_once(
    "src/keybindings.rs",
    '''    NewCategory,
    DeleteCategory,
    ArchiveCategory,
''',
    '''    NewCategory,
    DeleteCategory,
    CategoryLifecycle,
    ArchiveCategory,
''',
)
replace_once(
    "src/keybindings.rs",
    "    pub const ALL: [Self; 28] = [",
    "    pub const ALL: [Self; 29] = [",
)
replace_once(
    "src/keybindings.rs",
    '''        Self::NewCategory,
        Self::DeleteCategory,
        Self::ArchiveCategory,
''',
    '''        Self::NewCategory,
        Self::DeleteCategory,
        Self::CategoryLifecycle,
        Self::ArchiveCategory,
''',
)
replace_once(
    "src/keybindings.rs",
    '''            Self::NewCategory => "new_layer",
            Self::DeleteCategory => "delete_layer",
            Self::ArchiveCategory => "archive_layer",
''',
    '''            Self::NewCategory => "new_layer",
            Self::DeleteCategory => "delete_layer",
            Self::CategoryLifecycle => "category_lifecycle",
            Self::ArchiveCategory => "archive_layer",
''',
)
replace_once(
    "src/keybindings.rs",
    '''            "delete_layer" | "delete_category" => Some(Self::DeleteCategory),
            "archive_layer" | "archive_category" => Some(Self::ArchiveCategory),
''',
    '''            "delete_layer" | "delete_category" => Some(Self::DeleteCategory),
            "category_lifecycle" | "merge_or_delete_layer" | "permanent_layer_lifecycle" => {
                Some(Self::CategoryLifecycle)
            }
            "archive_layer" | "archive_category" => Some(Self::ArchiveCategory),
''',
)
replace_once(
    "src/keybindings.rs",
    '''            Self::NewCategory => "Create layer",
            Self::DeleteCategory => "Delete layer",
            Self::ArchiveCategory => "Archive selected layer",
''',
    '''            Self::NewCategory => "Create layer",
            Self::DeleteCategory => "Archive selected layer",
            Self::CategoryLifecycle => "Merge or permanently delete selected layer",
            Self::ArchiveCategory => "Archive selected layer",
''',
)
replace_once(
    "src/keybindings.rs",
    '''            Self::NewCategory
            | Self::DeleteCategory
            | Self::ArchiveCategory
''',
    '''            Self::NewCategory
            | Self::DeleteCategory
            | Self::CategoryLifecycle
            | Self::ArchiveCategory
''',
)
replace_once(
    "src/keybindings.rs",
    "const DEFAULT_BINDINGS: [(&str, Action); 26] = [",
    "const DEFAULT_BINDINGS: [(&str, Action); 27] = [",
)
replace_once(
    "src/keybindings.rs",
    '''    ("n", Action::NewCategory),
    ("x", Action::DeleteCategory),
    ("r", Action::RestoreCategory),
''',
    '''    ("n", Action::NewCategory),
    ("x", Action::DeleteCategory),
    ("X", Action::CategoryLifecycle),
    ("r", Action::RestoreCategory),
''',
)

# One keymap proof for distinct archive/lifecycle routes.
key_test_marker = '''    #[test]
    fn test_default_keymap_has_t_for_karma_day() {
'''
key_test = '''    #[test]
    fn default_archive_and_lifecycle_actions_are_distinct() {
        let keymap = default_keymap();
        assert_eq!(
            keymap.action_for(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Some(Action::DeleteCategory)
        );
        assert_eq!(
            keymap.action_for(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT)),
            Some(Action::CategoryLifecycle)
        );
        assert_eq!(Action::DeleteCategory.description(), "Archive selected layer");
    }

'''
replace_once("src/keybindings.rs", key_test_marker, key_test + key_test_marker)
