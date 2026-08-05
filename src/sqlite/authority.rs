use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
use std::cell::Cell;

use rusqlite::{OptionalExtension, params};

use crate::{profile, storage};

use super::SqliteRepository;

const STORAGE_AUTHORITY: &str = "sqlite";
const PROFILE_ID_KEY: &str = "profile_id";

pub(crate) fn profile_database_path() -> PathBuf {
    storage::get_data_dir().join("strata.sqlite3")
}

pub(crate) fn resolve_runtime_database() -> Result<PathBuf, String> {
    let path = profile_database_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "cannot create SQLite profile directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let repository = SqliteRepository::open(&path).map_err(|error| error.to_string())?;
    validate_and_bind_profile(&repository)?;
    Ok(path)
}

#[cfg(test)]
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
    })?;
    validate_and_bind_profile(&repository)?;
    Ok(repository)
}

fn validate_and_bind_profile(repository: &SqliteRepository) -> Result<(), String> {
    let authority = repository
        .metadata_value("storage_authority")
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| "missing".to_string());
    if authority != STORAGE_AUTHORITY {
        return Err(format!(
            "database storage authority is {authority}, expected {STORAGE_AUTHORITY}; remove or rebuild this development database"
        ));
    }

    let expected_profile = profile::profile_id();
    let existing: Option<String> = repository
        .connection
        .query_row(
            "SELECT value FROM database_metadata WHERE key = ?1",
            params![PROFILE_ID_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match existing {
        Some(found) if found != expected_profile => Err(format!(
            "SQLite database belongs to profile {found}, current profile is {expected_profile}"
        )),
        Some(_) => Ok(()),
        None => {
            repository
                .connection
                .execute(
                    "INSERT INTO database_metadata(key, value) VALUES (?1, ?2)",
                    params![PROFILE_ID_KEY, expected_profile],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        }
    }
}
