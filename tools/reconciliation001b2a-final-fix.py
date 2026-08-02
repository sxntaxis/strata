from pathlib import Path

legacy_path = Path("src/legacy_transition.rs")
legacy = legacy_path.read_text()

legacy = legacy.replace(
    "use chrono::{DateTime, Utc};\nuse serde::{Deserialize, Serialize};\n\nuse crate::domain::{CategoryId, OperationalDayPolicy, Session};",
    "use chrono::{\n    DateTime, Duration as ChronoDuration, FixedOffset, NaiveDate, NaiveTime, Utc,\n};\nuse serde::{Deserialize, Serialize};\n\nuse crate::{\n    domain::{CategoryId, OperationalDayPolicy, Session},\n    temporal,\n};",
)

old_policy = '''    fn operational_day_policy(&self) -> Result<Option<OperationalDayPolicy>, String> {
        match (
            self.operational_day_utc_offset_seconds,
            self.operational_day_start_minutes,
        ) {
            (None, None) => Ok(None),
            (Some(utc_offset_seconds), Some(start_minutes)) => Ok(Some(OperationalDayPolicy {
                utc_offset_seconds,
                start_minutes,
            })),
            _ => Err(format!(
                "legacy transition session {} has incomplete operational-day policy",
                self.id
            )),
        }
    }

    pub(crate) fn to_session(&self) -> Result<Session, String> {
        Ok(Session {
'''
new_policy = '''    fn operational_day_policy(&self) -> Result<Option<OperationalDayPolicy>, String> {
        match (
            self.operational_day_utc_offset_seconds,
            self.operational_day_start_minutes,
        ) {
            (None, None) => Ok(None),
            (Some(utc_offset_seconds), Some(start_minutes)) => {
                if FixedOffset::east_opt(utc_offset_seconds).is_none() {
                    return Err(format!(
                        "legacy transition session {} has invalid UTC offset {}",
                        self.id, utc_offset_seconds
                    ));
                }
                if start_minutes > 1439 {
                    return Err(format!(
                        "legacy transition session {} has invalid operational-day start minute {}",
                        self.id, start_minutes
                    ));
                }
                Ok(Some(OperationalDayPolicy {
                    utc_offset_seconds,
                    start_minutes,
                }))
            }
            _ => Err(format!(
                "legacy transition session {} has incomplete operational-day policy",
                self.id
            )),
        }
    }

    fn validate_payload(&self) -> Result<(), String> {
        if self.id == 0 {
            return Err("legacy transition session ID 0 is reserved".to_string());
        }
        if self.elapsed_seconds == 0 {
            return Err(format!(
                "legacy transition session {} has zero elapsed seconds",
                self.id
            ));
        }
        let started_at_utc = self.started_at_utc.ok_or_else(|| {
            format!(
                "legacy transition session {} has no authoritative start timestamp",
                self.id
            )
        })?;
        let ended_at_utc = self.ended_at_utc.ok_or_else(|| {
            format!(
                "legacy transition session {} has no authoritative end timestamp",
                self.id
            )
        })?;
        let policy = self.operational_day_policy()?.ok_or_else(|| {
            format!(
                "legacy transition session {} has no operational-day policy",
                self.id
            )
        })?;
        let elapsed = i64::try_from(self.elapsed_seconds).map_err(|_| {
            format!(
                "legacy transition session {} duration exceeds chrono range",
                self.id
            )
        })?;
        let expected_end = started_at_utc
            .checked_add_signed(ChronoDuration::seconds(elapsed))
            .ok_or_else(|| {
                format!(
                    "legacy transition session {} end exceeds chrono range",
                    self.id
                )
            })?;
        if ended_at_utc != expected_end {
            return Err(format!(
                "legacy transition session {} timestamps do not conserve {} elapsed seconds",
                self.id, self.elapsed_seconds
            ));
        }

        let start_civil = temporal::civil_from_policy(started_at_utc, policy)?;
        let end_civil = temporal::civil_from_policy(ended_at_utc, policy)?;
        let expected_start_time = start_civil.format("%H:%M:%S").to_string();
        let expected_end_time = end_civil.format("%H:%M:%S").to_string();
        if self.start_time != expected_start_time || self.end_time != expected_end_time {
            return Err(format!(
                "legacy transition session {} civil clock labels do not match authoritative UTC",
                self.id
            ));
        }

        let cutoff = NaiveTime::from_num_seconds_from_midnight_opt(
            u32::from(policy.start_minutes) * 60,
            0,
        )
        .ok_or_else(|| {
            format!(
                "legacy transition session {} has invalid operational-day cutoff",
                self.id
            )
        })?;
        let mut expected_day = end_civil.date_naive();
        if end_civil.time() < cutoff {
            expected_day = expected_day.pred_opt().ok_or_else(|| {
                format!(
                    "legacy transition session {} operational day is outside chrono range",
                    self.id
                )
            })?;
        }
        let recorded_day = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").map_err(|error| {
            format!(
                "legacy transition session {} has invalid operational day '{}': {error}",
                self.id, self.date
            )
        })?;
        if recorded_day != expected_day {
            return Err(format!(
                "legacy transition session {} operational day {} does not match authoritative end projection {}",
                self.id, recorded_day, expected_day
            ));
        }
        Ok(())
    }

    pub(crate) fn to_session(&self) -> Result<Session, String> {
        self.validate_payload()?;
        Ok(Session {
'''
if old_policy not in legacy:
    raise SystemExit("legacy policy anchor not found")
