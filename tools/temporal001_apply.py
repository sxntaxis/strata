from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"missing patch anchor in {path}: {old[:160]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_all(path: str, old: str, new: str, minimum: int = 1) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count < minimum:
        raise SystemExit(
            f"missing patch anchor in {path}: expected at least {minimum}, found {count}: {old[:160]!r}"
        )
    target.write_text(text.replace(old, new))


Path("src/temporal.rs").write_text(r'''use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, FixedOffset, NaiveDate, NaiveTime, Utc};

use crate::domain::{DayBoundaryConfig, DayBoundaryMode};

pub(crate) const MAX_LIVE_CLOCK_SKEW: Duration = Duration::from_secs(5 * 60);
pub(crate) const MAX_UNATTENDED_WALL_INTERVAL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReconciledInterval {
    pub ended_at_utc: DateTime<Utc>,
    pub elapsed_seconds: usize,
}

pub(crate) fn configured_offset(config: &DayBoundaryConfig) -> Result<FixedOffset, String> {
    FixedOffset::east_opt(config.utc_offset_seconds).ok_or_else(|| {
        format!(
            "configured UTC offset {} is outside the supported range",
            config.utc_offset_seconds
        )
    })
}

pub(crate) fn civil_from_utc(
    timestamp: DateTime<Utc>,
    config: &DayBoundaryConfig,
) -> Result<DateTime<FixedOffset>, String> {
    Ok(timestamp.with_timezone(&configured_offset(config)?))
}

pub(crate) fn operational_day_from_utc(
    timestamp: DateTime<Utc>,
    config: &DayBoundaryConfig,
) -> Result<NaiveDate, String> {
    let civil = civil_from_utc(timestamp, config)?;
    let cutoff = match config.mode {
        DayBoundaryMode::FixedHour | DayBoundaryMode::Sunrise => {
            NaiveTime::from_hms_opt(config.fixed_hour, config.fixed_minute, 0).ok_or_else(|| {
                format!(
                    "configured operational-day cutoff {:02}:{:02} is invalid",
                    config.fixed_hour, config.fixed_minute
                )
            })?
        }
    };

    let mut day = civil.date_naive();
    if civil.time() < cutoff {
        day -= ChronoDuration::days(1);
    }
    Ok(day)
}

pub(crate) fn checked_wall_interval(
    started_at_utc: DateTime<Utc>,
    ended_at_utc: DateTime<Utc>,
    accept_large_interval: bool,
) -> Result<ReconciledInterval, String> {
    let signed_seconds = (ended_at_utc - started_at_utc).num_seconds();
    if signed_seconds < 0 {
        return Err(format!(
            "active session starts in the future (start {}, observed end {}); refusing to create a negative duration",
            started_at_utc.to_rfc3339(),
            ended_at_utc.to_rfc3339()
        ));
    }

    let elapsed = Duration::from_secs(signed_seconds as u64);
    if !accept_large_interval && elapsed > MAX_UNATTENDED_WALL_INTERVAL {
        return Err(format!(
            "wall-clock interval is {} seconds, above the unattended safety limit of {} seconds; inspect the clock/profile and rerun stop with --accept-clock-jump only when this duration is intentional",
            elapsed.as_secs(),
            MAX_UNATTENDED_WALL_INTERVAL.as_secs()
        ));
    }

    let elapsed_seconds = usize::try_from(elapsed.as_secs())
        .map_err(|_| "session duration exceeds this platform's supported range".to_string())?;
    Ok(ReconciledInterval {
        ended_at_utc,
        elapsed_seconds,
    })
}

pub(crate) fn reconcile_live_interval(
    started_at_utc: DateTime<Utc>,
    observed_end_utc: DateTime<Utc>,
    monotonic_elapsed: Duration,
) -> Result<ReconciledInterval, String> {
    let elapsed_seconds = usize::try_from(monotonic_elapsed.as_secs())
        .map_err(|_| "session duration exceeds this platform's supported range".to_string())?;
    let expected_end_utc = started_at_utc
        + ChronoDuration::from_std(monotonic_elapsed)
            .map_err(|_| "monotonic session duration exceeds chrono's range".to_string())?;
    let skew_millis = (observed_end_utc - expected_end_utc)
        .num_milliseconds()
        .unsigned_abs();
    let allowed_millis = MAX_LIVE_CLOCK_SKEW.as_millis() as u64;

    if skew_millis > allowed_millis {
        let direction = if observed_end_utc < expected_end_utc {
            "backward"
        } else {
            "forward"
        };
        return Err(format!(
            "detected a substantial {direction} wall-clock discontinuity of {} milliseconds while monotonic elapsed time was {} seconds; active state was preserved for explicit recovery",
            skew_millis,
            monotonic_elapsed.as_secs()
        ));
    }

    Ok(ReconciledInterval {
        ended_at_utc: expected_end_utc,
        elapsed_seconds,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn config(offset_seconds: i32) -> DayBoundaryConfig {
        DayBoundaryConfig {
            mode: DayBoundaryMode::FixedHour,
            fixed_hour: 6,
            fixed_minute: 0,
            utc_offset_seconds: offset_seconds,
        }
    }

    #[test]
    fn future_start_is_rejected_instead_of_unsigned_casting() {
        let start = Utc.with_ymd_and_hms(2026, 8, 2, 0, 0, 0).single().unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 1, 23, 0, 0).single().unwrap();
        let error = checked_wall_interval(start, end, false).unwrap_err();
        assert!(error.contains("starts in the future"));
    }

    #[test]
    fn large_forward_wall_interval_requires_explicit_acceptance() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).single().unwrap();
        let end = start + ChronoDuration::days(8);
        let error = checked_wall_interval(start, end, false).unwrap_err();
        assert!(error.contains("--accept-clock-jump"));
        assert_eq!(
            checked_wall_interval(start, end, true)
                .unwrap()
                .elapsed_seconds,
            8 * 24 * 60 * 60
        );
    }

    #[test]
    fn live_backward_and_forward_clock_jumps_are_blocked() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).single().unwrap();
        let elapsed = Duration::from_secs(60);
        let backward = start - ChronoDuration::minutes(30);
        let forward = start + ChronoDuration::hours(4);
        assert!(reconcile_live_interval(start, backward, elapsed).is_err());
        assert!(reconcile_live_interval(start, forward, elapsed).is_err());
    }

    #[test]
    fn suspend_like_elapsed_time_is_accepted_when_monotonic_and_wall_agree() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).single().unwrap();
        let elapsed = Duration::from_secs(8 * 60 * 60);
        let observed = start + ChronoDuration::hours(8);
        let interval = reconcile_live_interval(start, observed, elapsed).unwrap();
        assert_eq!(interval.elapsed_seconds, 8 * 60 * 60);
        assert_eq!(interval.ended_at_utc, observed);
    }

    #[test]
    fn fixed_offset_civil_time_is_deterministic_across_dst_seasons() {
        let policy = config(-6 * 60 * 60);
        let winter = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).single().unwrap();
        let summer = Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).single().unwrap();
        assert_eq!(civil_from_utc(winter, &policy).unwrap().offset().local_minus_utc(), -21600);
        assert_eq!(civil_from_utc(summer, &policy).unwrap().offset().local_minus_utc(), -21600);
    }

    #[test]
    fn travel_changes_new_civil_projection_but_not_a_persisted_day_key() {
        let timestamp = Utc.with_ymd_and_hms(2026, 8, 2, 5, 30, 0).single().unwrap();
        let costa_rica = config(-6 * 60 * 60);
        let europe = config(2 * 60 * 60);
        let persisted_day = operational_day_from_utc(timestamp, &costa_rica).unwrap();
        assert_ne!(
            civil_from_utc(timestamp, &costa_rica).unwrap().time(),
            civil_from_utc(timestamp, &europe).unwrap().time()
        );
        assert_eq!(persisted_day.to_string(), "2026-08-01");
    }
}
''')

