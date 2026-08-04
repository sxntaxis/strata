from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one marker, found {count}")
    return text.replace(old, new, 1)


path = Path("src/storage.rs")
text = path.read_text()
text = replace_once(
    text,
    "    fs::{self, File, OpenOptions},",
    "    fs::{self, File, OpenOptions, TryLockError},",
    "storage lock import",
)
text = replace_once(
    text,
    '''pub fn get_active_session_path() -> PathBuf {
    get_state_dir().join("active_session.json")
}
''',
    '''pub fn get_active_session_path() -> PathBuf {
    get_state_dir().join("active_session.json")
}

pub fn get_legacy_lifecycle_lock_path() -> PathBuf {
    get_state_dir().join("legacy_lifecycle.lock")
}

pub struct LegacyLifecycleLock {
    _file: File,
}

pub fn try_acquire_legacy_lifecycle_lock() -> Result<LegacyLifecycleLock, String> {
    let path = get_legacy_lifecycle_lock_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "cannot create legacy lifecycle lock directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .map_err(|error| {
            format!(
                "cannot open legacy lifecycle lock {}: {error}",
                path.display()
            )
        })?;
    match file.try_lock() {
        Ok(()) => Ok(LegacyLifecycleLock { _file: file }),
        Err(TryLockError::WouldBlock) => Err(
            "another Strata process is already performing a legacy lifecycle transition"
                .to_string(),
        ),
        Err(TryLockError::Error(error)) => Err(format!(
            "cannot acquire legacy lifecycle lock {}: {error}",
            path.display()
        )),
    }
}
''',
    "legacy lifecycle lock",
)
path.write_text(text)

path = Path("src/cli.rs")
text = path.read_text()
text = replace_once(
    text,
    '''fn start_session_legacy(
    project: String,
    description: Option<String>,
    category_name: String,
) -> Result<(), String> {
    let categories_path = storage::get_categories_path();''',
    '''fn start_session_legacy(
    project: String,
    description: Option<String>,
    category_name: String,
) -> Result<(), String> {
    let _lifecycle_lock = storage::try_acquire_legacy_lifecycle_lock()?;
    let categories_path = storage::get_categories_path();''',
    "legacy start lock",
)
text = replace_once(
    text,
    '''fn stop_session_legacy(accept_clock_jump: bool) -> Result<usize, String> {
    let session_path = storage::get_active_session_path();''',
    '''fn stop_session_legacy(accept_clock_jump: bool) -> Result<usize, String> {
    let _lifecycle_lock = storage::try_acquire_legacy_lifecycle_lock()?;
    let session_path = storage::get_active_session_path();''',
    "legacy stop lock",
)
path.write_text(text)

print("legacy CLI lifecycle lock applied")
