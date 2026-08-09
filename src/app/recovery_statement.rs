use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use super::App;

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
            .unwrap_or("active generation");
        let description = if statement.active_description.is_empty() {
            "(empty)"
        } else {
            statement.active_description.as_str()
        };
        let lines = vec![
            Line::from(Span::styled(
                "RECOVERY EVIDENCE",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!(
                    "{} -> {}",
                    statement.recovered_interval_class.label(),
                    statement.post_target_class.label()
                ),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            labelled("Active identity", active_identity),
            labelled("Category", statement.active_category_id.to_string()),
            labelled("Description", description),
            labelled(
                "Active started",
                format_timestamp(statement.active_session_started_at_utc),
            ),
            Line::from(""),
            labelled(
                "Checkpoint captured",
                format_timestamp(statement.checkpoint_captured_at_utc),
            ),
            labelled(
                "Durable sediment through",
                format_timestamp(statement.checkpoint_simulation_at_utc),
            ),
            labelled(
                "Recovery target",
                format_timestamp(statement.recovery_target_utc),
            ),
            labelled(
                "Reconstructed duration",
                format_duration(statement.reconstructed_duration_nanos),
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

fn labelled(label: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value.into()),
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