replace_once(
    "src/lib.rs",
    "mod sqlite;\nmod storage;",
    "mod sqlite;\nmod storage;\nmod temporal;",
)

replace_once(
    "src/domain.rs",
    "use crate::constants::COLORS;",
    "use crate::{constants::COLORS, temporal};",
)

old_operational = '''pub fn operational_day_key_now() -> NaiveDate {
    operational_day_key_from_utc(Utc::now(), &day_boundary_config())
}

pub fn operational_day_key_for_local(local: &DateTime<Local>) -> NaiveDate {
    operational_day_key_from_utc(local.with_timezone(&Utc), &day_boundary_config())
}

pub fn report_period_date_bounds_with_offset(
    period: ReportPeriod,
    offset: usize,
) -> (NaiveDate, NaiveDate) {
    let (start, end, _) = period_bounds_with_offset(period, offset);
    (start, end)
}

fn operational_day_key_from_utc(now_utc: DateTime<Utc>, config: &DayBoundaryConfig) -> NaiveDate {
    let offset = if let Some(offset) = FixedOffset::east_opt(config.utc_offset_seconds) {
        offset
    } else if let Some(offset) = FixedOffset::west_opt(6 * 60 * 60) {
        offset
    } else if let Some(offset) = FixedOffset::east_opt(0) {
        offset
    } else {
        return now_utc.date_naive();
    };
    let local = now_utc.with_timezone(&offset);

    let cutoff = match config.mode {
        DayBoundaryMode::FixedHour | DayBoundaryMode::Sunrise => {
            NaiveTime::from_hms_opt(config.fixed_hour, config.fixed_minute, 0)
                .or_else(|| NaiveTime::from_hms_opt(6, 0, 0))
                .unwrap_or(NaiveTime::MIN)
        }
    };

    let mut day = local.date_naive();
    if local.time() < cutoff {
        day -= ChronoDuration::days(1);
    }
    day
}
'''
new_operational = '''pub fn operational_day_key_now() -> NaiveDate {
    operational_day_key_from_utc(Utc::now(), &day_boundary_config())
}

pub fn operational_day_key_for_local(local: &DateTime<Local>) -> NaiveDate {
    operational_day_key_from_utc(local.with_timezone(&Utc), &day_boundary_config())
}

pub fn operational_day_key_for_utc(timestamp: DateTime<Utc>) -> NaiveDate {
    operational_day_key_from_utc(timestamp, &day_boundary_config())
}

pub fn civil_time_for_utc(timestamp: DateTime<Utc>) -> DateTime<FixedOffset> {
    temporal::civil_from_utc(timestamp, &day_boundary_config())
        .expect("runtime UTC offset must be validated before time authority is used")
}

pub fn report_period_date_bounds_with_offset(
    period: ReportPeriod,
    offset: usize,
) -> (NaiveDate, NaiveDate) {
    let (start, end, _) = period_bounds_with_offset(period, offset);
    (start, end)
}

pub(crate) fn operational_day_key_from_utc(
    now_utc: DateTime<Utc>,
    config: &DayBoundaryConfig,
) -> NaiveDate {
    temporal::operational_day_from_utc(now_utc, config)
        .expect("runtime time policy must be validated before operational-day allocation")
}
'''
replace_once("src/domain.rs", old_operational, new_operational)

