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
