use chrono::Local;
use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, LineGauge, Paragraph},
};

use crate::constants::SAND_ENGINE;

use super::App;

impl App {
    pub(super) fn draw_frame(&mut self, f: &mut Frame) {
        let size = f.size();

        let inner_width = size.width.saturating_sub(2);
        let inner_height = size.height.saturating_sub(2);

        if self.sand_engine.width != inner_width * SAND_ENGINE.dot_width as u16
            || self.sand_engine.height != inner_height * SAND_ENGINE.dot_height as u16
        {
            self.sand_engine.resize(inner_width, inner_height);
        }

        let categories = self.time_tracker.categories_ordered();
        let sand = if self.in_karma_modal() && self.should_use_report_snapshot() {
            self.report_snapshot_lines(inner_width, inner_height, &categories)
                .unwrap_or_else(|| self.sand_engine.render(&categories))
        } else if let Some(catchup_lines) =
            self.catchup_visual_lines(inner_width, inner_height, &categories)
        {
            catchup_lines
        } else {
            self.sand_engine.render(&categories)
        };
        let active_index = self.time_tracker.active_category_index();

        let category_name = if active_index == Some(0) {
            self.get_idle_face()
        } else if let Some(idx) = active_index {
            categories
                .get(idx)
                .map(|category| self.display_layer_name(&category.name))
                .unwrap_or_else(|| self.get_idle_face())
        } else {
            self.get_idle_face()
        };

        let description = active_index
            .and_then(|idx| {
                categories
                    .get(idx)
                    .map(|category| category.description.clone())
            })
            .unwrap_or_default();

        let session_timer = if active_index == Some(0) {
            Local::now().format("%H:%M:%S").to_string()
        } else if let Some(start) = self.time_tracker.current_session_start {
            let elapsed = start.elapsed();
            self.format_time(elapsed.as_secs() as usize)
        } else {
            Local::now().format("%H:%M:%S").to_string()
        };

        let effective_time_str = if self.in_category_modal() {
            let cat_name = categories
                .get(self.selected_index)
                .map(|category| category.name.as_str())
                .unwrap_or("none");
            let karma_time = if cat_name == "none" {
                self.get_karma_adjusted_time()
            } else {
                self.get_category_karma_adjusted_time(cat_name)
            };
            self.format_signed_time(karma_time)
        } else if active_index == Some(0) {
            let karma_time = self.get_karma_adjusted_time();
            self.format_signed_time(karma_time)
        } else if let Some(idx) = active_index {
            let cat_name = categories
                .get(idx)
                .map(|category| category.name.as_str())
                .unwrap_or("none");
            let mut total = self.get_effective_time_for_category(cat_name);
            if let Some(start) = self.time_tracker.current_session_start {
                total += start.elapsed().as_secs() as usize;
            }
            self.format_time(total)
        } else {
            self.format_time(self.get_effective_time_today())
        };

        let border_color = self.get_active_color();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(
                Line::from(vec![
                    Span::styled(
                        &category_name,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    if description.is_empty() {
                        Span::raw("")
                    } else {
                        Span::styled(
                            format!(" {}", description),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::ITALIC),
                        )
                    },
                ])
                .alignment(Alignment::Left),
            )
            .title(
                Line::from(Span::styled(
                    session_timer.as_str(),
                    Style::default().fg(Color::White),
                ))
                .alignment(Alignment::Center),
            )
            .title(
                Line::from(Span::styled(
                    effective_time_str.as_str(),
                    Style::default().fg(Color::White),
                ))
                .alignment(Alignment::Right),
            )
            .border_style(Style::default().fg(border_color));

        let paragraph = Paragraph::new(sand).block(block);
        f.render_widget(paragraph, size);

        if let Some(progress) = self.catchup_progress_ratio() {
            let max_width = size.width.saturating_sub(4);
            let gauge_width = ((size.width.saturating_mul(2)) / 5).max(12).min(max_width);
            if gauge_width > 0 {
                let gauge_x = size.x + (size.width.saturating_sub(gauge_width)) / 2;
                let gauge_y = size.y + size.height.saturating_sub(1);
                let gauge = LineGauge::default()
                    .ratio(progress)
                    .label("")
                    .line_set(ratatui::symbols::line::THICK)
                    .style(Style::default().fg(Color::DarkGray))
                    .gauge_style(Style::default().fg(border_color).bg(Color::DarkGray));
                f.render_widget(gauge, Rect::new(gauge_x, gauge_y, gauge_width, 1));
            }
        }

        if self.in_category_modal() {
            self.render_modal(f, size);
        } else if self.in_karma_modal() {
            self.render_report_modal(f, size);
        }

        if self.show_keybindings_modal {
            self.render_keybindings_modal(f, size);
        }

        if self.show_command_palette {
            self.render_command_palette(f, size);
        }
    }
}