replace_once(
    "src/domain.rs",
    "    pub fn start_session_with_elapsed(&mut self, elapsed_seconds: usize) {\n        let offset = Duration::from_secs(elapsed_seconds as u64);\n        self.current_session_start = Some(Instant::now() - offset);\n    }",
    "    pub fn start_session_with_elapsed(&mut self, elapsed_seconds: usize) {\n        let offset = Duration::from_secs(elapsed_seconds as u64);\n        self.current_session_start = Some(Instant::now() - offset);\n    }\n\n    pub fn current_elapsed(&self) -> Option<Duration> {\n        self.current_session_start.map(|start| start.elapsed())\n    }",
)

replace_once(
    "src/domain.rs",
    "pub struct LiveSessionPreview {\n    pub category_id: CategoryId,\n    pub description: String,\n    pub elapsed_seconds: usize,\n    pub now_local: DateTime<Local>,\n}",
    "pub struct LiveSessionPreview {\n    pub category_id: CategoryId,\n    pub description: String,\n    pub elapsed_seconds: usize,\n    pub now_civil: DateTime<FixedOffset>,\n}",
)
replace_all("src/domain.rs", "live.now_local", "live.now_civil", minimum=2)

# CLI stop becomes an explicit recovery decision and legacy civil time uses the validated fixed offset.
replace_once(
    "src/cli.rs",
    "use chrono::{DateTime, Duration as ChronoDuration, Local, Utc};",
    "use chrono::{DateTime, Duration as ChronoDuration, Utc};",
)
replace_once(
    "src/cli.rs",
    "        build_period_report, operational_day_key_for_local, runtime_settings,",
    "        build_period_report, civil_time_for_utc, operational_day_key_for_utc, runtime_settings,",
)
replace_once("src/cli.rs", "    sqlite, storage,", "    sqlite, storage, temporal,")
replace_once(
    "src/cli.rs",
    "    #[command(about = \"Stop the current tracking session\")]\n    Stop,",
    "    #[command(about = \"Stop the current tracking session\")]\n    Stop {\n        #[arg(\n            long,\n            help = \"Explicitly accept a wall-clock interval above the unattended safety limit\"\n        )]\n        accept_clock_jump: bool,\n    },",
)
replace_once(
    "src/cli.rs",
    "pub fn stop_session() -> Result<usize, String> {\n    match sqlite::resolve_runtime_authority()? {\n        sqlite::RuntimeAuthority::LegacyFiles => stop_session_legacy(),\n        sqlite::RuntimeAuthority::SqliteCli { database_path } => {\n            let stopped = sqlite::stop_cli_session(&database_path)?;",
    "pub fn stop_session(accept_clock_jump: bool) -> Result<usize, String> {\n    match sqlite::resolve_runtime_authority()? {\n        sqlite::RuntimeAuthority::LegacyFiles => stop_session_legacy(accept_clock_jump),\n        sqlite::RuntimeAuthority::SqliteCli { database_path } => {\n            let stopped = sqlite::stop_cli_session(&database_path, accept_clock_jump)?;",
)
replace_once(
    "src/cli.rs",
    "fn stop_session_legacy() -> Result<usize, String> {",
    "fn stop_session_legacy(accept_clock_jump: bool) -> Result<usize, String> {",
)
replace_once(
    "src/cli.rs",
    "    let elapsed = (Utc::now() - active_session.start_time).num_seconds() as usize;",
    "    let now_utc = Utc::now();\n    let interval = temporal::checked_wall_interval(\n        active_session.start_time,\n        now_utc,\n        accept_clock_jump,\n    )?;\n    let elapsed = interval.elapsed_seconds;",
)
replace_once(
    "src/cli.rs",
    "    let now = Local::now();\n    let today = operational_day_key_for_local(&now)\n        .format(\"%Y-%m-%d\")\n        .to_string();\n    let start_time = now - ChronoDuration::seconds(elapsed as i64);",
    "    let now = civil_time_for_utc(interval.ended_at_utc);\n    let today = operational_day_key_for_utc(interval.ended_at_utc)\n        .format(\"%Y-%m-%d\")\n        .to_string();\n    let start_time = now - ChronoDuration::seconds(elapsed as i64);",
)
replace_once(
    "src/cli.rs",
    "        Cli::Stop => {\n            if let Err(e) = stop_session() {",
    "        Cli::Stop { accept_clock_jump } => {\n            if let Err(e) = stop_session(accept_clock_jump) {",
)

