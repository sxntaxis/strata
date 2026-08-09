use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::domain::{Category, Session};

#[derive(Debug)]
pub struct LoadedCategories {
    pub categories: Vec<Category>,
    pub next_category_id: u64,
}

#[derive(Debug)]
pub struct LoadedSessions {
    pub sessions: Vec<Session>,
    pub next_session_id: usize,
}

const BACKUP_RETENTION_MAX_FILES: usize = 10;
const PUBLICATION_NAME_ATTEMPTS: usize = 1024;
static PUBLICATION_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryTagsState {
    pub version: u8,
    pub tags_by_category: HashMap<u64, Vec<String>>,
}

impl CategoryTagsState {
    pub const VERSION: u8 = 1;
}

impl Default for CategoryTagsState {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            tags_by_category: HashMap::new(),
        }
    }
}

pub fn get_data_dir() -> PathBuf {
    crate::profile::data_dir()
}

pub fn get_config_dir() -> PathBuf {
    crate::profile::config_dir()
}

pub fn get_state_dir() -> PathBuf {
    crate::profile::state_dir()
}

pub fn get_keymap_path() -> PathBuf {
    get_config_dir().join("keymap.json")
}

pub fn write_text_file(path: &Path, content: &str) -> Result<(), String> {
    atomic_write(path, content)
}

fn next_publication_nonce() -> u64 {
    PUBLICATION_NONCE.fetch_add(1, Ordering::Relaxed)
}

fn create_exclusive_sibling(
    directory: &Path,
    name_for_nonce: impl Fn(u64) -> String,
) -> Result<(PathBuf, File), String> {
    for _ in 0..PUBLICATION_NAME_ATTEMPTS {
        let nonce = next_publication_nonce();
        let candidate = directory.join(name_for_nonce(nonce));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err(format!(
        "could not allocate a unique publication file in {}",
        directory.display()
    ))
}

pub fn create_backup(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let backup_dir = path.parent().unwrap_or(Path::new(".")).join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S%.9f").to_string();
    let source_name = path.file_name().unwrap_or_default().to_string_lossy();
    let process_id = std::process::id();
    let (backup_path, mut backup_file) = create_exclusive_sibling(&backup_dir, |nonce| {
        format!("{source_name}.{timestamp}.{process_id}-{nonce}")
    })?;
    let copy_result = (|| -> Result<(), String> {
        let mut source = File::open(path).map_err(|error| error.to_string())?;
        std::io::copy(&mut source, &mut backup_file).map_err(|error| error.to_string())?;
        backup_file.sync_all().map_err(|error| error.to_string())
    })();
    if let Err(error) = copy_result {
        let _ = fs::remove_file(&backup_path);
        return Err(error);
    }

    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    if let Ok(entries) = fs::read_dir(&backup_dir) {
        let mut backups: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(&*stem))
            .collect();
        backups.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

        while backups.len() > BACKUP_RETENTION_MAX_FILES {
            if let Some(oldest) = backups.first() {
                let _ = fs::remove_file(oldest.path());
                backups.remove(0);
            }
        }
    }

    Ok(())
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if path.exists() {
        create_backup(path)?;
    }

    let parent = path.parent().unwrap_or(Path::new("."));
    let target_name = path.file_name().unwrap_or_default().to_string_lossy();
    let process_id = std::process::id();
    let (tmp_path, mut tmp_file) = create_exclusive_sibling(parent, |nonce| {
        format!(".{target_name}.tmp-{process_id}-{nonce}")
    })?;
    let publication = (|| -> Result<(), String> {
        tmp_file
            .write_all(content.as_bytes())
            .map_err(|error| error.to_string())?;
        tmp_file.sync_all().map_err(|error| error.to_string())?;
        drop(tmp_file);
        fs::rename(&tmp_path, path).map_err(|error| error.to_string())
    })();
    if let Err(error) = publication {
        let _ = fs::remove_file(&tmp_path);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod publication_race_tests {
    use super::*;
    use std::{
        collections::HashSet,
        sync::{Arc, Barrier},
        thread,
    };

    fn race_path(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "strata-publication-race-{label}-{}-{nonce}.json",
            std::process::id()
        ))
    }

    #[test]
    fn concurrent_atomic_writers_use_independent_temporary_files() {
        let path = race_path("atomic-write");
        let writers = 24;
        let barrier = Arc::new(Barrier::new(writers));
        let payloads = (0..writers)
            .map(|index| format!("writer-{index}:{}", "x".repeat(256 * 1024)))
            .collect::<Vec<_>>();
        let mut handles = Vec::with_capacity(writers);

        for payload in payloads.clone() {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                atomic_write(&path, &payload)
            }));
        }

        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("writer thread should finish"))
            .collect::<Vec<_>>();
        let failures = results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            failures.is_empty(),
            "concurrent writes failed: {failures:?}"
        );

        let final_content = fs::read_to_string(&path).expect("published content should exist");
        assert!(payloads.contains(&final_content));
        let target_name = path.file_name().unwrap().to_string_lossy();
        let stale_temps = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&format!(".{target_name}.tmp-"))
            })
            .count();
        assert_eq!(stale_temps, 0);

        fs::remove_file(&path).ok();
        fs::remove_dir_all(path.parent().unwrap().join("backups")).ok();
    }

    #[test]
    fn backups_created_within_one_second_have_distinct_names() {
        let path = race_path("backup-name");
        fs::write(&path, "authority").unwrap();
        let backup_dir = path.parent().unwrap().join("backups");
        let prefix = format!("{}.", path.file_name().unwrap().to_string_lossy());

        let mut observed_same_second = false;
        for _ in 0..20 {
            fs::remove_dir_all(&backup_dir).ok();
            let before = chrono::Local::now().timestamp();
            create_backup(&path).unwrap();
            create_backup(&path).unwrap();
            let after = chrono::Local::now().timestamp();
            if before == after {
                observed_same_second = true;
                let names = fs::read_dir(&backup_dir)
                    .unwrap()
                    .filter_map(Result::ok)
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .filter(|name| name.starts_with(&prefix))
                    .collect::<HashSet<_>>();
                assert_eq!(
                    names.len(),
                    2,
                    "two successful backups in one second must not overwrite each other"
                );
                break;
            }
        }
        assert!(
            observed_same_second,
            "could not exercise same-second backup naming"
        );

        fs::remove_file(&path).ok();
        fs::remove_dir_all(backup_dir).ok();
    }
}
