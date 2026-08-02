from pathlib import Path

report = Path("src/app/report_state.rs")
text = report.read_text()
text = text.replace(
    "use chrono::{Duration as ChronoDuration, NaiveDate, Utc};",
    "use chrono::{Duration as ChronoDuration, NaiveDate};",
)
report.write_text(text)

cli = Path("src/cli.rs")
text = cli.read_text()
text = text.replace(
    "        CategoryId, DRIFT_CATEGORY_CONFIG_NAME, DayBoundaryMode, OperationalDayPolicy,\n",
    "        CategoryId, DRIFT_CATEGORY_CONFIG_NAME, OperationalDayPolicy,\n",
)
cli.write_text(text)

storage = Path("src/storage.rs")
text = storage.read_text()
old = """        let (started_at_utc, ended_at_utc, operational_day_policy) = if has_temporal_provenance {
            let parse_timestamp = |field: usize, label: &str| -> Result<DateTime<Utc>, StorageError> {
"""
new = """        let provenance_is_empty = (8..=11).all(|field| {
            record
                .get(field)
                .unwrap_or_default()
                .trim()
                .is_empty()
        });
        let (started_at_utc, ended_at_utc, operational_day_policy) = if has_temporal_provenance
            && provenance_is_empty
        {
            (None, None, None)
        } else if has_temporal_provenance {
            let parse_timestamp = |field: usize, label: &str| -> Result<DateTime<Utc>, StorageError> {
"""
if old not in text:
    raise SystemExit("storage provenance anchor not found")
text = text.replace(old, new, 1)
storage.write_text(text)

repository = Path("src/sqlite/repository.rs")
text = repository.read_text()
old = """        repository
            .insert_session(&session("session-2", category_id, "2026-08-02"))
            .unwrap();
"""
new = """        let mut second = session("session-2", category_id, "2026-08-02");
        second.started_at_utc = "2026-08-02T16:00:00Z";
        second.ended_at_utc = "2026-08-02T17:00:00Z";
        repository.insert_session(&second).unwrap();
"""
if old not in text:
    raise SystemExit("repository fixture anchor not found")
text = text.replace(old, new, 1)
repository.write_text(text)

lifecycle = Path("tests/cli_lifecycle.rs")
text = lifecycle.read_text()
text = text.replace(
    "use std::{\n",
    "use chrono::{Duration as ChronoDuration, Utc};\nuse std::{\n",
    1,
)
old = """    fn time_log_path(&self) -> PathBuf {
        self.data_dir().join("time_log.csv")
    }
}
"""
new = """    fn time_log_path(&self) -> PathBuf {
        self.data_dir().join("time_log.csv")
    }

    fn backdate_active_session(&self, seconds: i64) {
        let path = self.active_session_path();
        let content = fs::read_to_string(&path).expect("active session should be readable");
        let mut value: serde_json::Value =
            serde_json::from_str(&content).expect("active session JSON should be valid");
        value["start_time"] = serde_json::Value::String(
            (Utc::now() - ChronoDuration::seconds(seconds)).to_rfc3339(),
        );
        fs::write(
            path,
            serde_json::to_string_pretty(&value).expect("active session JSON should serialize"),
        )
        .expect("active session should be backdated");
    }
}
"""
if old not in text:
    raise SystemExit("lifecycle helper anchor not found")
text = text.replace(old, new, 1)
for anchor in [
    "    assert_exists(&profile.active_session_path());\n\n    let stop = profile.run(&[\"stop\"]);",
    "    assert!(start.status.success(), \"start failed: {}\", stderr(&start));\n\n    let first_stop = profile.run(&[\"stop\"]);",
    "    assert!(start.status.success(), \"start failed: {}\", stderr(&start));\n\n    fs::write(profile.time_log_path(), EMPTY_TIME_LOG).expect(\"time log should be initialized\");",
]:
    if anchor not in text:
        raise SystemExit(f"lifecycle test anchor not found: {anchor[:40]}")
replacements = {
    "    assert_exists(&profile.active_session_path());\n\n    let stop = profile.run(&[\"stop\"]);":
        "    assert_exists(&profile.active_session_path());\n    profile.backdate_active_session(2);\n\n    let stop = profile.run(&[\"stop\"]);",
    "    assert!(start.status.success(), \"start failed: {}\", stderr(&start));\n\n    let first_stop = profile.run(&[\"stop\"]);":
        "    assert!(start.status.success(), \"start failed: {}\", stderr(&start));\n    profile.backdate_active_session(2);\n\n    let first_stop = profile.run(&[\"stop\"]);",
    "    assert!(start.status.success(), \"start failed: {}\", stderr(&start));\n\n    fs::write(profile.time_log_path(), EMPTY_TIME_LOG).expect(\"time log should be initialized\");":
        "    assert!(start.status.success(), \"start failed: {}\", stderr(&start));\n    profile.backdate_active_session(2);\n\n    fs::write(profile.time_log_path(), EMPTY_TIME_LOG).expect(\"time log should be initialized\");",
}
for old, new in replacements.items():
    text = text.replace(old, new, 1)
lifecycle.write_text(text)
