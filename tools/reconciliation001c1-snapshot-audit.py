from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


# Repository and portable interchange must understand the schema-six daily contribution kind.
replace_once(
    "src/sqlite/repository.rs",
    '''pub(crate) enum SnapshotKind {
    Daily,
    Manual,
''',
    '''pub(crate) enum SnapshotKind {
    Daily,
    DailyContribution,
    Manual,
''',
)
replace_once(
    "src/sqlite/repository.rs",
    '''        match self {
            Self::Daily => "daily",
            Self::Manual => "manual",
''',
    '''        match self {
            Self::Daily => "daily",
            Self::DailyContribution => "daily-contribution",
            Self::Manual => "manual",
''',
)
replace_once(
    "src/sqlite/repository.rs",
    '''        match value {
            "daily" => Ok(Self::Daily),
            "manual" => Ok(Self::Manual),
''',
    '''        match value {
            "daily" => Ok(Self::Daily),
            "daily-contribution" => Ok(Self::DailyContribution),
            "manual" => Ok(Self::Manual),
''',
)
replace_once(
    "src/sqlite/repository.rs",
    '''    if snapshot.snapshot_kind == SnapshotKind::Daily && snapshot.operational_day.is_none() {
''',
    '''    if matches!(
        snapshot.snapshot_kind,
        SnapshotKind::Daily | SnapshotKind::DailyContribution
    ) && snapshot.operational_day.is_none()
    {
''',
)

replace_once(
    "src/sqlite/maintenance.rs",
    '''use csv::{ReaderBuilder, StringRecord, Terminator, WriterBuilder};
''',
    '''use chrono::DateTime;
use csv::{ReaderBuilder, StringRecord, Terminator, WriterBuilder};
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''use super::{
    CURRENT_SCHEMA_VERSION, SqliteRepository,
''',
    '''use super::{
    CURRENT_SCHEMA_VERSION, SqliteRepository,
    category_lifecycle::{CategoryIdentitySnapshot, CategoryReferenceCounts},
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''    match kind {
        SnapshotKind::Daily => "daily",
        SnapshotKind::Manual => "manual",
''',
    '''    match kind {
        SnapshotKind::Daily => "daily",
        SnapshotKind::DailyContribution => "daily-contribution",
        SnapshotKind::Manual => "manual",
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''    match value {
        "daily" => Ok(SnapshotKind::Daily),
        "manual" => Ok(SnapshotKind::Manual),
''',
    '''    match value {
        "daily" => Ok(SnapshotKind::Daily),
        "daily-contribution" => Ok(SnapshotKind::DailyContribution),
        "manual" => Ok(SnapshotKind::Manual),
''',
)
replace_once(
    "src/sqlite/maintenance.rs",
    '''        if item.snapshot_kind == SnapshotKind::Daily && item.operational_day.is_none() {
''',
    '''        if matches!(
            item.snapshot_kind,
            SnapshotKind::Daily | SnapshotKind::DailyContribution
        ) && item.operational_day.is_none()
        {
''',
)

