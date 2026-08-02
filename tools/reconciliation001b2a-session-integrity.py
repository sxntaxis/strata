from pathlib import Path

path = Path("src/storage.rs")
text = path.read_text()
text = text.replace(
    "    let mut loaded = default_sessions_loaded();\n\n    for (index, record) in reader.records().enumerate() {",
    "    let mut loaded = default_sessions_loaded();\n    let mut seen_ids = HashSet::new();\n\n    for (index, record) in reader.records().enumerate() {",
    1,
)
old_id = '''        let Some(id_raw) = record.get(0) else {
            continue;
        };
        let id: usize = match id_raw.parse() {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Warning: Invalid session ID '{}', skipping", id_raw);
                continue;
            }
        };
'''
new_id = '''        let id_raw = record.get(0).unwrap_or_default();
        let id = id_raw
            .parse::<usize>()
            .map_err(|error| StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: format!("invalid session ID '{id_raw}': {error}"),
            })?;
        if !seen_ids.insert(id) {
            return Err(StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: format!("duplicate session ID {id}"),
            });
        }
'''
if text.count(old_id) != 1:
    raise SystemExit("legacy session ID parsing block not found")
text = text.replace(old_id, new_id, 1)
insert = '''        loaded.sessions.push(Session {
'''
elapsed = '''        let elapsed_raw = record.get(elapsed_index).unwrap_or_default();
        let elapsed_seconds =
            elapsed_raw
                .parse::<usize>()
                .map_err(|error| StorageError::InvalidCsvData {
                    file: "time_log.csv",
                    row,
                    message: format!("invalid elapsed seconds '{elapsed_raw}': {error}"),
                })?;

        loaded.sessions.push(Session {
'''
if text.count(insert) < 1:
    raise SystemExit("session append anchor not found")
text = text.replace(insert, elapsed, 1)
old_elapsed = '''            elapsed_seconds: record
                .get(elapsed_index)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0),
'''
if text.count(old_elapsed) != 1:
    raise SystemExit("lenient elapsed parsing block not found")
text = text.replace(old_elapsed, "            elapsed_seconds,\n", 1)

marker = '''    #[test]
    fn unknown_or_malformed_session_category_fails_closed_but_idle_remains_valid() {
'''
proof = r'''    #[test]
    fn malformed_duplicate_session_ids_and_elapsed_fail_closed() {
        let categories = default_categories_loaded().categories;

        let malformed_id_path = unique_path("strata_sessions_malformed_id", "csv");
        fs::write(
            &malformed_id_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\nnot-an-id,2026-08-01,0,idle,break,10:00:00,11:00:00,3600\n",
        )
        .unwrap();
        let malformed_id =
            try_load_sessions_from_csv(&malformed_id_path, &categories).unwrap_err();
        assert!(malformed_id.to_string().contains("invalid session ID 'not-an-id'"));

        let duplicate_path = unique_path("strata_sessions_duplicate_id", "csv");
        fs::write(
            &duplicate_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,0,idle,first,10:00:00,11:00:00,3600\n1,2026-08-01,0,idle,second,11:00:00,12:00:00,3600\n",
        )
        .unwrap();
        let duplicate = try_load_sessions_from_csv(&duplicate_path, &categories).unwrap_err();
        assert!(duplicate.to_string().contains("duplicate session ID 1"));

        let elapsed_path = unique_path("strata_sessions_malformed_elapsed", "csv");
        fs::write(
            &elapsed_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\n1,2026-08-01,0,idle,break,10:00:00,11:00:00,not-seconds\n",
        )
        .unwrap();
        let elapsed = try_load_sessions_from_csv(&elapsed_path, &categories).unwrap_err();
        assert!(elapsed
            .to_string()
            .contains("invalid elapsed seconds 'not-seconds'"));

        fs::remove_file(malformed_id_path).ok();
        fs::remove_file(duplicate_path).ok();
        fs::remove_file(elapsed_path).ok();
    }

    #[test]
    fn unknown_or_malformed_session_category_fails_closed_but_idle_remains_valid() {
'''
if marker not in text:
    raise SystemExit("session integrity proof insertion marker not found")
path.write_text(text.replace(marker, proof, 1))

for temporary in [
    ".github/workflows/reconciliation001b2a-session-integrity.yml",
    "tools/reconciliation001b2a-session-integrity.py",
]:
    Path(temporary).unlink(missing_ok=True)
