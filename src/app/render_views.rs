use chrono::Local;
use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, LineGauge, Paragraph},
};

use crate::constants::APP_LAYOUT_SETTINGS;
use crate::domain::{CategoryId, DRIFT_CATEGORY_CONFIG_NAME, is_drift_category_id};

use super::App;

impl App {
    pub(super) fn draw_frame(&mut self, f: &mut Frame) {
        let size = f.size();

        let inner_width = size.width.saturating_sub(2);
        let inner_height = size.height.saturating_sub(2);

        if self.sand_engine.cell_width != inner_width
            || self.sand_engine.cell_height != inner_height
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

        let active_category_id = self.time_tracker.active_category_id();
        let category_name = if is_drift_category_id(active_category_id) {
            String::new()
        } else if let Some(idx) = active_index {
            categories
                .get(idx)
                .map(|category| self.display_layer_name(&category.name))
                .unwrap_or_default()
        } else {
            String::new()
        };

        let description =
            session_subtitle_for_frame(active_category_id, self.time_tracker.active_description());

        let session_timer = if is_drift_category_id(active_category_id) {
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
                .unwrap_or(DRIFT_CATEGORY_CONFIG_NAME);
            let karma_time = if cat_name == DRIFT_CATEGORY_CONFIG_NAME {
                self.get_karma_adjusted_time()
            } else {
                self.get_category_karma_adjusted_time(cat_name)
            };
            (karma_time != 0).then(|| self.format_signed_time(karma_time))
        } else if is_drift_category_id(active_category_id) {
            let karma_time = self.get_karma_adjusted_time();
            (karma_time != 0).then(|| self.format_signed_time(karma_time))
        } else if let Some(idx) = active_index {
            let cat_name = categories
                .get(idx)
                .map(|category| category.name.as_str())
                .unwrap_or(DRIFT_CATEGORY_CONFIG_NAME);
            let mut total = self.get_effective_time_for_category(cat_name);
            if let Some(start) = self.time_tracker.current_session_start {
                total += start.elapsed().as_secs() as usize;
            }
            (total != 0).then(|| self.format_time(total))
        } else {
            let total = self.get_effective_time_today();
            (total != 0).then(|| self.format_time(total))
        };

        let border_color = self.get_active_color();
        let block = frame_block(
            category_name,
            description,
            session_timer,
            effective_time_str,
            border_color,
        );

        let paragraph = Paragraph::new(sand).block(block);
        f.render_widget(paragraph, size);

        if let Some(progress) = self.catchup_progress_ratio() {
            let max_width = size
                .width
                .saturating_sub(APP_LAYOUT_SETTINGS.frame_margin.saturating_mul(2));
            let gauge_width = ((size
                .width
                .saturating_mul(APP_LAYOUT_SETTINGS.catchup_gauge_width_num))
                / APP_LAYOUT_SETTINGS.catchup_gauge_width_den)
                .max(APP_LAYOUT_SETTINGS.catchup_gauge_min_width)
                .min(max_width);
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

        if self.recovery_statement.is_some() {
            self.render_recovery_statement(f, size);
        }

        if self.has_persistence_recovery() {
            self.render_persistence_recovery(f, size);
        }
    }
}

fn session_subtitle_for_frame(active_category_id: CategoryId, active_description: &str) -> String {
    if is_drift_category_id(active_category_id) {
        String::new()
    } else {
        active_description.to_string()
    }
}

fn frame_block(
    category_name: String,
    description: String,
    session_timer: String,
    effective_time: Option<String>,
    border_color: Color,
) -> Block<'static> {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
    if !category_name.is_empty() || !description.is_empty() {
        block = block.title(
            Line::from(vec![
                Span::styled(
                    category_name,
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
        );
    }
    block = block.title(
        Line::from(Span::styled(
            session_timer,
            Style::default().fg(Color::White),
        ))
        .alignment(Alignment::Center),
    );
    if let Some(effective_time) = effective_time {
        block = block.title(
            Line::from(Span::styled(
                effective_time,
                Style::default().fg(Color::White),
            ))
            .alignment(Alignment::Right),
        );
    }
    block
}

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier},
        widgets::{Widget, block::Block},
    };

    use super::{frame_block, session_subtitle_for_frame};

    fn rendered_text(block: Block<'static>) -> String {
        let area = Rect::new(0, 0, 60, 3);
        let mut buffer = Buffer::empty(area);
        block.render(area, &mut buffer);
        let mut text = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                text.push_str(buffer.get(x, y).symbol());
            }
        }
        text
    }

    fn top_row_cell_start(buffer: &Buffer, area: Rect, needle: &str) -> u16 {
        let needle = needle.chars().map(|ch| ch.to_string()).collect::<Vec<_>>();
        let width = u16::try_from(needle.len()).expect("title needle should fit terminal width");
        (0..=area.width.saturating_sub(width))
            .find(|&start| {
                needle.iter().enumerate().all(|(offset, expected)| {
                    buffer.get(start + offset as u16, 0).symbol() == expected.as_str()
                })
            })
            .unwrap_or_else(|| panic!("top-row title {needle:?} should render"))
    }

    #[test]
    fn frame_uses_active_session_subtitle_and_hides_it_while_idle() {
        assert_eq!(
            session_subtitle_for_frame(crate::domain::CategoryId::new(1), "deep"),
            "deep"
        );
        assert_eq!(
            session_subtitle_for_frame(crate::domain::DRIFT_CATEGORY_ID, "must not leak"),
            ""
        );
    }

    #[test]
    fn frame_titles_match_idle_active_and_effective_time_contract() {
        let idle = rendered_text(frame_block(
            String::new(),
            String::new(),
            "12:34:56".to_string(),
            None,
            Color::White,
        ));
        assert!(idle.contains("12:34:56"));
        assert!(!idle.contains("00:00:00"));
        assert!(!idle.contains("o_o"));

        let active = rendered_text(frame_block(
            "Work".to_string(),
            "focus".to_string(),
            "00:00:07".to_string(),
            Some("00:00:03".to_string()),
            Color::White,
        ));
        assert!(active.contains("Work focus"));
        assert!(active.contains("00:00:07"));
        assert!(active.contains("00:00:03"));

        let area = Rect::new(0, 0, 60, 3);
        let mut styled = Buffer::empty(area);
        frame_block(
            "Work".to_string(),
            "focus".to_string(),
            "00:00:07".to_string(),
            Some("00:00:03".to_string()),
            Color::White,
        )
        .render(area, &mut styled);
        let work_start = top_row_cell_start(&styled, area, "Work");
        let subtitle_start = top_row_cell_start(&styled, area, "focus");
        for x in work_start..work_start + "Work".len() as u16 {
            assert!(styled.get(x, 0).modifier.contains(Modifier::BOLD));
        }
        for x in subtitle_start..subtitle_start + "focus".len() as u16 {
            assert!(styled.get(x, 0).modifier.contains(Modifier::ITALIC));
        }

        let negative = rendered_text(frame_block(
            "Work".to_string(),
            String::new(),
            "00:00:07".to_string(),
            Some("-00:00:03".to_string()),
            Color::White,
        ));
        assert!(negative.contains("-00:00:03"));
    }
}
