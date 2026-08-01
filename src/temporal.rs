use std::time::Duration;

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
        let winter = Utc
            .with_ymd_and_hms(2026, 1, 15, 12, 0, 0)
            .single()
            .unwrap();
        let summer = Utc
            .with_ymd_and_hms(2026, 7, 15, 12, 0, 0)
            .single()
            .unwrap();
        assert_eq!(
            civil_from_utc(winter, &policy)
                .unwrap()
                .offset()
                .local_minus_utc(),
            -21600
        );
        assert_eq!(
            civil_from_utc(summer, &policy)
                .unwrap()
                .offset()
                .local_minus_utc(),
            -21600
        );
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
