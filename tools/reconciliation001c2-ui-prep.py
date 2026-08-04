from pathlib import Path

path = Path("tools/reconciliation001c2-ui.py")
content = path.read_text()

# Normalize the lifecycle insertion marker to the current mandatory-action API
# before the main transformation executes.
content = content.replace(
    "if self.keymap.matches_mandatory(key, Action::Quit)",
    "if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit)",
)

# Normalize the category-modal action insertion to the current guarded archive
# arm. Ordinary DeleteCategory remains archive; CategoryLifecycle is separate.
legacy_desired = """            Action::DeleteCategory => self.delete_category(),
            Action::CategoryLifecycle => self.open_category_lifecycle_for_selected(),
            Action::ArchiveCategory => self.delete_category(),
"""
current_desired = """            Action::DeleteCategory => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.delete_category();
                }
            }
            Action::CategoryLifecycle => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.open_category_lifecycle_for_selected();
                }
            }
            Action::IncreaseKarma => {
"""
legacy_marker = """            Action::DeleteCategory => self.delete_category(),
            Action::ArchiveCategory => self.delete_category(),
"""
current_marker = """            Action::DeleteCategory => {
                if !self.is_on_insert_space() && self.selected_index > 0 {
                    self.delete_category();
                }
            }
            Action::IncreaseKarma => {
"""
if legacy_desired not in content or legacy_marker not in content:
    raise SystemExit("obsolete category-modal lifecycle transform block missing")
content = content.replace(legacy_desired, current_desired, 1)
content = content.replace(legacy_marker, current_marker, 1)

# Normalize the persistence-operation label marker to current terminology.
content = content.replace(
    '''            Self::CategoryArchive => "category archive",
            Self::CategoryLifecycle => "category lifecycle",
            Self::CategoryTagsSync => "category tags sync",
''',
    '''            Self::CategoryArchive => "category archive",
            Self::CategoryLifecycle => "category lifecycle",
            Self::CategoryTagsSync => "category-tag synchronization",
''',
)
content = content.replace(
    '''            Self::CategoryArchive => "category archive",
            Self::CategoryTagsSync => "category tags sync",
''',
    '''            Self::CategoryArchive => "category archive",
            Self::CategoryTagsSync => "category-tag synchronization",
''',
)

