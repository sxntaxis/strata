use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, FixedOffset, NaiveDate, NaiveTime, Utc};

use crate::domain::{DayBoundaryConfig, OperationalDayPolicy};

pub(crate) const MAX_LIVE_CLOCK_SKEW: Duration = Duration::from_secs(5);
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
    let cutoff =
        NaiveTime::from_hms_opt(config.fixed_hour, config.fixed_minute, 0).ok_or_else(|| {
            format!(
                "configured operational-day cutoff {:02}:{:02} is invalid",
                config.fixed_hour, config.fixed_minute
            )
        })?;

    let mut day = civil.date_naive();
    if civil.time() < cutoff {
        day -= ChronoDuration::days(1);
    }
    Ok(day)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OperationalDaySlice {
    pub operational_day: NaiveDate,
    pub started_at_utc: DateTime<Utc>,
    pub ended_at_utc: DateTime<Utc>,
    pub elapsed_seconds: usize,
}

pub(crate) fn civil_from_policy(
    timestamp: DateTime<Utc>,
    policy: OperationalDayPolicy,
) -> Result<DateTime<FixedOffset>, String> {
    let offset = FixedOffset::east_opt(policy.utc_offset_seconds).ok_or_else(|| {
        format!(
            "operational-day UTC offset {} is invalid",
            policy.utc_offset_seconds
        )
    })?;
    Ok(timestamp.with_timezone(&offset))
}

fn operational_day_from_policy(
    timestamp: DateTime<Utc>,
    policy: OperationalDayPolicy,
) -> Result<NaiveDate, String> {
    if policy.start_minutes > 1439 {
        return Err(format!(
            "operational-day start minute {} is invalid",
            policy.start_minutes
        ));
    }
    let civil = civil_from_policy(timestamp, policy)?;
    let cutoff =
        NaiveTime::from_num_seconds_from_midnight_opt(u32::from(policy.start_minutes) * 60, 0)
            .ok_or_else(|| "operational-day cutoff is invalid".to_string())?;
    let mut day = civil.date_naive();
    if civil.time() < cutoff {
        day -= ChronoDuration::days(1);
    }
    Ok(day)
}

fn boundary_start_utc(
    operational_day: NaiveDate,
    policy: OperationalDayPolicy,
) -> Result<DateTime<Utc>, String> {
    let offset = FixedOffset::east_opt(policy.utc_offset_seconds).ok_or_else(|| {
        format!(
            "operational-day UTC offset {} is invalid",
            policy.utc_offset_seconds
        )
    })?;
    if policy.start_minutes > 1439 {
        return Err(format!(
            "operational-day start minute {} is invalid",
            policy.start_minutes
        ));
    }
    let cutoff =
        NaiveTime::from_num_seconds_from_midnight_opt(u32::from(policy.start_minutes) * 60, 0)
            .ok_or_else(|| "operational-day cutoff is invalid".to_string())?;
    operational_day
        .and_time(cutoff)
        .and_local_timezone(offset)
        .single()
        .map(|value| value.with_timezone(&Utc))
        .ok_or_else(|| "fixed-offset operational-day boundary is not unique".to_string())
}

pub(crate) fn next_operational_day_boundary_after(
    timestamp: DateTime<Utc>,
    config: &DayBoundaryConfig,
) -> Result<(NaiveDate, DateTime<Utc>), String> {
    let operational_day = operational_day_from_utc(timestamp, config)?;
    let next_day = operational_day
        .succ_opt()
        .ok_or_else(|| "operational-day range exceeds chrono's supported dates".to_string())?;
    let boundary = boundary_start_utc(next_day, OperationalDayPolicy::from_config(*config))?;
    if boundary <= timestamp {
        return Err("next operational-day boundary did not advance".to_string());
    }
    Ok((operational_day, boundary))
}

