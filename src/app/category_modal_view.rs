use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use crate::constants::COLORS;

use super::{App, view_style};

impl App {
    pub(super) fn render_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let modal_rect = self.modal_rect(terminal_size);

        let border_color = self.get_selected_color();
        let categories = self.time_tracker.categories_ordered();

        let items: Vec<ListItem> = categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected_index;
                let dot = if cat.karma_effect < 0 { "◯ " } else { "● " };

                if is_selected {
                    let text_color = view_style::text_color_for_bg(cat.color);
                    let description_text = if self.modal_description.is_empty() {
                        Span::raw("")
                    } else {
                        Span::styled(
                            format!(" {}", self.modal_description),
                            Style::default().add_modifier(ratatui::style::Modifier::ITALIC),
                        )
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(cat.color),
                        Span::raw(&cat.name).fg(text_color),
                        description_text,
                    ]))
                    .style(Style::default().fg(text_color).bg(cat.color))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(cat.color),
                        Span::raw(&cat.name).fg(Color::White),
                    ]))
                }
            })
            .chain(std::iter::once({
                let is_selected = self.is_on_insert_space();
                let cycling_color = COLORS[self.color_index];

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Add new..."
                        } else {
                            &self.new_category_name
                        }),
                    ]))
                    .style(Style::default().fg(Color::Black).bg(Color::White))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Add new..."
                        } else {
                            &self.new_category_name
                        })
                        .fg(Color::White),
                    ]))
                }
            }))
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(Span::styled(
                        "strata",
                        Style::default().fg(Color::White),
                    )))
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .border_style(Style::default().fg(border_color)),
            )
            .highlight_style(Style::default());

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_stateful_widget(list, modal_rect, &mut list_state);
    }
}