# SQLite CLI recovery uses the same checked wall policy and explicit civil authority.
replace_once(
    "src/sqlite/cli_runtime.rs",
    "use chrono::{DateTime, FixedOffset, Local, SecondsFormat, Utc};",
    "use chrono::{DateTime, SecondsFormat, Utc};",
)
replace_once(
    "src/sqlite/cli_runtime.rs",
    "        Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, Session, is_drift_name,\n        operational_day_key_for_local, runtime_settings,",
    "        Category, CategoryId, DRIFT_CATEGORY_CONFIG_NAME, Session, civil_time_for_utc,\n        is_drift_name, operational_day_key_for_utc,",
)
replace_once(
    "src/sqlite/cli_runtime.rs",
    "};\n\nuse super::{",
    "    temporal,\n};\n\nuse super::{",
)
replace_once(
    "src/sqlite/cli_runtime.rs",
    "pub(crate) fn stop_session(database_path: &Path) -> Result<SqliteCliStopResult, String> {",
    "pub(crate) fn stop_session(\n    database_path: &Path,\n    accept_clock_jump: bool,\n) -> Result<SqliteCliStopResult, String> {",
)
replace_once(
    "src/sqlite/cli_runtime.rs",
    "        let ended_at = Utc::now();\n        let elapsed_i64 = (ended_at - started_at).num_seconds().max(0);\n        let operational_day = operational_day_key_for_local(&Local::now())\n            .format(\"%Y-%m-%d\")\n            .to_string();",
    "        let interval = temporal::checked_wall_interval(\n            started_at,\n            Utc::now(),\n            accept_clock_jump,\n        )?;\n        let ended_at = interval.ended_at_utc;\n        let elapsed_i64 = i64::try_from(interval.elapsed_seconds)\n            .map_err(|_| \"Active session duration exceeds SQLite's supported range\".to_string())?;\n        let operational_day = operational_day_key_for_utc(ended_at)\n            .format(\"%Y-%m-%d\")\n            .to_string();",
)
replace_once(
    "src/sqlite/cli_runtime.rs",
    "fn local_clock(timestamp: &str) -> Result<String, String> {\n    let utc = DateTime::parse_from_rfc3339(timestamp)\n        .map_err(|error| format!(\"Invalid SQLite session timestamp '{timestamp}': {error}\"))?\n        .with_timezone(&Utc);\n    let configured_offset = runtime_settings().day_boundary.utc_offset_seconds;\n    let offset = FixedOffset::east_opt(configured_offset)\n        .ok_or_else(|| format!(\"Configured UTC offset {configured_offset} is invalid\"))?;\n    Ok(utc.with_timezone(&offset).format(\"%H:%M:%S\").to_string())\n}",
    "fn local_clock(timestamp: &str) -> Result<String, String> {\n    let utc = DateTime::parse_from_rfc3339(timestamp)\n        .map_err(|error| format!(\"Invalid SQLite session timestamp '{timestamp}': {error}\"))?\n        .with_timezone(&Utc);\n    Ok(civil_time_for_utc(utc).format(\"%H:%M:%S\").to_string())\n}",
)

