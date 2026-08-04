from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


# DRIFT_CATEGORY_ID remains part of ordinary sediment rendering outside restore.
path = Path("src/sand/engine.rs")
text = path.read_text()
text = replace_once(
    text,
    '    domain::{Category, CategoryId},',
    '    domain::{Category, CategoryId, DRIFT_CATEGORY_ID},',
    "drift import restoration",
)
path.write_text(text)

# Authority reload must propagate strict tags/sediment validation for both SQLite and legacy.
path = Path("src/app/persistence_recovery.rs")
text = path.read_text()
old = '''            self.archived_categories = archived_categories;
            self.category_tags = storage::load_category_tags(&storage::get_category_tags_path());
            if let Some(state) = storage::load_sand_state(&storage::get_sand_state_path()) {
                let valid_category_ids = self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect();
                self.sand_engine.restore_state(&state, &valid_category_ids);
            }
'''
new = '''            self.archived_categories = archived_categories;
            self.category_tags =
                storage::try_load_category_tags(&storage::get_category_tags_path())?;
            if let Some(state) =
                storage::try_load_sand_state(&storage::get_sand_state_path())?
            {
                let valid_category_ids = self
                    .time_tracker
                    .categories_for_storage()
                    .into_iter()
                    .chain(self.archived_categories.iter().cloned())
                    .map(|category| category.id)
                    .collect();
                self.sand_engine
                    .restore_state(&state, &valid_category_ids)?;
            }
'''
text = replace_once(text, old, new, "legacy recovery authority")
old_restore = '                self.sand_engine.restore_state(&state, &valid_category_ids);'
count = text.count(old_restore)
if count < 1:
    raise SystemExit("strict recovery sediment restore: expected at least one remaining marker")
text = text.replace(
    old_restore,
    '                self.sand_engine\n                    .restore_state(&state, &valid_category_ids)?;',
)
path.write_text(text)

# Lifecycle preparation is also an authority read and may not default damaged tags.
path = Path("src/legacy_category_lifecycle.rs")
text = path.read_text()
text = replace_once(
    text,
    '    let tags = storage::load_category_tags(&paths.category_tags_json);',
    '    let tags = storage::try_load_category_tags(&paths.category_tags_json)?;',
    "lifecycle authority tags",
)
text = replace_once(
    text,
    '        let mut tags = storage::load_category_tags(&paths.category_tags_json);',
    '        let mut tags =\n            storage::try_load_category_tags(&paths.category_tags_json).unwrap();',
    "lifecycle stale-preview test tags",
)
path.write_text(text)

# Existing replay proof reads strict sediment state explicitly.
path = Path("src/app.rs")
text = path.read_text()
text = replace_once(
    text,
    '        let persisted_sand = storage::load_sand_state(sand_path).unwrap();',
    '        let persisted_sand = storage::try_load_sand_state(sand_path)\n            .unwrap()\n            .unwrap();',
    "legacy finish persisted sediment test",
)
path.write_text(text)

print("strict legacy custody callers integrated")
