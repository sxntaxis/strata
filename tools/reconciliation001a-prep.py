from pathlib import Path

path = Path("tools/reconciliation001a-apply.py")
text = path.read_text()
old = r'''marker = '''    #[test]
    fn imports_full_legacy_fixture_and_verifies_totals() {
'''
proof = r'''    #[test]
    fn imports_archived_category_catalog_without_reactivating_history() {
        let fixture = LegacyFixture::new("archived_category_catalog");
        fs::write(
            &fixture.paths.categories_csv,
            "id,name,description,color_index,karma_effect,archived\n1,Current,active,0,1,false\n2,Old Client,historical,1,-1,true\n",
        )
        .unwrap();
        fs::write(
            &fixture.paths.sessions_csv,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,2,Old Client,legacy work,10:00:00,11:00:00,3600\n",
        )
        .unwrap();

        let plan = LegacyImportPlan::from_paths(&fixture.paths, fixture.options()).unwrap();
        let mut repository = SqliteRepository::open(&fixture.database_path).unwrap();
        repository.import_legacy(&plan).unwrap();

        let archived: Option<String> = repository
            .connection
            .query_row(
                "SELECT archived_at_utc FROM categories WHERE id = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(archived.is_some());
        let category_id: i64 = repository
            .connection
            .query_row("SELECT category_id FROM sessions WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(category_id, 2);
    }

    #[test]
    fn imports_full_legacy_fixture_and_verifies_totals() {
'''
'''
new = r'''marker = '''    #[test]
    fn strict_import_preserves_sources_and_verifies_every_state_family() {
'''
proof = r'''    #[test]
    fn imports_archived_category_catalog_without_reactivating_history() {
        let fixture = Fixture::new("archived-category-catalog");
        fs::write(
            &fixture.paths.categories_csv,
            "id,name,description,color_index,karma_effect,archived\n1,Current,active,0,1,false\n2,Old Client,historical,1,-1,true\n",
        )
        .unwrap();
        fs::write(
            &fixture.paths.sessions_csv,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,2,Old Client,legacy work,10:00:00,11:00:00,3600\n",
        )
        .unwrap();

        let plan = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap();
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        repository.import_legacy(&plan).unwrap();

        let archived: Option<String> = repository
            .connection
            .query_row(
                "SELECT archived_at_utc FROM categories WHERE id = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(archived.is_some());
        let category_id: i64 = repository
            .connection
            .query_row("SELECT category_id FROM sessions WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(category_id, 2);
    }

    #[test]
    fn strict_import_preserves_sources_and_verifies_every_state_family() {
'''
'''
if old not in text:
    raise SystemExit("legacy importer proof template was not found")
path.write_text(text.replace(old, new, 1))
Path(__file__).unlink(missing_ok=True)
