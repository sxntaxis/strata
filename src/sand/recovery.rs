use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PeriodicAdvance {
    pub due_events: usize,
    pub remainder: Duration,
}

pub(crate) fn advance_periodic(
    accumulator: Duration,
    elapsed: Duration,
    period: Duration,
) -> Result<PeriodicAdvance, String> {
    let period_nanos = period.as_nanos();
    if period_nanos == 0 {
        return Err("periodic recovery interval must be non-zero".to_string());
    }

    let total_nanos = accumulator
        .as_nanos()
        .checked_add(elapsed.as_nanos())
        .ok_or_else(|| "periodic recovery duration overflow".to_string())?;
    let due_events = usize::try_from(total_nanos / period_nanos)
        .map_err(|_| "periodic recovery event count exceeds the supported range".to_string())?;
    let remainder_nanos = total_nanos % period_nanos;
    let remainder_seconds = u64::try_from(remainder_nanos / 1_000_000_000)
        .map_err(|_| "periodic recovery remainder exceeds Duration".to_string())?;
    let remainder_subsec = u32::try_from(remainder_nanos % 1_000_000_000)
        .map_err(|_| "periodic recovery remainder is invalid".to_string())?;

    Ok(PeriodicAdvance {
        due_events,
        remainder: Duration::new(remainder_seconds, remainder_subsec),
    })
}

#[cfg(test)]
mod tests {
    use super::{PeriodicAdvance, advance_periodic};
    use std::time::Duration;

    #[test]
    fn long_gap_is_counted_without_iterative_replay() {
        assert_eq!(
            advance_periodic(
                Duration::ZERO,
                Duration::from_secs(1_000_000_000),
                Duration::from_secs(1),
            )
            .unwrap(),
            PeriodicAdvance {
                due_events: 1_000_000_000,
                remainder: Duration::ZERO,
            }
        );
    }

    #[test]
    fn accumulator_and_remainder_are_exact() {
        assert_eq!(
            advance_periodic(
                Duration::from_millis(750),
                Duration::from_millis(2_500),
                Duration::from_secs(1),
            )
            .unwrap(),
            PeriodicAdvance {
                due_events: 3,
                remainder: Duration::from_millis(250),
            }
        );
    }

    #[test]
    fn zero_period_is_rejected() {
        assert!(advance_periodic(Duration::ZERO, Duration::from_secs(1), Duration::ZERO).is_err());
    }
}
