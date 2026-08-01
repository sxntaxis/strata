from pathlib import Path

path = Path("src/storage.rs")
text = path.read_text()
text = text.replace(
    "pub fn load_categories_from_csv(path: &Path) -> LoadedCategories {",
    "#[cfg(test)]\npub fn load_categories_from_csv(path: &Path) -> LoadedCategories {",
    1,
)
text = text.replace(
    "pub fn load_sessions_from_csv(path: &Path, categories: &[Category]) -> LoadedSessions {",
    "#[cfg(test)]\npub fn load_sessions_from_csv(path: &Path, categories: &[Category]) -> LoadedSessions {",
    1,
)
path.write_text(text)
