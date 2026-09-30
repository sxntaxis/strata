use std::{collections::HashSet, time::Duration};

use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::{
    domain::DRIFT_CATEGORY_ID,
    sand::{RecoveryTiming, recover_detached_sediment},
    sqlite,
};

use super::{App, RecoveryAction, SystemDialogSeverity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecoveryGapChoice {
    ActiveLayer,
    IdleWithSediment,
    IdleWithoutSediment,
}

impl App {
    pub(super) fn handle_recovery_gap_key(&mut self, key: KeyEvent) -> bool {
        if self.recovery_gap.is_none() {
            return false;
        }
        let previous_is_idle = self
            .recovery_gap
            .as_ref()
            .is_some_and(|gap| gap.active_category_id == DRIFT_CATEGORY_ID);
        let action_count = if previous_is_idle { 2 } else { 3 };
        match key.code {
            KeyCode::Up => self.move_system_dialog_selection(-1, action_count),
            KeyCode::Down => self.move_system_dialog_selection(1, action_count),
            KeyCode::Enter => {
                let choice = if previous_is_idle {
                    match self.system_dialog_selected_index.min(1) {
                        0 => RecoveryGapChoice::ActiveLayer,
                        1 => RecoveryGapChoice::IdleWithoutSediment,
                        _ => unreachable!(),
                    }
                } else {
                    match self.system_dialog_selected_index.min(2) {
                        0 => RecoveryGapChoice::ActiveLayer,
                        1 => RecoveryGapChoice::IdleWithSediment,
                        2 => RecoveryGapChoice::IdleWithoutSediment,
                        _ => unreachable!(),
                    }
                };
                self.apply_recovery_gap_choice(choice);
            }
            // This warning is a required classification boundary. Esc cannot
            // silently choose a history for the owner.
            KeyCode::Esc => {}
            _ => {}
        }
        false
    }

    fn apply_recovery_gap_choice(&mut self, choice: RecoveryGapChoice) {
        let Some(gap) = self.recovery_gap.clone() else {
            return;
        };
        let elapsed = match (gap.target_utc - gap.checkpoint.simulation_time_utc).to_std() {
            Ok(value) => value,
            Err(error) => {
                self.record_storage_result_for::<()>(
                    super::PersistenceOperation::CheckpointRecovery,
                    RecoveryAction::CommitCheckpointRecovery,
                    Err(format!("invalid recovery gap interval: {error}")),
                );
                return;
            }
        };

        let mut valid_category_ids = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .map(|category| category.id)
            .collect::<HashSet<_>>();
        valid_category_ids.insert(DRIFT_CATEGORY_ID);

        let sediment_category = match choice {
            RecoveryGapChoice::ActiveLayer => Some(gap.active_category_id),
            RecoveryGapChoice::IdleWithSediment => Some(DRIFT_CATEGORY_ID),
            RecoveryGapChoice::IdleWithoutSediment => None,
        };

        let (state, spawn_remainder, physics_remainder) = if let Some(category_id) = sediment_category
        {
            match recover_detached_sediment(
                &gap.checkpoint.sand_state,
                &valid_category_ids,
                category_id,
                RecoveryTiming {
                    elapsed,
                    spawn_accumulator: Duration::from_nanos(
                        gap.checkpoint.spawn_accumulator_nanos,
                    ),
                    physics_accumulator: Duration::from_nanos(
                        gap.checkpoint.physics_accumulator_nanos,
                    ),
                    spawn_period: Duration::from_millis(crate::constants::TIME_SETTINGS.tick_ms),
                    physics_period: Duration::from_millis(
                        crate::constants::TIME_SETTINGS.physics_ms,
                    ),
                },
            ) {
                Ok(recovered) => (
                    recovered.state,
                    recovered.spawn_remainder,
                    recovered.physics_remainder,
                ),
                Err(error) => {
                    self.record_storage_result_for::<()>(
                        super::PersistenceOperation::CheckpointRecovery,
                        RecoveryAction::CommitCheckpointRecovery,
                        Err(error),
                    );
                    return;
                }
            }
        } else {
            (
                gap.checkpoint.sand_state.clone(),
                Duration::from_nanos(gap.checkpoint.spawn_accumulator_nanos),
                Duration::from_nanos(gap.checkpoint.physics_accumulator_nanos),
            )
        };

        if let Err(error) = self.sand_engine.restore_state(&state, &valid_category_ids) {
            self.record_storage_result_for::<()>(
                super::PersistenceOperation::CheckpointRecovery,
                RecoveryAction::CommitCheckpointRecovery,
                Err(error),
            );
            return;
        }
        self.simulation.simulation_time_utc = gap.target_utc;
        self.simulation.spawn_accumulator = spawn_remainder;
        self.simulation.physics_accumulator = physics_remainder;
        self.simulation.catchup_cadence_accumulator = Duration::ZERO;
        self.simulation.catchup_visual_engine = None;
        self.simulation.catchup_progress_anchor = None;
        self.simulation.catchup_was_active = false;

        let preserves_active_identity = matches!(choice, RecoveryGapChoice::ActiveLayer)
            || (gap.active_category_id == DRIFT_CATEGORY_ID
                && matches!(choice, RecoveryGapChoice::IdleWithoutSediment));

        match choice {
            RecoveryGapChoice::ActiveLayer
            | RecoveryGapChoice::IdleWithoutSediment if preserves_active_identity => {
                if let Err(error) = self.begin_active_session_at(gap.active_started_at_utc, true) {
                    self.record_storage_result_for::<()>(
                        super::PersistenceOperation::CheckpointRecovery,
                        RecoveryAction::CommitCheckpointRecovery,
                        Err(error),
                    );
                    return;
                }
                self.recovery_gap = None;
                self.recovery_statement = None;
                self.commit_checkpoint_recovery_if_ready();
            }
            RecoveryGapChoice::IdleWithSediment | RecoveryGapChoice::IdleWithoutSediment => {
                let Some(database_path) = self.sqlite_database_path.clone() else {
                    self.record_storage_result_for::<()>(
                        super::PersistenceOperation::CheckpointRecovery,
                        RecoveryAction::CommitCheckpointRecovery,
                        Err("SQLite authority is unavailable".to_string()),
                    );
                    return;
                };
                let next_stable_id = sqlite::initial_tui_active_stable_id(gap.target_utc);
                let result = sqlite::commit_tui_recovery_gap_as_idle(
                    &database_path,
                    sqlite::TuiRecoveryGapIdleRequest {
                        expected_active_stable_id: &gap.expected_stable_id,
                        previous_category_id: gap.active_category_id,
                        active_started_at_utc: gap.active_started_at_utc,
                        durable_until_utc: gap.checkpoint.simulation_time_utc,
                        target_utc: gap.target_utc,
                        next_active_stable_id: &next_stable_id,
                        state: &state,
                    },
                );
                if self
                    .record_storage_result_for(
                        super::PersistenceOperation::CheckpointRecovery,
                        RecoveryAction::CommitCheckpointRecovery,
                        result,
                    )
                    .is_none()
                {
                    return;
                }

                self.recovery_gap = None;
                self.recovery_statement = None;
                self.checkpoint_recovery_active = false;
                self.checkpoint_recovery_payload = None;
                if let Err(error) = self.try_reload_authority() {
                    self.record_storage_result_for::<()>(
                        super::PersistenceOperation::StateReload,
                        RecoveryAction::ReloadAuthority,
                        Err(error),
                    );
                    return;
                }
                self.reconcile_all_daily_contributions();
            }
        }
        self.render_needed = true;
    }

    pub(super) fn render_recovery_gap(&self, frame: &mut Frame, size: Rect) {
        let Some(gap) = self.recovery_gap.as_ref() else {
            return;
        };
        let last_saved = gap
            .checkpoint
            .simulation_time_utc
            .with_timezone(&Local)
            .format("%H:%M")
            .to_string();
        let seconds = (gap.target_utc - gap.checkpoint.simulation_time_utc)
            .num_seconds()
            .max(0);
        let duration = self.format_time(usize::try_from(seconds).unwrap_or(usize::MAX));
        let values = format!("{last_saved:<5}{duration:^9}{:>5}", "now");
        let layer = self.report_layer_display_name(gap.active_category_id);
        let layer_color = self.category_color_for_id(gap.active_category_id);

        let actions = if gap.active_category_id == DRIFT_CATEGORY_ID {
            vec![
                Line::from("Reconstruct as Idle"),
                Line::from("Keep sediment unchanged"),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::raw("Reconstruct as "),
                    Span::styled(layer, Style::default().fg(layer_color)),
                ]),
                Line::from("Reconstruct as Idle"),
                Line::from("Keep sediment unchanged"),
            ]
        };
        self.render_system_dialog(
            frame,
            size,
            SystemDialogSeverity::Warning,
            Line::from(Span::styled(
                "Recovery gap",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            vec![
                Line::from(Span::styled(
                    "●────────?────────▶",
                    Style::default().fg(self.theme_warning()),
                )),
                Line::from(values),
                Line::default(),
                Line::from("Choose how to reconstruct the missing time."),
            ],
            actions,
            self.system_dialog_selected_index,
            42,
        );
    }
}
