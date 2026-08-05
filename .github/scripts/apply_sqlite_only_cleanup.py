from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


def sub_once(text: str, pattern: str, replacement: str, label: str) -> str:
    result, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return result


path = Path("src/cli.rs")
text = path.read_text()
text = replace_once(
    text,
    "use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, SecondsFormat, Utc};",
    "use chrono::{DateTime, NaiveDate, Utc};",
    "chrono imports",
)
text = replace_once(
    text,
    '''        CategoryId, DRIFT_CATEGORY_CONFIG_NAME, OperationalDayPolicy, ReportPeriod, Session,
        build_period_report, build_report_for_date_range, civil_time_for_utc, day_boundary_config,
        operational_day_key_for_utc, runtime_settings,
    },
    profile, sqlite, storage, temporal,''',
    '''        CategoryId, OperationalDayPolicy, ReportPeriod, Session, build_period_report,
        build_report_for_date_range, civil_time_for_utc, day_boundary_config,
        operational_day_key_for_utc,
    },
    profile, sqlite, storage,''',
    "domain imports",
)
path.write_text(text)

path = Path("src/sqlite.rs")
text = path.read_text()
text = replace_once(
    text,
    "pub(crate) use authority::{open_cli_repository, profile_database_path, resolve_runtime_database};",
    "pub(crate) use authority::resolve_runtime_database;",
    "authority reexport",
)
path.write_text(text)

path = Path("src/sqlite/authority.rs")
text = path.read_text()
text = replace_once(
    text,
    '''use std::{
    fs,
    path::{Path, PathBuf},
};
''',
    '''use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
use std::cell::Cell;
''',
    "test cell import",
)
text = replace_once(
    text,
    '''pub(crate) fn open_cli_repository(path: &Path) -> Result<SqliteRepository, String> {
    let repository = SqliteRepository::open(path).map_err(|error| error.to_string())?;
    validate_and_bind_profile(&repository)?;
    Ok(repository)
}
''',
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
    })?;
    validate_and_bind_profile(&repository)?;
    Ok(repository)
}
''',
    "disk-full test helper",
)
path.write_text(text)

path = Path("src/storage.rs")
text = path.read_text()
text = text.replace("OpenOptions, TryLockError", "OpenOptions")
text = sub_once(
    text,
    r"\npub fn get_active_session_path\(\) -> PathBuf \{.*?\n\}\n\npub fn get_detached_runtime_path",
    "\npub fn get_detached_runtime_path",
    "obsolete file runtime lock paths",
)
path.write_text(text)

# A fresh application database is final authority, never a migration candidate.
for rust_path in Path("src").rglob("*.rs"):
    source = rust_path.read_text()
    updated = source.replace("sqlite-candidate", "sqlite")
    updated = updated.replace('Some("sqlite" | "sqlite" | "sqlite-cli")', 'Some("sqlite" | "sqlite-cli")')
    if updated != source:
        rust_path.write_text(updated)

print("SQLite-only compile cleanup applied")
