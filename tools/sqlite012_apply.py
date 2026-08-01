from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:80]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


# maintenance: dry-run is a complete import/reconciliation into a disposable database.
replace_once(
    "src/sqlite/maintenance.rs",
    "    process,\n};",
    "    process,\n    time::{SystemTime, UNIX_EPOCH},\n};",
)
replace_once(
    "src/sqlite/maintenance.rs",
    "pub(crate) struct BundleImportOptions {\n    pub bundle_directory: PathBuf,\n    pub database_path: PathBuf,\n}",
    "pub(crate) struct BundleImportOptions {\n    pub bundle_directory: PathBuf,\n    pub database_path: PathBuf,\n    pub dry_run: bool,\n}",
)
new_import = r'''pub(super) fn import_bundle(
    options: BundleImportOptions,
) -> Result<SqliteMaintenanceReport, MaintenanceError> {
    let bundle_directory = absolute_existing_path(&options.bundle_directory)?;
    if !bundle_directory.is_dir() {
        return Err(MaintenanceError::InvalidBundle(format!(
            "{} is not a directory",
            display_path(&bundle_directory)
        )));
    }
    let (manifest, snapshot) = read_bundle(&bundle_directory)?;
    if manifest.database_schema_version != CURRENT_SCHEMA_VERSION {
        return Err(MaintenanceError::InvalidBundle(format!(
            "bundle schema target {} does not match supported SQLite schema {}",
            manifest.database_schema_version, CURRENT_SCHEMA_VERSION
        )));
    }

    if options.dry_run {
        let temporary_path = dry_run_import_path()?;
        remove_database_artifacts(&temporary_path);
        let result = (|| {
            let schema_version = validate_import_candidate(
                &temporary_path,
                &snapshot,
                &manifest.bundle_fingerprint,
            )?;
            Ok(SqliteMaintenanceReport {
                schema_version: MAINTENANCE_REPORT_SCHEMA_VERSION,
                operation: "sqlite-import".to_string(),
                status: "validated".to_string(),
                source_path: Some(display_path(&bundle_directory)),
                target_path: None,
                previous_database_path: None,
                bundle_fingerprint: Some(manifest.bundle_fingerprint.clone()),
                database_schema_version: Some(schema_version),
                counts: Some(manifest.counts.clone()),
                healthy: Some(true),
                checks: vec![
                    pass_check("manifest", "all file sizes and fingerprints matched"),
                    pass_check(
                        "validation-only",
                        "the full import and repository reconciliation passed without publication",
                    ),
                    pass_check(
                        "round-trip",
                        "the disposable repository snapshot matched the bundle exactly",
                    ),
                ],
            })
        })();
        remove_database_artifacts(&temporary_path);
        return result;
    }

    let database_path = absolute_output_path(&options.database_path)?;
    if database_path.exists() {
        return Err(MaintenanceError::TargetExists(display_path(&database_path)));
    }
    ensure_no_sidecars(&database_path)?;
    ensure_parent(&database_path)?;

    let _lock = MaintenanceLock::acquire(&database_path)?;
    let temporary_path = suffixed_path(&database_path, ".import.tmp");
    if temporary_path.exists() {
        return Err(MaintenanceError::TemporaryArtifactExists(display_path(
            &temporary_path,
        )));
    }
    ensure_parent(&temporary_path)?;

    let result = (|| {
        let schema_version = validate_import_candidate(
            &temporary_path,
            &snapshot,
            &manifest.bundle_fingerprint,
        )?;
        sync_file(&temporary_path)?;
        fs::rename(&temporary_path, &database_path)
            .map_err(|error| io_error("publishing imported database", &database_path, error))?;
        sync_parent(&database_path)?;

        Ok(SqliteMaintenanceReport {
            schema_version: MAINTENANCE_REPORT_SCHEMA_VERSION,
            operation: "sqlite-import".to_string(),
            status: "imported".to_string(),
            source_path: Some(display_path(&bundle_directory)),
            target_path: Some(display_path(&database_path)),
            previous_database_path: None,
            bundle_fingerprint: Some(manifest.bundle_fingerprint),
            database_schema_version: Some(schema_version),
            counts: Some(manifest.counts),
            healthy: Some(true),
            checks: vec![
                pass_check("manifest", "all file sizes and fingerprints matched"),
                pass_check(
                    "round-trip",
                    "the imported repository snapshot matched the bundle exactly",
                ),
            ],
        })
    })();

    if result.is_err() {
        remove_database_artifacts(&temporary_path);
    }
    result
}

fn validate_import_candidate(
    temporary_path: &Path,
    snapshot: &RepositorySnapshot,
    bundle_fingerprint: &str,
) -> Result<i64, MaintenanceError> {
    let mut repository = SqliteRepository::open(temporary_path)?;
    import_snapshot(&mut repository, snapshot, bundle_fingerprint)?;
    checkpoint_database(&repository.connection)?;
    let imported = repository.read_consistent_snapshot()?;
    if imported != *snapshot {
        return Err(MaintenanceError::SnapshotMismatch);
    }
    let schema_version = repository.schema_version()?;
    drop(repository);
    require_healthy_database(temporary_path)?;
    Ok(schema_version)
}

fn dry_run_import_path() -> Result<PathBuf, MaintenanceError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| MaintenanceError::InvalidData(error.to_string()))?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "strata-sqlite-import-validation-{}-{nonce}.sqlite3",
        process::id()
    )))
}

'''
replace_between(
    "src/sqlite/maintenance.rs",
    "pub(super) fn import_bundle(",
    "pub(super) fn doctor(",
    new_import,
)
replace_once(
    "src/sqlite/maintenance.rs",
    '        Some("sqlite-candidate" | "sqlite")\n',
    '        Some("sqlite-candidate" | "sqlite" | "sqlite-cli")\n',
)

