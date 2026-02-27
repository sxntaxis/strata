use chrono::Local;
use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
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
        let sand = self.sand_engine.render(&categories);
        let active_index = self.time_tracker.active_category_index();

        let category_name = if active_index == Some(0) {
            self.get_idle_face()
        } else if let Some(idx) = active_index {
            categories
                .get(idx)
                .map(|category| category.name.clone())
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

        if self.in_category_modal() {
            self.render_modal(f, size);
        } else if self.in_karma_modal() {
            self.render_report_modal(f, size);
        }
    }
}
