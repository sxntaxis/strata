from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:180]!r}")
    file.write_text(content.replace(old, new, 1))


replace_once(
    "src/legacy_category_lifecycle.rs",
    "    fn validate(&self) -> Result<(), String> {",
    "    pub(crate) fn validate(&self) -> Result<(), String> {",
)

replace_once(
    "src/sqlite/legacy_import.rs",
    '''use crate::{
    constants::COLORS,
    domain::{DRIFT_CATEGORY_CONFIG_NAME, is_drift_name},
};

use super::SqliteRepository;
''',
    '''use crate::{
    constants::COLORS,
    domain::{DRIFT_CATEGORY_CONFIG_NAME, is_drift_name},
    legacy_category_lifecycle::{
        LegacyCategoryLifecycleLedger, LegacyCategorySnapshot, LegacyLifecycleReceipt,
    },
};

use super::{
    SqliteRepository,
    category_lifecycle::{CategoryIdentitySnapshot, CategoryReferenceCounts},
};
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''    pub category_tags_json: PathBuf,
    pub sand_history_dir: PathBuf,
''',
    '''    pub category_tags_json: PathBuf,
    pub lifecycle_ledger_json: PathBuf,
    pub sand_history_dir: PathBuf,
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''            category_tags_json: state_dir.join("category_tags.json"),
            sand_history_dir: state_dir.join("sand_history"),
''',
    '''            category_tags_json: state_dir.join("category_tags.json"),
            lifecycle_ledger_json: state_dir.join("category_lifecycle_ledger.json"),
            sand_history_dir: state_dir.join("sand_history"),
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''struct LegacySnapshot {
    operational_day: String,
    captured_at_utc: String,
    payload_json: String,
}

#[derive(Debug, Clone)]
pub(super) struct LegacyImportPlan {
''',
    '''struct LegacySnapshot {
    operational_day: String,
    captured_at_utc: String,
    payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyLifecycleImport {
    operation_id: String,
    operation_kind: String,
    source_category_id: i64,
    target_category_id: Option<i64>,
    source_metadata_json: String,
    target_metadata_json: Option<String>,
    preview_revision: String,
    reference_counts_json: String,
    applied_at_utc: String,
}

#[derive(Debug, Clone)]
pub(super) struct LegacyImportPlan {
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''    snapshots: Vec<LegacySnapshot>,
    tags: Vec<LegacyTag>,
    summary: LegacyImportSummary,
''',
    '''    snapshots: Vec<LegacySnapshot>,
    tags: Vec<LegacyTag>,
    lifecycle_receipts: Vec<LegacyLifecycleImport>,
    summary: LegacyImportSummary,
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''        let tags = match sources.bytes.get("category_tags.json") {
            Some(bytes) => parse_tags(bytes, &category_ids)?,
            None => Vec::new(),
        };

        let total_elapsed_seconds = sessions.iter().map(|session| session.elapsed_seconds).sum();
''',
    '''        let tags = match sources.bytes.get("category_tags.json") {
            Some(bytes) => parse_tags(bytes, &category_ids)?,
            None => Vec::new(),
        };
        let lifecycle_receipts = match sources.bytes.get("category_lifecycle_ledger.json") {
            Some(bytes) => parse_lifecycle_receipts(bytes)?,
            None => Vec::new(),
        };

        let total_elapsed_seconds = sessions.iter().map(|session| session.elapsed_seconds).sum();
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''            snapshots,
            tags,
            summary,
''',
    '''            snapshots,
            tags,
            lifecycle_receipts,
            summary,
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''    collect_file(
        "category_tags.json",
        &paths.category_tags_json,
        &mut manifest_entries,
        &mut bytes,
    )?;

    if paths.sand_history_dir.exists() {
''',
    '''    collect_file(
        "category_tags.json",
        &paths.category_tags_json,
        &mut manifest_entries,
        &mut bytes,
    )?;
    collect_file(
        "category_lifecycle_ledger.json",
        &paths.lifecycle_ledger_json,
        &mut manifest_entries,
        &mut bytes,
    )?;

    if paths.sand_history_dir.exists() {
''',
)

