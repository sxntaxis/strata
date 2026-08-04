from pathlib import Path

path = Path("src/domain.rs")
text = path.read_text()

removals = [
    """    pub fn get_mut_by_id(&mut self, id: CategoryId) -> Option<&mut Category> {
        self.by_id.get_mut(&id)
    }

""",
    """    pub fn category_description_by_id(&self, id: CategoryId) -> Option<&str> {
        self.category_by_id(id)
            .map(|category| category.description.as_str())
    }

""",
    """    pub fn set_category_description_by_id(
        &mut self,
        category_id: CategoryId,
        description: String,
    ) -> bool {
        let Some(category) = self.category_store.get_mut_by_id(category_id) else {
            return false;
        };

        category.description = description;
        true
    }

""",
]

for block in removals:
    if text.count(block) != 1:
        raise SystemExit("superseded category API marker missing")
    text = text.replace(block, "", 1)

old = 'assert_eq!(tracker.category_description_by_id(id), Some("Stable metadata"));'
new = """assert_eq!(
            tracker
                .category_by_id(id)
                .map(|category| category.description.as_str()),
            Some("Stable metadata")
        );"""
if text.count(old) != 2:
    raise SystemExit("active draft proof metadata marker missing")
path.write_text(text.replace(old, new))

path = Path("src/app.rs")
text = path.read_text()
replacements = [
    (
        'fn categories(before_switch: bool) -> Vec<Category> {\n        vec![\n            category(DRIFT_CATEGORY_ID.0, "idle", ""),\n            category(1, "Previous", if before_switch { "focus" } else { "" }),\n            category(2, "Next", "next task"),\n        ]\n    }',
        'fn categories(_before_switch: bool) -> Vec<Category> {\n        vec![\n            category(DRIFT_CATEGORY_ID.0, "idle", ""),\n            category(1, "Previous", "focus"),\n            category(2, "Next", "next task"),\n        ]\n    }',
    ),
    (
        'assert_eq!(previous.description, "");\n        assert_eq!(next.description, "next task");',
        'assert_eq!(previous.description, "focus");\n        assert_eq!(next.description, "next task");',
    ),
    (
        'fn categories(before_finish: bool) -> Vec<Category> {\n        vec![\n            category(DRIFT_CATEGORY_ID.0, "idle", ""),\n            category(1, "Work", if before_finish { "focus" } else { "" }),\n        ]\n    }',
        'fn categories(_before_finish: bool) -> Vec<Category> {\n        vec![\n            category(DRIFT_CATEGORY_ID.0, "idle", ""),\n            category(1, "Work", "focus"),\n        ]\n    }',
    ),
    (
        'assert_eq!(work.description, "");',
        'assert_eq!(work.description, "focus");',
    ),
]
for old, new in replacements:
    if text.count(old) != 1:
        raise SystemExit(f"legacy metadata preservation marker missing: {old[:40]}")
    text = text.replace(old, new, 1)
path.write_text(text)
