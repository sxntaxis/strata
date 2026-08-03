from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"{label} anchor missing")
    return text.replace(old, new, 1)

# ---------------------------------------------------------------------------
# Receipt payload: bind canonical elapsed and all affected days.
# ---------------------------------------------------------------------------
legacy_path = Path("src/legacy_transition.rs")
legacy = legacy_path.read_text()
legacy = replace_once(
    legacy,
    "    pub idle_reset: bool,\n    pub affected_operational_days: Vec<String>,",
    "    pub idle_reset: bool,\n    pub previous_elapsed_seconds: usize,\n    pub affected_operational_days: Vec<String>,",
    "clear-all elapsed field",
)
legacy = replace_once(
    legacy,
    '''        if self.applied_at_utc < self.previous_active.started_at_utc {
            return Err(format!(
                "clear-all receipt {} predates its active generation",
                self.operation_id
            ));
        }
        if self.idle_reset {''',
    '''        if self.applied_at_utc < self.previous_active.started_at_utc {
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
        if self.idle_reset {''',
    "clear-all elapsed validation",
)
legacy = replace_once(
    legacy,
    '''            idle_reset,
            affected_operational_days: vec!["2026-08-01".to_string(), "2026-08-02".to_string()],''',
    '''            idle_reset,
            previous_elapsed_seconds: 90_000,
            affected_operational_days: vec!["2026-08-01".to_string(), "2026-08-02".to_string()],''',
    "clear-all receipt fixture elapsed",
)
legacy = replace_once(
    legacy,
    '''        let mut duplicate = clear_all_receipt(true);
        duplicate.affected_operational_days =
            vec!["2026-08-02".to_string(), "2026-08-02".to_string()];
        assert!(
            duplicate
                .validate_boundaries()
                .unwrap_err()
                .contains("unique and sorted")
        );
    }
''',
    '''        let mut duplicate = clear_all_receipt(true);
        duplicate.affected_operational_days =
            vec!["2026-08-02".to_string(), "2026-08-02".to_string()];
        assert!(
            duplicate
                .validate_boundaries()
                .unwrap_err()
                .contains("unique and sorted")
        );

        let mut divergent = clear_all_receipt(false);
        divergent.previous_elapsed_seconds = 1;
        assert!(
            divergent
                .validate_boundaries()
                .unwrap_err()
                .contains("diverges")
        );
    }
''',
    "clear-all divergence test",
)
legacy_path.write_text(legacy)
