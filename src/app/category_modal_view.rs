use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use crate::constants::APP_LAYOUT_SETTINGS;

use super::{App, overlay_layout};

impl App {
    pub(super) fn render_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let categories = self.categories_for_render();
        let minimum_content_height =
            categories.len().saturating_add(1).min(u16::MAX as usize) as u16;
        let category_width = categories
            .iter()
            .map(|category| {
                2usize.saturating_add(self.display_layer_name(&category.name).chars().count())
            })
            .max()
            .unwrap_or(0);
        let insert_label_width = if self.new_category_name.is_empty() {
            "+ Forge new layer...".chars().count()
        } else {
            self.new_category_name.chars().count()
        };
        let insert_width = 2usize.saturating_add(insert_label_width);
        let selected_width = categories
            .get(self.selected_index)
            .map(|category| {
                let layer_width =
                    2usize.saturating_add(self.display_layer_name(&category.name).chars().count());
                let description_width = if self.modal_editing_category_metadata {
                    self.modal_description.chars().count()
                } else if let Some(completion) =
                    self.tag_completion_for_category(category.id, &self.modal_description)
                {
                    let suffix_width = completion
                        .tag
                        .chars()
                        .skip(completion.typed_chars)
                        .count();
                    self.modal_description
                        .chars()
                        .count()
                        .saturating_add(suffix_width)
                } else {
                    self.modal_description.chars().count()
                };
                if description_width == 0 {
                    layer_width
                } else {
                    layer_width
                        .saturating_add(1)
                        .saturating_add(description_width)
                }
            })
            .unwrap_or(0);
        let minimum_content_width = category_width
            .max(insert_width)
            .max(selected_width)
            .min(u16::MAX as usize) as u16;
        let minimum_inner_width = minimum_content_width.saturating_add(2);
        let minimum_inner_height = minimum_content_height.saturating_add(2);
        let modal_rect = overlay_layout::centered_overlay_rect(
            terminal_size,
            minimum_inner_width,
            minimum_inner_height,
            1,
            3,
            APP_LAYOUT_SETTINGS.frame_margin,
        );

        let border_color = self.get_selected_color();

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
                    let text_color = crate::appearance::contrasting_text_color(
                        cat.color,
                        self.theme_foreground(),
                    );
                    let layer_name = self.display_layer_name(&cat.name);
                    let description_style = Style::default()
                        .fg(text_color)
                        .add_modifier(ratatui::style::Modifier::ITALIC);
                    let mut line_spans = vec![
                        Span::raw(dot).fg(text_color),
                        Span::raw(layer_name).fg(text_color),
                    ];
                    if self.modal_editing_category_metadata {
                        if !self.modal_description.is_empty() {
                            line_spans.push(Span::styled(
                                format!(" {}", self.modal_description),
                                description_style,
                            ));
                        }
                    } else if let Some(completion) =
                        self.tag_completion_for_category(cat.id, &self.modal_description)
                    {
                        let suffix = completion
                            .tag
                            .chars()
                            .skip(completion.typed_chars)
                            .collect::<String>();
                        line_spans.push(Span::styled(
                            format!(" {}", self.modal_description),
                            description_style,
                        ));
                        line_spans.push(Span::styled(
                            suffix,
                            description_style.add_modifier(ratatui::style::Modifier::DIM),
                        ));
                    } else if !self.modal_description.is_empty() {
                        line_spans.push(Span::styled(
                            format!(" {}", self.modal_description),
                            description_style,
                        ));
                    }
                    ListItem::new(Line::from(line_spans))
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
                let cycling_color = self
                    .appearance
                    .sand_color_at(self.new_category_color_cursor);

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
                            .fg(crate::appearance::contrasting_text_color(
                                self.theme_accent(),
                                self.theme_foreground(),
                            ))
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
            .highlight_style(Style::default());
        let frame_block = Block::default()
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
            .border_style(Style::default().fg(border_color));
        let inner = frame_block.inner(modal_rect);
        let list_area = overlay_layout::modal_content_rect(inner, minimum_content_height, true);

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_widget(frame_block, modal_rect);
        f.render_stateful_widget(list, list_area, &mut list_state);
    }
}
