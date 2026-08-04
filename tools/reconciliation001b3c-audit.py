from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    content = file.read_text()
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:120]!r}")
    file.write_text(content.replace(old, new, 1))


replace_once(
    "src/app.rs",
    '''    if started_at_utc > target_utc {
        return Err("recovery statement active session starts after its target".to_string());
    }
''',
    '''    if started_at_utc > checkpoint.simulation_time_utc {
        return Err(
            "recovery statement active session starts after durable simulation time".to_string(),
        );
    }
''',
)
replace_once(
    "src/app.rs",
    '''    #[test]
    fn non_monotonic_statement_fails_closed() {
        let invalid = checkpoint(4, 3);
        assert!(
            build_recovery_statement(&invalid, None, timestamp(5))
                .unwrap_err()
                .contains("not monotonic")
        );
    }
''',
    '''    #[test]
    fn non_monotonic_statement_fails_closed() {
        let invalid = checkpoint(4, 3);
        assert!(
            build_recovery_statement(&invalid, None, timestamp(5))
                .unwrap_err()
                .contains("not monotonic")
        );

        let mut invalid_start = checkpoint(2, 3);
        invalid_start.active_session_started_at_utc = Some(timestamp(4));
        assert!(
            build_recovery_statement(&invalid_start, None, timestamp(5))
                .unwrap_err()
                .contains("starts after durable simulation time")
        );
    }
''',
)
marker = '''    #[test]
    fn persistence_failure_classes_are_actionable() {
'''
test = r'''    #[test]
    fn emergency_export_schema_three_carries_exact_recovery_statement() {
        let captured = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 8, 3, 18, 0, 2).unwrap();
        let target = chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 8, 3, 18, 0, 7).unwrap();
        let statement = RecoveryStatement {
            checkpoint_captured_at_utc: captured,
            checkpoint_simulation_at_utc: captured,
            recovery_target_utc: target,
            reconstructed_duration_nanos: 5_000_000_000,
            recovered_interval_class: super::super::RecoveredIntervalClass::Reconstructed,
            post_target_class: super::super::PostTargetClass::ProvisionalLiveTime,
            active_stable_id: Some("stable-1".to_string()),
            active_category_id: 1,
            active_description: "Focused".to_string(),
            active_session_started_at_utc: captured,
            cutoff_policy: "persisted target; no post-target time is counted as recovered"
                .to_string(),
        };
        let bundle = EmergencyRecoveryBundle {
            schema_version: 3,
            created_at_utc: target.to_rfc3339(),
            failure: EmergencyFailure {
                operation: "checkpoint recovery".to_string(),
                class: PersistenceFailureClass::Commit,
                detail: "injected".to_string(),
                authority_path: None,
                occurred_at_utc: target.to_rfc3339(),
            },
            categories: Vec::new(),
            category_tags: storage::CategoryTagsState::default(),
            sessions: Vec::new(),
            active_session: Some(EmergencyActiveSession {
                stable_id: Some("stable-1".to_string()),
                category_id: 1,
                description: "Focused".to_string(),
                started_at_utc: captured.to_rfc3339(),
            }),
            sand_state: crate::sand::SandState {
                version: crate::sand::SandState::VERSION,
                grid_width: 2,
                grid_height: 2,
                grains: Vec::new(),
                frame_count: 0,
                sweep_left_to_right: true,
                rng_state: 1,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            simulation_time_utc: captured.to_rfc3339(),
            pending_mutations: Vec::new(),
            checkpoint_recovery_active: true,
            recovery_statement: Some(statement),
        };
        let value = serde_json::to_value(bundle).unwrap();
        assert_eq!(value["schema_version"], 3);
        let exported_target = chrono::DateTime::parse_from_rfc3339(
            value["recovery_statement"]["recovery_target_utc"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
        .with_timezone(&Utc);
        assert_eq!(exported_target, target);
        assert_eq!(
            value["recovery_statement"]["recovered_interval_class"],
            "reconstructed"
        );
        assert_eq!(
            value["recovery_statement"]["post_target_class"],
            "provisional-live-time"
        );
    }

'''
path = Path("src/app/persistence_recovery.rs")
content = path.read_text()
if marker not in content:
    raise SystemExit("persistence recovery test marker missing")
path.write_text(content.replace(marker, test + marker, 1))
