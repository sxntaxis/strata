from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


replace_once(
    "src/sqlite/category_lifecycle.rs",
    '''        assert_eq!((completed_category, active_category), (2, 2));
        let tags = repository.category_tags().unwrap();
''',
    '''        assert_eq!((completed_category, active_category), (2, 2));
        let completed_identity: (String, String, String, String, String, i64) = repository
            .connection
            .query_row(
                "SELECT stable_id, project, description, started_at_utc, ended_at_utc,
                        elapsed_seconds
                 FROM sessions WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            completed_identity,
            (
                "session-source".to_string(),
                "Project".to_string(),
                "completed".to_string(),
                "2026-08-03T16:00:00Z".to_string(),
                "2026-08-03T17:00:00Z".to_string(),
                3600,
            )
        );
        let active_identity: (String, String, String, String) = repository
            .connection
            .query_row(
                "SELECT stable_id, description, started_at_utc, recovery_kind
                 FROM active_session WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            active_identity,
            (
                "active-source".to_string(),
                "active".to_string(),
                "2026-08-03T18:00:00Z".to_string(),
                "live".to_string(),
            )
        );
        let target = query_category(&repository.connection, 2)
            .unwrap()
            .unwrap();
        assert_eq!(target.name, "Target");
        assert_eq!(target.description, "target metadata");
        assert_eq!(target.color_index, 2);
        assert_eq!(target.balance_effect, -1);
        let tags = repository.category_tags().unwrap();
''',
)
replace_once(
    "src/sqlite/category_lifecycle.rs",
    '''        assert_eq!(receipt_count, 1);
        let next = repository
''',
    '''        assert_eq!(receipt_count, 1);
        let retry = apply(
            &mut repository,
            CategoryLifecycleRequest {
                source_category_id: 1,
                target_category_id: Some(2),
                expected_revision: &receipt.preview_revision,
                applied_at_utc: "2026-08-03T20:00:00Z",
            },
        )
        .unwrap();
        assert!(retry.already_applied);
        assert_eq!(retry.operation_id, receipt.operation_id);
        let receipt_count_after_retry: i64 = repository
            .connection
            .query_row(
                "SELECT count(*) FROM category_lifecycle_receipts",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(receipt_count_after_retry, 1);
        let next = repository
''',
)

replace_once(
    "src/sqlite/maintenance.rs",
    '''        assert_eq!(collect_files(&bundle_a), collect_files(&bundle_b));

        import_bundle(BundleImportOptions {
''',
    '''        assert_eq!(collect_files(&bundle_a), collect_files(&bundle_b));
        assert!(bundle_a.join(CATEGORY_LIFECYCLE_RECEIPTS_FILENAME).exists());

        import_bundle(BundleImportOptions {
''',
)
doctor_test_marker = '''    #[test]
    fn doctor_detects_unsupported_schema_and_foreign_key_damage() {
'''
doctor_test = '''    #[test]
    fn doctor_rejects_tampered_lifecycle_receipts_and_retired_identity_reuse() {
        let root = unique_root("lifecycle_doctor");
        fs::create_dir_all(&root).unwrap();
        let database = root.join("lifecycle.sqlite3");
        fixture_database(&database);
        assert!(doctor_at(&database, None).unwrap().is_healthy());

        let connection = Connection::open(&database).unwrap();
        connection
            .execute(
                "UPDATE category_lifecycle_receipts
                 SET source_metadata_json = '{}'
                 WHERE source_category_id = 2",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO categories(
                    id, name, description, color_index, balance_effect, archived_at_utc,
                    sort_order
                 ) VALUES (2, 'Reused', '', 1, 0, NULL, 99)",
                [],
            )
            .unwrap();
        drop(connection);

        let report = doctor_at(&database, None).unwrap();
        assert!(!report.is_healthy());
        let lifecycle = report
            .checks
            .iter()
            .find(|check| check.name == "category-lifecycle-integrity")
            .unwrap();
        assert!(!lifecycle.passed);
        assert!(lifecycle.detail.contains("source metadata"));
        assert!(lifecycle.detail.contains("retired category identity 2"));

        let _ = fs::remove_dir_all(root);
    }

'''
replace_once("src/sqlite/maintenance.rs", doctor_test_marker, doctor_test + doctor_test_marker)
