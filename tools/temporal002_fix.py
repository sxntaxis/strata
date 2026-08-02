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