# TUI live duration is monotonic; wall time is reconciled only at transition boundaries.
replace_once(
    "src/app.rs",
    "        ReportPeriod, RuntimeSettings, TimeTracker, is_drift_category_id,\n        operational_day_key_for_local, set_runtime_settings,",
    "        ReportPeriod, RuntimeSettings, TimeTracker, civil_time_for_utc, is_drift_category_id,\n        operational_day_key_for_utc, set_runtime_settings,",
)
replace_once("src/app.rs", "    sqlite, storage,", "    sqlite, storage, temporal,")
replace_once(
    "src/app.rs",
    "enum UiMode {\n    Main,\n    CategoryModal,\n    KarmaModal,\n}",
    "enum UiMode {\n    Main,\n    CategoryModal,\n    KarmaModal,\n}\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\nenum SessionClockMode {\n    LiveMonotonic,\n    HistoricalWall,\n}",
)
replace_once(
    "src/app.rs",
    "                app.session.active_session_stable_id = Some(active.stable_id);\n                app.begin_active_session_at(active.started_at_utc);",
    "                app.session.active_session_stable_id = Some(active.stable_id);\n                app.begin_active_session_at(active.started_at_utc, false)?;",
)
replace_once(
    "src/app.rs",
    "            self.session.none_entry_time =\n                self.session\n                    .active_session_started_at_utc\n                    .map(|started_at| {\n                        let elapsed = (Utc::now() - started_at).to_std().unwrap_or(Duration::ZERO);\n                        Instant::now() - elapsed\n                    });",
    "            self.session.none_entry_time = self.time_tracker.current_session_start;",
)

