from pathlib import Path

path = Path("src/app.rs")
content = path.read_text()
old = '''        let valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<HashSet<_>>();
'''
new = '''        let mut valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        valid_category_ids.insert(DRIFT_CATEGORY_ID);
'''
if old not in content:
    raise SystemExit("transition valid-category marker missing")
path.write_text(content.replace(old, new, 1))
