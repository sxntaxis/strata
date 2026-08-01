from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"missing correction anchor in {path}: {old[:160]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_all(path: str, old: str, new: str, minimum: int = 1) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count < minimum:
        raise SystemExit(
            f"missing correction anchor in {path}: expected at least {minimum}, found {count}: {old[:160]!r}"
        )
    target.write_text(text.replace(old, new))


# Remove the no-longer-owned parser helper import.
replace_once(
    "src/domain.rs",
    "    DateTime, Datelike, Duration as ChronoDuration, FixedOffset, Local, NaiveDate, NaiveTime, Utc,",
    "    DateTime, Datelike, Duration as ChronoDuration, FixedOffset, Local, NaiveDate, Utc,",
)

# Domain recording accepts either the machine Local zone used by old tests or the
# configured FixedOffset used by authoritative runtime paths.
replace_once(
    "src/domain.rs",
    "    pub fn end_session_with_elapsed_at_local(\n        &mut self,\n        elapsed: usize,\n        end_local: DateTime<Local>,\n    ) -> Option<usize> {",
    "    pub fn end_session_with_elapsed_at_local<Tz>(\n        &mut self,\n        elapsed: usize,\n        end_local: DateTime<Tz>,\n    ) -> Option<usize>\n    where\n        Tz: chrono::TimeZone,\n        Tz::Offset: std::fmt::Display,\n    {",
)
replace_once(
    "src/domain.rs",
    "    pub fn record_session_at(\n        &mut self,\n        cat_id: CategoryId,\n        cat_description: &str,\n        elapsed: usize,\n        end_local: DateTime<Local>,\n    ) {",
    "    pub fn record_session_at<Tz>(\n        &mut self,\n        cat_id: CategoryId,\n        cat_description: &str,\n        elapsed: usize,\n        end_local: DateTime<Tz>,\n    )\n    where\n        Tz: chrono::TimeZone,\n        Tz::Offset: std::fmt::Display,\n    {",
)
replace_once(
    "src/domain.rs",
    "        let today = operational_day_key_for_local(&end_local)\n            .format(\"%Y-%m-%d\")\n            .to_string();",
    "        let today = operational_day_key_for_utc(end_local.with_timezone(&Utc))\n            .format(\"%Y-%m-%d\")\n            .to_string();",
)
replace_once(
    "src/domain.rs",
    "        let day = operational_day_key_for_local(&live.now_civil)\n            .format(\"%Y-%m-%d\")\n            .to_string();",
    "        let day = operational_day_key_for_utc(live.now_civil.with_timezone(&Utc))\n            .format(\"%Y-%m-%d\")\n            .to_string();",
)

# Every direct interactive category switch is a live monotonic transition.
replace_once(
    "src/app/category_state.rs",
    "self.switch_active_category_at(added_id, chrono::Utc::now());",
    "self.switch_active_category_at(\n                added_id,\n                chrono::Utc::now(),\n                super::SessionClockMode::LiveMonotonic,\n            );",
)
replace_once(
    "src/app/category_state.rs",
    "self.switch_active_category_at(DRIFT_CATEGORY_ID, chrono::Utc::now());",
    "self.switch_active_category_at(\n                    DRIFT_CATEGORY_ID,\n                    chrono::Utc::now(),\n                    super::SessionClockMode::LiveMonotonic,\n                );",
)

# Authority reload and checkpoint restoration are checked historical-wall paths.
replace_once(
    "src/app/persistence_recovery.rs",
    "                self.begin_active_session_at(active.started_at_utc);",
    "                self.begin_active_session_at(active.started_at_utc, false)?;",
)
replace_once(
    "src/app.rs",
    "        if let Some(started_at) = checkpoint.active_session_started_at_utc {\n            self.begin_active_session_at(started_at);\n        } else {",
    "        if let Some(started_at) = checkpoint.active_session_started_at_utc {\n            if let Err(error) = self.begin_active_session_at(started_at, false) {\n                self.record_storage_result::<()>(Err(error));\n                return false;\n            }\n        } else {",
)

# A queued mutation already carries UTC; allocating it through machine Local would
# reintroduce the authority split.
replace_once(
    "src/app.rs",
    "                let scheduled_local = scheduled_at_utc.with_timezone(&Local);\n                let scheduled_day = operational_day_key_for_local(&scheduled_local);",
    "                let scheduled_day = operational_day_key_for_utc(scheduled_at_utc);",
)

# Remaining report snapshot projections use the configured civil timestamp.
replace_all("src/app/report_state.rs", "live.now_local", "live.now_civil", minimum=2)