# SQLite module boundary and closure tests.
replace_once(
    "src/sqlite.rs",
    "mod legacy_import;\nmod maintenance;",
    "mod legacy_disposition;\nmod legacy_import;\nmod maintenance;",
)
replace_once(
    "src/sqlite.rs",
    "#[cfg(test)]\nmod fault_certification;",
    "#[cfg(test)]\nmod closure_tests;\n#[cfg(test)]\nmod fault_certification;",
)
replace_once(
    "src/sqlite.rs",
    "pub(crate) use maintenance::{\n    BackupOptions, BundleExportOptions, BundleImportOptions, DoctorOptions, RestoreOptions,\n    SqliteMaintenanceReport,\n};",
    "pub(crate) use legacy_disposition::{\n    LegacyEvidenceArchiveOptions, LegacyEvidenceInventoryOptions, LegacyEvidenceRemoveOptions,\n    LegacyEvidenceReport,\n};\npub(crate) use maintenance::{\n    BackupOptions, BundleExportOptions, BundleImportOptions, DoctorOptions, RestoreOptions,\n    SqliteMaintenanceReport,\n};",
)
replace_once(
    "src/sqlite.rs",
    "pub(crate) fn run_bundle_export(\n",
    "pub(crate) fn run_legacy_evidence_inventory(\n    options: LegacyEvidenceInventoryOptions,\n) -> Result<LegacyEvidenceReport, String> {\n    legacy_disposition::inventory(options)\n}\n\npub(crate) fn run_legacy_evidence_archive(\n    options: LegacyEvidenceArchiveOptions,\n) -> Result<LegacyEvidenceReport, String> {\n    legacy_disposition::archive(options)\n}\n\npub(crate) fn run_legacy_evidence_remove(\n    options: LegacyEvidenceRemoveOptions,\n) -> Result<LegacyEvidenceReport, String> {\n    legacy_disposition::remove(options)\n}\n\npub(crate) fn run_bundle_export(\n",
)

