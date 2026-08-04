from pathlib import Path

path = Path("src/sqlite.rs")
content = path.read_text()
old = "assert_eq!(repository.schema_version().unwrap(), 6);"
count = content.count(old)
if count != 3:
    raise SystemExit(f"expected 3 schema-six repository assertions, found {count}")
content = content.replace(old, "assert_eq!(repository.schema_version().unwrap(), 7);")
content = content.replace(
    "schema 6 must accept typed daily contributions",
    "schema 7 must retain typed daily contributions",
)
path.write_text(content)
