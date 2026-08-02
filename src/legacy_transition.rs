use chrono::{DateTime, Duration as ChronoDuration, FixedOffset, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{CategoryId, OperationalDayPolicy, Session},
    temporal,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum LegacyTransitionKind {
    Switch,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LegacySessionReceipt {
    pub id: usize,
    pub date: String,
    pub category_id: u64,
    pub project: String,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: usize,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub operational_day_utc_offset_seconds: Option<i32>,
    pub operational_day_start_minutes: Option<u16>,
}

impl LegacySessionReceipt {
    pub(crate) fn from_session(session: &Session) -> Self {
        let (operational_day_utc_offset_seconds, operational_day_start_minutes) = session
            .operational_day_policy
            .map(|policy| (Some(policy.utc_offset_seconds), Some(policy.start_minutes)))
            .unwrap_or((None, None));
        Self {
            id: session.id,
            date: session.date.clone(),
            category_id: session.category_id.0,
            project: session.project.clone(),
            description: session.description.clone(),
            start_time: session.start_time.clone(),
            end_time: session.end_time.clone(),
            elapsed_seconds: session.elapsed_seconds,
            started_at_utc: session.started_at_utc,
            ended_at_utc: session.ended_at_utc,
            operational_day_utc_offset_seconds,
            operational_day_start_minutes,
        }
    }

    fn operational_day_policy(&self) -> Result<Option<OperationalDayPolicy>, String> {
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

    pub(crate) fn validate_payload(&self) -> Result<(), String> {
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

        let cutoff =
            NaiveTime::from_num_seconds_from_midnight_opt(u32::from(policy.start_minutes) * 60, 0)
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
            id: self.id,
            date: self.date.clone(),
            category_id: CategoryId::new(self.category_id),
            project: self.project.clone(),
            description: self.description.clone(),
            start_time: self.start_time.clone(),
            end_time: self.end_time.clone(),
            elapsed_seconds: self.elapsed_seconds,
            started_at_utc: self.started_at_utc,
            ended_at_utc: self.ended_at_utc,
            operational_day_policy: self.operational_day_policy()?,
        })
    }

    pub(crate) fn matches_session(&self, session: &Session) -> bool {
        Self::from_session(session) == *self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LegacyActiveReceipt {
    pub category_id: u64,
    pub description: String,
    pub started_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LegacyTransitionReceipt {
    pub version: u8,
    pub operation_id: String,
    pub kind: LegacyTransitionKind,
    pub expected_previous_category_id: u64,
    pub expected_previous_started_at_utc: DateTime<Utc>,
    pub transition_at_utc: DateTime<Utc>,
    pub completed_session: Option<LegacySessionReceipt>,
    pub resulting_active: LegacyActiveReceipt,
}

impl LegacyTransitionReceipt {
    pub(crate) const VERSION: u8 = 1;

    pub(crate) fn validate_switch_boundaries(&self) -> Result<(), String> {
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LegacyFinishReceipt {
    pub version: u8,
    pub operation_id: String,
    pub expected_previous_category_id: u64,
    pub expected_previous_description: String,
    pub expected_previous_started_at_utc: DateTime<Utc>,
    pub finished_at_utc: DateTime<Utc>,
    pub completed_session: Option<LegacySessionReceipt>,
}

impl LegacyFinishReceipt {
    pub(crate) const VERSION: u8 = 1;

    pub(crate) fn validate_boundaries(&self) -> Result<(), String> {
        if self.version != Self::VERSION {
            return Err(format!(
                "unsupported legacy finish receipt version {}",
                self.version
            ));
        }
        if self.finished_at_utc < self.expected_previous_started_at_utc {
            return Err(format!(
                "legacy finish receipt {} ends before its active start",
                self.operation_id
            ));
        }
        let whole_elapsed = usize::try_from(
            (self.finished_at_utc - self.expected_previous_started_at_utc).num_seconds(),
        )
        .map_err(|_| {
            format!(
                "legacy finish receipt {} duration exceeds this platform's range",
                self.operation_id
            )
        })?;
        match (whole_elapsed, self.completed_session.as_ref()) {
            (0, None) => Ok(()),
            (0, Some(_)) => Err(format!(
                "legacy finish receipt {} stores a completed row for a zero-whole-second finish",
                self.operation_id
            )),
            (_, None) => Err(format!(
                "legacy finish receipt {} omits {} completed whole seconds",
                self.operation_id, whole_elapsed
            )),
            (expected_elapsed, Some(completed)) => {
                completed.validate_payload()?;
                if completed.category_id != self.expected_previous_category_id {
                    return Err(format!(
                        "legacy finish receipt {} completed the wrong category",
                        self.operation_id
                    ));
                }
                if completed.description != self.expected_previous_description {
                    return Err(format!(
                        "legacy finish receipt {} completed the wrong description",
                        self.operation_id
                    ));
                }
                if completed.elapsed_seconds != expected_elapsed {
                    return Err(format!(
                        "legacy finish receipt {} completed {} seconds but its active boundary owns {}",
                        self.operation_id, completed.elapsed_seconds, expected_elapsed
                    ));
                }
                if completed.ended_at_utc != Some(self.finished_at_utc) {
                    return Err(format!(
                        "legacy finish receipt {} has inconsistent completion time",
                        self.operation_id
                    ));
                }
                let elapsed = i64::try_from(expected_elapsed).map_err(|_| {
                    format!(
                        "legacy finish receipt {} duration exceeds chrono range",
                        self.operation_id
                    )
                })?;
                let expected_completed_start = self
                    .finished_at_utc
                    .checked_sub_signed(ChronoDuration::seconds(elapsed))
                    .ok_or_else(|| {
                        format!(
                            "legacy finish receipt {} completed start exceeds chrono range",
                            self.operation_id
                        )
                    })?;
                if completed.started_at_utc != Some(expected_completed_start) {
                    return Err(format!(
                        "legacy finish receipt {} completed start does not preserve its whole-second interval",
                        self.operation_id
                    ));
                }
                Ok(())
            }
        }
    }
}

pub(crate) fn reconcile_completed_session(
    sessions: &mut Vec<Session>,
    next_session_id: &mut usize,
    receipt: Option<&LegacySessionReceipt>,
) -> Result<(), String> {
    let Some(receipt) = receipt else {
        return Ok(());
    };

    if let Some(existing) = sessions.iter().find(|session| session.id == receipt.id) {
        if receipt.matches_session(existing) {
            *next_session_id = (*next_session_id).max(receipt.id.saturating_add(1));
            return Ok(());
        }
        return Err(format!(
            "legacy transition session ID {} conflicts with existing history",
            receipt.id
        ));
    }

    if sessions.iter().any(|session| session.id > receipt.id) {
        return Err(format!(
            "legacy transition session ID {} is older than already published history",
            receipt.id
        ));
    }

    sessions.push(receipt.to_session()?);
    sessions.sort_by_key(|session| session.id);
    *next_session_id = (*next_session_id).max(receipt.id.saturating_add(1));
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Timelike, Utc};

    use super::*;

    fn session(id: usize, description: &str) -> Session {
        Session {
            id,
            date: "2026-08-02".to_string(),
            category_id: CategoryId::new(4),
            project: String::new(),
            description: description.to_string(),
            start_time: "10:00:00".to_string(),
            end_time: "11:00:00".to_string(),
            elapsed_seconds: 3600,
            started_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap()),
            ended_at_utc: Some(Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap()),
            operational_day_policy: Some(OperationalDayPolicy {
                utc_offset_seconds: -21600,
                start_minutes: 360,
            }),
        }
    }

    fn switch_receipt(completed_session: Option<LegacySessionReceipt>) -> LegacyTransitionReceipt {
        LegacyTransitionReceipt {
            version: LegacyTransitionReceipt::VERSION,
            operation_id: "legacy-switch:test".to_string(),
            kind: LegacyTransitionKind::Switch,
            expected_previous_category_id: 4,
            expected_previous_started_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap(),
            transition_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap(),
            completed_session,
            resulting_active: LegacyActiveReceipt {
                category_id: 5,
                description: String::new(),
                started_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap(),
            },
        }
    }

    #[test]
    fn switch_receipt_validates_all_temporal_and_category_boundaries() {
        let completed = LegacySessionReceipt::from_session(&session(7, "work"));
        switch_receipt(Some(completed.clone()))
            .validate_switch_boundaries()
            .unwrap();

        let mut wrong_category = switch_receipt(Some(completed.clone()));
        wrong_category.expected_previous_category_id = 99;
        assert!(wrong_category.validate_switch_boundaries().is_err());

        let mut wrong_transition = switch_receipt(Some(completed));
        wrong_transition.resulting_active.started_at_utc =
            Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 1).unwrap();
        assert!(wrong_transition.validate_switch_boundaries().is_err());
    }

    #[test]
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

    fn finish_receipt(completed_session: Option<LegacySessionReceipt>) -> LegacyFinishReceipt {
        LegacyFinishReceipt {
            version: LegacyFinishReceipt::VERSION,
            operation_id: "legacy-finish:test".to_string(),
            expected_previous_category_id: 4,
            expected_previous_description: "work".to_string(),
            expected_previous_started_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap(),
            finished_at_utc: Utc.with_ymd_and_hms(2026, 8, 2, 17, 0, 0).unwrap(),
            completed_session,
        }
    }

    #[test]
    fn finish_receipt_validates_completed_and_zero_second_boundaries() {
        let completed = LegacySessionReceipt::from_session(&session(7, "work"));
        finish_receipt(Some(completed))
            .validate_boundaries()
            .unwrap();

        let start = Utc.with_ymd_and_hms(2026, 8, 2, 16, 0, 0).unwrap();
        let mut zero = finish_receipt(None);
        zero.expected_previous_started_at_utc = start;
        zero.finished_at_utc = start + ChronoDuration::milliseconds(900);
        zero.validate_boundaries().unwrap();
    }

    #[test]
    fn finish_receipt_rejects_missing_or_wrong_completion() {
        assert!(
            finish_receipt(None)
                .validate_boundaries()
                .unwrap_err()
                .contains("omits 3600 completed whole seconds")
        );
        let mut wrong = session(7, "other");
        wrong.description = "other".to_string();
        assert!(
            finish_receipt(Some(LegacySessionReceipt::from_session(&wrong)))
                .validate_boundaries()
                .unwrap_err()
                .contains("wrong description")
        );
    }

    #[test]
    fn absent_receipt_session_is_appended_once() {
        let receipt = LegacySessionReceipt::from_session(&session(7, "work"));
        let mut sessions = vec![session(6, "earlier")];
        let mut next_id = 7;
        reconcile_completed_session(&mut sessions, &mut next_id, Some(&receipt)).unwrap();
        reconcile_completed_session(&mut sessions, &mut next_id, Some(&receipt)).unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[1].id, 7);
        assert_eq!(next_id, 8);
    }

    #[test]
    fn conflicting_same_id_fails_closed() {
        let receipt = LegacySessionReceipt::from_session(&session(7, "receipt"));
        let mut sessions = vec![session(7, "different")];
        let mut next_id = 8;
        let error =
            reconcile_completed_session(&mut sessions, &mut next_id, Some(&receipt)).unwrap_err();
        assert!(error.contains("conflicts with existing history"));
        assert_eq!(sessions[0].description, "different");
    }

    #[test]
    fn older_missing_id_cannot_be_inserted_behind_newer_history() {
        let receipt = LegacySessionReceipt::from_session(&session(7, "receipt"));
        let mut sessions = vec![session(8, "newer")];
        let mut next_id = 9;
        let error =
            reconcile_completed_session(&mut sessions, &mut next_id, Some(&receipt)).unwrap_err();
        assert!(error.contains("older than already published history"));
    }
}