parse_marker = '''fn parse_categories(bytes: &[u8]) -> Result<Vec<LegacyCategory>, LegacyImportError> {
'''
parse_functions = '''fn parse_lifecycle_receipts(
    bytes: &[u8],
) -> Result<Vec<LegacyLifecycleImport>, LegacyImportError> {
    let ledger: LegacyCategoryLifecycleLedger = serde_json::from_slice(bytes)
        .map_err(|error| json_error("category_lifecycle_ledger.json", error))?;
    ledger.validate().map_err(|message| {
        invalid("category_lifecycle_ledger.json", None, message)
    })?;
    ledger
        .receipts
        .iter()
        .map(lifecycle_receipt_import)
        .collect()
}

fn lifecycle_receipt_import(
    receipt: &LegacyLifecycleReceipt,
) -> Result<LegacyLifecycleImport, LegacyImportError> {
    let source = lifecycle_metadata(&receipt.source, receipt.applied_at_utc)?;
    let target = receipt
        .target
        .as_ref()
        .map(|snapshot| lifecycle_metadata(snapshot, receipt.applied_at_utc))
        .transpose()?;
    let counts = CategoryReferenceCounts {
        completed_sessions: receipt.references.completed_sessions,
        active_sessions: receipt.references.active_session,
        tags: receipt.references.tags,
        sand_placed: receipt.references.sand_placed,
        sand_pending: receipt.references.sand_pending,
        snapshot_placed: receipt.references.history_placed,
        snapshot_pending: receipt.references.history_pending,
        checkpoint_references: receipt.references.checkpoint_references,
    };
    Ok(LegacyLifecycleImport {
        operation_id: receipt.operation_id.clone(),
        operation_kind: receipt.operation_kind.clone(),
        source_category_id: i64::try_from(receipt.source.id).map_err(|_| {
            invalid(
                "category_lifecycle_ledger.json",
                None,
                "source category identity exceeds SQLite range",
            )
        })?,
        target_category_id: receipt
            .target
            .as_ref()
            .map(|target| i64::try_from(target.id))
            .transpose()
            .map_err(|_| {
                invalid(
                    "category_lifecycle_ledger.json",
                    None,
                    "target category identity exceeds SQLite range",
                )
            })?,
        source_metadata_json: serde_json::to_string(&source)
            .map_err(|error| json_error("category lifecycle source metadata", error))?,
        target_metadata_json: target
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| json_error("category lifecycle target metadata", error))?,
        preview_revision: receipt.preview_revision.clone(),
        reference_counts_json: serde_json::to_string(&counts)
            .map_err(|error| json_error("category lifecycle reference counts", error))?,
        applied_at_utc: format_utc(receipt.applied_at_utc),
    })
}

fn lifecycle_metadata(
    snapshot: &LegacyCategorySnapshot,
    applied_at_utc: DateTime<Utc>,
) -> Result<CategoryIdentitySnapshot, LegacyImportError> {
    Ok(CategoryIdentitySnapshot {
        id: i64::try_from(snapshot.id).map_err(|_| {
            invalid(
                "category_lifecycle_ledger.json",
                None,
                "category metadata identity exceeds SQLite range",
            )
        })?,
        name: snapshot.name.clone(),
        description: snapshot.description.clone(),
        color_index: i64::try_from(snapshot.color_index).map_err(|_| {
            invalid(
                "category_lifecycle_ledger.json",
                None,
                "category metadata color exceeds SQLite range",
            )
        })?,
        balance_effect: i64::from(snapshot.balance_effect),
        archived_at_utc: snapshot.archived.then(|| format_utc(applied_at_utc)),
        sort_order: i64::try_from(snapshot.id).map_err(|_| {
            invalid(
                "category_lifecycle_ledger.json",
                None,
                "category metadata sort identity exceeds SQLite range",
            )
        })?,
    })
}

'''
replace_once("src/sqlite/legacy_import.rs", parse_marker, parse_functions + parse_marker)

insert_marker = '''        for session in &plan.sessions {
'''
insert_receipts = '''        for receipt in &plan.lifecycle_receipts {
            transaction.execute(
                "INSERT INTO category_lifecycle_receipts (
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
replace_once("src/sqlite/legacy_import.rs", insert_marker, insert_receipts + insert_marker)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''        verify_import(&transaction, plan, import_id)?;
        let verification_json =
''',
    '''        verify_import(&transaction, plan, import_id)?;
        let lifecycle_count: i64 = transaction.query_row(
            "SELECT count(*) FROM category_lifecycle_receipts",
            [],
            |row| row.get(0),
        )?;
        let expected_lifecycle_count = i64::try_from(plan.lifecycle_receipts.len()).map_err(|_| {
            LegacyImportError::VerificationMismatch(
                "lifecycle receipt count exceeds SQLite range".to_string(),
            )
        })?;
        if lifecycle_count != expected_lifecycle_count {
            return Err(mismatch(
                "category lifecycle receipt count",
                &expected_lifecycle_count,
                &lifecycle_count,
            ));
        }
        let verification_json =
''',
)

# Fixture source custody and one migration proof.
replace_once(
    "src/sqlite/legacy_import.rs",
    '''                &self.paths.category_tags_json,
            ] {
''',
    '''                &self.paths.category_tags_json,
                &self.paths.lifecycle_ledger_json,
            ] {
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''        fixture.write_valid_sources();
        let before = fixture.source_bytes();
        let plan = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap();
''',
    '''        fixture.write_valid_sources();
        let lifecycle_ledger = LegacyCategoryLifecycleLedger {
            version: 1,
            receipts: vec![LegacyLifecycleReceipt {
                operation_id: "legacy-category-delete:3:none:retired-three".to_string(),
                operation_kind: "delete".to_string(),
                source: LegacyCategorySnapshot {
                    id: 3,
                    name: "Retired".to_string(),
                    description: "retired before migration".to_string(),
                    color_index: 4,
                    balance_effect: 0,
                    archived: false,
                },
                target: None,
                preview_revision: "retired-three".to_string(),
                references: crate::legacy_category_lifecycle::LegacyCategoryReferenceCounts::default(),
                applied_at_utc: "2026-08-01T14:00:00Z".parse().unwrap(),
            }],
        };
        crate::storage::write_json_atomic(
            &fixture.paths.lifecycle_ledger_json,
            &lifecycle_ledger,
        )
        .unwrap();
        let before = fixture.source_bytes();
        let plan = LegacyImportPlan::from_paths(&fixture.paths, options()).unwrap();
''',
)
replace_once(
    "src/sqlite/legacy_import.rs",
    '''        assert_eq!(repository.completed_session_count().unwrap(), 2);
    }
''',
    '''        assert_eq!(repository.completed_session_count().unwrap(), 2);
        let lifecycle_count: i64 = repository
            .connection
            .query_row(
                "SELECT count(*) FROM category_lifecycle_receipts
                 WHERE source_category_id = 3",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(lifecycle_count, 1);
        let next = repository
            .create_category(&crate::sqlite::repository::NewCategoryRecord {
                name: "After migration",
                description: "",
                color_index: 5,
                balance_effect: 0,
            })
            .unwrap();
        assert_eq!(next, 4, "migration must preserve retired identity high-water mark");
    }
''',
)
