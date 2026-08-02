from pathlib import Path

path = Path("tools/reconciliation001a-apply.py")
text = path.read_text()
replacements = [
    (
        "imports_full_legacy_fixture_and_verifies_totals",
        "strict_import_preserves_sources_and_verifies_every_state_family",
    ),
    (
        'LegacyFixture::new("archived_category_catalog")',
        'Fixture::new("archived-category-catalog")',
    ),
    ("fixture.options()", "options()"),
    (
        "SqliteRepository::open(&fixture.database_path).unwrap()",
        "SqliteRepository::open_in_memory().unwrap()",
    ),
]
for old, new in replacements:
    if old not in text:
        raise SystemExit(f"importer fixture substitution not found: {old}")
    text = text.replace(old, new)
path.write_text(text)
Path(__file__).unlink(missing_ok=True)
