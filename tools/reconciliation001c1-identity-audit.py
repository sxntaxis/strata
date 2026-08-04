from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


# Repository identity allocation and snapshot custody.
replace_once(
    "src/sqlite/repository.rs",
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepositorySnapshot {
    pub categories: Vec<CategoryRecord>,
    pub category_tags: BTreeMap<i64, Vec<String>>,
    pub sessions: Vec<SessionRecord>,
    pub active_session: Option<ActiveSessionRecord>,
    pub checkpoint: Option<CheckpointRecord>,
    pub sand_state: Option<SandStateRecord>,
    pub sand_snapshots: Vec<SandSnapshotRecord>,
}
''',
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryLifecycleReceiptRecord {
    pub operation_id: String,
    pub operation_kind: String,
    pub source_category_id: i64,
    pub target_category_id: Option<i64>,
    pub source_metadata_json: String,
    pub target_metadata_json: Option<String>,
    pub preview_revision: String,
    pub reference_counts_json: String,
    pub applied_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepositorySnapshot {
    pub categories: Vec<CategoryRecord>,
    pub category_tags: BTreeMap<i64, Vec<String>>,
    pub sessions: Vec<SessionRecord>,
    pub active_session: Option<ActiveSessionRecord>,
    pub checkpoint: Option<CheckpointRecord>,
    pub sand_state: Option<SandStateRecord>,
    pub sand_snapshots: Vec<SandSnapshotRecord>,
    pub category_lifecycle_receipts: Vec<CategoryLifecycleReceiptRecord>,
}
''',
)
replace_once(
    "src/sqlite/repository.rs",
    '''        transaction.execute(
            "INSERT INTO categories(name, description, color_index, balance_effect)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                category.name.trim(),
                category.description,
                category.color_index,
                category.balance_effect,
            ],
        )?;
        let id = transaction.last_insert_rowid();
''',
    '''        let maximum_identity: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(identity), 0)
             FROM (
                 SELECT id AS identity FROM categories
                 UNION ALL
                 SELECT source_category_id AS identity FROM category_lifecycle_receipts
                 UNION ALL
                 SELECT target_category_id AS identity FROM category_lifecycle_receipts
                 WHERE target_category_id IS NOT NULL
             )",
            [],
            |row| row.get(0),
        )?;
        let id = maximum_identity.checked_add(1).ok_or_else(|| {
            RepositoryError::InvalidInput("category identity space is exhausted".to_string())
        })?;
        transaction.execute(
            "INSERT INTO categories(id, name, description, color_index, balance_effect)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                category.name.trim(),
                category.description,
                category.color_index,
                category.balance_effect,
            ],
        )?;
''',
)
replace_once(
    "src/sqlite/repository.rs",
    '''            sand_state: query_sand_state(&transaction)?,
            sand_snapshots: query_sand_snapshots(&transaction)?,
        };
''',
    '''            sand_state: query_sand_state(&transaction)?,
            sand_snapshots: query_sand_snapshots(&transaction)?,
            category_lifecycle_receipts: query_category_lifecycle_receipts(&transaction)?,
        };
''',
)
repository_marker = '''fn validate_category(category: &NewCategoryRecord<'_>) -> Result<(), RepositoryError> {
'''
repository_query = '''fn query_category_lifecycle_receipts(
    connection: &Connection,
) -> Result<Vec<CategoryLifecycleReceiptRecord>, RepositoryError> {
    let mut statement = connection.prepare(
        "SELECT operation_id, operation_kind, source_category_id, target_category_id,
                source_metadata_json, target_metadata_json, preview_revision,
                reference_counts_json, applied_at_utc
         FROM category_lifecycle_receipts
         ORDER BY applied_at_utc, operation_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(CategoryLifecycleReceiptRecord {
            operation_id: row.get(0)?,
            operation_kind: row.get(1)?,
            source_category_id: row.get(2)?,
            target_category_id: row.get(3)?,
            source_metadata_json: row.get(4)?,
            target_metadata_json: row.get(5)?,
            preview_revision: row.get(6)?,
            reference_counts_json: row.get(7)?,
            applied_at_utc: row.get(8)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

'''
replace_once("src/sqlite/repository.rs", repository_marker, repository_query + repository_marker)