old_begin_end = '''    fn begin_active_session_now(&mut self) {
        let now = Utc::now();
        self.time_tracker.start_session();
        self.session.active_session_started_at_utc = Some(now);
    }

    fn begin_active_session_at(&mut self, started_at_utc: DateTime<Utc>) {
        let now = Utc::now();
        let clamped_start = if started_at_utc > now {
            now
        } else {
            started_at_utc
        };
        let elapsed = (now - clamped_start).to_std().unwrap_or(Duration::ZERO);
        self.time_tracker
            .start_session_with_elapsed(elapsed.as_secs() as usize);
        self.session.active_session_started_at_utc = Some(clamped_start);
    }

    fn end_active_session_now(&mut self) -> Option<usize> {
        self.end_active_session_at(Utc::now())
    }

    fn end_active_session_at(&mut self, ended_at_utc: DateTime<Utc>) -> Option<usize> {
        let start_utc = self
            .session
            .active_session_started_at_utc
            .or_else(|| {
                self.time_tracker.current_session_start.map(|start| {
                    Utc::now()
                        - ChronoDuration::from_std(start.elapsed())
                            .unwrap_or(ChronoDuration::zero())
                })
            })
            .unwrap_or(ended_at_utc);

        let clamped_end = if ended_at_utc < start_utc {
            start_utc
        } else {
            ended_at_utc
        };

        let elapsed = (clamped_end - start_utc)
            .to_std()
            .unwrap_or(Duration::ZERO)
            .as_secs() as usize;
        let ended_local = clamped_end.with_timezone(&Local);
'''
new_begin_end = '''    fn begin_active_session_now(&mut self) {
        let now = Utc::now();
        self.time_tracker.start_session();
        self.session.active_session_started_at_utc = Some(now);
    }

    fn begin_active_session_at(
        &mut self,
        started_at_utc: DateTime<Utc>,
        accept_large_wall_interval: bool,
    ) -> Result<(), String> {
        let interval = temporal::checked_wall_interval(
            started_at_utc,
            Utc::now(),
            accept_large_wall_interval,
        )?;
        self.time_tracker
            .start_session_with_elapsed(interval.elapsed_seconds);
        self.session.active_session_started_at_utc = Some(started_at_utc);
        Ok(())
    }

    fn reconciled_active_interval(
        &self,
        observed_end_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Result<temporal::ReconciledInterval, String> {
        let started_at_utc = self
            .session
            .active_session_started_at_utc
            .ok_or_else(|| "active session is missing its UTC start timestamp".to_string())?;
        match clock_mode {
            SessionClockMode::LiveMonotonic => temporal::reconcile_live_interval(
                started_at_utc,
                observed_end_utc,
                self.time_tracker.current_elapsed().unwrap_or_default(),
            ),
            SessionClockMode::HistoricalWall => {
                temporal::checked_wall_interval(started_at_utc, observed_end_utc, true)
            }
        }
    }

    fn end_active_session_now(&mut self) -> Option<usize> {
        self.end_active_session_at(Utc::now(), SessionClockMode::LiveMonotonic)
    }

    fn end_active_session_at(
        &mut self,
        observed_end_utc: DateTime<Utc>,
        clock_mode: SessionClockMode,
    ) -> Option<usize> {
        let interval = match self.reconciled_active_interval(observed_end_utc, clock_mode) {
            Ok(interval) => interval,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    PersistenceOperation::ActiveFinish,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
                return None;
            }
        };
        let elapsed = interval.elapsed_seconds;
        let ended_civil = civil_time_for_utc(interval.ended_at_utc);
'''
replace_once("src/app.rs", old_begin_end, new_begin_end)
replace_all("src/app.rs", "clamped_end", "interval.ended_at_utc", minimum=2)
replace_all("src/app.rs", "ended_local", "ended_civil", minimum=2)
replace_once(
    "src/app.rs",
    "            let operational_day = operational_day_key_for_local(&ended_civil)\n                .format(\"%Y-%m-%d\")\n                .to_string();",
    "            let operational_day = operational_day_key_for_utc(interval.ended_at_utc)\n                .format(\"%Y-%m-%d\")\n                .to_string();",
)
replace_once(
    "src/app.rs",
    "    fn switch_active_category_at(\n        &mut self,\n        category_id: CategoryId,\n        switched_at_utc: DateTime<Utc>,\n    ) -> bool {",
    "    fn switch_active_category_at(\n        &mut self,\n        category_id: CategoryId,\n        switched_at_utc: DateTime<Utc>,\n        clock_mode: SessionClockMode,\n    ) -> bool {",
)
old_switch_elapsed = '''            let start_utc = self
                .session
                .active_session_started_at_utc
                .unwrap_or(switched_at_utc);
            let elapsed = (switched_at_utc - start_utc)
                .to_std()
                .unwrap_or(Duration::ZERO)
                .as_secs() as usize;
            let switched_local = switched_at_utc.with_timezone(&Local);
            let operational_day = operational_day_key_for_local(&switched_local)
                .format("%Y-%m-%d")
                .to_string();'''