pub(crate) fn allocate_operational_day_slices(
    started_at_utc: DateTime<Utc>,
    recorded_ended_at_utc: DateTime<Utc>,
    elapsed_seconds: usize,
    policy: OperationalDayPolicy,
) -> Result<Vec<OperationalDaySlice>, String> {
    if elapsed_seconds == 0 {
        return Ok(Vec::new());
    }
    let elapsed = i64::try_from(elapsed_seconds)
        .map_err(|_| "session duration exceeds chrono's supported range".to_string())?;
    let ended_at_utc = started_at_utc
        .checked_add_signed(ChronoDuration::seconds(elapsed))
        .ok_or_else(|| "session end exceeds chrono's supported range".to_string())?;
    if recorded_ended_at_utc < started_at_utc {
        return Err("session end precedes its start".to_string());
    }
    if recorded_ended_at_utc < ended_at_utc {
        return Err("session end precedes its recorded elapsed duration".to_string());
    }

    // `elapsed_seconds` is one canonical whole-second duration for the session.  Splitting
    // each wall-clock overlap independently with `num_seconds()` loses a second whenever
    // a sub-second session start crosses an exact operational-day boundary: each side is
    // floored separately even though the unsplit duration is conserved.  Allocate from
    // cumulative whole seconds since the canonical start instead, so every boundary only
    // partitions an already-conserved integer duration.
    let mut cursor = started_at_utc;
    let mut allocated = 0usize;
    let mut slices = Vec::new();
    while cursor < ended_at_utc {
        let operational_day = operational_day_from_policy(cursor, policy)?;
        let next_day = operational_day
            .succ_opt()
            .ok_or_else(|| "operational-day range exceeds chrono's supported dates".to_string())?;
        let next_boundary = boundary_start_utc(next_day, policy)?;
        let slice_end = ended_at_utc.min(next_boundary);
        if slice_end <= cursor {
            return Err("operational-day allocation did not advance".to_string());
        }

        let cumulative = if slice_end == ended_at_utc {
            elapsed_seconds
        } else {
            usize::try_from((slice_end - started_at_utc).num_seconds())
                .map_err(|_| "slice duration exceeds this platform's range".to_string())?
        };
        if cumulative < allocated || cumulative > elapsed_seconds {
            return Err(
                "operational-day allocation produced an invalid cumulative duration".to_string(),
            );
        }
        let seconds = cumulative - allocated;
        if seconds > 0 {
            slices.push(OperationalDaySlice {
                operational_day,
                started_at_utc: cursor,
                ended_at_utc: slice_end,
                elapsed_seconds: seconds,
            });
        }
        allocated = cumulative;
        cursor = slice_end;
    }
    if allocated != elapsed_seconds {
        return Err(format!(
            "operational-day allocation conserved {allocated} of {elapsed_seconds} seconds"
        ));
    }
    Ok(slices)
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
            mode: crate::domain::DayBoundaryMode::FixedHour,
            fixed_hour: 6,
            fixed_minute: 0,
            utc_offset_seconds: offset_seconds,
        }
    }

    #[test]
    fn next_operational_boundary_returns_the_cutoff_ending_current_day() {
        let policy = config(-6 * 60 * 60);
        let before = Utc
            .with_ymd_and_hms(2026, 8, 2, 11, 59, 59)
            .single()
            .unwrap();
        let (ending_day, boundary) = next_operational_day_boundary_after(before, &policy).unwrap();
        assert_eq!(ending_day, NaiveDate::from_ymd_opt(2026, 8, 1).unwrap());
        assert_eq!(
            boundary,
            Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).single().unwrap()
        );

        let at_boundary = boundary;
        let (new_day, next_boundary) =
            next_operational_day_boundary_after(at_boundary, &policy).unwrap();
        assert_eq!(new_day, NaiveDate::from_ymd_opt(2026, 8, 2).unwrap());
        assert_eq!(
            next_boundary,
            Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).single().unwrap()
        );
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
    fn ordinary_subsecond_wall_jitter_uses_monotonic_elapsed() {
        let start = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).single().unwrap();
        let elapsed = Duration::from_secs(60);
        let observed = start + ChronoDuration::seconds(61);
        let interval = reconcile_live_interval(start, observed, elapsed).unwrap();
        assert_eq!(interval.elapsed_seconds, 60);
        assert_eq!(interval.ended_at_utc, start + ChronoDuration::seconds(60));
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

    #[test]
    fn cross_boundary_allocation_conserves_seconds() {
        let policy = OperationalDayPolicy {
            utc_offset_seconds: -21600,
            start_minutes: 360,
        };
        let start = Utc
            .with_ymd_and_hms(2026, 8, 1, 11, 30, 0)
            .single()
            .unwrap();
        let end = Utc
            .with_ymd_and_hms(2026, 8, 1, 12, 30, 0)
            .single()
            .unwrap();
        let slices = allocate_operational_day_slices(start, end, 3600, policy).unwrap();
        assert_eq!(slices.len(), 2);
        assert_eq!(slices[0].operational_day.to_string(), "2026-07-31");
        assert_eq!(slices[0].elapsed_seconds, 1800);
        assert_eq!(slices[1].operational_day.to_string(), "2026-08-01");
        assert_eq!(slices[1].elapsed_seconds, 1800);
    }

    #[test]
    fn fractional_cross_boundary_allocation_conserves_whole_seconds() {
        let policy = OperationalDayPolicy {
            utc_offset_seconds: -21600,
            start_minutes: 360,
        };
        let start = Utc
            .with_ymd_and_hms(2026, 8, 21, 7, 14, 55)
            .single()
            .unwrap()
            + ChronoDuration::nanoseconds(773_810_532);
        let elapsed_seconds = 32_937;
        let end = start + ChronoDuration::seconds(elapsed_seconds as i64);

        let slices = allocate_operational_day_slices(start, end, elapsed_seconds, policy).unwrap();

        assert_eq!(slices.len(), 2);
        assert_eq!(
            slices
                .iter()
                .map(|slice| slice.elapsed_seconds)
                .sum::<usize>(),
            elapsed_seconds
        );
        assert_eq!(slices[0].operational_day.to_string(), "2026-08-20");
        assert_eq!(slices[1].operational_day.to_string(), "2026-08-21");
    }

    #[test]
    fn subsecond_pre_boundary_fragment_does_not_create_zero_second_slice() {
        let policy = OperationalDayPolicy {
            utc_offset_seconds: -21600,
            start_minutes: 360,
        };
        let boundary = Utc
            .with_ymd_and_hms(2026, 8, 21, 12, 0, 0)
            .single()
            .unwrap();
        let start = boundary - ChronoDuration::milliseconds(250);
        let end = start + ChronoDuration::seconds(1);

        let slices = allocate_operational_day_slices(start, end, 1, policy).unwrap();

        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].operational_day.to_string(), "2026-08-21");
        assert_eq!(slices[0].elapsed_seconds, 1);
    }

    #[test]
    fn exact_boundary_does_not_create_zero_slice() {
        let policy = OperationalDayPolicy {
            utc_offset_seconds: -21600,
            start_minutes: 360,
        };
        let end = Utc.with_ymd_and_hms(2026, 8, 1, 12, 0, 0).single().unwrap();
        let start = end - ChronoDuration::minutes(30);
        let slices = allocate_operational_day_slices(start, end, 1800, policy).unwrap();
        assert_eq!(slices.len(), 1);
        assert_eq!(slices[0].operational_day.to_string(), "2026-07-31");
    }
}
