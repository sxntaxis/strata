from pathlib import Path

path = Path("src/app.rs")
text = path.read_text()

anchor = '''struct PaletteEntry {
    command: PaletteCommand,
    title: String,
    search_text: String,
    hint: String,
}

'''
helper = '''struct PaletteEntry {
    command: PaletteCommand,
    title: String,
    search_text: String,
    hint: String,
}

fn valid_category_ids_for_catalog(
    active_categories: impl IntoIterator<Item = Category>,
    archived_categories: &[Category],
) -> HashSet<u64> {
    let mut category_ids = active_categories
        .into_iter()
        .map(|category| category.id.0)
        .collect::<HashSet<_>>();
    category_ids.extend(archived_categories.iter().map(|category| category.id.0));
    category_ids
}

'''
if text.count(anchor) != 1:
    raise SystemExit("palette entry helper insertion point not found")
text = text.replace(anchor, helper, 1)

old = '''        let valid_category_ids: HashSet<u64> = tracker
            .categories_for_storage()
            .into_iter()
            .map(|category| category.id.0)
            .collect();
'''
new = '''        let valid_category_ids = valid_category_ids_for_catalog(
            tracker.categories_for_storage(),
            &archived_categories,
        );
'''
if text.count(old) != 1:
    raise SystemExit("category tag validation set was not found")
text = text.replace(old, new, 1)

text += r'''

#[cfg(test)]
mod category_catalog_tests {
    use super::valid_category_ids_for_catalog;
    use crate::domain::{Category, CategoryId, DRIFT_CATEGORY_ID};
    use ratatui::style::Color;

    fn category(id: u64, name: &str) -> Category {
        Category {
            id: CategoryId::new(id),
            name: name.to_string(),
            color: Color::White,
            description: String::new(),
            karma_effect: 0,
        }
    }

    #[test]
    fn archived_category_ids_remain_valid_for_tag_retention() {
        let active = vec![category(DRIFT_CATEGORY_ID.0, "idle"), category(1, "Current")];
        let archived = vec![category(7, "Historical")];
        let ids = valid_category_ids_for_catalog(active, &archived);
        assert!(ids.contains(&DRIFT_CATEGORY_ID.0));
        assert!(ids.contains(&1));
        assert!(ids.contains(&7));
    }
}
'''
path.write_text(text)

for temporary in [
    ".github/workflows/reconciliation001a-fixup.yml",
    "tools/reconciliation001a-fixup.py",
]:
    Path(temporary).unlink(missing_ok=True)
