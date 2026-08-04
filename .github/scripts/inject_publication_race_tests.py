from pathlib import Path

path = Path("src/storage.rs")
text = path.read_text()
if "mod publication_race_tests" in text:
    raise SystemExit("publication race tests already injected")

text += r'''

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

        for payload in payloads.iter().cloned() {
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
        assert!(failures.is_empty(), "concurrent writes failed: {failures:?}");

        let final_content = fs::read_to_string(&path).expect("published content should exist");
        assert!(payloads.contains(&final_content));
        assert!(!path.with_extension("tmp").exists());

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
        assert!(observed_same_second, "could not exercise same-second backup naming");

        fs::remove_file(&path).ok();
        fs::remove_dir_all(backup_dir).ok();
    }
}
'''
path.write_text(text)
