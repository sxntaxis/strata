from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"missing cleanup anchor in {path}: {old!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/app.rs",
    "use chrono::{DateTime, Duration as ChronoDuration, Local, SecondsFormat, Utc};",
    "use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};",
)
replace_once(
    "src/domain.rs",
    "        let start_time = end_local - ChronoDuration::seconds(elapsed as i64);\n        let today = operational_day_key_for_utc(end_local.with_timezone(&Utc))",
    "        let end_utc = end_local.with_timezone(&Utc);\n        let start_time = end_local.clone() - ChronoDuration::seconds(elapsed as i64);\n        let today = operational_day_key_for_utc(end_utc)",
)
replace_once(
    "src/domain.rs",
    "pub fn operational_day_key_for_local(local: &DateTime<Local>) -> NaiveDate {\n    operational_day_key_from_utc(local.with_timezone(&Utc), &day_boundary_config())\n}\n\n",
    "",
)
replace_once(
    "src/domain.rs",
    "    DateTime, Datelike, Duration as ChronoDuration, FixedOffset, Local, NaiveDate, Utc,\n};\nuse ratatui::style::Color;",
    "    DateTime, Datelike, Duration as ChronoDuration, FixedOffset, NaiveDate, Utc,\n};\n#[cfg(test)]\nuse chrono::Local;\nuse ratatui::style::Color;",
)
