from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise SystemExit(f"missing post-audit anchor in {path}: {old[:180]!r}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/temporal.rs",
    "pub(crate) const MAX_LIVE_CLOCK_SKEW: Duration = Duration::from_secs(5 * 60);",
    "pub(crate) const MAX_LIVE_CLOCK_SKEW: Duration = Duration::from_secs(5);",
)

replace_once(
    "src/domain.rs",
    "    pub fn start_session_with_elapsed(&mut self, elapsed_seconds: usize) {\n        let offset = Duration::from_secs(elapsed_seconds as u64);\n        self.current_session_start = Some(Instant::now() - offset);\n    }",
    "    pub fn start_session_with_elapsed(\n        &mut self,\n        elapsed_seconds: usize,\n    ) -> Result<(), String> {\n        let offset = Duration::from_secs(elapsed_seconds as u64);\n        let start = Instant::now().checked_sub(offset).ok_or_else(|| {\n            format!(\n                \"elapsed interval of {elapsed_seconds} seconds exceeds the monotonic clock range\"\n            )\n        })?;\n        self.current_session_start = Some(start);\n        Ok(())\n    }",
)

replace_once(
    "src/app.rs",
    "        self.time_tracker\n            .start_session_with_elapsed(interval.elapsed_seconds);\n        self.session.active_session_started_at_utc = Some(started_at_utc);\n        Ok(())\n    }\n\n    fn reconciled_active_interval(",
    "        self.time_tracker\n            .start_session_with_elapsed(interval.elapsed_seconds)?;\n        self.session.active_session_started_at_utc = Some(started_at_utc);\n        Ok(())\n    }\n\n    fn begin_transition_session(\n        &mut self,\n        started_at_utc: DateTime<Utc>,\n        clock_mode: SessionClockMode,\n    ) -> Result<(), String> {\n        match clock_mode {\n            SessionClockMode::LiveMonotonic => {\n                self.time_tracker.start_session();\n                self.session.active_session_started_at_utc = Some(started_at_utc);\n                Ok(())\n            }\n            SessionClockMode::HistoricalWall => {\n                self.begin_active_session_at(started_at_utc, true)\n            }\n        }\n    }\n\n    fn reconciled_active_interval(",
)

replace_once(
    "src/app.rs",
    "    fn reset_active_session_at(\n        &mut self,\n        started_at_utc: DateTime<Utc>,\n        accept_large_wall_interval: bool,\n    ) {\n        if let Some(database_path) = self.sqlite_database_path.clone() {",
    "    fn reset_active_session_at(\n        &mut self,\n        started_at_utc: DateTime<Utc>,\n        accept_large_wall_interval: bool,\n    ) {\n        if let Err(error) = temporal::checked_wall_interval(\n            started_at_utc,\n            Utc::now(),\n            accept_large_wall_interval,\n        ) {\n            self.record_storage_result_for::<()>(\n                PersistenceOperation::ActiveStart,\n                RecoveryAction::ReloadAuthority,\n                Err(error),\n            );\n            return;\n        }\n\n        if let Some(database_path) = self.sqlite_database_path.clone() {",
)

old_begin_after_switch = """            if let Err(error) = self.begin_active_session_at(
                interval.ended_at_utc,
                clock_mode == SessionClockMode::HistoricalWall,
            ) {"""
new_begin_after_switch = """            if let Err(error) =
                self.begin_transition_session(interval.ended_at_utc, clock_mode)
            {"""
replace_once("src/app.rs", old_begin_after_switch, new_begin_after_switch)

replace_once(
    "src/app.rs",
    "        self.end_active_session_at(switched_at_utc, clock_mode);\n        self.persist_sessions();",
    "        if self\n            .end_active_session_at(switched_at_utc, clock_mode)\n            .is_none()\n        {\n            return false;\n        }\n        self.persist_sessions();",
)

old_legacy_begin = """        if let Err(error) = self.begin_active_session_at(
            switched_at_utc,
            clock_mode == SessionClockMode::HistoricalWall,
        ) {"""
new_legacy_begin = """        if let Err(error) = self.begin_transition_session(switched_at_utc, clock_mode) {"""
replace_once("src/app.rs", old_legacy_begin, new_legacy_begin)

# Verify the tighter tolerance still permits ordinary scheduler jitter while
# rejecting a real clock correction.
temporal = Path("src/temporal.rs").read_text()
marker = """    #[test]
    fn live_backward_and_forward_clock_jumps_are_blocked() {"""
insert = """    #[test]
    fn ordinary_subsecond_wall_jitter_uses_monotonic_elapsed() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).single().unwrap();
        let elapsed = Duration::from_secs(60);
        let observed = start + ChronoDuration::seconds(61);
        let interval = reconcile_live_interval(start, observed, elapsed).unwrap();
        assert_eq!(interval.elapsed_seconds, 60);
        assert_eq!(interval.ended_at_utc, start + ChronoDuration::seconds(60));
    }

"""
if marker not in temporal:
    raise SystemExit("missing temporal test insertion marker")
Path("src/temporal.rs").write_text(temporal.replace(marker, insert + marker, 1))