# Translate the lifecycle action transform from the superseded keymap model to
# the current Action enum, registry, descriptions, category, and defaults.
keymap_replacements = [
    (
        '''    NewCategory,
    DeleteCategory,
    CategoryLifecycle,
    ArchiveCategory,
''',
        '''    DeleteCategory,
    CategoryLifecycle,
    IncreaseKarma,
''',
    ),
    (
        '''    NewCategory,
    DeleteCategory,
    ArchiveCategory,
''',
        '''    DeleteCategory,
    IncreaseKarma,
''',
    ),
    ("    pub const ALL: [Self; 29] = [", "    const ALL: [Action; 29] = ["),
    ("    pub const ALL: [Self; 28] = [", "    const ALL: [Action; 28] = ["),
    (
        '''        Self::NewCategory,
        Self::DeleteCategory,
        Self::CategoryLifecycle,
        Self::ArchiveCategory,
''',
        '''        Action::DeleteCategory,
        Action::CategoryLifecycle,
        Action::IncreaseKarma,
''',
    ),
    (
        '''        Self::NewCategory,
        Self::DeleteCategory,
        Self::ArchiveCategory,
''',
        '''        Action::DeleteCategory,
        Action::IncreaseKarma,
''',
    ),
    (
        '''            Self::NewCategory => "new_layer",
            Self::DeleteCategory => "delete_layer",
            Self::CategoryLifecycle => "category_lifecycle",
            Self::ArchiveCategory => "archive_layer",
''',
        '''            Action::DeleteCategory => "delete_layer",
            Action::CategoryLifecycle => "category_lifecycle",
            Action::IncreaseKarma => "boost_layer_karma",
''',
    ),
    (
        '''            Self::NewCategory => "new_layer",
            Self::DeleteCategory => "delete_layer",
            Self::ArchiveCategory => "archive_layer",
''',
        '''            Action::DeleteCategory => "delete_layer",
            Action::IncreaseKarma => "boost_layer_karma",
''',
    ),
    (
        '''            "delete_layer" | "delete_category" => Some(Self::DeleteCategory),
            "category_lifecycle" | "merge_or_delete_layer" | "permanent_layer_lifecycle" => {
                Some(Self::CategoryLifecycle)
            }
            "archive_layer" | "archive_category" => Some(Self::ArchiveCategory),
''',
        '''            "delete_layer" | "delete_category" => Some(Self::DeleteCategory),
            "category_lifecycle" | "merge_or_delete_layer" | "permanent_layer_lifecycle" => {
                Some(Self::CategoryLifecycle)
            }
            "boost_layer_karma" | "increase_karma" => Some(Self::IncreaseKarma),
''',
    ),
    (
        '''            "delete_layer" | "delete_category" => Some(Self::DeleteCategory),
            "archive_layer" | "archive_category" => Some(Self::ArchiveCategory),
''',
        '''            "delete_layer" | "delete_category" => Some(Self::DeleteCategory),
            "boost_layer_karma" | "increase_karma" => Some(Self::IncreaseKarma),
''',
    ),
    (
        '''            Self::NewCategory => "Create layer",
            Self::DeleteCategory => "Archive selected layer",
            Self::CategoryLifecycle => "Merge or permanently delete selected layer",
            Self::ArchiveCategory => "Archive selected layer",
''',
        '''            Action::DeleteCategory => "Archive selected layer",
            Action::CategoryLifecycle => "Merge or permanently delete selected layer",
            Action::IncreaseKarma => "Set selected layer karma to +1",
''',
    ),
    (
        '''            Self::NewCategory => "Create layer",
            Self::DeleteCategory => "Delete layer",
            Self::ArchiveCategory => "Archive selected layer",
''',
        '''            Action::DeleteCategory => "Delete selected layer",
            Action::IncreaseKarma => "Set selected layer karma to +1",
''',
    ),
    (
        '''            Self::NewCategory
            | Self::DeleteCategory
            | Self::CategoryLifecycle
            | Self::ArchiveCategory
''',
        '''            Action::DeleteCategory
            | Action::CategoryLifecycle
            | Action::IncreaseKarma
''',
    ),
    (
        '''            Self::NewCategory
            | Self::DeleteCategory
            | Self::ArchiveCategory
''',
        '''            Action::DeleteCategory
            | Action::IncreaseKarma
''',
    ),
    ("const DEFAULT_BINDINGS: [(&str, Action); 27] = [", "const DEFAULT_BINDINGS: [(&str, Action); 31] = ["),
    ("const DEFAULT_BINDINGS: [(&str, Action); 26] = [", "const DEFAULT_BINDINGS: [(&str, Action); 30] = ["),
    (
        '''    ("n", Action::NewCategory),
    ("x", Action::DeleteCategory),
    ("X", Action::CategoryLifecycle),
    ("r", Action::RestoreCategory),
''',
        '''    ("x", Action::DeleteCategory),
    ("shift-x", Action::CategoryLifecycle),
    ("+", Action::IncreaseKarma),
''',
    ),
    (
        '''    ("n", Action::NewCategory),
    ("x", Action::DeleteCategory),
    ("r", Action::RestoreCategory),
''',
        '''    ("x", Action::DeleteCategory),
    ("+", Action::IncreaseKarma),
''',
    ),
]
for old, new in keymap_replacements:
    content = content.replace(old, new)
content = content.replace("keymap.action_for(", "keymap.action_for_key_event(")
content = content.replace(
    "KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT)",
    "KeyEvent::new(KeyCode::Char('x'), KeyModifiers::SHIFT)",
)

# The retired-ID high-watermark is now added by the post-transform adapter
# against the current load-state structure, so remove the obsolete earlier block.
start = content.find('replace_once(\n    "src/sqlite/tui_runtime.rs",')
if start < 0:
    raise SystemExit("obsolete TUI high-watermark transform block missing")
end_marker = "\n\n# App state and module wiring."
end = content.find(end_marker, start)
if end < 0:
    raise SystemExit("TUI transform block end missing")
path.write_text(content[:start] + content[end:])
