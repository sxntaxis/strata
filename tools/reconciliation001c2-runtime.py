from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


replace_once(
    "src/lib.rs",
    "#[allow(dead_code)]\nmod legacy_category_lifecycle;",
    "mod legacy_category_lifecycle;",
)
replace_once(
    "src/storage.rs",
    "#[allow(dead_code)]\npub fn get_category_lifecycle_prepared_path() -> PathBuf {",
    "pub fn get_category_lifecycle_prepared_path() -> PathBuf {",
)
replace_once(
    "src/storage.rs",
    "#[allow(dead_code)]\npub fn get_category_lifecycle_ledger_path() -> PathBuf {",
    "pub fn get_category_lifecycle_ledger_path() -> PathBuf {",
)
replace_once(
    "src/app.rs",
    '''            sqlite::RuntimeAuthority::LegacyFiles => {
                let categories_path = storage::get_categories_path();
                let sessions_path = storage::get_time_log_path();
                let loaded_categories = storage::try_load_categories_from_csv(&categories_path)
                    .map_err(|error| error.to_string())?;
''',
    '''            sqlite::RuntimeAuthority::LegacyFiles => {
                let lifecycle_paths =
                    crate::legacy_category_lifecycle::LegacyCategoryLifecyclePaths::runtime();
                crate::legacy_category_lifecycle::replay_prepared(&lifecycle_paths)?;
                let ledger = crate::legacy_category_lifecycle::load_ledger(&lifecycle_paths)?;
                let categories_path = storage::get_categories_path();
                let sessions_path = storage::get_time_log_path();
                let mut loaded_categories = storage::try_load_categories_from_csv(&categories_path)
                    .map_err(|error| error.to_string())?;
                loaded_categories.next_category_id =
                    crate::legacy_category_lifecycle::next_category_id(
                        loaded_categories.next_category_id,
                        &ledger,
                    )?;
''',
)

# Process-level replay proof: make the core test explicitly simulate startup's order.
path = Path("src/legacy_category_lifecycle.rs")
content = path.read_text()
marker = '''    #[test]
    fn targetless_delete_requires_complete_zero_reference_preview() {
'''
test = '''    #[test]
    fn startup_order_replays_prepared_before_loading_catalog_and_identity() {
        let root = unique_root("legacy-lifecycle-startup");
        let paths = paths(&root);
        seed(&paths);
        let review = build_review(&paths, 1, Some(2)).unwrap();
        prepare(
            &paths,
            1,
            Some(2),
            &review.revision,
            "2026-08-03T19:00:00Z".parse().unwrap(),
        )
        .unwrap();
        assert!(has_prepared(&paths));

        replay_prepared(&paths).unwrap();
        let loaded = storage::try_load_categories_from_csv(&paths.categories_csv).unwrap();
        let ledger = load_ledger(&paths).unwrap();
        let next = next_category_id(loaded.next_category_id, &ledger).unwrap();
        assert!(!has_prepared(&paths));
        assert!(loaded.categories.iter().all(|category| category.id.0 != 1));
        assert_eq!(next, 3);

        fs::remove_dir_all(root).ok();
    }

'''
if marker not in content:
    raise SystemExit("legacy lifecycle startup test marker missing")
path.write_text(content.replace(marker, test + marker, 1))
