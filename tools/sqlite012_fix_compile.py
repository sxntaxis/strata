from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"anchor not found in {path}: {old[:100]!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/sqlite/legacy_disposition.rs",
    "    let result = (|| {\n        let mut manifest_files = Vec::with_capacity(context.files.len());",
    "    let result: Result<(), LegacyEvidenceError> = (|| {\n        let mut manifest_files = Vec::with_capacity(context.files.len());",
)

replace_once(
    "src/sqlite/maintenance.rs",
    "        import_bundle(BundleImportOptions {\n            bundle_directory: bundle_a.clone(),\n            database_path: imported.clone(),\n        })",
    "        import_bundle(BundleImportOptions {\n            bundle_directory: bundle_a.clone(),\n            database_path: imported.clone(),\n            dry_run: false,\n        })",
)

replace_once(
    "src/sqlite/maintenance.rs",
    "        let error = import_bundle(BundleImportOptions {\n            bundle_directory: bundle,\n            database_path: root.join(\"imported.sqlite3\"),\n        })",
    "        let error = import_bundle(BundleImportOptions {\n            bundle_directory: bundle,\n            database_path: root.join(\"imported.sqlite3\"),\n            dry_run: false,\n        })",
)
