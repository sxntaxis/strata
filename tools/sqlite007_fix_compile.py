from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing {label} anchor")
    return text.replace(old, new, 1)


path = Path("src/sqlite/tui_runtime.rs")
text = path.read_text()
text = text.replace("    path::{Path, PathBuf},\n", "    path::Path,\n", 1)
text = text.replace(
    "    NewActiveSession, SessionCompletion, SqliteRepository,\n",
    "    NewActiveSession, SessionCompletion,\n",
    1,
)
text = text.replace(
    "        CheckpointRecord, CheckpointStatus, NewSandSnapshotRecord, SandStateRecord, SnapshotKind,\n",
    "        CheckpointRecord, CheckpointStatus, SandStateRecord,\n",
    1,
)
text = replace_once(
    text,
    "            Ok(SqliteTuiActiveSession {",
    "            Ok::<SqliteTuiActiveSession, String>(SqliteTuiActiveSession {",
    "active-session result type",
)
text = replace_once(
    text,
    '''            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))''',
    '''            Ok((
                row.get::<_, i64>(0)?,
                (
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ),
            ))''',
    "stored session map entry",
)
text = replace_once(
    text,
    '''pub(crate) fn delete_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;''',
    '''pub(crate) fn delete_daily_snapshot(
    database_path: &Path,
    operational_day: &str,
) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;''',
    "daily snapshot delete mutability",
)
text = replace_once(
    text,
    '''pub(crate) fn clear_checkpoint(database_path: &Path) -> Result<(), String> {
    let mut repository = open_cli_repository(database_path)?;''',
    '''pub(crate) fn clear_checkpoint(database_path: &Path) -> Result<(), String> {
    let repository = open_cli_repository(database_path)?;''',
    "checkpoint clear mutability",
)
text = replace_once(
    text,
    "mod tests {\n    use super::*;\n",
    "mod tests {\n    use std::path::PathBuf;\n\n    use super::*;\n",
    "test path import",
)
path.write_text(text)

path = Path("src/sqlite.rs")
text = path.read_text()
text = text.replace(
    "    RuntimeAuthority, SqliteCliActivationOptions, activate_sqlite_cli, ensure_tui_legacy_allowed,\n",
    "    RuntimeAuthority, SqliteCliActivationOptions, activate_sqlite_cli,\n",
    1,
)
text = text.replace(
    "    SqliteTuiActiveSession, SqliteTuiState, clear_checkpoint as clear_tui_checkpoint,\n",
    "    clear_checkpoint as clear_tui_checkpoint,\n",
    1,
)
path.write_text(text)

path = Path("src/app/report_state.rs")
text = path.read_text()
text = replace_once(
    text,
    "Some(preview_engine.render(categories))",
    "Some(preview_engine.render(&categories))",
    "report snapshot render borrow",
)
path.write_text(text)