# Test-only interruption seam stays entirely inside the disposition module.
replace_once(
    "src/sqlite/legacy_disposition.rs",
    "fn load_context(marker_path: &Path) -> Result<EvidenceContext, LegacyEvidenceError> {",
    "#[cfg(test)]\npub(super) fn remove_with_test_failure(\n    options: LegacyEvidenceRemoveOptions,\n    fail_after: usize,\n) -> Result<LegacyEvidenceReport, String> {\n    remove_with_hook(options, |deleted| {\n        if deleted == fail_after {\n            Err(LegacyEvidenceError::Io {\n                operation: \"injecting legacy-removal interruption\",\n                path: \"test\".to_string(),\n                message: \"injected interruption\".to_string(),\n            })\n        } else {\n            Ok(())\n        }\n    })\n    .map_err(|error| error.to_string())\n}\n\nfn load_context(marker_path: &Path) -> Result<EvidenceContext, LegacyEvidenceError> {",
)
replace_once(
    "src/sqlite/legacy_disposition.rs",
    '        healthy: files.iter().all(|file| file.live_status == "matches")\n            || status.contains("removed"),',
    '        healthy: files.iter().all(|file| file.live_status == "matches")\n            || status.contains("remov"),',
)

# CLI command surface.
replace_once(
    "src/cli.rs",
    "    SqliteImport {\n        #[arg(long, value_name = \"DIRECTORY\", help = \"Portable bundle directory\")]\n        bundle: PathBuf,\n\n        #[arg(long, value_name = \"PATH\", help = \"New SQLite database path\")]\n        database: Option<PathBuf>,",
    "    SqliteImport {\n        #[arg(long, value_name = \"DIRECTORY\", help = \"Portable bundle directory\")]\n        bundle: PathBuf,\n\n        #[arg(long, help = \"Validate the complete import without publishing a database\")]\n        dry_run: bool,\n\n        #[arg(long, value_name = \"PATH\", help = \"New SQLite database path\")]\n        database: Option<PathBuf>,",
)
legacy_variants = r'''    #[command(about = "Inventory verified legacy migration evidence")]
    SqliteLegacyInventory {
        #[arg(long, value_name = "PATH", help = "Storage authority marker path")]
        authority_marker: Option<PathBuf>,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Archive verified legacy migration evidence")]
    SqliteLegacyArchive {
        #[arg(long, value_name = "DIRECTORY", help = "New archive directory")]
        out: PathBuf,

        #[arg(long, value_name = "PATH", help = "Storage authority marker path")]
        authority_marker: Option<PathBuf>,

        #[arg(long, help = "Confirm archive publication")]
        confirm: bool,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

    #[command(about = "Remove legacy files after a verified archive exists")]
    SqliteLegacyRemove {
        #[arg(long, value_name = "DIRECTORY", help = "Verified legacy evidence archive")]
        archive: PathBuf,

        #[arg(long, value_name = "PATH", help = "Storage authority marker path")]
        authority_marker: Option<PathBuf>,

        #[arg(
            long,
            value_name = "FINGERPRINT",
            help = "Exact migration fingerprint confirming irreversible removal"
        )]
        confirm_fingerprint: String,

        #[arg(long, help = "Print the result as JSON")]
        json: bool,
    },

'''
replace_once(
    "src/cli.rs",
    "    #[command(about = \"Generate shell completions\")]",
    legacy_variants + "    #[command(about = \"Generate shell completions\")]",
)
replace_once(
    "src/cli.rs",
    "pub fn sqlite_import(bundle: PathBuf, database: Option<PathBuf>, json: bool) -> Result<(), String> {\n    let report = sqlite::run_bundle_import(sqlite::BundleImportOptions {\n        bundle_directory: bundle,\n        database_path: database.unwrap_or_else(default_sqlite_database_path),\n    })?;",
    "pub fn sqlite_import(\n    bundle: PathBuf,\n    database: Option<PathBuf>,\n    dry_run: bool,\n    json: bool,\n) -> Result<(), String> {\n    let report = sqlite::run_bundle_import(sqlite::BundleImportOptions {\n        bundle_directory: bundle,\n        database_path: database.unwrap_or_else(default_sqlite_database_path),\n        dry_run,\n    })?;",
)
legacy_functions = r'''
fn print_legacy_evidence_report(
    report: sqlite::LegacyEvidenceReport,
    json: bool,
) -> Result<(), String> {
    let healthy = report.is_healthy();
    if json {
        println!("{}", report.to_pretty_json()?);
    } else {
        report.print_human();
    }
    if healthy {
        Ok(())
    } else {
        Err("legacy evidence differs from the verified migration backup".to_string())
    }
}

pub fn sqlite_legacy_inventory(
    authority_marker: Option<PathBuf>,
    json: bool,
) -> Result<(), String> {
    let report = sqlite::run_legacy_evidence_inventory(sqlite::LegacyEvidenceInventoryOptions {
        authority_marker_path: authority_marker.unwrap_or_else(default_authority_marker_path),
    })?;
    print_legacy_evidence_report(report, json)
}

pub fn sqlite_legacy_archive(
    out: PathBuf,
    authority_marker: Option<PathBuf>,
    confirm: bool,
    json: bool,
) -> Result<(), String> {
    let report = sqlite::run_legacy_evidence_archive(sqlite::LegacyEvidenceArchiveOptions {
        authority_marker_path: authority_marker.unwrap_or_else(default_authority_marker_path),
        output_directory: out,
        confirm,
    })?;
    print_legacy_evidence_report(report, json)
}

pub fn sqlite_legacy_remove(
    archive: PathBuf,
    authority_marker: Option<PathBuf>,
    confirm_fingerprint: String,
    json: bool,
) -> Result<(), String> {
    let report = sqlite::run_legacy_evidence_remove(sqlite::LegacyEvidenceRemoveOptions {
        authority_marker_path: authority_marker.unwrap_or_else(default_authority_marker_path),
        archive_directory: archive,
        confirm_fingerprint,
    })?;
    print_legacy_evidence_report(report, json)
}

'''
replace_once(
    "src/cli.rs",
    "pub fn print_completions(shell: &str) -> Result<(), String> {",
    legacy_functions + "pub fn print_completions(shell: &str) -> Result<(), String> {",
)
replace_once(
    "src/cli.rs",
    "        Cli::SqliteImport {\n            bundle,\n            database,\n            json,\n        } => {\n            if let Err(error) = sqlite_import(bundle, database, json) {",
    "        Cli::SqliteImport {\n            bundle,\n            dry_run,\n            database,\n            json,\n        } => {\n            if let Err(error) = sqlite_import(bundle, database, dry_run, json) {",
)
legacy_dispatch = r'''        Cli::SqliteLegacyInventory {
            authority_marker,
            json,
        } => {
            if let Err(error) = sqlite_legacy_inventory(authority_marker, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteLegacyArchive {
            out,
            authority_marker,
            confirm,
            json,
        } => {
            if let Err(error) = sqlite_legacy_archive(out, authority_marker, confirm, json) {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }
        Cli::SqliteLegacyRemove {
            archive,
            authority_marker,
            confirm_fingerprint,
            json,
        } => {
            if let Err(error) =
                sqlite_legacy_remove(archive, authority_marker, confirm_fingerprint, json)
            {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
        }

'''
replace_once(
    "src/cli.rs",
    "        Cli::Completions { shell } => {",
    legacy_dispatch + "        Cli::Completions { shell } => {",
)

# Activation output no longer claims that the TUI is blocked.
replace_once(
    "src/sqlite/authority.rs",
    '        println!("TUI status: blocked until its SQLite cutover");',
    '        println!("TUI status: SQLite-backed");',
)
