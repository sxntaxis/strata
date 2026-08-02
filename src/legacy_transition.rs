use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{CategoryId, OperationalDayPolicy, Session};

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
    use chrono::{TimeZone, Utc};

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
