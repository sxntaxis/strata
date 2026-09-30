use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use super::{App, overlay_layout};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SystemDialogSeverity {
    Warning,
    Error,
}

impl App {
    pub(super) fn move_system_dialog_selection(&mut self, delta: isize, action_count: usize) {
        if action_count == 0 {
            self.system_dialog_selected_index = 0;
            return;
        }
        let current = self.system_dialog_selected_index.min(action_count - 1);
        self.system_dialog_selected_index = if delta < 0 {
            (current + action_count - 1) % action_count
        } else if delta > 0 {
            (current + 1) % action_count
        } else {
            current
        };
        self.render_needed = true;
    }

    pub(super) fn present_warning(&mut self, subtitle: impl Into<String>, message: impl Into<String>) {
        self.system_notice = Some(super::SystemNoticeState {
            severity: SystemDialogSeverity::Warning,
            subtitle: subtitle.into(),
            message: message.into(),
            restore_selected_index: self.system_dialog_selected_index,
        });
        self.system_dialog_selected_index = 0;
        self.render_needed = true;
    }

    pub(super) fn present_error(&mut self, subtitle: impl Into<String>, message: impl Into<String>) {
        self.system_notice = Some(super::SystemNoticeState {
            severity: SystemDialogSeverity::Error,
            subtitle: subtitle.into(),
            message: message.into(),
            restore_selected_index: self.system_dialog_selected_index,
        });
        self.system_dialog_selected_index = 0;
        self.render_needed = true;
    }

    pub(super) fn dismiss_system_notice(&mut self) {
        if let Some(notice) = self.system_notice.take() {
            self.system_dialog_selected_index = notice.restore_selected_index;
        }
        self.render_needed = true;
    }

    pub(super) fn render_system_notice(&self, frame: &mut Frame, terminal: Rect) {
        let Some(notice) = self.system_notice.as_ref() else {
            return;
        };
        let natural = notice
            .subtitle
            .chars()
            .count()
            .max(notice.message.chars().count())
            .max("Go back".chars().count())
            .saturating_add(2);
        self.render_system_dialog(
            frame,
            terminal,
            notice.severity,
            Line::from(Span::styled(
                notice.subtitle.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            vec![Line::from(notice.message.clone())],
            vec![Line::from("Go back")],
            0,
            u16::try_from(natural).unwrap_or(u16::MAX).max(32),
        );
    }

    pub(super) fn render_system_dialog(
        &self,
        frame: &mut Frame,
        terminal: Rect,
        severity: SystemDialogSeverity,
        subtitle: Line<'static>,
        mut body: Vec<Line<'static>>,
        actions: Vec<Line<'static>>,
        selected: usize,
        minimum_inner_width: u16,
    ) {
        let semantic = match severity {
            SystemDialogSeverity::Warning => self.theme_warning(),
            SystemDialogSeverity::Error => self.theme_error(),
        };
        let title = match severity {
            SystemDialogSeverity::Warning => "WARNING",
            SystemDialogSeverity::Error => "ERROR",
        };
        let natural_content_width = body
            .iter()
            .map(|line| line.width())
            .chain(actions.iter().map(|line| line.width()))
            .chain(std::iter::once(subtitle.width()))
            .max()
            .unwrap_or(0);
        let requested_inner_width = minimum_inner_width.max(
            u16::try_from(natural_content_width.saturating_add(2)).unwrap_or(u16::MAX),
        );
        let max_inner_width = terminal
            .width
            .saturating_sub(crate::constants::APP_LAYOUT_SETTINGS.frame_margin.saturating_mul(2))
            .saturating_sub(2)
            .max(1);
        let effective_width = requested_inner_width.min(max_inner_width).max(1);
        let wrapped_body_height = body
            .iter()
            .map(|line| {
                let width = u16::try_from(line.width()).unwrap_or(u16::MAX);
                width.saturating_add(effective_width.saturating_sub(1)) / effective_width
            })
            .map(|height| height.max(1))
            .fold(0u16, u16::saturating_add);
        let inner_height = 1u16
            .saturating_add(1)
            .saturating_add(wrapped_body_height)
            .saturating_add(1)
            .saturating_add(u16::try_from(actions.len()).unwrap_or(u16::MAX));
        let area = overlay_layout::centered_overlay_rect(
            terminal,
            requested_inner_width,
            inner_height,
            1,
            3,
            crate::constants::APP_LAYOUT_SETTINGS.frame_margin,
        );
        frame.render_widget(Clear, area);
        let block = Block::default()
            .style(Style::default().bg(self.theme_background()))
            .title(Line::from(Span::styled(
                title,
                Style::default()
                    .fg(semantic)
                    .add_modifier(Modifier::BOLD),
            )))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(semantic));
        frame.render_widget(block.clone(), area);
        let inner = overlay_layout::modal_content_rect(block.inner(area), inner_height, true);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let mut lines = Vec::new();
        lines.push(subtitle.alignment(Alignment::Center));
        lines.push(Line::default());
        for line in &mut body {
            lines.push(line.clone().alignment(Alignment::Center));
        }
        lines.push(Line::default());

        let selected = selected.min(actions.len().saturating_sub(1));
        for (index, mut action) in actions.into_iter().enumerate() {
            if index == selected {
                let foreground = self.theme_selection_text();
                let padding = usize::from(inner.width).saturating_sub(action.width());
                if padding > 0 {
                    action.spans.push(Span::raw(" ".repeat(padding)));
                }
                lines.push(
                    action
                        .style(Style::default().fg(foreground).bg(self.theme_selection()))
                        .alignment(Alignment::Left),
                );
            } else {
                lines.push(action.alignment(Alignment::Left));
            }
        }
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }
}