legacy = legacy.replace(old_policy, new_policy)

start = legacy.index("    pub(crate) fn validate_switch_boundaries(&self) -> Result<(), String> {")
end = legacy.index("\n    }\n}\n\npub(crate) fn reconcile_completed_session", start) + len("\n    }\n")
new_validate = '''    pub(crate) fn validate_switch_boundaries(&self) -> Result<(), String> {
        if self.version != Self::VERSION {
            return Err(format!(
                "unsupported legacy transition receipt version {}",
                self.version
            ));
        }
        if self.kind != LegacyTransitionKind::Switch {
            return Err("unsupported legacy transition kind".to_string());
        }
        if self.resulting_active.category_id == self.expected_previous_category_id {
            return Err(format!(
                "legacy switch receipt {} does not change category",
                self.operation_id
            ));
        }
        if self.resulting_active.started_at_utc != self.transition_at_utc {
            return Err(format!(
                "legacy switch receipt {} has inconsistent resulting start time",
                self.operation_id
            ));
        }
        if self.transition_at_utc < self.expected_previous_started_at_utc {
            return Err(format!(
                "legacy switch receipt {} transitions before its previous active start",
                self.operation_id
            ));
        }

        let whole_elapsed = usize::try_from(
            (self.transition_at_utc - self.expected_previous_started_at_utc).num_seconds(),
        )
        .map_err(|_| {
            format!(
                "legacy switch receipt {} duration exceeds this platform's range",
                self.operation_id
            )
        })?;

        match (whole_elapsed, self.completed_session.as_ref()) {
            (0, None) => {}
            (0, Some(_)) => {
                return Err(format!(
                    "legacy switch receipt {} stores a completed row for a zero-whole-second transition",
                    self.operation_id
                ));
            }
            (_, None) => {
                return Err(format!(
                    "legacy switch receipt {} omits {} completed whole seconds",
                    self.operation_id, whole_elapsed
                ));
            }
            (expected_elapsed, Some(completed)) => {
                completed.validate_payload()?;
                if completed.category_id != self.expected_previous_category_id {
                    return Err(format!(
                        "legacy switch receipt {} completed the wrong category",
                        self.operation_id
                    ));
                }
                if completed.elapsed_seconds != expected_elapsed {
                    return Err(format!(
                        "legacy switch receipt {} completed {} seconds but its active boundary owns {}",
                        self.operation_id, completed.elapsed_seconds, expected_elapsed
                    ));
                }
                if completed.ended_at_utc != Some(self.transition_at_utc) {
                    return Err(format!(
                        "legacy switch receipt {} has inconsistent completion time",
                        self.operation_id
                    ));
                }
                let elapsed = i64::try_from(expected_elapsed).map_err(|_| {
                    format!(
                        "legacy switch receipt {} duration exceeds chrono range",
                        self.operation_id
                    )
                })?;
                let expected_completed_start = self
                    .transition_at_utc
                    .checked_sub_signed(ChronoDuration::seconds(elapsed))
                    .ok_or_else(|| {
                        format!(
                            "legacy switch receipt {} completed start exceeds chrono range",
                            self.operation_id
                        )
                    })?;
                if completed.started_at_utc != Some(expected_completed_start) {
                    return Err(format!(
                        "legacy switch receipt {} completed start does not preserve its whole-second interval",
                        self.operation_id
                    ));
                }
            }
        }
        Ok(())
    }
'''
legacy = legacy[:start] + new_validate + legacy[end:]

