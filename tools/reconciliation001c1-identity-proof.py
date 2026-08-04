from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


replace_once(
    "src/sqlite/maintenance.rs",
    '''    use crate::sqlite::{
        NewActiveSession,
        repository::{NewCategoryRecord, NewSandSnapshotRecord, NewSessionRecord},
    };
''',
    '''    use crate::sqlite::{
        NewActiveSession,
        category_lifecycle::{CategoryLifecycleRequest, apply, preview},
        repository::{NewCategoryRecord, NewSandSnapshotRecord, NewSessionRecord},
    };
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''        let study_id = repository
            .create_category(&NewCategoryRecord {
                name: "Study",
                description: "Deep reading",
                color_index: 2,
                balance_effect: 1,
            })
            .unwrap();
        repository
            .replace_category_tags(study_id, &["reading".to_string(), "focus".to_string()])
''',
    '''        let study_id = repository
            .create_category(&NewCategoryRecord {
                name: "Study",
                description: "Deep reading",
                color_index: 2,
                balance_effect: 1,
            })
            .unwrap();
        let disposable_id = repository
            .create_category(&NewCategoryRecord {
                name: "Disposable",
                description: "retired before export",
                color_index: 3,
                balance_effect: 0,
            })
            .unwrap();
        let lifecycle_preview = preview(&repository, disposable_id, None).unwrap();
        apply(
            &mut repository,
            CategoryLifecycleRequest {
                source_category_id: disposable_id,
                target_category_id: None,
                expected_revision: &lifecycle_preview.revision,
                applied_at_utc: "2026-08-01T14:00:00Z",
            },
        )
        .unwrap();
        repository
            .replace_category_tags(study_id, &["reading".to_string(), "focus".to_string()])
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''        assert_eq!(read_repository_snapshot(&imported).unwrap(), expected);
        assert!(doctor_at(&imported, None).unwrap().is_healthy());

        let _ = fs::remove_dir_all(root);
''',
    '''        assert_eq!(read_repository_snapshot(&imported).unwrap(), expected);
        assert!(doctor_at(&imported, None).unwrap().is_healthy());
        let mut imported_repository = SqliteRepository::open(&imported).unwrap();
        let post_import_id = imported_repository
            .create_category(&NewCategoryRecord {
                name: "After bundle",
                description: "",
                color_index: 4,
                balance_effect: 0,
            })
            .unwrap();
        assert_eq!(
            post_import_id, 3,
            "portable round-trip must preserve retired category identity"
        );

        let _ = fs::remove_dir_all(root);
''',
)
