from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one match, found {count}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/sqlite/authority.rs",
    '''use std::{
    fs,
    path::{Path, PathBuf},
};''',
    '''use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
use std::cell::Cell;''',
)

replace_once(
    "src/sqlite/authority.rs",
    '''pub(crate) fn open_cli_repository(path: &Path) -> Result<SqliteRepository, String> {
    let repository = SqliteRepository::open(path).map_err(|error| error.to_string())?;''',
    '''#[cfg(test)]
thread_local! {
    static TEST_PAGE_LIMIT_ENABLED: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn with_test_page_limit<T>(action: impl FnOnce() -> T) -> T {
    struct Reset(bool);

    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_PAGE_LIMIT_ENABLED.with(|enabled| enabled.set(self.0));
        }
    }

    let previous = TEST_PAGE_LIMIT_ENABLED.with(|enabled| enabled.replace(true));
    let _reset = Reset(previous);
    action()
}

pub(crate) fn open_cli_repository(path: &Path) -> Result<SqliteRepository, String> {
    let repository = SqliteRepository::open(path).map_err(|error| error.to_string())?;
    #[cfg(test)]
    TEST_PAGE_LIMIT_ENABLED.with(|enabled| -> Result<(), String> {
        if !enabled.get() {
            return Ok(());
        }
        repository
            .connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .map_err(|error| error.to_string())?;
        let page_count: i64 = repository
            .connection
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        repository
            .connection
            .pragma_update(None, "max_page_count", page_count)
            .map_err(|error| error.to_string())?;
        Ok(())
    })?;''',
)

replace_once(
    "src/sqlite/fault_certification.rs",
    '''use super::{
    NewActiveSession, SqliteRepository,
    repository::{CheckpointStatus, NewCategoryRecord, SandStateRecord},
    runtime_coordination,
    tui_runtime,
};''',
    '''use super::{
    NewActiveSession, SqliteRepository, authority,
    repository::{CheckpointStatus, NewCategoryRecord, SandStateRecord},
    runtime_coordination,
    tui_runtime,
};''',
)

replace_once(
    "src/sqlite/fault_certification.rs",
    '''    with_database("real-full", |path| {
        let repository = SqliteRepository::open(path).unwrap();
        repository
            .connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        let page_count: i64 = repository
            .connection
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap();
        repository
            .connection
            .pragma_update(None, "max_page_count", page_count)
            .unwrap();
        drop(repository);
        let categories = vec![
            category(0, "None", "", 0, 0),
            category(1, "Work", &"x".repeat(2 * 1024 * 1024), 0, 1),
            category(2, "Rest", "original-rest", 1, -1),
        ];
        let error = tui_runtime::sync_categories(path, &categories, CategoryId::new(0), None)
            .unwrap_err();''',
    '''    with_database("real-full", |path| {
        let categories = vec![
            category(0, "None", "", 0, 0),
            category(1, "Work", &"x".repeat(2 * 1024 * 1024), 0, 1),
            category(2, "Rest", "original-rest", 1, -1),
        ];
        let error = authority::with_test_page_limit(|| {
            tui_runtime::sync_categories(path, &categories, CategoryId::new(0), None)
        })
        .unwrap_err();''',
)