legacy = legacy.replace(
    "    use chrono::{TimeZone, Utc};",
    "    use chrono::{TimeZone, Timelike, Utc};",
)

insert_before = '''    #[test]
    fn absent_receipt_session_is_appended_once() {
'''
new_tests = '''    #[test]
    fn subsecond_monotonic_remainder_replays_with_canonical_whole_second_start() {
        let previous_start = Utc
            .with_ymd_and_hms(2026, 8, 2, 16, 0, 0)
            .unwrap()
            .with_nanosecond(100_000_000)
            .unwrap();
        let transition = previous_start + ChronoDuration::milliseconds(5_900);
        let mut completed = session(7, "work");
        completed.elapsed_seconds = 5;
        completed.started_at_utc = Some(transition - ChronoDuration::seconds(5));
        completed.ended_at_utc = Some(transition);
        completed.start_time = "10:00:01".to_string();
        completed.end_time = "10:00:06".to_string();

        let mut receipt = switch_receipt(Some(LegacySessionReceipt::from_session(&completed)));
        receipt.expected_previous_started_at_utc = previous_start;
        receipt.transition_at_utc = transition;
        receipt.resulting_active.started_at_utc = transition;
        receipt.validate_switch_boundaries().unwrap();
    }

    #[test]
    fn receipt_requires_completed_row_exactly_when_whole_seconds_exist() {
        let missing = switch_receipt(None);
        assert!(
            missing
                .validate_switch_boundaries()
                .unwrap_err()
                .contains("omits 3600 completed whole seconds")
        );

        let previous_start = Utc
            .with_ymd_and_hms(2026, 8, 2, 16, 0, 0)
            .unwrap()
            .with_nanosecond(100_000_000)
            .unwrap();
        let transition = previous_start + ChronoDuration::milliseconds(500);
        let mut unexpected = switch_receipt(Some(LegacySessionReceipt::from_session(&session(
            7, "work",
        ))));
        unexpected.expected_previous_started_at_utc = previous_start;
        unexpected.transition_at_utc = transition;
        unexpected.resulting_active.started_at_utc = transition;
        assert!(
            unexpected
                .validate_switch_boundaries()
                .unwrap_err()
                .contains("zero-whole-second transition")
        );
    }

'''
if insert_before not in legacy:
    raise SystemExit("legacy test anchor not found")
legacy = legacy.replace(insert_before, new_tests + insert_before)
legacy_path.write_text(legacy)

storage_path = Path("src/storage.rs")
storage = storage_path.read_text()
old_id = '''        let id = id_raw
            .parse::<usize>()
            .map_err(|error| StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: format!("invalid session ID '{id_raw}': {error}"),
            })?;
        if !seen_ids.insert(id) {
'''
new_id = '''        let id = id_raw
            .parse::<usize>()
            .map_err(|error| StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: format!("invalid session ID '{id_raw}': {error}"),
            })?;
        if id == 0 {
            return Err(StorageError::InvalidCsvData {
                file: "time_log.csv",
                row,
                message: "session ID 0 is reserved".to_string(),
            });
        }
        if !seen_ids.insert(id) {
'''
if old_id not in storage:
    raise SystemExit("storage ID anchor not found")
storage = storage.replace(old_id, new_id)

old_test = '''        let duplicate_path = unique_path("strata_sessions_duplicate_id", "csv");
'''
new_test = '''        let zero_id_path = unique_path("strata_sessions_zero_id", "csv");
        fs::write(
            &zero_id_path,
            "id,date,category_id,category_name,description,start_time,end_time,elapsed_seconds\\n0,2026-08-01,0,idle,break,10:00:00,11:00:00,3600\\n",
        )
        .unwrap();
        let zero_id = try_load_sessions_from_csv(&zero_id_path, &categories).unwrap_err();
        assert!(zero_id.to_string().contains("session ID 0 is reserved"));

        let duplicate_path = unique_path("strata_sessions_duplicate_id", "csv");
'''
if old_test not in storage:
    raise SystemExit("storage test anchor not found")
storage = storage.replace(old_test, new_test)
storage = storage.replace(
    "        fs::remove_file(malformed_id_path).ok();\n        fs::remove_file(duplicate_path).ok();",
    "        fs::remove_file(malformed_id_path).ok();\n        fs::remove_file(zero_id_path).ok();\n        fs::remove_file(duplicate_path).ok();",
)
storage_path.write_text(storage)

Path("tools/reconciliation001b2a-final-fix.py").unlink()
Path(".github/workflows/reconciliation001b2a-final-fix.yml").unlink()
