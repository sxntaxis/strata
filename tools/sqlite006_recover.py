from pathlib import Path
import subprocess

workflow_path = ".github/workflows/sqlite006-assemble.yml"
history = subprocess.check_output(
    ["git", "rev-list", "--reverse", "HEAD", "--", workflow_path],
    text=True,
).splitlines()
if not history:
    raise SystemExit("original SQLITE-006 workflow commit not found")

original = subprocess.check_output(
    ["git", "show", f"{history[0]}:{workflow_path}"],
    text=True,
)
lines = original.splitlines()
start = next(index for index, line in enumerate(lines) if "python3 - <<'PY'" in line) + 1
end = next(index for index in range(start, len(lines)) if lines[index].strip() == "PY")
wire_script = "\n".join(
    line[10:] if line.startswith("          ") else line
    for line in lines[start:end]
) + "\n"
wire_script = wire_script.replace(
    "RuntimeAuthority, SqliteCliActivationOptions, SqliteCliActivationReport,\n",
    "RuntimeAuthority, SqliteCliActivationOptions,\n",
    1,
)
wire_script = wire_script.replace(
    "    SqliteCliSnapshot, SqliteCliStartResult, SqliteCliStopResult,\n",
    "",
    1,
)

block_start = wire_script.index("integrity_anchor = " + chr(39) * 3)
block_end = wire_script.index("sqlite.write_text(text)", block_start)
replacement = 'metadata_methods = r\'\'\'    pub fn metadata_value(&self, key: &str) -> Result<Option<String>, SqliteStoreError> {\n        Ok(self\n            .connection\n            .query_row(\n                "SELECT value FROM database_metadata WHERE key = ?1",\n                params![key],\n                |row| row.get(0),\n            )\n            .optional()?)\n    }\n\n    pub fn transition_storage_authority(\n        &mut self,\n        expected: &str,\n        next: &str,\n        activated_at_utc: &str,\n    ) -> Result<(), SqliteStoreError> {\n        let transaction = self\n            .connection\n            .transaction_with_behavior(TransactionBehavior::Immediate)?;\n        let current: Option<String> = transaction\n            .query_row(\n                "SELECT value FROM database_metadata WHERE key = \'storage_authority\'",\n                [],\n                |row| row.get(0),\n            )\n            .optional()?;\n        let found = current.unwrap_or_else(|| "missing".to_string());\n        if found != expected {\n            return Err(SqliteStoreError::AuthorityConflict {\n                expected: expected.to_string(),\n                found,\n            });\n        }\n        transaction.execute(\n            "UPDATE database_metadata SET value = ?1 WHERE key = \'storage_authority\'",\n            params![next],\n        )?;\n        transaction.execute(\n            "INSERT INTO database_metadata(key, value)\n             VALUES (\'sqlite_cli_activated_at_utc\', ?1)\n             ON CONFLICT(key) DO UPDATE SET value = excluded.value",\n            params![activated_at_utc],\n        )?;\n        transaction.commit()?;\n        Ok(())\n    }\n\n\'\'\'\nif \'pub fn metadata_value(\' not in text:\n    text = replace_once(\n        text,\n        "    pub fn start_session(&mut self, active: &NewActiveSession<\'_>)",\n        metadata_methods + "    pub fn start_session(&mut self, active: &NewActiveSession<\'_>)",\n        \'metadata methods\',\n    )\n'
wire_script = wire_script[:block_start] + replacement + wire_script[block_end:]
compile(wire_script, "/tmp/sqlite006-wire.py", "exec")
Path("/tmp/sqlite006-wire.py").write_text(wire_script)
