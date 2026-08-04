from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


replace_once(
    "src/lib.rs",
    "mod keybindings;\nmod legacy_transition;\n",
    "mod keybindings;\nmod legacy_category_lifecycle;\nmod legacy_transition;\n",
)
replace_once(
    "src/storage.rs",
    '''pub fn get_category_tags_path() -> PathBuf {
    get_state_dir().join("category_tags.json")
}

pub fn get_keymap_path() -> PathBuf {
''',
    '''pub fn get_category_tags_path() -> PathBuf {
    get_state_dir().join("category_tags.json")
}

pub fn get_category_lifecycle_prepared_path() -> PathBuf {
    get_state_dir().join("category_lifecycle_prepared.json")
}

pub fn get_category_lifecycle_ledger_path() -> PathBuf {
    get_state_dir().join("category_lifecycle_ledger.json")
}

pub fn get_keymap_path() -> PathBuf {
''',
)
path = Path("src/legacy_category_lifecycle.rs")
content = path.read_text()
content = content.replace("storage::write_private_json_atomic", "storage::write_json_atomic")
content = content.replace("collections::{BTreeMap, BTreeSet}", "collections::BTreeSet")
content = content.replace("use ratatui::style::Color;\n", "")
content = content.replace(
    "    daily_contribution_from_slices(operational_day, width, height, &slices)\n",
    "    Ok(daily_contribution_from_slices(operational_day, width, height, &slices))\n",
)
path.write_text(content)
