use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    domain::{CategoryId, DRIFT_CATEGORY_ID},
    legacy_category_lifecycle::{LegacyCategoryLifecyclePaths, LegacyCategoryLifecycleReview},
    sqlite,
};

use super::{App, PersistenceOperation, RecoveryAction, UiMode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CategoryLifecycleTarget {
    pub category_id: Option<CategoryId>,
    pub label: String,
    pub archived: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct CategoryLifecycleCounts {
    pub completed_sessions: u64,
    pub active_sessions: u64,
    pub tags: u64,
    pub sand_placed: u64,
    pub sand_pending: u64,
    pub history_placed: u64,
    pub history_pending: u64,
    pub checkpoint_references: u64,
}

impl CategoryLifecycleCounts {
    fn total(&self) -> u64 {
        [
            self.completed_sessions,
            self.active_sessions,
            self.tags,
            self.sand_placed,
            self.sand_pending,
            self.history_placed,
            self.history_pending,
            self.checkpoint_references,
        ]
        .into_iter()
        .fold(0u64, u64::saturating_add)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CategoryLifecycleReview {
    pub source_id: CategoryId,
    pub source_name: String,
    pub target_id: Option<CategoryId>,
    pub target_name: Option<String>,
    pub counts: CategoryLifecycleCounts,
    pub checkpoint_custody: String,
    pub revision: String,
    pub confirmation_phrase: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum CategoryLifecycleStage {
    SelectTarget,
    Confirm(Box<CategoryLifecycleReview>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CategoryLifecycleOverlay {
    pub source_id: CategoryId,
    pub source_name: String,
    pub targets: Vec<CategoryLifecycleTarget>,
    pub selected_target: usize,
    pub stage: CategoryLifecycleStage,
    pub confirmation_input: String,
    pub error: Option<String>,
}

impl App {
    fn show_category_lifecycle_error(
        &mut self,
        source_id: CategoryId,
        source_name: String,
        error: String,
    ) {
        self.category_lifecycle_overlay = Some(CategoryLifecycleOverlay {
            source_id,
            source_name,
            targets: Vec::new(),
            selected_target: 0,
            stage: CategoryLifecycleStage::SelectTarget,
            confirmation_input: String::new(),
            error: Some(error),
        });
        self.render_needed = true;
    }

    pub(super) fn open_category_lifecycle_for_selected(&mut self) {
        if self.selected_index >= self.time_tracker.category_count() || self.selected_index == 0 {
            self.show_category_lifecycle_error(
                DRIFT_CATEGORY_ID,
                "Unavailable".to_string(),
                "Select a non-idle layer. Archive remains the ordinary retirement action."
                    .to_string(),
            );
            return;
        }
        let Some(source) = self.time_tracker.category_by_index(self.selected_index) else {
            self.show_category_lifecycle_error(
                DRIFT_CATEGORY_ID,
                "Unavailable".to_string(),
                "Selected layer is unavailable.".to_string(),
            );
            return;
        };
        self.open_category_lifecycle(source.id);
    }

    pub(super) fn open_category_lifecycle_for_active(&mut self) {
        let source_id = self.time_tracker.active_category_id();
        if source_id == DRIFT_CATEGORY_ID {
            self.open_modal();
            self.show_category_lifecycle_error(
                DRIFT_CATEGORY_ID,
                "idle".to_string(),
                "Idle cannot be merged or permanently deleted. Select another layer.".to_string(),
            );
            return;
        }
        if let Some(index) = self.time_tracker.active_category_index() {
            self.open_modal();
            self.selected_index = index;
        }
        self.open_category_lifecycle(source_id);
    }

    fn open_category_lifecycle(&mut self, source_id: CategoryId) {
        let source = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .chain(self.archived_categories.iter().cloned())
            .find(|category| category.id == source_id);
        let Some(source) = source else {
            self.show_category_lifecycle_error(
                source_id,
                "Unavailable".to_string(),
                format!("Layer {} is unavailable for lifecycle review.", source_id.0),
            );
            return;
        };
        if source.id == DRIFT_CATEGORY_ID {
            self.show_category_lifecycle_error(
                source_id,
                source.name.clone(),
                "Idle cannot be merged or permanently deleted.".to_string(),
            );
            return;
        }

        let mut targets = self
            .time_tracker
            .categories_for_storage()
            .into_iter()
            .filter(|category| category.id != source_id && category.id != DRIFT_CATEGORY_ID)
            .map(|category| CategoryLifecycleTarget {
                category_id: Some(category.id),
                label: format!("{} — {}", category.id.0, category.name),
                archived: false,
            })
            .collect::<Vec<_>>();
        targets.extend(
            self.archived_categories
                .iter()
                .filter(|category| category.id != source_id)
                .map(|category| CategoryLifecycleTarget {
                    category_id: Some(category.id),
                    label: format!("{} — {} [archived]", category.id.0, category.name),
                    archived: true,
                }),
        );
        targets.push(CategoryLifecycleTarget {
            category_id: None,
            label: "Permanent deletion — only when every reference count is zero".to_string(),
            archived: false,
        });

        self.category_lifecycle_overlay = Some(CategoryLifecycleOverlay {
            source_id,
            source_name: source.name,
            targets,
            selected_target: 0,
            stage: CategoryLifecycleStage::SelectTarget,
            confirmation_input: String::new(),
            error: None,
        });
        self.render_needed = true;
    }

    pub(super) fn handle_category_lifecycle_key(&mut self, key: KeyEvent) -> bool {
        let Some(mut overlay) = self.category_lifecycle_overlay.take() else {
            return false;
        };
        match &mut overlay.stage {
            CategoryLifecycleStage::SelectTarget => match key.code {
                KeyCode::Esc => {
                    self.render_needed = true;
                    return false;
                }
                KeyCode::Up | KeyCode::Left => {
                    if !overlay.targets.is_empty() {
                        overlay.selected_target = if overlay.selected_target == 0 {
                            overlay.targets.len() - 1
                        } else {
                            overlay.selected_target - 1
                        };
                    }
                    overlay.error = None;
                }
                KeyCode::Down | KeyCode::Right => {
                    if !overlay.targets.is_empty() {
                        overlay.selected_target =
                            (overlay.selected_target + 1) % overlay.targets.len();
                    }
                    overlay.error = None;
                }
                KeyCode::Enter => {
                    let target_id = overlay
                        .targets
                        .get(overlay.selected_target)
                        .and_then(|target| target.category_id);
                    match self.build_category_lifecycle_review(overlay.source_id, target_id) {
                        Ok(review) => {
                            overlay.stage = CategoryLifecycleStage::Confirm(Box::new(review));
                            overlay.confirmation_input.clear();
                            overlay.error = None;
                        }
                        Err(error) => overlay.error = Some(error),
                    }
                }
                _ => {}
            },
            CategoryLifecycleStage::Confirm(review) => match key.code {
                KeyCode::Esc => {
                    self.render_needed = true;
                    return false;
                }
                KeyCode::Backspace => {
                    overlay.confirmation_input.pop();
                    overlay.error = None;
                }
                KeyCode::Char(character)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    overlay.confirmation_input.push(character);
                    overlay.error = None;
                }
                KeyCode::Enter => {
                    if overlay.confirmation_input != review.confirmation_phrase {
                        overlay.error = Some(
                            "Confirmation does not exactly match the displayed phrase.".to_string(),
                        );
                    } else {
                        let review = review.as_ref().clone();
                        self.category_lifecycle_overlay = Some(overlay);
                        self.apply_category_lifecycle(review);
                        self.render_needed = true;
                        return false;
                    }
                }
                _ => {}
            },
        }
        self.category_lifecycle_overlay = Some(overlay);
        self.render_needed = true;
        false
    }

    fn build_category_lifecycle_review(
        &mut self,
        source_id: CategoryId,
        target_id: Option<CategoryId>,
    ) -> Result<CategoryLifecycleReview, String> {
        if let Some(database_path) = self.sqlite_database_path.as_deref() {
            let preview =
                sqlite::preview_category_lifecycle_at(database_path, source_id, target_id)?;
            let source_id = CategoryId::new(
                u64::try_from(preview.source.id)
                    .map_err(|_| "SQLite source category identity is invalid".to_string())?,
            );
            let target_id = preview
                .target
                .as_ref()
                .map(|target| u64::try_from(target.id).map(CategoryId::new))
                .transpose()
                .map_err(|_| "SQLite target category identity is invalid".to_string())?;
            let confirmation_phrase =
                lifecycle_confirmation_phrase(source_id, target_id, &preview.revision);
            Ok(CategoryLifecycleReview {
                source_id,
                source_name: preview.source.name,
                target_id,
                target_name: preview.target.map(|target| target.name),
                counts: CategoryLifecycleCounts {
                    completed_sessions: preview.references.completed_sessions,
                    active_sessions: preview.references.active_sessions,
                    tags: preview.references.tags,
                    sand_placed: preview.references.sand_placed,
                    sand_pending: preview.references.sand_pending,
                    history_placed: preview.references.snapshot_placed,
                    history_pending: preview.references.snapshot_pending,
                    checkpoint_references: preview.references.checkpoint_references,
                },
                checkpoint_custody: preview
                    .checkpoint_status
                    .unwrap_or_else(|| "absent".to_string()),
                revision: preview.revision,
                confirmation_phrase,
            })
        } else {
            self.try_write_runtime_checkpoint()?;
            let review = crate::legacy_category_lifecycle::build_review(
                &LegacyCategoryLifecyclePaths::runtime(),
                source_id.0,
                target_id.map(|target| target.0),
            )?;
            Ok(normalize_legacy_review(review))
        }
    }

    fn apply_category_lifecycle(&mut self, review: CategoryLifecycleReview) {
        let source_was_active = self.time_tracker.active_category_id() == review.source_id;
        let active_description = if source_was_active {
            self.time_tracker
                .category_description_by_id(review.source_id)
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };

        let result = if let Some(database_path) = self.sqlite_database_path.clone() {
            sqlite::apply_category_lifecycle_at(
                &database_path,
                review.source_id,
                review.target_id,
                &review.revision,
                Utc::now(),
            )
            .map(|_| ())
        } else {
            let paths = LegacyCategoryLifecyclePaths::runtime();
            crate::legacy_category_lifecycle::prepare(
                &paths,
                review.source_id.0,
                review.target_id.map(|target| target.0),
                &review.revision,
                Utc::now(),
            )
            .and_then(|_| crate::legacy_category_lifecycle::replay_prepared(&paths).map(|_| ()))
        };

        if let Err(error) = result {
            let prepared = self.sqlite_database_path.is_none()
                && crate::legacy_category_lifecycle::has_prepared(
                    &LegacyCategoryLifecyclePaths::runtime(),
                );
            if prepared {
                let _ = self.record_storage_result_for::<()>(
                    PersistenceOperation::CategoryLifecycle,
                    RecoveryAction::ReloadAuthority,
                    Err(error),
                );
            } else if let Some(overlay) = self.category_lifecycle_overlay.as_mut() {
                overlay.error = Some(error);
                overlay.confirmation_input.clear();
            }
            return;
        }

        if let Err(error) = self.try_reload_authority() {
            let _ = self.record_storage_result_for::<()>(
                PersistenceOperation::CategoryLifecycle,
                RecoveryAction::ReloadAuthority,
                Err(error),
            );
            return;
        }

        if self.sqlite_database_path.is_none()
            && source_was_active
            && let Some(target_id) = review.target_id
        {
            if !self.time_tracker.set_active_category_by_id(target_id) {
                let _ = self.record_storage_result_for::<()>(
                    PersistenceOperation::CategoryLifecycle,
                    RecoveryAction::ReloadAuthority,
                    Err(format!(
                        "legacy lifecycle target {} is unavailable after reload",
                        target_id.0
                    )),
                );
                return;
            }
            let _ = self
                .time_tracker
                .set_category_description_by_id(target_id, active_description);
        }

        self.category_lifecycle_overlay = None;
        self.ui_mode = UiMode::Main;
        self.selected_index = 0;
        self.render_needed = true;
    }

    pub(super) fn render_category_lifecycle(&self, frame: &mut Frame, size: Rect) {
        let Some(overlay) = self.category_lifecycle_overlay.as_ref() else {
            return;
        };
        let width = size.width.saturating_sub(4).clamp(54, 108);
        let height = size.height.saturating_sub(4).clamp(18, 32);
        let area = centered_rect(width, height, size);
        frame.render_widget(Clear, area);

        let mut lines = vec![
            Line::from(Span::styled(
                "DESTRUCTIVE LAYER LIFECYCLE",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from("Archive with x for ordinary retirement. This route transforms history."),
            Line::from(""),
            labelled(
                "Source",
                format!("{} — {}", overlay.source_id.0, overlay.source_name),
            ),
        ];

        match &overlay.stage {
            CategoryLifecycleStage::SelectTarget => {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Choose explicit target or permanent deletion:",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                for (index, target) in overlay.targets.iter().enumerate() {
                    let marker = if index == overlay.selected_target {
                        "▶"
                    } else {
                        " "
                    };
                    let style = if target.category_id.is_none() {
                        Style::default().fg(Color::Red)
                    } else if target.archived {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(Span::styled(
                        format!("{marker} {}", target.label),
                        style,
                    )));
                }
                lines.push(Line::from(""));
                lines.push(Line::from("[↑/↓] select  [Enter] review  [Esc] cancel"));
            }
            CategoryLifecycleStage::Confirm(review) => {
                lines.extend([
                    labelled(
                        "Target",
                        review
                            .target_id
                            .zip(review.target_name.as_deref())
                            .map(|(id, name)| format!("{} — {name}", id.0))
                            .unwrap_or_else(|| "PERMANENT DELETION".to_string()),
                    ),
                    labelled("Revision", review.revision.clone()),
                    labelled("Checkpoint custody", review.checkpoint_custody.clone()),
                    Line::from(""),
                    labelled("Completed sessions", review.counts.completed_sessions),
                    labelled("Active generations", review.counts.active_sessions),
                    labelled("Tags", review.counts.tags),
                    labelled("Canonical sand placed", review.counts.sand_placed),
                    labelled("Canonical sand pending", review.counts.sand_pending),
                    labelled("Historical placed", review.counts.history_placed),
                    labelled("Historical pending", review.counts.history_pending),
                    labelled("Checkpoint references", review.counts.checkpoint_references),
                    labelled("Total references", review.counts.total()),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Type this exact phrase:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        review.confirmation_phrase.clone(),
                        Style::default().fg(Color::Yellow),
                    )),
                    labelled("Input", overlay.confirmation_input.clone()),
                    Line::from("[Enter] apply only on exact match  [Esc] cancel"),
                ]);
            }
        }
        if let Some(error) = overlay.error.as_deref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                error.to_string(),
                Style::default().fg(Color::Red),
            )));
        }

        let block = Block::default()
            .title(" Layer lifecycle review ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Red));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

fn normalize_legacy_review(review: LegacyCategoryLifecycleReview) -> CategoryLifecycleReview {
    let source_id = CategoryId::new(review.source.id);
    let target_id = review
        .target
        .as_ref()
        .map(|target| CategoryId::new(target.id));
    CategoryLifecycleReview {
        source_id,
        source_name: review.source.name,
        target_id,
        target_name: review.target.map(|target| target.name),
        counts: CategoryLifecycleCounts {
            completed_sessions: review.references.completed_sessions,
            active_sessions: review.references.active_session,
            tags: review.references.tags,
            sand_placed: review.references.sand_placed,
            sand_pending: review.references.sand_pending,
            history_placed: review.references.history_placed,
            history_pending: review.references.history_pending,
            checkpoint_references: review.references.checkpoint_references,
        },
        checkpoint_custody: review.checkpoint_custody,
        revision: review.revision.clone(),
        confirmation_phrase: review.confirmation_phrase,
    }
}

fn lifecycle_confirmation_phrase(
    source: CategoryId,
    target: Option<CategoryId>,
    revision: &str,
) -> String {
    match target {
        Some(target) => format!("MERGE {} INTO {} {revision}", source.0, target.0),
        None => format!("DELETE {} {revision}", source.0),
    }
}

fn labelled(label: &str, value: impl ToString) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.to_string()),
    ])
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
    use super::*;

    #[test]
    fn confirmation_phrase_binds_source_target_and_revision() {
        assert_eq!(
            lifecycle_confirmation_phrase(CategoryId::new(7), Some(CategoryId::new(9)), "abc123"),
            "MERGE 7 INTO 9 abc123"
        );
        assert_eq!(
            lifecycle_confirmation_phrase(CategoryId::new(7), None, "abc123"),
            "DELETE 7 abc123"
        );
    }

    #[test]
    fn exact_confirmation_is_not_case_or_whitespace_fuzzy() {
        let expected = "MERGE 1 INTO 2 deadbeef";
        assert_eq!(expected, "MERGE 1 INTO 2 deadbeef");
        assert_ne!(expected, "merge 1 into 2 deadbeef");
        assert_ne!(expected, "MERGE 1 INTO 2 deadbeef ");
    }
}