new_switch_elapsed = '''            let interval = match self.reconciled_active_interval(switched_at_utc, clock_mode) {
                Ok(interval) => interval,
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        PersistenceOperation::ActiveSwitch,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return false;
                }
            };
            let elapsed = interval.elapsed_seconds;
            let operational_day = operational_day_key_for_utc(interval.ended_at_utc)
                .format("%Y-%m-%d")
                .to_string();'''
replace_once("src/app.rs", old_switch_elapsed, new_switch_elapsed)
replace_once(
    "src/app.rs",
    "                switched_at_utc,\n                &operational_day,",
    "                interval.ended_at_utc,\n                &operational_day,",
)
replace_once(
    "src/app.rs",
    "                switched_at_utc,\n                &category_id.0.to_string(),",
    "                interval.ended_at_utc,\n                &category_id.0.to_string(),",
)
replace_once(
    "src/app.rs",
    "            self.begin_active_session_at(switched_at_utc);",
    "            if let Err(error) = self.begin_active_session_at(\n                interval.ended_at_utc,\n                clock_mode == SessionClockMode::HistoricalWall,\n            ) {\n                self.record_storage_result_for::<()>(\n                    PersistenceOperation::ActiveStart,\n                    RecoveryAction::ReloadAuthority,\n                    Err(error),\n                );\n                return false;\n            }",
)
replace_once(
    "src/app.rs",
    "        self.end_active_session_at(switched_at_utc);",
    "        self.end_active_session_at(switched_at_utc, clock_mode);",
)
replace_once(
    "src/app.rs",
    "        self.begin_active_session_at(switched_at_utc);",
    "        if let Err(error) = self.begin_active_session_at(\n            switched_at_utc,\n            clock_mode == SessionClockMode::HistoricalWall,\n        ) {\n            self.record_storage_result_for::<()>(\n                PersistenceOperation::ActiveStart,\n                RecoveryAction::ReloadAuthority,\n                Err(error),\n            );\n            return false;\n        }",
)
replace_once(
    "src/app.rs",
    "            self.apply_mutation_at(mutation, Utc::now());",
    "            self.apply_mutation_at(mutation, Utc::now(), SessionClockMode::LiveMonotonic);",
)
replace_once(
    "src/app.rs",
    "    fn apply_mutation_at(&mut self, mutation: QueuedMutation, scheduled_at_utc: DateTime<Utc>) {",
    "    fn apply_mutation_at(\n        &mut self,\n        mutation: QueuedMutation,\n        scheduled_at_utc: DateTime<Utc>,\n        clock_mode: SessionClockMode,\n    ) {",
)
replace_once(
    "src/app.rs",
    "                self.apply_switch_layer_at(category_id, scheduled_at_utc);",
    "                self.apply_switch_layer_at(category_id, scheduled_at_utc, clock_mode);",
)
replace_once(
    "src/app.rs",
    "                    self.reset_active_session_at(scheduled_at_utc);",
    "                    self.reset_active_session_at(\n                        scheduled_at_utc,\n                        clock_mode == SessionClockMode::HistoricalWall,\n                    );",
)
replace_once(
    "src/app.rs",
    "    fn apply_switch_layer_at(&mut self, category_id: CategoryId, scheduled_at_utc: DateTime<Utc>) {\n        self.switch_active_category_at(category_id, scheduled_at_utc);\n    }",
    "    fn apply_switch_layer_at(\n        &mut self,\n        category_id: CategoryId,\n        scheduled_at_utc: DateTime<Utc>,\n        clock_mode: SessionClockMode,\n    ) {\n        self.switch_active_category_at(category_id, scheduled_at_utc, clock_mode);\n    }",
)
replace_once(
    "src/app.rs",
    "            self.apply_mutation_at(next.mutation, next.execute_at_utc);",
    "            self.apply_mutation_at(\n                next.mutation,\n                next.execute_at_utc,\n                SessionClockMode::HistoricalWall,\n            );",
)
replace_once(
    "src/app.rs",
    "    fn reset_active_session_at(&mut self, started_at_utc: DateTime<Utc>) {",
    "    fn reset_active_session_at(\n        &mut self,\n        started_at_utc: DateTime<Utc>,\n        accept_large_wall_interval: bool,\n    ) {",
)
replace_once(
    "src/app.rs",
    "        self.begin_active_session_at(started_at_utc);",
    "        if let Err(error) =\n            self.begin_active_session_at(started_at_utc, accept_large_wall_interval)\n        {\n            self.record_storage_result_for::<()>(\n                PersistenceOperation::ActiveStart,\n                RecoveryAction::ReloadAuthority,\n                Err(error),\n            );\n        }",
)

