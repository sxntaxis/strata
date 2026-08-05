use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{domain::DRIFT_CATEGORY_ID, temporal};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ActiveIntervalReceipt {
    pub category_id: u64,
    pub description: String,
    pub started_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ClearAllReceipt {
    pub operation_id: String,
    pub applied_at_utc: DateTime<Utc>,
    pub previous_active: ActiveIntervalReceipt,
    pub resulting_active: ActiveIntervalReceipt,
    pub idle_reset: bool,
    pub previous_elapsed_seconds: usize,
    pub affected_operational_days: Vec<String>,
}

impl ClearAllReceipt {
    pub(crate) fn validate_boundaries(&self) -> Result<(), String> {
        if self.previous_active.category_id != self.resulting_active.category_id
            || self.previous_active.description != self.resulting_active.description
        {
            return Err(format!(
                "clear-all receipt {} changes active classification",
                self.operation_id
            ));
        }
        if self.applied_at_utc < self.previous_active.started_at_utc {
            return Err(format!(
                "clear-all receipt {} predates its active generation",
                self.operation_id
            ));
        }
        let wall_seconds = u64::try_from(
            (self.applied_at_utc - self.previous_active.started_at_utc).num_seconds(),
        )
        .map_err(|_| {
            format!(
                "clear-all receipt {} has an invalid wall interval",
                self.operation_id
            )
        })?;
        let elapsed_seconds = u64::try_from(self.previous_elapsed_seconds).map_err(|_| {
            format!(
                "clear-all receipt {} elapsed value exceeds the supported range",
                self.operation_id
            )
        })?;
        if wall_seconds.abs_diff(elapsed_seconds) > temporal::MAX_LIVE_CLOCK_SKEW.as_secs() {
            return Err(format!(
                "clear-all receipt {} elapsed payload diverges from its UTC interval",
                self.operation_id
            ));
        }
        if self.idle_reset {
            if self.previous_active.category_id != DRIFT_CATEGORY_ID.0 {
                return Err(format!(
                    "clear-all receipt {} resets a non-idle active generation",
                    self.operation_id
                ));
            }
            if self.resulting_active.started_at_utc != self.applied_at_utc {
                return Err(format!(
                    "clear-all receipt {} has inconsistent idle reset time",
                    self.operation_id
                ));
            }
        } else {
            if self.previous_active.category_id == DRIFT_CATEGORY_ID.0 {
                return Err(format!(
                    "clear-all receipt {} leaves an idle generation unreset",
                    self.operation_id
                ));
            }
            if self.resulting_active.started_at_utc != self.previous_active.started_at_utc {
                return Err(format!(
                    "clear-all receipt {} changes a non-idle active start",
                    self.operation_id
                ));
            }
        }
        if self.affected_operational_days.is_empty() {
            return Err(format!(
                "clear-all receipt {} has no affected operational day",
                self.operation_id
            ));
        }
        let mut previous = None;
        for value in &self.affected_operational_days {
            let day = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|error| {
                format!(
                    "clear-all receipt {} has invalid operational day '{}': {error}",
                    self.operation_id, value
                )
            })?;
            if previous.is_some_and(|prior| prior >= day) {
                return Err(format!(
                    "clear-all receipt {} operational days are not unique and sorted",
                    self.operation_id
                ));
            }
            previous = Some(day);
        }
        Ok(())
    }
}