# Verify retired identities are never reused.
replace_once(
    "src/sqlite/category_lifecycle.rs",
    '''        assert_eq!(receipt.operation_kind, "delete");
        assert!(query_category(&empty.connection, 1).unwrap().is_none());
    }
''',
    '''        assert_eq!(receipt.operation_kind, "delete");
        assert!(query_category(&empty.connection, 1).unwrap().is_none());
        let replacement = empty
            .create_category(&NewCategoryRecord {
                name: "Replacement",
                description: "",
                color_index: 2,
                balance_effect: 0,
            })
            .unwrap();
        assert_eq!(replacement, 2, "retired stable identity must not be reused");
    }
''',
)
replace_once(
    "src/sqlite/category_lifecycle.rs",
    '''        assert_eq!(receipt_count, 1);
    }
''',
    '''        assert_eq!(receipt_count, 1);
        let next = repository
            .create_category(&NewCategoryRecord {
                name: "After merge",
                description: "",
                color_index: 3,
                balance_effect: 0,
            })
            .unwrap();
        assert_eq!(next, 3, "merged source identity must remain retired");
    }
''',
)

# Portable bundle schema 3 preserves lifecycle receipts and identity retirement.
replace_once(
    "src/sqlite/maintenance.rs",
    '''        ActiveSessionRecord, CategoryRecord, CheckpointRecord, CheckpointStatus,
        RepositorySnapshot, SandSnapshotRecord, SandStateRecord, SessionRecord, SnapshotKind,
''',
    '''        ActiveSessionRecord, CategoryLifecycleReceiptRecord, CategoryRecord, CheckpointRecord,
        CheckpointStatus, RepositorySnapshot, SandSnapshotRecord, SandStateRecord, SessionRecord,
        SnapshotKind,
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    "const PORTABLE_BUNDLE_SCHEMA_VERSION: u8 = 2;",
    "const PORTABLE_BUNDLE_SCHEMA_VERSION: u8 = 3;",
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''const SAND_SNAPSHOTS_FILENAME: &str = "sand_snapshots.csv";
const BUNDLE_FILES: [&str; 7] = [
''',
    '''const SAND_SNAPSHOTS_FILENAME: &str = "sand_snapshots.csv";
const CATEGORY_LIFECYCLE_RECEIPTS_FILENAME: &str = "category_lifecycle_receipts.csv";
const BUNDLE_FILES: [&str; 8] = [
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''    SAND_STATE_FILENAME,
    SAND_SNAPSHOTS_FILENAME,
];
''',
    '''    SAND_STATE_FILENAME,
    SAND_SNAPSHOTS_FILENAME,
    CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
];
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''    pub sand_snapshots: usize,
    pub total_elapsed_seconds: i64,
''',
    '''    pub sand_snapshots: usize,
    pub category_lifecycle_receipts: usize,
    pub total_elapsed_seconds: i64,
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''            sand_snapshots: snapshot.sand_snapshots.len(),
            total_elapsed_seconds,
''',
    '''            sand_snapshots: snapshot.sand_snapshots.len(),
            category_lifecycle_receipts: snapshot.category_lifecycle_receipts.len(),
            total_elapsed_seconds,
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''        "legacy_imports",
        "category_tags",
    ];
''',
    '''        "legacy_imports",
        "category_tags",
        "category_lifecycle_receipts",
    ];
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''    files.insert(
        SAND_SNAPSHOTS_FILENAME,
        serialize_sand_snapshots(&snapshot.sand_snapshots)?,
    );
    Ok(files)
''',
    '''    files.insert(
        SAND_SNAPSHOTS_FILENAME,
        serialize_sand_snapshots(&snapshot.sand_snapshots)?,
    );
    files.insert(
        CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
        serialize_category_lifecycle_receipts(&snapshot.category_lifecycle_receipts)?,
    );
    Ok(files)
''',
)
serialize_marker = '''fn write_record<I, T>(
'''
serialize_receipts = '''fn serialize_category_lifecycle_receipts(
    receipts: &[CategoryLifecycleReceiptRecord],
) -> Result<Vec<u8>, MaintenanceError> {
    let mut writer = csv_writer().from_writer(Vec::new());
    write_record(
        &mut writer,
        CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
        [
            "operation_id",
            "operation_kind",
            "source_category_id",
            "target_category_id",
            "source_metadata_json",
            "target_metadata_json",
            "preview_revision",
            "reference_counts_json",
            "applied_at_utc",
        ],
    )?;
    for receipt in receipts {
        write_record(
            &mut writer,
            CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
            [
                receipt.operation_id.clone(),
                receipt.operation_kind.clone(),
                receipt.source_category_id.to_string(),
                receipt
                    .target_category_id
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                receipt.source_metadata_json.clone(),
                receipt.target_metadata_json.clone().unwrap_or_default(),
                receipt.preview_revision.clone(),
                receipt.reference_counts_json.clone(),
                receipt.applied_at_utc.clone(),
            ],
        )?;
    }
    finish_writer(writer, CATEGORY_LIFECYCLE_RECEIPTS_FILENAME)
}

'''
replace_once("src/sqlite/maintenance.rs", serialize_marker, serialize_receipts + serialize_marker)
replace_once(
    "src/sqlite/maintenance.rs",
    '''        sand_snapshots: parse_sand_snapshots(required_file(files, SAND_SNAPSHOTS_FILENAME)?)?,
    })
''',
    '''        sand_snapshots: parse_sand_snapshots(required_file(files, SAND_SNAPSHOTS_FILENAME)?)?,
        category_lifecycle_receipts: parse_category_lifecycle_receipts(required_file(
            files,
            CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
        )?)?,
    })
''',
)
parse_marker = '''fn validate_snapshot_references(snapshot: &RepositorySnapshot) -> Result<(), MaintenanceError> {
'''
parse_receipts = '''fn parse_category_lifecycle_receipts(
    bytes: &[u8],
) -> Result<Vec<CategoryLifecycleReceiptRecord>, MaintenanceError> {
    let records = csv_records(
        CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
        bytes,
        &[
            "operation_id",
            "operation_kind",
            "source_category_id",
            "target_category_id",
            "source_metadata_json",
            "target_metadata_json",
            "preview_revision",
            "reference_counts_json",
            "applied_at_utc",
        ],
    )?;
    let mut receipts = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        receipts.push(CategoryLifecycleReceiptRecord {
            operation_id: field(record, 0)?.to_string(),
            operation_kind: field(record, 1)?.to_string(),
            source_category_id: parse_i64(
                CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
                index,
                field(record, 2)?,
                "source_category_id",
            )?,
            target_category_id: optional_i64(
                CATEGORY_LIFECYCLE_RECEIPTS_FILENAME,
                index,
                field(record, 3)?,
                "target_category_id",
            )?,
            source_metadata_json: field(record, 4)?.to_string(),
            target_metadata_json: optional_string(field(record, 5)?),
            preview_revision: field(record, 6)?.to_string(),
            reference_counts_json: field(record, 7)?.to_string(),
            applied_at_utc: field(record, 8)?.to_string(),
        });
    }
    Ok(receipts)
}

'''
replace_once("src/sqlite/maintenance.rs", parse_marker, parse_receipts + parse_marker)
replace_once(
    "src/sqlite/maintenance.rs",
    '''    let mut category_ids = BTreeSet::new();
    let mut active_names = BTreeSet::new();
''',
    '''    let mut category_ids = BTreeSet::new();
    let mut retired_category_ids = BTreeSet::new();
    let mut lifecycle_operation_ids = BTreeSet::new();
    for receipt in &snapshot.category_lifecycle_receipts {
        require_text(&receipt.operation_id, "category lifecycle operation id")?;
        require_text(&receipt.preview_revision, "category lifecycle preview revision")?;
        require_text(&receipt.applied_at_utc, "category lifecycle application timestamp")?;
        if !lifecycle_operation_ids.insert(receipt.operation_id.as_str()) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate category lifecycle operation id {}",
                receipt.operation_id
            )));
        }
        if receipt.source_category_id <= 0 {
            return Err(MaintenanceError::InvalidBundle(
                "category lifecycle receipt source must be a positive identity".to_string(),
            ));
        }
        retired_category_ids.insert(receipt.source_category_id);
        match receipt.operation_kind.as_str() {
            "merge" => {
                let target = receipt.target_category_id.ok_or_else(|| {
                    MaintenanceError::InvalidBundle(
                        "merge receipt has no target category identity".to_string(),
                    )
                })?;
                if target == receipt.source_category_id || target <= 0 {
                    return Err(MaintenanceError::InvalidBundle(
                        "merge receipt has an invalid target category identity".to_string(),
                    ));
                }
                if receipt.target_metadata_json.is_none() {
                    return Err(MaintenanceError::InvalidBundle(
                        "merge receipt has no target metadata".to_string(),
                    ));
                }
            }
            "delete" => {
                if receipt.target_category_id.is_some() || receipt.target_metadata_json.is_some() {
                    return Err(MaintenanceError::InvalidBundle(
                        "delete receipt unexpectedly names a target category".to_string(),
                    ));
                }
            }
            other => {
                return Err(MaintenanceError::InvalidBundle(format!(
                    "unknown category lifecycle operation kind {other}"
                )));
            }
        }
        validate_json(&receipt.source_metadata_json, "category lifecycle source metadata")?;
        if let Some(target) = &receipt.target_metadata_json {
            validate_json(target, "category lifecycle target metadata")?;
        }
        validate_json(
            &receipt.reference_counts_json,
            "category lifecycle reference counts",
        )?;
    }

    let mut active_names = BTreeSet::new();
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''        if !category_ids.insert(category.id) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate category id {}",
                category.id
            )));
        }
''',
    '''        if !category_ids.insert(category.id) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate category id {}",
                category.id
            )));
        }
        if retired_category_ids.contains(&category.id) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "category id {} was retired by a lifecycle receipt and cannot be active",
                category.id
            )));
        }
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''    for item in &snapshot.sand_snapshots {
        transaction.execute(
''',
    '''    for item in &snapshot.sand_snapshots {
        transaction.execute(
''',
)
import_marker = '''    transaction.execute(
        "INSERT INTO database_metadata(key, value)
'''
import_receipts = '''    for receipt in &snapshot.category_lifecycle_receipts {
        transaction.execute(
            "INSERT INTO category_lifecycle_receipts(
                operation_id, operation_kind, source_category_id, target_category_id,
                source_metadata_json, target_metadata_json, preview_revision,
                reference_counts_json, applied_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                receipt.operation_id,
                receipt.operation_kind,
                receipt.source_category_id,
                receipt.target_category_id,
                receipt.source_metadata_json,
                receipt.target_metadata_json,
                receipt.preview_revision,
                receipt.reference_counts_json,
                receipt.applied_at_utc,
            ],
        )?;
    }

'''
replace_once("src/sqlite/maintenance.rs", import_marker, import_receipts + import_marker)

# Correct the working contract: unresolved recovery receipts block C1 rather than being rewritten.
replace_once(
    "notebook/work/RECONCILIATION-001C1.md",
    '''- remap the runtime checkpoint payload, including active identity, sediment, queued switch mutations, and legacy transition/finish/clear receipts;
''',
    '''- remap receipt-free runtime checkpoint payload identity, sediment, and queued switch mutations; unresolved transition/finish/clear receipts block the operation because their deterministic operation identities bind the original categories;
''',
)
replace_once(
    "notebook/work/RECONCILIATION-001C1.md",
    '''- migration, backup, and restore flows preserve the new integrity rules;
''',
    '''- migration, backup, restore, and portable bundle round-trip preserve lifecycle receipts and prevent retired category ID reuse;
''',
) if "- migration, backup, and restore flows preserve the new integrity rules;\n" in Path("notebook/work/RECONCILIATION-001C1.md").read_text() else None
