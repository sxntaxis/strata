from pathlib import Path

path = Path("src/sqlite/maintenance.rs")
content = path.read_text()
start_marker = "    let mut category_ids = BTreeSet::new();\n    let mut retired_category_ids = BTreeSet::new();\n"
end_marker = "    let mut active_names = BTreeSet::new();\n"
start = content.find(start_marker)
if start < 0:
    raise SystemExit("formatted lifecycle receipt loop start missing")
end = content.find(end_marker, start)
if end < 0:
    raise SystemExit("formatted lifecycle receipt loop end missing")
normalized = '''    let mut category_ids = BTreeSet::new();
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

'''
path.write_text(content[:start] + normalized + content[end:])

path = Path("src/sqlite/category_lifecycle.rs")
content = path.read_text()
formatted = '''        assert_eq!(
            count_checkpoint_category_references(&checkpoint, 1).unwrap(),
            0
        );
        assert!(count_checkpoint_category_references(&checkpoint, 2).unwrap() > 0);

        let residual = reference_counts_on(&repository.connection, 1).unwrap();
'''
normalized = '''        assert_eq!(count_checkpoint_category_references(&checkpoint, 1).unwrap(), 0);
        assert!(count_checkpoint_category_references(&checkpoint, 2).unwrap() > 0);

        let residual = reference_counts_on(&repository.connection, 1).unwrap();
'''
if formatted not in content:
    raise SystemExit("formatted lifecycle checkpoint assertion missing")
path.write_text(content.replace(formatted, normalized, 1))
