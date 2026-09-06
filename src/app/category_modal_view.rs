use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};


use super::{App, view_style};

impl App {
    pub(super) fn render_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let modal_rect = self.modal_rect(terminal_size);

        let border_color = self.get_selected_color();
        let categories = self.categories_for_render();

        let items: Vec<ListItem> = categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected_index;
                let dot = if cat.balance_effect < 0 {
                    "◯ "
                } else {
                    "● "
                };

                if is_selected {
                    let text_color = view_style::text_color_for_bg(cat.color);
                    let layer_name = self.display_layer_name(&cat.name);
                    let description_text = if self.modal_description.is_empty() {
                        Span::raw("")
                    } else {
                        Span::styled(
                            format!(" {}", self.modal_description),
                            Style::default()
                                .fg(text_color)
                                .add_modifier(ratatui::style::Modifier::ITALIC),
                        )
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(text_color),
                        Span::raw(layer_name).fg(text_color),
                        description_text,
                    ]))
                    .style(Style::default().fg(text_color).bg(cat.color))
                } else {
                    let layer_name = self.display_layer_name(&cat.name);
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(cat.color),
                        Span::raw(layer_name).fg(self.theme_foreground()),
                    ]))
                }
            })
            .chain(std::iter::once({
                let is_selected = self.is_on_insert_space();
                let cycling_color = self.appearance.sand_color_at(self.new_category_color_cursor);

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Forge new layer..."
                        } else {
                            &self.new_category_name
                        }),
                    ]))
                    .style(
                        Style::default()
                            .fg(view_style::text_color_for_bg(self.theme_accent()))
                            .bg(self.theme_accent()),
                    )
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Forge new layer..."
                        } else {
                            &self.new_category_name
                        })
                        .fg(self.theme_foreground()),
                    ]))
                }
            }))
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        let list = List::new(items)
            .style(Style::default().bg(self.theme_background()))
            .block(
                Block::default()
                    .style(Style::default().bg(self.theme_background()))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Line::from(Span::styled(
                        if self.modal_editing_category_metadata {
                            "Strata · layer metadata"
                        } else {
                            "Strata"
                        },
                        Style::default().fg(self.theme_foreground()),
                    )))
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .border_style(Style::default().fg(border_color)),
            )
            .highlight_style(Style::default());

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_stateful_widget(list, modal_rect, &mut list_state);
    }
}