# One validator owns receipt shape, metadata identity, timestamp, and counts.
old_receipt_loop = '''    let mut category_ids = BTreeSet::new();
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
'''
new_receipt_loop = '''    let mut category_ids = BTreeSet::new();
    let mut retired_category_ids = BTreeSet::new();
    let mut lifecycle_operation_ids = BTreeSet::new();
    for receipt in &snapshot.category_lifecycle_receipts {
        validate_category_lifecycle_receipt(receipt)?;
        if !lifecycle_operation_ids.insert(receipt.operation_id.as_str()) {
            return Err(MaintenanceError::InvalidBundle(format!(
                "duplicate category lifecycle operation id {}",
                receipt.operation_id
            )));
        }
        retired_category_ids.insert(receipt.source_category_id);
    }

    let mut active_names = BTreeSet::new();
'''
replace_once("src/sqlite/maintenance.rs", old_receipt_loop, new_receipt_loop)
validator_marker = '''fn validate_snapshot_references(snapshot: &RepositorySnapshot) -> Result<(), MaintenanceError> {
'''
validator = '''fn validate_category_lifecycle_receipt(
    receipt: &CategoryLifecycleReceiptRecord,
) -> Result<(), MaintenanceError> {
    require_text(&receipt.operation_id, "category lifecycle operation id")?;
    require_text(&receipt.preview_revision, "category lifecycle preview revision")?;
    require_text(&receipt.applied_at_utc, "category lifecycle application timestamp")?;
    DateTime::parse_from_rfc3339(&receipt.applied_at_utc).map_err(|error| {
        MaintenanceError::InvalidBundle(format!(
            "category lifecycle application timestamp is invalid: {error}"
        ))
    })?;
    if receipt.source_category_id <= 0 {
        return Err(MaintenanceError::InvalidBundle(
            "category lifecycle receipt source must be a positive identity".to_string(),
        ));
    }
    let source: CategoryIdentitySnapshot = serde_json::from_str(&receipt.source_metadata_json)
        .map_err(|error| {
            MaintenanceError::InvalidBundle(format!(
                "category lifecycle source metadata is invalid: {error}"
            ))
        })?;
    if source.id != receipt.source_category_id {
        return Err(MaintenanceError::InvalidBundle(
            "category lifecycle source metadata identity does not match its receipt".to_string(),
        ));
    }
    let _counts: CategoryReferenceCounts = serde_json::from_str(&receipt.reference_counts_json)
        .map_err(|error| {
            MaintenanceError::InvalidBundle(format!(
                "category lifecycle reference counts are invalid: {error}"
            ))
        })?;
    match receipt.operation_kind.as_str() {
        "merge" => {
            let target_id = receipt.target_category_id.ok_or_else(|| {
                MaintenanceError::InvalidBundle(
                    "merge receipt has no target category identity".to_string(),
                )
            })?;
            if target_id <= 0 || target_id == receipt.source_category_id {
                return Err(MaintenanceError::InvalidBundle(
                    "merge receipt has an invalid target category identity".to_string(),
                ));
            }
            let target_json = receipt.target_metadata_json.as_deref().ok_or_else(|| {
                MaintenanceError::InvalidBundle(
                    "merge receipt has no target metadata".to_string(),
                )
            })?;
            let target: CategoryIdentitySnapshot = serde_json::from_str(target_json).map_err(
                |error| {
                    MaintenanceError::InvalidBundle(format!(
                        "category lifecycle target metadata is invalid: {error}"
                    ))
                },
            )?;
            if target.id != target_id {
                return Err(MaintenanceError::InvalidBundle(
                    "category lifecycle target metadata identity does not match its receipt"
                        .to_string(),
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
    Ok(())
}

fn database_category_lifecycle_issues(
    connection: &Connection,
) -> Result<Vec<String>, MaintenanceError> {
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
    let mut issues = Vec::new();
    let mut operation_ids = BTreeSet::new();
    for row in rows {
        let receipt = row?;
        if !operation_ids.insert(receipt.operation_id.clone()) {
            issues.push(format!(
                "duplicate lifecycle operation id {}",
                receipt.operation_id
            ));
        }
        if let Err(error) = validate_category_lifecycle_receipt(&receipt) {
            issues.push(error.to_string());
        }
    }
    let mut collision_statement = connection.prepare(
        "SELECT categories.id
         FROM categories
         JOIN category_lifecycle_receipts
           ON category_lifecycle_receipts.source_category_id = categories.id
         ORDER BY categories.id",
    )?;
    let collisions = collision_statement.query_map([], |row| row.get::<_, i64>(0))?;
    for collision in collisions {
        issues.push(format!(
            "retired category identity {} is present in the active catalog",
            collision?
        ));
    }
    Ok(issues)
}

'''
replace_once("src/sqlite/maintenance.rs", validator_marker, validator + validator_marker)

# Doctor reports malformed lifecycle authority rather than treating table presence as sufficient.
doctor_marker = '''    let metadata_authority = if existing_tables.contains("database_metadata") {
'''
doctor_check = '''    let lifecycle_issues = if existing_tables.contains("category_lifecycle_receipts") {
        database_category_lifecycle_issues(&connection)?
    } else {
        vec!["category_lifecycle_receipts table missing".to_string()]
    };
    checks.push(check(
        "category-lifecycle-integrity",
        lifecycle_issues.is_empty(),
        if lifecycle_issues.is_empty() {
            "all lifecycle receipts and retired identities are coherent".to_string()
        } else {
            lifecycle_issues.join("; ")
        },
    ));

'''
replace_once("src/sqlite/maintenance.rs", doctor_marker, doctor_check + doctor_marker)

# The lifecycle success proof must inspect regenerated daily contribution identity.
replace_once(
    "src/sqlite/category_lifecycle.rs",
    '''        assert_eq!(count_checkpoint_category_references(&checkpoint, 1).unwrap(), 0);
        assert!(count_checkpoint_category_references(&checkpoint, 2).unwrap() > 0);

        let residual = reference_counts_on(&repository.connection, 1).unwrap();
''',
    '''        assert_eq!(count_checkpoint_category_references(&checkpoint, 1).unwrap(), 0);
        assert!(count_checkpoint_category_references(&checkpoint, 2).unwrap() > 0);

        let daily_json: String = repository
            .connection
            .query_row(
                "SELECT payload_json FROM sand_snapshots
                 WHERE snapshot_kind = 'daily-contribution'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let daily: SedimentSnapshot = serde_json::from_str(&daily_json).unwrap();
        assert_eq!(count_snapshot_category(&daily, 1).unwrap().total().unwrap(), 0);
        assert!(count_snapshot_category(&daily, 2).unwrap().total().unwrap() > 0);

        let residual = reference_counts_on(&repository.connection, 1).unwrap();
''',
)

# Schema migration proof now verifies the repository can read the accepted kind.
replace_once(
    "src/sqlite.rs",
    '''            .expect("schema 7 must retain typed daily contributions");
    }
''',
    '''            .expect("schema 7 must retain typed daily contributions");
        let snapshots = repository.list_sand_snapshots().unwrap();
        assert!(snapshots
            .iter()
            .any(|snapshot| snapshot.snapshot_kind == repository::SnapshotKind::DailyContribution));
    }
''',
)