# Live report rows use the configured civil projection instead of the machine Local timezone.
replace_once(
    "src/app/report_state.rs",
    "use chrono::{Duration as ChronoDuration, Local, NaiveDate};",
    "use chrono::{Duration as ChronoDuration, NaiveDate, Utc};",
)
replace_once(
    "src/app/report_state.rs",
    "    build_period_karma_report_with_live_and_offset, operational_day_key_now,",
    "    build_period_karma_report_with_live_and_offset, civil_time_for_utc, operational_day_key_now,",
)
replace_once(
    "src/app/report_state.rs",
    "            now_local: Local::now(),",
    "            now_civil: civil_time_for_utc(Utc::now()),",
)

Path("tests/temporal_authority.rs").write_text(r'''#![cfg(target_os = "linux")]

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use serde_json::json;

struct TestProfile {
    root: PathBuf,
    data_home: PathBuf,
    state_home: PathBuf,
    config_home: PathBuf,
}

impl TestProfile {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "strata-temporal-{name}-{}-{nonce}",
            std::process::id()
        ));
        let data_home = root.join("data");
        let state_home = root.join("state");
        let config_home = root.join("config");
        fs::create_dir_all(data_home.join("strata")).unwrap();
        fs::create_dir_all(state_home.join("strata")).unwrap();
        fs::create_dir_all(config_home.join("strata")).unwrap();
        fs::write(
            data_home.join("strata/categories.csv"),
            "id,name,description,color_index,karma_effect\n1,Work,,0,1\n",
        )
        .unwrap();
        Self {
            root,
            data_home,
            state_home,
            config_home,
        }
    }

    fn active_path(&self) -> PathBuf {
        self.state_home.join("strata/active_session.json")
    }

    fn time_log_path(&self) -> PathBuf {
        self.data_home.join("strata/time_log.csv")
    }

    fn write_active_start(&self, started_at: chrono::DateTime<Utc>) {
        fs::write(
            self.active_path(),
            serde_json::to_vec_pretty(&json!({
                "project": "clock-test",
                "description": "",
                "category_id": 1,
                "category_name": "Work",
                "start_time": started_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_strata"))
            .args(args)
            .env("XDG_DATA_HOME", &self.data_home)
            .env("XDG_STATE_HOME", &self.state_home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env_remove("STRATA_DATA_DIR")
            .output()
            .unwrap()
    }
}

impl Drop for TestProfile {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn future_legacy_start_is_rejected_without_consuming_active_state() {
    let profile = TestProfile::new("future");
    profile.write_active_start(Utc::now() + ChronoDuration::hours(2));

    let output = profile.run(&["stop"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("starts in the future"));
    assert!(profile.active_path().exists());
    assert!(!profile.time_log_path().exists());
}

#[test]
fn large_wall_interval_requires_explicit_clock_jump_acceptance() {
    let profile = TestProfile::new("forward");
    profile.write_active_start(Utc::now() - ChronoDuration::days(8));

    let blocked = profile.run(&["stop"]);
    assert!(!blocked.status.success());
    assert!(stderr(&blocked).contains("--accept-clock-jump"));
    assert!(profile.active_path().exists());
    assert!(!profile.time_log_path().exists());

    let accepted = profile.run(&["stop", "--accept-clock-jump"]);
    assert!(accepted.status.success(), "{}", stderr(&accepted));
    assert!(!profile.active_path().exists());
    let log = fs::read_to_string(profile.time_log_path()).unwrap();
    assert!(log.contains("clock-test"));
}
''')
