from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text()


def write(path: str, content: str) -> None:
    Path(path).write_text(content)


def replace_once(path: str, old: str, new: str) -> None:
    content = read(path)
    if old not in content:
        raise SystemExit(f"marker missing in {path}: {old[:120]!r}")
    write(path, content.replace(old, new, 1))


def insert_before(path: str, marker: str, insertion: str) -> None:
    content = read(path)
    if insertion.strip() in content:
        return
    if marker not in content:
        raise SystemExit(f"marker missing in {path}: {marker[:120]!r}")
    write(path, content.replace(marker, insertion + marker, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    content = read(path)
    start_index = content.find(start)
    if start_index < 0:
        raise SystemExit(f"start marker missing in {path}: {start[:120]!r}")
    end_index = content.find(end, start_index)
    if end_index < 0:
        raise SystemExit(f"end marker missing in {path}: {end[:120]!r}")
    write(path, content[:start_index] + replacement + content[end_index:])


replace_once(
    "src/app.rs",
    "mod report_state;\nmod terminal_lifecycle;",
    "mod report_state;\nmod recovery_statement;\nmod terminal_lifecycle;",
)

statement_types = r'''#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum RecoveredIntervalClass {
    Exact,
    Reconstructed,
}

impl RecoveredIntervalClass {
    fn label(self) -> &'static str {
        match self {
            Self::Exact => "EXACT",
            Self::Reconstructed => "RECONSTRUCTED",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum PostTargetClass {
    ProvisionalLiveTime,
}

impl PostTargetClass {
    fn label(self) -> &'static str {
        "PROVISIONAL LIVE TIME"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct RecoveryStatement {
    checkpoint_captured_at_utc: DateTime<Utc>,
    checkpoint_simulation_at_utc: DateTime<Utc>,
    recovery_target_utc: DateTime<Utc>,
    reconstructed_duration_nanos: u64,
    recovered_interval_class: RecoveredIntervalClass,
    post_target_class: PostTargetClass,
    active_stable_id: Option<String>,
    active_category_id: u64,
    active_description: String,
    active_session_started_at_utc: DateTime<Utc>,
    cutoff_policy: String,
}

fn recovery_target_for_claim(
    persisted_target_utc: Option<DateTime<Utc>>,
    claim_time_utc: DateTime<Utc>,
) -> DateTime<Utc> {
    persisted_target_utc.unwrap_or(claim_time_utc)
}

fn build_recovery_statement(
    checkpoint: &DetachedRuntimeCheckpoint,
    active_stable_id: Option<String>,
    target_utc: DateTime<Utc>,
) -> Result<RecoveryStatement, String> {
    if checkpoint.simulation_time_utc > checkpoint.detached_at_utc
        || checkpoint.detached_at_utc > target_utc
    {
        return Err("recovery statement timestamps are not monotonic".to_string());
    }
    let started_at_utc = checkpoint
        .active_session_started_at_utc
        .ok_or_else(|| "recovery statement has no active-session start".to_string())?;
    if started_at_utc > target_utc {
        return Err("recovery statement active session starts after its target".to_string());
    }
    let reconstructed = (target_utc - checkpoint.simulation_time_utc)
        .to_std()
        .map_err(|error| format!("invalid recovery statement interval: {error}"))?;
    let reconstructed_duration_nanos = u64::try_from(reconstructed.as_nanos())
        .map_err(|_| "recovery statement interval exceeds the supported range".to_string())?;
    let recovered_interval_class = if reconstructed_duration_nanos == 0 {
        RecoveredIntervalClass::Exact
    } else {
        RecoveredIntervalClass::Reconstructed
    };
    Ok(RecoveryStatement {
        checkpoint_captured_at_utc: checkpoint.detached_at_utc,
        checkpoint_simulation_at_utc: checkpoint.simulation_time_utc,
        recovery_target_utc: target_utc,
        reconstructed_duration_nanos,
        recovered_interval_class,
        post_target_class: PostTargetClass::ProvisionalLiveTime,
        active_stable_id,
        active_category_id: checkpoint.active_category_id,
        active_description: checkpoint.active_description.clone(),
        active_session_started_at_utc: started_at_utc,
        cutoff_policy: "persisted target; no post-target time is counted as recovered"
            .to_string(),
    })
}

'''
insert_before("src/app.rs", "fn transition_operation_id(", statement_types)

replace_once(
    "src/app.rs",
    "    checkpoint_recovery_payload: Option<DetachedRuntimeCheckpoint>,\n    persistence_recovery: Option<PersistenceRecoveryState>,",
    "    checkpoint_recovery_payload: Option<DetachedRuntimeCheckpoint>,\n    recovery_statement: Option<RecoveryStatement>,\n    persistence_recovery: Option<PersistenceRecoveryState>,",
)
replace_once(
    "src/app.rs",
    "            checkpoint_recovery_payload: None,\n            persistence_recovery: None,",
    "            checkpoint_recovery_payload: None,\n            recovery_statement: None,\n            persistence_recovery: None,",
)

restore_prefix = r'''    fn restore_from_detached_checkpoint(&mut self) -> bool {
        let mut checkpoint: DetachedRuntimeCheckpoint =
            if let Some(database_path) = self.sqlite_database_path.clone() {
                match sqlite::load_tui_checkpoint(&database_path) {
                    Ok(Some(claimed)) => {
                        let Some(active_stable_id) = claimed.active_session_stable_id else {
                            let _ = sqlite::quarantine_tui_checkpoint(&database_path);
                            self.record_storage_result::<()>(Err(
                                "SQLite recovery checkpoint has no active stable identity"
                                    .to_string(),
                            ));
                            return false;
                        };
                        self.session.active_session_stable_id = Some(active_stable_id);
                        claimed.payload
                    }
                    Ok(None) => return false,
                    Err(error) => {
                        self.record_storage_result::<()>(Err(error));
                        return false;
                    }
                }
            } else {
                let path = storage::get_detached_runtime_path();
                if !storage::file_exists(&path) {
                    return false;
                }
                match storage::read_json::<DetachedRuntimeCheckpoint>(&path) {
                    Ok(value) => value,
                    Err(error) => {
                        self.record_storage_result::<()>(Err(error));
                        return false;
                    }
                }
            };

'''
replace_between(
    "src/app.rs",
    "    fn restore_from_detached_checkpoint(&mut self) -> bool {",
    "        if checkpoint.clear_all.is_some()",
    restore_prefix,
)

old_target = '''        let now_utc = Utc::now();
        let target_utc = if was_committed || checkpoint.legacy_recovery_committed {
            now_utc
        } else {
            checkpoint.recovery_target_utc.unwrap_or(now_utc)
        };
'''
new_target = '''        let now_utc = Utc::now();
        let target_utc = recovery_target_for_claim(checkpoint.recovery_target_utc, now_utc);
'''
replace_once("src/app.rs", old_target, new_target)

statement_build = r'''        let recovery_statement = match build_recovery_statement(
            &checkpoint,
            self.session.active_session_stable_id.clone(),
            target_utc,
        ) {
            Ok(statement) => statement,
            Err(error) => {
                self.record_storage_result::<()>(Err(error));
                return false;
            }
        };

'''
insert_before(
    "src/app.rs",
    "        let valid_category_ids = self\n            .time_tracker",
    statement_build,
)
replace_once(
    "src/app.rs",
    "        self.checkpoint_recovery_payload = Some(checkpoint);\n        true",
    "        self.checkpoint_recovery_payload = Some(checkpoint);\n        self.recovery_statement = Some(recovery_statement);\n        true",
)

statement_tests = r'''#[cfg(test)]
mod recovery_statement_tests {
    use chrono::{TimeZone, Utc};

    use super::{
        DetachedRuntimeCheckpoint, PostTargetClass, RecoveredIntervalClass,
        build_recovery_statement, recovery_target_for_claim,
    };
    use crate::sand::SandState;

    fn timestamp(second: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 3, 18, 0, second)
            .unwrap()
    }

    fn checkpoint(simulation_second: u32, capture_second: u32) -> DetachedRuntimeCheckpoint {
        DetachedRuntimeCheckpoint {
            schema_version: DetachedRuntimeCheckpoint::VERSION,
            detached_at_utc: timestamp(capture_second),
            simulation_time_utc: timestamp(simulation_second),
            spawn_accumulator_nanos: 0,
            physics_accumulator_nanos: 0,
            active_category_id: 1,
            active_description: "Focused".to_string(),
            active_session_started_at_utc: Some(timestamp(0)),
            sand_state: SandState {
                version: SandState::VERSION,
                grid_width: 2,
                grid_height: 2,
                grains: Vec::new(),
                frame_count: 0,
                sweep_left_to_right: true,
                rng_state: 1,
                pending_grains: Vec::new(),
                pending_runs: Vec::new(),
            },
            pending_mutations: Vec::new(),
            recovery_target_utc: None,
            legacy_recovery_committed: false,
            legacy_transition: None,
            legacy_finish: None,
            clear_all: None,
        }
    }

    #[test]
    fn exact_and_reconstructed_statements_are_distinct() {
        let exact_checkpoint = checkpoint(2, 2);
        let exact = build_recovery_statement(
            &exact_checkpoint,
            Some("stable".to_string()),
            timestamp(2),
        )
        .unwrap();
        assert_eq!(exact.reconstructed_duration_nanos, 0);
        assert_eq!(exact.recovered_interval_class, RecoveredIntervalClass::Exact);
        assert_eq!(exact.post_target_class, PostTargetClass::ProvisionalLiveTime);

        let reconstructed_checkpoint = checkpoint(2, 3);
        let reconstructed = build_recovery_statement(
            &reconstructed_checkpoint,
            Some("stable".to_string()),
            timestamp(7),
        )
        .unwrap();
        assert_eq!(reconstructed.reconstructed_duration_nanos, 5_000_000_000);
        assert_eq!(
            reconstructed.recovered_interval_class,
            RecoveredIntervalClass::Reconstructed
        );
    }

    #[test]
    fn persisted_target_is_reused_after_wall_time_advances() {
        let persisted = timestamp(5);
        assert_eq!(
            recovery_target_for_claim(Some(persisted), timestamp(30)),
            persisted
        );
        assert_eq!(recovery_target_for_claim(None, timestamp(30)), timestamp(30));
    }

    #[test]
    fn non_monotonic_statement_fails_closed() {
        let invalid = checkpoint(4, 3);
        assert!(
            build_recovery_statement(&invalid, None, timestamp(5))
                .unwrap_err()
                .contains("not monotonic")
        );
    }
}

'''
insert_before("src/app.rs", "#[cfg(test)]\nmod transition_edge_tests {", statement_tests)

recovery_module = r'''use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use super::{App, RecoveryStatement};

impl App {
    pub(super) fn handle_recovery_statement_key(&mut self, key: KeyEvent) -> bool {
        if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
            self.recovery_statement = None;
            self.render_needed = true;
        }
        false
    }

    pub(super) fn render_recovery_statement(&self, frame: &mut Frame, size: Rect) {
        let Some(statement) = self.recovery_statement.as_ref() else {
            return;
        };
        let width = size.width.saturating_sub(4).clamp(48, 104);
        let height = size.height.saturating_sub(4).clamp(18, 26);
        let area = centered_rect(width, height, size);
        frame.render_widget(Clear, area);

        let active_identity = statement
            .active_stable_id
            .as_deref()
            .unwrap_or("legacy-file active generation");
        let description = if statement.active_description.is_empty() {
            "(empty)"
        } else {
            statement.active_description.as_str()
        };
        let lines = vec![
            Line::from(Span::styled(
                "RECOVERY EVIDENCE",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            labelled("Active identity", active_identity),
            labelled("Category", &statement.active_category_id.to_string()),
            labelled("Description", description),
            labelled(
                "Active started",
                &format_timestamp(statement.active_session_started_at_utc),
            ),
            Line::from(""),
            labelled(
                "Checkpoint captured",
                &format_timestamp(statement.checkpoint_captured_at_utc),
            ),
            labelled(
                "Durable sediment through",
                &format_timestamp(statement.checkpoint_simulation_at_utc),
            ),
            labelled(
                "Recovery target",
                &format_timestamp(statement.recovery_target_utc),
            ),
            labelled(
                "Reconstructed duration",
                &format_duration(statement.reconstructed_duration_nanos),
            ),
            labelled(
                "Recovered interval",
                statement.recovered_interval_class.label(),
            ),
            labelled("After target", statement.post_target_class.label()),
            Line::from(""),
            Line::from(Span::styled(
                statement.cutoff_policy.clone(),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(
                "Retry reuses this persisted target. Later live time is not recovered history.",
            ),
            Line::from(""),
            Line::from(Span::styled(
                "[Enter/Esc] acknowledge",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ];
        let block = Block::default()
            .title(" Checkpoint recovery ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Yellow));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

fn labelled<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

fn format_timestamp(value: chrono::DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn format_duration(nanos: u64) -> String {
    let duration = Duration::from_nanos(nanos);
    let seconds = duration.as_secs();
    let millis = duration.subsec_millis();
    format!("{seconds}.{millis:03}s")
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(area.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn acknowledges(code: KeyCode) -> bool {
        matches!(code, KeyCode::Enter | KeyCode::Esc)
    }

    #[test]
    fn only_explicit_acknowledgment_keys_dismiss_statement() {
        assert!(acknowledges(KeyCode::Enter));
        assert!(acknowledges(KeyCode::Esc));
        assert!(!acknowledges(KeyCode::Char('q')));
        let _ = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    }
}
'''
write("src/app/recovery_statement.rs", recovery_module)

replace_once(
    "src/app/event_handlers.rs",
    '''        if self.has_persistence_recovery() {
            if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
                return self.request_persistence_recovery_quit();
            }
            return self.handle_persistence_recovery_key(key);
        }

        if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
''',
    '''        if self.has_persistence_recovery() {
            if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
                return self.request_persistence_recovery_quit();
            }
            return self.handle_persistence_recovery_key(key);
        }

        if self.recovery_statement.is_some() {
            if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
                return true;
            }
            return self.handle_recovery_statement_key(key);
        }

        if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
''',
)
replace_once(
    "src/app/render_views.rs",
    '''        if self.has_persistence_recovery() {
            self.render_persistence_recovery(f, size);
        }
''',
    '''        if self.recovery_statement.is_some() {
            self.render_recovery_statement(f, size);
        }

        if self.has_persistence_recovery() {
            self.render_persistence_recovery(f, size);
        }
''',
)

replace_once(
    "src/app/persistence_recovery.rs",
    "use super::{App, QueuedMutation, QueuedMutationEventRecord, QueuedMutationRecord};",
    "use super::{\n    App, QueuedMutation, QueuedMutationEventRecord, QueuedMutationRecord, RecoveryStatement,\n};",
)
replace_once(
    "src/app/persistence_recovery.rs",
    "            schema_version: 2,",
    "            schema_version: 3,",
)
replace_once(
    "src/app/persistence_recovery.rs",
    "            checkpoint_recovery_active: self.checkpoint_recovery_active,\n        };",
    "            checkpoint_recovery_active: self.checkpoint_recovery_active,\n            recovery_statement: self.recovery_statement.clone(),\n        };",
)
replace_once(
    "src/app/persistence_recovery.rs",
    "    checkpoint_recovery_active: bool,\n}",
    "    checkpoint_recovery_active: bool,\n    recovery_statement: Option<RecoveryStatement>,\n}",
)

process_test = r'''#[test]
fn failed_recovery_commit_reuses_and_displays_persisted_cutoff() {
    let profile = TestProfile::new("visible-recovery-cutoff");
    profile.migrate();
    assert!(profile.activate().status.success());

    let detached = profile.run_tui_with_input(b"d", None);
    assert!(
        detached.status.success(),
        "detach failed: stdout={} stderr={}",
        stdout(&detached),
        stderr(&detached)
    );

    let failed = profile.run_tui_with_input(b"", Some("checkpoint-recovery:commit:cutoff"));
    assert!(!failed.status.success(), "injected recovery commit unexpectedly succeeded");

    let connection = Connection::open(profile.database_path()).expect("database should open");
    let (status, payload_json): (String, String) = connection
        .query_row(
            "SELECT status, payload_json FROM runtime_checkpoint WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("recovering checkpoint should remain");
    assert_eq!(status, "recovering");
    let payload: Value = serde_json::from_str(&payload_json).expect("payload should be JSON");
    let persisted_target = payload["recovery_target_utc"]
        .as_str()
        .expect("recovery target should be persisted")
        .to_string();
    drop(connection);

    thread::sleep(Duration::from_millis(1_200));
    let recovered = profile.run_tui_with_input(b"\rq", None);
    assert!(
        recovered.status.success(),
        "recovery retry failed: stdout={} stderr={}",
        stdout(&recovered),
        stderr(&recovered)
    );
    let visible_target = chrono::DateTime::parse_from_rfc3339(&persisted_target)
        .unwrap()
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let output = stdout(&recovered);
    assert!(output.contains("RECOVERY EVIDENCE"));
    assert!(output.contains("Recovery target"));
    assert!(output.contains(&visible_target));
    assert!(output.contains("PROVISIONAL LIVE TIME"));
}

'''
insert_before(
    "tests/sqlite_cli_authority.rs",
    "fn recovery_bundle(profile: &TestProfile) -> Value {",
    process_test,
)
