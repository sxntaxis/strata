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
